use core::fmt;
use std::{io, sync::MutexGuard};

use bitflags::bitflags;
use evloop_core::{
	dpi::Size,
	ime::ImeCapabilities,
	window::{Theme, WindowPositioner},
};
use geometry_types::{PhysicalPosition, PhysicalSize};
use windows_sys::Win32::{
	Foundation::{HWND, RECT},
	UI::WindowsAndMessaging::WINDOWPLACEMENT,
};

use crate::{
	monitor::Fullscreen,
	resource::{cursor::SelectedCursor, icon::Icon},
	sys::{
		cursor_control::{
			get_cursor_clip, get_cursor_position, set_cursor_clip, set_cursor_hidden,
		},
		window_query::{get_desktop_rect, is_focused},
	},
	window::{
		area::WindowArea, attributes::WindowAttributes, flags::WindowFlags, r#type::WindowType,
	},
};

#[derive(Debug)]
pub(crate) struct WindowState {
	pub mouse: MouseProperties,

	pub min_size: Option<Size>,
	pub max_size: Option<Size>,

	pub surface_size: PhysicalSize,

	pub surface_resize_increments: Option<Size>,

	pub window_icon: Option<Icon>,
	pub taskbar_icon: Option<Icon>,

	pub saved_window: Option<SavedWindow>,
	pub scale_factor: f64,

	pub modifiers_state: ModifiersState,
	pub fullscreen: Option<Fullscreen>,
	pub current_theme: Theme,
	pub preferred_theme: Option<Theme>,

	pub window_flags: WindowFlags,

	pub window_type: WindowType,

	pub anchored: bool,

	pub positioner: WindowPositioner,

	pub ime_state: ImeState,
	pub ime_capabilities: Option<ImeCapabilities>,

	pub is_active: bool,
	pub is_focused: bool,

	pub redraw_requested: bool,

	pub dragging: bool,

	pub skip_taskbar: bool,

	pub use_system_wheel_speed: bool,
}

impl WindowState {
	pub(crate) fn new(
		attributes: &WindowAttributes,
		scale_factor: f64,
		current_theme: Theme,
		preferred_theme: Option<Theme>,
	) -> WindowState {
		WindowState {
			mouse: MouseProperties {
				selected_cursor: SelectedCursor::default(),
				capture_count: 0,
				cursor_flags: CursorFlags::empty(),
				last_position: None,
			},

			min_size: attributes.min_inner_size,
			max_size: attributes.max_inner_size,

			surface_size: PhysicalSize::new(0, 0),

			surface_resize_increments: attributes.resize_increments,

			window_icon: attributes.window_icon.clone(),
			taskbar_icon: None,

			saved_window: None,
			scale_factor,

			modifiers_state: ModifiersState::default(),
			fullscreen: None,
			current_theme,
			preferred_theme,
			window_flags: WindowFlags::empty(),

			window_type: attributes.window_type,
			anchored: matches!(attributes.window_type, WindowType::Popup)
				|| attributes.positioner.is_some(),
			positioner: attributes.positioner.unwrap_or_default(),

			ime_state: ImeState::Disabled,
			ime_capabilities: None,

			is_active: false,
			is_focused: false,
			redraw_requested: false,

			dragging: false,

			skip_taskbar: false,

			use_system_wheel_speed: true,
		}
	}

	pub fn window_flags(&self) -> WindowFlags {
		self.window_flags
	}

	pub fn set_window_flags<F>(mut this: MutexGuard<'_, Self>, window: HWND, f: F)
	where
		F: FnOnce(&mut WindowFlags),
	{
		let old_flags = this.window_flags;
		f(&mut this.window_flags);
		let new_flags = this.window_flags;

		drop(this);
		old_flags.apply_diff(window, new_flags);
	}

	pub fn set_window_flags_in_place<F>(&mut self, f: F)
	where
		F: FnOnce(&mut WindowFlags),
	{
		f(&mut self.window_flags);
	}

	pub fn has_active_focus(&self) -> bool {
		self.is_active && self.is_focused
	}

	// Updates is_active and returns whether active-focus state has changed
	pub fn set_active(&mut self, is_active: bool) -> bool {
		let old = self.has_active_focus();
		self.is_active = is_active;
		old != self.has_active_focus()
	}

	// Updates is_focused and returns whether active-focus state has changed
	pub fn set_focused(&mut self, is_focused: bool) -> bool {
		let old = self.has_active_focus();
		self.is_focused = is_focused;
		old != self.has_active_focus()
	}
}

#[derive(Clone)]
pub struct SavedWindow {
	pub placement: WINDOWPLACEMENT,
}

impl fmt::Debug for SavedWindow {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SavedWindow").finish_non_exhaustive()
	}
}

#[derive(Clone, Debug)]
pub struct MouseProperties {
	pub(crate) selected_cursor: SelectedCursor,
	pub capture_count: u32,
	cursor_flags: CursorFlags,
	pub last_position: Option<PhysicalPosition>,
}

impl MouseProperties {
	pub fn cursor_flags(&self) -> CursorFlags {
		self.cursor_flags
	}

	pub fn set_cursor_flags<F>(&mut self, window: HWND, f: F) -> Result<(), io::Error>
	where
		F: FnOnce(&mut CursorFlags),
	{
		let old_flags = self.cursor_flags;
		f(&mut self.cursor_flags);
		match self.cursor_flags.refresh_os_cursor(window) {
			Ok(()) => (),
			Err(e) => {
				self.cursor_flags = old_flags;
				return Err(e);
			}
		}

		Ok(())
	}
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum ImeState {
	Disabled,
	Enabled,
	Preedit,
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct CursorFlags: u8 {
		const GRABBED   = 1 << 0;
		const HIDDEN    = 1 << 1;
		const IN_WINDOW = 1 << 2;
		const LOCKED    = 1 << 3;
	}
}

impl CursorFlags {
	fn refresh_os_cursor(self, window: HWND) -> Result<(), io::Error> {
		let client_rect = WindowArea::Inner.get_rect(window)?;

		if is_focused(window) {
			let cursor_clip = match self.contains(CursorFlags::GRABBED) {
				true => {
					if self.contains(CursorFlags::LOCKED) {
						if let Ok(pos) = get_cursor_position() {
							Some(RECT {
								left: pos.x,
								right: pos.x + 1,
								top: pos.y,
								bottom: pos.y + 1,
							})
						} else {
							let cx = (client_rect.left + client_rect.right) / 2;
							let cy = (client_rect.top + client_rect.bottom) / 2;
							Some(RECT { left: cx, right: cx + 1, top: cy, bottom: cy + 1 })
						}
					} else {
						Some(client_rect)
					}
				}
				false => None,
			};

			let rect_to_tuple = |rect: RECT| (rect.left, rect.top, rect.right, rect.bottom);
			let active_cursor_clip = rect_to_tuple(get_cursor_clip()?);
			let desktop_rect = rect_to_tuple(get_desktop_rect());

			let active_cursor_clip = match desktop_rect == active_cursor_clip {
				true => None,
				false => Some(active_cursor_clip),
			};

			if active_cursor_clip != cursor_clip.map(rect_to_tuple) {
				set_cursor_clip(cursor_clip)?;
			}
		}

		let cursor_in_client = self.contains(CursorFlags::IN_WINDOW);
		if cursor_in_client {
			set_cursor_hidden(self.contains(CursorFlags::HIDDEN));
		} else {
			set_cursor_hidden(false);
		}

		Ok(())
	}
}
