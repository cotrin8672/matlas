# Validation

Current v0.6.0 local checks use MATLAB R2024a/API 800, MSVC, and Rust 1.94.1.
The declared Rust 1.77 minimum was checked by building the full workspace
with `cargo +1.77.0 build --workspace --locked`.

The v0.6.0 scalar text, logical scalar, fixed-arity MEX, callback-array, and
custom error-ID cases passed on R2024a on 2026-09-25. The v0.6.0 workspace
also built with Rust 1.77.0.

```powershell
$env:MATLABROOT = 'C:\Program Files\MATLAB\R2024a'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --workspace --no-deps
powershell -File scripts/audit_api.ps1
cargo build --workspace
Copy-Item target/debug/matlas_integration.dll target/debug/matlas_integration.mexw64 -Force
Copy-Item target/debug/matlas_fixed_integration.dll target/debug/matlas_fixed_integration.mexw64 -Force
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
