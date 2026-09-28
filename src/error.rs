use std::{borrow::Cow, fmt};

/// A validated MATLAB exception identifier of at most 255 ASCII bytes.
///
/// Each colon-separated component starts with a letter and contains only
/// letters, digits, or underscores. At least two components are required.
/// Use [`crate::error_id!`] for literals or [`TryFrom<String>`] for dynamic IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorId(Cow<'static, str>);

impl ErrorId {
    /// Return the validated identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[doc(hidden)]
    pub const fn __from_static(identifier: &'static str) -> Self {
        assert!(
            valid_identifier(identifier),
            "expected colon-separated ASCII identifiers of at most 255 bytes"
        );
        Self(Cow::Borrowed(identifier))
    }
}

impl TryFrom<String> for ErrorId {
    type Error = Error;

    fn try_from(identifier: String) -> Result<Self> {
        if !valid_identifier(&identifier) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "error identifier",
                "expected colon-separated ASCII identifiers of at most 255 bytes",
            ));
        }
        Ok(Self(Cow::Owned(identifier)))
    }
}

const fn valid_identifier(identifier: &str) -> bool {
    let bytes = identifier.as_bytes();
    if bytes.len() > 255 {
        return false;
    }
    let mut index = 0;
    let mut component_start = true;
    let mut has_separator = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if component_start {
            if !byte.is_ascii_alphabetic() {
                return false;
            }
            component_start = false;
        } else if byte == b':' {
            component_start = true;
            has_separator = true;
        } else if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return false;
        }
        index += 1;
    }
    has_separator && !component_start
}

/// Create an [`ErrorId`] from a string literal, validated at compile time.
///
/// ```
/// use matlas::{error_id, ErrorId};
/// const WRITE_FAILED: ErrorId = error_id!("store:FileWriteFailed");
/// assert_eq!(WRITE_FAILED.as_str(), "store:FileWriteFailed");
/// ```
///
/// Invalid literals fail to compile even outside a constant declaration:
///
/// ```compile_fail
/// let id = matlas::error_id!("store:bad-name");
/// ```
#[macro_export]
macro_rules! error_id {
    ($identifier:literal $(,)?) => {{
        const ID: $crate::ErrorId = $crate::ErrorId::__from_static($identifier);
        ID
    }};
}

/// An unmodified error code from `matGetErrno` (not an OS errno).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MatError(pub i32);

/// Failures distinguished without guessing undocumented native error numbers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An argument violates the Rust API contract.
    InvalidInput,
    /// An array has the wrong MATLAB class or storage layout.
    Type,
    /// An index, dimension, or allocation size is out of bounds.
    Bounds,
    /// MATLAB could not allocate memory.
    Allocation,
    /// A trapped MATLAB callback failed.
    Callback,
    /// A workspace lookup or update failed.
    Workspace,
    /// The runtime cannot perform the operation while a borrow is active.
    Busy,
    /// A MAT-file mode does not support the requested operation.
    InvalidMode,
    /// Opening or creating a MAT-file failed.
    Open,
    /// Reading a MAT-file failed.
    Read,
    /// Writing a MAT-file failed.
    Write,
    /// Closing a MAT-file failed.
    Close,
    /// Sequential MAT-file reading ended unexpectedly.
    UnexpectedEnd,
    /// A native operation failed without a more specific category.
    Native,
}

/// Identifier and message copied from a trapped MATLAB callback exception.
/// This does not include the exception's stack or nested causes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatlabErrorInfo {
    identifier: Option<String>,
    message: Option<String>,
}
impl MatlabErrorInfo {
    pub(crate) fn new(identifier: Option<String>, message: Option<String>) -> Self {
        Self {
            identifier,
            message,
        }
    }
    /// Return the original MATLAB identifier, if it could be read.
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }
    /// Return the original MATLAB message, if it could be read.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// An error that preserves the native operation and MATLAB error status.
#[derive(Debug)]
pub struct Error {
    /// Optional MATLAB exception identifier overriding the category default.
    identifier: Option<ErrorId>,
    /// Additional context, in the order it was added (inner to outer).
    contexts: Vec<String>,
    /// Original identifier and message from a trapped MATLAB callback.
    matlab_error: Option<Box<MatlabErrorInfo>>,
    /// Stable high-level error category.
    kind: ErrorKind,
    /// Operation that detected the failure.
    operation: &'static str,
    /// Human-readable failure detail.
    detail: String,
    /// Optional status returned by the native function.
    status: Option<i32>,
    /// Optional unmodified `matGetErrno` value.
    mat_error: Option<MatError>,
}

