# rustmat 実装設計

2026-09-22。初期調査は [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)。本書は実装後の決定を記す。

## 目的と依存

MATLAB 内の Rust MEX で、rustmex の値を MAT ファイルへ保存・復元する。配列wrapper、Rust側のMATparser、Serde層、独自HDF5処理を重ねない。`rustmex 0.6.4` (default features無効、matlab800有効)、build dependency `cc 1` のみを直接依存にする。推移依存の版は Cargo.lock に記録する。

FFIはprivate。C shimがインストール済み `mat.h` のAPI800マクロ解決を担当し、Rustが所有権・入力検証・モード制約を担当する。C shimはbindgen不要で、MATLAB SDKや生成済みMathWorksヘッダーを配布しない。

## mat.h の12関数対応

| Native | 公開Rust API | 重要な契約 |
| --- | --- | --- |
| `matOpen` | `MatFile::open/create/create_with_format` | context必須、Read/Update/Write、全作成形式 |
| `matClose` | `close(self)` / `Drop` | 失敗時もhandle消費、二重closeしない |
| `matPutVariable` | `put` | `&mxArray`借用、Rust側複製なし |
| `matPutVariableAsGlobal` | `put_global` | native global属性を保持 |
| `matGetVariable` | `get` | 新規full配列をrustmex owned `MxArray`へ |
| `matGetVariableInfo` | `info` | metadataだけの `ArrayInfo` |
| `matGetNextVariable` | `Variables::next` | 独立handle、名前を即座にownedコピー |
| `matGetNextVariableInfo` | `VariableInfos::next` | 独立handle、full配列と混在しない |
| `matGetDir` | `variables` | 件数を検査、単一確保領域を1回だけmxFree |
| `matDeleteVariable` | `delete` | Read拒否、missingもErr |
| `matGetErrno` | `last_error` / Errorの`mat_error` | native整数保持、OS errnoと混同しない |
| `matGetFp` | `stream` / `FileStream` | Optionと借用寿命、診断のみ |

`wL` は `V6`、`wz` は `V7` と同義のため別variantを作らない。`Default`はnative `w`。`create`はバージョン安定性のため `V7`。Updateは元ファイルの形式を保つ。保存成功を確認するには明示closeを使う。

`matGetFp`のsafe公開はnative機能をそのままraw pointerで露出する意味ではない。EOF/error flagの検査・clear・現在位置の照会に限定する。native formatがFILEを提供しなければNone。FILE所有権移転・任意seek/write/closeはMAT APIの内部状態を壊すので提供しない。Windowsではshared CRTを要求する。

## 所有権と安全性

1. `unsafe Matlab::attach()` が唯一の利用側runtime bootstrap。現在のMATLAB MEX実行スレッド、API800、同一ランタイム、同時MATLAB API呼び出し禁止、現在の呼び出し中に解放またはMATLABへ出力移譲する契約を負う。
2. `MatFile<'context>` と `ArrayInfo<'context>` はcontextを超えて生存できない。Rc phantom / NonNull によりSend/Syncを持たない。ファイルはcloneできない。
3. `put`は借用のみ。`get`はnative新規確保を `MxArray::assume_responsibility_ptr` で1回だけ受領する。`get`の戻り値はファイルclose後も有効。
4. 既存の `MxArray` にはMATLABcontext lifetimeがない。型を二重に包む代わりにattachのunsafe契約に出力寿命を含める。Rust globals / thread_localで呼び出しをまたいで保持しない。この境界を「完全safeなMEX環境」とは称さない。
5. `ArrayInfo` は `mxArray` へDerefしない。配列データアクセス・通常output化・putは禁止。形状、class、complex/sparse/global属性、cell/structの子headerだけを公開する。
6. `InfoRef`は親headerの借用。子headerを個別destroyしない。indexは0基準・線形・column-major。C側でclassと範囲を確認する。
7. matCloseの失敗時にもhandleは無効。`Option::take`後に呼び出し、errno照会や再試行をしない。Dropはnativecloseを一度だけ試み、panic/警告発報しない。

