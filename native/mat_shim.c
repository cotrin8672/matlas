#include "mex.h"
#include "mat.h"
#include <stdint.h>

/* MATLAB's versioned ABI and ownership rules stay behind non-variadic calls. */

void matrust_array_destroy(mxArray *value) { mxDestroyArray(value); }
mxArray *matrust_array_duplicate(const mxArray *value) { return mxDuplicateArray(value); }
size_t matrust_array_m(const mxArray *value) { return mxGetM(value); }
size_t matrust_array_n(const mxArray *value) { return mxGetN(value); }
size_t matrust_array_numel(const mxArray *value) { return mxGetNumberOfElements(value); }
size_t matrust_array_ndims(const mxArray *value) { return mxGetNumberOfDimensions(value); }
const size_t *matrust_array_dims(const mxArray *value) { return mxGetDimensions(value); }
int matrust_array_class(const mxArray *value) { return (int)mxGetClassID(value); }
const char *matrust_array_class_name(const mxArray *value) { return mxGetClassName(value); }
size_t matrust_array_element_size(const mxArray *value) { return mxGetElementSize(value); }
int matrust_array_is_numeric(const mxArray *value) { return mxIsNumeric(value); }
int matrust_array_is_cell(const mxArray *value) { return mxIsCell(value); }
int matrust_array_is_logical(const mxArray *value) { return mxIsLogical(value); }
int matrust_array_is_char(const mxArray *value) { return mxIsChar(value); }
int matrust_array_is_struct(const mxArray *value) { return mxIsStruct(value); }
int matrust_array_is_sparse(const mxArray *value) { return mxIsSparse(value); }
int matrust_array_is_complex(const mxArray *value) { return mxIsComplex(value); }
int matrust_array_is_empty(const mxArray *value) { return mxIsEmpty(value); }
int matrust_array_is_scalar(const mxArray *value) { return mxIsScalar(value); }
int matrust_array_is_object(const mxArray *value) { return mxIsObject(value); }
int matrust_array_is_opaque(const mxArray *value) { return mxIsOpaque(value); }
int matrust_array_is_function(const mxArray *value) { return mxIsFunctionHandle(value); }
int matrust_array_is_class(const mxArray *value, const char *name) { return mxIsClass(value, name); }
int matrust_array_is_global(const mxArray *value) { return mxIsFromGlobalWS(value); }
void matrust_array_set_global(mxArray *value, int global) { mxSetFromGlobalWS(value, global != 0); }
unsigned char matrust_array_user_bits(const mxArray *value) { return (unsigned char)mxGetUserBits(value); }
void matrust_array_set_user_bits(mxArray *value, unsigned char bits) { mxSetUserBits(value, bits); }
double matrust_array_scalar(const mxArray *value) { return mxGetScalar(value); }
void *matrust_array_data(const mxArray *value) { return mxGetData(value); }
const uint16_t *matrust_array_chars(const mxArray *value) { return (const uint16_t *)mxGetChars(value); }
const unsigned char *matrust_array_logicals(const mxArray *value) { return (const unsigned char *)mxGetLogicals(value); }

/* One-to-one raw coverage for the published Matrix API.  The safe Rust layer
 * normally folds these aliases into typed, shape-checked operations. */
