# Validation

Current local checks use MATLAB R2025a/API 800, MSVC, and Rust 1.95:
The declared Rust 1.77 minimum was also checked by building both `matlas` and
the `matlas-integration` MEX crate with `cargo +1.77.0 build --locked`.

```powershell
$env:MATLABROOT = 'C:\Program Files\MATLAB\R2025a'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --workspace --no-deps
powershell -File scripts/audit_api.ps1
cargo build -p matlas-integration
Copy-Item target/debug/matlas_integration.dll target/debug/matlas_integration.mexw64 -Force
& "$env:MATLABROOT\bin\matlab.exe" -batch "addpath('$((Resolve-Path tests/matlab).Path -replace '\\','/')'); run_tests('$((Resolve-Path .).Path -replace '\\','/')')"
```

The checked-in MATLAB test exercises five MAT-file formats, 156 array-direction
checks, sequential and metadata readers, workspace copies, explicit close,
panic/error cleanup, and the ownership constructors (numeric, char, logical,
cell, and sparse). It also validates real/complex/logical sparse value access,
empty sparse arrays, multidimensional indexing, and padded character matrices.
It stores, reads twice, and removes a persistent array across separate MEX
invocations, and holds/releases an RAII module lock across calls. Linux and
macOS are not yet validated. Successful and failing trapped function calls,
plus a failing trapped source evaluation, exercise callback `Result`
propagation, including zero-input and zero-output calls. The suite also covers
workspace borrows/copies, owned object properties, structure field ownership,
reshape and real/complex conversion, metadata bits, and `mxRealloc` data
preservation.
