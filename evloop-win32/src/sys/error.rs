use std::io;

use windows_sys::core::BOOL;

macro_rules! os_error {
	($error:expr) => {{ evloop_core::error::OsError::new(line!(), file!(), $error) }};
}

pub(crate) fn win_to_err(result: BOOL) -> Result<(), io::Error> {
	if result != false.into() { Ok(()) } else { Err(io::Error::last_os_error()) }
}
