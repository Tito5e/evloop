use std::{io, ptr};

use bitflags::bitflags;
use geometry_types::PhysicalSize;
use windows_sys::Win32::{
	Foundation::{HWND, RECT},
	Graphics::Gdi::InvalidateRgn,
	UI::WindowsAndMessaging::{
		AdjustWindowRectEx, EnableMenuItem, GWL_EXSTYLE, GWL_STYLE, GetMenu, GetSystemMenu,
		GetWindowLongW, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, MF_BYCOMMAND, MF_DISABLED,
		MF_ENABLED, SC_CLOSE, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
		SW_SHOWNOACTIVATE, SWP_ASYNCWINDOWPOS, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
		SWP_NOREPOSITION, SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetWindowLongW, SetWindowPos,
		ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_BORDER, WS_CAPTION, WS_CHILD,
		WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_ACCEPTFILES, WS_EX_APPWINDOW, WS_EX_LAYERED,
		WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
		WS_EX_WINDOWEDGE, WS_MAXIMIZE, WS_MAXIMIZEBOX, WS_MINIMIZE, WS_MINIMIZEBOX,
		WS_OVERLAPPEDWINDOW, WS_POPUP, WS_SIZEBOX, WS_SYSMENU, WS_VISIBLE,
	},
};

use crate::sys::{
	dynamic::{ADJUST_WINDOW_RECT_EX_FOR_DPI, GET_DPI_FOR_WINDOW},
	error::win_to_err,
	message_id::SET_RETAIN_STATE_ON_SIZE_MSG_ID,
};

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct WindowFlags: u32 {
		const RESIZABLE         = 1 << 0;
		const MINIMIZABLE       = 1 << 1;
		const MAXIMIZABLE       = 1 << 2;
		const CLOSABLE          = 1 << 3;
		const VISIBLE           = 1 << 4;
		const ON_TASKBAR        = 1 << 5;
		const ALWAYS_ON_TOP     = 1 << 6;
		const ALWAYS_ON_BOTTOM  = 1 << 7;
		const NO_BACK_BUFFER    = 1 << 8;
		const TRANSPARENT       = 1 << 9;
		const CHILD             = 1 << 10;
		const MAXIMIZED         = 1 << 11;
		const POPUP             = 1 << 12;

		/// Marker flag for fullscreen. Should always match `WindowState::fullscreen`, but is
		/// included here to make masking easier.
		const MARKER_EXCLUSIVE_FULLSCREEN = 1 << 13;
		const MARKER_BORDERLESS_FULLSCREEN = 1 << 14;

		/// The `WM_SIZE` event contains some parameters that can effect the state of `WindowFlags`.
		/// In most cases, it's okay to let those parameters change the state. However, when we're
		/// running the `WindowFlags::apply_diff` function, we *don't* want those parameters to
		/// effect our stored state, because the purpose of `apply_diff` is to update the actual
		/// window's state to match our stored state. This controls whether to accept those changes.
		const MARKER_RETAIN_STATE_ON_SIZE = 1 << 15;

		const MARKER_IN_SIZE_MOVE = 1 << 16;

		const MINIMIZED = 1 << 17;

		const IGNORE_CURSOR_EVENT = 1 << 18;

		/// Fully decorated window (incl. caption, border and drop shadow).
		const MARKER_DECORATIONS = 1 << 19;
		/// Drop shadow for undecorated windows.
		const MARKER_UNDECORATED_SHADOW = 1 << 20;

		const MARKER_ACTIVATE = 1 << 21;

		const CLIP_CHILDREN = 1 << 22;

		/// Whether this window is positioned relative to its parent via the anchor/gravity/
		/// positioner system. Independent of `POPUP`, which only selects the OS window style --
		/// a `WindowType::Window` can be `ANCHORED` too. Used to pick the coordinate frame in
		/// `translate_outer_position`/`translate_outer_position_to_parent`.
		const ANCHORED = 1 << 23;

		const EXCLUSIVE_FULLSCREEN_OR_MASK = WindowFlags::ALWAYS_ON_TOP.bits();
	}
}

impl WindowFlags {
	fn mask(mut self) -> WindowFlags {
		if self.contains(WindowFlags::MARKER_EXCLUSIVE_FULLSCREEN) {
			self |= WindowFlags::EXCLUSIVE_FULLSCREEN_OR_MASK;
		}
		self
	}

