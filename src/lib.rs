//! Safe, lifetime-aware Rust bindings for MATLAB's Matrix, MEX and MAT-file APIs.
//!
//! The safe API has three ownership categories: [`OwnedArray`] is destroyed by
//! Rust, [`ArrayRef`] is a read-only borrow, and [`WorkspaceRef`] is a borrow of
//! MATLAB workspace memory that also prevents calls which may invalidate it.

#![deny(missing_docs)]

#[doc(hidden)]
#[path = "entrypoint.rs"]
pub mod __private;
mod array;
mod error;
#[allow(dead_code, missing_docs)]
mod ffi;
#[allow(dead_code)]
mod runtime;

pub use array::{
    ArrayMut, ArrayRef, Class, Complex, Numeric, OwnedArray, PersistentArray, SparseIndices,
    UninitNumeric,
};
pub use error::{Error, ErrorKind, MatError, Result};

/// Function-by-function coverage of the R2025a API-800 C headers.
#[doc = include_str!("../docs/API_COVERAGE.md")]
pub mod api_coverage {}

use std::{
    ffi::{c_char, c_void, CStr, CString},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};

/// Return MATLAB's floating-point epsilon constant.
pub fn eps() -> f64 {
    unsafe { ffi::matrust_eps() }
}
/// Return MATLAB's positive infinity constant.
pub fn inf() -> f64 {
    unsafe { ffi::matrust_inf() }
}
/// Return MATLAB's quiet NaN constant.
pub fn nan() -> f64 {
    unsafe { ffi::matrust_nan() }
}
/// Test a floating-point value with MATLAB's finite predicate.
pub fn is_finite(value: f64) -> bool {
    unsafe { ffi::matrust_is_finite(value) != 0 }
}
/// Test a floating-point value with MATLAB's infinity predicate.
pub fn is_inf(value: f64) -> bool {
    unsafe { ffi::matrust_is_inf(value) != 0 }
}
/// Test a floating-point value with MATLAB's NaN predicate.
pub fn is_nan(value: f64) -> bool {
    unsafe { ffi::matrust_is_nan(value) != 0 }
}

/// A byte buffer allocated by MATLAB's memory manager. It is initialized by
/// `mxCalloc` and is always released with the matching `mxFree`.
pub struct MxBuffer<'mex> {
    raw: NonNull<u8>,
    len: usize,
    _brand: PhantomData<fn(&'mex mut ()) -> &'mex mut ()>,
    _thread: PhantomData<Rc<()>>,
}
impl MxBuffer<'_> {
    /// Return the allocation length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Test whether the allocation has zero logical length.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Borrow the allocation as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.raw.as_ptr(), self.len) }
    }
    /// Mutably borrow the allocation as bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.raw.as_ptr(), self.len) }
    }
    /// Resize the allocation with `mxRealloc`.
    pub fn resize(&mut self, len: usize) -> Result<()> {
        let raw = unsafe { ffi::matrust_realloc(self.raw.as_ptr().cast(), len.max(1)) };
        self.raw = NonNull::new(raw.cast())
            .ok_or_else(|| Error::allocation("reallocate MATLAB buffer"))?;
        self.len = len;
        Ok(())
    }
}
impl Drop for MxBuffer<'_> {
    fn drop(&mut self) {
        unsafe { ffi::matrust_free(self.raw.as_ptr().cast()) }
    }
}

/// A generative proof of the current MATLAB invocation and thread.
pub struct Matlab<'mex> {
    _brand: PhantomData<fn(&'mex mut ()) -> &'mex mut ()>,
    _thread: PhantomData<Rc<()>>,
}

impl<'mex> Matlab<'mex> {
    pub(crate) fn new() -> Self {
        Self {
            _brand: PhantomData,
            _thread: PhantomData,
        }
    }

    /// Attach to an already-running MEX invocation.
    ///
    /// # Safety
    /// The caller must be executing synchronously on MATLAB's MEX thread and
    /// must not call the MATLAB C API concurrently from another thread.
    pub unsafe fn attach() -> Self {
        Self::new()
    }

