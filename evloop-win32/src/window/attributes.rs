use evloop_core::{
	dpi::{Position, Size},
	window::{Theme, WindowButtons, WindowLevel, WindowPositioner},
};
use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU};

use crate::{
	monitor::Fullscreen,
	resource::{cursor::SelectedCursor, icon::Icon},
	window::{
		dwm::{BackdropType, Color, CornerPreference},
		r#type::WindowType,
	},
};

#[derive(Clone, Debug)]
pub struct WindowAttributes {
	pub surface_size: Option<Size>,
	pub min_surface_size: Option<Size>,
	pub max_surface_size: Option<Size>,
	pub resize_increments: Option<Size>,
	pub position: Option<Position>,
	pub resizable: bool,

	pub enabled_buttons: WindowButtons,
	pub title: String,
	pub maximized: bool,
	pub visible: bool,
	pub transparent: bool,
	pub blur: bool,
	pub decorations: bool,
	pub window_icon: Option<Icon>,
	pub preferred_theme: Option<Theme>,

	pub content_protected: bool,
	pub window_level: WindowLevel,
	pub active: bool,
	pub cursor: SelectedCursor,
	pub fullscreen: Option<Fullscreen>,
	pub window_type: WindowType,
	pub positioner: Option<WindowPositioner>,

	pub(crate) parent_window: Option<RawWindowHandle>,
	pub(crate) owner: Option<HWND>,
	pub(crate) menu: Option<HMENU>,
	pub(crate) taskbar_icon: Option<Icon>,
	pub(crate) no_redirection_bitmap: bool,
	pub(crate) drag_and_drop: bool,
	pub(crate) skip_taskbar: bool,
	pub(crate) class_name: String,
	pub(crate) decoration_shadow: bool,
	pub(crate) backdrop_type: BackdropType,
	pub(crate) clip_children: bool,
	pub(crate) border_color: Option<Color>,
	pub(crate) title_background_color: Option<Color>,
	pub(crate) title_text_color: Option<Color>,
	pub(crate) corner_preference: Option<CornerPreference>,
	pub(crate) use_system_wheel_speed: bool,
}

impl Default for WindowAttributes {
	fn default() -> Self {
		Self {
			surface_size: None,
			min_surface_size: None,
			max_surface_size: None,
			resize_increments: None,
			position: None,
			resizable: true,

			enabled_buttons: WindowButtons::all(),
			title: String::from("evloop window"),
			maximized: false,
			visible: true,
			transparent: false,
			blur: false,
			decorations: true,
			window_icon: None,
			preferred_theme: None,

			content_protected: false,
			window_level: Default::default(),
			active: true,
			cursor: Default::default(),
			fullscreen: None,
			window_type: Default::default(),
			positioner: None,

			parent_window: None,
			owner: None,
			menu: None,
			taskbar_icon: None,
			no_redirection_bitmap: false,
			drag_and_drop: true,
			skip_taskbar: false,
			class_name: String::from("Window Class"),
			decoration_shadow: false,
			backdrop_type: Default::default(),
			clip_children: true,
			border_color: None,
			title_background_color: None,
			title_text_color: None,
			corner_preference: None,
			use_system_wheel_speed: true,
		}
	}
}

unsafe impl Send for WindowAttributes {}

unsafe impl Sync for WindowAttributes {}

impl WindowAttributes {
	#[inline]
	#[must_use]
	pub fn with_surface_size(mut self, size: impl Into<Size>) -> Self {
		self.surface_size = Some(size.into());
		self
	}

	#[inline]
	#[must_use]
	pub fn with_min_surface_size(mut self, min_surface_size: impl Into<Size>) -> Self {
		self.min_surface_size = Some(min_surface_size.into());
		self
	}

	#[inline]
	#[must_use]
	pub fn with_max_surface_size(mut self, max_surface_size: impl Into<Size>) -> Self {
		self.max_surface_size = Some(max_surface_size.into());
		self
	}

