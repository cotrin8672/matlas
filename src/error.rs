use std::fmt;

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

/// An error that preserves the native operation and MATLAB error status.
#[derive(Debug)]
pub struct Error {
    /// Optional MATLAB exception identifier overriding the category default.
    identifier: Option<String>,
    /// Stable high-level error category.
    pub kind: ErrorKind,
    /// Operation that detected the failure.
    pub operation: &'static str,
    /// Human-readable failure detail.
    pub detail: String,
    /// Optional status returned by the native function.
    pub status: Option<i32>,
    /// Optional unmodified `matGetErrno` value.
    pub mat_error: Option<MatError>,
}

impl Error {
    /// Construct an error without a native status code.
    pub fn new(kind: ErrorKind, operation: &'static str, detail: impl Into<String>) -> Self {
        Self {
            identifier: None,
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

    /// Set a MATLAB exception identifier such as `store:MissingRecordFile`.
    ///
    /// # Errors
    ///
    /// Returns an input error if the identifier is invalid or exceeds the
    /// native MEX entrypoint's 255-byte identifier buffer.
    pub fn with_id(mut self, identifier: impl Into<String>) -> crate::Result<Self> {
        let identifier = identifier.into();
        if identifier.len() > 255
            || identifier.split(':').count() < 2
            || !identifier.split(':').all(|field| {
                let mut bytes = field.bytes();
                bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
                    && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        {
            return Err(Self::new(
                ErrorKind::InvalidInput,
                "error identifier",
                "expected colon-separated ASCII identifiers of at most 255 bytes",
            ));
        }
        self.identifier = Some(identifier);
        Ok(self)
    }

    /// Return the custom identifier, or the default for this category.
    pub fn id(&self) -> &str {
        self.identifier.as_deref().unwrap_or(match self.kind {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_identifier_is_validated() {
        let error = Error::new(ErrorKind::Open, "open", "missing")
            .with_id("store:MissingRecordFile")
            .unwrap();
        assert_eq!(error.id(), "store:MissingRecordFile");
        assert_eq!(error.kind, ErrorKind::Open);
        for id in ["one", "bad:", "0bad:Good", "good:bad-name", "good:日本語"] {
            assert!(Error::new(ErrorKind::Open, "open", "missing")
                .with_id(id)
                .is_err());
        }
        assert!(Error::new(ErrorKind::Open, "open", "missing")
            .with_id(format!("good:{}", "x".repeat(251)))
            .is_err());
    }
}