    /// Return the name by which MATLAB invoked the current MEX function.
    pub fn function_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(ffi::matrust_function_name()) }
    }

    /// Print literal UTF-8 text to MATLAB's command window.
    pub fn printf(&mut self, text: &str) -> Result<()> {
        let text = CString::new(text)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "printf", "text contains NUL"))?;
        let status = unsafe { ffi::matrust_printf(text.as_ptr()) };
        (status >= 0)
            .then_some(())
            .ok_or_else(|| Error::native_status("printf", status))
    }

    /// Issue a MATLAB warning with an identifier and literal message.
    pub fn warning(&mut self, id: &CStr, text: &str) -> Result<()> {
        let text = CString::new(text)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "warning", "text contains NUL"))?;
        unsafe { ffi::matrust_warning(id.as_ptr(), text.as_ptr()) };
        Ok(())
    }

    /// Allocate zeroed MATLAB-managed memory.
    pub fn calloc(&self, len: usize) -> Result<MxBuffer<'mex>> {
        let bytes = if len == 0 { 1 } else { len };
        let raw = unsafe { ffi::matrust_calloc(1, bytes) };
        let raw =
            NonNull::new(raw.cast()).ok_or_else(|| Error::allocation("allocate MATLAB buffer"))?;
        Ok(MxBuffer {
            raw,
            len,
            _brand: PhantomData,
            _thread: PhantomData,
        })
    }

    /// Prevent MATLAB from clearing the current MEX module.
    pub fn lock(&mut self) {
        unsafe { ffi::matrust_lock() }
    }
    /// Release one MEX module lock.
    pub fn unlock(&mut self) {
        unsafe { ffi::matrust_unlock() }
    }
    /// Test whether the current MEX module is locked.
    pub fn is_locked(&self) -> bool {
        unsafe { ffi::matrust_is_locked() != 0 }
    }

    /// Copy a variable from a MATLAB workspace into a Rust-owned array.
    pub fn workspace_get(&mut self, space: Workspace, name: &CStr) -> Result<OwnedArray<'mex>> {
        let raw = unsafe { ffi::matrust_workspace_get(space.as_ptr(), name.as_ptr()) };
        NonNull::new(raw)
            .map(|raw| unsafe { OwnedArray::from_raw(raw) })
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::Workspace,
                    "get workspace variable",
                    format!("{name:?}"),
                )
            })
    }

    /// Borrow a MATLAB-owned workspace variable.
    ///
    /// The mutable context borrow prevents callbacks and workspace mutations
    /// while the returned pointer may be invalidated by MATLAB.
    pub fn workspace_borrow<'a>(
        &'a mut self,
        space: Workspace,
        name: &CStr,
    ) -> Result<WorkspaceRef<'a>> {
        let raw = unsafe { ffi::matrust_workspace_borrow(space.as_ptr(), name.as_ptr()) };
        let raw = NonNull::new(raw.cast_mut()).ok_or_else(|| {
            Error::new(
                ErrorKind::Workspace,
                "borrow workspace variable",
                format!("{name:?}"),
            )
        })?;
        Ok(WorkspaceRef {
            raw,
            _guard: runtime::begin_external_borrow(),
            _context: PhantomData,
        })
    }

    /// Copy a borrowed array into a MATLAB workspace.
    pub fn workspace_put(
        &mut self,
        space: Workspace,
        name: &CStr,
        value: ArrayRef<'_>,
    ) -> Result<()> {
        runtime::require_unborrowed("put workspace variable")?;
        let status =
            unsafe { ffi::matrust_workspace_put(space.as_ptr(), name.as_ptr(), value.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::native_status("put workspace variable", status))
        }
    }

    /// Call a MATLAB function through the trapping API.
    pub fn call(
        &mut self,
        name: &CStr,
        inputs: &[ArrayRef<'_>],
        output_count: usize,
    ) -> Result<Vec<OwnedArray<'mex>>> {
        runtime::require_unborrowed("call MATLAB")?;
        if output_count > 50 || inputs.len() > 50 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "call MATLAB",
                "MATLAB callbacks support at most 50 inputs and 50 outputs",
            ));
        }
        let count = i32::try_from(output_count)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "call MATLAB", "too many outputs"))?;
        let mut outputs = vec![std::ptr::null_mut(); output_count];
        let mut raw_inputs: Vec<*mut ffi::RawArray> =
            inputs.iter().map(|v| v.as_ptr().cast_mut()).collect();
        let input_count = i32::try_from(raw_inputs.len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "call MATLAB", "too many inputs"))?;
        let trap = unsafe {
            ffi::matrust_call_with_trap(
                count,
                outputs.as_mut_ptr(),
                input_count,
                raw_inputs.as_mut_ptr(),
                name.as_ptr(),
            )
        };
        if !trap.is_null() {
            for raw in outputs {
                if !raw.is_null() {
                    unsafe { ffi::matrust_array_destroy(raw) }
                }
            }
            unsafe { ffi::matrust_array_destroy(trap) };
            return Err(Error::new(
                ErrorKind::Callback,
                "call MATLAB",
                format!("{name:?} failed"),
            ));
        }
        if outputs.iter().any(|raw| raw.is_null()) {
            for raw in outputs {
                if !raw.is_null() {
                    unsafe { ffi::matrust_array_destroy(raw) }
                }
            }
            return Err(Error::new(
                ErrorKind::Callback,
                "call MATLAB",
                "MATLAB returned a null output",
            ));
        }
        let mut result = Vec::with_capacity(output_count);
        for raw in outputs {
            let raw = NonNull::new(raw).expect("outputs were checked for null");
            result.push(unsafe { OwnedArray::from_raw(raw) });
        }
        Ok(result)
    }

    /// Evaluate MATLAB source through the trapping API.
    pub fn eval(&mut self, command: &str) -> Result<()> {
        runtime::require_unborrowed("evaluate MATLAB")?;
        let command = CString::new(command).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "evaluate MATLAB",
                "command contains NUL",
            )
        })?;
        let trap = unsafe { ffi::matrust_eval_with_trap(command.as_ptr()) };
        if trap.is_null() {
            Ok(())
        } else {
            unsafe { ffi::matrust_array_destroy(trap) };
            Err(Error::new(
                ErrorKind::Callback,
                "evaluate MATLAB",
                "command failed",
            ))
        }
    }
}

