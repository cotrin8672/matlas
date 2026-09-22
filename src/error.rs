use std::fmt;

/// An unmodified error code from `matGetErrno` (not an OS errno).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MatError(pub i32);

/// Failures distinguished without guessing undocumented native error numbers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    InvalidInput,
    Type,
    Bounds,
    Allocation,
    Callback,
    Workspace,
    Busy,
    InvalidMode,
    Open,
    Read,
    Write,
    Close,
    UnexpectedEnd,
    Native,
}

/// An error that preserves the native operation and MATLAB error status.
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub operation: &'static str,
    pub detail: String,
    pub status: Option<i32>,
    pub mat_error: Option<MatError>,
}

impl Error {
    pub fn new(kind: ErrorKind, operation: &'static str, detail: impl Into<String>) -> Self {
        Self {
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

    pub(crate) fn native_status(operation: &'static str, status: i32) -> Self {
        let mut error = Self::new(ErrorKind::Native, operation, "native operation failed");
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

    pub fn id(&self) -> &'static str {
        match self.kind {
            ErrorKind::InvalidInput => "matrust:input:invalid",
            ErrorKind::Type => "matrust:array:type",
            ErrorKind::Bounds => "matrust:array:bounds",
            ErrorKind::Allocation => "matrust:allocation",
            ErrorKind::Callback => "matrust:callback",
            ErrorKind::Workspace => "matrust:workspace",
            ErrorKind::Busy => "matrust:runtime:busy",
            ErrorKind::InvalidMode => "matrust:file:mode",
            ErrorKind::Open => "matrust:file:open",
            ErrorKind::Read | ErrorKind::UnexpectedEnd => "matrust:file:read",
            ErrorKind::Write => "matrust:file:write",
            ErrorKind::Close => "matrust:file:close",
            ErrorKind::Native => "matrust:native",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.operation, self.detail)?;
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
pub type Result<T> = std::result::Result<T, Error>;
