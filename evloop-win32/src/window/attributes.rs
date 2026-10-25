use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HMENU};

use crate::resource::icon::Icon;

#[derive(Clone, Debug)]
pub struct WindowAttributes {
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
		}
	}
}

unsafe impl Send for WindowAttributes {}

unsafe impl Sync for WindowAttributes {}

impl WindowAttributes {
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