/// MATLAB's three workspace namespaces.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Workspace {
    /// Calling function's workspace.
    Caller,
    /// Global workspace.
    Global,
    /// Base workspace.
    Base,
}
impl Workspace {
    fn as_ptr(self) -> *const c_char {
        match self {
            Self::Caller => c"caller".as_ptr(),
            Self::Global => c"global".as_ptr(),
            Self::Base => c"base".as_ptr(),
        }
    }
}

/// A pointer returned by `mexGetVariablePtr`. It cannot outlive the exclusive
/// borrow of [`Matlab`], so a callback or workspace mutation cannot invalidate it.
///
/// ```compile_fail
/// # use matrust::{Matlab, Workspace};
/// fn invalid(cx: &mut Matlab<'_>) {
///     let view = cx.workspace_borrow(Workspace::Caller, c"x").unwrap();
///     let _ = cx.eval("clear x");
///     let _ = view.as_ref().numel();
/// }
/// ```
pub struct WorkspaceRef<'a> {
    raw: NonNull<ffi::RawArray>,
    _guard: runtime::ExternalBorrowGuard,
    _context: PhantomData<&'a mut Matlab<'a>>,
}
impl WorkspaceRef<'_> {
    /// Borrow the referenced MATLAB-owned array.
    pub fn as_ref(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }
}

/// Read-only MEX input arguments owned by MATLAB.
pub struct Inputs<'mex> {
    values: Vec<ArrayRef<'mex>>,
}
impl<'mex> Inputs<'mex> {
    fn new(values: Vec<ArrayRef<'mex>>) -> Self {
        Self { values }
    }
    /// Return the number of input arguments.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Test whether there are no input arguments.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Borrow an input argument by index.
    pub fn get(&self, index: usize) -> Option<ArrayRef<'mex>> {
        self.values.get(index).copied()
    }
    /// Iterate over every input argument.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ArrayRef<'mex>> + '_ {
        self.values.iter().copied()
    }
}

/// MEX output slots that accept ownership of Rust-created arrays.
pub struct Outputs<'mex> {
    values: Vec<Option<OwnedArray<'mex>>>,
}
impl<'mex> Outputs<'mex> {
    fn new(count: usize) -> Self {
        Self {
            values: (0..count).map(|_| None).collect(),
        }
    }
    /// Return the number of requested output slots.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Test whether MATLAB requested no outputs.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Transfer an owned array into an output slot.
    pub fn set(&mut self, index: usize, value: OwnedArray<'mex>) -> Result<()> {
        let len = self.values.len();
        let slot = self
            .values
            .get_mut(index)
            .ok_or_else(|| Error::bounds("set output", index, len))?;
        if slot.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "set output",
                "output slot already set",
            ));
        }
        *slot = Some(value);
        Ok(())
    }
    /// Take back an output array that was previously set.
    pub fn take(&mut self, index: usize) -> Result<Option<OwnedArray<'mex>>> {
        let len = self.values.len();
        self.values
            .get_mut(index)
            .ok_or_else(|| Error::bounds("take output", index, len))
            .map(Option::take)
    }
}

