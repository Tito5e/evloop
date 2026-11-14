//! Data transfer: the clipboard and drag and drop.
//!
//! # API (same names and semantics on every backend)
//!
//! On `ActiveEventLoop`:
//!
//! ```ignore
//! fn available_formats(&self, id: DataTransferId) -> Result<Vec<TypeHint>, TransferError>;
//! fn fetch_data_transfer(&self, id: DataTransferId, format: &TypeHint)
//!     -> Result<AsyncRequestSerial, TransferError>;
//! fn set_valid_dnd_actions(&self, id: DataTransferId, actions: &[DndAction])
//!     -> Result<(), TransferError>;
//! fn clipboard_transfer_id(&self) -> Result<DataTransferId, TransferError>;
//! fn set_clipboard(&self, source: DataSource) -> Result<(), TransferError>;
//! fn clear_clipboard(&self) -> Result<(), TransferError>;
//! ```
//!
//! On `Window`:
//!
//! ```ignore
//! fn start_drag(&self, source: DataSource, actions: &[DndAction], image: Option<DragImage>)
//!     -> Result<DataTransferId, TransferError>;
//! ```
//!
//! Events (`WindowEvent`): `DragEntered`, `DragPosition`, `DragDropped`, `DragLeft`,
//! `OutgoingDragDropped`, `OutgoingDragCanceled`, `DataTransferReceived`, `ClipboardChanged`.
//!
//! # Reading data
//!
//! - [`available_formats`] returns, synchronously, the formats the transfer offers: only those
//!   the platform actually reports (no conversions it might perform), in the platform's order
//!   (which is not a priority).
//! - `fetch_data_transfer` requests the data in one format and returns a new token. The result
//!   arrives as `WindowEvent::DataTransferReceived { id, serial, value }`. It can be called
//!   several times for the same id; a format not returned by `available_formats` is an error.
//! - Delivery of the result is best effort.
//!
//! # Where results go
//!
//! `DataTransferReceived` is a window event:
//!
//! - Drag and drop: the window the data is dropped on.
//! - Clipboard: the window that has focus **when `fetch_data_transfer` is called**; it is
//!   recorded with the request, so a later focus change does not redirect the result. Without a
//!   focused window, `fetch_data_transfer` fails with [`TransferError::NoFocusedWindow`]
//!   (reading the clipboard without focus is not allowed on several platforms, so it is not
//!   allowed on any).
//! - If that window is closed before the result arrives, the result is dropped.
//!
//! `ClipboardChanged` is sent to the focused window. A change while no window has focus is
//! reported once, when a window gains focus.
//!
//! # Lifetimes of transfer ids
//!
//! - Clipboard: [`clipboard_transfer_id`] identifies the current content. It changes whenever
//!   the content changes; an outdated id fails with [`TransferError::UnknownTransfer`].
//! - Incoming drag: from `DragEntered` until `DragLeft`, or until the end of the handling of
//!   `DragDropped`. **`fetch_data_transfer` must be called by the end of the `DragDropped`
//!   handler**; afterwards the data is not guaranteed to be available.
//!
//! [`available_formats`]: #reading-data
//! [`clipboard_transfer_id`]: #lifetimes-of-transfer-ids

mod action;
mod data;
mod hint;
mod source;

pub use action::*;
pub use data::*;
pub use hint::*;
pub use source::*;

/// Identifies a data transfer: the current clipboard content or a drag.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DataTransferId(i64);

impl DataTransferId {
	pub const fn into_raw(&self) -> i64 {
		self.0
	}

	pub const fn from_raw(id: i64) -> Self {
		Self(id)
	}
}
