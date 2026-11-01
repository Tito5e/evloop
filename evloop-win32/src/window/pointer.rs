use std::{
	io, ptr,
	sync::{Arc, mpsc::channel},
};

use evloop_core::{dpi::Position, error::OsError};
use windows_sys::Win32::{
	Foundation::POINT,
	Graphics::Gdi::ClientToScreen,
	UI::WindowsAndMessaging::{LoadCursorW, SetCursor, SetCursorPos},
};

use crate::{
	resource::cursor::{SelectedCursor, to_windows_cursor},
	window::{
		Window,
		flags::WindowFlags,
		state::{CursorFlags, WindowState},
		r#type::CursorGrabMode,
	},
};

impl Window {
	pub fn set_cursor(&self, cursor: impl Into<SelectedCursor>) {
		let cursor = cursor.into();
		self.window_state_lock().mouse.selected_cursor = cursor.clone();
		match cursor {
			SelectedCursor::Named(icon) => {
				self.thread_executor.execute_in_thread(move || unsafe {
					let cursor = LoadCursorW(ptr::null_mut(), to_windows_cursor(icon));
					SetCursor(cursor);
				});
			}
			SelectedCursor::Custom(handle) => {
				self.thread_executor.execute_in_thread(move || unsafe {
					SetCursor(handle.as_raw_handle());
				});
			}
		}
	}

	pub fn set_cursor_visible(&self, visible: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);
		let (tx, rx) = channel();

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			let result = window_state
				.lock()
				.unwrap()
				.mouse
				.set_cursor_flags(window.hwnd(), |f| f.set(CursorFlags::HIDDEN, !visible))
				.map_err(|e| e.to_string());
			let _ = tx.send(result);
		});
		rx.recv().unwrap().ok();
	}

	pub fn set_cursor_grab(&self, mode: CursorGrabMode) -> Result<(), OsError> {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);
		let (tx, rx) = channel();

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			let result = window_state
				.lock()
				.unwrap()
				.mouse
				.set_cursor_flags(window.hwnd(), |f| {
					f.set(CursorFlags::GRABBED, mode != CursorGrabMode::None);
					f.set(CursorFlags::LOCKED, mode == CursorGrabMode::Locked);
				})
				.map_err(|err| os_error!(err));
			let _ = tx.send(result);
		});

		rx.recv().unwrap()
	}

	pub fn set_cursor_position(&self, position: Position) -> Result<(), OsError> {
		let scale_factor = self.scale_factor();
		let physical = position.to_physical(scale_factor);
		let (x, y) = (physical.x, physical.y);

		let mut point = POINT { x, y };
		unsafe {
			if ClientToScreen(self.hwnd(), &mut point) == false.into() {
				return Err(os_error!(io::Error::last_os_error()));
			}
			if SetCursorPos(point.x, point.y) == false.into() {
				return Err(os_error!(io::Error::last_os_error()));
			}
		}

		Ok(())
	}

	pub fn set_cursor_hittest(&self, hittest: bool) -> Result<(), OsError> {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);
		self.thread_executor.execute_in_thread(move || {
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::IGNORE_CURSOR_EVENT, !hittest)
			});
		});

		Ok(())
	}

	pub fn set_use_system_scroll_speed(&self, should_use: bool) {
		self.window_state_lock().use_system_wheel_speed = should_use;
	}
}
