use core::fmt;
use std::cell::Cell;

use windows_sys::{
	Win32::{
		Foundation::{OLE_E_WRONGCOMPOBJ, RPC_E_CHANGED_MODE, S_FALSE, S_OK},
		System::Ole::OleInitialize,
	},
	core::HRESULT,
};

#[derive(Debug, Copy, Clone)]
pub(crate) struct OleInitError(HRESULT);

impl fmt::Display for OleInitError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0 {
			RPC_E_CHANGED_MODE => write!(
				f,
				"OleInitialize failed: `RPC_E_CHANGED_MODE`. Make sure other crates are not using \
                 multithreaded COM library on the same thread or disable drag-and-drop and \
                 clipboard support."
			),
			OLE_E_WRONGCOMPOBJ => write!(f, "OleInitialize failed: `OLE_E_WRONGCOMPOBJ`"),
			hr => write!(f, "OleInitialize failed: 0x{hr:08x}"),
		}
	}
}

impl std::error::Error for OleInitError {}

thread_local! {
	static OLE_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn ensure_ole_initialized() -> Result<(), OleInitError> {
	if OLE_INITIALIZED.get() {
		return Ok(());
	}

	let hr = unsafe { OleInitialize(std::ptr::null_mut()) };
	// `S_FALSE` means OLE was already initialized on this thread.
	if hr != S_OK && hr != S_FALSE {
		return Err(OleInitError(hr));
	}

	OLE_INITIALIZED.set(true);
	Ok(())
}
