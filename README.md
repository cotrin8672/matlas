# rustmat

MathWorks の MAT-File C API (`libmat`) を、rustmex の配列型・所有権・エラー処理と接続する Rust クレートの設計プロジェクトです。

現在は**調査・実装計画の段階**です。ライブラリ本体はまだ実装していません。

- [詳細な調査結果と実装計画](docs/IMPLEMENTATION_PLAN.md)
- 想定する初期対象: Windows x86_64 / MSVC / MATLAB R2025a / rustmex 0.6.4 / interleaved complex API
- rustmex と互換性を持つ独立プロジェクトであり、rustmex または MathWorks の公式製品ではありません。

今後の開発はこのリポジトリの ghq チェックアウトを作業場所とします。
