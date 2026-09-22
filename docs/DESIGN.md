# Design

`matrust` makes MATLAB's ownership conventions visible in Rust's type system.
The C shim is private; safe callers never manipulate an `mxArray*` directly.

## Ownership categories

- `OwnedArray<'mex>` is the sole owner and calls `mxDestroyArray` on drop.
- `ArrayRef<'a>` is a read-only borrow from an input, owned array, cell, or
  struct. It has no destructor and cannot be transferred to MATLAB.
- `ArrayMut<'a, 'mex>` is an exclusive borrow of an owned array. Replacing a
  cell/field consumes an `OwnedArray`, making transfer explicit.
- `WorkspaceRef<'a>` models `mexGetVariablePtr`; it borrows `&mut Matlab` and
  holds a runtime guard. Calls that may invalidate MATLAB-owned pointers are
  rejected while it is alive.
- `PersistentArray<'mex>` owns a generation-checked persistent slot. Dropping
  or explicitly removing the key releases the MATLAB array; the MEX exit hook
  is a final cleanup path.
- `ArrayInfo<'mex>` owns metadata returned by `matGetVariableInfo` but exposes
  no public conversion to `ArrayRef`, because MATLAB fills its data pointers
  with non-dereferenceable sentinels.

The `Matlab<'mex>` brand is invariant and thread-bound. MEX entrypoints create
it internally; `Matlab::attach` is unsafe for standalone/manual integration.

## Calls and errors

`Matlab::call` and `eval` use MATLAB's trap APIs. A trapped exception becomes a
`Callback` error after all temporary outputs are destroyed. The entrypoint
converts Rust `Result` and panics to MATLAB errors only after Rust destructors
have run. Native aborts and MATLAB out-of-memory termination are outside any
Rust destructor guarantee because they do not unwind the Rust stack.

`MatFile` uses a separate owner for every file handle. `close(self)` reports
the final native status; `Drop` closes once when the caller does not inspect
that status. Returned arrays remain valid after the file is closed.
