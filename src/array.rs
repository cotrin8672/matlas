use crate::{ffi, Error, ErrorKind, Matlab, Result};
use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    mem::{size_of, ManuallyDrop},
    ptr::NonNull,
    rc::Rc,
};

/// MATLAB's stable matrix class identifiers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum Class {
    /// Unknown or invalid class identifier.
    Unknown = 0,
    /// Cell array.
    Cell = 1,
    /// Structure array.
    Struct = 2,
    /// Logical array.
    Logical = 3,
    /// UTF-16 character array.
    Char = 4,
    /// Reserved void class.
    Void = 5,
    /// Double-precision floating-point array.
    Double = 6,
    /// Single-precision floating-point array.
    Single = 7,
    /// Signed 8-bit integer array.
    Int8 = 8,
    /// Unsigned 8-bit integer array.
    Uint8 = 9,
    /// Signed 16-bit integer array.
    Int16 = 10,
    /// Unsigned 16-bit integer array.
    Uint16 = 11,
    /// Signed 32-bit integer array.
    Int32 = 12,
    /// Unsigned 32-bit integer array.
    Uint32 = 13,
    /// Signed 64-bit integer array.
    Int64 = 14,
    /// Unsigned 64-bit integer array.
    Uint64 = 15,
    /// Function-handle array.
    Function = 16,
    /// Opaque MATLAB value.
    Opaque = 17,
    /// MATLAB object array.
    Object = 18,
}

impl Class {
    /// Convert a native `mxClassID` integer into a known class.
    pub fn from_raw(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::Unknown,
            1 => Self::Cell,
            2 => Self::Struct,
            3 => Self::Logical,
            4 => Self::Char,
            5 => Self::Void,
            6 => Self::Double,
            7 => Self::Single,
            8 => Self::Int8,
            9 => Self::Uint8,
            10 => Self::Int16,
            11 => Self::Uint16,
            12 => Self::Int32,
            13 => Self::Uint32,
            14 => Self::Int64,
            15 => Self::Uint64,
            16 => Self::Function,
            17 => Self::Opaque,
            18 => Self::Object,
            _ => return None,
        })
    }
}

/// API-800 interleaved complex element layout.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[repr(C)]
pub struct Complex<T> {
    /// Real component.
    pub real: T,
    /// Imaginary component.
    pub imag: T,
}

mod sealed {
    pub trait Sealed {}
}

/// A Rust element with exactly the layout used by an API-800 numeric array.
pub trait Numeric: sealed::Sealed + Copy + 'static {
    /// MATLAB class corresponding to this Rust element type.
    const CLASS: Class;
    /// Whether this element uses interleaved complex storage.
    const COMPLEX: bool;
}

/// A numeric element supported by MATLAB's published sparse constructor.
///
/// API-800 sparse numeric arrays are always real or interleaved-complex
/// double arrays. Keeping this as a separate sealed bound makes unsupported
/// sparse element types a compile-time error instead of a runtime failure.
///
/// ```compile_fail
/// # use matlas::Matlab;
/// fn integer_sparse(cx: &Matlab<'_>) {
///     let _ = cx.sparse(1, 1, &[0, 1], &[0], &[1_i32]);
/// }
/// ```
pub trait SparseNumeric: Numeric {}

macro_rules! numeric {
    ($type:ty, $class:ident) => {
        impl sealed::Sealed for $type {}
        impl Numeric for $type {
            const CLASS: Class = Class::$class;
            const COMPLEX: bool = false;
        }
        impl sealed::Sealed for Complex<$type> {}
        impl Numeric for Complex<$type> {
            const CLASS: Class = Class::$class;
            const COMPLEX: bool = true;
        }
    };
}
numeric!(f64, Double);
numeric!(f32, Single);
numeric!(i8, Int8);
numeric!(u8, Uint8);
numeric!(i16, Int16);
numeric!(u16, Uint16);
numeric!(i32, Int32);
numeric!(u32, Uint32);
numeric!(i64, Int64);
numeric!(u64, Uint64);

impl SparseNumeric for f64 {}
impl SparseNumeric for Complex<f64> {}

/// A uniquely owned MATLAB array. The invariant brand prevents it from escaping
/// the MEX invocation that created it. Drop calls `mxDestroyArray` exactly once.
pub struct OwnedArray<'mex> {
    raw: NonNull<ffi::RawArray>,
    _brand: PhantomData<fn(&'mex mut ()) -> &'mex mut ()>,
    _thread: PhantomData<Rc<()>>,
}

/// A read-only view. It can neither be destroyed nor transferred to MATLAB.
#[derive(Clone, Copy)]
pub struct ArrayRef<'a> {
    raw: NonNull<ffi::RawArray>,
    _borrow: PhantomData<&'a ffi::RawArray>,
    _thread: PhantomData<Rc<()>>,
}

