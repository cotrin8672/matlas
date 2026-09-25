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
have no status channel. These calls are memory-safe when their documented
preconditions hold, but an out-of-memory termination or MATLAB-side abort can
skip Rust destructors. This is a limitation of the C MEX ABI, not a recoverable
Rust error.

Non-trapping callbacks, MATLAB error functions, pointer adoption, allocator
replacement, and other operations that can bypass the ownership model are only
available in `matlas::raw`. Safe code must not mix raw mutations with live safe
owners or borrows.
