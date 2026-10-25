use std::{
	io, mem, ptr,
	sync::atomic::{AtomicBool, Ordering},
};

use windows_sys::{
	Win32::{
		Foundation::{POINT, RECT},
		UI::WindowsAndMessaging::{ClipCursor, GetClipCursor, GetCursorPos, ShowCursor},
	},
	core::BOOL,
};

use crate::sys::error::win_to_err;

pub fn set_cursor_hidden(hidden: bool) {
	static HIDDEN: AtomicBool = AtomicBool::new(false);
	let changed = HIDDEN.swap(hidden, Ordering::SeqCst) ^ hidden;
	if changed {
		unsafe { ShowCursor(BOOL::from(!hidden)) };
	}
}

pub fn get_cursor_position() -> Result<POINT, io::Error> {
	unsafe {
		let mut point: POINT = mem::zeroed();
		win_to_err(GetCursorPos(&mut point)).map(|_| point)
	}
}

pub fn get_cursor_clip() -> Result<RECT, io::Error> {
	unsafe {
		let mut rect: RECT = mem::zeroed();
		win_to_err(GetClipCursor(&mut rect)).map(|_| rect)
	}
}

pub fn set_cursor_clip(rect: Option<RECT>) -> Result<(), io::Error> {
	unsafe {
		let rect_ptr = rect.as_ref().map(|r| r as *const RECT).unwrap_or(ptr::null());
		win_to_err(ClipCursor(rect_ptr))
	}
}