/// An exclusive array view. Child and data borrows are tied to each method call.
pub struct ArrayMut<'a, 'mex> {
    raw: NonNull<ffi::RawArray>,
    _borrow: PhantomData<&'a mut ffi::RawArray>,
    _brand: PhantomData<fn(&'mex mut ()) -> &'mex mut ()>,
    _thread: PhantomData<Rc<()>>,
}

impl<'mex> OwnedArray<'mex> {
    pub(crate) unsafe fn from_raw(raw: NonNull<ffi::RawArray>) -> Self {
        Self {
            raw,
            _brand: PhantomData,
            _thread: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> *mut ffi::RawArray {
        self.into_non_null().as_ptr()
    }

    fn into_non_null(self) -> NonNull<ffi::RawArray> {
        let this = ManuallyDrop::new(self);
        this.raw
    }

    /// Borrow the array without transferring or destroying it.
    pub fn as_ref(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }

    /// Exclusively borrow the array for in-place mutation.
    pub fn as_mut(&mut self) -> ArrayMut<'_, 'mex> {
        ArrayMut {
            raw: self.raw,
            _borrow: PhantomData,
            _brand: PhantomData,
            _thread: PhantomData,
        }
    }

    /// Deep-copy this array into a new Rust-owned MATLAB allocation.
    ///
    /// # Errors
    ///
    /// Returns an allocation error if MATLAB cannot return a copy.
    pub fn duplicate(&self) -> Result<Self> {
        let raw = unsafe { ffi::matrust_array_duplicate(self.raw.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("duplicate array"))?;
        Ok(unsafe { Self::from_raw(raw) })
    }

    /// Read a public object property. MATLAB returns an owned copy.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-object, an out-of-bounds index, an unknown
    /// or nonpublic property, or a native allocation failure.
    pub fn property(&self, index: usize, name: &CStr) -> Result<Self> {
        if index >= self.as_ref().numel() {
            return Err(Error::bounds("get property", index, self.as_ref().numel()));
        }
        let raw = unsafe { ffi::matrust_property_get(self.raw.as_ptr(), index, name.as_ptr()) };
        NonNull::new(raw)
            .map(|raw| unsafe { Self::from_raw(raw) })
            .ok_or_else(|| Error::new(ErrorKind::Native, "get property", format!("{name:?}")))
    }

    /// Transfer ownership to MATLAB's persistent MEX storage. The returned
    /// key is the only safe way to access the value again.
    ///
    /// # Errors
    ///
    /// Returns an error if a workspace borrow is active or the MEX cleanup
    /// callback cannot be registered.
    pub fn persist(self) -> Result<PersistentArray> {
        let raw = self.into_non_null();
        match crate::runtime::persist(raw) {
            Ok((slot, generation)) => Ok(PersistentArray {
                slot,
                generation,
                active: true,
                _thread: PhantomData,
            }),
            Err(error) => {
                unsafe { ffi::matrust_array_destroy(raw.as_ptr()) };
                Err(error)
            }
        }
    }

    /// Change the dimensions while preserving the element count.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid shape, a changed element count, a
    /// sparse reshape, or a native dimension-allocation failure.
    pub fn reshape(&mut self, dimensions: &[usize]) -> Result<()> {
        validate_dimensions(dimensions)?;
        if self.as_ref().is_sparse() && dimensions != self.as_ref().dimensions() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "reshape",
                "sparse arrays cannot be reshaped without rebuilding CSC storage",
            ));
        }
        if element_count(dimensions)? != self.as_ref().numel() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "reshape",
                "new shape must preserve the element count",
            ));
        }
        let status = unsafe {
            ffi::matrust_array_set_dimensions(
                self.raw.as_ptr(),
                dimensions.as_ptr(),
                dimensions.len(),
            )
        };
        if status != 0 {
            return Err(Error::native_status("reshape", status));
        }
        Ok(())
    }

    /// Set or clear MATLAB's global-workspace metadata bit.
    pub fn set_global_flag(&mut self, global: bool) {
        unsafe { ffi::matrust_array_set_global(self.raw.as_ptr(), i32::from(global)) }
    }

    /// Replace MATLAB's eight user-defined metadata bits.
    pub fn set_user_bits(&mut self, bits: u8) {
        unsafe { ffi::matrust_array_set_user_bits(self.raw.as_ptr(), bits) }
    }

    /// Convert a numeric array to real storage.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonnumeric array or native conversion failure.
    pub fn make_real(&mut self) -> Result<()> {
        if !self.as_ref().is_numeric() {
            return Err(Error::type_mismatch("make real", "numeric", self.as_ref()));
        }
        if unsafe { ffi::matrust_array_make_real(self.raw.as_ptr()) } == 0 {
            return Err(Error::native_status("make real", 0));
        }
        Ok(())
    }

    /// Convert a numeric array to interleaved complex storage.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonnumeric array or native conversion failure.
    pub fn make_complex(&mut self) -> Result<()> {
        if !self.as_ref().is_numeric() {
            return Err(Error::type_mismatch(
                "make complex",
                "numeric",
                self.as_ref(),
            ));
        }
        if unsafe { ffi::matrust_array_make_complex(self.raw.as_ptr()) } == 0 {
            return Err(Error::native_status("make complex", 0));
        }
        Ok(())
    }
}

/// A MATLAB-persistent array handle that may be stored between MEX calls.
///
/// Access requires a live [`Matlab`] context, so the underlying pointer cannot
/// be used outside a MEX invocation. Dropping the handle releases the array;
/// the registered MEX cleanup callback is a final safety net for abandoned
/// handles.
pub struct PersistentArray {
    slot: usize,
    generation: u64,
    active: bool,
    _thread: PhantomData<Rc<()>>,
}
impl PersistentArray {
    /// Borrow the persistent array during the current MEX invocation.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle is stale.
    pub fn as_ref<'a, 'mex>(&'a self, _context: &'a Matlab<'mex>) -> Result<ArrayRef<'a>> {
        let raw = crate::runtime::persistent(self.slot, self.generation)?;
        Ok(unsafe { ArrayRef::from_raw(raw) })
    }

    /// Exclusively borrow the persistent array during the current MEX invocation.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle is stale.
    pub fn as_mut<'a, 'mex>(
        &'a mut self,
        _context: &'a mut Matlab<'mex>,
    ) -> Result<ArrayMut<'a, 'mex>> {
        let raw = crate::runtime::persistent(self.slot, self.generation)?;
        Ok(ArrayMut {
            raw,
            _borrow: PhantomData,
            _brand: PhantomData,
            _thread: PhantomData,
        })
    }

    /// Explicitly remove and destroy the persistent array during a MEX invocation.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle is stale or a workspace borrow is active.
    pub fn remove<'mex>(mut self, _context: &mut Matlab<'mex>) -> Result<()> {
        crate::runtime::remove_persistent(self.slot, self.generation)?;
        self.active = false;
        Ok(())
    }
}
impl Drop for PersistentArray {
    fn drop(&mut self) {
        if self.active {
            let _ = crate::runtime::remove_persistent(self.slot, self.generation);
        }
    }
}

impl Drop for OwnedArray<'_> {
    fn drop(&mut self) {
        unsafe { crate::runtime::destroy_or_defer(self.raw) }
    }
}

impl<'a> ArrayRef<'a> {
    pub(crate) unsafe fn from_raw(raw: NonNull<ffi::RawArray>) -> Self {
        Self {
            raw,
            _borrow: PhantomData,
            _thread: PhantomData,
        }
    }

    pub(crate) fn as_ptr(self) -> *const ffi::RawArray {
        self.raw.as_ptr()
    }

    /// Return the row count reported by MATLAB.
    pub fn rows(self) -> usize {
        unsafe { ffi::matrust_array_m(self.as_ptr()) }
    }

    /// Return the column count reported by MATLAB.
    pub fn columns(self) -> usize {
        unsafe { ffi::matrust_array_n(self.as_ptr()) }
    }

    /// Return the total number of elements.
    pub fn numel(self) -> usize {
        unsafe { ffi::matrust_array_numel(self.as_ptr()) }
    }

