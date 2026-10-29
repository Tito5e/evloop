use std::{ptr, sync::LazyLock};

use windows_sys::Win32::{
	Foundation::HWND,
	UI::WindowsAndMessaging::{
		CS_HREDRAW, CS_VREDRAW, CreateWindowExW, GWL_STYLE, RegisterClassExW, WNDCLASSEXW,
		WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED,
		WS_POPUP, WS_VISIBLE,
	},
};

use crate::sys::{dynamic::get_instance_handle, wide::encode_wide};

static THREAD_EVENT_TARGET_WINDOW_CLASS_NAME: LazyLock<Vec<u16>> =
	LazyLock::new(|| encode_wide("Evloop Thread Event Target"));

pub(crate) fn create_event_target_window() -> HWND {
	unsafe {
		let class = WNDCLASSEXW {
			cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
			style: CS_HREDRAW | CS_VREDRAW,
			lpfnWndProc: Some(thread_event_target_callback),
			cbClsExtra: 0,
			cbWndExtra: 0,
			hInstance: get_instance_handle(),
			hIcon: ptr::null_mut(),
			hCursor: ptr::null_mut(),
			hbrBackground: ptr::null_mut(),
			lpszMenuName: THREAD_EVENT_TARGET_WINDOW_CLASS_NAME.as_ptr(),
			hIconSm: ptr::null_mut(),
		};

		RegisterClassExW(&class);
	}

	unsafe {
		let window = CreateWindowExW(
			WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
			THREAD_EVENT_TARGET_WINDOW_CLASS_NAME.as_ptr(),
			ptr::null(),
			WS_OVERLAPPED,
			0,
			0,
			0,
			0,
			ptr::null_mut(),
			ptr::null_mut(),
			get_instance_handle(),
			ptr::null(),
		);

		set_window_long(window, GWL_STYLE, (WS_VISIBLE | WS_POPUP) as isize);

		window
	}
}
