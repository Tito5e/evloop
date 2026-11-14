use std::{mem::MaybeUninit, ptr, sync::Arc};

use evloop_core::{dpi::Position, error::RequestError, window::ResizeDirection};
use windows_sys::Win32::{
	Foundation::{LPARAM, POINT, WPARAM},
	Graphics::Gdi::ClientToScreen,
	UI::{
		Input::KeyboardAndMouse::{
			GetKeyboardLayout, MAPVK_VK_TO_VSC, MapVirtualKeyExW, ReleaseCapture, ToUnicode,
			VK_SPACE,
		},
		WindowsAndMessaging::{
			EnableMenuItem, GetCursorPos, GetSystemMenu, GetSystemMetrics, HTBOTTOM, HTBOTTOMLEFT,
			HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, MF_BYCOMMAND,
			MF_DISABLED, MF_ENABLED, PostMessageW, SC_CLOSE, SC_MAXIMIZE, SC_MINIMIZE, SC_MOVE,
			SC_RESTORE, SC_SIZE, SM_MENUDROPALIGNMENT, SetMenuDefaultItem, TPM_LEFTALIGN,
			TPM_RETURNCMD, TPM_RIGHTALIGN, TrackPopupMenu, WM_NCLBUTTONDOWN, WM_SYSCOMMAND,
		},
	},
};

use crate::window::{Window, flags::WindowFlags};

impl Window {
	/// Starts moving the window with the pointer, as if its title bar had been grabbed.
	///
	/// Must be called while the primary mouse button is held down (typically in response to a
	/// `PointerButton` press).
	pub fn drag_window(&self) -> Result<(), RequestError> {
		self.start_os_dragging(HTCAPTION as WPARAM);
		Ok(())
	}

	/// Starts resizing the window with the pointer from the given edge or corner.
	///
	/// Must be called while the primary mouse button is held down.
	pub fn drag_resize_window(&self, direction: ResizeDirection) -> Result<(), RequestError> {
		let hit_test = match direction {
			ResizeDirection::East => HTRIGHT,
			ResizeDirection::North => HTTOP,
			ResizeDirection::NorthEast => HTTOPRIGHT,
			ResizeDirection::NorthWest => HTTOPLEFT,
			ResizeDirection::South => HTBOTTOM,
			ResizeDirection::SouthEast => HTBOTTOMRIGHT,
			ResizeDirection::SouthWest => HTBOTTOMLEFT,
			ResizeDirection::West => HTLEFT,
		};
		self.start_os_dragging(hit_test as WPARAM);
		Ok(())
	}

	fn start_os_dragging(&self, hit_test: WPARAM) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		self.thread_executor.execute_in_thread(move || {
			{
				let mut state = window_state.lock().unwrap();
				if state.dragging {
					return;
				}
				state.dragging = true;
			}

			let mut cursor = POINT { x: 0, y: 0 };
			unsafe { GetCursorPos(&mut cursor) };
			// `WM_NCLBUTTONDOWN` takes the cursor position in screen coordinates (`POINTS`).
			let lparam = ((cursor.y as u16 as u32) << 16 | cursor.x as u16 as u32) as LPARAM;

			unsafe {
				ReleaseCapture();
				PostMessageW(window.hwnd(), WM_NCLBUTTONDOWN, hit_test, lparam);
			}
		});
	}

	/// Shows the system window menu (move, size, minimize, maximize, close) at `position`, in
	/// surface coordinates.
	pub fn show_window_menu(&self, position: Position) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);
		let position = position.to_physical(self.scale_factor());

		self.thread_executor.execute_in_thread(move || unsafe {
			let hwnd = window.hwnd();

			let mut point = POINT { x: position.x, y: position.y };
			if ClientToScreen(hwnd, &mut point) == false.into() {
				return;
			}

			let menu = GetSystemMenu(hwnd, false.into());
			if menu.is_null() {
				return;
			}

			let flags = window_state.lock().unwrap().window_flags();
			let maximized = flags.contains(WindowFlags::MAXIMIZED);
			let enable = |item: u32, enabled: bool| {
				EnableMenuItem(
					menu,
					item,
					MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_DISABLED },
				);
			};

			enable(SC_RESTORE, maximized);
			enable(SC_MOVE, !maximized);
			enable(SC_SIZE, flags.contains(WindowFlags::RESIZABLE) && !maximized);
			enable(SC_MINIMIZE, flags.contains(WindowFlags::MINIMIZABLE));
			enable(SC_MAXIMIZE, flags.contains(WindowFlags::MAXIMIZABLE) && !maximized);
			SetMenuDefaultItem(menu, SC_CLOSE, false.into());

			let align = if GetSystemMetrics(SM_MENUDROPALIGNMENT) != 0 {
				TPM_RIGHTALIGN
			} else {
				TPM_LEFTALIGN
			};

			let command =
				TrackPopupMenu(menu, TPM_RETURNCMD | align, point.x, point.y, 0, hwnd, ptr::null());
			if command != 0 {
				PostMessageW(hwnd, WM_SYSCOMMAND, command as WPARAM, 0);
			}
		});
	}

	/// Discards a pending dead key, so the next key press is not combined with it.
	pub fn reset_dead_keys(&self) {
		self.thread_executor.execute_in_thread(|| unsafe {
			// `ToUnicode` consumes the pending dead key; feed it a harmless key to clear it.
			let vk = VK_SPACE as u32;
			let scancode = MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, GetKeyboardLayout(0));
			let keyboard_state = [0u8; 256];
			let mut buffer = [MaybeUninit::<u16>::uninit(); 8];
			ToUnicode(
				vk,
				scancode,
				keyboard_state.as_ptr(),
				buffer.as_mut_ptr().cast(),
				buffer.len() as i32,
				0,
			);
		});
	}
}
