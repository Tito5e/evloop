use std::{borrow::Cow, mem, ptr, sync::Arc};

use geometry_types::PhysicalPosition;
use windows_sys::Win32::{
	Graphics::Gdi::{
		CDS_FULLSCREEN, ChangeDisplaySettingsExW, DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE,
		DISP_CHANGE_BADPARAM, DISP_CHANGE_FAILED, DISP_CHANGE_SUCCESSFUL, InvalidateRgn,
	},
	UI::WindowsAndMessaging::{
		GetWindowPlacement, PM_NOREMOVE, PeekMessageW, SWP_ASYNCWINDOWPOS, SWP_NOZORDER,
		SetWindowPlacement, SetWindowPos,
	},
};

use crate::{
	monitor::{Fullscreen, current_monitor, get_monitor_info},
	window::{
		Window,
		flags::WindowFlags,
		state::{SavedWindow, WindowState},
		taskbar::taskbar_mark_fullscreen,
	},
};

impl Window {
	pub(crate) fn apply_fullscreen(&self, fullscreen: Option<Fullscreen>) {
		let window = self.window;
		let window_state = Arc::clone(&self.window_state);

		let mut window_state_lock = window_state.lock().unwrap();
		let old_fullscreen = window_state_lock.fullscreen.clone();

		match (&old_fullscreen, &fullscreen) {
			// Return if we already are in the same fullscreen mode
			_ if old_fullscreen == fullscreen => return,
			// Return if saved Borderless(monitor) is the same as current monitor when requested
			// fullscreen is Borderless(None)
			(Some(Fullscreen::Borderless(Some(monitor))), Some(Fullscreen::Borderless(None)))
				if monitor.native_id() == current_monitor(window.hwnd()).native_id() =>
			{
				return;
			}
			_ => {}
		}

		window_state_lock.fullscreen.clone_from(&fullscreen);
		drop(window_state_lock);

		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			// Change video mode if we're transitioning to or from exclusive
			// fullscreen
			match (&old_fullscreen, &fullscreen) {
				(_, Some(Fullscreen::Exclusive(monitor, video_mode))) => {
					let video_mode =
						match monitor.video_mode_handles().find(|mode| &mode.mode == video_mode) {
							Some(monitor) => monitor,
							None => return,
						};
					let monitor_info = get_monitor_info(monitor.native_id() as _).unwrap();

					let res = unsafe {
						ChangeDisplaySettingsExW(
							monitor_info.szDevice.as_ptr(),
							&*video_mode.native_video_mode,
							ptr::null_mut(),
							CDS_FULLSCREEN,
							ptr::null(),
						)
					};

					debug_assert!(res != DISP_CHANGE_BADFLAGS);
					debug_assert!(res != DISP_CHANGE_BADMODE);
					debug_assert!(res != DISP_CHANGE_BADPARAM);
					debug_assert!(res != DISP_CHANGE_FAILED);
					assert_eq!(res, DISP_CHANGE_SUCCESSFUL);
				}
				(Some(Fullscreen::Exclusive(..)), _) => {
					let res = unsafe {
						ChangeDisplaySettingsExW(
							ptr::null(),
							ptr::null(),
							ptr::null_mut(),
							CDS_FULLSCREEN,
							ptr::null(),
						)
					};

					debug_assert!(res != DISP_CHANGE_BADFLAGS);
					debug_assert!(res != DISP_CHANGE_BADMODE);
					debug_assert!(res != DISP_CHANGE_BADPARAM);
					debug_assert!(res != DISP_CHANGE_FAILED);
					assert_eq!(res, DISP_CHANGE_SUCCESSFUL);
				}
				_ => (),
			}

			unsafe {
				// There are some scenarios where calling `ChangeDisplaySettingsExW` takes long
				// enough to execute that the DWM thinks our program has frozen and takes over
				// our program's window. When that happens, the `SetWindowPos` call below gets
				// eaten and the window doesn't get set to the proper fullscreen position.
				//
				// Calling `PeekMessageW` here notifies Windows that our process is still running
				// fine, taking control back from the DWM and ensuring that the `SetWindowPos` call
				// below goes through.
				let mut msg = mem::zeroed();
				PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_NOREMOVE);
			}

			// Update window style
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(
					WindowFlags::MARKER_EXCLUSIVE_FULLSCREEN,
					matches!(fullscreen, Some(Fullscreen::Exclusive(_, _))),
				);
				f.set(
					WindowFlags::MARKER_BORDERLESS_FULLSCREEN,
					matches!(fullscreen, Some(Fullscreen::Borderless(_))),
				);
			});

			// Mark as fullscreen window wrt to z-order
			//
			// this needs to be called before the below fullscreen SetWindowPos as this itself
			// will generate WM_SIZE messages of the old window size that can race with what we set
			// below
			unsafe {
				taskbar_mark_fullscreen(window.hwnd(), fullscreen.is_some());
			}

			// Update window bounds
			match &fullscreen {
				Some(fullscreen) => {
					// Save window bounds before entering fullscreen
					let placement = unsafe {
						let mut placement = mem::zeroed();
						GetWindowPlacement(window.hwnd(), &mut placement);
						placement
					};

					window_state.lock().unwrap().saved_window = Some(SavedWindow { placement });

					let monitor = match &fullscreen {
						Fullscreen::Exclusive(monitor, _)
						| Fullscreen::Borderless(Some(monitor)) => Some(Cow::Borrowed(monitor)),
						_ => None,
					};

					let monitor =
						monitor.unwrap_or_else(|| Cow::Owned(current_monitor(window.hwnd())));

					let position = monitor.position().unwrap_or(PhysicalPosition::new(0, 0));
					let size = monitor.size();

					unsafe {
						SetWindowPos(
							window.hwnd(),
							ptr::null_mut(),
							position.x,
							position.y,
							size.width as i32,
							size.height as i32,
							SWP_ASYNCWINDOWPOS | SWP_NOZORDER,
						);
						InvalidateRgn(window.hwnd(), ptr::null_mut(), false.into());
					}
				}
				None => {
					let mut window_state_lock = window_state.lock().unwrap();
					if let Some(SavedWindow { placement }) = window_state_lock.saved_window.take() {
						drop(window_state_lock);
						unsafe {
							SetWindowPlacement(window.hwnd(), &placement);
							InvalidateRgn(window.hwnd(), ptr::null_mut(), false.into());
						}
					}
				}
			}
		});
	}
}
