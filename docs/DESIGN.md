# Design

`rustmat` extends rustmex's ownership model to MATLAB's published MAT-file C API. MATLAB parses and writes the file format; the crate does not implement a MAT parser, an array type, or a serialization framework.

## Boundaries

- A private C shim resolves the API 800 symbols from `mat.h`.
- `Matlab::attach()` is the explicit unsafe boundary for the current MEX thread and runtime.
- `MatFile<'ctx>` and metadata are tied to that context and are neither `Send` nor `Sync`.
- `put` borrows an `mxArray`; `get` returns an owned rustmex `MxArray`.
- `ArrayInfo` is metadata-only and cannot become a normal `mxArray`.
- `close(self)` reports finalization errors. `Drop` closes once but cannot report failure.

## Native API mapping

| MAT-file C API | Rust API |
| --- | --- |
| `matOpen`, `matClose` | `MatFile::open`, `create*`, `close` |
| `matPutVariable`, `matPutVariableAsGlobal` | `put`, `put_global` |
| `matGetVariable`, `matGetVariableInfo` | `get`, `info` |
| `matGetNextVariable`, `matGetNextVariableInfo` | `Variables`, `VariableInfos` |
| `matGetDir`, `matDeleteVariable` | `variables`, `delete` |
| `matGetErrno` | `last_error`, `Error::mat_error` |
| `matGetFp` | borrowed `FileStream` diagnostics |

Sequential readers own a dedicated handle because MATLAB forbids mixing `matGetNextVariable*` with other file operations. They use a separate handle to obtain the expected variable count, making premature end-of-file an error.

`mex_entrypoint!` places `mexFunction` in C. Rust first handles `Result`, panic conversion, output ownership, and destructors; only then does C call MATLAB's error function. Native exceptions, aborts, out-of-memory failures, and double panics remain outside this guarantee.

The crate does not distribute MATLAB headers or libraries. Supported MATLAB value types remain limited by the MAT-file API.