impl Error {
    /// Construct an error without a native status code.
    pub fn new(kind: ErrorKind, operation: &'static str, detail: impl Into<String>) -> Self {
        Self {
            identifier: None,
            contexts: Vec::new(),
            matlab_error: None,
            kind,
            operation,
            detail: detail.into(),
            status: None,
            mat_error: None,
        }
    }
    pub(crate) fn native(
        kind: ErrorKind,
        operation: &'static str,
        detail: impl Into<String>,
        status: i32,
        code: i32,
    ) -> Self {
        Self {
            identifier: None,
            contexts: Vec::new(),
            matlab_error: None,
            kind,
            operation,
            detail: detail.into(),
            status: Some(status),
            mat_error: Some(MatError(code)),
        }
    }

    pub(crate) fn allocation(operation: &'static str) -> Self {
        Self::new(
            ErrorKind::Allocation,
            operation,
            "MATLAB returned a null allocation",
        )
    }

    pub(crate) fn from_native_status(operation: &'static str, status: i32) -> Self {
        Self::native_status_kind(ErrorKind::Native, operation, status)
    }

    pub(crate) fn native_status_kind(
        kind: ErrorKind,
        operation: &'static str,
        status: i32,
    ) -> Self {
        let mut error = Self::new(kind, operation, "native operation failed");
        error.status = Some(status);
        error
    }

    pub(crate) fn bounds(operation: &'static str, index: usize, length: usize) -> Self {
        Self::new(
            ErrorKind::Bounds,
            operation,
            format!("index {index} is outside length {length}"),
        )
    }

    pub(crate) fn type_mismatch(
        operation: &'static str,
        expected: &str,
        actual: crate::ArrayRef<'_>,
    ) -> Self {
        Self::new(
            ErrorKind::Type,
            operation,
            format!("expected {expected}, got {:?}", actual.class_name()),
        )
    }

    pub(crate) fn with_matlab_error(mut self, info: MatlabErrorInfo) -> Self {
        self.matlab_error = Some(Box::new(info));
        self
    }

    /// Return the high-level error category.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    /// Return the operation that detected the failure.
    pub fn operation(&self) -> &'static str {
        self.operation
    }
    /// Return the human-readable failure detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
    /// Return the native operation status, if available.
    pub fn native_status(&self) -> Option<i32> {
        self.status
    }
    /// Return the unmodified `matGetErrno` value, if available.
    pub fn mat_error(&self) -> Option<MatError> {
        self.mat_error
    }
    /// Return the original MATLAB callback identifier and message, if trapped.
    pub fn matlab_error(&self) -> Option<&MatlabErrorInfo> {
        self.matlab_error.as_deref()
    }

    /// Set the MATLAB exception identifier without changing the original cause.
    ///
    /// The last identifier supplied wins. All other fields and context are
    /// preserved. Use [`crate::error_id!`] or [`ErrorId::try_from`] to validate
    /// the identifier before attaching it.
    #[must_use]
    pub fn with_id(mut self, identifier: ErrorId) -> Self {
        self.identifier = Some(identifier);
        self
    }

    /// Add context while preserving the original operation, detail, and status.
    ///
    /// Display prints the most recently added (outermost) context first,
    /// followed by earlier context and the original cause.
    #[must_use]
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.contexts.push(context.into());
        self
    }

    /// Return the custom identifier, or the default for this category.
    pub fn id(&self) -> &str {
        self.identifier
            .as_ref()
            .map(ErrorId::as_str)
            .unwrap_or(match self.kind {
                ErrorKind::InvalidInput => "matlas:input:invalid",
                ErrorKind::Type => "matlas:array:type",
                ErrorKind::Bounds => "matlas:array:bounds",
                ErrorKind::Allocation => "matlas:allocation",
                ErrorKind::Callback => "matlas:callback",
                ErrorKind::Workspace => "matlas:workspace",
                ErrorKind::Busy => "matlas:runtime:busy",
                ErrorKind::InvalidMode => "matlas:file:mode",
                ErrorKind::Open => "matlas:file:open",
                ErrorKind::Read | ErrorKind::UnexpectedEnd => "matlas:file:read",
                ErrorKind::Write => "matlas:file:write",
                ErrorKind::Close => "matlas:file:close",
                ErrorKind::Native => "matlas:native",
            })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for context in self.contexts.iter().rev() {
            write!(f, "{context}\n  ")?;
        }
        write!(f, "{}: {}", self.operation, self.detail)?;
        if let Some(status) = self.status {
            write!(f, " (status {status})")?;
        }
        if let Some(MatError(code)) = self.mat_error {
            write!(f, " (MAT error {code})")?;
        }
        Ok(())
    }
}
impl std::error::Error for Error {}
/// Result type used by the safe API.
pub type Result<T> = std::result::Result<T, Error>;