size_t matrust_mx_calc_single_subscript(const mxArray *value, size_t count, const size_t *subscripts) {
    return mxCalcSingleSubscript(value, count, subscripts);
}
mxArray *matrust_mx_create_cell_matrix(size_t m, size_t n) { return mxCreateCellMatrix(m, n); }
mxArray *matrust_mx_create_char_matrix_from_strings(size_t m, const char **strings) {
    return mxCreateCharMatrixFromStrings(m, strings);
}
mxArray *matrust_mx_create_double_matrix(size_t m, size_t n, int complex) {
    return mxCreateDoubleMatrix(m, n, complex ? mxCOMPLEX : mxREAL);
}
mxArray *matrust_mx_create_double_scalar(double value) { return mxCreateDoubleScalar(value); }
mxArray *matrust_mx_create_logical_matrix(size_t m, size_t n) { return mxCreateLogicalMatrix(m, n); }
mxArray *matrust_mx_create_logical_scalar(int value) { return mxCreateLogicalScalar(value != 0); }
mxArray *matrust_mx_create_numeric_matrix(size_t m, size_t n, int class_id, int complex) {
    return mxCreateNumericMatrix(m, n, (mxClassID)class_id, complex ? mxCOMPLEX : mxREAL);
}
mxArray *matrust_mx_create_struct_matrix(size_t m, size_t n, int fields, const char **names) {
    return mxCreateStructMatrix(m, n, fields, names);
}
mxArray *matrust_mx_create_uninit_numeric_matrix(size_t m, size_t n, int class_id, int complex) {
    return mxCreateUninitNumericMatrix(m, n, (mxClassID)class_id, complex ? mxCOMPLEX : mxREAL);
}
void *matrust_mx_get_doubles(const mxArray *value) { return mxGetDoubles(value); }
void *matrust_mx_get_complex_doubles(const mxArray *value) { return mxGetComplexDoubles(value); }
void *matrust_mx_get_singles(const mxArray *value) { return mxGetSingles(value); }
void *matrust_mx_get_complex_singles(const mxArray *value) { return mxGetComplexSingles(value); }
void *matrust_mx_get_int8s(const mxArray *value) { return mxGetInt8s(value); }
void *matrust_mx_get_complex_int8s(const mxArray *value) { return mxGetComplexInt8s(value); }
void *matrust_mx_get_uint8s(const mxArray *value) { return mxGetUint8s(value); }
void *matrust_mx_get_complex_uint8s(const mxArray *value) { return mxGetComplexUint8s(value); }
void *matrust_mx_get_int16s(const mxArray *value) { return mxGetInt16s(value); }
void *matrust_mx_get_complex_int16s(const mxArray *value) { return mxGetComplexInt16s(value); }
void *matrust_mx_get_uint16s(const mxArray *value) { return mxGetUint16s(value); }
void *matrust_mx_get_complex_uint16s(const mxArray *value) { return mxGetComplexUint16s(value); }
void *matrust_mx_get_int32s(const mxArray *value) { return mxGetInt32s(value); }
void *matrust_mx_get_complex_int32s(const mxArray *value) { return mxGetComplexInt32s(value); }
void *matrust_mx_get_uint32s(const mxArray *value) { return mxGetUint32s(value); }
void *matrust_mx_get_complex_uint32s(const mxArray *value) { return mxGetComplexUint32s(value); }
void *matrust_mx_get_int64s(const mxArray *value) { return mxGetInt64s(value); }
void *matrust_mx_get_complex_int64s(const mxArray *value) { return mxGetComplexInt64s(value); }
void *matrust_mx_get_uint64s(const mxArray *value) { return mxGetUint64s(value); }
void *matrust_mx_get_complex_uint64s(const mxArray *value) { return mxGetComplexUint64s(value); }
double *matrust_mx_get_pr(const mxArray *value) { return mxGetPr(value); }
mxArray *matrust_mx_get_field(const mxArray *value, size_t index, const char *name) {
    return mxGetField(value, index, name);
}
int matrust_mx_is_double(const mxArray *value) { return mxIsDouble(value); }
int matrust_mx_is_single(const mxArray *value) { return mxIsSingle(value); }
int matrust_mx_is_int8(const mxArray *value) { return mxIsInt8(value); }
int matrust_mx_is_uint8(const mxArray *value) { return mxIsUint8(value); }
int matrust_mx_is_int16(const mxArray *value) { return mxIsInt16(value); }
int matrust_mx_is_uint16(const mxArray *value) { return mxIsUint16(value); }
int matrust_mx_is_int32(const mxArray *value) { return mxIsInt32(value); }
int matrust_mx_is_uint32(const mxArray *value) { return mxIsUint32(value); }
int matrust_mx_is_int64(const mxArray *value) { return mxIsInt64(value); }
int matrust_mx_is_uint64(const mxArray *value) { return mxIsUint64(value); }
int matrust_mx_is_logical_scalar(const mxArray *value) { return mxIsLogicalScalar(value); }
int matrust_mx_is_logical_scalar_true(const mxArray *value) { return mxIsLogicalScalarTrue(value); }
int matrust_mx_set_class_name(mxArray *value, const char *name) { return mxSetClassName(value, name); }
void matrust_mx_set_data(mxArray *value, void *data) { mxSetData(value, data); }
void matrust_mx_set_doubles(mxArray *value, void *data) { mxSetDoubles(value, (mxDouble *)data); }
void matrust_mx_set_complex_doubles(mxArray *value, void *data) { mxSetComplexDoubles(value, (mxComplexDouble *)data); }
void matrust_mx_set_singles(mxArray *value, void *data) { mxSetSingles(value, (mxSingle *)data); }
void matrust_mx_set_complex_singles(mxArray *value, void *data) { mxSetComplexSingles(value, (mxComplexSingle *)data); }
void matrust_mx_set_int8s(mxArray *value, void *data) { mxSetInt8s(value, (mxInt8 *)data); }
void matrust_mx_set_complex_int8s(mxArray *value, void *data) { mxSetComplexInt8s(value, (mxComplexInt8 *)data); }
void matrust_mx_set_uint8s(mxArray *value, void *data) { mxSetUint8s(value, (mxUint8 *)data); }
void matrust_mx_set_complex_uint8s(mxArray *value, void *data) { mxSetComplexUint8s(value, (mxComplexUint8 *)data); }
void matrust_mx_set_int16s(mxArray *value, void *data) { mxSetInt16s(value, (mxInt16 *)data); }
void matrust_mx_set_complex_int16s(mxArray *value, void *data) { mxSetComplexInt16s(value, (mxComplexInt16 *)data); }
void matrust_mx_set_uint16s(mxArray *value, void *data) { mxSetUint16s(value, (mxUint16 *)data); }
void matrust_mx_set_complex_uint16s(mxArray *value, void *data) { mxSetComplexUint16s(value, (mxComplexUint16 *)data); }
void matrust_mx_set_int32s(mxArray *value, void *data) { mxSetInt32s(value, (mxInt32 *)data); }
void matrust_mx_set_complex_int32s(mxArray *value, void *data) { mxSetComplexInt32s(value, (mxComplexInt32 *)data); }
void matrust_mx_set_uint32s(mxArray *value, void *data) { mxSetUint32s(value, (mxUint32 *)data); }
void matrust_mx_set_complex_uint32s(mxArray *value, void *data) { mxSetComplexUint32s(value, (mxComplexUint32 *)data); }
void matrust_mx_set_int64s(mxArray *value, void *data) { mxSetInt64s(value, (mxInt64 *)data); }
void matrust_mx_set_complex_int64s(mxArray *value, void *data) { mxSetComplexInt64s(value, (mxComplexInt64 *)data); }
void matrust_mx_set_uint64s(mxArray *value, void *data) { mxSetUint64s(value, (mxUint64 *)data); }
void matrust_mx_set_complex_uint64s(mxArray *value, void *data) { mxSetComplexUint64s(value, (mxComplexUint64 *)data); }
void matrust_mx_set_field(mxArray *value, size_t index, const char *name, mxArray *child) {
    mxSetField(value, index, name, child);
}
void matrust_mx_set_ir(mxArray *value, size_t *data) { mxSetIr(value, data); }
void matrust_mx_set_jc(mxArray *value, size_t *data) { mxSetJc(value, data); }
void matrust_mx_set_m(mxArray *value, size_t rows) { mxSetM(value, rows); }
void matrust_mx_set_n(mxArray *value, size_t columns) { mxSetN(value, columns); }
void matrust_mx_set_nzmax(mxArray *value, size_t nzmax) { mxSetNzmax(value, nzmax); }
void matrust_mx_set_pr(mxArray *value, double *data) { mxSetPr(value, data); }

