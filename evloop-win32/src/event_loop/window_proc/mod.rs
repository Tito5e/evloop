mod handlers;
mod message;
mod user_data;

pub use user_data::*;
use windows_sys::Win32::{
	Foundation::{HWND, LPARAM, LRESULT, WPARAM},
	UI::{
		Controls::WM_MOUSELEAVE,
		WindowsAndMessaging::{
			CREATESTRUCTW, DefWindowProcW, GWL_USERDATA, WM_CAPTURECHANGED, WM_CLOSE, WM_CREATE,
			WM_DESTROY, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_GETMINMAXINFO,
			WM_IME_COMPOSITION, WM_IME_ENDCOMPOSITION, WM_IME_SETCONTEXT, WM_IME_STARTCOMPOSITION,
			WM_INPUTLANGCHANGE, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP,
			WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MENUCHAR, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
			WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCCREATE, WM_NCDESTROY, WM_NCLBUTTONDOWN, WM_PAINT,
			WM_POINTERDOWN, WM_POINTERUP, WM_POINTERUPDATE, WM_RBUTTONDOWN, WM_RBUTTONUP,
			WM_SETCURSOR, WM_SETFOCUS, WM_SETTINGCHANGE, WM_SIZE, WM_SIZING, WM_SYSCOMMAND,
			WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TOUCH, WM_WINDOWPOSCHANGED, WM_WINDOWPOSCHANGING,
			WM_XBUTTONDOWN, WM_XBUTTONUP,
		},
	},
};

use crate::{
	event::WindowEvent::KeyboardInput,
	event_loop::window_proc::{handlers::*, keyboard::update_modifiers, message::WindowMessage},
	sys::{
		proc_result::ProcResult,
		window_long::{get_window_long, set_window_long},
	},
	window::InitData,
};

