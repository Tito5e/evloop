use std::{io, mem};

use windows_sys::Win32::{
	Foundation::{HWND, RECT},
	Graphics::Gdi::ClientToScreen,
	UI::WindowsAndMessaging::{GetClientRect, GetWindowRect},
};

use crate::sys::error::win_to_err;

pub(crate) fn get_outer_rect(hwnd: HWND) -> Result<RECT, io::Error> {
	let mut rect = unsafe { mem::zeroed() };
	win_to_err(unsafe { GetWindowRect(hwnd, &mut rect) })?;

	Ok(rect)
}

pub(crate) fn get_inner_rect(hwnd: HWND) -> Result<RECT, io::Error> {
	unsafe {
		let mut rect = mem::zeroed();
		let mut top_left = mem::zeroed();
		win_to_err(ClientToScreen(hwnd, &mut top_left))?;
		win_to_err(GetClientRect(hwnd, &mut rect))?;
		rect.left += top_left.x;
		rect.top += top_left.y;
		rect.right += top_left.x;
		rect.bottom += top_left.y;

		Ok(rect)
	}
}