mxArray *matrust_create_numeric(size_t ndims, const size_t *dims, int class_id, int complex) {
    return mxCreateNumericArray(ndims, dims, (mxClassID)class_id, complex ? mxCOMPLEX : mxREAL);
}
mxArray *matrust_create_uninit_numeric(size_t ndims, const size_t *dims, int class_id, int complex) {
    return mxCreateUninitNumericArray(ndims, (size_t *)dims, (mxClassID)class_id, complex ? mxCOMPLEX : mxREAL);
}
mxArray *matrust_create_logical(size_t ndims, const size_t *dims) { return mxCreateLogicalArray(ndims, dims); }
mxArray *matrust_create_char(size_t ndims, const size_t *dims) { return mxCreateCharArray(ndims, dims); }
mxArray *matrust_create_string(const char *text) { return mxCreateString(text); }
mxArray *matrust_create_string_n(const char *text, size_t count) { return mxCreateStringFromNChars(text, count); }
mxArray *matrust_create_cell(size_t ndims, const size_t *dims) { return mxCreateCellArray(ndims, dims); }
mxArray *matrust_create_struct(size_t ndims, const size_t *dims, int fields, const char **names) {
    return mxCreateStructArray(ndims, dims, fields, names);
}
mxArray *matrust_create_sparse(size_t m, size_t n, size_t nzmax, int logical, int complex) {
    if (logical) return mxCreateSparseLogicalMatrix(m, n, nzmax);
    return mxCreateSparse(m, n, nzmax, complex ? mxCOMPLEX : mxREAL);
}
int matrust_array_set_dimensions(mxArray *value, const size_t *dims, size_t ndims) {
    return mxSetDimensions(value, dims, ndims);
}
int matrust_array_make_real(mxArray *value) { return mxMakeArrayReal(value); }
int matrust_array_make_complex(mxArray *value) { return mxMakeArrayComplex(value); }

