//! Safe, lifetime-aware Rust bindings for MATLAB's Matrix, MEX and MAT-file APIs.
//!
//! The safe API has three ownership categories: [`OwnedArray`] is destroyed by
//! Rust, [`ArrayRef`] is a read-only borrow, and [`WorkspaceValue`] is a
//! MATLAB workspace pointer valid until the next MATLAB callback.

#![deny(missing_docs)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::missing_panics_doc)]

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
    SparseNumeric, UninitNumeric,
};
pub use error::{Error, ErrorKind, MatError, Result};

/// Function-by-function coverage of the R2025a API-800 C headers.
#[doc = include_str!("../docs/API_COVERAGE.md")]
pub mod api_coverage {}

/// Error propagation guarantees and unavoidable native termination cases.
#[doc = include_str!("../docs/ERROR_HANDLING.md")]
pub mod error_handling {}

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
    ///
    /// # Errors
    ///
    /// Returns an allocation error if MATLAB cannot resize the buffer. The
    /// original allocation remains owned by this value on failure.
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

/// One counted lock on the current MEX module.
///
/// The guard may be stored between invocations. Dropping it calls `mexUnlock`
/// exactly once, so safe code cannot accidentally unbalance the native lock
/// count. Use [`ModuleLock::release`] when the release point should be explicit.
#[must_use = "dropping the module lock immediately unlocks the MEX module"]
pub struct ModuleLock {
    active: bool,
    _thread: PhantomData<Rc<()>>,
}

impl ModuleLock {
    /// Release this counted lock immediately.
    pub fn release(mut self) {
        unsafe { ffi::matrust_unlock() };
        self.active = false;
    }
}

impl Drop for ModuleLock {
    fn drop(&mut self) {
        if self.active {
            unsafe { ffi::matrust_unlock() };
        }
    }
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

    /// Deep-copy any borrowed array into a new Rust-owned MATLAB allocation.
    ///
    /// # Errors
    ///
    /// Returns an allocation error if MATLAB cannot return a copy.
    pub fn duplicate(&self, value: ArrayRef<'_>) -> Result<OwnedArray<'mex>> {
        let raw = unsafe { ffi::matrust_array_duplicate(value.as_ptr()) };
        let raw = NonNull::new(raw).ok_or_else(|| Error::allocation("duplicate array"))?;
        Ok(unsafe { OwnedArray::from_raw(raw) })
    }

    /// Read a public property from any borrowed object. MATLAB returns an
    /// owned copy branded to the current invocation.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-bounds index, a non-object, an unknown
    /// or nonpublic property, or a native allocation failure.
    pub fn property(
        &self,
        object: ArrayRef<'_>,
        index: usize,
        name: &CStr,
    ) -> Result<OwnedArray<'mex>> {
        if index >= object.numel() {
            return Err(Error::bounds("get property", index, object.numel()));
        }
        let raw = unsafe { ffi::matrust_property_get(object.as_ptr(), index, name.as_ptr()) };
        NonNull::new(raw)
            .map(|raw| unsafe { OwnedArray::from_raw(raw) })
            .ok_or_else(|| Error::new(ErrorKind::Native, "get property", format!("{name:?}")))
    }