pub(crate) unsafe extern "system" fn public_window_callback(
	window: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	let userdata = unsafe { get_window_long(window, GWL_USERDATA) };

	let userdata_ptr = match (userdata, msg) {
		(0, WM_NCCREATE) => {
			let createstruct = unsafe { &mut *(lparam as *mut CREATESTRUCTW) };
			let initdata = unsafe { &mut *(createstruct.lpCreateParams as *mut InitData<'_>) };

			let result = match unsafe { initdata.on_nccreate(window) } {
				Some(userdata) => unsafe {
					set_window_long(window, GWL_USERDATA, userdata as _);
					DefWindowProcW(window, msg, wparam, lparam)
				},
				None => -1,
			};

			return result;
		}
		(0, WM_CREATE) => return -1,
		(_, WM_CREATE) => unsafe {
			let createstruct = &mut *(lparam as *mut CREATESTRUCTW);
			let initdata = &mut *(createstruct.lpCreateParams as *mut InitData<'_>);

			initdata.on_create();
			return DefWindowProcW(window, msg, wparam, lparam);
		},
		(0, _) => return unsafe { DefWindowProcW(window, msg, wparam, lparam) },
		_ => userdata as *mut WindowUserData,
	};

	let (result, userdata_removed, recurse_depth) = {
		let userdata = unsafe { &*(userdata_ptr) };

		userdata.recurse_depth.set(userdata.recurse_depth.get() + 1);

		let result = unsafe { public_window_callback_inner(window, msg, wparam, lparam, userdata) };

		let userdata_removed = userdata.userdata_removed.get();
		let recurse_depth = userdata.recurse_depth.get() - 1;
		userdata.recurse_depth.set(recurse_depth);

		(result, userdata_removed, recurse_depth)
	};

	if userdata_removed && recurse_depth == 0 {
		drop(unsafe { Box::from_raw(userdata_ptr) });
	}

	result
}

unsafe fn public_window_callback_inner(
	window: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
	userdata: &WindowUserData,
) -> LRESULT {
	let mut result = ProcResult::DefWindowProc(wparam);

	let mods_changed_callback = || match msg {
		WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
			update_modifiers(window, userdata);
			result = ProcResult::Value(0);
		}
		_ => (),
	};
	userdata
		.event_loop_runner
		.catch_unwind(mods_changed_callback)
		.unwrap_or_else(|| result = ProcResult::Value(-1));

	let keyboard_callback = || {
		let events =
			userdata.key_event_builder.process_message(window, msg, wparam, lparam, &mut result);
		for event in events {
			userdata.send_window_event(
				window,
				KeyboardInput {
					device_id: None,
					event: event.event,
					is_synthetic: event.is_synthetic,
				},
			);
		}
	};
	userdata
		.event_loop_runner
		.catch_unwind(keyboard_callback)
		.unwrap_or_else(|| result = ProcResult::Value(-1));

	let message = WindowMessage { window, msg, wparam, lparam, userdata };
	let callback = || dispatch(&message, &mut result);

	userdata
		.event_loop_runner
		.catch_unwind(callback)
		.unwrap_or_else(|| result = ProcResult::Value(-1));
	//userdata.event_loop_runner.try_execute_drag_drop();

	match result {
		ProcResult::DefWindowProc(wparam) => unsafe { DefWindowProcW(window, msg, wparam, lparam) },
		ProcResult::Value(val) => val,
	}
}

fn dispatch(m: &WindowMessage<'_>, result: &mut ProcResult) {
	match m.msg {
		WM_NCCALCSIZE => geometry::on_wm_nccalcsize(m, result),
		WM_ENTERSIZEMOVE => geometry::on_wm_entersizemove(m, result),
		WM_EXITSIZEMOVE => geometry::on_wm_exitsizemove(m, result),
		WM_NCLBUTTONDOWN => geometry::on_wm_nclbuttondown(m, result),
		WM_CLOSE => lifecycle::on_wm_close(m, result),
		WM_DESTROY => lifecycle::on_wm_destroy(m, result),
		WM_NCDESTROY => lifecycle::on_wm_ncdestroy(m, result),
		WM_PAINT => lifecycle::on_wm_paint(m, result),
		WM_WINDOWPOSCHANGING => geometry::on_wm_windowposchanging(m, result),
		WM_WINDOWPOSCHANGED => geometry::on_wm_windowposchanged(m, result),
		WM_SIZE => geometry::on_wm_size(m, result),
		WM_SIZING => geometry::on_wm_sizing(m, result),
		WM_MENUCHAR => keyboard::on_wm_menuchar(m, result),
		WM_IME_STARTCOMPOSITION => ime::on_wm_ime_startcomposition(m, result),
		WM_IME_COMPOSITION => ime::on_wm_ime_composition(m, result),
		WM_IME_ENDCOMPOSITION => ime::on_wm_ime_endcomposition(m, result),
		WM_IME_SETCONTEXT => ime::on_wm_ime_setcontext(m, result),
		WM_INPUTLANGCHANGE => keyboard::on_wm_inputlangchange(m, result),
		WM_SYSCOMMAND => lifecycle::on_wm_syscommand(m, result),
		WM_MOUSEMOVE => mouse::on_wm_mousemove(m, result),
		WM_MOUSELEAVE => mouse::on_wm_mouseleave(m, result),
		WM_MOUSEWHEEL => mouse::on_wm_mousewheel(m, result),
		WM_MOUSEHWHEEL => mouse::on_wm_mousehwheel(m, result),
		WM_KEYDOWN | WM_SYSKEYDOWN => keyboard::on_key_down(m, result),
		WM_KEYUP | WM_SYSKEYUP => keyboard::on_key_up(m, result),
		WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => mouse::on_mouse_button_down(m, result),
		WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => mouse::on_mouse_button_up(m, result),
		WM_XBUTTONDOWN => mouse::on_wm_xbuttondown(m, result),
		WM_XBUTTONUP => mouse::on_wm_xbuttonup(m, result),
		WM_CAPTURECHANGED => mouse::on_wm_capturechanged(m, result),
		WM_TOUCH => touch::on_wm_touch(m, result),
		WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP => pointer::on_pointer_event(m, result),
		WM_NCACTIVATE => focus::on_wm_ncactivate(m, result),
		WM_SETFOCUS => focus::on_wm_setfocus(m, result),
		WM_KILLFOCUS => focus::on_wm_killfocus(m, result),
		WM_SETCURSOR => mouse::on_wm_setcursor(m, result),
		WM_GETMINMAXINFO => geometry::on_wm_getminmaxinfo(m, result),
		WM_DPICHANGED => dpi::on_wm_dpichanged(m, result),
		WM_SETTINGCHANGE => system_setting::on_wm_settingchange(m, result),
		_ => custom::on_custom_message(m, result),
	}
}
