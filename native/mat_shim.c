#include "mat.h"
#include <stdint.h>

/* Only this translation unit knows the versioned MATLAB symbol names. */
MATFile *rustmat_open(const char *path, const char *mode) { return matOpen(path, mode); }
int rustmat_close(MATFile *file) { return matClose(file); }
int rustmat_error(MATFile *file) { return matGetErrno(file); }
int rustmat_put(MATFile *file, const char *name, const mxArray *value, int global, int *error) {
    int status = global ? matPutVariableAsGlobal(file, name, value) : matPutVariable(file, name, value);
    *error = status ? matGetErrno(file) : 0;
    return status;
}
mxArray *rustmat_get(MATFile *file, const char *name, int info, int *error) {
    mxArray *value = info ? matGetVariableInfo(file, name) : matGetVariable(file, name);
    *error = value ? 0 : matGetErrno(file);
    return value;
}
int rustmat_delete(MATFile *file, const char *name, int *error) {
    int status = matDeleteVariable(file, name);
    *error = status ? matGetErrno(file) : 0;
    return status;
}
char **rustmat_directory(MATFile *file, int *count, int *error) {
    char **names = matGetDir(file, count);
    *error = *count < 0 ? matGetErrno(file) : 0;
    return names;
}
mxArray *rustmat_next(MATFile *file, const char **name, int info, int *error) {
    mxArray *value = info ? matGetNextVariableInfo(file, name) : matGetNextVariable(file, name);
    *error = value ? 0 : matGetErrno(file);
    return value;
}
void rustmat_free(void *ptr) { mxFree(ptr); }
void rustmat_destroy(mxArray *value) { mxDestroyArray(value); }
const size_t *rustmat_dimensions(const mxArray *value, size_t *count) {
    *count = mxGetNumberOfDimensions(value);
    return mxGetDimensions(value);
}
size_t rustmat_numel(const mxArray *value) { return mxGetNumberOfElements(value); }
unsigned int rustmat_class_id(const mxArray *value) { return (unsigned int)mxGetClassID(value); }
const char *rustmat_class_name(const mxArray *value) { return mxGetClassName(value); }
int rustmat_flags(const mxArray *value) {
    return (mxIsComplex(value) ? 1 : 0) | (mxIsSparse(value) ? 2 : 0) | (mxIsFromGlobalWS(value) ? 4 : 0);
}
int rustmat_fields(const mxArray *value) { return mxIsStruct(value) ? mxGetNumberOfFields(value) : 0; }
const char *rustmat_field_name(const mxArray *value, int field) { return mxGetFieldNameByNumber(value, field); }
const mxArray *rustmat_field(const mxArray *value, size_t index, const char *name) {
    return mxIsStruct(value) && index < mxGetNumberOfElements(value) ? mxGetField(value, index, name) : NULL;
}
const mxArray *rustmat_cell(const mxArray *value, size_t index) {
    return mxIsCell(value) && index < mxGetNumberOfElements(value) ? mxGetCell(value, index) : NULL;
}
FILE *rustmat_stream(MATFile *file) { return matGetFp(file); }
int rustmat_stream_eof(FILE *file) { return feof(file); }
int rustmat_stream_error(FILE *file) { return ferror(file); }
void rustmat_stream_clear(FILE *file) { clearerr(file); }
int64_t rustmat_stream_position(FILE *file) {
#ifdef _WIN32
    return _ftelli64(file);
#else
    return (int64_t)ftello(file);
#endif
}
