use crate::{
	glue::input::Keycode,
	platform::{ImeAction, TextInputState},
};
use evloop_core::data_transfer::DataTransferId;
use evloop_core::data_transfer::DndAction;
use evloop_core::event::DeviceId;
use evloop_core::event::ElementState;
use evloop_core::ime::Ime;
use evloop_core::keyboard::KeyLocation;
use evloop_core::keyboard::LogicalKey;
use evloop_core::keyboard::Modifiers;
use evloop_core::keyboard::PhysicalKey;
use evloop_core::pointer::ButtonSource;
use evloop_core::pointer::MouseScrollDelta;
use evloop_core::pointer::PointerKind;
use evloop_core::pointer::PointerSource;
use evloop_core::pointer::ScrollSource;
use evloop_core::pointer::TouchPhase;
use evloop_core::window::{PhysicalInsets, Theme};
use geometry_types::PhysicalPosition;
use geometry_types::PhysicalSize;
use smol_str::SmolStr;

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
	/*ScaleFactorChanged {
		scale_factor: f64,
		/// Lets the application pick the surface size the window gets after the change.
		surface_size_writer: SurfaceSizeWriter,
	},*/
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
	Occluded(bool),
	/// The safe area (`Window::safe_area`) changed. Not emitted when only the surface size
	/// changes: query `Window::safe_area` in `surface_available` for the initial value.
	SafeAreaChanged(PhysicalInsets),
	/// The area covered by the on-screen keyboard (`Window::ime_occlusion`) changed. Use this to
	/// keep the edited text visible.
	ImeOcclusionChanged(PhysicalInsets),
	/// Any window inset changed (see `Window::insets`). Android-specific.
	InsetsChanged,
	/// The on-screen keyboard was shown or hidden, independently of the IME session (the user
	/// can hide it while the session stays enabled). Android-specific.
	SoftKeyboardVisibilityChanged(bool),
	/// The action key of the on-screen keyboard was pressed. Android-specific.
	ImeAction(ImeAction),
	/// The IME's raw editor state changed. Android-specific; `Ime` events are derived from it.
	TextInputStateChanged(TextInputState),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyEvent {
	pub raw_key: Keycode,
	pub physical_key: PhysicalKey,
	pub logical_key: LogicalKey,
	pub text: Option<SmolStr>,
	pub location: KeyLocation,
	pub state: ElementState,
	pub repeat: bool,
	pub text_with_all_modifiers: Option<SmolStr>,
	pub key_without_modifiers: LogicalKey,
}
