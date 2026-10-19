use core::fmt;
use std::error::Error;

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
