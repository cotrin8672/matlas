//! MAT-File I/O using the same arrays and error conventions as rustmex.
//!
//! Enter once using [`Matlab::attach`] on the MEX calling thread. File operations
//! are then safe; handles cannot leave the context or move to another thread.
//! Metadata has a distinct type and sequential readers own separate handles.
//!
//! ```no_run
//! use rustmat::{MatFile, Matlab};
//! # fn example(value: &rustmex::mxArray) -> rustmat::Result<()> {
//! // SAFETY: called synchronously from the current MEX invocation; all arrays
//! // are destroyed here or transferred to MATLAB before that invocation ends.
//! let matlab = unsafe { Matlab::attach() };
//! let mut file = MatFile::create(&matlab, "result.mat")?;
//! file.put(c"value", value)?;
//! file.close()?;
//! # Ok(()) }
//! ```

#[doc(hidden)]
#[path = "entrypoint.rs"]
pub mod __private;
mod error;
mod ffi;
mod info;
mod iter;

pub use error::{Error, ErrorKind, MatError, Result};
pub use info::{ArrayInfo, InfoRef};
pub use iter::{Named, VariableInfos, Variables};
use rustmex::{mxArray, MxArray};
use std::{
    ffi::{c_void, CStr, CString},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};

/// Proof, supplied at the unsafe MEX boundary, of a live MATLAB calling context.
/// This token and its file handles are neither Send nor Sync.
pub struct Matlab {
    _thread: PhantomData<Rc<()>>,
}

impl Matlab {
    /// Enter the current MATLAB MEX calling context.
    ///
    /// # Safety
    /// Call only on the thread currently executing a MATLAB MEX function, with
    /// the matlab800 backend and the same MATLAB runtime as this crate. No
    /// concurrent MATLAB API calls are permitted. Drop this token and every
    /// associated handle before returning from that invocation. Full `MxArray`s
    /// obtained here must be destroyed on this thread before return, or moved to
    /// MATLAB through rustmex's output ownership transfer; never retain them in
    /// Rust globals or use borrowed arrays after MATLAB invalidates them.
    pub unsafe fn attach() -> Self {
        Self {
            _thread: PhantomData,
        }
    }
}

/// All creation formats accepted by matOpen (including its legacy aliases).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MatVersion {
    /// Native default (`w`). Use an explicit version for a stable file format.
    Default,
    V4,
    /// Version 5 file compatible with MATLAB 6 (`w6`, alias `wL`).
    V6,
    /// Compressed v7 (`w7`, alias `wz`).
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

/// An owned MAT-file. Call `close` to observe final write failures.
///
/// ```compile_fail
/// fn send<T: Send>() {}
/// send::<rustmat::MatFile<'static>>();
/// ```
/// ```compile_fail
/// fn sync<T: Sync>() {}
/// sync::<rustmat::MatFile<'static>>();
/// ```
/// A file cannot outlive its context:
/// ```compile_fail
/// let file;
/// {
///     let context = unsafe { rustmat::Matlab::attach() };
///     file = rustmat::MatFile::create(&context, "data.mat").unwrap();
/// }
/// file.close().unwrap();
/// ```
pub struct MatFile<'matlab> {
    raw: Option<NonNull<ffi::RawFile>>,
    mode: OpenMode,
    pub(crate) context: &'matlab Matlab,
}