	pub fn to_window_styles(self) -> (WINDOW_STYLE, WINDOW_EX_STYLE) {
		// Required styles to properly support common window functionality like aero snap.
		let mut style = WS_CLIPSIBLINGS;
		let mut style_ex = WS_EX_WINDOWEDGE | WS_EX_ACCEPTFILES;
		if self.contains(WindowFlags::POPUP) {
			style |= WS_POPUP;
			// Don't activate the popup (and thus don't deactivate the parent) when it is shown or
			// clicked, unless the caller requested activation via `WindowAttributes::active`.
			if !self.contains(WindowFlags::MARKER_ACTIVATE) {
				style_ex |= WS_EX_NOACTIVATE;
			}
		} else {
			style |= WS_CAPTION | WS_SYSMENU | WS_BORDER;

			if self.contains(WindowFlags::RESIZABLE) {
				style |= WS_SIZEBOX;
			}
			if self.contains(WindowFlags::MAXIMIZABLE) {
				style |= WS_MAXIMIZEBOX;
			}
			if self.contains(WindowFlags::MINIMIZABLE) {
				style |= WS_MINIMIZEBOX;
			}

			if self.contains(WindowFlags::ON_TASKBAR) {
				style_ex |= WS_EX_APPWINDOW;
			}
		};

		if self.contains(WindowFlags::VISIBLE) {
			style |= WS_VISIBLE;
		}
		if self.contains(WindowFlags::NO_BACK_BUFFER) {
			style_ex |= WS_EX_NOREDIRECTIONBITMAP;
		}
		if self.contains(WindowFlags::CHILD) && !self.contains(WindowFlags::POPUP) {
			style |= WS_CHILD;

			// Remove decorations window styles for child
			if !self.contains(WindowFlags::MARKER_DECORATIONS) {
				style &= !(WS_CAPTION | WS_BORDER);
				style_ex &= !WS_EX_WINDOWEDGE;
			}
		}
		if self.contains(WindowFlags::MINIMIZED) {
			style |= WS_MINIMIZE;
		}
		if self.contains(WindowFlags::MAXIMIZED) {
			style |= WS_MAXIMIZE;
		}
		if self.contains(WindowFlags::IGNORE_CURSOR_EVENT) {
			style_ex |= WS_EX_TRANSPARENT | WS_EX_LAYERED;
		}
		if self.contains(WindowFlags::CLIP_CHILDREN) {
			style |= WS_CLIPCHILDREN;
		}

		if self.intersects(
			WindowFlags::MARKER_EXCLUSIVE_FULLSCREEN | WindowFlags::MARKER_BORDERLESS_FULLSCREEN,
		) {
			style &= !WS_OVERLAPPEDWINDOW;
		}

		if self.contains(WindowFlags::ALWAYS_ON_TOP) {
			style_ex |= WS_EX_TOPMOST;

			// A window with WS_EX_TOPMOST set must have one of the following style flags:
			// WS_POPUP, WS_BORDER, WS_CAPTION, WS_SYSMENU, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
			// WS_THICKFRAME, WS_EX_CLIENTEDGE, WS_EX_CONTEXTHELP, WS_EX_DLGMODALFRAME,
			// WS_EX_TOOLWINDOW, WS_EX_WINDOWEDGE.
			// If none of them are set, it will cause a DXGI error when creating a swapchain.
			if style & WS_CAPTION == 0 {
				style |= WS_POPUP;
			}
		}

		(style, style_ex)
	}

