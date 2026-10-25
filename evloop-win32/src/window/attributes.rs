use evloop_core::{
	dpi::{Position, Size},
	window::{Theme, WindowButtons, WindowLevel, WindowPositioner},
};
use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU};

use crate::{
	monitor::Fullscreen,
	resource::{cursor::Cursor, icon::Icon},
	window::{
		dwm::{BackdropType, Color, CornerPreference},
		r#type::WindowType,
	},
};

#[derive(Clone, Debug)]
pub struct WindowAttributes {
	pub inner_size: Option<Size>,
	pub min_inner_size: Option<Size>,
	pub max_inner_size: Option<Size>,
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
	pub cursor: Cursor,
	pub(crate) parent_window: Option<RawWindowHandle>,
	pub fullscreen: Option<Fullscreen>,
	pub window_type: WindowType,
	pub positioner: Option<WindowPositioner>,
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
			owner: None,
			menu: None,
			taskbar_icon: None,
			no_redirection_bitmap: false,
			drag_and_drop: true,
			skip_taskbar: false,
			class_name: "Window Class".to_string(),
			decoration_shadow: false,
			backdrop_type: BackdropType::default(),
			clip_children: true,
			border_color: None,
			title_background_color: None,
			title_text_color: None,
			corner_preference: None,
			use_system_wheel_speed: true,
			enabled_buttons: WindowButtons::all(),
			title: String::from("evloop window"),
			decorations: true,
			resizable: true,
			visible: true,
			active: true,
			resize_increments: Default::default(),
			content_protected: Default::default(),
			min_inner_size: Default::default(),
			max_inner_size: Default::default(),
			preferred_theme: Default::default(),
			parent_window: Default::default(),
			inner_size: Default::default(),
			window_level: Default::default(),
			window_icon: Default::default(),
			transparent: Default::default(),
			fullscreen: Default::default(),
			maximized: Default::default(),
			position: Default::default(),
			cursor: Cursor::default(),
			blur: Default::default(),
			window_type: Default::default(),
			positioner: Default::default(),
		}
	}
}

unsafe impl Send for WindowAttributes {}

unsafe impl Sync for WindowAttributes {}

impl WindowAttributes {
	pub fn parent_window(&self) -> Option<&raw_window_handle::RawWindowHandle> {
		self.parent_window.as_ref()
	}

	/// Requests the surface to be of specific dimensions.
	///
	/// If this is not set, some platform-specific dimensions will be used.
	///
	/// See [`Window::request_surface_size`] for details.
	#[inline]
	pub fn with_surface_size<S: Into<Size>>(mut self, size: S) -> Self {
		self.inner_size = Some(size.into());
		self
	}

	/// Sets the minimum dimensions the surface can have.
	///
	/// If this is not set, the surface will have no minimum dimensions (aside from reserved).
	///
	/// See [`Window::set_min_surface_size`] for details.
	#[inline]
	pub fn with_min_surface_size<S: Into<Size>>(mut self, min_size: S) -> Self {
		self.min_inner_size = Some(min_size.into());
		self
	}

	/// Sets the maximum dimensions the surface can have.
	///
	/// If this is not set, the surface will have no maximum, or the maximum will be restricted to
	/// the primary monitor's dimensions by the platform.
	///
	/// See [`Window::set_max_surface_size`] for details.
	#[inline]
	pub fn with_max_surface_size<S: Into<Size>>(mut self, max_size: S) -> Self {
		self.max_inner_size = Some(max_size.into());
		self
	}

	/// Build window with resize increments hint.
	///
	/// The default is `None`.
	///
	/// See [`Window::set_surface_resize_increments`] for details.
	#[inline]
	pub fn with_surface_resize_increments<S: Into<Size>>(
		mut self,
		surface_resize_increments: S,
	) -> Self {
		self.surface_resize_increments = Some(surface_resize_increments.into());
		self
	}

	/// Sets a desired initial position for the window.
	///
	/// If this is not set, some platform-specific position will be chosen.
	///
	/// See [`Window::set_outer_position`] for details.
	///
	/// ## Platform-specific
	///
	/// - **macOS:** The top left corner position of the window content, the window's "inner"
	///   position. The window title bar will be placed above it. The window will be positioned such
	///   that it fits on screen, maintaining set `surface_size` if any. If you need to precisely
	///   position the top left corner of the whole window you have to use
	///   [`Window::set_outer_position`] after creating the window.
	/// - **Windows:** The top left corner position of the window title bar, the window's "outer"
	///   position. There may be a small gap between this position and the window due to the
	///   specifics of the Window Manager.
	/// - **X11:** The top left corner of the window, the window's "outer" position.
	/// - **Wayland:** The top left corner of the window if the window type is `WindowType::Popup`
	///   otherwise ignored
	/// - **Others:** Ignored.
	#[inline]
	pub fn with_position<P: Into<Position>>(mut self, position: P) -> Self {
		self.position = Some(position.into());
		self
	}