    /// Return the name by which MATLAB invoked the current MEX function.
    pub fn function_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(ffi::matrust_function_name()) }
    }

    /// Print literal UTF-8 text to MATLAB's command window.
    ///
    /// # Errors
    ///
    /// Returns an error if the text contains NUL or the native print routine
    /// reports failure.
    pub fn printf(&mut self, text: &str) -> Result<()> {
        let text = CString::new(text)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "printf", "text contains NUL"))?;
        let status = unsafe { ffi::matrust_printf(text.as_ptr()) };
        (status >= 0)
            .then_some(())
            .ok_or_else(|| Error::native_status("printf", status))
    }

    /// Issue a MATLAB warning with an identifier and literal message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message contains NUL.
    pub fn warning(&mut self, id: &CStr, text: &str) -> Result<()> {
        let text = CString::new(text)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "warning", "text contains NUL"))?;
        unsafe { ffi::matrust_warning(id.as_ptr(), text.as_ptr()) };
        Ok(())
    }

    /// Allocate zeroed MATLAB-managed memory.
    ///
    /// # Errors
    ///
    /// Returns an allocation error if MATLAB returns a null pointer.
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

    /// Prevent MATLAB from clearing the current MEX module until the returned
    /// guard is dropped or explicitly released.
    pub fn lock(&mut self) -> ModuleLock {
        unsafe { ffi::matrust_lock() };
        ModuleLock {
            active: true,
            _thread: PhantomData,
        }
    }
    /// Test whether the current MEX module is locked.
    pub fn is_locked(&self) -> bool {
        unsafe { ffi::matrust_is_locked() != 0 }
    }

    /// Copy a variable from a MATLAB workspace into a Rust-owned array.
    ///
    /// # Errors
    ///
    /// Returns an error if the named variable does not exist or MATLAB cannot
    /// copy it.
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

    /// Open a scope for borrowing multiple variables from one MATLAB workspace.
    /// The scope must be consumed to pass those pointers into a callback.
    pub fn workspace_scope(&mut self, space: Workspace) -> WorkspaceScope<'_, 'mex> {
        WorkspaceScope {
            space,
            _context: self,
        }
    }

    /// Copy a borrowed array into a MATLAB workspace.
    ///
    /// # Errors
    ///
    /// Returns an error while a workspace pointer is borrowed or if MATLAB
    /// rejects the copy.
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
    ///
    /// # Errors
    ///
    /// Returns an error for an active workspace borrow, more than 50 inputs or
    /// outputs, a trapped MATLAB exception, or a null output.
    pub fn call(
        &mut self,
        name: &CStr,
        inputs: &[ArrayRef<'_>],
        output_count: usize,
    ) -> Result<Vec<OwnedArray<'mex>>> {
        runtime::require_unborrowed("call MATLAB")?;
        call_raw(
            name,
            inputs.iter().map(|v| v.as_ptr().cast_mut()).collect(),
            output_count,
        )
    }

    /// Call MATLAB and return exactly `N` owned outputs.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Matlab::call`].
    pub fn call_array<const N: usize>(
        &mut self,
        name: &CStr,
        inputs: &[ArrayRef<'_>],
    ) -> Result<[OwnedArray<'mex>; N]> {
        self.call(name, inputs, N)?
            .try_into()
            .map_err(|_| Error::new(ErrorKind::Native, "call MATLAB", "unexpected output count"))
    }

    /// Evaluate MATLAB source through the trapping API.
    ///
    /// # Errors
    ///
    /// Returns an error for an active workspace borrow, an embedded NUL, or a
    /// trapped MATLAB exception.
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
        if let Some(trap) = NonNull::new(trap) {
            Err(callback_error(
                trap,
                "evaluate MATLAB",
                "command failed without exception details".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

fn call_raw<'mex>(
    name: &CStr,
    mut raw_inputs: Vec<*mut ffi::RawArray>,
    output_count: usize,
) -> Result<Vec<OwnedArray<'mex>>> {
    if output_count > 50 || raw_inputs.len() > 50 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "call MATLAB",
            "MATLAB callbacks support at most 50 inputs and 50 outputs",
        ));
    }
    let count = i32::try_from(output_count)
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "call MATLAB", "too many outputs"))?;
    let mut outputs = vec![std::ptr::null_mut(); output_count];
    let input_count = i32::try_from(raw_inputs.len())
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "call MATLAB", "too many inputs"))?;
    let outputs_ptr = if outputs.is_empty() {
        std::ptr::null_mut()
    } else {
        outputs.as_mut_ptr()
    };
    let inputs_ptr = if raw_inputs.is_empty() {
        std::ptr::null_mut()
    } else {
        raw_inputs.as_mut_ptr()
    };
    let trap = unsafe {
        ffi::matrust_call_with_trap(count, outputs_ptr, input_count, inputs_ptr, name.as_ptr())
    };
    if let Some(trap) = NonNull::new(trap) {
        for raw in outputs {
            if !raw.is_null() {
                unsafe { ffi::matrust_array_destroy(raw) }
            }
        }
        return Err(callback_error(
            trap,
            "call MATLAB",
            format!("{name:?} failed without exception details"),
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

fn callback_error(raw: NonNull<ffi::RawArray>, operation: &'static str, fallback: String) -> Error {
    let trap = unsafe { OwnedArray::from_raw(raw) };
    let identifier = trap
        .property(0, c"identifier")
        .and_then(|value| value.as_ref().to_utf8())
        .ok();
    let message = trap
        .property(0, c"message")
        .and_then(|value| value.as_ref().to_utf8())
        .ok();
    let detail = match (identifier, message) {
        (Some(identifier), Some(message)) if !identifier.is_empty() => {
            format!("{identifier}: {message}")
        }
        (_, Some(message)) => message,
        _ => fallback,
    };
    Error::new(ErrorKind::Callback, operation, detail)
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

/// An exclusive workspace access period. [`WorkspaceScope::call`] consumes it
/// and the supplied workspace values before invoking MATLAB.
pub struct WorkspaceScope<'a, 'mex> {
    space: Workspace,
    _context: &'a mut Matlab<'mex>,
}

impl<'a, 'mex> WorkspaceScope<'a, 'mex> {
    /// Borrow a workspace variable without copying it. More than one value
    /// may be fetched before the callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the named variable does not exist.
    pub fn get(&self, name: &CStr) -> Result<WorkspaceValue<'a>> {
        let raw = unsafe { ffi::matrust_workspace_borrow(self.space.as_ptr(), name.as_ptr()) };
        let raw = NonNull::new(raw.cast_mut()).ok_or_else(|| {
            Error::new(
                ErrorKind::Workspace,
                "borrow workspace variable",
                format!("{name:?}"),
            )
        })?;
        Ok(WorkspaceValue {
            raw,
            _guard: runtime::begin_external_borrow(),
            _context: PhantomData,
        })
    }

    /// Call MATLAB with borrowed workspace values and ordinary array views in
    /// argument order. Each workspace value must be moved into `inputs`. An
    /// unused live value causes an error before MATLAB is called.
    ///
    /// # Errors
    ///
    /// Returns an error for an unused live workspace value, more than 50
    /// inputs or outputs, a trapped MATLAB exception, or a null output.
    pub fn call(
        self,
        name: &CStr,
        inputs: impl IntoIterator<Item = CallInput<'a>>,
        output_count: usize,
    ) -> Result<Vec<OwnedArray<'mex>>> {
        let inputs: Vec<_> = inputs.into_iter().collect();
        let raw = inputs.iter().map(CallInput::as_ptr).collect();
        drop(inputs);
        runtime::require_unborrowed("call MATLAB")?;
        call_raw(name, raw, output_count)
    }

    /// Call MATLAB with borrowed workspace inputs and return exactly `N` outputs.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`WorkspaceScope::call`].
    pub fn call_array<const N: usize>(
        self,
        name: &CStr,
        inputs: impl IntoIterator<Item = CallInput<'a>>,
    ) -> Result<[OwnedArray<'mex>; N]> {
        self.call(name, inputs, N)?
            .try_into()
            .map_err(|_| Error::new(ErrorKind::Native, "call MATLAB", "unexpected output count"))
    }
}

/// A pointer returned by `mexGetVariablePtr`. It can be read until it is
/// consumed by [`WorkspaceScope::call`] or dropped.
///
/// ```compile_fail
/// # use matlas::{Matlab, Workspace};
/// fn invalid(cx: &mut Matlab<'_>) {
///     let ws = cx.workspace_scope(Workspace::Caller);
///     let value = ws.get(c"x").unwrap();
///     let _ = ws.call(c"disp", [value.into()], 0);
///     let _ = value.as_ref().numel();
/// }
/// ```
///
/// ```compile_fail
/// # use matlas::{Matlab, Workspace};
/// fn invalid(cx: &mut Matlab<'_>) {
///     let ws = cx.workspace_scope(Workspace::Caller);
///     let value = ws.get(c"x").unwrap();
///     drop(ws);
///     let _ = cx.eval("clear x");
///     let _ = value.as_ref().numel();
/// }
/// ```
pub struct WorkspaceValue<'a> {
    raw: NonNull<ffi::RawArray>,
    _guard: runtime::ExternalBorrowGuard,
    _context: PhantomData<&'a Matlab<'a>>,
}
impl WorkspaceValue<'_> {
    /// Borrow the referenced MATLAB-owned array.
    pub fn as_ref(&self) -> ArrayRef<'_> {
        unsafe { ArrayRef::from_raw(self.raw) }
    }
}