	/// Adjust the window client rectangle to the return value, if present.
	pub(crate) fn apply_diff(mut self, window: HWND, mut new: WindowFlags) {
		self = self.mask();
		new = new.mask();

		let mut diff = self ^ new;

		if diff == WindowFlags::empty() {
			return;
		}

		if new.contains(WindowFlags::VISIBLE) {
			let flag = if !self.contains(WindowFlags::MARKER_ACTIVATE) {
				self.set(WindowFlags::MARKER_ACTIVATE, true);
				SW_SHOWNOACTIVATE
			} else {
				SW_SHOW
			};
			unsafe {
				ShowWindow(window, flag);
			}
		}

		if diff.intersects(WindowFlags::ALWAYS_ON_TOP | WindowFlags::ALWAYS_ON_BOTTOM) {
			unsafe {
				SetWindowPos(
					window,
					match (
						new.contains(WindowFlags::ALWAYS_ON_TOP),
						new.contains(WindowFlags::ALWAYS_ON_BOTTOM),
					) {
						(true, false) => HWND_TOPMOST,
						(false, false) => HWND_NOTOPMOST,
						(false, true) => HWND_BOTTOM,
						(true, true) => unreachable!(),
					},
					0,
					0,
					0,
					0,
					SWP_ASYNCWINDOWPOS | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
				);
				InvalidateRgn(window, ptr::null_mut(), false.into());
			}
		}

		if diff.contains(WindowFlags::MAXIMIZED) || new.contains(WindowFlags::MAXIMIZED) {
			unsafe {
				ShowWindow(
					window,
					match new.contains(WindowFlags::MAXIMIZED) {
						true => SW_MAXIMIZE,
						false => SW_RESTORE,
					},
				);
			}
		}

		// Minimize operations should execute after maximize for proper window animations
		if diff.contains(WindowFlags::MINIMIZED) {
			unsafe {
				ShowWindow(
					window,
					match new.contains(WindowFlags::MINIMIZED) {
						true => SW_MINIMIZE,
						false => SW_RESTORE,
					},
				);
			}

			diff.remove(WindowFlags::MINIMIZED);
		}

		if diff.contains(WindowFlags::CLOSABLE) || new.contains(WindowFlags::CLOSABLE) {
			let flags = MF_BYCOMMAND
				| if new.contains(WindowFlags::CLOSABLE) { MF_ENABLED } else { MF_DISABLED };

			unsafe {
				EnableMenuItem(GetSystemMenu(window, 0), SC_CLOSE, flags);
			}
		}

		if !new.contains(WindowFlags::VISIBLE) {
			unsafe {
				ShowWindow(window, SW_HIDE);
			}
		}

		if diff != WindowFlags::empty() {
			let (style, style_ex) = new.to_window_styles();

			unsafe {
				SendMessageW(window, SET_RETAIN_STATE_ON_SIZE_MSG_ID.get(), 1, 0);

				// This condition is necessary to avoid having an unrestorable window
				if !new.contains(WindowFlags::MINIMIZED) {
					SetWindowLongW(window, GWL_STYLE, style as i32);
					SetWindowLongW(window, GWL_EXSTYLE, style_ex as i32);
				}

				let mut flags = SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED;

				// We generally don't want style changes here to affect window
				// focus, but for fullscreen windows they must be activated
				// (i.e. focused) so that they appear on top of the taskbar
				if !new.contains(WindowFlags::MARKER_EXCLUSIVE_FULLSCREEN)
					&& !new.contains(WindowFlags::MARKER_BORDERLESS_FULLSCREEN)
				{
					flags |= SWP_NOACTIVATE;
				}

				// Refresh the window frame
				SetWindowPos(window, ptr::null_mut(), 0, 0, 0, 0, flags);
				SendMessageW(window, SET_RETAIN_STATE_ON_SIZE_MSG_ID.get(), 0, 0);
			}
		}
	}

	pub fn adjust_rect(self, hwnd: HWND, mut rect: RECT) -> Result<RECT, io::Error> {
		unsafe {
			let mut style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
			let style_ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

			// Frameless style implemented by manually overriding the non-client area in
			// `WM_NCCALCSIZE`.
			if !self.contains(WindowFlags::MARKER_DECORATIONS) {
				style &= !(WS_CAPTION | WS_SIZEBOX);
			}

			win_to_err({
				let b_menu = !GetMenu(hwnd).is_null();
				if let (Some(get_dpi_for_window), Some(adjust_window_rect_ex_for_dpi)) =
					(*GET_DPI_FOR_WINDOW, *ADJUST_WINDOW_RECT_EX_FOR_DPI)
				{
					let dpi = get_dpi_for_window(hwnd);
					adjust_window_rect_ex_for_dpi(&mut rect, style, b_menu.into(), style_ex, dpi)
				} else {
					AdjustWindowRectEx(&mut rect, style, b_menu.into(), style_ex)
				}
			})?;
			Ok(rect)
		}
	}

	pub fn adjust_size(self, hwnd: HWND, size: PhysicalSize) -> PhysicalSize {
		let (width, height) = (size.width, size.height);
		let rect = RECT { left: 0, right: width as i32, top: 0, bottom: height as i32 };
		let rect = self.adjust_rect(hwnd, rect).unwrap_or(rect);

		let outer_x = (rect.right - rect.left).abs();
		let outer_y = (rect.top - rect.bottom).abs();

		PhysicalSize::new(outer_x as _, outer_y as _)
	}

	pub fn set_size(self, hwnd: HWND, size: PhysicalSize) {
		unsafe {
			let adjust_size = self.adjust_size(hwnd, size);
			let (width, height) = (adjust_size.width, adjust_size.height);
			SetWindowPos(
				hwnd,
				ptr::null_mut(),
				0,
				0,
				width as _,
				height as _,
				SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOREPOSITION | SWP_NOMOVE | SWP_NOACTIVATE,
			);
			InvalidateRgn(hwnd, ptr::null_mut(), false.into());
		}
	}
}
