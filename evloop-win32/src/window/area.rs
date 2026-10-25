use std::{io, mem};

use windows_sys::Win32::{
	Foundation::{HWND, RECT},
	Graphics::Gdi::ClientToScreen,
	UI::WindowsAndMessaging::{GetClientRect, GetWindowRect},
};

use crate::sys::error::win_to_err;

pub enum WindowArea {
	Outer,
	Inner,
}

impl WindowArea {
	pub fn get_rect(self, hwnd: HWND) -> Result<RECT, io::Error> {
		let mut rect = unsafe { mem::zeroed() };

		match self {
			WindowArea::Outer => {
				win_to_err(unsafe { GetWindowRect(hwnd, &mut rect) })?;
			}
			WindowArea::Inner => unsafe {
				let mut top_left = mem::zeroed();

				win_to_err(ClientToScreen(hwnd, &mut top_left))?;
				win_to_err(GetClientRect(hwnd, &mut rect))?;
				rect.left += top_left.x;
				rect.top += top_left.y;
				rect.right += top_left.x;
				rect.bottom += top_left.y;
			},
		}

		Ok(rect)
	}
}
