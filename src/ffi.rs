use std::ffi::{c_char, c_int, c_uchar, c_void};

#[repr(C)]
pub struct RawArray {
    _private: [u8; 0],
}

#[repr(C)]
pub struct RawFile {
    _private: [u8; 0],
}

extern "C" {
    pub fn matrust_array_destroy(value: *mut RawArray);
    pub fn matrust_array_duplicate(value: *const RawArray) -> *mut RawArray;
    pub fn matrust_array_m(value: *const RawArray) -> usize;
    pub fn matrust_array_n(value: *const RawArray) -> usize;
    pub fn matrust_array_numel(value: *const RawArray) -> usize;
    pub fn matrust_array_ndims(value: *const RawArray) -> usize;
    pub fn matrust_array_dims(value: *const RawArray) -> *const usize;
    pub fn matrust_array_class(value: *const RawArray) -> c_int;
    pub fn matrust_array_class_name(value: *const RawArray) -> *const c_char;
    pub fn matrust_array_element_size(value: *const RawArray) -> usize;
    pub fn matrust_array_is_numeric(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_cell(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_logical(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_char(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_struct(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_sparse(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_complex(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_empty(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_scalar(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_object(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_opaque(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_function(value: *const RawArray) -> c_int;
    pub fn matrust_array_is_class(value: *const RawArray, name: *const c_char) -> c_int;
    pub fn matrust_array_is_global(value: *const RawArray) -> c_int;
    pub fn matrust_array_set_global(value: *mut RawArray, global: c_int);
    pub fn matrust_array_user_bits(value: *const RawArray) -> c_uchar;
    pub fn matrust_array_set_user_bits(value: *mut RawArray, bits: c_uchar);
    pub fn matrust_array_scalar(value: *const RawArray) -> f64;
    pub fn matrust_array_data(value: *const RawArray) -> *mut c_void;
    pub fn matrust_array_chars(value: *const RawArray) -> *const u16;
    pub fn matrust_array_logicals(value: *const RawArray) -> *const u8;
    pub fn matrust_mx_calc_single_subscript(
        value: *const RawArray,
        count: usize,
        subscripts: *const usize,
    ) -> usize;
    pub fn matrust_mx_create_cell_matrix(m: usize, n: usize) -> *mut RawArray;
    pub fn matrust_mx_create_char_matrix_from_strings(
        m: usize,
        strings: *const *const c_char,
    ) -> *mut RawArray;
    pub fn matrust_mx_create_double_matrix(m: usize, n: usize, complex: c_int) -> *mut RawArray;
    pub fn matrust_mx_create_double_scalar(value: f64) -> *mut RawArray;
    pub fn matrust_mx_create_logical_matrix(m: usize, n: usize) -> *mut RawArray;
    pub fn matrust_mx_create_logical_scalar(value: c_int) -> *mut RawArray;
    pub fn matrust_mx_create_numeric_matrix(
        m: usize,
        n: usize,
        class_id: c_int,
        complex: c_int,
    ) -> *mut RawArray;
    pub fn matrust_mx_create_struct_matrix(
        m: usize,
        n: usize,
        fields: c_int,
        names: *const *const c_char,
    ) -> *mut RawArray;
    pub fn matrust_mx_create_uninit_numeric_matrix(
        m: usize,
        n: usize,
        class_id: c_int,
        complex: c_int,
    ) -> *mut RawArray;
    pub fn matrust_mx_get_doubles(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_doubles(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_singles(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_singles(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_int8s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_int8s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_uint8s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_uint8s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_int16s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_int16s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_uint16s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_uint16s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_int32s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_int32s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_uint32s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_uint32s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_int64s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_int64s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_uint64s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_complex_uint64s(value: *const RawArray) -> *mut c_void;
    pub fn matrust_mx_get_pr(value: *const RawArray) -> *mut f64;
    pub fn matrust_mx_get_field(
        value: *const RawArray,
        index: usize,
        name: *const c_char,
    ) -> *mut RawArray;
    pub fn matrust_mx_is_double(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_single(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_int8(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_uint8(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_int16(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_uint16(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_int32(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_uint32(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_int64(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_uint64(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_logical_scalar(value: *const RawArray) -> c_int;
    pub fn matrust_mx_is_logical_scalar_true(value: *const RawArray) -> c_int;
    pub fn matrust_mx_set_class_name(value: *mut RawArray, name: *const c_char) -> c_int;
    pub fn matrust_mx_set_data(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_doubles(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_doubles(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_singles(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_singles(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_int8s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_int8s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_uint8s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_uint8s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_int16s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_int16s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_uint16s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_uint16s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_int32s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_int32s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_uint32s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_uint32s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_int64s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_int64s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_uint64s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_complex_uint64s(value: *mut RawArray, data: *mut c_void);
    pub fn matrust_mx_set_field(
        value: *mut RawArray,
        index: usize,
        name: *const c_char,
        child: *mut RawArray,
    );
    pub fn matrust_mx_set_ir(value: *mut RawArray, data: *mut usize);
    pub fn matrust_mx_set_jc(value: *mut RawArray, data: *mut usize);
    pub fn matrust_mx_set_m(value: *mut RawArray, rows: usize);
    pub fn matrust_mx_set_n(value: *mut RawArray, columns: usize);
    pub fn matrust_mx_set_nzmax(value: *mut RawArray, nzmax: usize);
    pub fn matrust_mx_set_pr(value: *mut RawArray, data: *mut f64);
    pub fn matrust_create_numeric(
        ndims: usize,
        dims: *const usize,
        class_id: c_int,
        complex: c_int,
    ) -> *mut RawArray;
    pub fn matrust_create_uninit_numeric(
        ndims: usize,
        dims: *const usize,
        class_id: c_int,
        complex: c_int,
    ) -> *mut RawArray;
    pub fn matrust_create_logical(ndims: usize, dims: *const usize) -> *mut RawArray;
    pub fn matrust_create_char(ndims: usize, dims: *const usize) -> *mut RawArray;
    pub fn matrust_create_string(text: *const c_char) -> *mut RawArray;
    pub fn matrust_create_string_n(text: *const c_char, count: usize) -> *mut RawArray;
    pub fn matrust_create_cell(ndims: usize, dims: *const usize) -> *mut RawArray;
    pub fn matrust_create_struct(
        ndims: usize,
        dims: *const usize,
        fields: c_int,
        names: *const *const c_char,
    ) -> *mut RawArray;
    pub fn matrust_create_sparse(
        m: usize,
        n: usize,
        nzmax: usize,
        logical: c_int,
        complex: c_int,
    ) -> *mut RawArray;
    pub fn matrust_array_set_dimensions(
        value: *mut RawArray,
        dims: *const usize,
        ndims: usize,
    ) -> c_int;
    pub fn matrust_array_make_real(value: *mut RawArray) -> c_int;
    pub fn matrust_array_make_complex(value: *mut RawArray) -> c_int;
    pub fn matrust_cell_get(value: *const RawArray, index: usize) -> *mut RawArray;
    pub fn matrust_cell_set(value: *mut RawArray, index: usize, child: *mut RawArray);
    pub fn matrust_struct_field_count(value: *const RawArray) -> c_int;
    pub fn matrust_struct_field_name(value: *const RawArray, field: c_int) -> *const c_char;
    pub fn matrust_struct_field_number(value: *const RawArray, name: *const c_char) -> c_int;
    pub fn matrust_struct_add_field(value: *mut RawArray, name: *const c_char) -> c_int;
    pub fn matrust_struct_remove_field(value: *mut RawArray, field: c_int);
    pub fn matrust_struct_get(value: *const RawArray, index: usize, field: c_int) -> *mut RawArray;
    pub fn matrust_struct_set(
        value: *mut RawArray,
        index: usize,
        field: c_int,
        child: *mut RawArray,
    );
    pub fn matrust_sparse_nzmax(value: *const RawArray) -> usize;
    pub fn matrust_sparse_ir(value: *const RawArray) -> *const usize;
    pub fn matrust_sparse_jc(value: *const RawArray) -> *const usize;
    pub fn matrust_array_to_utf8(value: *const RawArray) -> *mut c_char;
    pub fn matrust_array_to_local(value: *const RawArray) -> *mut c_char;
    pub fn matrust_array_get_string(
        value: *const RawArray,
        out: *mut c_char,
        length: usize,
    ) -> c_int;
    pub fn matrust_array_get_nchars(value: *const RawArray, out: *mut c_char, count: usize);
    pub fn matrust_malloc(bytes: usize) -> *mut c_void;
    pub fn matrust_calloc(count: usize, bytes: usize) -> *mut c_void;
    pub fn matrust_realloc(ptr: *mut c_void, bytes: usize) -> *mut c_void;
    pub fn matrust_free(ptr: *mut c_void);
    pub fn matrust_make_memory_persistent(ptr: *mut c_void);
    pub fn matrust_call_with_trap(
        nlhs: c_int,
        plhs: *mut *mut RawArray,
        nrhs: c_int,
        prhs: *mut *mut RawArray,
        name: *const c_char,
    ) -> *mut RawArray;
    pub fn matrust_eval_with_trap(command: *const c_char) -> *mut RawArray;
    pub fn matrust_mex_call(
        nlhs: c_int,
        plhs: *mut *mut RawArray,
        nrhs: c_int,
        prhs: *mut *mut RawArray,
        name: *const c_char,
    ) -> c_int;
    pub fn matrust_mex_eval(command: *const c_char) -> c_int;
    pub fn matrust_mex_error(id: *const c_char, text: *const c_char);
    pub fn matrust_mex_error_text(text: *const c_char);
    pub fn matrust_mex_warning_text(text: *const c_char);
    pub fn matrust_mex_print_assertion(
        test: *const c_char,
        file: *const c_char,
        line: c_int,
        message: *const c_char,
    );
    pub fn matrust_mex_is_global(value: *const RawArray) -> c_int;
    pub fn matrust_workspace_get(space: *const c_char, name: *const c_char) -> *mut RawArray;
    pub fn matrust_workspace_borrow(space: *const c_char, name: *const c_char) -> *const RawArray;
    pub fn matrust_workspace_put(
        space: *const c_char,
        name: *const c_char,
        value: *const RawArray,
    ) -> c_int;
    pub fn matrust_property_get(
        object: *const RawArray,
        index: usize,
        name: *const c_char,
    ) -> *mut RawArray;
    pub fn matrust_property_set(
        object: *mut RawArray,
        index: usize,
        name: *const c_char,
        value: *const RawArray,
    );
    pub fn matrust_function_name() -> *const c_char;
    pub fn matrust_printf(text: *const c_char) -> c_int;
    pub fn matrust_warning(id: *const c_char, text: *const c_char);
    pub fn matrust_lock();
    pub fn matrust_unlock();
    pub fn matrust_is_locked() -> c_int;
    pub fn matrust_make_array_persistent(value: *mut RawArray);
    pub fn matrust_at_exit(callback: unsafe extern "C" fn()) -> c_int;

    pub fn matrust_mat_open(path: *const c_char, mode: *const c_char) -> *mut RawFile;
    #[link_name = "matrust_mat_close"]
    pub fn matrust_mat_close_raw(file: *mut RawFile) -> c_int;
    pub fn matrust_mat_error(file: *mut RawFile) -> c_int;
    pub fn matrust_mat_put(
        file: *mut RawFile,
        name: *const c_char,
        value: *const RawArray,
        global: c_int,
        error: *mut c_int,
    ) -> c_int;
    pub fn matrust_mat_get(
        file: *mut RawFile,
        name: *const c_char,
        info: c_int,
        error: *mut c_int,
    ) -> *mut RawArray;
    pub fn matrust_mat_delete(file: *mut RawFile, name: *const c_char, error: *mut c_int) -> c_int;
    pub fn matrust_mat_directory(
        file: *mut RawFile,
        count: *mut c_int,
        error: *mut c_int,
    ) -> *mut *mut c_char;
    pub fn matrust_mat_next(
        file: *mut RawFile,
        name: *mut *const c_char,
        info: c_int,
        error: *mut c_int,
    ) -> *mut RawArray;
    pub fn matrust_mat_stream(file: *mut RawFile) -> *mut c_void;
    pub fn matrust_stream_eof(file: *mut c_void) -> c_int;
    pub fn matrust_stream_error(file: *mut c_void) -> c_int;
    pub fn matrust_stream_clear(file: *mut c_void);
    pub fn matrust_stream_position(file: *mut c_void) -> i64;

    pub fn matrust_eps() -> f64;
    pub fn matrust_inf() -> f64;
    pub fn matrust_nan() -> f64;
    pub fn matrust_is_finite(value: f64) -> c_int;
    pub fn matrust_is_inf(value: f64) -> c_int;
    pub fn matrust_is_nan(value: f64) -> c_int;
}

pub(crate) unsafe fn matrust_mat_close(file: *mut RawFile) -> c_int {
    #[cfg(test)]
    if let Some(status) = CLOSE_PROBE.with(|probe| {
        probe.get().map(|(status, count)| {
            probe.set(Some((status, count + 1)));
            status
        })
    }) {
        return status;
    }
    unsafe { matrust_mat_close_raw(file) }
}

#[cfg(test)]
thread_local! {
    pub(crate) static CLOSE_PROBE: std::cell::Cell<Option<(i32, usize)>> = const { std::cell::Cell::new(None) };
}

pub(crate) struct InfoArray(pub std::ptr::NonNull<RawArray>);
impl Drop for InfoArray {
    fn drop(&mut self) {
        unsafe { matrust_array_destroy(self.0.as_ptr()) }
    }
}
