use std::{mem, ptr, sync::Arc};

use evloop_core::window::{WindowButtons, WindowLevel};
use windows_sys::Win32::{
	Graphics::Gdi::{RDW_INTERNALPAINT, RedrawWindow},
	UI::{
		Input::KeyboardAndMouse::{EnableWindow, GetActiveWindow},
		WindowsAndMessaging::{
			FLASHW_ALL, FLASHW_STOP, FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx,
			GetForegroundWindow, IsWindowVisible,
		},
	},
};

use crate::{
	sys::window_query::is_minimized,
	window::{
		Window, flags::WindowFlags, force_window_active, state::WindowState,
		taskbar::set_skip_taskbar, r#type::UserAttentionType,
	},
};

impl Window {
	pub fn set_visible(&self, visible: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::VISIBLE, visible)
			});
		});
	}

	pub fn is_visible(&self) -> bool {
		unsafe { IsWindowVisible(self.hwnd()) == true.into() }
	}

	pub fn set_minimized(&self, minimized: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		let is_minimized = is_minimized(self.hwnd());

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags_in_place(&mut window_state.lock().unwrap(), |f| {
				f.set(WindowFlags::MINIMIZED, is_minimized)
			});
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::MINIMIZED, minimized)
			});
		});
	}

	pub fn is_minimized(&self) -> bool {
		is_minimized(self.hwnd())
	}

	pub fn set_maximized(&self, maximized: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::MAXIMIZED, maximized)
			});
		});
	}

	pub fn is_maximized(&self) -> bool {
		let window_state = self.window_state_lock();
		window_state.window_flags.contains(WindowFlags::MAXIMIZED)
	}

	pub fn set_resizable(&self, resizable: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::RESIZABLE, resizable)
			});
		});
	}

	pub fn is_resizable(&self) -> bool {
		let window_state = self.window_state_lock();
		window_state.window_flags.contains(WindowFlags::RESIZABLE)
	}

	pub fn set_enabled_buttons(&self, buttons: WindowButtons) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::MINIMIZABLE, buttons.contains(WindowButtons::MINIMIZE));
				f.set(WindowFlags::MAXIMIZABLE, buttons.contains(WindowButtons::MAXIMIZE));
				f.set(WindowFlags::CLOSABLE, buttons.contains(WindowButtons::CLOSE))
			});
		});
	}

	pub fn enabled_buttons(&self) -> WindowButtons {
		let mut buttons = WindowButtons::empty();
		let window_state = self.window_state_lock();
		if window_state.window_flags.contains(WindowFlags::MINIMIZABLE) {
			buttons |= WindowButtons::MINIMIZE;
		}
		if window_state.window_flags.contains(WindowFlags::MAXIMIZABLE) {
			buttons |= WindowButtons::MAXIMIZE;
		}
		if window_state.window_flags.contains(WindowFlags::CLOSABLE) {
			buttons |= WindowButtons::CLOSE;
		}
		buttons
	}

	pub fn set_window_level(&self, level: WindowLevel) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::ALWAYS_ON_TOP, level == WindowLevel::AlwaysOnTop);
				f.set(WindowFlags::ALWAYS_ON_BOTTOM, level == WindowLevel::AlwaysOnBottom);
			});
		});
	}

	pub fn set_skip_taskbar(&self, skip: bool) {
		self.window_state_lock().skip_taskbar = skip;
		unsafe { set_skip_taskbar(self.hwnd(), skip) };
	}

	pub fn set_enable(&self, enabled: bool) {
		unsafe { EnableWindow(self.hwnd(), enabled.into()) };
	}

	pub fn has_focus(&self) -> bool {
		let window_state = self.window_state.lock().unwrap();
		window_state.has_active_focus()
	}

	pub fn focus_window(&self) {
		let window_flags = self.window_state_lock().window_flags();

		let is_visible = window_flags.contains(WindowFlags::VISIBLE);
		let is_minimized = is_minimized(self.hwnd());
		let is_foreground = self.hwnd() == unsafe { GetForegroundWindow() };

		if is_visible && !is_minimized && !is_foreground {
			unsafe { force_window_active(self.hwnd()) };
		}
	}

	pub fn request_redraw(&self) {
		self.window_state.lock().unwrap().redraw_requested = true;
		unsafe { RedrawWindow(self.hwnd(), ptr::null(), ptr::null_mut(), RDW_INTERNALPAINT) };
	}

	pub fn request_user_attension(&self, request_type: Option<UserAttentionType>) {
		let window = self.window;
		let active_window_handle = unsafe { GetActiveWindow() };
		if window.hwnd() == active_window_handle {
			return;
		}

		self.thread_executor.execute_in_thread(move || unsafe {
			let (flags, count) = request_type
				.map(|ty| match ty {
					UserAttentionType::Critical => (FLASHW_ALL | FLASHW_TIMERNOFG, u32::MAX),
					UserAttentionType::Informational => (FLASHW_TRAY | FLASHW_TIMERNOFG, 0),
				})
				.unwrap_or((FLASHW_STOP, 0));

			let flash_info = FLASHWINFO {
				cbSize: mem::size_of::<FLASHWINFO>() as u32,
				hwnd: window.hwnd(),
				dwFlags: flags,
				uCount: count,
				dwTimeout: 0,
			};
			FlashWindowEx(&flash_info);
		});
	}
}