/// Attach an exception identifier or context to a [`Result`] without losing its cause.
pub trait ResultExt<T> {
    /// Set the exception identifier on any error, leaving a successful value unchanged.
    ///
    /// # Errors
    ///
    /// Returns the original error with only its identifier replaced.
    fn with_id(self, identifier: ErrorId) -> Result<T>;

    /// Add context on failure, leaving a successful value unchanged.
    ///
    /// Use [`Self::with_context`] to defer formatting until an error occurs.
    ///
    /// # Errors
    ///
    /// Returns the original error with the supplied context added.
    fn context(self, context: impl Into<String>) -> Result<T>;

    /// Generate and add context only on failure. The closure runs at most once.
    ///
    /// # Errors
    ///
    /// Returns the original error with the generated context added.
    fn with_context<F, S>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T> ResultExt<T> for Result<T> {
    fn with_id(self, identifier: ErrorId) -> Result<T> {
        self.map_err(|error| error.with_id(identifier))
    }

    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|error| error.context(context))
    }

    fn with_context<F, S>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(|error| error.context(context()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_identifier_is_validated() {
        const ID: ErrorId = crate::error_id!("store:MissingRecordFile");
        let error = Error::new(ErrorKind::Open, "open", "missing").with_id(ID);
        assert_eq!(error.id(), "store:MissingRecordFile");
        assert_eq!(error.kind(), ErrorKind::Open);
        assert_eq!(ErrorId::try_from(ID.as_str().to_owned()).unwrap(), ID);
        assert_eq!(
            ErrorId::try_from("a:B_2:c3".to_owned()).unwrap(),
            crate::error_id!("a:B_2:c3"),
        );
        for id in [
            "",
            "one",
            ":good",
            "bad:",
            "good::bad",
            "0bad:Good",
            "good:0bad",
            "_bad:Good",
            "good:_bad",
            "good:bad-name",
            "good:日本語",
            "good:bad\0",
            "good:bad\n",
        ] {
            let error = ErrorId::try_from(id.to_owned()).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput, "{id:?}");
        }
        let longest = format!("good:{}", "x".repeat(250));
        assert_eq!(longest.len(), 255);
        assert!(ErrorId::try_from(longest.clone()).is_ok());
        assert!(ErrorId::try_from(format!("{longest}x")).is_err());
    }

    #[test]
    fn context_and_identifiers_preserve_the_native_cause() {
        let cause = Error::native(ErrorKind::Write, "put MAT variable", "native detail", 7, 42);
        let original_message = cause.to_string();
        let calls = std::cell::Cell::new(0);
        let inner = String::from("inner context");
        let result: Result<()> = Err(cause);
        let error = result
            .with_id(crate::error_id!("store:First"))
            .with_context(|| {
                calls.set(calls.get() + 1);
                inner
            })
            .context("outer context")
            .with_id(crate::error_id!("store:FileWriteFailed"))
            .unwrap_err();
        assert_eq!(calls.get(), 1);
        assert_eq!(error.id(), "store:FileWriteFailed");
        assert_eq!(error.kind(), ErrorKind::Write);
        assert_eq!(error.operation(), "put MAT variable");
        assert_eq!(error.detail(), "native detail");
        assert_eq!(error.native_status(), Some(7));
        assert_eq!(error.mat_error(), Some(MatError(42)));
        assert_eq!(
            error.to_string(),
            format!("outer context\n  inner context\n  {original_message}"),
        );
    }

    #[test]
    fn successful_results_skip_context_generation() {
        let result: Result<_> = Ok(42);
        let value = result
            .with_id(crate::error_id!("store:Unused"))
            .context("unused context")
            .with_context(|| -> String { panic!("context must not run on success") })
            .unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn callback_info_survives_relabeling_and_context() {
        let info = MatlabErrorInfo::new(
            Some("matlab:Original".to_owned()),
            Some("original message".to_owned()),
        );
        let error = Error::new(
            ErrorKind::Callback,
            "call MATLAB",
            "matlab:Original: original message",
        )
        .with_matlab_error(info)
        .with_id(crate::error_id!("store:ReadFailed"))
        .context("load data");
        assert_eq!(error.id(), "store:ReadFailed");
        assert_eq!(
            error.matlab_error().and_then(MatlabErrorInfo::identifier),
            Some("matlab:Original")
        );
        assert_eq!(
            error.matlab_error().and_then(MatlabErrorInfo::message),
            Some("original message")
        );
        assert_eq!(
            error.to_string(),
            "load data\n  call MATLAB: matlab:Original: original message"
        );
    }
}
