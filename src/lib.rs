//! Safe, lifetime-aware Rust bindings for MATLAB's Matrix, MEX and MAT-file APIs.
//!
//! The safe API has three ownership categories: [`OwnedArray`] is destroyed by
//! Rust, [`ArrayRef`] is a read-only borrow, and [`WorkspaceRef`] is a borrow of
//! MATLAB workspace memory that also prevents calls which may invalidate it.

#[doc(hidden)]
#[path = "entrypoint.rs"]
pub mod __private;
mod array;
mod error;
#[allow(dead_code)]
mod ffi;
#[allow(dead_code)]
mod runtime;

pub use array::{
    ArrayMut, ArrayRef, Class, Complex, Numeric, OwnedArray, PersistentArray, SparseIndices,
    UninitNumeric,
};
pub use error::{Error, ErrorKind, MatError, Result};
use std::{
    ffi::{c_char, c_void, CStr, CString},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};

pub fn eps() -> f64 {
    unsafe { ffi::matrust_eps() }
}
pub fn inf() -> f64 {
    unsafe { ffi::matrust_inf() }
}
pub fn nan() -> f64 {
    unsafe { ffi::matrust_nan() }
}
pub fn is_finite(value: f64) -> bool {
    unsafe { ffi::matrust_is_finite(value) != 0 }
}
pub fn is_inf(value: f64) -> bool {
    unsafe { ffi::matrust_is_inf(value) != 0 }
}
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
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.raw.as_ptr(), self.len) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.raw.as_ptr(), self.len) }
    }
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

    pub fn function_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(ffi::matrust_function_name()) }
    }

    pub fn printf(&mut self, text: &str) -> Result<()> {
        let text = CString::new(text)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "printf", "text contains NUL"))?;
        let status = unsafe { ffi::matrust_printf(text.as_ptr()) };
        (status >= 0)
            .then_some(())
            .ok_or_else(|| Error::native_status("printf", status))
    }

    pub fn warning(&mut self, id: &CStr, text: &str) -> Result<()> {
        let text = CString::new(text)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "warning", "text contains NUL"))?;
        unsafe { ffi::matrust_warning(id.as_ptr(), text.as_ptr()) };
        Ok(())
    }

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

    pub fn lock(&mut self) {
        unsafe { ffi::matrust_lock() }
    }
    pub fn unlock(&mut self) {
        unsafe { ffi::matrust_unlock() }
    }
    pub fn is_locked(&self) -> bool {
        unsafe { ffi::matrust_is_locked() != 0 }
    }

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

    pub fn call(
        &mut self,
        name: &CStr,
        inputs: &[ArrayRef<'_>],
        output_count: usize,
    ) -> Result<Vec<OwnedArray<'mex>>> {
        runtime::require_unborrowed("call MATLAB")?;
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
        let mut result = Vec::with_capacity(output_count);
        for raw in outputs {
            let raw = NonNull::new(raw).ok_or_else(|| {
                Error::new(
                    ErrorKind::Callback,
                    "call MATLAB",
                    "MATLAB returned a null output",
                )
            })?;
            result.push(unsafe { OwnedArray::from_raw(raw) });
        }
        Ok(result)
    }

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
    Caller,
    Global,
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
    pub fn as_ref(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }
}

pub struct Inputs<'mex> {
    values: Vec<ArrayRef<'mex>>,
}
impl<'mex> Inputs<'mex> {
    fn new(values: Vec<ArrayRef<'mex>>) -> Self {
        Self { values }
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn get(&self, index: usize) -> Option<ArrayRef<'mex>> {
        self.values.get(index).copied()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ArrayRef<'mex>> + '_ {
        self.values.iter().copied()
    }
}

pub struct Outputs<'mex> {
    values: Vec<Option<OwnedArray<'mex>>>,
}
impl<'mex> Outputs<'mex> {
    fn new(count: usize) -> Self {
        Self {
            values: (0..count).map(|_| None).collect(),
        }
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
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
    Default,
    V4,
    V6,
    V7,
    V73,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OpenMode {
    Read,
    Update,
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

pub struct MatFile<'mex, 'ctx> {
    raw: Option<NonNull<ffi::RawFile>>,
    mode: OpenMode,
    _context: &'ctx Matlab<'mex>,
}
impl<'mex, 'ctx> MatFile<'mex, 'ctx> {
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
    pub fn create(context: &'ctx Matlab<'mex>, path: impl AsRef<Path>) -> Result<Self> {
        Self::open(context, path, OpenMode::Write(MatVersion::V7))
    }
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
    pub fn last_error(&mut self) -> MatError {
        MatError(unsafe { ffi::matrust_mat_error(self.ptr()) })
    }
    pub fn stream(&mut self) -> Option<FileStream<'_>> {
        NonNull::new(unsafe { ffi::matrust_mat_stream(self.ptr()) }).map(|raw| FileStream {
            raw,
            _borrow: PhantomData,
            _thread: PhantomData,
        })
    }
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

pub struct FileStream<'a> {
    raw: NonNull<c_void>,
    _borrow: PhantomData<&'a mut ()>,
    _thread: PhantomData<Rc<()>>,
}
impl FileStream<'_> {
    pub fn is_eof(&self) -> bool {
        unsafe { ffi::matrust_stream_eof(self.raw.as_ptr()) != 0 }
    }
    pub fn has_error(&self) -> bool {
        unsafe { ffi::matrust_stream_error(self.raw.as_ptr()) != 0 }
    }
    pub fn clear_error(&mut self) {
        unsafe { ffi::matrust_stream_clear(self.raw.as_ptr()) }
    }
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

pub struct ArrayInfo<'mex> {
    raw: NonNull<ffi::RawArray>,
    context: PhantomData<&'mex Matlab<'mex>>,
}
impl ArrayInfo<'_> {
    pub fn as_ref(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }
    pub fn dimensions(&self) -> &[usize] {
        self.as_ref().dimensions()
    }
    pub fn numel(&self) -> usize {
        self.as_ref().numel()
    }
    pub fn class(&self) -> Option<Class> {
        self.as_ref().class()
    }
    pub fn class_name(&self) -> &CStr {
        self.as_ref().class_name()
    }
    pub fn is_sparse(&self) -> bool {
        self.as_ref().is_sparse()
    }
    pub fn is_complex(&self) -> bool {
        self.as_ref().is_complex()
    }
    pub fn is_global(&self) -> bool {
        self.as_ref().is_from_global_workspace()
    }
}
impl Drop for ArrayInfo<'_> {
    fn drop(&mut self) {
        unsafe { ffi::matrust_array_destroy(self.raw.as_ptr()) }
    }
}

pub struct Named<T> {
    pub name: CString,
    pub value: T,
}
pub struct Variables<'mex, 'ctx> {
    file: MatFile<'mex, 'ctx>,
    remaining: usize,
    done: bool,
}
impl<'mex, 'ctx> Variables<'mex, 'ctx> {
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

pub struct VariableInfos<'mex, 'ctx> {
    file: MatFile<'mex, 'ctx>,
    remaining: usize,
    done: bool,
}
impl<'mex, 'ctx> VariableInfos<'mex, 'ctx> {
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

/// The deliberately unsafe escape hatch for APIs not yet modeled by the
/// ownership layer. Safe applications should prefer the types above.
pub mod raw {
    pub use crate::ffi::*;
}
