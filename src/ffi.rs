use rustmex::mxArray;
use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct RawFile {
    _private: [u8; 0],
}

extern "C" {
    pub fn rustmat_open(path: *const c_char, mode: *const c_char) -> *mut RawFile;
    #[link_name = "rustmat_close"]
    fn native_close(file: *mut RawFile) -> c_int;
    pub fn rustmat_error(file: *mut RawFile) -> c_int;
    pub fn rustmat_put(
        file: *mut RawFile,
        name: *const c_char,
        value: *const mxArray,
        global: c_int,
        error: *mut c_int,
    ) -> c_int;
    pub fn rustmat_get(
        file: *mut RawFile,
        name: *const c_char,
        info: c_int,
        error: *mut c_int,
    ) -> *mut mxArray;
    pub fn rustmat_delete(file: *mut RawFile, name: *const c_char, error: *mut c_int) -> c_int;
    pub fn rustmat_directory(
        file: *mut RawFile,
        count: *mut c_int,
        error: *mut c_int,
    ) -> *mut *mut c_char;
    pub fn rustmat_next(
        file: *mut RawFile,
        name: *mut *const c_char,
        info: c_int,
        error: *mut c_int,
    ) -> *mut mxArray;
    pub fn rustmat_free(ptr: *mut c_void);
    pub fn rustmat_destroy(value: *mut mxArray);
    pub fn rustmat_dimensions(value: *const mxArray, count: *mut usize) -> *const usize;
    pub fn rustmat_numel(value: *const mxArray) -> usize;
    pub fn rustmat_class_id(value: *const mxArray) -> u32;
    pub fn rustmat_class_name(value: *const mxArray) -> *const c_char;
    pub fn rustmat_flags(value: *const mxArray) -> c_int;
    pub fn rustmat_fields(value: *const mxArray) -> c_int;
    pub fn rustmat_field_name(value: *const mxArray, field: c_int) -> *const c_char;
    pub fn rustmat_field(
        value: *const mxArray,
        index: usize,
        name: *const c_char,
    ) -> *const mxArray;
    pub fn rustmat_cell(value: *const mxArray, index: usize) -> *const mxArray;
    pub fn rustmat_stream(file: *mut RawFile) -> *mut c_void;
    pub fn rustmat_stream_eof(file: *mut c_void) -> c_int;
    pub fn rustmat_stream_error(file: *mut c_void) -> c_int;
    pub fn rustmat_stream_clear(file: *mut c_void);
    pub fn rustmat_stream_position(file: *mut c_void) -> i64;
}

// A private test-only seam exercises finalization failures without filling a
// disk or passing a fake handle to MATLAB. Production always calls native_close.
pub(crate) unsafe fn rustmat_close(file: *mut RawFile) -> c_int {
    #[cfg(test)]
    if let Some(status) = CLOSE_PROBE.with(|probe| {
        probe.get().map(|(status, count)| {
            probe.set(Some((status, count + 1)));
            status
        })
    }) {
        return status;
    }
    // SAFETY: caller supplies the uniquely owned, live native handle.
    unsafe { native_close(file) }
}

#[cfg(test)]
thread_local! {
    pub(crate) static CLOSE_PROBE: std::cell::Cell<Option<(i32, usize)>> = const { std::cell::Cell::new(None) };
}

/// Owns either a full array or a metadata-only header, never exposed as mxArray.
pub(crate) struct Array(pub std::ptr::NonNull<mxArray>);
impl Drop for Array {
    fn drop(&mut self) {
        // SAFETY: this unique allocation came from a matGet* function.
        unsafe { rustmat_destroy(self.0.as_ptr()) }
    }
}
