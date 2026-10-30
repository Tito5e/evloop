use core::fmt;
use std::{
	any::Any,
	cell::{Cell, RefCell},
	collections::VecDeque,
	mem, panic,
	rc::Rc,
	time::Instant,
};

use evloop_core::{
	data_transfer::DataTransferId,
	event::{EvloopEvent, WindowEvent},
	event_loop::{ControlFlow, StartCause},
	window::WindowId,
};
use geometry_types::PhysicalSize;
use windows_sys::Win32::Foundation::HWND;

use crate::{
	application::ApplicationHandler, data_transfer::state::DataTransferState,
	event_loop::active::ActiveEventLoop,
};

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

	pub(crate) fn data_transfer(&self) -> &DataTransferState {
		&self.data_transfer
	}

	pub(crate) unsafe fn set_app<'app>(
		&self,
		app: &'app mut (dyn ApplicationHandler + 'app),
	) -> impl Drop + 'app {
		let f = unsafe {
			mem::transmute::<
				&'app mut (dyn ApplicationHandler + 'app),
				&'static mut (dyn ApplicationHandler + 'static),
			>(app)
		};

		let old_event_handler = self.event_handler.replace(Some(f));

		assert!(old_event_handler.is_none());

		struct Resetter(Rc<EventHandler>);

		impl Drop for Resetter {
			fn drop(&mut self) {
				self.0.set(None);
			}
		}

		Resetter(self.event_handler.clone())
	}

	pub(crate) fn reset_runner(&self) {
		self.interrupt_msg_dispatch.set(false);
		self.runner_state.set(RunnerState::Uninitialized);
		self.panic_error.set(None);
		self.exit.set(None);
		self.event_handler.set(None);
		self.data_transfer.reset();
		self.source_drag.set(None);
		self.pending_drag.take();
		self.pending_source_drag_cleanup.set(None);
	}

	pub fn thread_msg_target(&self) -> HWND {
		self.thread_msg_target
	}

	pub fn take_panic_error(&self) -> Result<(), PanicError> {
		match self.panic_error.take() {
			Some(err) => Err(err),
			None => Ok(()),
		}
	}

	pub fn control_flow(&self) -> ControlFlow {
		self.control_flow.get()
	}

	pub fn exit_code(&self) -> Option<i32> {
		self.exit.get()
	}

	pub fn clear_exit(&self) {
		self.exit.set(None);
	}

	pub fn should_buffer(&self) -> bool {
		let handler = self.event_handler.take();
		let should_buffer = handler.is_none();
		self.event_handler.set(handler);
		should_buffer
	}

	pub fn catch_unwind<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
		let panic_error = self.panic_error.take();
		if panic_error.is_none() {
			let result = panic::catch_unwind(panic::AssertUnwindSafe(f));

			match self.panic_error.take() {
				None => match result {
					Ok(r) => Some(r),
					Err(e) => {
						self.panic_error.set(Some(e));
						None
					}
				},
				Some(e) => {
					self.panic_error.set(Some(e));
					None
				}
			}
		} else {
			self.panic_error.set(panic_error);
			None
		}
	}

	pub(crate) fn prepare_wait(self: &Rc<Self>) {
		self.move_state_to(RunnerState::Idle);
	}

	pub(crate) fn wakeup(self: &Rc<Self>) {
		self.move_state_to(RunnerState::HandlingMainEvents);
	}

	pub(crate) fn send_event(self: &Rc<Self>, event: Event) {
		if let Event::Window { event: WindowEvent::RedrawRequested, .. } = event {
			self.call_event_handler(|app, event_loop| event.dispatch_event(app, event_loop));

			self.interrupt_msg_dispatch.set(true);
		} else if self.should_buffer() {
			self.event_buffer.borrow_mut().push_back(event.buffer_scale_factor());
		} else {
			self.call_event_handler(|app, event_loop| event.dispatch_event(app, event_loop));
			self.dispatch_buffered_events();
		}
	}

	pub(crate) fn loop_destroyed(self: &Rc<Self>) {
		self.move_state_to(RunnerState::Destroyed);
	}

	fn call_event_handler(
		self: &Rc<Self>,
		closure: impl FnOnce(&mut dyn ApplicationHandler, &ActiveEventLoop),
	) {
		self.catch_unwind(|| {
			let event_handler = self.event_handler.take().expect("either event handler is re-entrant (likely), or no event handler is registered (very unlikely)");

			closure(event_handler, ActiveEventLoop::from_ref(self));

			assert!(self.event_handler.replace(Some(event_handler)).is_none());
		});
	}

	fn dispatch_buffered_events(self: &Rc<Self>) {
		loop {
			let buffered_event_opt = self.event_buffer.borrow_mut().pop_front();
			match buffered_event_opt {
				Some(e) => {
					self.call_event_handler(|app, event_loop| e.dispatch_event(app, event_loop))
				}
				None => break,
			}
		}

		if let Some(id) = self.pending_source_drag_cleanup.take() {
			self.data_transfer.remove_drag(id);
		}
	}

	fn move_state_to(self: &Rc<Self>, new_runner_state: RunnerState) {
		use RunnerState::*;

		match (self.runner_state.replace(new_runner_state), new_runner_state) {
			(Uninitialized, Uninitialized)
			| (Idle, Idle)
			| (HandlingMainEvents, HandlingMainEvents)
			| (Destroyed, Destroyed) => (),

			(Uninitialized, HandlingMainEvents) => {
				self.call_new_events(true);
			}
			(Uninitialized, Idle) => {
				self.call_new_events(true);
				self.call_event_handler(|app, event_loop| app.exit_events(event_loop));
				self.last_events_cleared.set(Instant::now());
			}
			(Uninitialized, Destroyed) => {
				self.call_new_events(true);
				self.call_event_handler(|app, event_loop| app.exit_events(event_loop));
				self.last_events_cleared.set(Instant::now());
			}
			(_, Uninitialized) => panic!("cannot move state to Uninitialized"),

			(Idle, HandlingMainEvents) => {
				self.call_new_events(false);
			}
			(Idle, Destroyed) => {}

			(HandlingMainEvents, Idle) => {
				self.call_event_handler(|app, event_loop| app.exit_events(event_loop));
				self.last_events_cleared.set(Instant::now());
			}
			(HandlingMainEvents, Destroyed) => {
				self.call_event_handler(|app, event_loop| app.exit_events(event_loop));
				self.last_events_cleared.set(Instant::now());
			}

			(Destroyed, _) => panic!("cannot move state from Destroyed"),
		}
	}

	fn call_new_events(self: &Rc<Self>, init: bool) {
		let start_cause = match (init, self.control_flow(), self.exit_code()) {
			(true, ..) => StartCause::Init,
			(false, ControlFlow::Poll, None) => StartCause::Poll,
			(false, _, Some(_)) | (false, ControlFlow::Wait, None) => StartCause::WaitCancelled {
				start: self.last_events_cleared.get(),
				requested_resume: None,
			},
			(false, ControlFlow::WaitUntil(requested_resume), None) => {
				if Instant::now() < requested_resume {
					StartCause::WaitCancelled {
						start: self.last_events_cleared.get(),
						requested_resume: Some(requested_resume),
					}
				} else {
					StartCause::ResumeTimeReached {
						start: self.last_events_cleared.get(),
						requested_resume,
					}
				}
			}
		};
		self.call_event_handler(|app, event_loop| app.enter_events(event_loop, start_cause));

		if init {
			self.call_event_handler(|app, event_loop| app.surface_available(event_loop));
		}
		self.dispatch_buffered_events();
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
	Window { window_id: WindowId, event: WindowEvent },
	BufferedScaleFactorChanged(HWND, f64 /* , PhysicalSize */),
	// FIXME(Tito5e): [UPSTREAM] Coalesce these into a flag (or similar) instead of handling them as events.
	// https://github.com/rust-windowing/winit/pull/3687
	WakeUp,
}

impl Event {
	pub fn buffer_scale_factor(self) -> Self {
		match self {
			Self::Window { window_id, event: WindowEvent::ScaleFactorChanged(scale_factor) } => {
				Event::BufferedScaleFactorChanged(
					window_id.into_raw() as HWND,
					scale_factor, /* TODO: Size Handling */
				)
			}
			event => event,
		}
	}

	pub fn dispatch_event(self, app: &mut dyn ApplicationHandler, event_loop: &ActiveEventLoop) {
		match self {
			Self::Window { window_id, event } => app.window_event(event_loop, window_id, event),
			Self::BufferedScaleFactorChanged(window, scale_factor) => {
				//let user_new_surface_size = Arc::new(Mutex::new(new_surface_size));
				app.window_event(
					event_loop,
					WindowId::from_raw(window as usize),
					WindowEvent::ScaleFactorChanged(scale_factor),
				);

				// TODO: Size Handling
			}
			Self::WakeUp => app.evloop_event(event_loop, EvloopEvent::ProxyWakeUp),
		}
	}
}