    /// Borrow the native dimension vector.
    pub fn dimensions(self) -> &'a [usize] {
        let count = unsafe { ffi::matrust_array_ndims(self.as_ptr()) };
        if count == 0 {
            return &[];
        }
        let raw = unsafe { ffi::matrust_array_dims(self.as_ptr()) };
        checked_slice(raw, count, "array dimensions").unwrap_or(&[])
    }

    /// Convert zero-based multidimensional subscripts to MATLAB's linear,
    /// column-major index. The number of subscripts must match the rank.
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong rank, an out-of-bounds subscript, or
    /// arithmetic overflow.
    pub fn linear_index(self, subscripts: &[usize]) -> Result<usize> {
        let dimensions = self.dimensions();
        if subscripts.len() != dimensions.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "linear index",
                format!(
                    "expected {} subscripts, got {}",
                    dimensions.len(),
                    subscripts.len()
                ),
            ));
        }
        dimensions
            .iter()
            .zip(subscripts)
            .try_fold(
                (0usize, 1usize),
                |(index, stride), (&dimension, &subscript)| {
                    if subscript >= dimension {
                        return Err(Error::bounds("linear index", subscript, dimension));
                    }
                    let offset = subscript.checked_mul(stride).ok_or_else(|| {
                        Error::new(ErrorKind::Bounds, "linear index", "index overflow")
                    })?;
                    let index = index.checked_add(offset).ok_or_else(|| {
                        Error::new(ErrorKind::Bounds, "linear index", "index overflow")
                    })?;
                    let stride = stride.checked_mul(dimension).ok_or_else(|| {
                        Error::new(ErrorKind::Bounds, "linear index", "stride overflow")
                    })?;
                    Ok((index, stride))
                },
            )
            .map(|(index, _)| index)
    }

    /// Return the native array class when it is known to this crate.
    pub fn class(self) -> Option<Class> {
        Class::from_raw(unsafe { ffi::matrust_array_class(self.as_ptr()) })
    }

    /// Borrow MATLAB's static class-name string.
    pub fn class_name(self) -> &'a CStr {
        unsafe { CStr::from_ptr(ffi::matrust_array_class_name(self.as_ptr())) }
    }

    /// Return the storage size of one array element.
    pub fn element_size(self) -> usize {
        unsafe { ffi::matrust_array_element_size(self.as_ptr()) }
    }

    /// Test whether this is a numeric array.
    pub fn is_numeric(self) -> bool {
        unsafe { ffi::matrust_array_is_numeric(self.as_ptr()) != 0 }
    }
    /// Test whether this is a cell array.
    pub fn is_cell(self) -> bool {
        unsafe { ffi::matrust_array_is_cell(self.as_ptr()) != 0 }
    }
    /// Test whether this is a logical array.
    pub fn is_logical(self) -> bool {
        unsafe { ffi::matrust_array_is_logical(self.as_ptr()) != 0 }
    }
    /// Test whether this is a 1-by-1 MATLAB logical value.
    pub fn is_logical_scalar(self) -> bool {
        unsafe { ffi::matrust_mx_is_logical_scalar(self.as_ptr()) != 0 }
    }
    /// Test whether this is a character array.
    pub fn is_char(self) -> bool {
        unsafe { ffi::matrust_array_is_char(self.as_ptr()) != 0 }
    }
    /// Test whether this is a character row or a scalar MATLAB string.
    pub fn is_text_scalar(self) -> bool {
        (self.is_char()
            && (self.dimensions() == [0, 0] || (self.dimensions().len() == 2 && self.rows() == 1)))
            || (self.is_class(c"string") && self.numel() == 1)
    }

    /// Decode a character row or scalar MATLAB string into Rust text.
    ///
    /// # Errors
    ///
    /// Returns an error for another type or shape, a missing string, an active
    /// workspace borrow, a failed MATLAB conversion, or invalid UTF-16.
    pub fn to_text(self, cx: &mut Matlab<'_>) -> Result<String> {
        if !self.is_text_scalar() {
            return Err(Error::type_mismatch(
                "convert text",
                "char row or scalar string",
                self,
            ));
        }
        if self.is_char() {
            return self.string();
        }
        let [missing] = cx.call_array(c"ismissing", &[self])?;
        if missing.as_ref().logical_scalar()? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "convert text",
                "missing MATLAB string",
            ));
        }
        let [chars] = cx.call_array(c"char", &[self])?;
        chars.as_ref().string()
    }

    /// Read a logical scalar as a Rust boolean.
    ///
    /// # Errors
    ///
    /// Returns a type error unless the array is a logical scalar.
    pub fn logical_scalar(self) -> Result<bool> {
        if !self.is_logical_scalar() {
            return Err(Error::type_mismatch(
                "logical scalar",
                "logical scalar",
                self,
            ));
        }
        Ok(unsafe { ffi::matrust_mx_is_logical_scalar_true(self.as_ptr()) != 0 })
    }
    /// Test whether this is a structure array.
    pub fn is_struct(self) -> bool {
        unsafe { ffi::matrust_array_is_struct(self.as_ptr()) != 0 }
    }
    /// Test whether this uses sparse storage.
    pub fn is_sparse(self) -> bool {
        unsafe { ffi::matrust_array_is_sparse(self.as_ptr()) != 0 }
    }
    /// Test whether this uses complex storage.
    pub fn is_complex(self) -> bool {
        unsafe { ffi::matrust_array_is_complex(self.as_ptr()) != 0 }
    }
    /// Test whether this has zero elements.
    pub fn is_empty(self) -> bool {
        unsafe { ffi::matrust_array_is_empty(self.as_ptr()) != 0 }
    }
    /// Test whether this has exactly one element.
    pub fn is_scalar(self) -> bool {
        unsafe { ffi::matrust_array_is_scalar(self.as_ptr()) != 0 }
    }
    /// Test whether this is a legacy MATLAB v5 object array.
    ///
    /// Custom `classdef` objects can have dynamic class identifiers and should
    /// be tested with [`ArrayRef::is_class`] or [`ArrayRef::class_name`].
    pub fn is_object(self) -> bool {
        unsafe { ffi::matrust_array_is_object(self.as_ptr()) != 0 }
    }
    /// Test whether this is an opaque MATLAB value.
    pub fn is_opaque(self) -> bool {
        unsafe { ffi::matrust_array_is_opaque(self.as_ptr()) != 0 }
    }
    /// Test whether this is a function handle.
    pub fn is_function_handle(self) -> bool {
        unsafe { ffi::matrust_array_is_function(self.as_ptr()) != 0 }
    }
    /// Test whether this value has the named MATLAB class.
    pub fn is_class(self, name: &CStr) -> bool {
        unsafe { ffi::matrust_array_is_class(self.as_ptr(), name.as_ptr()) != 0 }
    }
    /// Test MATLAB's global-workspace metadata bit.
    pub fn is_from_global_workspace(self) -> bool {
        unsafe { ffi::matrust_array_is_global(self.as_ptr()) != 0 }
    }
    /// Return MATLAB's eight user-defined metadata bits.
    pub fn user_bits(self) -> u8 {
        unsafe { ffi::matrust_array_user_bits(self.as_ptr()) }
    }

    /// Convert the first dense numeric element to `f64` using MATLAB semantics.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is nonempty, dense, and numeric.
    pub fn scalar(self) -> Result<f64> {
        if !self.is_numeric() || self.is_sparse() || self.is_empty() {
            return Err(Error::type_mismatch(
                "scalar",
                "nonempty dense numeric",
                self,
            ));
        }
        Ok(unsafe { ffi::matrust_array_scalar(self.as_ptr()) })
    }

    /// Borrow dense numeric data after checking class, complexity, and layout.
    ///
    /// # Errors
    ///
    /// Returns an error if `T` does not match the array or the native data
    /// pointer and length do not form a valid Rust slice.
    pub fn data<T: Numeric>(self) -> Result<&'a [T]> {
        self.check_numeric::<T>("numeric data")?;
        let raw = unsafe { ffi::matrust_array_data(self.as_ptr()) }.cast::<T>();
        if self.numel() == 0 {
            return Ok(&[]);
        }
        if raw.is_null() {
            return Err(Error::new(
                ErrorKind::Native,
                "numeric data",
                "null data pointer",
            ));
        }
        checked_slice(raw, self.numel(), "numeric data")
    }

    /// Borrow dense logical data.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is dense logical storage with a
    /// valid native data pointer.
    pub fn logicals(self) -> Result<&'a [bool]> {
        if !self.is_logical() || self.is_sparse() {
            return Err(Error::type_mismatch("logical data", "dense logical", self));
        }
        let raw = unsafe { ffi::matrust_array_logicals(self.as_ptr()) }.cast::<bool>();
        if self.numel() == 0 {
            return Ok(&[]);
        }
        checked_slice(raw, self.numel(), "logical data")
    }

    /// Borrow UTF-16 character data.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is character storage with a valid
    /// native data pointer.
    pub fn chars(self) -> Result<&'a [u16]> {
        if !self.is_char() {
            return Err(Error::type_mismatch("character data", "char", self));
        }
        let raw = unsafe { ffi::matrust_array_chars(self.as_ptr()) };
        if self.numel() == 0 {
            return Ok(&[]);
        }
        checked_slice(raw, self.numel(), "character data")
    }

    /// Decode a character array as UTF-16.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-character array or unpaired UTF-16 surrogate.
    pub fn string(self) -> Result<String> {
        String::from_utf16(self.chars()?).map_err(|error| {
            Error::new(ErrorKind::InvalidInput, "decode UTF-16", error.to_string())
        })
    }

    /// Convert a MATLAB character array to UTF-8 through MATLAB's allocator.
    ///
    /// # Errors
    ///
    /// Returns an error if MATLAB cannot convert the value or returns invalid
    /// UTF-8.
    pub fn to_utf8(self) -> Result<String> {
        let raw = unsafe { ffi::matrust_array_to_utf8(self.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| {
            Error::new(
                ErrorKind::Native,
                "convert UTF-8",
                "MATLAB returned a null string",
            )
        })?;
        let result = unsafe { CStr::from_ptr(raw.as_ptr()) }
            .to_str()
            .map(str::to_owned)
            .map_err(|error| {
                Error::new(
                    ErrorKind::Native,
                    "convert UTF-8",
                    format!("MATLAB returned invalid UTF-8: {error}"),
                )
            });
        unsafe { ffi::matrust_free(raw.as_ptr().cast()) };
        result
    }

    /// Convert a MATLAB character array to the current local encoding.
    ///
    /// # Errors
    ///
    /// Returns an error if MATLAB cannot convert the value.
    pub fn to_local(self) -> Result<Vec<u8>> {
        let raw = unsafe { ffi::matrust_array_to_local(self.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| {
            Error::new(
                ErrorKind::Native,
                "convert local string",
                "MATLAB returned a null string",
            )
        })?;
        let result = unsafe { CStr::from_ptr(raw.as_ptr()) }.to_bytes().to_vec();
        unsafe { ffi::matrust_free(raw.as_ptr().cast()) };
        Ok(result)
    }

    /// Borrow a cell value, returning `None` for an empty cell.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-cell array or out-of-bounds index.
    pub fn cell(self, index: usize) -> Result<Option<ArrayRef<'a>>> {
        self.check_index("cell", index, self.is_cell())?;
        let raw = unsafe { ffi::matrust_cell_get(self.as_ptr(), index) };
        Ok(NonNull::new(raw).map(|raw| unsafe { ArrayRef::from_raw(raw) }))
    }

    /// Look up a structure field number by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the array is not a structure.
    pub fn field_number(self, name: &CStr) -> Result<Option<usize>> {
        if !self.is_struct() {
            return Err(Error::type_mismatch("field number", "struct", self));
        }
        let field = unsafe { ffi::matrust_struct_field_number(self.as_ptr(), name.as_ptr()) };
        Ok((field >= 0).then_some(field as usize))
    }

    /// Copy all structure field names.
    ///
    /// # Errors
    ///
    /// Returns an error if the array is not a structure.
    pub fn field_names(self) -> Result<Vec<CString>> {
        if !self.is_struct() {
            return Err(Error::type_mismatch("field names", "struct", self));
        }
        let count = unsafe { ffi::matrust_struct_field_count(self.as_ptr()) };
        Ok((0..count)
            .map(|field| unsafe {
                CStr::from_ptr(ffi::matrust_struct_field_name(self.as_ptr(), field)).to_owned()
            })
            .collect())
    }

    /// Borrow a named structure field.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-structure array, unknown field, or
    /// out-of-bounds element index.
    pub fn field(self, index: usize, name: &CStr) -> Result<Option<ArrayRef<'a>>> {
        let field = self.field_number(name)?.ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "field",
                format!("unknown field {name:?}"),
            )
        })?;
        self.field_by_number(index, field)
    }

    /// Borrow a structure field by numeric field index.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-structure array or out-of-bounds element or
    /// field index.
    pub fn field_by_number(self, index: usize, field: usize) -> Result<Option<ArrayRef<'a>>> {
        self.check_index("field", index, self.is_struct())?;
        let fields = unsafe { ffi::matrust_struct_field_count(self.as_ptr()) } as usize;
        if field >= fields {
            return Err(Error::bounds("field", field, fields));
        }
        let raw = unsafe { ffi::matrust_struct_get(self.as_ptr(), index, field as i32) };
        Ok(NonNull::new(raw).map(|raw| unsafe { ArrayRef::from_raw(raw) }))
    }

    /// Validate and borrow the compressed-column sparse indices.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonsparse array or malformed native CSC storage.
    pub fn sparse_indices(self) -> Result<SparseIndices<'a>> {
        if !self.is_sparse() {
            return Err(Error::type_mismatch("sparse indices", "sparse", self));
        }
        let nzmax = unsafe { ffi::matrust_sparse_nzmax(self.as_ptr()) };
        let ir = unsafe { ffi::matrust_sparse_ir(self.as_ptr()) };
        let jc = unsafe { ffi::matrust_sparse_jc(self.as_ptr()) };
        let columns = self.columns().checked_add(1).ok_or_else(|| {
            Error::new(ErrorKind::Bounds, "sparse indices", "column count overflow")
        })?;
        if columns > 0 && jc.is_null() || nzmax > 0 && ir.is_null() {
            return Err(Error::new(
                ErrorKind::Native,
                "sparse indices",
                "null sparse index pointer",
            ));
        }
        let columns = checked_slice(jc, columns, "sparse column indices")?;
        let nnz = *columns.last().unwrap_or(&0);
        if columns.first() != Some(&0) || nnz > nzmax || columns.windows(2).any(|w| w[0] > w[1]) {
            return Err(Error::new(
                ErrorKind::Native,
                "sparse indices",
                "invalid compressed-column structure",
            ));
        }
        let rows = checked_slice(ir, nnz, "sparse row indices")?;
        if rows.iter().any(|&row| row >= self.rows())
            || columns.windows(2).any(|range| {
                rows[range[0]..range[1]]
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
            })
        {
            return Err(Error::new(
                ErrorKind::Native,
                "sparse indices",
                "row indices are not sorted, unique, and in bounds",
            ));
        }
        Ok(SparseIndices { rows, columns })
    }

    /// Borrow the stored nonzero values of a sparse numeric array.
    ///
    /// # Errors
    ///
    /// Returns an error if `T` does not match the sparse array or its CSC/data
    /// storage is malformed.
    pub fn sparse_data<T: SparseNumeric>(self) -> Result<&'a [T]> {
        if !self.is_sparse()
            || self.class() != Some(T::CLASS)
            || self.is_complex() != T::COMPLEX
            || self.element_size() != size_of::<T>()
        {
            return Err(Error::type_mismatch(
                "sparse numeric data",
                std::any::type_name::<T>(),
                self,
            ));
        }
        let count = self.sparse_indices()?.rows.len();
        let raw = unsafe { ffi::matrust_array_data(self.as_ptr()) }.cast::<T>();
        checked_slice(raw, count, "sparse numeric data")
    }

    /// Borrow the stored true values of a sparse logical array.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is sparse logical storage with valid
    /// CSC/data pointers.
    pub fn sparse_logicals(self) -> Result<&'a [bool]> {
        if !self.is_sparse() || !self.is_logical() {
            return Err(Error::type_mismatch(
                "sparse logical data",
                "sparse logical",
                self,
            ));
        }
        let count = self.sparse_indices()?.rows.len();
        let raw = unsafe { ffi::matrust_array_logicals(self.as_ptr()) }.cast::<bool>();
        checked_slice(raw, count, "sparse logical data")
    }

    fn check_numeric<T: Numeric>(self, operation: &'static str) -> Result<()> {
        if self.class() != Some(T::CLASS)
            || self.is_complex() != T::COMPLEX
            || self.is_sparse()
            || self.element_size() != size_of::<T>()
        {
            return Err(Error::type_mismatch(
                operation,
                std::any::type_name::<T>(),
                self,
            ));
        }
        Ok(())
    }

    fn check_index(self, operation: &'static str, index: usize, right_class: bool) -> Result<()> {
        if !right_class {
            return Err(Error::type_mismatch(operation, operation, self));
        }
        if index >= self.numel() {
            return Err(Error::bounds(operation, index, self.numel()));
        }
        Ok(())
    }
}

