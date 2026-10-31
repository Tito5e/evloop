use core::fmt;
use std::{error::Error, fmt::Display};

use crate::data_transfer::DataTransferId;

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
		error: impl Into<Box<dyn Error + Send + Sync + 'static>>,
	) -> Self {
		Self { line, file, error: error.into() }
	}
}

impl fmt::Display for OsError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.pad(&format!("os error at {}:{}: {}", self.file, self.line, self.error))
	}
}

impl Error for OsError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		Some(self.error.as_ref())
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub enum CustomCursorError {
	/// Custom cursors are not supported.
	NotSupported,
	/// The [`CustomCursorSource`](crate::cursor::CustomCursorSource) variant is not supported.
	UnsupportedSource,
	/// Got unspecified OS specific error during the request.
	Os(OsError),
}

impl Display for CustomCursorError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NotSupported => write!(f, "create_custom_cursor is not supported"),
			Self::UnsupportedSource => {
				write!(f, "the provided CustomCursorSource is not supported")
			}
			Self::Os(err) => err.fmt(f),
		}
	}
}
impl Error for CustomCursorError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		if let Self::Os(err) = self { err.source() } else { None }
	}
}

impl From<OsError> for CustomCursorError {
	fn from(value: OsError) -> Self {
		Self::Os(value)
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
		if let Self::Os(err) = self { err.source() } else { None }
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

#[derive(Debug)]
#[non_exhaustive]
pub enum TransferError {
	NotSupported(NotSupportedError),
	UnknownTransfer(DataTransferId),
	Failed,
	Os(OsError),
}

impl fmt::Display for TransferError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NotSupported(err) => err.fmt(f),
			Self::UnknownTransfer(id) => {
				write!(f, "Unknown data transfer with ID {}", id.into_raw())
			}
			Self::Failed => write!(f, "Transfer request failed"),
			Self::Os(err) => err.fmt(f),
		}
	}
}

impl Error for TransferError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		if let Self::Os(err) = self { err.source() } else { None }
	}
}

impl From<NotSupportedError> for TransferError {
	fn from(value: NotSupportedError) -> Self {
		Self::NotSupported(value)
	}
}

impl From<OsError> for TransferError {
	fn from(value: OsError) -> Self {
		Self::Os(value)
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ImeSurroundingTextError {
	TextTooLong,
	CursorBadPosition,
	AnchorBadPosition,
}

impl fmt::Display for ImeSurroundingTextError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ImeSurroundingTextError::TextTooLong => write!(f, "text exceeds maximum length"),
			ImeSurroundingTextError::CursorBadPosition => {
				write!(f, "cursor is not at a valid text index")
			}
			ImeSurroundingTextError::AnchorBadPosition => {
				write!(f, "anchor is not at a valid text index")
			}
		}
	}
}

impl Error for ImeSurroundingTextError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeRequestError {
	NotEnabled,
	AlreadyEnabled,
	NotSupported,
}

impl fmt::Display for ImeRequestError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ImeRequestError::NotEnabled => write!(f, "ime is not enabled."),
			ImeRequestError::AlreadyEnabled => write!(f, "ime is already enabled."),
			ImeRequestError::NotSupported => write!(f, "ime is not supported."),
		}
	}
}