/// One MATLAB callback input, either an ordinary view or a consumed workspace
/// pointer. Use `into()` to assemble a mixed argument list.
pub enum CallInput<'a> {
    /// A borrowed array from a MEX input or an owned array.
    Borrowed(ArrayRef<'a>),
    /// A workspace pointer borrowed without copying.
    Workspace(WorkspaceValue<'a>),
}

impl<'a> From<ArrayRef<'a>> for CallInput<'a> {
    fn from(value: ArrayRef<'a>) -> Self {
        Self::Borrowed(value)
    }
}

impl<'a> From<WorkspaceValue<'a>> for CallInput<'a> {
    fn from(value: WorkspaceValue<'a>) -> Self {
        Self::Workspace(value)
    }
}

impl CallInput<'_> {
    fn as_ptr(&self) -> *mut ffi::RawArray {
        match self {
            Self::Borrowed(value) => value.as_ptr().cast_mut(),
            Self::Workspace(value) => value.raw.as_ptr(),
        }
    }
}

/// Read-only MEX input arguments owned by MATLAB.
///
/// A fixed `N` is checked before the handler runs. The default const value
/// reserves `usize::MAX` for handlers with a variable input count.
pub struct Inputs<'mex, const N: usize = { usize::MAX }> {
    values: Vec<ArrayRef<'mex>>,
}
impl<'mex, const N: usize> Inputs<'mex, N> {
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