/// All creation formats accepted by `matOpen`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MatVersion {
    /// MATLAB's default output format.
    Default,
    /// Level-4 MAT-file format.
    V4,
    /// Version-6 MAT-file format.
    V6,
    /// Version-7 compressed MAT-file format.
    V7,
    /// Version-7.3 HDF5 MAT-file format.
    V73,
}
/// Access mode for a MAT-file.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OpenMode {
    /// Open an existing file for reading.
    Read,
    /// Open an existing file for reading and writing.
    Update,
    /// Create or replace a file in the selected format.
    Write(MatVersion),
}
impl OpenMode {
    fn as_cstr(self) -> &'static CStr {
        match self {
            Self::Read => c"r",
            Self::Update => c"u",
            Self::Write(MatVersion::Default) => c"w",
            Self::Write(MatVersion::V4) => c"w4",
            Self::Write(MatVersion::V6) => c"w6",
            Self::Write(MatVersion::V7) => c"w7",
            Self::Write(MatVersion::V73) => c"w7.3",
        }
    }
}

/// Owned `MATFile` handle tied to the creating MATLAB context.
pub struct MatFile<'mex, 'ctx> {
    raw: Option<NonNull<ffi::RawFile>>,
    mode: OpenMode,
    _context: &'ctx Matlab<'mex>,
}
impl<'mex, 'ctx> MatFile<'mex, 'ctx> {
    /// Open a MAT-file with an explicit access mode.
    pub fn open(
        context: &'ctx Matlab<'mex>,
        path: impl AsRef<Path>,
        mode: OpenMode,
    ) -> Result<Self> {
        let path = path_cstring(path.as_ref())?;
        let raw = unsafe { ffi::matrust_mat_open(path.as_ptr(), mode.as_cstr().as_ptr()) };
        NonNull::new(raw)
            .map(|raw| Self {
                raw: Some(raw),
                mode,
                _context: context,
            })
            .ok_or_else(|| Error::new(ErrorKind::Open, "open MAT-file", format!("{path:?}")))
    }
    /// Create a version-7 MAT-file.
    pub fn create(context: &'ctx Matlab<'mex>, path: impl AsRef<Path>) -> Result<Self> {
        Self::open(context, path, OpenMode::Write(MatVersion::V7))
    }
    /// Create a MAT-file in a selected format.
    pub fn create_with_format(
        context: &'ctx Matlab<'mex>,
        path: impl AsRef<Path>,
        version: MatVersion,
    ) -> Result<Self> {
        Self::open(context, path, OpenMode::Write(version))
    }
    fn ptr(&self) -> *mut ffi::RawFile {
        self.raw.expect("live MAT-file").as_ptr()
    }
    fn require_read(&self) -> Result<()> {
        if matches!(self.mode, OpenMode::Write(_)) {
            Err(Error::new(
                ErrorKind::InvalidMode,
                "read MAT-file",
                "file is write-only",
            ))
        } else {
            Ok(())
        }
    }
    fn require_write(&self) -> Result<()> {
        if self.mode == OpenMode::Read {
            Err(Error::new(
                ErrorKind::InvalidMode,
                "write MAT-file",
                "file is read-only",
            ))
        } else {
            Ok(())
        }
    }
    /// Write a variable by copying a borrowed array.
    pub fn put(&mut self, name: &CStr, value: ArrayRef<'_>) -> Result<()> {
        self.require_write()?;
        validate_name(name)?;
        let mut code = 0;
        let status = unsafe {
            ffi::matrust_mat_put(self.ptr(), name.as_ptr(), value.as_ptr(), 0, &mut code)
        };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::native(
                ErrorKind::Write,
                "put MAT variable",
                format!("{name:?}"),
                status,
                code,
            ))
        }
    }
    /// Write a variable marked as global.
    pub fn put_global(&mut self, name: &CStr, value: ArrayRef<'_>) -> Result<()> {
        self.require_write()?;
        validate_name(name)?;
        let mut code = 0;
        let status = unsafe {
            ffi::matrust_mat_put(self.ptr(), name.as_ptr(), value.as_ptr(), 1, &mut code)
        };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::native(
                ErrorKind::Write,
                "put global MAT variable",
                format!("{name:?}"),
                status,
                code,
            ))
        }
    }
    /// Read a complete variable into an owned array.
    pub fn get(&mut self, name: &CStr) -> Result<OwnedArray<'mex>> {
        self.require_read()?;
        validate_name(name)?;
        let mut code = 0;
        let raw = unsafe { ffi::matrust_mat_get(self.ptr(), name.as_ptr(), 0, &mut code) };
        NonNull::new(raw)
            .map(|raw| unsafe { OwnedArray::from_raw(raw) })
            .ok_or_else(|| {
                Error::native(
                    ErrorKind::Read,
                    "get MAT variable",
                    format!("{name:?}"),
                    -1,
                    code,
                )
            })
    }
    /// Read only a variable header and metadata.
    pub fn info(&mut self, name: &CStr) -> Result<ArrayInfo<'mex>> {
        self.require_read()?;
        validate_name(name)?;
        let mut code = 0;
        let raw = unsafe { ffi::matrust_mat_get(self.ptr(), name.as_ptr(), 1, &mut code) };
        NonNull::new(raw)
            .map(|raw| ArrayInfo {
                raw,
                context: PhantomData,
            })
            .ok_or_else(|| {
                Error::native(
                    ErrorKind::Read,
                    "get MAT header",
                    format!("{name:?}"),
                    -1,
                    code,
                )
            })
    }
    /// Delete a variable from an update-mode file.
    pub fn delete(&mut self, name: &CStr) -> Result<()> {
        self.require_write()?;
        validate_name(name)?;
        let mut code = 0;
        let status = unsafe { ffi::matrust_mat_delete(self.ptr(), name.as_ptr(), &mut code) };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::native(
                ErrorKind::Write,
                "delete MAT variable",
                format!("{name:?}"),
                status,
                code,
            ))
        }
    }
    /// Return every variable name in the file.
    pub fn variables(&mut self) -> Result<Vec<CString>> {
        self.require_read()?;
        let (mut count, mut code) = (0, 0);
        let ptr = unsafe { ffi::matrust_mat_directory(self.ptr(), &mut count, &mut code) };
        if count < 0 || (count > 0 && ptr.is_null()) {
            return Err(Error::native(
                ErrorKind::Read,
                "MAT directory",
                "invalid directory",
                count,
                code,
            ));
        }
        struct Dir(*mut *mut c_char);
        impl Drop for Dir {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { ffi::matrust_free(self.0.cast()) }
                }
            }
        }
        let names = Dir(ptr);
        (0..count as usize)
            .map(|i| {
                let p = unsafe { *names.0.add(i) };
                if p.is_null() {
                    Err(Error::new(ErrorKind::Native, "MAT directory", "null name"))
                } else {
                    Ok(unsafe { CStr::from_ptr(p) }.to_owned())
                }
            })
            .collect()
    }
    /// Return the unmodified current `matGetErrno` value.
    pub fn last_error(&mut self) -> MatError {
        MatError(unsafe { ffi::matrust_mat_error(self.ptr()) })
    }
    /// Borrow limited diagnostics from the file's underlying C stream.
    pub fn stream(&mut self) -> Option<FileStream<'_>> {
        NonNull::new(unsafe { ffi::matrust_mat_stream(self.ptr()) }).map(|raw| FileStream {
            raw,
            _borrow: PhantomData,
            _thread: PhantomData,
        })
    }
    /// Close the file and report the native close status.
    pub fn close(mut self) -> Result<()> {
        let raw = self.raw.take().expect("live MAT-file");
        let status = unsafe { ffi::matrust_mat_close(raw.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::native_status("close MAT-file", status))
        }
    }
}
impl Drop for MatFile<'_, '_> {
    fn drop(&mut self) {
        if let Some(raw) = self.raw.take() {
            unsafe {
                ffi::matrust_mat_close(raw.as_ptr());
            }
        }
    }
}

