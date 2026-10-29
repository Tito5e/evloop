mod user_data;

use std::{mem, ptr, rc::Rc, sync::LazyLock};

use windows_sys::Win32::{
	Foundation::{HWND, LPARAM, LRESULT, WPARAM},
	Graphics::Gdi::{RDW_INTERNALPAINT, RedrawWindow, ValidateRect},
	UI::WindowsAndMessaging::{
		CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GWL_STYLE, GWL_USERDATA,
		RegisterClassExW, WM_INPUT, WM_NCDESTROY, WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED,
		WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
	},
};

use crate::{
	event_loop::{runner::EventLoopRunner, thread_proc::user_data::ThreadTargetUserData},
	sys::{
		dynamic::get_instance_handle,
		message_id::{EXEC_MSG_ID, USER_EVENT_MSG_ID},
		wide::encode_wide,
	},
};

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
			lpszMenuName: ptr::null(),
			lpszClassName: THREAD_EVENT_TARGET_WINDOW_CLASS_NAME.as_ptr(),
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

pub(crate) fn insert_event_target_window_userdata(
	thread_msg_target: HWND,
	event_loop_runner: Rc<EventLoopRunner>,
) {
	let userdata = ThreadTargetUserData { event_loop_runner };
	let input_ptr = Box::into_raw(Box::new(userdata));

	unsafe { set_window_long(thread_msg_target, GWL_USERDATA, input_ptr as isize) };
}

unsafe extern "system" fn thread_event_target_callback(
	window: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	let userdata_ptr =
		unsafe { get_window_long(window, GWL_USERDATA) } as *mut ThreadTargetUserData;
	if userdata_ptr.is_null() {
		return unsafe { DefWindowProcW(window, msg, wparam, lparam) };
	}
	let userdata = unsafe { Box::from_raw(userdata_ptr) };

	if msg != WM_PAINT {
		unsafe { RedrawWindow(window, ptr::null(), ptr::null_mut(), RDW_INTERNALPAINT) };
	}

	let mut userdata_removed = false;

	let callback = || match msg {
		WM_NCDESTROY => {
			unsafe { set_window_long(window, GWL_USERDATA, 0) };
			userdata_removed = true;
			0
		}
		WM_PAINT => unsafe {
			ValidateRect(window, ptr::null());
			DefWindowProcW(window, msg, wparam, lparam)
		},
		WM_INPUT => {
			if let Some(data) = get_raw_input_data(lparam as _) {
				unsafe { handle_raw_input(&userdata, data) };
			}

			unsafe { DefWindowProcW(window, msg, wparam, lparam) }
		}
		_ if msg == USER_EVENT_MSG_ID.get() => {
			userdata.send_wakeup();
			0
		}
		_ if msg == EXEC_MSG_ID.get() => {
			let mut function: ThreadExecFn = unsafe { Box::from_raw(wparam as *mut _) };
			function();
			0
		}
		_ => unsafe { DefWindowProcW(window, msg, wparam, lparam) },
	};

	let result = userdata.event_loop_runner.catch_unwind(callback).unwrap_or(-1);
	if userdata_removed {
		drop(userdata);
	} else {
		Box::leak(userdata);
	}
	result
}
