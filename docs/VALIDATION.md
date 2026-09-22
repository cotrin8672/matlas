# Validation

Validated on Windows x86_64 with MATLAB R2025a Update 1, MSVC 14.43, and Rust 1.95.

```text
RUSTMAT_FORMAT_PASS 0 (2 arrays)
RUSTMAT_FORMAT_PASS 1 (2 arrays)
RUSTMAT_FORMAT_PASS 2 (2 arrays)
RUSTMAT_FORMAT_PASS 3 (23 arrays)
RUSTMAT_FORMAT_PASS 4 (23 arrays)
RUSTMAT_ALL_PASS 156 array-direction checks; 5 formats; lifecycle/global/Unicode/workspace
```

The MATLAB integration test covers:

- default, v4, v6, v7, and v7.3 files, including format signatures;
- numeric classes, real and complex arrays, logical, char, sparse, empty, multidimensional, cell, and struct values;
- Rust-to-MATLAB, MATLAB-to-Rust, full sequential reads, and metadata reads;
- update, delete, global variables, Unicode paths, caller workspace values, and invalid inputs;
- normal close, early `Err`, panic cleanup, empty argument lists, and a successful call after an error.

Rust checks include Clippy with warnings denied, four unit tests, and seven doctests. Compile-fail tests cover context lifetimes, thread transfer, metadata misuse, and borrowed stream lifetimes.

Not yet validated: Linux, macOS, MATLAB releases other than R2025a, disk-full close failures, very large files, or user-defined MATLAB classes.

Run the checked-in integration test after configuring the linker as described in the README:

```powershell
$env:MATLABROOT = 'C:\Program Files\MATLAB\R2025a'
cargo build -p rustmat-integration
Copy-Item target/debug/rustmat_integration.dll target/debug/rustmat_integration.mexw64 -Force
$repo = (Get-Location).Path.Replace('\', '/')
& "$env:MATLABROOT\bin\matlab.exe" -wait -batch "addpath('$repo/tests/matlab'); run_tests('$repo')"
```