impl<'a, 'mex> ArrayMut<'a, 'mex> {
    /// Reborrow this exclusive view as read-only.
    pub fn as_ref(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }

    /// Mutably borrow dense numeric data after validating its layout.
    ///
    /// # Errors
    ///
    /// Returns an error if `T` does not match the array or the native data
    /// pointer and length do not form a valid Rust slice.
    pub fn data_mut<T: Numeric>(&mut self) -> Result<&mut [T]> {
        self.as_ref().check_numeric::<T>("mutable numeric data")?;
        let count = self.as_ref().numel();
        let raw = unsafe { ffi::matrust_array_data(self.raw.as_ptr()) }.cast::<T>();
        if count == 0 {
            return Ok(&mut []);
        }
        if raw.is_null() {
            return Err(Error::new(
                ErrorKind::Native,
                "mutable numeric data",
                "null data pointer",
            ));
        }
        checked_slice_mut(raw, count, "mutable numeric data")
    }

    /// Mutably borrow dense logical data.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is dense logical storage with a
    /// valid native data pointer.
    pub fn logicals_mut(&mut self) -> Result<&mut [bool]> {
        if !self.as_ref().is_logical() || self.as_ref().is_sparse() {
            return Err(Error::type_mismatch(
                "mutable logical data",
                "dense logical",
                self.as_ref(),
            ));
        }
        let count = self.as_ref().numel();
        let raw: *mut bool = unsafe { ffi::matrust_array_logicals(self.raw.as_ptr()) }
            .cast_mut()
            .cast();
        if count == 0 {
            return Ok(&mut []);
        }
        if raw.is_null() {
            return Err(Error::new(
                ErrorKind::Native,
                "mutable logical data",
                "null data pointer",
            ));
        }
        checked_slice_mut(raw, count, "mutable logical data")
    }

