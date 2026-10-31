use std::{
	cell::Cell,
	rc::Rc,
	sync::{Arc, Mutex},
};

use evloop_core::dpi::Size;
use geometry_types::PhysicalSize;
use windows_sys::Win32::{
	Foundation::HWND,
	Graphics::{
		Dwm::{DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND, DwmEnableBlurBehindWindow},
		Gdi::{CreateRectRgn, DeleteObject},
	},
	UI::{
		Input::Touch::{RegisterTouchWindow, TWF_WANTPALM},
		WindowsAndMessaging::{GetSystemMetrics, NID_READY, SM_DIGITIZER},
	},
};

use crate::{
	event_loop::EventLoopRunner,
	ime::ImeContext,
	sys::{
		self,
		dpi::{dpi_to_scale_factor, enable_non_client_dpi_scaling},
	},
	window::{
		SyncWindowHandle, Window, attributes::WindowAttributes, flags::WindowFlags,
		state::WindowState,
	},
};

pub(crate) struct InitData<'a> {
	pub runner: &'a Rc<EventLoopRunner>,
	pub attributes: WindowAttributes,
	pub window_flags: WindowFlags,
	pub window: Option<Window>,
}

impl InitData<'_> {
	unsafe fn create_window(&self, window: HWND) -> Window {
		{
			let digitizer = unsafe { GetSystemMetrics(SM_DIGITIZER) as u32 };
			if digitizer & NID_READY != 0 {
				unsafe { RegisterTouchWindow(window, TWF_WANTPALM) };
			}
		}

		let dpi = unsafe { sys::dpi::get_window_dpi(window) };
		let scale_factor = dpi_to_scale_factor(dpi);

		let current_theme = try_theme(window, self.attributes.preferred_theme, false);

		let window_state = {
			let window_state = WindowState::new(
				&self.attributes,
				scale_factor,
				current_theme,
				self.attributes.preferred_theme,
			);
			let window_state = Arc::new(Mutex::new(window_state));
			WindowState::set_window_flags(window_state.lock().unwrap(), window, |f| {
				*f = self.window_flags
			});
			window_state
		};

		enable_non_client_dpi_scaling(window);

		unsafe { ImeContext::set_ime_allowed(window, false) };

		Window {
			window: SyncWindowHandle(window),
			window_state,
			thread_executor: self.runner.create_thread_executor(),
		}
	}

	unsafe fn create_window_data(&self, win: &Window) -> WindowData {
		WindowData {
			window_state: win.window_state.clone(),
			event_loop_runner: self.runner.clone(),
			key_event_builder: KeyEventBuilder::default(),
			userdata_removed: Cell::new(false),
			recurse_depth: Cell::new(0),
			last_tablet_down_button_state: Cell::new(0),
		}
	}

	pub unsafe fn on_nccreate(&mut self, window: HWND) -> Option<isize> {
		let runner = self.runner.clone();
		let result = runner.catch_unwind(|| {
			let window = unsafe { self.create_window(window) };
			let window_data = unsafe { self.create_window_data(&window) };
			(window, window_data)
		});

		result.map(|(win, userdata)| {
			self.runner.register_window(win.id());
			self.window = Some(win);
			let userdata = Box::into_raw(Box::new(userdata));
			userdata as _
		})
	}

	pub unsafe fn on_create(&mut self) {
		let win = self.window.as_mut().expect("failed window creation");

		if self.attributes.transparent && !self.attributes.no_redirection_bitmap {
			let region = unsafe { CreateRectRgn(0, 0, -1, -1) };

			let bb = DWM_BLURBEHIND {
				dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
				fEnable: true.into(),
				hRgnBlur: region,
				fTransitionOnMaximized: false.into(),
			};
			let hr = unsafe { DwmEnableBlurBehindWindow(win.hwnd(), &bb) };
			if hr < 0 {
				tracing::warn!("Setting transparent window is failed. HRESULT Code: 0x{:X}", hr);
			}
			unsafe { DeleteObject(region) };
		}

		win.set_skip_taskbar(self.attributes.skip_taskbar);
		win.set_window_icon(self.attributes.window_icon.clone());
		win.set_taskbar_icon(self.attributes.taskbar_icon.clone());
		win.set_use_system_scroll_speed(self.attributes.use_system_wheel_speed);

		let attributes = self.attributes.clone();

		if attributes.content_protected {
			win.set_content_protected(true);
		}

		win.set_cursor(attributes.cursor);
		win.set_visible(attributes.visible);

		win.set_enabled_buttons(attributes.enabled_buttons);

		let scale_factor = win.scale_factor();
		let size = attributes
			.surface_size
			.unwrap_or(Size::Physical(PhysicalSize::new(800, 600)))
			.to_physical(scale_factor);
		let min_size = attributes.min_surface_size.map(|size| size.to_physical(scale_factor));
		let max_size = attributes.max_surface_size.map(|size| size.to_physical(scale_factor));
		let clamp = |value: u32, min: Option<u32>, max: Option<u32>| {
			let value = min.map_or(value, |min| value.max(min));
			max.map_or(value, |max| value.min(max))
		};
		let clamped_size = PhysicalSize::new(
			clamp(size.width, min_size.map(|s| s.width), max_size.map(|s| s.width)),
			clamp(size.height, min_size.map(|s| s.height), max_size.map(|s| s.height)),
		);
		win.request_surface_size(Size::Physical(clamped_size));
		if let Some(position) = attributes.position {
			win.set_outer_position(position);
		}

		win.set_system_backdrop(self.attributes.backdrop_type);

		if let Some(color) = self.attributes.border_color {
			win.set_border_color(Some(color));
		}
		if let Some(color) = self.attributes.title_background_color {
			win.set_title_background_color(Some(color));
		}
		if let Some(color) = self.attributes.title_text_color {
			win.set_title_text_color(color);
		}
		if let Some(corner) = self.attributes.corner_preference {
			win.set_corner_preference(corner);
		}
	}
}
