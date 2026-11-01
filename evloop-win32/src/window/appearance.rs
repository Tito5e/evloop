use std::{mem, sync::Arc};

use evloop_core::window::Theme;
use windows_sys::Win32::{
	Graphics::Dwm::{
		DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE, DWMWA_BORDER_COLOR,
		DWMWA_CAPTION_COLOR, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_TEXT_COLOR,
		DWMWA_WINDOW_CORNER_PREFERENCE, DwmSetWindowAttribute,
	},
	UI::WindowsAndMessaging::{
		GetWindowTextLengthW, GetWindowTextW, SendMessageW, SetWindowDisplayAffinity,
		SetWindowTextW, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WM_SETICON,
	},
};

use crate::{
	resource::icon::{Icon, IconType},
	sys::wide::{decode_wide, encode_wide},
	window::{
		Window,
		dark_mode::try_theme,
		dwm::{BackdropType, Color, CornerPreference},
		flags::WindowFlags,
		state::WindowState,
	},
};

impl Window {
	pub fn set_title(&self, text: &str) {
		let wide_text = encode_wide(text);
		unsafe {
			SetWindowTextW(self.hwnd(), wide_text.as_ptr());
		}
	}

	pub fn title(&self) -> String {
		let len = unsafe { GetWindowTextLengthW(self.hwnd()) } + 1;
		let mut buf = vec![0; len as usize];
		unsafe { GetWindowTextW(self.hwnd(), buf.as_mut_ptr(), len) };
		decode_wide(&buf).to_string_lossy().to_string()
	}

	pub fn set_transparent(&self, transparent: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);
		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::TRANSPARENT, transparent)
			});
		});
	}

	pub fn set_decorations(&self, decorations: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::MARKER_DECORATIONS, decorations)
			});
		});
	}

	pub fn has_decorations(&self) -> bool {
		let window_state = self.window_state_lock();
		window_state.window_flags.contains(WindowFlags::MARKER_DECORATIONS)
	}

	pub fn set_theme(&self, theme: Option<Theme>) {
		self.window_state_lock().current_theme = try_theme(self.hwnd(), theme, true);
	}

	pub fn theme(&self) -> Theme {
		self.window_state_lock().current_theme
	}

	pub fn set_content_protected(&self, protected: bool) {
		unsafe {
			SetWindowDisplayAffinity(
				self.hwnd(),
				if protected { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE },
			)
		};
	}

	pub fn set_window_icon(&self, window_icon: Option<Icon>) {
		if let Some(window_icon) = window_icon {
			unsafe {
				SendMessageW(
					self.hwnd(),
					WM_SETICON,
					IconType::Small as usize,
					window_icon.as_raw_handle() as isize,
				)
			};

			self.window_state_lock().window_icon = Some(window_icon)
		} else {
			unsafe { SendMessageW(self.hwnd(), WM_SETICON, IconType::Small as usize, 0) };

			self.window_state_lock().window_icon = None;
		}
	}

	pub fn set_taskbar_icon(&self, taskbar_icon: Option<Icon>) {
		if let Some(taskbar_icon) = taskbar_icon {
			unsafe {
				SendMessageW(
					self.hwnd(),
					WM_SETICON,
					IconType::Big as usize,
					taskbar_icon.as_raw_handle() as isize,
				)
			};

			self.window_state_lock().taskbar_icon = Some(taskbar_icon)
		} else {
			unsafe { SendMessageW(self.hwnd(), WM_SETICON, IconType::Big as usize, 0) };

			self.window_state_lock().taskbar_icon = None;
		}
	}

	pub fn set_system_backdrop(&self, backdrop_type: BackdropType) {
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_SYSTEMBACKDROP_TYPE as u32,
				&(backdrop_type as i32) as *const _ as _,
				mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as _,
			)
		};
	}

	pub fn set_border_color(&self, color: Option<Color>) {
		let color = color.unwrap_or(Color::NONE);
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_BORDER_COLOR as u32,
				&color as *const _ as _,
				mem::size_of::<Color>() as _,
			)
		};
	}

	pub fn set_title_background_color(&self, color: Option<Color>) {
		let color = color.unwrap_or(Color::NONE);
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_CAPTION_COLOR as u32,
				&color as *const _ as _,
				mem::size_of::<Color>() as _,
			)
		};
	}

	pub fn set_title_text_color(&self, color: Color) {
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_TEXT_COLOR as u32,
				&color as *const _ as _,
				mem::size_of::<Color>() as _,
			)
		};
	}

	pub fn set_corner_preference(&self, preference: CornerPreference) {
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_WINDOW_CORNER_PREFERENCE as u32,
				&(preference as DWM_WINDOW_CORNER_PREFERENCE) as *const _ as _,
				mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as _,
			)
		};
	}

	pub fn set_undecorated_shadow(&self, shadow: bool) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::MARKER_UNDECORATED_SHADOW, shadow)
			});
		});
	}
}
