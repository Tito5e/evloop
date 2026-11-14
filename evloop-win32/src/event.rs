use std::sync::{Mutex, Weak};

use evloop_core::{
	data_transfer::{DataTransferId, DndAction},
	event::{DeviceId, ElementState},
	ime::Ime,
	keyboard::{Modifiers, PhysicalKey},
	pointer::{
		ButtonSource, MouseScrollDelta, PointerKind, PointerSource, ScrollSource, TouchPhase,
	},
	window::{PhysicalInsets, Theme},
};
use geometry_types::{PhysicalPosition, PhysicalSize};

use crate::keyboard::KeyEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
	Resized(PhysicalSize),
	Moved(PhysicalPosition),
	CloseRequested,
	Destroyed,
	Focused(bool),
	KeyboardInput {
		device_id: Option<DeviceId>,
		event: KeyEvent,
		is_synthetic: bool,
	},
	ModifiersChanged(Modifiers),
	Ime(Ime),
	PointerMoved {
		device_id: Option<DeviceId>,
		position: PhysicalPosition,
		primary: bool,
		source: PointerSource,
	},
	PointerEntered {
		device_id: Option<DeviceId>,
		position: PhysicalPosition,
		primary: bool,
		kind: PointerKind,
	},
	PointerLeft {
		device_id: Option<DeviceId>,
		position: Option<PhysicalPosition>,
		primary: bool,
		kind: PointerKind,
	},
	MouseWheel {
		device_id: Option<DeviceId>,
		delta: MouseScrollDelta,
		phase: TouchPhase,
		source: ScrollSource,
	},
	PointerButton {
		device_id: Option<DeviceId>,
		state: ElementState,
		position: PhysicalPosition,
		primary: bool,
		button: ButtonSource,
	},
	/// The scale factor is about to change to `scale_factor` (the window moved to a monitor with
	/// a different DPI). Sent from `WM_GETDPISCALEDSIZE`, before [`Self::ScaleFactorChanged`].
	///
	/// `surface_size_writer` initially holds the surface size scaled linearly to the new scale
	/// factor. Write another size to choose the surface size the window gets after the change;
	/// leave it untouched to let the OS scale the window. The writer is only effective while this
	/// event is being handled.
	///
	/// Only emitted for processes that are Per-Monitor V2 DPI aware, and never while the window
	/// is maximized or fullscreen. The order is `ScaleFactorChanging` → `ScaleFactorChanged` →
	/// `Resized`.
	ScaleFactorChanging {
		scale_factor: f64,
		surface_size_writer: SurfaceSizeWriter,
	},
	/// The scale factor changed. Followed by `Resized` if the surface size changed.
	ScaleFactorChanged(f64),
	ThemeChanged(Theme),
	RedrawRequested,
	DragEntered {
		id: DataTransferId,
		position: Option<PhysicalPosition>,
	},
	DragPosition {
		id: DataTransferId,
		position: PhysicalPosition,
		proposed_action: Option<DndAction>,
	},
	DragDropped {
		id: DataTransferId,
		proposed_action: Option<DndAction>,
	},
	DragLeft {
		id: DataTransferId,
	},
	OutgoingDragDropped {
		id: DataTransferId,
		action: Option<DndAction>,
	},
	OutgoingDragCanceled {
		id: DataTransferId,
	},
	/// The safe area (`Window::safe_area`) changed.
	///
	/// Windows draws no system UI over the client area, so the safe area is always zero and this
	/// event is never emitted. It exists so that the same code handles it on every backend.
	SafeAreaChanged(PhysicalInsets),
	/// The area covered by an on-screen keyboard (`Window::ime_occlusion`) changed.
	///
	/// The Windows touch keyboard is not tracked yet, so this event is currently never emitted.
	ImeOcclusionChanged(PhysicalInsets),
	/// The window became fully hidden from the user (`true`) or visible again (`false`).
	///
	/// On Windows this is reported when the window is minimized and restored. Applications can
	/// stop rendering while occluded.
	Occluded(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceEvent {
	PointerMotion { delta: (f64, f64) },
	MouseWheel { delta: MouseScrollDelta },
	Button { button: u32, state: ElementState },
	Key(RawKeyEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawKeyEvent {
	pub physical_key: PhysicalKey,
	pub state: ElementState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSizeWriterClosed;

#[derive(Debug, Clone)]
pub struct SurfaceSizeWriter {
	new_surface_size: Weak<Mutex<PhysicalSize>>,
}

impl SurfaceSizeWriter {
	pub(crate) fn new(new_surface_size: Weak<Mutex<PhysicalSize>>) -> Self {
		Self { new_surface_size }
	}

	pub fn request_surface_size(
		&mut self,
		new_surface_size: PhysicalSize,
	) -> Result<(), SurfaceSizeWriterClosed> {
		let inner = self.new_surface_size.upgrade().ok_or(SurfaceSizeWriterClosed)?;
		*inner.lock().unwrap() = new_surface_size;
		Ok(())
	}

	pub fn surface_size(&self) -> Result<PhysicalSize, SurfaceSizeWriterClosed> {
		let inner = self.new_surface_size.upgrade().ok_or(SurfaceSizeWriterClosed)?;
		Ok(*inner.lock().unwrap())
	}
}

impl PartialEq for SurfaceSizeWriter {
	fn eq(&self, other: &Self) -> bool {
		self.new_surface_size.as_ptr() == other.new_surface_size.as_ptr()
	}
}

impl Eq for SurfaceSizeWriter {}

/// Returns a stable identifier (the device interface path) for a device reported in
/// [`ApplicationHandler::device_event`](crate::application::ApplicationHandler::device_event),
/// which stays the same across reconnections and reboots.
pub fn device_persistent_identifier(device_id: DeviceId) -> Option<String> {
	crate::input::device_persistent_identifier(&device_id)
}
