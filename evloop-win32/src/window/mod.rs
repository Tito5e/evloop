pub(super) mod attributes;
pub(super) mod dwm;
pub(super) mod flags;
pub(super) mod fullscreen;
pub(super) mod placement;
pub(super) mod state;
pub(super) mod taskbar;
pub(super) mod r#type;

use std::{
	mem,
	sync::{Arc, Mutex, MutexGuard},
};

use evloop_core::{
	dpi::{Position, Size},
	error::CreateWindowError,
};
use geometry_types::PhysicalPosition;
use windows_sys::Win32::{
	Foundation::{HWND, LPARAM, POINT, POINTS, WPARAM},
	Graphics::{
		Dwm::{
			DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE, DWMWA_BORDER_COLOR,
			DWMWA_CAPTION_COLOR, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_TEXT_COLOR,
			DWMWA_WINDOW_CORNER_PREFERENCE, DwmSetWindowAttribute,
		},
		Gdi::{ClientToScreen, ScreenToClient},
	},
	UI::{
		Input::KeyboardAndMouse::{EnableWindow, ReleaseCapture},
		WindowsAndMessaging::{
			EnableMenuItem, GWLP_HINSTANCE, GetCursorPos, GetParent, GetSystemMenu,
			MENU_ITEM_STATE, MF_BYCOMMAND, MFS_DISABLED, MFS_ENABLED, PostMessageW, SC_CLOSE,
			SC_MAXIMIZE, SC_MINIMIZE, SC_MOVE, SC_RESTORE, SC_SIZE, SendMessageW,
			SetMenuDefaultItem, TPM_LEFTALIGN, TPM_RETURNCMD, TrackPopupMenu, WM_NCLBUTTONDOWN,
			WM_SETICON, WM_SYSCOMMAND,
		},
	},
};

use crate::{
	event_loop::ActiveEventLoop,
	monitor::Fullscreen,
	resource::icon::{Icon, IconType},
	sys::message_id::DESTROY_MSG_ID,
	window::{
		attributes::WindowAttributes,
		dwm::{BackdropType, Color, CornerPreference},
		flags::WindowFlags,
		state::WindowState,
		taskbar::set_skip_taskbar,
	},
};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct SyncWindowHandle(HWND);

unsafe impl Send for SyncWindowHandle {}

unsafe impl Sync for SyncWindowHandle {}

impl SyncWindowHandle {
	fn hwnd(&self) -> HWND {
		self.0
	}
}

#[derive(Debug)]
pub struct Window {
	window: SyncWindowHandle,
	window_state: Arc<Mutex<WindowState>>,

	thread_executor: EventLoopThreadExecutor,
}

impl Window {
	pub(crate) fn new(
		event_loop: &ActiveEventLoop,
		w_attr: WindowAttributes,
	) -> Result<Window, CreateWindowError> {
		let window = unsafe { init(w_attr, &event_loop.0) }?;
		window.reposition();
		Ok(window)
	}

