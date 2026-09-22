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
