use core::fmt;
use std::{
	any::Any,
	cell::{Cell, RefCell},
	collections::VecDeque,
	rc::Rc,
	time::Instant,
};

use evloop_core::{
	data_transfer::DataTransferId,
	event::{DeviceId, WindowEvent},
	event_loop::ControlFlow,
	window::WindowId,
};
use geometry_types::PhysicalSize;
use windows_sys::Win32::Foundation::HWND;

use crate::{application::ApplicationHandler, data_transfer::state::DataTransferState};

type EventHandler = Cell<Option<&'static mut (dyn ApplicationHandler + 'static)>>;

pub type PanicError = Box<dyn Any + Send + 'static>;

pub(crate) struct EventLoopRunner {
	pub(crate) thread_id: u32,
	pub(crate) thread_msg_target: HWND,
	pub(crate) interrupt_msg_dispatch: Cell<bool>,

	control_flow: Cell<ControlFlow>,
	exit: Cell<Option<i32>>,
	runner_state: Cell<RunnerState>,
	last_events_cleared: Cell<Instant>,
	event_handler: Rc<EventHandler>,
	event_buffer: RefCell<VecDeque<Event>>,

	data_transfer: DataTransferState,
	windows: RefCell<Vec<WindowId>>,

	pub(crate) source_drag: Cell<Option<SourceDrag>>,
	pub(super) pending_drag: RefCell<Option<PendingDrag>>,

	pending_source_drag_cleanup: Cell<Option<DataTransferId>>,
	panic_error: Cell<Option<PanicError>>,
}

impl EventLoopRunner {
	pub(crate) fn new(thread_id: u32, thread_msg_target: HWND) -> Self {
		Self {
			thread_id,
			thread_msg_target,
			interrupt_msg_dispatch: Cell::new(false),
			runner_state: Cell::new(RunnerState::Uninitialized),
			control_flow: Cell::new(ControlFlow::default()),
			exit: Cell::new(None),
			panic_error: Cell::new(None),
			last_events_cleared: Cell::new(Instant::now()),
			event_handler: Rc::new(Cell::new(None)),
			event_buffer: RefCell::new(VecDeque::new()),
			data_transfer: DataTransferState::default(),
			windows: RefCell::new(Vec::new()),
			source_drag: Cell::new(None),
			pending_drag: RefCell::new(None),
			pending_source_drag_cleanup: Cell::new(None),
		}
	}
}

impl fmt::Debug for EventLoopRunner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("EventLoopRunner")
			.field("thread_msg_target", &self.thread_msg_target)
			.finish_non_exhaustive()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RunnerState {
	Uninitialized,
	Idle,
	HandlingMainEvents,
	Destroyed,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
	Device { device_id: DeviceId, event: DeviceEvent },
	Window { window_id: WindowId, event: WindowEvent },
	BufferedScaleFactorChanged(HWND, f64, PhysicalSize),

	// FIXME(Tito5e): [UPSTREAM] Coalesce these into a flag (or similar) instead of handling them as events.
	// https://github.com/rust-windowing/winit/pull/3687
	WakeUp,
}