impl<'matlab> MatFile<'matlab> {
    /// Open a file. Update requires an existing file; Write truncates it.
    pub fn open(context: &'matlab Matlab, path: impl AsRef<Path>, mode: OpenMode) -> Result<Self> {
        let path = path_cstring(path.as_ref())?;
        // SAFETY: the context proves the runtime/thread contract; both strings
        // are NUL terminated and live across the synchronous native call.
        let raw = unsafe { ffi::rustmat_open(path.as_ptr(), mode.as_cstr().as_ptr()) };
        let raw = NonNull::new(raw)
            .ok_or_else(|| Error::new(ErrorKind::Open, "open", format!("{path:?} ({mode:?})")))?;
        Ok(Self {
            raw: Some(raw),
            mode,
            context,
        })
    }
    /// Create a compressed v7 file, replacing any existing contents.
    pub fn create(context: &'matlab Matlab, path: impl AsRef<Path>) -> Result<Self> {
        Self::create_with_format(context, path, MatVersion::V7)
    }
    /// Create a file in the requested format, replacing existing contents.
    pub fn create_with_format(
        context: &'matlab Matlab,
        path: impl AsRef<Path>,
        version: MatVersion,
    ) -> Result<Self> {
        Self::open(context, path, OpenMode::Write(version))
    }
    pub(crate) fn ptr(&self) -> *mut ffi::RawFile {
        self.raw.expect("live MAT-file").as_ptr()
    }
    fn require_read(&self, operation: &'static str) -> Result<()> {
        if matches!(self.mode, OpenMode::Write(_)) {
            return Err(self.mode_error(operation));
        }
        Ok(())
    }
    fn require_write(&self, operation: &'static str) -> Result<()> {
        if self.mode == OpenMode::Read {
            return Err(self.mode_error(operation));
        }
        Ok(())
    }
    fn mode_error(&self, operation: &'static str) -> Error {
        Error::new(
            ErrorKind::InvalidMode,
            operation,
            format!("file is open in {:?} mode", self.mode),
        )
    }
    /// Save without taking ownership or copying the array in Rust.
    pub fn put(&mut self, name: &CStr, value: &mxArray) -> Result<()> {
        self.put_impl(name, value, false)
    }
    /// Save with the global-workspace flag used by MATLAB's load command.
    pub fn put_global(&mut self, name: &CStr, value: &mxArray) -> Result<()> {
        self.put_impl(name, value, true)
    }
    fn put_impl(&mut self, name: &CStr, value: &mxArray, global: bool) -> Result<()> {
        self.require_write("put")?;
        validate_name(name)?;
        let mut code = 0;
        // SAFETY: live exclusive file, valid borrowed array and C string.
        let status = unsafe {
            ffi::rustmat_put(
                self.ptr(),
                name.as_ptr(),
                value,
                i32::from(global),
                &mut code,
            )
        };
        if status != 0 {
            return Err(Error::native(
                ErrorKind::Write,
                "put",
                format!("{name:?}"),
                status,
                code,
            ));
        }
        Ok(())
    }
    /// Read a complete array. Missing names and read failures are both errors.
    /// The array owns its allocation independently of this file. The context's
    /// MEX lifetime contract still applies to the resulting rustmex value.
    pub fn get(&mut self, name: &CStr) -> Result<MxArray> {
        let raw = self.get_raw(name, false)?;
        // SAFETY: matGetVariable returned a new, non-null, full owned array.
        Ok(unsafe { MxArray::assume_responsibility_ptr(raw.as_ptr()) })
    }
    /// Read only a header, never convertible into a full mxArray.
    pub fn info(&mut self, name: &CStr) -> Result<ArrayInfo<'matlab>> {
        let raw = self.get_raw(name, true)?;
        Ok(ArrayInfo::new(ffi::Array(raw), self.context))
    }
    fn get_raw(&mut self, name: &CStr, info: bool) -> Result<NonNull<mxArray>> {
        self.require_read("get")?;
        validate_name(name)?;
        let mut code = 0;
        // SAFETY: exclusive live handle; output is handled before any further call.
        let raw =
            unsafe { ffi::rustmat_get(self.ptr(), name.as_ptr(), i32::from(info), &mut code) };
        NonNull::new(raw)
            .ok_or_else(|| Error::native(ErrorKind::Read, "get", format!("{name:?}"), -1, code))
    }
    /// Delete an existing variable. Does not silently ignore missing variables.
    pub fn delete(&mut self, name: &CStr) -> Result<()> {
        self.require_write("delete")?;
        validate_name(name)?;
        let mut code = 0;
        // SAFETY: exclusive live file and valid name.
        let status = unsafe { ffi::rustmat_delete(self.ptr(), name.as_ptr(), &mut code) };
        if status != 0 {
            return Err(Error::native(
                ErrorKind::Write,
                "delete",
                format!("{name:?}"),
                status,
                code,
            ));
        }
        Ok(())
    }
    /// Copy variable names and free MATLAB's directory allocation exactly once.
    pub fn variables(&mut self) -> Result<Vec<CString>> {
        self.require_read("variables")?;
        let (mut count, mut code) = (0, 0);
        // SAFETY: exclusive file and valid output pointers.
        let ptr = unsafe { ffi::rustmat_directory(self.ptr(), &mut count, &mut code) };
        struct Directory(*mut *mut std::ffi::c_char);
        impl Drop for Directory {
            fn drop(&mut self) {
                // SAFETY: matGetDir allocates the entire result in one block.
                if !self.0.is_null() {
                    unsafe { ffi::rustmat_free(self.0.cast()) }
                }
            }
        }
        let names = Directory(ptr);
        if count < 0 || (count > 0 && ptr.is_null()) {
            return Err(Error::native(
                ErrorKind::Read,
                "variables",
                "invalid directory",
                count,
                code,
            ));
        }
        (0..count as usize)
            .map(|i| {
                // SAFETY: MATLAB reports count valid pointers; allocation stays alive.
                let name = unsafe { *names.0.add(i) };
                if name.is_null() {
                    return Err(Error::new(
                        ErrorKind::Native,
                        "variables",
                        "null variable name",
                    ));
                }
                Ok(unsafe { CStr::from_ptr(name) }.to_owned())
            })
            .collect()
    }
    /// Query the native MAT error state without interpreting it as an OS errno.
    pub fn last_error(&mut self) -> MatError {
        // SAFETY: the handle remains open and is exclusively borrowed.
        MatError(unsafe { ffi::rustmat_error(self.ptr()) })
    }
    /// Borrow the native stream for diagnostics. Some formats have no FILE*.
    /// This never transfers ownership or allows fclose/seek/write.
    pub fn stream(&mut self) -> Option<FileStream<'_>> {
        // SAFETY: file is live; returned view cannot outlive its mutable borrow.
        NonNull::new(unsafe { ffi::rustmat_stream(self.ptr()) }).map(|raw| FileStream {
            raw,
            _borrow: PhantomData,
            _thread: PhantomData,
        })
    }
    /// Close exactly once, even if finalization fails. No retry is possible.
    pub fn close(mut self) -> Result<()> {
        let raw = self.raw.take().expect("live MAT-file");
        // SAFETY: consume the only handle; matClose invalidates it on all paths.
        let status = unsafe { ffi::rustmat_close(raw.as_ptr()) };
        if status != 0 {
            let mut error = Error::new(ErrorKind::Close, "close", "MAT-file finalization failed");
            error.status = Some(status);
            return Err(error);
        }
        Ok(())
    }
}
impl Drop for MatFile<'_> {
    fn drop(&mut self) {
        if let Some(raw) = self.raw.take() {
            // SAFETY: still uniquely owned. Drop must not report via MATLAB or panic.
            unsafe {
                ffi::rustmat_close(raw.as_ptr());
            }
        }
    }
}

