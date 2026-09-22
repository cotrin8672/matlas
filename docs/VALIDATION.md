# Validation

Current local checks use MATLAB R2025a/API 800, MSVC, and Rust 1.95:

```powershell
$env:MATLABROOT = 'C:\Program Files\MATLAB\R2025a'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p matrust-integration
Copy-Item target/debug/matrust_integration.dll target/debug/matrust_integration.mexw64 -Force
```

The checked-in MATLAB test exercises five MAT-file formats, 156 array-direction
checks, sequential and metadata readers, workspace copies, explicit close,
panic/error cleanup, and the ownership constructors (numeric, char, logical,
cell, and sparse). Linux and macOS are not yet validated.
