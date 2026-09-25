# Design

`matlas` makes MATLAB's ownership conventions visible in Rust's type system.
The C shim is private; safe callers never manipulate an `mxArray*` directly.

## Ownership categories

- `OwnedArray<'mex>` is the sole owner and calls `mxDestroyArray` on drop.
- `ArrayRef<'a>` is a read-only borrow from an input, owned array, cell, or
  struct. It has no destructor and cannot be transferred to MATLAB.
- `ArrayMut<'a, 'mex>` is an exclusive borrow of an owned array. Replacing a
  cell/field consumes an `OwnedArray`, making transfer explicit.
- `WorkspaceScope<'a, 'mex>` shares `Matlab` with MAT-file handles while multiple
  `WorkspaceValue<'a>` pointers from `mexGetVariablePtr` are fetched. Each value
  holds an external-borrow guard. Operations that can run MATLAB code, including
  property accessors and generic MAT-file serialization, return `Busy` while a
  value is live. Its consuming `call` accepts workspace values and ordinary
  `ArrayRef` inputs in argument order. Workspace values are released immediately before
  `mexCallMATLABWithTrap`; a live value omitted from the call is rejected by
  the runtime guard. After the callback, old workspace pointers cannot be read.
- `PlainArrayRef<'a>` validates the native class ID before `MatFile::put_plain`
  writes a borrowed array. Only numeric, logical, and character classes are
  accepted, including numeric/logical sparse arrays. Cell, struct, string, and
  object arrays cannot enter this path because their serialization can run
  user-defined MATLAB code. Generic `put` and `put_global` remain available
  when no workspace value is borrowed.
- `PersistentArray` owns a generation-checked persistent slot and may be stored
  between MEX invocations. Reading, mutating, or explicitly removing it
  requires the current invocation's `Matlab<'mex>` context. Dropping it
  releases the MATLAB array; the MEX exit hook is a final cleanup path.
- `ArrayInfo<'mex>` owns metadata returned by `matGetVariableInfo` but exposes
  no public conversion to `ArrayRef`, because MATLAB fills its data pointers
  with non-dereferenceable sentinels.

The `Matlab<'mex>` brand is invariant and thread-bound. MEX entrypoints create
it internally; `Matlab::attach` is unsafe for standalone/manual integration.
`Matlab::lock` returns a thread-bound `ModuleLock` guard whose destructor
balances exactly one `mexLock`; raw manual unlock remains outside the safe API.

## Workspace-borrow callback audit

| Safe operation | During a live `WorkspaceValue` |
|---|---|
| `Matlab::call`, `eval`, `workspace_put` | `Busy`; also require `&mut Matlab` |
| `Matlab::property`, `OwnedArray::property`, `ArrayMut::set_property` | `Busy`; accessors can run MATLAB code |
| `MatFile::put`, `put_global`, `get`, `Variables::next` | `Busy`; object serialization/deserialization can run MATLAB code |
| `MatFile::put_plain` | Allowed for validated primitive arrays |
| Array inspection, constructors, duplication, MAT-file headers/directory/handle operations | Allowed; these do not invoke MATLAB user code |

Other safe context-mutating operations require `&mut Matlab` and are excluded
by the scope's shared borrow. Raw API calls are outside this guarantee.

## Calls and errors

`Matlab::call` and `eval` use MATLAB's trap APIs. A trapped exception becomes a
`Callback` error after all temporary outputs are destroyed. The entrypoint
converts Rust `Result` and panics to MATLAB errors only after Rust destructors
have run. Native aborts and MATLAB out-of-memory termination are outside any
Rust destructor guarantee because they do not unwind the Rust stack.

`MatFile` uses a separate owner for every file handle. `close(self)` reports
the final native status; `Drop` closes once when the caller does not inspect
that status. Returned arrays remain valid after the file is closed. Full-value
reads and sequential value iteration also require no active workspace borrow
because object deserialization can run `loadobj`. Header-only reads, directory
listing, and file-handle operations do not invoke MATLAB user code.
