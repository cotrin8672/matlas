#define MW_NEEDS_VERSION_H
#include "mex.h"
#include <stddef.h>

#ifdef _WIN32
#define MATRUST_EXPORT __declspec(dllexport)
#else
#define MATRUST_EXPORT __attribute__((visibility("default")))
#endif

extern int matrust_mex_dispatch(int, mxArray **, int, const mxArray **,
                               char *, size_t, char *, size_t);

/* A reference from the consumer pulls this separate object from the archive. */
void matrust_mex_anchor(void) {}

MATRUST_EXPORT void mexfilerequiredapiversion(unsigned int *release,
                                             unsigned int *api) {
    *release = MATRUST_BUILD_RELEASE;
    *api = MX_TARGET_API_VER;
}

MATRUST_EXPORT void mexFunction(int nlhs, mxArray **plhs,
                                int nrhs, const mxArray **prhs) {
    char id[256] = {0};
    char message[8192] = {0};
    int failed = matrust_mex_dispatch(nlhs, plhs, nrhs, prhs,
                                     id, sizeof(id), message, sizeof(message));
    /* MATLAB may throw/unwind here. All Rust frames have already returned. */
    if (failed) mexErrMsgIdAndTxt(id, "%s", message);
}
