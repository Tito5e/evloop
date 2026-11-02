use std::mem;

use evloop_core::{
	event::ElementState,
	pointer::{ButtonSource::Touch, FingerId, PointerKind, PointerSource},
};
use geometry_types::PhysicalPosition;
use windows_sys::Win32::{
	Foundation::POINT,
	Graphics::Gdi::ScreenToClient,
	UI::Input::Touch::{
		CloseTouchInputHandle, GetTouchInputInfo, TOUCHEVENTF_DOWN, TOUCHEVENTF_MOVE,
		TOUCHEVENTF_PRIMARY, TOUCHEVENTF_UP, TOUCHINPUT,
	},
};

use crate::{
	event::WindowEvent,
	event_loop::window_proc::message::WindowMessage,
	sys::{bits::has_flag, msg_params::loword, proc_result::ProcResult},
};

pub(crate) fn on_wm_touch(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;

	let pcount = loword(wparam as u32) as usize;
	let mut inputs = Vec::with_capacity(pcount);
	let htouch = lparam as *mut _;
	if unsafe {
		GetTouchInputInfo(
			htouch,
			pcount as u32,
			inputs.as_mut_ptr(),
			mem::size_of::<TOUCHINPUT>() as i32,
		) > 0
	} {
		unsafe { inputs.set_len(pcount) };
		for input in &inputs {
			let mut position = POINT { x: input.x / 100, y: input.y / 100 };

			if unsafe { ScreenToClient(window, &mut position) } == false.into() {
				continue;
			}

			let x = position.x as f64 + (input.x % 100) as f64 / 100f64;
			let y = position.y as f64 + (input.y % 100) as f64 / 100f64;
			let position = PhysicalPosition::new(x as i32, y as i32);

			let finger_id = FingerId::from_raw(input.dwID as usize);
			let primary = has_flag(input.dwFlags, TOUCHEVENTF_PRIMARY);

			if has_flag(input.dwFlags, TOUCHEVENTF_DOWN) {
				userdata.send_window_event(
					window,
					WindowEvent::PointerEntered {
						device_id: None,
						primary,
						position,
						kind: PointerKind::Touch(finger_id),
					},
				);
				userdata.send_window_event(
					window,
					WindowEvent::PointerButton {
						device_id: None,
						primary,
						state: ElementState::Pressed,
						position,
						button: Touch { finger_id, force: None },
					},
				);
			} else if has_flag(input.dwFlags, TOUCHEVENTF_UP) {
				userdata.send_window_event(
					window,
					WindowEvent::PointerButton {
						device_id: None,
						primary,
						state: ElementState::Released,
						position,
						button: Touch { finger_id, force: None },
					},
				);
				userdata.send_window_event(
					window,
					WindowEvent::PointerLeft {
						device_id: None,
						primary,
						position: Some(position),
						kind: PointerKind::Touch(finger_id),
					},
				);
			} else if has_flag(input.dwFlags, TOUCHEVENTF_MOVE) {
				userdata.send_window_event(
					window,
					WindowEvent::PointerMoved {
						device_id: None,
						primary,
						position,
						source: PointerSource::Touch { finger_id, force: None },
					},
				);
			} else {
				continue;
			}
		}
	}
	unsafe { CloseTouchInputHandle(htouch) };
	*result = ProcResult::Value(0);
}