    /// Mutably borrow UTF-16 character data.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is character storage with a valid
    /// native data pointer.
    pub fn chars_mut(&mut self) -> Result<&mut [u16]> {
        if !self.as_ref().is_char() {
            return Err(Error::type_mismatch(
                "mutable character data",
                "char",
                self.as_ref(),
            ));
        }
        let count = self.as_ref().numel();
        let raw = unsafe { ffi::matrust_array_chars(self.raw.as_ptr()) }.cast_mut();
        if count == 0 {
            return Ok(&mut []);
        }
        if raw.is_null() {
            return Err(Error::new(
                ErrorKind::Native,
                "mutable character data",
                "null data pointer",
            ));
        }
        checked_slice_mut(raw, count, "mutable character data")
    }

    /// Mutably borrow the stored nonzero values of a sparse numeric array.
    ///
    /// # Errors
    ///
    /// Returns an error if `T` does not match the sparse array or its CSC/data
    /// storage is malformed.
    pub fn sparse_data_mut<T: SparseNumeric>(&mut self) -> Result<&mut [T]> {
        let value = self.as_ref();
        if !value.is_sparse()
            || value.class() != Some(T::CLASS)
            || value.is_complex() != T::COMPLEX
            || value.element_size() != size_of::<T>()
        {
            return Err(Error::type_mismatch(
                "mutable sparse numeric data",
                std::any::type_name::<T>(),
                value,
            ));
        }
        let count = value.sparse_indices()?.rows.len();
        let raw = unsafe { ffi::matrust_array_data(self.raw.as_ptr()) }.cast::<T>();
        checked_slice_mut(raw, count, "mutable sparse numeric data")
    }