/// Restricted diagnostic view of a MAT-file's underlying C stream.
pub struct FileStream<'a> {
    raw: NonNull<c_void>,
    _borrow: PhantomData<&'a mut ()>,
    _thread: PhantomData<Rc<()>>,
}
impl FileStream<'_> {
    /// Test the stream end-of-file indicator.
    pub fn is_eof(&self) -> bool {
        unsafe { ffi::matrust_stream_eof(self.raw.as_ptr()) != 0 }
    }
    /// Test the stream error indicator.
    pub fn has_error(&self) -> bool {
        unsafe { ffi::matrust_stream_error(self.raw.as_ptr()) != 0 }
    }
    /// Clear the stream error and end-of-file indicators.
    pub fn clear_error(&mut self) {
        unsafe { ffi::matrust_stream_clear(self.raw.as_ptr()) }
    }
    /// Return the current byte position.
    pub fn position(&self) -> Result<u64> {
        u64::try_from(unsafe { ffi::matrust_stream_position(self.raw.as_ptr()) }).map_err(|_| {
            Error::new(
                ErrorKind::Native,
                "stream position",
                "native stream returned a negative position",
            )
        })
    }
}

/// Owned metadata-only array header returned by a MAT-file info operation.
/// It intentionally cannot become an [`ArrayRef`] because MATLAB stores
/// non-dereferenceable sentinels in its data pointers.
///
/// ```compile_fail
/// # use matrust::ArrayInfo;
/// fn cannot_read_data(info: &ArrayInfo<'_>) {
///     let _ = info.as_ref().data::<f64>();
/// }
/// ```
pub struct ArrayInfo<'mex> {
    raw: NonNull<ffi::RawArray>,
    context: PhantomData<&'mex Matlab<'mex>>,
}
impl ArrayInfo<'_> {
    fn view(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }
    /// Borrow the variable dimensions.
    pub fn dimensions(&self) -> &[usize] {
        self.view().dimensions()
    }
    /// Return the variable element count.
    pub fn numel(&self) -> usize {
        self.view().numel()
    }
    /// Return the variable class.
    pub fn class(&self) -> Option<Class> {
        self.view().class()
    }
    /// Return the variable class name.
    pub fn class_name(&self) -> &CStr {
        self.view().class_name()
    }
    /// Test whether the variable uses sparse storage.
    pub fn is_sparse(&self) -> bool {
        self.view().is_sparse()
    }
    /// Test whether the variable uses complex storage.
    pub fn is_complex(&self) -> bool {
        self.view().is_complex()
    }
    /// Test whether the variable is marked global.
    pub fn is_global(&self) -> bool {
        self.view().is_from_global_workspace()
    }
}
impl Drop for ArrayInfo<'_> {
    fn drop(&mut self) {
        unsafe { ffi::matrust_array_destroy(self.raw.as_ptr()) }
    }
}