	fn window_state_lock(&self) -> MutexGuard<'_, WindowState> {
		self.window_state.lock().unwrap()
	}

	fn translate_outer_position(&self, position: Position) -> PhysicalPosition {
		translate_outer_position(self.hwnd(), &self.window_state, position, self.scale_factor())
	}

	fn translate_outer_position_to_parent(&self, position: PhysicalPosition) -> PhysicalPosition {
		let mut point = POINT { x: position.x, y: position.y };

		let window_flags = self.window_state_lock().window_flags;
		if window_flags.contains(WindowFlags::ANCHORED)
			&& !window_flags.contains(WindowFlags::CHILD)
		{
			let parent = unsafe { GetParent(self.hwnd()) };
			if !parent.is_null() {
				unsafe {
					ScreenToClient(parent, &mut point);
				}
			}
		}

		PhysicalPosition::new(point.x, point.y)
	}

	fn reposition(&self) {
		let (anchored, positioner) = {
			let window_state = self.window_state_lock();
			(window_state.anchored, window_state.positioner)
		};
		let Some((origin, size, current_size)) =
			compute_anchored_placement(self.hwnd(), anchored, &positioner, self.scale_factor())
		else {
			return;
		};

		self.set_outer_position(Position::Logical(origin));
		if size != current_size {
			let _ = self.request_surface_size(Size::Logical(size));
		}
	}

	pub fn hwnd(&self) -> HWND {
		self.window.hwnd()
	}

	pub(crate) unsafe fn rwh_06_no_thread_check(
		&self,
	) -> Result<raw_window_handle::RawWindowHandle, raw_window_handle::HandleError> {
		let mut window_handle = raw_window_handle::Win32WindowHandle::new(unsafe {
			// SAFETY: Handle will never be zero.
			std::num::NonZeroIsize::new_unchecked(self.window.hwnd() as isize)
		});
		let hinstance = unsafe { get_window_long(self.hwnd(), GWLP_HINSTANCE) };
		window_handle.hinstance = std::num::NonZeroIsize::new(hinstance);
		Ok(raw_window_handle::RawWindowHandle::Win32(window_handle))
	}

	pub fn raw_window_handle_rwh_06(
		&self,
	) -> Result<raw_window_handle::RawWindowHandle, raw_window_handle::HandleError> {
		// TODO: Write a test once integration framework is ready to ensure that it holds.
		// If we aren't in the GUI thread, we can't return the window.
		if !self.thread_executor.in_event_loop_thread() {
			tracing::error!("tried to access window handle outside of the main thread");
			return Err(raw_window_handle::HandleError::Unavailable);
		}

		// SAFETY: We are on the correct thread.
		unsafe { self.rwh_06_no_thread_check() }
	}

	pub fn raw_display_handle_rwh_06(
		&self,
	) -> Result<raw_window_handle::RawDisplayHandle, raw_window_handle::HandleError> {
		Ok(raw_window_handle::RawDisplayHandle::Windows(
			raw_window_handle::WindowsDisplayHandle::new(),
		))
	}

	/// For more information, see <https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enablewindow#remarks>
	/// and <https://docs.microsoft.com/en-us/windows/win32/winmsg/window-features#disabled-windows>
	pub fn set_enable(&self, enabled: bool) {
		unsafe { EnableWindow(self.hwnd(), enabled.into()) };
	}

	pub fn set_skip_taskbar(&self, skip: bool) {
		self.window_state_lock().skip_taskbar = skip;
		unsafe { set_skip_taskbar(self.hwnd(), skip) };
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

	pub fn set_system_backdrop(&self, backdrop_type: BackdropType) {
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_SYSTEMBACKDROP_TYPE as u32,
				&(backdrop_type as i32) as *const _ as _,
				mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as _,
			);
		}
	}

	pub fn set_taskbar_icon(&self, taskbar_icon: Option<Icon>) {
		if let Some(taskbar_icon) = taskbar_icon {
			self.set_icon(taskbar_icon, IconType::Big);
		} else {
			self.unset_icon(IconType::Big);
		}
	}

	unsafe fn handle_os_dragging(&self, wparam: WPARAM) {
		let window = self.window;
		let window_state = self.window_state.clone();

		self.thread_executor.execute_in_thread(move || {
			{
				let mut guard = window_state.lock().unwrap();
				if !guard.dragging {
					guard.dragging = true;
				} else {
					return;
				}
			}

			let points = {
				let mut pos = unsafe { mem::zeroed() };
				unsafe { GetCursorPos(&mut pos) };
				pos
			};
			let points = POINTS { x: points.x as i16, y: points.y as i16 };

			// ReleaseCapture needs to execute on the main thread
			unsafe { ReleaseCapture() };

			unsafe {
				PostMessageW(window.hwnd(), WM_NCLBUTTONDOWN, wparam, &points as *const _ as LPARAM)
			};
		});
	}

	unsafe fn handle_showing_window_menu(&self, position: Position) {
		unsafe {
			let point = {
				let mut point = POINT { x: 0, y: 0 };
				let scale_factor = self.scale_factor();
				let position = position.to_physical(scale_factor);
				point.x = position.x;
				point.y = position.y;
				if ClientToScreen(self.hwnd(), &mut point) == false.into() {
					tracing::warn!(
						"Can't convert client-area coordinates to screen coordinates when showing \
                         window menu."
					);
					return;
				}
				point
			};

			let h_menu = GetSystemMenu(self.hwnd(), 0);
			if h_menu.is_null() {
				tracing::warn!("The corresponding window doesn't have a system menu");
				return;
			}

			fn enable(b: bool) -> MENU_ITEM_STATE {
				if b { MFS_ENABLED } else { MFS_DISABLED }
			}

			let restore_btn = enable(self.is_maximized() && self.is_resizable());
			let size_btn = enable(!self.is_maximized() && self.is_resizable());
			let maximize_btn = enable(!self.is_maximized() && self.is_resizable());

			EnableMenuItem(h_menu, SC_RESTORE, MF_BYCOMMAND | restore_btn);
			EnableMenuItem(h_menu, SC_MOVE, MF_BYCOMMAND | enable(!self.is_maximized()));
			EnableMenuItem(h_menu, SC_SIZE, MF_BYCOMMAND | size_btn);
			EnableMenuItem(h_menu, SC_MINIMIZE, MF_BYCOMMAND | MFS_ENABLED);
			EnableMenuItem(h_menu, SC_MAXIMIZE, MF_BYCOMMAND | maximize_btn);
			EnableMenuItem(h_menu, SC_CLOSE, MF_BYCOMMAND | MFS_ENABLED);

			// Set the default menu item.
			SetMenuDefaultItem(h_menu, SC_CLOSE, 0);

			// Popup the system menu at the position.
			let result = TrackPopupMenu(
				h_menu,
				TPM_RETURNCMD | TPM_LEFTALIGN,
				point.x,
				point.y,
				0,
				self.hwnd(),
				std::ptr::null_mut(),
			);

			if result == 0 {
				// User canceled the menu, no need to continue.
				return;
			}

			// Send the command that the user select to the corresponding window.
			if PostMessageW(self.hwnd(), WM_SYSCOMMAND, result as _, 0) == 0 {
				tracing::warn!("Can't post the system menu message to the window.");
			}
		}
	}

	#[inline]
	pub fn set_border_color(&self, color: Option<Color>) {
		let color = color.unwrap_or(Color::NONE);
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_BORDER_COLOR as u32,
				&color as *const _ as _,
				mem::size_of::<Color>() as _,
			);
		}
	}

	#[inline]
	pub fn set_title_background_color(&self, color: Option<Color>) {
		// The windows docs don't mention NONE as a valid options but it works in practice and is
		// useful to circumvent the Windows option "Show accent color on title bars and
		// window borders"
		let color = color.unwrap_or(Color::NONE);
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_CAPTION_COLOR as u32,
				&color as *const _ as _,
				mem::size_of::<Color>() as _,
			);
		}
	}

	#[inline]
	pub fn set_title_text_color(&self, color: Color) {
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_TEXT_COLOR as u32,
				&color as *const _ as _,
				mem::size_of::<Color>() as _,
			);
		}
	}

	#[inline]
	pub fn set_corner_preference(&self, preference: CornerPreference) {
		unsafe {
			DwmSetWindowAttribute(
				self.hwnd(),
				DWMWA_WINDOW_CORNER_PREFERENCE as u32,
				&(preference as DWM_WINDOW_CORNER_PREFERENCE) as *const _ as _,
				mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as _,
			);
		}
	}

	#[inline]
	pub fn set_use_system_scroll_speed(&self, should_use: bool) {
		self.window_state_lock().use_system_wheel_speed = should_use;
	}

	fn set_icon(&self, new_icon: Icon, icon_type: IconType) {
		unsafe {
			SendMessageW(
				self.hwnd(),
				WM_SETICON,
				icon_type as usize,
				new_icon.as_raw_handle() as isize,
			);
		}

		match icon_type {
			IconType::Small => self.window_state_lock().window_icon = Some(new_icon),
			IconType::Big => self.window_state_lock().taskbar_icon = Some(new_icon),
		}
	}

	fn unset_icon(&self, icon_type: IconType) {
		unsafe {
			SendMessageW(self.hwnd(), WM_SETICON, icon_type as usize, 0);
		}
		match icon_type {
			IconType::Small => self.window_state_lock().window_icon = None,
			IconType::Big => self.window_state_lock().taskbar_icon = None,
		}
	}

	pub unsafe fn window_handle_any_thread(
		&self,
	) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
		unsafe {
			let handle = self.rwh_06_no_thread_check()?;

			// SAFETY: The handle is valid in this context.
			Ok(raw_window_handle::WindowHandle::borrow_raw(handle))
		}
	}
}

impl Drop for Window {
	fn drop(&mut self) {
		if matches!(self.fullscreen(), Some(Fullscreen::Exclusive(_, _))) {
			self.set_fullscreen(None);
		}

		unsafe {
			PostMessageW(self.hwnd(), DESTROY_MSG_ID.get(), 0, 0);
		}
	}
}

impl raw_window_handle::HasDisplayHandle for Window {
	fn display_handle(
		&self,
	) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
		let raw = self.raw_display_handle_rwh_06()?;
		unsafe { Ok(raw_window_handle::DisplayHandle::borrow_raw(raw)) }
	}
}

impl raw_window_handle::HasWindowHandle for Window {
	fn window_handle(
		&self,
	) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
		let raw = self.raw_window_handle_rwh_06()?;
		unsafe { Ok(raw_window_handle::WindowHandle::borrow_raw(raw)) }
	}
}
