use std::mem;

use windows_sys::Win32::{
	Foundation::{HWND, RECT},
	UI::{
		Input::KeyboardAndMouse::GetActiveWindow,
		WindowsAndMessaging::{
			GetSystemMetrics, GetWindowPlacement, IsIconic, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
			SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_MAXIMIZE, WINDOWPLACEMENT,
		},
	},
};

pub fn is_maximized(window: HWND) -> bool {
	unsafe {
		let mut placement: WINDOWPLACEMENT = mem::zeroed();
		placement.length = mem::size_of::<WINDOWPLACEMENT>() as u32;
		GetWindowPlacement(window, &mut placement);
		placement.showCmd == SW_MAXIMIZE as u32
	}
}

pub fn is_minimized(window: HWND) -> bool {
	unsafe { IsIconic(window) != false.into() }
}

pub fn is_focused(window: HWND) -> bool {
	window == unsafe { GetActiveWindow() }
}

pub fn get_desktop_rect() -> RECT {
	unsafe {
		let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
		let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
		RECT {
			left,
			top,
			right: left + GetSystemMetrics(SM_CXVIRTUALSCREEN),
			bottom: top + GetSystemMetrics(SM_CYVIRTUALSCREEN),
		}
	}
}
