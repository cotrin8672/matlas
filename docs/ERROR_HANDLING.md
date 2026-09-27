# Error handling

Safe handlers should return `matlas::Result<()>` and propagate failures with
`?`. The entrypoint waits until the handler and all of its Rust frames have
returned before the C shim calls `mexErrMsgIdAndTxt`, so ordinary `Result`
errors and panics run Rust destructors first.

The safe API reports every failure that the underlying C operation reports by
status or null pointer:

| Failure source | Rust behavior |
|---|---|
| invalid shape, type, index, mode, name, or active workspace borrow | validated before the native mutation and returned as `Err` |
| MAT-file status/null return | `Err` with the operation, status, and current `matGetErrno` where available |
| `mexCallMATLABWithTrap` / `mexEvalStringWithTrap` exception | `Err(ErrorKind::Callback)` with the MATLAB identifier/message after the trap and any partial outputs are destroyed |
| explicit `MatFile::close` failure | `Err(ErrorKind::Close)`; implicit `Drop` still closes but cannot report a status |
| iterator read failure | an `Err` iterator item; the owned file still closes on drop |

`WorkspaceValue` keeps an external-borrow guard until it is dropped or moved
into `WorkspaceScope::call`. Callback-capable safe operations return
`Err(ErrorKind::Busy)` before entering MATLAB while that guard is active. This
includes property get/set, generic MAT-file writes, full-value MAT-file reads,
and sequential full-value reads. `MatFile::put_plain` accepts only a validated
`PlainArrayRef` and can write primitive workspace arrays without duplicating
the source `mxArray`.

`Result` cannot turn a native non-local exit into Rust unwinding. MATLAB
documents that many `mxCreate*` allocation failures terminate a MEX function
instead of returning null. Void C operations such as `mxSetProperty` likewise
have no status channel. An out-of-memory termination or MATLAB-side abort can
skip Rust destructors. The Rust Reference does not permit discarding Rust
frames without running their destructors, so v0.9 does not claim that such a
native exit is safely recoverable. The exact control transfer and a possible
C-only trapping boundary still need isolated validation before 1.0. See the
[MATLAB allocation behavior](https://www.mathworks.com/help/matlab/apiref/mxcreatenumericarray.html),
[MATLAB MEX cleanup behavior](https://www.mathworks.com/help/matlab/matlab_external/automatic-cleanup-of-temporary-arrays.html),
and [Rust's runtime assumptions](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).

Non-trapping callbacks, MATLAB error functions, pointer adoption, allocator
replacement, and other operations that can bypass the ownership model are only
available in `matlas::raw`. Safe code must not mix raw mutations with live safe
owners or borrows.

## Error identifiers and context

`ErrorId` contains an already validated MATLAB exception identifier.
`error_id!("store:FileWriteFailed")` validates a literal at compile time,
including when the macro is used in an ordinary expression. Dynamic IDs use
`ErrorId::try_from(String)` and return `ErrorKind::InvalidInput` on invalid text.
Both paths require at least two colon-separated ASCII components, each starting
with a letter and containing only letters, digits, or underscores, with a
255-byte limit for the complete ID.

`Error::with_id` accepts an `ErrorId` and returns `Error` directly. It replaces
only the identifier exposed to MATLAB; `kind`, `operation`, `detail`, `status`,
and `mat_error` keep the original cause. The last ID supplied wins. Without a
custom ID, the default still comes from `ErrorKind`.

Import `ResultExt` to attach IDs and context directly to `matlas::Result<T>`:

```rust,no_run
use matlas::{error_id, ArrayRef, ErrorId, MatFile, Result, ResultExt};
use std::ffi::CStr;

const WRITE_FAILED: ErrorId = error_id!("store:FileWriteFailed");

fn save(file: &mut MatFile<'_, '_>, name: &CStr, value: ArrayRef<'_>) -> Result<()> {
    file.put(name, value)
        .with_id(WRITE_FAILED)
        .with_context(|| format!("Could not save Value '{}'.", name.to_string_lossy()))
}
```

`context("description")` adds fixed text. `with_context(|| format!(...))`
generates text only when the result is an error. Successful values pass through
unchanged. Context does not change the identifier or overwrite the original
detail, including a trapped MATLAB exception's original ID and message.
Repeated context is displayed from outermost (last added) to innermost,
followed by the original operation, detail, and native status codes.

`ResultExt::with_id` applies to every error, including `Busy` and validation
errors. Use it where one public ID represents all failures of the operation;
use `map_err` when only specific error kinds should be relabeled.

### Migration from v0.7

Replace `error.with_id("store:FileWriteFailed")?` with
`error.with_id(error_id!("store:FileWriteFailed"))`. For dynamic identifiers,
validate the string with `ErrorId::try_from` before attaching the ID. This
moves fallible ID validation out of the error-attachment operation.
