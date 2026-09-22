use std::fmt;

/// An unmodified error code from `matGetErrno` (not an OS errno).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MatError(pub i32);

/// Failures distinguished without guessing undocumented native error numbers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    InvalidInput,
    InvalidMode,
    Open,
    Read,
    Write,
    Close,
    UnexpectedEnd,
    Native,
}

/// A Rust error that also converts to `rustmex::Error` through `?`.
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub operation: &'static str,
    pub detail: String,
    pub status: Option<i32>,
    pub mat_error: Option<MatError>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, operation: &'static str, detail: impl Into<String>) -> Self {
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
impl rustmex::MexMessage for Error {
    fn id(&self) -> &str {
        match self.kind {
            ErrorKind::InvalidInput => "rustmat:input:invalid",
            ErrorKind::InvalidMode => "rustmat:file:mode",
            ErrorKind::Open => "rustmat:file:open",
            ErrorKind::Read | ErrorKind::UnexpectedEnd => "rustmat:file:read",
            ErrorKind::Write => "rustmat:file:write",
            ErrorKind::Close => "rustmat:file:close",
            ErrorKind::Native => "rustmat:file:native",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
