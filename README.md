# rustmat

MathWorks の MAT-File C API (`mat.h` の全12関数) を、rustmex の配列型・所有権・エラー処理につなぐ Rust クレートです。MAT フォーマットの解析は MATLAB の `libmat` が担当します。

主対象は **Windows x86_64 / MSVC / MATLAB R2025a / rustmex 0.6.4 / matlab800**。独立プロジェクトであり、rustmex または MathWorks の公式製品ではありません。crates.io には未公開です。

- [調査結果と当初の実装計画](docs/IMPLEMENTATION_PLAN.md)
- [現在の設計・安全性・API対応表](docs/DESIGN.md)
- [実機検証と再現手順](docs/VALIDATION.md)

## MEX から使う

利用側は `cdylib` にし、rustmex の `matlab800` と **`alloc`** を有効にします。`ToMatlab` / `Numeric` など、Rust の Box を MATLAB の配列に移す変換には MATLAB allocator が必要です。System allocator との混用はヒープ破損を起こします。rustmat 自体はプロセス全体の allocator を選択しません。

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
rustmat = { git = "https://github.com/cotrin8672/rustmat" }
rustmex = { version = "0.6.4", default-features = false, features = ["matlab800", "alloc"] }
```

```rust
use rustmat::{MatFile, Matlab, OpenMode};
use rustmex::prelude::*;

rustmat::mex_entrypoint!(run);

fn run(lhs: Lhs, rhs: Rhs) -> rustmex::Result<()> {
    rustmex::assert!(rhs.len() == 1 && lhs.len() == 1,
        "example:args", "one input and one output required");
    // SAFETY: MATLAB が呼び出した MEX の実行スレッド上で使用し、
    // 全ハンドルをこの呼び出し内で破棄。owned 配列は MATLAB に返す。
    let matlab = unsafe { Matlab::attach() };
    let mut file = MatFile::create(&matlab, "result.mat")?;
    file.put(c"value", rhs[0])?;
    file.close()?;

    let mut file = MatFile::open(&matlab, "result.mat", OpenMode::Read)?;
    lhs[0] = Some(file.get(c"value")?);
    file.close()?;
    Ok(())
}
```

入口には `rustmat::mex_entrypoint!(run)` を一度だけ使います。`#[rustmex::entrypoint]` と併用しません。引数・戻り値は rustmex の `Lhs` / `Rhs` / `Result` のままです。出力を要求しない呼び出しでは `lhs` は空です。

この入口は Rust が `Err` または unwind panic を返した後、C 側で MATLAB のエラーを発報します。失敗途中の owned 出力とファイルは先に破棄されます。`rustmex::trigger_error!` など、Rust フレームの途中から直接 MATLAB の非局所脱出を起こすAPIは使わず、`Err` / `?` で返してください。外部ネイティブ例外、OOM、abort、panic中の二重panicからの回復は保証しません。

## Windows のビルド

MATLAB と MSVC C コンパイラをインストールし、**利用側プロジェクト**の `.cargo/config.toml` に backend のリンク設定を置きます。MATLAB の場所に合わせて変更してください。

```toml
[target.x86_64-pc-windows-msvc.mex800]
rustc-link-search = ['C:\Program Files\MATLAB\R2025a\extern\lib\win64\microsoft']
rustc-link-lib = ["libmx", "libmex", "libmat"]
```

```powershell
$env:MATLABROOT = 'C:\Program Files\MATLAB\R2025a'
cargo build --release
# 利用側の crate 名に合わせて変更
Copy-Item target/release/example.dll target/release/example.mexw64
```

`mex800` override は rustmex_matlab800 0.2.0 の build script を置き換えます。rustmat の C shim は `MATLABROOT` のヘッダーを API 800 でコンパイルします。`FILE*` 診断で CRT を混用しないため、Windows の `crt-static` は拒否します。通常の MSVC ターゲットの shared CRT を使用してください。

Linux / macOS のライブラリ探索分岐はありますが、ビルド・export・実行は未検証です。Octave、matlab700、独立 executable、MATLAB Runtime だけの配置は対応保証に含めません。

## 公開 API

| 操作 | Rust API |
| --- | --- |
| 開く・作成 | `MatFile::open` / `create` / `create_with_format` |
| 通常／global保存 | `put` / `put_global` (`&mxArray` を借用) |
| 配列読出し | `get` → `rustmex::MxArray` |
| メタデータのみ | `info` → `ArrayInfo`、cell / struct 内は `InfoRef` |
| 名前一覧・削除 | `variables` / `delete` |
| 逐次読出し | `Variables` / `VariableInfos` (専用handleを所有) |
| エラー状態 | `last_error` → raw `MatError(i32)` |
| native stream診断 | `stream` → `Option<FileStream>`、EOF / error / clear / position |
| 終了 | `close(self)` → `Result<()>`、未close時は `Drop` |

作成形式は `MatVersion::{Default, V4, V6, V7, V73}`。`create()` は明示的に v7 を選びます。Read / Update / Write は `OpenMode` で区別し、不正な読み書きは C を呼ぶ前に拒否します。Update は既存ファイルが必要です。

変数名は `&CStr` (例: `c"value"`)。パスは `AsRef<Path>` を受け、Unicode のまま UTF-8 に変換します。空パス・内部NUL・非Unicodeパスはエラーです。Windowsで日本語と絵文字を含むパスの実機テストを用意しています。

`Matlab::attach()` の unsafe は、MATLABの実行スレッド・backend・ランタイム・配列寿命を利用者が保証する境界です。その後のファイル操作は safe です。ファイルとメタデータはcontextの寿命に束縛され、別スレッドに移せません。rustmex の owned `MxArray` 自体にはcontext lifetimeがないため、返却値をRustのstatic等へ保存して次のMEX呼び出しに持ち越さないでください。

正常終了を確認したい保存には必ず `close()?` を使います。`Drop` はclose失敗を通知できません。エラーが返っても同じhandleを再closeしません。

`ArrayInfo` はデータを持たない専用型です。通常の `mxArray` への変換や `put` はできません。逐次readerは別handleで取得した変数数を基準に動作するため、読み取り中にファイルを変更しないでください。

`matGetFp` は所有権移転や任意I/Oを許可せず、借用中の診断として公開します。v7.3等でnative pointerがない場合は `None` です。MAT API の位置管理を壊す `fseek` / `fwrite` / `fclose` / raw pointer は公開しません。

MAT-File API の保存可能型は MathWorks の制約に従います。built-in 数値、logical、char、sparse、cell、structが主対象です。user-defined class、table、string、GPU配列等の保存・復元を一括保証しません。
