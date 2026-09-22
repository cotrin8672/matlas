# 実機検証記録

検証日: 2026-09-22。Windows x86_64、MATLAB R2025a Update 1、MSVC 14.43.34808 (Visual Studio 2022 Community)、Rust 1.95.0。依存版はCargo.lock参照。以下は実際にビルドしたMEXをMATLAB内で呼び出した結果。

## 結果

| 検証 | 結果 |
| --- | --- |
| `cargo build -p rustmat-integration` | 成功、debug DLLをmexw64へコピー |
| DLL exports | `mexFunction` / `mexfilerequiredapiversion` をdumpbinで確認 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 成功 |
| `cargo test --lib` | 4 passed |
| `cargo test --doc` | 7 passed (6 compile-fail + 1 compile-only usage example) |
| MATLAB `run_tests` | プロセスexit 0、全5形式、156 array-direction checks |

MATLABの完了マーカー:

```text
RUSTMAT_FORMAT_PASS 0 (2 arrays)
RUSTMAT_FORMAT_PASS 1 (2 arrays)
RUSTMAT_FORMAT_PASS 2 (2 arrays)
RUSTMAT_FORMAT_PASS 3 (23 arrays)
RUSTMAT_FORMAT_PASS 4 (23 arrays)
RUSTMAT_ALL_PASS 156 array-direction checks; 5 formats; lifecycle/global/Unicode/workspace
```

format番号は `Default / V4 / V6 / V7 / V73`。1方向あたりの配列件数を、Rust保存→MATLAB load、MATLAB save→Rust get、Rust逐次readerの3経路で数えたもの。metadata等を156件へ水増しして数えていない。

## テスト範囲

- 全形式でreal/complex double。v7/v7.3ではsingle、complex single、符号付き/符号なし8/16/32/64bit整数、logical、char (日本語)、empty、0x3、3D、real/complex/logical sparse、cell、nested struct、empty cellを含む計23配列。
- 値、class、shape、sparseをMATLAB側で確認。全形式で全MATLAB型をテストしたという意味ではない。legacy形式の試験は2種類のdoubleに限定する。
- v4 descriptor、v6/v7の非圧縮/圧縮tag、v7.3のHDF5 signatureをファイルから検査。単に拡張子やload成功でformat対応と判断しない。
- random get、directory、metadataとcell/struct子header、full/info両iterator、fused EOF、explicit close。
- updateで既存値を置換し別変数を削除。Read/Writeモード違反を拒否。
- missing read/update、corrupt file、missing variable/delete、空/NULパス、空変数名。
- missing取得後に成功するgetを実行し、過去のerror状態で正常結果を誤判定しないことを確認。
- empty file: Default/V6/V7/V73はdirectoryと両iteratorが空。V4の空出力はゼロbytesで、native openは失敗。この形式固有の挙動を期待値として確認。
- stream診断はv7.3以外でSomeと位置照会成功を検査。EOF/error/clear経路も呼び出す。v7.3はNoneを許容。
- v7/v7.3のput_globalを `whos(...).global` とload値で確認。file close後もmetadataを読めることを確認。
- 日本語と絵文字のdirectory/filenameで保存・get・MATLAB loadを確認。
- caller workspaceの配列をrustmexで借用して保存し、MATLAB側で値を照合。
- `?` による早期Errの前に保存したファイルがload/rename可能。途中生成したowned出力もエラー経路で破棄。
- 故意のRust panic後もMATLAB catchに `rustmat:mex:panic` が届き、保存済みファイルがload/rename可能。panic hookの標準エラー出力は意図したテストログであり、プロセスはabortしない。
- 引数・出力数ゼロ、invalid error ID、Unicode/`%s`/NULを含むエラーmessage、エラー後の正常呼び出し。
- 25回の保存・逐次読出し反復。

unitテストはNUL/Unicode入力、エラーbufferのUTF-8境界とID検証、明示close成功・失敗・Drop時の1回だけのfinalizeを検査。close失敗はprivate test seamによるstatus注入であり、実ディスク障害を発生させた検証ではない。close済みhandleでerrnoを照会しないことも確認する。

compile-failテストはファイルのSend/Sync禁止、context寿命、stream borrow中のclose禁止、metadataのput禁止、子metadataの親寿命を検査。

## 検証で見つけて修正した統合問題

1. **allocatorの混用**: rustmexの数値変換はRust BoxをmxSetDataへ渡す。利用側の `alloc` が無効だとMATLABがSystem allocationを解放してheap corruptionした。integration consumerに `alloc` を明示し、利用手順にも必須条件として記載した。root libraryが一方的にglobal allocatorを選択することは避けた。
2. **既存entrypointのエラー境界**: rustmex 0.6.4のRust extern C入口からmexErrMsgIdAndTxtを呼ぶと、この環境ではnative exceptionがRust frameを横断して `panic in a function that cannot unwind` / 0xc0000409となった。Rustが帰還してからCで発報する `mex_entrypoint!` に変更し、Err/panicと後続呼び出しを実機で再確認した。
3. **empty V4**: write/closeは成功するがファイルheaderがないため空ファイルのreopenは失敗する。wrapperが空directoryとして偽装せず、native open errorを保持する。

missing variable時のmatGetErrnoはlegacy側で2、v7.3では0も観測した。NULL自体を失敗として扱い、code0を「成功」の意味で上書きしない。raw整数を保存し、未確認の独自列挙に変換しない判断を裏付ける。

## 再現手順

[README](../README.md)の `.cargo/config.toml` をこのcheckoutに作成してからPowerShellで実行する。MATLABROOTとrepoは自身の配置に合わせる。

```powershell
$env:MATLABROOT = 'C:\Program Files\MATLAB\R2025a'
$env:PATH = "$env:MATLABROOT\bin\win64;$env:PATH"
cargo build -p rustmat-integration
Copy-Item target/debug/rustmat_integration.dll target/debug/rustmat_integration.mexw64 -Force
cargo clippy --workspace --all-targets -- -D warnings
cargo test --lib
cargo test --doc
$repo = (Get-Location).Path.Replace('\', '/')
& "$env:MATLABROOT\bin\matlab.exe" -wait -batch "addpath('$repo/tests/matlab'); run_tests('$repo')" -logfile work/matlab-integration.log
```

`work/integration` 内に生成したMATファイルを使う。テストはそのdirectory内の同名ファイルを置き換える。利用者のデータdirectoryから実行しない。`work/`、`target/`、ローカルの `.cargo/config.toml` はGit追跡外。別プロセスのMATLABが旧MEXをロード中なら、置換前にそのMEXをclearする。

## 未検証・保証しないもの

Linux/macOSのビルド・native exports・実行、R2025a以外のrelease、release最適化構成、2GB超の実データ、実ディスクfullやI/O切断、MATLAB user-defined class/objects、同時更新、OOMやnative非局所脱出、MATLAB外のstandalone実行は未検証。native close失敗はfault injectionのみ。実機テストの成功をこれらの保証には広げない。

APIとcrate構造は実装済みだがcrates.io publication、名前確保、配布ライセンス選定は行っていない。
