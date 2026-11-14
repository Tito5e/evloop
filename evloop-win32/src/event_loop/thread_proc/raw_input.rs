use std::rc::Rc;

use evloop_core::{
	event::{DeviceId, ElementState},
	pointer::MouseScrollDelta,
};
use windows_sys::Win32::UI::{
	Input::{HRAWINPUT, MOUSE_MOVE_ABSOLUTE, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE},
	WindowsAndMessaging::{
		RI_MOUSE_HWHEEL, RI_MOUSE_WHEEL, WHEEL_DELTA, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
		WM_SYSKEYUP,
	},
};

use crate::{
	event::{DeviceEvent, RawKeyEvent},
	event_loop::runner::{Event, EventLoopRunner},
	input::raw_input::{get_keyboard_physical_key, get_raw_input_data, get_raw_mouse_button_state},
	sys::bits::has_flag,
};

/// Translates one `WM_INPUT` message into device events.
pub(super) fn handle_raw_input(runner: &Rc<EventLoopRunner>, handle: HRAWINPUT) {
	let Some(data) = get_raw_input_data(handle) else {
		return;
	};

	let device_id = Some(DeviceId::from_raw(data.header.hDevice as i64));
	let send = |event: DeviceEvent| runner.send_event(Event::Device { device_id, event });

	match data.header.dwType {
		RIM_TYPEMOUSE => {
			let mouse = unsafe { data.data.mouse };

			// Absolute motion (tablets, remote desktop) has no meaningful delta.
			if !has_flag(mouse.usFlags, MOUSE_MOVE_ABSOLUTE) {
				let (x, y) = (mouse.lLastX as f64, mouse.lLastY as f64);
				if x != 0.0 || y != 0.0 {
					send(DeviceEvent::PointerMotion { delta: (x, y) });
				}
			}

			let button_flags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags } as u32;
			let button_data = unsafe { mouse.Anonymous.Anonymous.usButtonData } as i16;

			if has_flag(button_flags, RI_MOUSE_WHEEL) {
				let delta = button_data as f32 / WHEEL_DELTA as f32;
				send(DeviceEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(0.0, delta) });
			}

			if has_flag(button_flags, RI_MOUSE_HWHEEL) {
				// Inverted, consistent with `WM_MOUSEHWHEEL` handling.
				let delta = -(button_data as f32) / WHEEL_DELTA as f32;
				send(DeviceEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(delta, 0.0) });
			}

			for (button, state) in get_raw_mouse_button_state(button_flags).into_iter().enumerate()
			{
				if let Some(state) = state {
					send(DeviceEvent::Button { button: button as u32, state });
				}
			}
		}
		RIM_TYPEKEYBOARD => {
			let keyboard = unsafe { data.data.keyboard };

			let state = match keyboard.Message {
				WM_KEYDOWN | WM_SYSKEYDOWN => ElementState::Pressed,
				WM_KEYUP | WM_SYSKEYUP => ElementState::Released,
				_ => return,
			};

			if let Some(physical_key) = get_keyboard_physical_key(keyboard) {
				send(DeviceEvent::Key(RawKeyEvent { physical_key, state }));
			}
		}
		_ => {}
	}
}