/// A MAT-file value paired with its variable name.
pub struct Named<T> {
    /// Variable name.
    pub name: CString,
    /// Variable value or metadata.
    pub value: T,
}
/// Sequential iterator over complete variables in a MAT-file.
pub struct Variables<'mex, 'ctx> {
    file: MatFile<'mex, 'ctx>,
    remaining: usize,
    done: bool,
}
impl<'mex, 'ctx> Variables<'mex, 'ctx> {
    /// Open a file for sequential variable reading.
    pub fn open(context: &'ctx Matlab<'mex>, path: impl AsRef<Path>) -> Result<Self> {
        let mut directory = MatFile::open(context, &path, OpenMode::Read)?;
        let remaining = directory.variables()?.len();
        directory.close()?;
        Ok(Self {
            file: MatFile::open(context, path, OpenMode::Read)?,
            remaining,
            done: false,
        })
    }
    /// Close the underlying MAT-file and report its status.
    pub fn close(self) -> Result<()> {
        self.file.close()
    }
}
impl<'mex, 'ctx> Iterator for Variables<'mex, 'ctx> {
    type Item = Result<Named<OwnedArray<'mex>>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.remaining == 0 {
            self.done = true;
            return None;
        }
        let mut name: *const c_char = std::ptr::null();
        let mut code = 0;
        let raw = unsafe { ffi::matrust_mat_next(self.file.ptr(), &mut name, 0, &mut code) };
        let raw = match NonNull::new(raw) {
            Some(raw) => raw,
            None => {
                self.done = true;
                return Some(Err(Error::native(
                    ErrorKind::UnexpectedEnd,
                    "next MAT variable",
                    "native reader ended early",
                    -1,
                    code,
                )));
            }
        };
        if name.is_null() {
            self.done = true;
            unsafe { ffi::matrust_array_destroy(raw.as_ptr()) };
            return Some(Err(Error::new(
                ErrorKind::Native,
                "next MAT variable",
                "null variable name",
            )));
        }
        let name = unsafe { CStr::from_ptr(name) }.to_owned();
        self.remaining -= 1;
        let value = unsafe { OwnedArray::from_raw(raw) };
        Some(Ok(Named { name, value }))
    }
}