	#[inline]
	#[must_use]
	pub fn with_resize_increments(mut self, resize_increments: impl Into<Size>) -> Self {
		self.resize_increments = Some(resize_increments.into());
		self
	}

	#[inline]
	#[must_use]
	pub fn with_position(mut self, position: impl Into<Position>) -> Self {
		self.position = Some(position.into());
		self
	}

	#[inline]
	#[must_use]
	pub fn with_resizable(mut self, resizable: bool) -> Self {
		self.resizable = resizable;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_enabled_buttons(mut self, buttons: WindowButtons) -> Self {
		self.enabled_buttons = buttons;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
		self.title = title.into();
		self
	}

	#[inline]
	#[must_use]
	pub fn with_maximized(mut self, maximized: bool) -> Self {
		self.maximized = maximized;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_visible(mut self, visible: bool) -> Self {
		self.visible = visible;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_transparent(mut self, transparent: bool) -> Self {
		self.transparent = transparent;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_blur(mut self, blur: bool) -> Self {
		self.blur = blur;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_decorations(mut self, decorations: bool) -> Self {
		self.decorations = decorations;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_window_icon(mut self, window_icon: Option<Icon>) -> Self {
		self.window_icon = window_icon;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_preferred_theme(mut self, preferred_theme: Option<Theme>) -> Self {
		self.preferred_theme = preferred_theme;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_content_protected(mut self, content_protected: bool) -> Self {
		self.content_protected = content_protected;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_window_level(mut self, window_level: WindowLevel) -> Self {
		self.window_level = window_level;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_active(mut self, active: bool) -> Self {
		self.active = active;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_cursor(mut self, cursor: impl Into<SelectedCursor>) -> Self {
		self.cursor = cursor.into();
		self
	}

	#[inline]
	#[must_use]
	pub fn with_fullscreen(mut self, fullscreen: Option<Fullscreen>) -> Self {
		self.fullscreen = fullscreen;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_window_type(mut self, window_type: WindowType) -> Self {
		self.window_type = window_type;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_positioner(mut self, positioner: Option<WindowPositioner>) -> Self {
		self.positioner = positioner;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_parent_window(mut self, parent_window: Option<RawWindowHandle>) -> Self {
		self.parent_window = parent_window;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_owner_window(mut self, owner: Option<HWND>) -> Self {
		self.owner = owner;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_menu(mut self, menu: Option<HMENU>) -> Self {
		self.menu = menu;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_taskbar_icon(mut self, taskbar_icon: Option<Icon>) -> Self {
		self.taskbar_icon = taskbar_icon;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_no_redirection_bitmap(mut self, flag: bool) -> Self {
		self.no_redirection_bitmap = flag;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_drag_and_drop(mut self, flag: bool) -> Self {
		self.drag_and_drop = flag;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_skip_taskbar(mut self, flag: bool) -> Self {
		self.skip_taskbar = flag;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_class_name(mut self, class_name: impl Into<String>) -> Self {
		self.class_name = class_name.into();
		self
	}

	#[inline]
	#[must_use]
	pub fn with_undecorated_shadow(mut self, flag: bool) -> Self {
		self.decoration_shadow = flag;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_backdrop_type(mut self, backdrop_type: BackdropType) -> Self {
		self.backdrop_type = backdrop_type;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_clip_children(mut self, flag: bool) -> Self {
		self.clip_children = flag;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_border_color(mut self, color: Option<Color>) -> Self {
		self.border_color = color;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_title_background_color(mut self, color: Option<Color>) -> Self {
		self.title_background_color = color;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_title_text_color(mut self, color: Option<Color>) -> Self {
		self.title_text_color = color;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_corner_preference(mut self, preference: Option<CornerPreference>) -> Self {
		self.corner_preference = preference;
		self
	}

	#[inline]
	#[must_use]
	pub fn with_use_system_wheel_speed(mut self, flag: bool) -> Self {
		self.use_system_wheel_speed = flag;
		self
	}
}
