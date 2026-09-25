# matlas

`matlas` is a Rust-native safety layer for MATLAB's API-800 Matrix, MEX, and
MAT-file interfaces. It is not a wrapper around `rustmex`: MATLAB-owned
arrays, Rust-owned arrays, workspace borrows, and persistent arrays are
different Rust types with different lifetimes and drop behavior.

## Requirements

- MATLAB with `matrix.h`, `mex.h`, and `mat.h`
- Rust and a native C compiler supported by MATLAB
- MATLAB R2024a/API 800 is the v0.6 validation target

Set `MATLABROOT` to the MATLAB installation. The build script links the
versioned API libraries and compiles the C shim automatically.

## Example

```rust
use matlas::{Inputs, Matlab, Outputs, Result};

matlas::mex_entrypoint!(run);

fn run<'mex>(cx: &mut Matlab<'mex>, inputs: Inputs<'mex, 1>, out: &mut Outputs<'mex, 1>) -> Result<()> {
    let [value] = inputs.into_array();
    out.set(0, cx.duplicate(value)?)?;
    Ok(())
}
```

Fixed input and output counts are checked before the handler runs. Omitting
the const parameters keeps the existing variable-count API. `ArrayRef::to_text`
reads a character row or scalar MATLAB string; `logical_scalar` reads and
creates MATLAB logical scalars. `call_array::<N>` returns a fixed number of
callback outputs, and `Error::with_id` sets a validated MATLAB exception ID.

`OwnedArray` is the unique Rust owner and destroys its `mxArray` on drop.
`ArrayRef` is read-only and cannot be destroyed or transferred. A
`WorkspaceScope::get` borrows MATLAB workspace memory without copying it. A
scope can fetch several `WorkspaceValue`s and pass them, with ordinary array
views, to `WorkspaceScope::call`. The call consumes the scope and the passed
values; it rejects any workspace values left live outside the call.

```rust,no_run
# use matlas::{ArrayRef, Matlab, Result, Workspace};
fn sum<'mex>(cx: &mut Matlab<'mex>, id: ArrayRef<'mex>) -> Result<()> {
    let ws = cx.workspace_scope(Workspace::Caller);
    let f = ws.get(c"F")?;
    let detuning = ws.get(c"detuning")?;
    let _outputs = ws.call(c"store.findRecord", [id.into(), f.into(), detuning.into()], 2)?;
    Ok(())
}
```

`MatFile` provides typed open/create, get, metadata, put, global put, delete,
directory listing, and explicit close. `OwnedArray::persist` returns a
generation-checked handle that can be kept between MEX invocations; accessing
it again requires the new invocation's `Matlab` context.

## Status

The old `rustmat` and `rustmex` APIs are intentionally not dependencies.
Windows is the supported target; v0.6 was validated on R2024a. The
R2025a/API-800 audit covers all 178 published C functions: safe operations
use lifetime-aware types, aliases use a more general
safe operation, and ownership-adopting or non-local-exit operations remain
explicitly unsafe in `matlas::raw`. See [API_COVERAGE.md](docs/API_COVERAGE.md)
for the function-by-function inventory, [DESIGN.md](docs/DESIGN.md) for the
ownership model, [ERROR_HANDLING.md](docs/ERROR_HANDLING.md) for what `Result`
can and cannot catch, and [VALIDATION.md](docs/VALIDATION.md) for current checks.

## License

MIT. MATLAB headers and libraries are not distributed; users must follow
MathWorks licensing and deployment terms.
