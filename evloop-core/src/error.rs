use core::fmt;
use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum EventLoopError {
    Recreation,
    ExitFailure(i32),
    Os(OsError),
    NotSupported(NotSupportedError),
}

#[derive(Debug)]
pub struct OsError {
    line: u32,
    file: &'static str,
    error: Box<dyn Error + Send + Sync + 'static>,
}

impl OsError {
    pub fn new(
        line: u32,
        file: &'static str,
        error: Box<dyn Error + Send + Sync + 'static>,
    ) -> Self {
        Self { line, file, error }
    }
}

impl fmt::Display for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&format!(
            "os error at {}:{}: {}",
            self.file, self.line, self.error
        ))
    }
}

impl Error for OsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.error.as_ref())
    }
}

#[derive(Debug)]
pub struct NotSupportedError {
    reason: &'static str,
}

impl NotSupportedError {
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

impl fmt::Display for NotSupportedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Operation isn't supported: {}", self.reason)
    }
}

#[derive(Debug)]
pub enum CreateWindowError {
    InvalidAttribute(InvalidInput),
    Os(OsError),
}

impl From<OsError> for CreateWindowError {
    fn from(err: OsError) -> Self {
        Self::Os(err)
    }
}

impl Display for CreateWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAttribute(InvalidInput { reason }) => {
                write!(f, "Invalid WindowAttributes: {reason}")
            }
            Self::Os(err) => err.fmt(f),
        }
    }
}

impl Error for CreateWindowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::Os(err) = self {
            err.source()
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct InvalidInput {
    reason: &'static str,
}

impl InvalidInput {
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid input: {}", self.reason)
    }
}

impl Error for InvalidInput {}
