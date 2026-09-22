use crate::{ffi, Matlab};
use rustmex::mxArray;
use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    ptr::NonNull,
};

/// An owned metadata-only header. It deliberately does NOT dereference or convert
/// to mxArray: its data pointers are absent and must never reach get/put data APIs.
/// ```compile_fail
/// fn save_header(file: &mut rustmat::MatFile<'_>, header: &rustmat::ArrayInfo<'_>) {
///     file.put(c"bad", header).unwrap();
/// }
/// ```
/// ```compile_fail
/// fn borrow_child(context: &rustmat::Matlab) -> rustmat::Result<()> {
///     let mut file = rustmat::MatFile::open(context, "data.mat", rustmat::OpenMode::Read)?;
///     let child;
///     {
///         let header = file.info(c"cell")?;
///         child = header.cell(0).unwrap();
///     }
///     let _ = child.dimensions();
///     Ok(())
/// }
/// ```
pub struct ArrayInfo<'matlab> {
    raw: ffi::Array,
    _context: PhantomData<&'matlab Matlab>,
}
impl<'matlab> ArrayInfo<'matlab> {
    pub(crate) fn new(raw: ffi::Array, _: &'matlab Matlab) -> Self {
        Self {
            raw,
            _context: PhantomData,
        }
    }
    pub fn view(&self) -> InfoRef<'_> {
        InfoRef {
            raw: self.raw.0,
            _owner: PhantomData,
        }
    }
    pub fn dimensions(&self) -> &[usize] {
        self.view().dimensions()
    }
    pub fn numel(&self) -> usize {
        self.view().numel()
    }
    pub fn class_id(&self) -> u32 {
        self.view().class_id()
    }
    pub fn class_name(&self) -> &CStr {
        self.view().class_name()
    }
    pub fn is_complex(&self) -> bool {
        self.view().is_complex()
    }
    pub fn is_sparse(&self) -> bool {
        self.view().is_sparse()
    }
    pub fn is_global(&self) -> bool {
        self.view().is_global()
    }
    pub fn field_names(&self) -> Vec<CString> {
        self.view().field_names()
    }
    pub fn cell(&self, index: usize) -> Option<InfoRef<'_>> {
        self.view().cell(index)
    }
    pub fn field(&self, index: usize, name: &CStr) -> Option<InfoRef<'_>> {
        self.view().field(index, name)
    }
}

/// A read-only header view, including headers inside cells and structures.
/// Indices are zero-based, linear, column-major indices.
#[derive(Clone, Copy)]
pub struct InfoRef<'owner> {
    raw: NonNull<mxArray>,
    _owner: PhantomData<&'owner ArrayInfo<'owner>>,
}
impl<'owner> InfoRef<'owner> {
    pub fn dimensions(self) -> &'owner [usize] {
        let mut count = 0;
        // SAFETY: header and dimensions remain alive for the owner borrow.
        let ptr = unsafe { ffi::rustmat_dimensions(self.raw.as_ptr(), &mut count) };
        if count == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(ptr, count) }
    }
    pub fn numel(self) -> usize {
        // SAFETY: header query does not access array data.
        unsafe { ffi::rustmat_numel(self.raw.as_ptr()) }
    }
    pub fn class_id(self) -> u32 {
        // SAFETY: header query only; preserve unknown future class IDs.
        unsafe { ffi::rustmat_class_id(self.raw.as_ptr()) }
    }
    pub fn class_name(self) -> &'owner CStr {
        // SAFETY: MATLAB returns a header-owned class name for a valid mxArray.
        unsafe { CStr::from_ptr(ffi::rustmat_class_name(self.raw.as_ptr())) }
    }
    fn flags(self) -> i32 {
        // SAFETY: all three queries use only header metadata.
        unsafe { ffi::rustmat_flags(self.raw.as_ptr()) }
    }
    pub fn is_complex(self) -> bool {
        self.flags() & 1 != 0
    }
    pub fn is_sparse(self) -> bool {
        self.flags() & 2 != 0
    }
    pub fn is_global(self) -> bool {
        self.flags() & 4 != 0
    }
    pub fn field_names(self) -> Vec<CString> {
        // SAFETY: the shim checks the class before asking for field count.
        let count = unsafe { ffi::rustmat_fields(self.raw.as_ptr()) };
        (0..count)
            .map(|i| unsafe {
                // SAFETY: i is within the valid field range; copy before owner drops.
                CStr::from_ptr(ffi::rustmat_field_name(self.raw.as_ptr(), i)).to_owned()
            })
            .collect()
    }
    pub fn cell(self, index: usize) -> Option<Self> {
        // SAFETY: the shim checks both class and index; no data pointer access.
        let raw = unsafe { ffi::rustmat_cell(self.raw.as_ptr(), index) };
        NonNull::new(raw.cast_mut()).map(|raw| Self {
            raw,
            _owner: PhantomData,
        })
    }
    pub fn field(self, index: usize, name: &CStr) -> Option<Self> {
        // SAFETY: the shim checks both class and index; name is NUL terminated.
        let raw = unsafe { ffi::rustmat_field(self.raw.as_ptr(), index, name.as_ptr()) };
        NonNull::new(raw.cast_mut()).map(|raw| Self {
            raw,
            _owner: PhantomData,
        })
    }
}
