use std::sync::{Mutex, Weak};

use evloop_core::{
	data_transfer::{DataTransferId, DndAction},
	event::{DeviceId, ElementState},
	ime::Ime,
	keyboard::{KeyEvent, Modifiers, PhysicalKey},
	pointer::{
		ButtonSource, MouseScrollDelta, PointerKind, PointerSource, ScrollSource, TouchPhase,
	},
	window::Theme,
};
use geometry_types::{PhysicalPosition, PhysicalSize};

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
	ScaleFactorChanged {
		scale_factor: f64,
		/// Lets the application pick the surface size the window gets after the change.
		surface_size_writer: SurfaceSizeWriter,
	},
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
