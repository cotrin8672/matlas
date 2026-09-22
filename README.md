# rustmat

Safe MAT-file I/O for Rust MEX functions built with [rustmex](https://crates.io/crates/rustmex).

`rustmat` wraps all 12 functions in MATLAB's published `mat.h` API. It uses `rustmex::mxArray` directly, so values can move between a MEX function and a MAT-file without another Rust array model.

## Requirements

- MATLAB with the C Matrix, MEX, and MAT-file libraries
- Rust with a supported native C compiler
- API 800 / interleaved-complex rustmex backend

Only Windows x86_64, MSVC, and MATLAB R2025a have been tested. Linux and macOS build paths are present but unverified.

## Use

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
rustmat = "0.1"
rustmex = { version = "0.6.4", default-features = false, features = ["matlab800", "alloc"] }
```

```rust
use rustmat::{MatFile, Matlab, OpenMode};
use rustmex::prelude::*;

rustmat::mex_entrypoint!(run);

fn run(lhs: Lhs, rhs: Rhs) -> rustmex::Result<()> {
    rustmex::assert!(lhs.len() == 1 && rhs.len() == 1,
        "example:args", "one input and one output required");

    // SAFETY: this runs synchronously on MATLAB's MEX calling thread, and all
    // handles and arrays are released or returned before the call ends.
    let matlab = unsafe { Matlab::attach() };

    let mut file = MatFile::create(&matlab, "data.mat")?;
    file.put(c"value", rhs[0])?;
    file.close()?;

    let mut file = MatFile::open(&matlab, "data.mat", OpenMode::Read)?;
    lhs[0] = Some(file.get(c"value")?);
    file.close()?;
    Ok(())
}
```

Use `rustmat::mex_entrypoint!` instead of `#[rustmex::entrypoint]`. It lets Rust destroy temporary arrays and close files before MATLAB receives an error. Do not call APIs such as `rustmex::trigger_error!` from inside the handler; return `Err` with `?`.

The consuming MEX crate must enable rustmex's `alloc` feature when it creates MATLAB arrays from Rust allocations.

On Windows, set `MATLABROOT` and add the rustmex backend override to the consuming project's `.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-msvc.mex800]
rustc-link-search = ['C:\Program Files\MATLAB\R2025a\extern\lib\win64\microsoft']
rustc-link-lib = ["libmx", "libmex", "libmat"]
```

## API

`MatFile` provides open/create, get, metadata-only info, put, put-as-global, delete, directory listing, raw error status, limited stream diagnostics, and explicit close. `Variables` and `VariableInfos` provide dedicated sequential readers. Creation supports MAT-file versions 4, 6, 7, and 7.3.

`Matlab::attach` is unsafe because the caller must uphold MATLAB's thread, runtime, and array-lifetime rules. File operations are safe after attachment. Metadata-only arrays have a separate type and cannot be returned to MATLAB or written as full arrays.

See [design notes](docs/DESIGN.md) and [validation](docs/VALIDATION.md) for the remaining constraints.

## License and MATLAB

The original code in this repository is licensed under the [MIT License](LICENSE). Dependencies retain their own licenses.

MATLAB, its headers, and its libraries are not included. Users need an appropriate MATLAB license and must follow MathWorks' terms, including the deployment rules for MAT-file applications. MATLAB is a registered trademark of The MathWorks, Inc. This project is independent of MathWorks.
