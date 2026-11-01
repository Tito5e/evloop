use std::{
	cell::Cell,
	rc::Rc,
	sync::{Arc, Mutex, MutexGuard},
};

use evloop_core::window::WindowId;
use windows_sys::Win32::Foundation::HWND;

use crate::{
	event::WindowEvent,
	event_loop::{Event, EventLoopRunner},
	window::WindowState,
};

pub(crate) struct WindowUserData {
	pub window_state: Arc<Mutex<WindowState>>,
	pub event_loop_runner: Rc<EventLoopRunner>,
	pub key_event_builder: KeyEventBuilder,
	pub userdata_removed: Cell<bool>,
	pub last_tablet_down_button_state: Cell<u32>,
	pub recurse_depth: Cell<u32>,
}

impl WindowUserData {
	pub(crate) fn send_window_event(&self, window: HWND, event: WindowEvent) {
		let window_id = WindowId::from_raw(window as usize);
		self.event_loop_runner.send_event(Event::Window { window_id, event });
	}

	pub(crate) fn window_state_lock(&self) -> MutexGuard<'_, WindowState> {
		self.window_state.lock().unwrap()
	}
}
