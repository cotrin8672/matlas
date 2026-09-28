# matlas

`matlas` is a Rust-native safety layer for MATLAB's API-800 Matrix, MEX, and
MAT-file interfaces. It is not a wrapper around `rustmex`: MATLAB-owned
arrays, Rust-owned arrays, workspace borrows, and persistent arrays are
different Rust types with different lifetimes and drop behavior.

## Requirements

- MATLAB with `matrix.h`, `mex.h`, and `mat.h`
- Rust and a native C compiler supported by MATLAB
- MATLAB R2024a/API 800 is the v0.10 validation target

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

Fixed input and output counts are checked before the handler runs. For a
variable input count, use `DynInputs<'mex>` and
`mex_entrypoint!(run, dynamic)`; `inputs.require::<N>()` checks and destructures
an exact count when needed. Omitting the output const parameter keeps variable
output counts. `Outputs::requested()` reports the explicit MATLAB request;
`set_first()` also supports an optional first result in `ans` when that count
is zero. Every explicitly requested output must be set before the handler
returns. `ArrayRef::to_text`
reads a character row or scalar MATLAB string; `logical_scalar` reads and
creates MATLAB logical scalars. `call_array::<N>` returns a fixed number of
callback outputs. `error_id!` validates static exception IDs at compile time;
`ErrorId::try_from(String)` validates dynamic IDs. `Error::with_id` attaches an
ID without another `Result`. Import `ResultExt` to use `.with_id(id)`,
`.context("description")`, and `.with_context(|| format!(...))` directly on
fallible operations while preserving the original error. See the
[v0.8 migration notes](docs/ERROR_HANDLING.md#migration-from-v07).

### Migration from v0.9

- Fixed input handlers use `Inputs<'mex, N>` with exactly `N` values. Variable
  input handlers use `DynInputs<'mex>` and `mex_entrypoint!(run, dynamic)`.
- `MatFile`, `Variables`, and `VariableInfos` now take only the invocation
  lifetime; they no longer borrow the `Matlab` variable for their full lifetime.
- `Error` fields are private. Read them through accessors, including
  `matlab_error()` for a trapped callback's original identifier and message.
- Replace `ArrayMut::set_property` with trapped `Matlab::with_property` and use
  its returned object for value classes. Explicitly requested MEX output slots
  must all be filled; use `set_first` to set optional `ans` when none was
  requested.

### Migration from v0.8

- Read an exact, single numeric element with `array.as_scalar::<T>()`.
  This rejects arrays with more than one element and never converts the class.
- Use `cx.char_row(text)` to create a MATLAB character row and
  `array.decode_chars()` to decode all of a character array's UTF-16 units.
  These replace `Matlab::string` and `ArrayRef::string`; neither method creates
  or reads a MATLAB `string` object.
- `matlas::Complex<T>` now re-exports `num_complex::Complex<T>`. Use its `re`
  and `im` fields instead of `real` and `imag`. Values from `num-complex` can be
  passed directly to `numeric` and `scalar`.

`OwnedArray` is the unique Rust owner and retires its `mxArray` on drop;
destruction may wait until a workspace borrow or callback handoff completes.
`ArrayRef` is read-only and cannot be destroyed or transferred. A
`WorkspaceScope::get` borrows MATLAB workspace memory without copying it. A
scope can fetch several `WorkspaceValue`s and pass them, with ordinary array
views, to `WorkspaceScope::call`. The call consumes the scope and the passed
values; it rejects any workspace values left live outside the call.
`WorkspaceScope` can coexist with a `MatFile`; callback-capable operations
return `Busy` while a workspace value is live. For a primitive workspace array,
`WorkspaceValue::plain` validates its class and `MatFile::put_plain` writes it
without first duplicating the source array. `ArrayRef::is_plain` and
`WorkspaceValue::is_plain` query the same classification without creating an
error. Cell, struct, string, and object arrays are excluded from this path.

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

```rust,no_run
# use matlas::{MatFile, Matlab, Result, Workspace};
fn save_waveform(cx: &mut Matlab<'_>) -> Result<()> {
    let mut file = MatFile::create(cx, "waveform.mat")?;
    let ws = cx.workspace_scope(Workspace::Caller);
    let value = ws.get(c"At")?;
    file.put_plain(c"At", value.plain()?)?;
    file.close()
}
```

## Status

The old `rustmat` and `rustmex` APIs are intentionally not dependencies.
Windows is the supported target; v0.10 was validated on R2024a. The
R2025a/API-800 audit covers all 178 published C functions: safe operations
use lifetime-aware types, aliases use a more general
safe operation, and ownership-adopting or non-trapping callback and error
operations remain explicitly unsafe in `matlas::raw`. Native allocation
failure can still terminate a MEX function. See [API_COVERAGE.md](docs/API_COVERAGE.md)
for the function-by-function inventory, [DESIGN.md](docs/DESIGN.md) for the
ownership model, [ERROR_HANDLING.md](docs/ERROR_HANDLING.md) for what `Result`
can and cannot catch, and [VALIDATION.md](docs/VALIDATION.md) for current checks.

## License

MIT. MATLAB headers and libraries are not distributed; users must follow
MathWorks licensing and deployment terms.