mxArray *matrust_cell_get(const mxArray *value, size_t index) { return mxGetCell(value, index); }
void matrust_cell_set(mxArray *value, size_t index, mxArray *child) { mxSetCell(value, index, child); }
int matrust_struct_field_count(const mxArray *value) { return mxGetNumberOfFields(value); }
const char *matrust_struct_field_name(const mxArray *value, int field) { return mxGetFieldNameByNumber(value, field); }
int matrust_struct_field_number(const mxArray *value, const char *name) { return mxGetFieldNumber(value, name); }
int matrust_struct_add_field(mxArray *value, const char *name) { return mxAddField(value, name); }
void matrust_struct_remove_field(mxArray *value, int field) { mxRemoveField(value, field); }
mxArray *matrust_struct_get(const mxArray *value, size_t index, int field) { return mxGetFieldByNumber(value, index, field); }
void matrust_struct_set(mxArray *value, size_t index, int field, mxArray *child) {
    mxSetFieldByNumber(value, index, field, child);
}

size_t matrust_sparse_nzmax(const mxArray *value) { return mxGetNzmax(value); }
const size_t *matrust_sparse_ir(const mxArray *value) { return mxGetIr(value); }
const size_t *matrust_sparse_jc(const mxArray *value) { return mxGetJc(value); }

char *matrust_array_to_utf8(const mxArray *value) { return mxArrayToUTF8String(value); }
char *matrust_array_to_local(const mxArray *value) { return mxArrayToString(value); }
int matrust_array_get_string(const mxArray *value, char *out, size_t length) { return mxGetString(value, out, length); }
void matrust_array_get_nchars(const mxArray *value, char *out, size_t count) { mxGetNChars(value, out, count); }
void *matrust_malloc(size_t bytes) { return mxMalloc(bytes); }
void *matrust_calloc(size_t count, size_t bytes) { return mxCalloc(count, bytes); }
void *matrust_realloc(void *ptr, size_t bytes) { return mxRealloc(ptr, bytes); }
void matrust_free(void *ptr) { mxFree(ptr); }
void matrust_make_memory_persistent(void *ptr) { mexMakeMemoryPersistent(ptr); }

