# MATLAB C API coverage

This inventory is for the published C declarations visible from MATLAB R2025a
with `TARGET_API_VERSION=800`. `scripts/audit_api.ps1` derives the declaration
set from Clang's AST and fails if any function is absent from this document or
is not referenced by the C shim. The current baseline is 178 of 178 functions:
12 `mat.h`, 21 `mex.h`, and 145 `matrix.h` functions.

“Safe” means the operation is available through the ownership- and
lifetime-aware Rust API. “Alias” means a more general safe Rust operation has
the same capability. “Raw” means the exact operation is exported by
`matlas::raw`, but cannot soundly be made safe without additional invariants.
Raw pointer-adoption functions require MATLAB-allocated storage of exactly the
right layout and transfer its ownership to an `mxArray`. Non-trapping MEX error
and callback functions can bypass Rust destructors, so they also remain raw.

Deprecated identifiers that API 800 deliberately remaps to `*IsDeprecated`
(including the separate-complex `mxGetPi`/`mxSetPi` family) are not callable
published API declarations and are not counted. `mexFunction` is the MEX entry
point implemented by `mex_entrypoint!`, not an API function called by users.

## `mat.h` — 12/12

| Disposition | MATLAB functions | Rust API |
| --- | --- | --- |
| Safe | `matOpen`, `matClose` | `MatFile::open`, `create`, `create_with_format`, `close`, and `Drop` |
| Safe | `matGetVariable`, `matGetVariableInfo` | `MatFile::get`, `info` |
| Safe | `matGetNextVariable`, `matGetNextVariableInfo` | `Variables`, `VariableInfos` |
| Safe | `matPutVariable`, `matPutVariableAsGlobal`, `matDeleteVariable` | `MatFile::put`, `put_global`, `delete` |
| Safe | `matGetDir`, `matGetErrno`, `matGetFp` | `MatFile::variables`, `last_error`, `stream` |

## `mex.h` — 21/21

| Disposition | MATLAB functions | Rust API |
| --- | --- | --- |
| Safe | `mexCallMATLABWithTrap`, `mexEvalStringWithTrap` | `Matlab::call`, `WorkspaceScope::call`, `Matlab::eval` |
| Safe | `mexGetVariable`, `mexGetVariablePtr`, `mexPutVariable` | `Matlab::workspace_get`, `workspace_scope` / `WorkspaceScope::get`, `workspace_put` |
| Safe | `mexFunctionName`, `mexPrintf`, `mexWarnMsgIdAndTxt` | `Matlab::function_name`, `printf`, `warning` |
| Safe | `mexLock`, `mexUnlock`, `mexIsLocked` | RAII `Matlab::lock` / `ModuleLock`, and `is_locked` |
| Safe/internal | `mexAtExit`, `mexMakeArrayPersistent` | persistent-array registry |
| Raw | `mexMakeMemoryPersistent` | persisting arbitrary memory across invocations requires an application-specific owner and cleanup policy |
| Raw | `mexCallMATLAB`, `mexEvalString` | `raw::matrust_mex_call`, `raw::matrust_mex_eval`; non-trapping callbacks may skip Rust cleanup |
| Raw/internal | `mexErrMsgIdAndTxt`, `mexErrMsgTxt` | raw fixed-string shims; the entrypoint reports errors only after Rust frames return |
| Raw | `mexWarnMsgTxt`, `mexPrintAssertion` | raw fixed-string shims |
| Raw/deprecated | `mexIsGlobal` | exact raw shim; it always returns false in R2025a. Use `ArrayRef::is_from_global_workspace` (`mxIsFromGlobalWS`) instead |

## `matrix.h` — 145/145

