#define MW_NEEDS_VERSION_H
#include "mex.h"
#include <stddef.h>

#ifdef _WIN32
#define RUSTMAT_EXPORT __declspec(dllexport)
#else
#define RUSTMAT_EXPORT __attribute__((visibility("default")))
#endif

extern int rustmat_mex_dispatch(int, mxArray **, int, const mxArray **,
                               char *, size_t, char *, size_t);

/* A reference from the consumer pulls this separate object from the archive. */
void rustmat_mex_anchor(void) {}

RUSTMAT_EXPORT void mexfilerequiredapiversion(unsigned int *release,
                                             unsigned int *api) {
    *release = RUSTMAT_BUILD_RELEASE;
    *api = MX_TARGET_API_VER;
}

RUSTMAT_EXPORT void mexFunction(int nlhs, mxArray **plhs,
                                int nrhs, const mxArray **prhs) {
    char id[256] = {0};
    char message[8192] = {0};
    int failed = rustmat_mex_dispatch(nlhs, plhs, nrhs, prhs,
                                     id, sizeof(id), message, sizeof(message));
    /* MATLAB may throw/unwind here. All Rust frames have already returned. */
    if (failed) mexErrMsgIdAndTxt(id, "%s", message);
}