    /// Mutably borrow the stored true values of a sparse logical array.
    ///
    /// # Errors
    ///
    /// Returns an error unless the array is sparse logical storage with valid
    /// CSC/data pointers.
    pub fn sparse_logicals_mut(&mut self) -> Result<&mut [bool]> {
        let value = self.as_ref();
        if !value.is_sparse() || !value.is_logical() {
            return Err(Error::type_mismatch(
                "mutable sparse logical data",
                "sparse logical",
                value,
            ));
        }
        let count = value.sparse_indices()?.rows.len();
        let raw = unsafe { ffi::matrust_array_logicals(self.raw.as_ptr()) }
            .cast_mut()
            .cast::<bool>();
        checked_slice_mut(raw, count, "mutable sparse logical data")
    }

    /// Borrow a cell value from this array.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-cell array or out-of-bounds index.
    pub fn cell(&self, index: usize) -> Result<Option<ArrayRef<'_>>> {
        self.as_ref().cell(index)
    }

    /// Exclusively borrow a cell value, if present.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-cell array or out-of-bounds index.
    pub fn cell_mut(&mut self, index: usize) -> Result<Option<ArrayMut<'_, 'mex>>> {
        self.as_ref()
            .check_index("cell", index, self.as_ref().is_cell())?;
        let raw = unsafe { ffi::matrust_cell_get(self.raw.as_ptr(), index) };
        Ok(NonNull::new(raw).map(|raw| ArrayMut {
            raw,
            _borrow: PhantomData,
            _brand: PhantomData,
            _thread: PhantomData,
        }))
    }

    /// Replace a cell, consuming the new owner and returning the previous owner.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-cell array or out-of-bounds index. Ownership
    /// is not transferred when validation fails.
    pub fn replace_cell(
        &mut self,
        index: usize,
        value: Option<OwnedArray<'mex>>,
    ) -> Result<Option<OwnedArray<'mex>>> {
        self.as_ref()
            .check_index("cell", index, self.as_ref().is_cell())?;
        let old = unsafe { ffi::matrust_cell_get(self.raw.as_ptr(), index) };
        let new = value.map_or(std::ptr::null_mut(), OwnedArray::into_raw);
        unsafe { ffi::matrust_cell_set(self.raw.as_ptr(), index, new) };
        Ok(NonNull::new(old).map(|raw| unsafe { OwnedArray::from_raw(raw) }))
    }

    /// Replace a named structure field and return its previous owner.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-structure array, unknown field, or
    /// out-of-bounds element index. Ownership is not transferred on failure.
    pub fn replace_field(
        &mut self,
        index: usize,
        name: &CStr,
        value: Option<OwnedArray<'mex>>,
    ) -> Result<Option<OwnedArray<'mex>>> {
        let field = self
            .as_ref()
            .field_number(name)?
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "replace field", "unknown field"))?;
        self.replace_field_by_number(index, field, value)
    }

    /// Replace a numbered structure field and return its previous owner.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-structure array or out-of-bounds element or
    /// field index. Ownership is not transferred on failure.
    pub fn replace_field_by_number(
        &mut self,
        index: usize,
        field: usize,
        value: Option<OwnedArray<'mex>>,
    ) -> Result<Option<OwnedArray<'mex>>> {
        self.as_ref()
            .check_index("field", index, self.as_ref().is_struct())?;
        let fields = unsafe { ffi::matrust_struct_field_count(self.raw.as_ptr()) } as usize;
        if field >= fields {
            return Err(Error::bounds("field", field, fields));
        }
        let old = unsafe { ffi::matrust_struct_get(self.raw.as_ptr(), index, field as i32) };
        let new = value.map_or(std::ptr::null_mut(), OwnedArray::into_raw);
        unsafe { ffi::matrust_struct_set(self.raw.as_ptr(), index, field as i32, new) };
        Ok(NonNull::new(old).map(|raw| unsafe { OwnedArray::from_raw(raw) }))
    }

    /// Add a field to a structure array and return its field number.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-structure array or a native invalid-name or
    /// allocation failure.
    pub fn add_field(&mut self, name: &CStr) -> Result<usize> {
        if !self.as_ref().is_struct() {
            return Err(Error::type_mismatch("add field", "struct", self.as_ref()));
        }
        let field = unsafe { ffi::matrust_struct_add_field(self.raw.as_ptr(), name.as_ptr()) };
        if field < 0 {
            return Err(Error::new(
                ErrorKind::Native,
                "add field",
                format!("{name:?}"),
            ));
        }
        Ok(field as usize)
    }

    /// Remove a structure field after destroying all values it contained.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-structure array or out-of-bounds field.
    pub fn remove_field(&mut self, field: usize) -> Result<()> {
        if !self.as_ref().is_struct() {
            return Err(Error::type_mismatch(
                "remove field",
                "struct",
                self.as_ref(),
            ));
        }
        let fields = unsafe { ffi::matrust_struct_field_count(self.raw.as_ptr()) } as usize;
        if field >= fields {
            return Err(Error::bounds("remove field", field, fields));
        }
        for index in 0..self.as_ref().numel() {
            drop(self.replace_field_by_number(index, field, None)?);
        }
        unsafe { ffi::matrust_struct_remove_field(self.raw.as_ptr(), field as i32) };
        Ok(())
    }

    /// Assign a public object property. MATLAB copies `value`; its ownership
    /// remains with Rust and is never transferred into the object.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-object or out-of-bounds index. The native
    /// `mxSetProperty` operation has no status return; see
    /// [`crate::error_handling`].
    pub fn set_property(&mut self, index: usize, name: &CStr, value: ArrayRef<'_>) -> Result<()> {
        if index >= self.as_ref().numel() {
            return Err(Error::bounds("set property", index, self.as_ref().numel()));
        }
        let existing =
            unsafe { ffi::matrust_property_get(self.raw.as_ptr(), index, name.as_ptr()) };
        let existing = NonNull::new(existing).ok_or_else(|| {
            Error::new(
                ErrorKind::Native,
                "set property",
                "property does not exist or is not public",
            )
        })?;
        unsafe { ffi::matrust_array_destroy(existing.as_ptr()) };
        unsafe {
            ffi::matrust_property_set(self.raw.as_ptr(), index, name.as_ptr(), value.as_ptr())
        };
        Ok(())
    }
}

