# matrust

`matrust` is a Rust-native safety layer for MATLAB's API-800 Matrix, MEX, and
MAT-file interfaces. It is not a wrapper around `rustmex`: MATLAB-owned
arrays, Rust-owned arrays, workspace borrows, and persistent arrays are
different Rust types with different lifetimes and drop behavior.

## Requirements

- MATLAB with `matrix.h`, `mex.h`, and `mat.h`
- Rust and a native C compiler supported by MATLAB
- MATLAB R2025a/API 800 is the current validation target

Set `MATLABROOT` to the MATLAB installation. The build script links the
versioned API libraries and compiles the C shim automatically.

## Example

```rust
use matrust::{Inputs, Matlab, Outputs, Result};

matrust::mex_entrypoint!(run);

fn run<'mex>(cx: &mut Matlab<'mex>, inputs: Inputs<'mex>, out: &mut Outputs<'mex>) -> Result<()> {
    let value = inputs.get(0).ok_or_else(|| matrust::Error::new(
        matrust::ErrorKind::InvalidInput, "example", "one input required"))?;
    out.set(0, cx.call(c"double", &[value], 1)?.pop().unwrap())?;
    Ok(())
}
```

`OwnedArray` is the unique Rust owner and destroys its `mxArray` on drop.
`ArrayRef` is read-only and cannot be destroyed or transferred. A
`WorkspaceRef` borrows MATLAB workspace memory through `&mut Matlab`, so a
callback or workspace mutation cannot occur while that pointer is live.

`MatFile` provides typed open/create, get, metadata, put, global put, delete,
directory listing, and explicit close. `OwnedArray::persist` provides an
explicit persistent lifetime with generation-checked keys.

## Status

The crate is being rebuilt in this repository as `matrust`; the old `rustmat`
and `rustmex` APIs are intentionally not dependencies. Windows/R2025a is the
first supported target. See [DESIGN.md](docs/DESIGN.md) for the ownership
model and [VALIDATION.md](docs/VALIDATION.md) for current checks.

## License

MIT. MATLAB headers and libraries are not distributed; users must follow
MathWorks licensing and deployment terms.