mxArray *matrust_call_with_trap(int nlhs, mxArray **plhs, int nrhs, mxArray **prhs, const char *name) {
    return mexCallMATLABWithTrap(nlhs, plhs, nrhs, prhs, name);
}
mxArray *matrust_eval_with_trap(const char *command) { return mexEvalStringWithTrap(command); }
int matrust_mex_call(int nlhs, mxArray **plhs, int nrhs, mxArray **prhs, const char *name) {
    return mexCallMATLAB(nlhs, plhs, nrhs, prhs, name);
}
int matrust_mex_eval(const char *command) { return mexEvalString(command); }
void matrust_mex_error(const char *id, const char *text) { mexErrMsgIdAndTxt(id, "%s", text); }
void matrust_mex_error_text(const char *text) { mexErrMsgTxt(text); }
void matrust_mex_warning_text(const char *text) { mexWarnMsgTxt(text); }
void matrust_mex_print_assertion(const char *test, const char *file, int line, const char *message) {
    mexPrintAssertion(test, file, line, message);
}
int matrust_mex_is_global(const mxArray *value) { return mexIsGlobal(value); }
mxArray *matrust_workspace_get(const char *space, const char *name) { return mexGetVariable(space, name); }
const mxArray *matrust_workspace_borrow(const char *space, const char *name) { return mexGetVariablePtr(space, name); }
int matrust_workspace_put(const char *space, const char *name, const mxArray *value) {
    return mexPutVariable(space, name, value);
}
mxArray *matrust_property_get(const mxArray *object, size_t index, const char *name) {
    return mxGetProperty(object, index, name);
}
void matrust_property_set(mxArray *object, size_t index, const char *name, const mxArray *value) {
    mxSetProperty(object, index, name, value);
}
const char *matrust_function_name(void) { return mexFunctionName(); }
int matrust_printf(const char *text) { return mexPrintf("%s", text); }
void matrust_warning(const char *id, const char *text) { mexWarnMsgIdAndTxt(id, "%s", text); }
void matrust_lock(void) { mexLock(); }
void matrust_unlock(void) { mexUnlock(); }
int matrust_is_locked(void) { return mexIsLocked(); }
void matrust_make_array_persistent(mxArray *value) { mexMakeArrayPersistent(value); }
int matrust_at_exit(void (*callback)(void)) { return mexAtExit(callback); }

MATFile *matrust_mat_open(const char *path, const char *mode) { return matOpen(path, mode); }
int matrust_mat_close(MATFile *file) { return matClose(file); }
int matrust_mat_error(MATFile *file) { return matGetErrno(file); }
int matrust_mat_put(MATFile *file, const char *name, const mxArray *value, int global, int *error) {
    int status = global ? matPutVariableAsGlobal(file, name, value) : matPutVariable(file, name, value);
    *error = status ? matGetErrno(file) : 0;
    return status;
}
mxArray *matrust_mat_get(MATFile *file, const char *name, int info, int *error) {
    mxArray *value = info ? matGetVariableInfo(file, name) : matGetVariable(file, name);
    *error = value ? 0 : matGetErrno(file);
    return value;
}
int matrust_mat_delete(MATFile *file, const char *name, int *error) {
    int status = matDeleteVariable(file, name);
    *error = status ? matGetErrno(file) : 0;
    return status;
}
char **matrust_mat_directory(MATFile *file, int *count, int *error) {
    char **names = matGetDir(file, count);
    *error = *count < 0 ? matGetErrno(file) : 0;
    return names;
}
mxArray *matrust_mat_next(MATFile *file, const char **name, int info, int *error) {
    mxArray *value = info ? matGetNextVariableInfo(file, name) : matGetNextVariable(file, name);
    *error = value ? 0 : matGetErrno(file);
    return value;
}
FILE *matrust_mat_stream(MATFile *file) { return matGetFp(file); }
int matrust_stream_eof(FILE *file) { return feof(file); }
int matrust_stream_error(FILE *file) { return ferror(file); }
void matrust_stream_clear(FILE *file) { clearerr(file); }
int64_t matrust_stream_position(FILE *file) {
#ifdef _WIN32
    return _ftelli64(file);
#else
    return (int64_t)ftello(file);
#endif
}

double matrust_eps(void) { return mxGetEps(); }
double matrust_inf(void) { return mxGetInf(); }
double matrust_nan(void) { return mxGetNaN(); }
int matrust_is_finite(double value) { return mxIsFinite(value); }
int matrust_is_inf(double value) { return mxIsInf(value); }
int matrust_is_nan(double value) { return mxIsNaN(value); }
