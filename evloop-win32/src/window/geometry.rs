use std::{mem, ptr, sync::Arc};

use evloop_core::{
	dpi::{Position, Size},
	error::OsError,
	window::WindowPositioner,
};
use geometry_types::{PhysicalBounds, PhysicalPosition, PhysicalSize};
use windows_sys::Win32::{
	Foundation::{POINT, RECT},
	Graphics::Gdi::{InvalidateRgn, ScreenToClient},
	UI::WindowsAndMessaging::{
		GetClientRect, GetParent, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
		SetWindowPos,
	},
};

use crate::{
	monitor::{MonitorHandle, available_monitors, current_monitor, primary_monitor},
	window::{
		Window,
		flags::WindowFlags,
		placement::{
			compute_anchored_placement, get_outer_rect, outer_size_of, translate_outer_position,
		},
		state::WindowState,
	},
};

impl Window {
	pub fn scale_factor(&self) -> f64 {
		self.window_state_lock().scale_factor
	}

	pub fn outer_position(&self) -> Result<PhysicalPosition, OsError> {
		get_outer_rect(self.hwnd())
			.map(|rect| {
				Ok(self
					.translate_outer_position_to_parent(PhysicalPosition::new(rect.left, rect.top)))
			})
			.expect("Unexpected GetWindowRect failure")
	}

	pub fn set_outer_position(&self, position: Position) {
		let position = self.translate_outer_position(position);

		let window_state = Arc::clone(&self.window_state);
		let window = self.window;
		self.thread_executor.execute_in_thread(move || {
			let _ = &window;
			WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
				f.set(WindowFlags::MAXIMIZABLE, false)
			});
		});

		unsafe {
			SetWindowPos(
				self.hwnd(),
				ptr::null_mut(),
				position.x,
				position.y,
				0,
				0,
				SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOSIZE | SWP_NOACTIVATE,
			);
			InvalidateRgn(self.hwnd(), ptr::null_mut(), false.into());
		}
	}

	pub fn surface_position(&self) -> PhysicalPosition {
		let mut rect: RECT = unsafe { mem::zeroed() };
		if unsafe { GetClientRect(self.hwnd(), &mut rect) } == false.into() {
			panic!("Unexpected GetClientRect failure")
		}
		PhysicalPosition::new(rect.left, rect.top)
	}

	pub fn content_rect(&self) -> PhysicalBounds {
		let size = self.surface_size();
		PhysicalBounds::lrtb(0, size.width as i32, 0, size.height as i32)
	}

	pub fn outer_size(&self) -> PhysicalSize {
		outer_size_of(self.hwnd())
	}

	pub fn surface_size(&self) -> PhysicalSize {
		self.window_state_lock().surface_size
	}

	pub fn request_surface_size(&self, size: Size) {
		let scale_factor = self.scale_factor();
		let physical_size = size.to_physical(scale_factor);

		let window_flags = self.window_state_lock().window_flags;
		window_flags.set_size(self.hwnd(), physical_size);

		if physical_size != self.surface_size() {
			let window_state = Arc::clone(&self.window_state);
			let window = self.window;
			self.thread_executor.execute_in_thread(move || {
				let _ = &window;
				WindowState::set_window_flags(window_state.lock().unwrap(), window.hwnd(), |f| {
					f.set(WindowFlags::MAXIMIZABLE, false)
				});
			});
		}
	}

	pub fn set_min_surface_size(&self, size: Option<Size>) {
		self.window_state_lock().min_surface_size = size;
		let size = self.surface_size();
		self.request_surface_size(Size::Physical(size));
	}

	pub fn set_max_surface_size(&self, size: Option<Size>) {
		self.window_state_lock().max_surface_size = size;
		let size = self.surface_size();
		self.request_surface_size(Size::Physical(size));
	}

	pub fn resize_increments(&self) -> Option<PhysicalSize> {
		let w = self.window_state_lock();
		let scale_factor = w.scale_factor;
		w.resize_increments.map(|size| size.to_physical(scale_factor))
	}

	pub fn set_resize_increments(&self, increments: Option<Size>) {
		self.window_state_lock().resize_increments = increments;
	}

	pub fn current_monitor(&self) -> MonitorHandle {
		current_monitor(self.hwnd())
	}

	pub fn available_monitors(&self) -> impl Iterator<Item = MonitorHandle> {
		available_monitors().into_iter()
	}

	pub fn primary_monitor(&self) -> MonitorHandle {
		primary_monitor()
	}

	pub fn positioner(&self) -> WindowPositioner {
		self.window_state_lock().positioner
	}

	pub fn set_positioner(&self, positioner: WindowPositioner) {
		self.window_state_lock().positioner = positioner;
		self.reposition();
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
				unsafe { ScreenToClient(parent, &mut point) };
			}
		}

		PhysicalPosition::new(point.x, point.y)
	}

	pub(super) fn reposition(&self) {
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
			self.request_surface_size(Size::Logical(size));
		}
	}
}