/// Lifetime-bound FILE* diagnostics executed by the same C runtime as the shim.
/// ```compile_fail
/// # fn example(context: &rustmat::Matlab) -> rustmat::Result<()> {
/// let mut file = rustmat::MatFile::open(context, "data.mat", rustmat::OpenMode::Read)?;
/// let stream = file.stream().unwrap();
/// file.close()?;
/// let _ = stream.is_eof();
/// # Ok(()) }
/// ```
pub struct FileStream<'file> {
    raw: NonNull<c_void>,
    _borrow: PhantomData<&'file mut ()>,
    _thread: PhantomData<Rc<()>>,
}
impl FileStream<'_> {
    pub fn is_eof(&self) -> bool {
        // SAFETY: stream is borrowed from a live file; no ownership is transferred.
        unsafe { ffi::rustmat_stream_eof(self.raw.as_ptr()) != 0 }
    }
    pub fn has_error(&self) -> bool {
        // SAFETY: same stream lifetime contract.
        unsafe { ffi::rustmat_stream_error(self.raw.as_ptr()) != 0 }
    }
    pub fn clear_error(&mut self) {
        // SAFETY: exclusive file borrow and matching CRT.
        unsafe { ffi::rustmat_stream_clear(self.raw.as_ptr()) }
    }
    pub fn position(&self) -> Result<u64> {
        // SAFETY: querying a borrowed live stream does not change its position.
        let position = unsafe { ffi::rustmat_stream_position(self.raw.as_ptr()) };
        u64::try_from(position).map_err(|_| {
            Error::new(
                ErrorKind::Native,
                "stream position",
                "native position query failed",
            )
        })
    }
}