/// Validated compressed-column indices borrowed from a sparse array.
pub struct SparseIndices<'a> {
    /// Row index for each stored value.
    pub rows: &'a [usize],
    /// Column offsets; its length is the column count plus one.
    pub columns: &'a [usize],
}

/// An allocated numeric array that cannot be read or transferred until filled.
pub struct UninitNumeric<'mex, T: Numeric> {
    array: OwnedArray<'mex>,
    _element: PhantomData<T>,
}

impl<'mex, T: Numeric> UninitNumeric<'mex, T> {
    /// Initialize every element and convert the allocation into an owned array.
    ///
    /// # Errors
    ///
    /// Returns an error if the value count does not match the allocation or
    /// the native buffer cannot form a valid Rust slice.
    pub fn fill(self, values: &[T]) -> Result<OwnedArray<'mex>> {
        if values.len() != self.array.as_ref().numel() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "initialize numeric array",
                format!(
                    "expected {} elements, got {}",
                    self.array.as_ref().numel(),
                    values.len()
                ),
            ));
        }
        let count = values.len();
        let raw = unsafe { ffi::matrust_array_data(self.array.raw.as_ptr()) }.cast::<T>();
        if count != 0 && raw.is_null() {
            return Err(Error::new(
                ErrorKind::Native,
                "initialize numeric array",
                "null data pointer",
            ));
        }
        if count != 0
            && count
                .checked_mul(size_of::<T>())
                .map_or(true, |bytes| bytes > isize::MAX as usize)
        {
            return Err(Error::new(
                ErrorKind::Bounds,
                "initialize numeric array",
                "buffer is too large for a Rust slice",
            ));
        }
        if count != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(values.as_ptr(), raw, count);
            }
        }
        Ok(self.array)
    }
}

impl<'mex> Matlab<'mex> {
    /// Allocate an uninitialized dense numeric array.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid or overflowing shape or a null native
    /// allocation result.
    pub fn uninit_numeric<T: Numeric>(
        &self,
        dimensions: &[usize],
    ) -> Result<UninitNumeric<'mex, T>> {
        validate_dimensions(dimensions)?;
        element_count(dimensions)?;
        let raw = unsafe {
            ffi::matrust_create_uninit_numeric(
                dimensions.len(),
                dimensions.as_ptr(),
                T::CLASS as i32,
                i32::from(T::COMPLEX),
            )
        };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create numeric array"))?;
        Ok(UninitNumeric {
            array: unsafe { OwnedArray::from_raw(raw) },
            _element: PhantomData,
        })
    }

    /// Create a dense numeric array from a complete element slice.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid shape, a shape/value length mismatch,
    /// or a null native allocation result.
    pub fn numeric<T: Numeric>(
        &self,
        dimensions: &[usize],
        values: &[T],
    ) -> Result<OwnedArray<'mex>> {
        self.uninit_numeric::<T>(dimensions)?.fill(values)
    }

    /// Create a 1-by-1 numeric value.
    ///
    /// # Errors
    ///
    /// Returns an error if MATLAB cannot allocate the array.
    pub fn scalar<T: Numeric>(&self, value: T) -> Result<OwnedArray<'mex>> {
        self.numeric(&[1, 1], &[value])
    }

    /// Create a dense logical array from a complete value slice.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid shape, a shape/value length mismatch,
    /// or a null native allocation result.
    pub fn logical(&self, dimensions: &[usize], values: &[bool]) -> Result<OwnedArray<'mex>> {
        validate_dimensions(dimensions)?;
        if element_count(dimensions)? != values.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "create logical",
                "shape/value length mismatch",
            ));
        }
        let raw = unsafe { ffi::matrust_create_logical(dimensions.len(), dimensions.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create logical array"))?;
        let mut array = unsafe { OwnedArray::from_raw(raw) };
        array.as_mut().logicals_mut()?.copy_from_slice(values);
        Ok(array)
    }

    /// Create a 1-by-1 MATLAB logical value.
    ///
    /// # Errors
    ///
    /// Returns an allocation error if MATLAB cannot create the array.
    pub fn logical_scalar(&self, value: bool) -> Result<OwnedArray<'mex>> {
        let raw = unsafe { ffi::matrust_mx_create_logical_scalar(i32::from(value)) };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create logical scalar"))?;
        Ok(unsafe { OwnedArray::from_raw(raw) })
    }

    /// Create a UTF-16 character array from column-major code units.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid shape, a shape/value length mismatch,
    /// or a null native allocation result.
    pub fn char_array(&self, dimensions: &[usize], values: &[u16]) -> Result<OwnedArray<'mex>> {
        validate_dimensions(dimensions)?;
        if element_count(dimensions)? != values.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "create character array",
                "shape/value length mismatch",
            ));
        }
        let raw = unsafe { ffi::matrust_create_char(dimensions.len(), dimensions.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create character array"))?;
        let mut array = unsafe { OwnedArray::from_raw(raw) };
        array.as_mut().chars_mut()?.copy_from_slice(values);
        Ok(array)
    }

    /// Create a 1-by-N UTF-16 MATLAB character array.
    ///
    /// # Errors
    ///
    /// Returns an error for length overflow or a null native allocation result.
    pub fn string(&self, value: &str) -> Result<OwnedArray<'mex>> {
        let utf16: Vec<u16> = value.encode_utf16().collect();
        self.char_array(&[1, utf16.len()], &utf16)
    }

    /// Create a padded MATLAB character matrix from UTF-8 Rust strings.
    ///
    /// # Errors
    ///
    /// Returns an error for length overflow or a null native allocation result.
    pub fn char_matrix(&self, rows: &[&str]) -> Result<OwnedArray<'mex>> {
        let rows: Vec<Vec<u16>> = rows
            .iter()
            .map(|row| row.encode_utf16().collect())
            .collect();
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        let count = rows.len().checked_mul(width).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "create character matrix",
                "element count overflow",
            )
        })?;
        let mut values = Vec::with_capacity(count);
        for column in 0..width {
            for row in &rows {
                values.push(row.get(column).copied().unwrap_or(u16::from(b' ')));
            }
        }
        self.char_array(&[rows.len(), width], &values)
    }

    /// Create an empty cell array with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid or overflowing shape or a null native
    /// allocation result.
    pub fn cell(&self, dimensions: &[usize]) -> Result<OwnedArray<'mex>> {
        validate_dimensions(dimensions)?;
        element_count(dimensions)?;
        let raw = unsafe { ffi::matrust_create_cell(dimensions.len(), dimensions.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create cell array"))?;
        Ok(unsafe { OwnedArray::from_raw(raw) })
    }

    /// Create an empty structure array with the requested fields.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid or overflowing shape, too many fields,
    /// invalid native field metadata, or a null allocation result.
    pub fn structure(&self, dimensions: &[usize], fields: &[&CStr]) -> Result<OwnedArray<'mex>> {
        validate_dimensions(dimensions)?;
        element_count(dimensions)?;
        let count = i32::try_from(fields.len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "create struct", "too many fields"))?;
        let names: Vec<_> = fields.iter().map(|name| name.as_ptr()).collect();
        let names_ptr = if names.is_empty() {
            std::ptr::null()
        } else {
            names.as_ptr()
        };
        let raw = unsafe {
            ffi::matrust_create_struct(dimensions.len(), dimensions.as_ptr(), count, names_ptr)
        };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create struct array"))?;
        Ok(unsafe { OwnedArray::from_raw(raw) })
    }

    /// Create a numeric sparse array from validated CSC components.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed CSC components or a null native
    /// allocation result.
    pub fn sparse<T: SparseNumeric>(
        &self,
        rows: usize,
        columns: usize,
        column_offsets: &[usize],
        row_indices: &[usize],
        values: &[T],
    ) -> Result<OwnedArray<'mex>> {
        validate_sparse(rows, columns, column_offsets, row_indices, values.len())?;
        let raw = unsafe {
            ffi::matrust_create_sparse(rows, columns, values.len().max(1), 0, i32::from(T::COMPLEX))
        };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("create sparse array"))?;
        let array = unsafe { OwnedArray::from_raw(raw) };
        unsafe {
            std::ptr::copy_nonoverlapping(
                values.as_ptr(),
                ffi::matrust_array_data(raw.as_ptr()).cast::<T>(),
                values.len(),
            );
            std::ptr::copy_nonoverlapping(
                row_indices.as_ptr(),
                ffi::matrust_sparse_ir(raw.as_ptr()).cast_mut(),
                row_indices.len(),
            );
            std::ptr::copy_nonoverlapping(
                column_offsets.as_ptr(),
                ffi::matrust_sparse_jc(raw.as_ptr()).cast_mut(),
                column_offsets.len(),
            );
        }
        Ok(array)
    }

    /// Create a sparse logical array from validated CSC components.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed CSC components or a null native
    /// allocation result.
    pub fn logical_sparse(
        &self,
        rows: usize,
        columns: usize,
        column_offsets: &[usize],
        row_indices: &[usize],
        values: &[bool],
    ) -> Result<OwnedArray<'mex>> {
        validate_sparse(rows, columns, column_offsets, row_indices, values.len())?;
        let raw = unsafe { ffi::matrust_create_sparse(rows, columns, values.len().max(1), 1, 0) };
        let raw =
            NonNull::new(raw).ok_or_else(|| Error::allocation("create logical sparse array"))?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                row_indices.as_ptr(),
                ffi::matrust_sparse_ir(raw.as_ptr()).cast_mut(),
                row_indices.len(),
            );
            std::ptr::copy_nonoverlapping(
                column_offsets.as_ptr(),
                ffi::matrust_sparse_jc(raw.as_ptr()).cast_mut(),
                column_offsets.len(),
            );
        }
        let mut array = unsafe { OwnedArray::from_raw(raw) };
        array
            .as_mut()
            .sparse_logicals_mut()?
            .copy_from_slice(values);
        Ok(array)
    }
}