    /// Require exactly `N` input arguments and return them in order.
    ///
    /// # Errors
    ///
    /// Returns an input error if the argument count differs from `N`.
    pub fn require<const M: usize>(&self) -> Result<[ArrayRef<'mex>; M]> {
        if self.len() != M {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "MEX inputs",
                format!("expected {M} inputs, got {}", self.len()),
            ));
        }
        Ok(std::array::from_fn(|index| self.values[index]))
    }

    /// Consume fixed-arity inputs as an array for destructuring.
    ///
    /// The entrypoint checks the actual MATLAB argument count before calling
    /// a handler with fixed arity.
    pub fn into_array(self) -> [ArrayRef<'mex>; N] {
        std::array::from_fn(|index| self.values[index])
    }
}

/// MEX output slots that accept ownership of Rust-created arrays.
///
/// A fixed `N` is checked before the handler runs. The default const value
/// reserves `usize::MAX` for handlers with a variable output count.
pub struct Outputs<'mex, const N: usize = { usize::MAX }> {
    values: Vec<Option<OwnedArray<'mex>>>,
}
impl<'mex, const N: usize> Outputs<'mex, N> {
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
    /// Require exactly `count` requested output slots.
    ///
    /// # Errors
    ///
    /// Returns an input error if MATLAB requested a different count.
    pub fn require_len(&self, count: usize) -> Result<()> {
        if self.len() != count {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "MEX outputs",
                format!("expected {count} outputs, got {}", self.len()),
            ));
        }
        Ok(())
    }
    /// Transfer an owned array into an output slot.
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds or the slot is already set.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds.
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
    ///
    /// # Errors
    ///
    /// Returns an error for a non-Unicode/NUL-containing path or if MATLAB
    /// cannot open the file in the requested mode.
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
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid path or native open failure.
    pub fn create(context: &'ctx Matlab<'mex>, path: impl AsRef<Path>) -> Result<Self> {
        Self::open(context, path, OpenMode::Write(MatVersion::V7))
    }
    /// Create a MAT-file in a selected format.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid path or native open failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong file mode, an empty name, or a native
    /// write failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong file mode, an empty name, or a native
    /// write failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong file mode, an empty name, a missing
    /// variable, or a native read failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong file mode, an empty name, a missing
    /// variable, or a native read failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong file mode, an empty name, or a native
    /// delete failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong file mode or malformed/native directory
    /// output.
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
    ///
    /// # Errors
    ///
    /// Returns an error if `matClose` reports failure.
    pub fn close(mut self) -> Result<()> {
        let raw = self.raw.take().ok_or_else(|| {
            Error::new(
                ErrorKind::Close,
                "close MAT-file",
                "file handle was already closed",
            )
        })?;
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
    ///
    /// # Errors
    ///
    /// Returns an error if the native stream position is negative.
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
/// # use matlas::ArrayInfo;
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
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, listed, or closed while
    /// preparing the sequential reader.
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
    ///
    /// # Errors
    ///
    /// Returns an error if `matClose` reports failure.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, listed, or closed while
    /// preparing the sequential reader.
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
    ///
    /// # Errors
    ///
    /// Returns an error if `matClose` reports failure.
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
///
/// # Safety
///
/// Callers must uphold every contract from the corresponding MATLAB C API,
/// including pointer validity, class/layout agreement, allocation-family
/// matching, unique ownership, thread affinity, and non-local-exit behavior.
/// Raw calls must not destroy, transfer, mutate, or replace the exit callback
/// for values or runtime state still managed by the safe API. Violating those
/// rules can invalidate safe Rust references or cause double frees.
#[allow(missing_docs)]
pub mod raw {
    pub use crate::ffi::*;
}
