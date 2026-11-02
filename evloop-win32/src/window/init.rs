use std::{
	cell::Cell,
	io, mem, panic, ptr,
	rc::Rc,
	sync::{Arc, Mutex},
};

use evloop_core::{
	dpi::Size,
	error::{CreateWindowError, InvalidInput},
	window::WindowLevel,
};
use geometry_types::PhysicalSize;
use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::{
	Foundation::HWND,
	Graphics::{
		Dwm::{DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND, DwmEnableBlurBehindWindow},
		Gdi::{CreateRectRgn, DeleteObject},
	},
	UI::{
		Input::{
			KeyboardAndMouse::{
				INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
				MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VK_LMENU, VK_MENU,
			},
			Touch::{RegisterTouchWindow, TWF_WANTPALM},
		},
		WindowsAndMessaging::{
			CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, GetSystemMetrics, NID_READY,
			RegisterClassExW, SM_DIGITIZER, SetForegroundWindow, WNDCLASSEXW,
		},
	},
};

use crate::{
	event_loop::{EventLoopRunner, WindowUserData, public_window_callback},
	input::{ImeContext, KeyEventBuilder},
	sys::{
		self,
		dpi::{dpi_to_scale_factor, enable_non_client_dpi_scaling},
		dynamic::get_instance_handle,
		wide::encode_wide,
	},
	window::{
		SyncWindowHandle, Window, attributes::WindowAttributes, dark_mode::try_theme,
		flags::WindowFlags, state::WindowState, r#type::WindowType,
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
			window: SyncWindowHandle::new(window),
			window_state,
			thread_executor: self.runner.create_thread_executor(),
		}
	}

	unsafe fn create_window_data(&self, win: &Window) -> WindowUserData {
		WindowUserData {
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

pub(crate) unsafe fn init(
	attributes: WindowAttributes,
	runner: &Rc<EventLoopRunner>,
) -> Result<Window, CreateWindowError> {
	let title = encode_wide(&attributes.title);

	let class_name = encode_wide(&attributes.class_name);
	unsafe { register_window_class(&class_name) };

	let is_popup = match attributes.window_type {
		WindowType::Window => false,
		WindowType::Popup => true,
	};

	let anchored = is_popup || attributes.positioner.is_some();
	let mut window_flags = WindowFlags::empty();

	window_flags.set(WindowFlags::MARKER_DECORATIONS, attributes.decorations);
	window_flags.set(WindowFlags::POPUP, is_popup);
	window_flags.set(WindowFlags::ANCHORED, anchored);
	window_flags.set(WindowFlags::MARKER_UNDECORATED_SHADOW, attributes.decoration_shadow);
	window_flags
		.set(WindowFlags::ALWAYS_ON_TOP, attributes.window_level == WindowLevel::AlwaysOnTop);
	window_flags
		.set(WindowFlags::ALWAYS_ON_BOTTOM, attributes.window_level == WindowLevel::AlwaysOnBottom);
	window_flags.set(WindowFlags::NO_BACK_BUFFER, attributes.no_redirection_bitmap);
	window_flags.set(WindowFlags::MARKER_ACTIVATE, attributes.active);
	window_flags.set(WindowFlags::TRANSPARENT, attributes.transparent);
	window_flags.set(WindowFlags::RESIZABLE, attributes.resizable && !is_popup);
	window_flags.set(WindowFlags::CLOSABLE, true);
	window_flags.set(WindowFlags::CLIP_CHILDREN, attributes.clip_children);

	let mut fallback_parent = || match attributes.owner {
		Some(parent) => {
			window_flags.set(WindowFlags::POPUP, true);
			Some(parent)
		}
		None => {
			window_flags.set(WindowFlags::ON_TASKBAR, true);
			None
		}
	};

	let parent = match attributes.parent_window {
		Some(RawWindowHandle::Win32(handle)) => {
			if !is_popup {
				window_flags.set(WindowFlags::CHILD, true);
			}
			if attributes.menu.is_some() {
				tracing::warn!("Setting a menu on a child window is unsupported");
			}
			Some(handle.hwnd.get() as HWND)
		}
		Some(raw) => unreachable!("Invalid raw window handle {raw:?} on Windows"),
		None => {
			if is_popup {
				return Err(InvalidInput::new("Popup without a parent is not supported").into());
			}
			fallback_parent()
		}
	};

	let menu = attributes.menu;
	let fullscreen = attributes.fullscreen.clone();
	let maximized = attributes.maximized;
	let mut initdata = InitData { runner, attributes, window_flags, window: None };

	let (style, ex_style) = window_flags.to_window_styles();
	let handle = unsafe {
		CreateWindowExW(
			ex_style,
			class_name.as_ptr(),
			title.as_ptr(),
			style,
			CW_USEDEFAULT,
			CW_USEDEFAULT,
			CW_USEDEFAULT,
			CW_USEDEFAULT,
			parent.unwrap_or(ptr::null_mut()),
			menu.unwrap_or(ptr::null_mut()),
			get_instance_handle(),
			&mut initdata as *mut _ as *mut _,
		)
	};

	if let Err(panic_error) = runner.take_panic_error() {
		panic::resume_unwind(panic_error)
	}

	if handle.is_null() {
		return Err(os_error!(io::Error::last_os_error()).into());
	}

	let window = initdata.window.unwrap();

	if fullscreen.is_some() {
		window.set_fullscreen(fullscreen);
		unsafe { force_window_active(window.window.hwnd()) };
	} else if maximized {
		window.set_maximized(true);
	}

	Ok(window)
}

unsafe fn register_window_class(class_name: &[u16]) {
	let class = WNDCLASSEXW {
		cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
		style: CS_HREDRAW | CS_VREDRAW,
		lpfnWndProc: Some(public_window_callback),
		cbClsExtra: 0,
		cbWndExtra: 0,
		hInstance: get_instance_handle(),
		hIcon: ptr::null_mut(),
		hCursor: ptr::null_mut(),
		hbrBackground: ptr::null_mut(),
		lpszMenuName: ptr::null(),
		lpszClassName: class_name.as_ptr(),
		hIconSm: ptr::null_mut(),
	};

	unsafe { RegisterClassExW(&class) };
}

pub unsafe fn force_window_active(handle: HWND) {
	let alt_sc = unsafe { MapVirtualKeyW(VK_MENU as u32, MAPVK_VK_TO_VSC) };

	let inputs = [
		INPUT {
			r#type: INPUT_KEYBOARD,
			Anonymous: INPUT_0 {
				ki: KEYBDINPUT {
					wVk: VK_LMENU,
					wScan: alt_sc as u16,
					dwFlags: KEYEVENTF_EXTENDEDKEY,
					dwExtraInfo: 0,
					time: 0,
				},
			},
		},
		INPUT {
			r#type: INPUT_KEYBOARD,
			Anonymous: INPUT_0 {
				ki: KEYBDINPUT {
					wVk: VK_LMENU,
					wScan: alt_sc as u16,
					dwFlags: KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP,
					time: 0,
					dwExtraInfo: 0,
				},
			},
		},
	];

	unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), mem::size_of::<INPUT>() as i32) };

	unsafe { SetForegroundWindow(handle) };
}