fn validate_name(name: &CStr) -> Result<()> {
    if name.to_bytes().is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "variable name",
            "empty name",
        ));
    }
    Ok(())
}
fn path_cstring(path: &Path) -> Result<CString> {
    let text = path
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "path", "path is not valid Unicode"))?;
    if text.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "path", "empty path"));
    }
    CString::new(text).map_err(|_| Error::new(ErrorKind::InvalidInput, "path", "path contains NUL"))
}

/// Export a C-owned MEX entrypoint for a `fn(Lhs, Rhs) -> rustmex::Result<()>`.
/// Invoke once in the final cdylib, instead of rustmex's entrypoint attribute.
/// Returned errors and unwinding Rust panics become MATLAB errors after Rust
/// destructors run. This cannot catch native exceptions or `panic = "abort"`.
/// The handler receives exactly `nlhs` output slots (zero when no output was
/// requested). Error messages are limited to 8191 UTF-8 bytes, with NUL replaced.
/// This macro does not establish the unsafe [`Matlab::attach`] runtime contract.
#[macro_export]
macro_rules! mex_entrypoint {
    ($handler:path) => {
        #[used]
        #[no_mangle]
        pub static rustmat_mex_link: unsafe extern "C" fn() = $crate::__private::rustmat_mex_anchor;
        #[no_mangle]
        /// # Safety
        /// Called only by rustmat's C entrypoint with MATLAB's valid arguments.
        pub unsafe extern "C" fn rustmat_mex_dispatch(
            nlhs: i32,
            plhs: *mut *mut $crate::__private::mxArray,
            nrhs: i32,
            prhs: *const *const $crate::__private::mxArray,
            id: *mut std::ffi::c_char,
            id_len: usize,
            message: *mut std::ffi::c_char,
            message_len: usize,
        ) -> i32 {
            // SAFETY: the C-owned entrypoint supplies all pointers and capacities.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_lossy_path_inputs_and_empty_names() {
        assert!(path_cstring(Path::new("")).is_err());
        assert!(path_cstring(Path::new("a\0b")).is_err());
        assert_eq!(
            path_cstring(Path::new("日本語.mat")).unwrap().to_bytes(),
            "日本語.mat".as_bytes()
        );
        assert!(validate_name(c"").is_err());
        assert!(validate_name(c"valid").is_ok());
    }
    #[test]
    fn error_messages_escape_nul_for_mex() {
        let e = Error::new(ErrorKind::Read, "get", "bad\0name");
        assert!(!e.to_string().contains('\0'));
    }

    #[test]
    fn close_failure_and_drop_each_finalize_exactly_once() {
        // No MATLAB call is made in this test: the private close seam consumes
        // these sentinel handles, and the synthetic context never enters FFI.
        let context = Matlab {
            _thread: PhantomData,
        };
        for status in [0, -1] {
            ffi::CLOSE_PROBE.with(|probe| probe.set(Some((status, 0))));
            let file = MatFile {
                raw: Some(NonNull::dangling()),
                mode: OpenMode::Read,
                context: &context,
            };
            let result = file.close();
            assert_eq!(result.is_ok(), status == 0);
            if let Err(error) = result {
                assert_eq!(error.kind, ErrorKind::Close);
                assert_eq!(error.status, Some(-1));
                assert!(error.mat_error.is_none()); // never query an invalid handle
            }
            ffi::CLOSE_PROBE.with(|probe| assert_eq!(probe.get(), Some((status, 1))));
            ffi::CLOSE_PROBE.with(|probe| probe.set(Some((status, 0))));
            drop(MatFile {
                raw: Some(NonNull::dangling()),
                mode: OpenMode::Read,
                context: &context,
            });
            ffi::CLOSE_PROBE.with(|probe| assert_eq!(probe.get(), Some((status, 1))));
        }
        ffi::CLOSE_PROBE.with(|probe| probe.set(None));
    }
}