	/// Sets whether the window is resizable or not.
	///
	/// The default is `true`.
	///
	/// See [`Window::set_resizable`] for details.
	#[inline]
	pub fn with_resizable(mut self, resizable: bool) -> Self {
		self.resizable = resizable;
		self
	}

	/// Sets the enabled window buttons.
	///
	/// The default is [`WindowButtons::all`]
	///
	/// See [`Window::set_enabled_buttons`] for details.
	#[inline]
	pub fn with_enabled_buttons(mut self, buttons: WindowButtons) -> Self {
		self.enabled_buttons = buttons;
		self
	}

	/// Sets the initial title of the window in the title bar.
	///
	/// The default is `"winit window"`.
	///
	/// See [`Window::set_title`] for details.
	#[inline]
	pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
		self.title = title.into();
		self
	}

	/// Sets whether the window should be put into fullscreen upon creation.
	///
	/// The default is `None`.
	///
	/// See [`Window::set_fullscreen`] for details.
	#[inline]
	pub fn with_fullscreen(mut self, fullscreen: Option<Fullscreen>) -> Self {
		self.fullscreen = fullscreen;
		self
	}

	/// Request that the window is maximized upon creation.
	///
	/// The default is `false`.
	///
	/// See [`Window::set_maximized`] for details.
	#[inline]
	pub fn with_maximized(mut self, maximized: bool) -> Self {
		self.maximized = maximized;
		self
	}

	/// Sets whether the window will be initially visible or hidden.
	///
	/// The default is to show the window.
	///
	/// See [`Window::set_visible`] for details.
	#[inline]
	pub fn with_visible(mut self, visible: bool) -> Self {
		self.visible = visible;
		self
	}

	/// Sets whether the background of the window should be transparent.
	///
	/// If this is `true`, writing colors with alpha values different than
	/// `1.0` will produce a transparent window. On some platforms this
	/// is more of a hint for the system and you'd still have the alpha
	/// buffer. To control it see [`Window::set_transparent`].
	///
	/// The default is `false`.
	#[inline]
	pub fn with_transparent(mut self, transparent: bool) -> Self {
		self.transparent = transparent;
		self
	}

	/// Sets whether the background of the window should be blurred by the system.
	///
	/// The default is `false`.
	///
	/// See [`Window::set_blur`] for details.
	#[inline]
	pub fn with_blur(mut self, blur: bool) -> Self {
		self.blur = blur;
		self
	}

	/// Get whether the window will support transparency.
	#[inline]
	pub fn transparent(&self) -> bool {
		self.transparent
	}

	/// Sets whether the window should have a border, a title bar, etc.
	///
	/// The default is `true`.
	///
	/// See [`Window::set_decorations`] for details.
	#[inline]
	pub fn with_decorations(mut self, decorations: bool) -> Self {
		self.decorations = decorations;
		self
	}

	/// Sets the window level.
	///
	/// This is just a hint to the OS, and the system could ignore it.
	///
	/// The default is [`WindowLevel::Normal`].
	///
	/// See [`WindowLevel`] for details.
	#[inline]
	pub fn with_window_level(mut self, level: WindowLevel) -> Self {
		self.window_level = level;
		self
	}

	/// Sets the window icon.
	///
	/// The default is `None`.
	///
	/// See [`Window::set_window_icon`] for details.
	#[inline]
	pub fn with_window_icon(mut self, window_icon: Option<Icon>) -> Self {
		self.window_icon = window_icon;
		self
	}

	/// Sets a specific theme for the window.
	///
	/// If `None` is provided, the window will use the system theme.
	///
	/// The default is `None`.
	///
	/// ## Platform-specific
	///
	/// - **Wayland:** This controls only CSD. When using `None` it'll try to use dbus to get the
	///   system preference. When explicit theme is used, this will avoid dbus all together.
	/// - **x11:** Build window with `_GTK_THEME_VARIANT` hint set to `dark` or `light`.
	/// - **iOS / Android / Web / x11 / Orbital:** Ignored.
	#[inline]
	pub fn with_theme(mut self, theme: Option<Theme>) -> Self {
		self.preferred_theme = theme;
		self
	}

	/// Prevents the window contents from being captured by other apps.
	///
	/// The default is `false`.
	///
	/// ## Platform-specific
	///
	/// - **macOS**: if `false`, [`NSWindowSharingNone`] is used but doesn't completely prevent all
	///   apps from reading the window content, for instance, QuickTime.
	/// - **iOS / Android / Web / x11 / Orbital:** Ignored.
	///
	/// [`NSWindowSharingNone`]: https://developer.apple.com/documentation/appkit/nswindowsharingtype/nswindowsharingnone
	#[inline]
	pub fn with_content_protected(mut self, protected: bool) -> Self {
		self.content_protected = protected;
		self
	}

	/// Whether the window will be initially focused or not.
	///
	/// The window should be assumed as not focused by default
	/// following by the [`WindowEvent::Focused`].
	///
	/// For [`WindowType::Popup`] windows, also controls keyboard grabbing — see
	/// [`WindowAttributes::active`] for details.
	///
	/// ## Platform-specific:
	///
	/// **Android / iOS / X11 / Orbital:** Unsupported.
	/// **Wayland:** Only supported for [`WindowType::Popup`].
	///
	/// [`WindowEvent::Focused`]: crate::event::WindowEvent::Focused
	#[inline]
	pub fn with_active(mut self, active: bool) -> Self {
		self.active = active;
		self
	}

	/// Modifies the cursor icon of the window.
	///
	/// The default is [`CursorIcon::Default`].
	///
	/// See [`Window::set_cursor()`] for more details.
	#[inline]
	pub fn with_cursor(mut self, cursor: impl Into<Cursor>) -> Self {
		self.cursor = cursor.into();
		self
	}

	/// Build window with parent window.
	///
	/// The default is `None`.
	///
	/// ## Safety
	///
	/// `parent_window` must be a valid window handle.
	///
	/// ## Platform-specific
	///
	/// - **Windows** : A child window has the WS_CHILD style and is confined
	///   to the client area of its parent window. For more information, see
	///   <https://docs.microsoft.com/en-us/windows/win32/winmsg/window-features#child-windows>
	/// - **X11**: A child window is confined to the client area of its parent window.
	/// - **Android / iOS / Wayland / Web:** Unsupported.
	#[inline]
	pub unsafe fn with_parent_window(
		mut self,
		parent_window: Option<raw_window_handle::RawWindowHandle>,
	) -> Self {
		self.parent_window = parent_window;
		self
	}

	/// Sets the [`WindowType`] (window vs. popup).
	///
	/// Used by the Windows, Wayland and macOS backends; on X11 [`WindowType::Popup`] is not
	/// implemented and window creation returns an error.
	/// If the type is [`WindowType::Popup`], the parent must also be set via
	/// [`with_parent_window`](Self::with_parent_window), and the position is interpreted
	/// relative to that parent.
	///
	/// See [`WindowType::Popup`] for the per-platform behavior, including how to obtain a
	/// rounded, native-looking popup on macOS.
	pub fn with_window_type(mut self, window_type: WindowType) -> Self {
		self.window_type = window_type;
		self
	}

	/// Returns if the window type is a popup or a normal window
	#[inline]
	pub fn window_type(&self) -> WindowType {
		self.window_type
	}

	/// Sets the positioner used to place the window relative to its anchor rect.
	///
	/// See [`WindowPositioner`] and its fields for what each part of the positioner controls and
	/// its default when left as `None`.
	///
	/// ## Platform-specific
	///
	/// - **Wayland:** Only takes effect when the window is a [`WindowType::Popup`], since the
	///   Wayland positioner is part of the `xdg_popup` protocol role.
	/// - **macOS, Windows:** Works for both [`WindowType::Window`] and [`WindowType::Popup`]. A
	///   [`WindowType::Popup`] always requires a parent window to be set via
	///   [`with_parent_window`](Self::with_parent_window). A [`WindowType::Window`] without a
	///   parent is positioned relative to the screen's available space instead of the parent's
	///   content area.
	/// - **X11, Web, Android, iOS, Orbital:** No effect.
	#[inline]
	pub fn with_positioner(mut self, positioner: WindowPositioner) -> Self {
		self.positioner = Some(positioner);
		self
	}

	/// Set an owner to the window to be created. Can be used to create a dialog box, for example.
	/// This only works when [`WindowAttributes::with_parent_window`] isn't called or set to `None`.
	/// Can be used in combination with
	/// [`Window::set_enable(false)`][crate::Window::set_enable] on the owner
	/// window to create a modal dialog box.
	///
	/// From MSDN:
	/// - An owned window is always above its owner in the z-order.
	/// - The system automatically destroys an owned window when its owner is destroyed.
	/// - An owned window is hidden when its owner is minimized.
	///
	/// For more information, see <https://docs.microsoft.com/en-us/windows/win32/winmsg/window-features#owned-windows>
	///
	/// [`WindowAttributes::with_parent_window`]: winit_core::window::WindowAttributes::with_parent_window
	pub fn with_owner_window(mut self, parent: HWND) -> Self {
		self.owner = Some(parent);
		self
	}

	/// Sets a menu on the window to be created.
	///
	/// Parent and menu are mutually exclusive; a child window cannot have a menu!
	///
	/// The menu must have been manually created beforehand with [`CreateMenu`] or similar.
	///
	/// Note: Dark mode cannot be supported for win32 menus, it's simply not possible to change how
	/// the menus look. If you use this, it is recommended that you combine it with
	/// `with_theme(Some(Theme::Light))` to avoid a jarring effect.
	///
	/// [`CreateMenu`]: windows_sys::Win32::UI::WindowsAndMessaging::CreateMenu"
	pub fn with_menu(mut self, menu: HMENU) -> Self {
		self.menu = Some(menu);
		self
	}

	/// This sets `ICON_BIG`. A good ceiling here is 256x256.
	pub fn with_taskbar_icon(mut self, taskbar_icon: Option<Icon>) -> Self {
		self.taskbar_icon = taskbar_icon;
		self
	}

	/// This sets `WS_EX_NOREDIRECTIONBITMAP`.
	pub fn with_no_redirection_bitmap(mut self, flag: bool) -> Self {
		self.no_redirection_bitmap = flag;
		self
	}

	/// Enables or disables drag and drop support (enabled by default). Will interfere with other
	/// crates that use multi-threaded COM API (`CoInitializeEx` with `COINIT_MULTITHREADED`
	/// instead of `COINIT_APARTMENTTHREADED`) on the same thread. Note that winit may still
	/// attempt to initialize COM API regardless of this option. Currently only fullscreen mode
	/// does that, but there may be more in the future. If you need COM API with
	/// `COINIT_MULTITHREADED` you must initialize it before calling any winit functions. See <https://docs.microsoft.com/en-us/windows/win32/api/objbase/nf-objbase-coinitialize#remarks> for more information.
	pub fn with_drag_and_drop(mut self, flag: bool) -> Self {
		self.drag_and_drop = flag;
		self
	}

	/// Whether show or hide the window icon in the taskbar.
	pub fn with_skip_taskbar(mut self, skip: bool) -> Self {
		self.skip_taskbar = skip;
		self
	}

	/// Customize the window class name.
	pub fn with_class_name<S: Into<String>>(mut self, class_name: S) -> Self {
		self.class_name = class_name.into();
		self
	}

	/// Shows or hides the background drop shadow for undecorated windows.
	///
	/// The shadow is hidden by default.
	/// Enabling the shadow causes a thin 1px line to appear on the top of the window.
	pub fn with_undecorated_shadow(mut self, shadow: bool) -> Self {
		self.decoration_shadow = shadow;
		self
	}

	/// Sets system-drawn backdrop type.
	///
	/// Requires Windows 11 build 22523+.
	pub fn with_system_backdrop(mut self, backdrop_type: BackdropType) -> Self {
		self.backdrop_type = backdrop_type;
		self
	}

	/// This sets or removes `WS_CLIPCHILDREN` style.
	pub fn with_clip_children(mut self, flag: bool) -> Self {
		self.clip_children = flag;
		self
	}

	/// Sets the color of the window border.
	///
	/// Supported starting with Windows 11 Build 22000.
	pub fn with_border_color(mut self, color: Option<Color>) -> Self {
		self.border_color = Some(color.unwrap_or(Color::NONE));
		self
	}

	/// Sets the background color of the title bar.
	///
	/// Supported starting with Windows 11 Build 22000.
	pub fn with_title_background_color(mut self, color: Option<Color>) -> Self {
		self.title_background_color = Some(color.unwrap_or(Color::NONE));
		self
	}

	/// Sets the color of the window title.
	///
	/// Supported starting with Windows 11 Build 22000.
	pub fn with_title_text_color(mut self, color: Color) -> Self {
		self.title_text_color = Some(color);
		self
	}

	/// Sets the preferred style of the window corners.
	///
	/// Supported starting with Windows 11 Build 22000.
	pub fn with_corner_preference(mut self, corners: CornerPreference) -> Self {
		self.corner_preference = Some(corners);
		self
	}

	/// Sets if the reported [`winit_core::event::WindowEvent::MouseWheel`] event
	/// should account for scroll speed system settings.
	///
	/// The default scroll speed on Windows is 3 lines/characters per scroll,
	/// this will be 1 if you set it to false.
	///
	/// The default is `true`.
	pub fn with_use_system_scroll_speed(mut self, should_use: bool) -> Self {
		self.use_system_wheel_speed = should_use;
		self
	}
}