/// Sequential iterator over variable headers in a MAT-file.
pub struct VariableInfos<'mex, 'ctx> {
    file: MatFile<'mex, 'ctx>,
    remaining: usize,
    done: bool,
}
impl<'mex, 'ctx> VariableInfos<'mex, 'ctx> {
    /// Open a file for sequential metadata reading.
    pub fn open(context: &'ctx Matlab<'mex>, path: impl AsRef<Path>) -> Result<Self> {
        let mut directory = MatFile::open(context, &path, OpenMode::Read)?;
        let remaining = directory.variables()?.len();
        directory.close()?;
        Ok(Self {
            file: MatFile::open(context, path, OpenMode::Read)?,
            remaining,
            done: false,
        })
    }
    /// Close the underlying MAT-file and report its status.
    pub fn close(self) -> Result<()> {
        self.file.close()
    }
}
impl<'mex, 'ctx> Iterator for VariableInfos<'mex, 'ctx> {
    type Item = Result<Named<ArrayInfo<'mex>>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.remaining == 0 {
            self.done = true;
            return None;
        }
        let mut name: *const c_char = std::ptr::null();
        let mut code = 0;
        let raw = unsafe { ffi::matrust_mat_next(self.file.ptr(), &mut name, 1, &mut code) };
        let raw = match NonNull::new(raw) {
            Some(raw) => raw,
            None => {
                self.done = true;
                return Some(Err(Error::native(
                    ErrorKind::UnexpectedEnd,
                    "next MAT header",
                    "native reader ended early",
                    -1,
                    code,
                )));
            }
        };
        if name.is_null() {
            self.done = true;
            unsafe { ffi::matrust_array_destroy(raw.as_ptr()) };
            return Some(Err(Error::new(
                ErrorKind::Native,
                "next MAT header",
                "null variable name",
            )));
        }
        let name = unsafe { CStr::from_ptr(name) }.to_owned();
        self.remaining -= 1;
        let value = ArrayInfo {
            raw,
            context: PhantomData,
        };
        Some(Ok(Named { name, value }))
    }
}

fn validate_name(name: &CStr) -> Result<()> {
    if name.to_bytes().is_empty() {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "variable name",
            "empty name",
        ))
    } else {
        Ok(())
    }
}
fn path_cstring(path: &Path) -> Result<CString> {
    let text = path
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "path", "path is not valid Unicode"))?;
    CString::new(text).map_err(|_| Error::new(ErrorKind::InvalidInput, "path", "path contains NUL"))
}

/// Export a MEX entrypoint for `fn(&mut Matlab, Inputs, &mut Outputs) -> Result<()>`.
#[macro_export]
macro_rules! mex_entrypoint {
    ($handler:path) => {
        #[used]
        #[no_mangle]
        pub static matrust_mex_link: unsafe extern "C" fn() = $crate::__private::matrust_mex_anchor;
        #[no_mangle]
        pub unsafe extern "C" fn matrust_mex_dispatch(
            nlhs: i32,
            plhs: *mut *mut $crate::__private::RawArray,
            nrhs: i32,
            prhs: *const *const $crate::__private::RawArray,
            id: *mut std::ffi::c_char,
            id_len: usize,
            message: *mut std::ffi::c_char,
            message_len: usize,
        ) -> i32 {
            unsafe {
                $crate::__private::dispatch(
                    $handler,
                    nlhs,
                    plhs,
                    nrhs,
                    prhs,
                    id,
                    id_len,
                    message,
                    message_len,
                )
            }
        }
    };
}

#[doc(hidden)]
pub use ffi::RawArray;

/// Unsafe one-to-one escape hatches for published API-800 operations.
///
/// The safe types above cover ordinary use. This module also exposes pointer
/// adoption, non-trapping callbacks, and MATLAB error functions that cannot be
/// made safe without caller-provided invariants. See [`crate::api_coverage`].
#[allow(missing_docs)]
pub mod raw {
    pub use crate::ffi::*;
}