compile-fail doctestでcontext寿命、Send/Sync禁止、metadataをputできないこと、子headerの寿命、stream borrow中のclose禁止を検証する。

## 逐次reader

`matGetNextVariable*` はrandom accessやdirectory照会と混在させない。最初に別handleでdirectoryを取り、そのhandleを閉じた後に新しいread handleを開く。後者はnext/close専用であり、公開型はget/delete/stream等を持たない。

directoryの件数を読み終えた時点でEOFにする。件数到達前のNULLは `UnexpectedEnd` として一度だけ返し、以降はNone。両readerはFusedIterator。この方針はnativeのNULLを正常EOFか読み取り失敗か推測しないためのもの。対象ファイルの同時変更は非対応。variable名はnext呼び出し中にCStringへコピーする。

## エラー

`Error { kind, operation, detail, status, mat_error }` を公開し、`std::error::Error` と `rustmex::MexMessage` を実装する。rustmexの既存From実装により `?` を使える。native数値は `MatError(i32)` として残し、不確かな独自enumに変換しない。

open失敗は有効handleがないためmatGetErrnoを呼ばない。close失敗もhandle無効のためstatusだけ記録する。get/put/delete/nextはnative戻り値が失敗を示す時だけcodeを付加し、過去のerror状態で成功を誤判定しない。`last_error` はあくまでnativeの現在状態であり、Rust側の入力検証エラーを反映するものではない。

パスはUnicodeを厳密にUTF-8へ、名前はCStrを使用。lossy変換をしない。空と内部NULは入力エラーにする。format/release依存の名前長や保存可能型はnativeの責務にする。

## MEX入口を追加した理由

Windows R2025aの実機検証で、rustmex 0.6.4の既存entrypointがErrを処理する際、`mexErrMsgIdAndTxt_800`のnative unwindがRust `extern "C" mexFunction` を横断してabortすることを確認した。Result変換が可能であることだけでは実用上のエラー処理が完成しなかった。

`mex_entrypoint!(handler)` はC-owned `mexFunction`とAPI-version handshakeを取り込む。CがRust dispatchを同期呼び出しし、RustがResultまたはpanicを処理してからstatusとstack bufferをCへ返す。Cがエラーを通知する時点にはRust frameが残らない。

- handlerの型はrustmex `fn(Lhs, Rhs) -> rustmex::Result<()>`。
- 出力はRustの `Vec<Option<MxArray>>` が所有。成功時だけMATLABへ移譲し、失敗時はC帰還前に破棄。
- `catch_unwind` はhandlerだけでなくエラー整形・Rust後処理を含む。二重panic/abort/OOM/native例外までは回復しない。
- 空の入力をNULLからRust sliceに変換しない。出力スロット数は実際のnlhs (0も可能)。
- IDは255bytes以内でMATLAB形式に適合するものだけ通し、それ以外は `rustmat:mex:error`。messageはUTF-8境界で8191bytesまで、NULを`?`へ置換。Cのformatには常に`"%s"`を使う。
- panicは `rustmat:mex:panic`。未知のcustom panic payloadは破棄時再panicを避けるため例外的にリークさせる。通常String/&strは破棄する。
- このmacroはMatlab tokenを自動生成しない。attachのunsafe契約を隠さない。

MEXでRustから配列を構築する利用側にはrustmexの `alloc` が必要。rustmexの数値変換がBoxをmxSetDataへ移譲するため、System allocatorのままだとMATLAB解放時にheap corruptionする。ライブラリ側でglobal allocatorを強制せず、利用側manifestで明示する。

## 対応範囲

12関数のsafe利用経路を提供するが、MathWorksのMAT-File APIが非対応とするMATLAB classを独自シリアライズするものではない。partial array I/O、mmap、serde、async、Octave、standalone runtime bootstrap、crates.io publishは含めない。

Windows x64 MSVC / R2025aで実機検証を行う。Linux/macOSの探索分岐は将来検証のための実装であり、動作確認済みとは扱わない。特にC archiveのexportを各プラットフォームで確認する必要がある。