| Disposition | MATLAB functions | Rust API |
| --- | --- | --- |
| Safe | `mxDestroyArray`, `mxDuplicateArray` | `OwnedArray::drop`, `OwnedArray::duplicate` |
| Safe | `mxMalloc`, `mxCalloc`, `mxRealloc`, `mxFree` | `Matlab::calloc`, `MxBuffer::resize`, `MxBuffer::drop`; exact raw shims are also exported |
| Safe | `mxGetM`, `mxGetN`, `mxGetNumberOfElements`, `mxGetNumberOfDimensions`, `mxGetDimensions`, `mxGetClassID`, `mxGetClassName`, `mxGetElementSize`, `mxGetUserBits` | `ArrayRef` metadata methods |
| Safe | `mxCalcSingleSubscript` | `ArrayRef::linear_index`; exact permissive native behavior is also raw |
| Safe | `mxIsNumeric`, `mxIsCell`, `mxIsLogical`, `mxIsChar`, `mxIsStruct`, `mxIsSparse`, `mxIsComplex`, `mxIsEmpty`, `mxIsScalar`, `mxIsObject`, `mxIsOpaque`, `mxIsFunctionHandle`, `mxIsClass`, `mxIsFromGlobalWS` | `ArrayRef::is_*` methods |
| Alias | `mxIsDouble`, `mxIsSingle`, `mxIsInt8`, `mxIsUint8`, `mxIsInt16`, `mxIsUint16`, `mxIsInt32`, `mxIsUint32`, `mxIsInt64`, `mxIsUint64` | `ArrayRef::class` plus `Class`; exact predicates are raw |
| Alias | `mxIsLogicalScalar`, `mxIsLogicalScalarTrue` | `is_logical`, `is_scalar`, and `logicals`; exact predicates are raw |
| Safe | `mxGetScalar`, `mxGetData`, `mxGetChars`, `mxGetLogicals` | `ArrayRef::scalar`, `data`, `chars`, `logicals`, including sparse value accessors |
| Alias | `mxGetDoubles`, `mxGetSingles`, `mxGetInt8s`, `mxGetUint8s`, `mxGetInt16s`, `mxGetUint16s`, `mxGetInt32s`, `mxGetUint32s`, `mxGetInt64s`, `mxGetUint64s` | `ArrayRef::data::<T>`; exact getters are raw |
| Alias | `mxGetComplexDoubles`, `mxGetComplexSingles`, `mxGetComplexInt8s`, `mxGetComplexUint8s`, `mxGetComplexInt16s`, `mxGetComplexUint16s`, `mxGetComplexInt32s`, `mxGetComplexUint32s`, `mxGetComplexInt64s`, `mxGetComplexUint64s` | `ArrayRef::data::<Complex<T>>`; exact getters are raw |
| Alias | `mxGetPr` | `ArrayRef::data::<f64>`; exact compatibility getter is raw |
| Safe | `mxCreateNumericArray`, `mxCreateUninitNumericArray` | `Matlab::numeric`, `uninit_numeric` |
| Alias | `mxCreateNumericMatrix`, `mxCreateUninitNumericMatrix`, `mxCreateDoubleMatrix`, `mxCreateDoubleScalar` | the same constructors with `[m, n]`, or `Matlab::scalar`; exact constructors are raw |
| Safe | `mxCreateLogicalArray`, `mxCreateSparseLogicalMatrix` | `Matlab::logical`, `logical_sparse` |
| Alias | `mxCreateLogicalMatrix`, `mxCreateLogicalScalar` | `Matlab::logical` with `[m, n]`; exact constructors are raw |
| Safe | `mxCreateCharArray`, `mxCreateString`, `mxCreateStringFromNChars` | UTF-16-safe `Matlab::char_array` and `string`; exact locale-dependent constructors are raw |
| Safe/raw | `mxCreateCharMatrixFromStrings` | UTF-16-safe `Matlab::char_matrix`; the exact locale-dependent constructor is raw |
| Safe | `mxArrayToString`, `mxArrayToUTF8String`, `mxGetString`, `mxGetNChars` | `ArrayRef::to_local`, `to_utf8`, `chars`, `string`; exact shims are raw |
| Safe | `mxCreateCellArray`, `mxCreateStructArray` | `Matlab::cell`, `structure` |
| Alias | `mxCreateCellMatrix`, `mxCreateStructMatrix` | the same constructors with `[m, n]`; exact constructors are raw |
| Safe | `mxCreateSparse`, `mxGetNzmax`, `mxGetIr`, `mxGetJc` | `Matlab::sparse`, `ArrayRef::sparse_indices`, `sparse_data` |
| Safe | `mxGetCell`, `mxSetCell` | borrowed `cell`/`cell_mut` and ownership-transferring `replace_cell` |
| Safe | `mxGetNumberOfFields`, `mxGetFieldNameByNumber`, `mxGetFieldNumber`, `mxGetFieldByNumber`, `mxAddField`, `mxRemoveField`, `mxSetFieldByNumber` | safe struct access and replacement methods |
| Alias | `mxGetField`, `mxSetField` | name-based `field` and `replace_field`; exact raw shims are exported |
| Safe | `mxGetProperty`, `mxSetProperty` | `Matlab::property`, `OwnedArray::property`, `ArrayMut::set_property` |
| Safe | `mxSetDimensions`, `mxSetFromGlobalWS`, `mxSetUserBits`, `mxMakeArrayReal`, `mxMakeArrayComplex` | validated `reshape` and explicit owned-array mutation methods |
| Raw | `mxSetM`, `mxSetN`, `mxSetClassName` | can invalidate shape/class invariants; `reshape` is the safe alternative |
| Raw | `mxSetData`, `mxSetPr`, `mxSetDoubles`, `mxSetSingles`, `mxSetInt8s`, `mxSetUint8s`, `mxSetInt16s`, `mxSetUint16s`, `mxSetInt32s`, `mxSetUint32s`, `mxSetInt64s`, `mxSetUint64s` | pointer ownership adoption cannot be inferred by Rust |
| Raw | `mxSetComplexDoubles`, `mxSetComplexSingles`, `mxSetComplexInt8s`, `mxSetComplexUint8s`, `mxSetComplexInt16s`, `mxSetComplexUint16s`, `mxSetComplexInt32s`, `mxSetComplexUint32s`, `mxSetComplexInt64s`, `mxSetComplexUint64s` | same ownership-transfer constraint for interleaved complex storage |
| Raw | `mxSetIr`, `mxSetJc`, `mxSetNzmax` | sparse CSC pointer/capacity adoption must remain mutually consistent |
| Safe | `mxGetEps`, `mxGetInf`, `mxGetNaN`, `mxIsFinite`, `mxIsInf`, `mxIsNaN` | crate-level numeric helpers |

## Raw safety contract

The `raw` module intentionally does not construct `OwnedArray`, `ArrayRef`, or
`WorkspaceValue`. A caller must uphold all MATLAB requirements: correct API-800
layout, main-thread execution, valid pointers and lengths, exclusive mutation,
allocator pairing, ownership transfer, callback invalidation, and the fact that
MATLAB error functions may not unwind Rust frames. Safe code should never mix a
raw ownership transfer with a live safe owner or borrow of the same allocation.