fn validate_dimensions(dimensions: &[usize]) -> Result<()> {
    if dimensions.len() < 2 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "dimensions",
            "MATLAB arrays require at least two dimensions",
        ));
    }
    Ok(())
}

fn element_count(dimensions: &[usize]) -> Result<usize> {
    dimensions.iter().try_fold(1usize, |count, &dimension| {
        count.checked_mul(dimension).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "dimensions",
                "element count overflow",
            )
        })
    })
}

fn checked_slice<'a, T>(ptr: *const T, len: usize, operation: &'static str) -> Result<&'a [T]> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(Error::new(
            ErrorKind::Native,
            operation,
            "null data pointer",
        ));
    }
    if len
        .checked_mul(size_of::<T>())
        .map_or(true, |bytes| bytes > isize::MAX as usize)
    {
        return Err(Error::new(
            ErrorKind::Bounds,
            operation,
            "buffer is too large for a Rust slice",
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

fn checked_slice_mut<'a, T>(
    ptr: *mut T,
    len: usize,
    operation: &'static str,
) -> Result<&'a mut [T]> {
    if len == 0 {
        return Ok(&mut []);
    }
    if ptr.is_null() {
        return Err(Error::new(
            ErrorKind::Native,
            operation,
            "null data pointer",
        ));
    }
    if len
        .checked_mul(size_of::<T>())
        .map_or(true, |bytes| bytes > isize::MAX as usize)
    {
        return Err(Error::new(
            ErrorKind::Bounds,
            operation,
            "buffer is too large for a Rust slice",
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
}

fn validate_sparse(
    rows: usize,
    columns: usize,
    offsets: &[usize],
    indices: &[usize],
    values: usize,
) -> Result<()> {
    let expected_offsets = columns
        .checked_add(1)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "sparse", "column count overflow"))?;
    if offsets.len() != expected_offsets || offsets.first() != Some(&0) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "sparse",
            "invalid column offsets",
        ));
    }
    if indices.len() != values || offsets.last() != Some(&values) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "sparse",
            "index/value length mismatch",
        ));
    }
    for pair in offsets.windows(2) {
        if pair[0] > pair[1] {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "sparse",
                "column offsets are not monotonic",
            ));
        }
    }
    for range in offsets.windows(2) {
        let column_rows = &indices[range[0]..range[1]];
        if column_rows.iter().any(|&row| row >= rows)
            || column_rows.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "sparse",
                "row indices must be sorted, unique, and in bounds",
            ));
        }
    }
    Ok(())
}
