use core::fmt;
use std::{
	any::Any,
	cell::{Cell, RefCell},
	collections::VecDeque,
	ffi::c_void,
	mem,
	os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle},
	panic, ptr,
	rc::Rc,
	sync::atomic::{AtomicBool, Ordering},
	time::{Duration, Instant},
};

use evloop_core::{
	error::EventLoopError,
	event::{DeviceEvent, DeviceId, WindowEvent},
	event_loop::{ControlFlow, PumpStatus},
	geometry::PhysicalSize,
};
use tracing::{trace, warn};
use windows_sys::Win32::{
	Foundation::{FALSE, GetLastError, HANDLE, HWND, WAIT_FAILED},
	System::Threading::{
		CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, GetCurrentThreadId,
		INFINITE, SetWaitableTimer, TIMER_ALL_ACCESS,
	},
	UI::WindowsAndMessaging::{
		DestroyWindow, DispatchMessageW, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx,
		PM_REMOVE, PeekMessageW, QS_ALLINPUT, TranslateMessage,
	},
};

pub struct EventLoopAttributes {
	pub any_thread: bool,
	pub dpi_aware: bool,
	pub msg_hook: Option<Box<dyn FnMut(*const c_void) -> bool + 'static>>,
}

pub struct EventLoop {
	runner: Rc<EventLoopRunner>,
	msg_hook: Option<Box<dyn FnMut(*const c_void) -> bool + 'static>>,
	high_resolution_timer: Option<OwnedHandle>,
}

impl fmt::Debug for EventLoop {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("EventLoop").finish_non_exhaustive()
	}
}

impl EventLoop {
	pub fn new(attributes: &mut EventLoopAttributes) -> Result<Self, EventLoopError> {
		static EVENT_LOOP_CREATED: AtomicBool = AtomicBool::new(false);
		if EVENT_LOOP_CREATED.swap(true, Ordering::Relaxed) {
			return Err(EventLoopError::Recreation);
		}

		let thread_id = unsafe { GetCurrentThreadId() };

		if !attributes.any_thread && thread_id != main_thread_id() {
			panic!(
				"Initializing the event loop outside of the main thread is a significant \
            cross-platform compatibility hazard. If you absolutely need to create an \
            EventLoop on a different thread, you can use the \
            `EventLoopBuilderExtWindows::with_any_thread` function."
			);
		}

		if attributes.dpi_aware {
			become_dpi_aware();
		}

		let thread_msg_target = create_event_target_window();

		let runner_shared = Rc::new(EventLoopRunner::new(thread_id, thread_msg_target));

		insert_event_target_window_data(thread_msg_target, runner_shared.clone());
		raw_input::register_all_mice_and_keyboards_for_raw_input(
			thread_msg_target,
			Default::default(),
		);

		Ok(EventLoop {
			runner: runner_shared,
			msg_hook: attributes.msg_hook.take(),
			high_resolution_timer: None,
		})
	}

	pub fn run_app_on_demand<A: ApplicationHandler>(
		&mut self,
		mut app: A,
	) -> Result<(), EventLoopError> {
		self.runner.clear_exit();

		let _app_resetter = unsafe { self.runner.set_app(&mut app) };

		let exit_code = loop {
			self.wait_for_messages(None);

			if let Some(code) = self.exit_code() {
				break code;
			}

			self.dispatch_peeked_messages();

			if let Some(code) = self.exit_code() {
				break code;
			}
		};

		self.runner.loop_destroyed();
		self.runner.reset_runner();

		if exit_code == 0 { Ok(()) } else { Err(EventLoopError::ExitFailure(exit_code)) }
	}

	pub fn run_app<A: ApplicationHandler + 'static>(
		mut self,
		mut app: A,
	) -> Result<(), EventLoopError> {
		let result = self.run_app_on_demand(&mut app);
		drop(app);
		result
	}

	pub fn pump_app_events<A: ApplicationHandler>(
		&mut self,
		timeout: Option<Duration>,
		mut app: A,
	) -> PumpStatus {
		let _app_resetter = unsafe { self.runner.set_app(&mut app) };

		self.runner.wakeup();

		if self.exit_code().is_none() {
			self.wait_for_messages(timeout);
		}

		if self.exit_code().is_none() {
			self.dispatch_peeked_messages();
		}

		if let Some(code) = self.runner.exit_code() {
			self.runner.loop_destroyed();

			self.runner.reset_runner();
			PumpStatus::Exit(code)
		} else {
			self.runner.prepare_wait();
			PumpStatus::Continue
		}
	}

	fn wait_for_messages(&mut self, timeout: Option<Duration>) {
		self.runner.prepare_wait();
		wait_for_messages_impl(
			&mut self.high_resolution_timer,
			self.runner.control_flow(),
			timeout,
		);
		self.runner.wakeup();
	}

	fn dispatch_peeked_messages(&mut self) {
		self.runner.interrupt_msg_dispatch.set(false);

		let mut msg: MSG = unsafe { mem::zeroed() };

		loop {
			unsafe {
				if PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) == false.into() {
					break;
				}

				let handled = if let Some(callback) = self.msg_hook.as_deref_mut() {
					callback(&mut msg as *mut _ as *mut _)
				} else {
					false
				};
				if !handled {
					TranslateMessage(&msg);
					DispatchMessageW(&msg);
				}
			}

			if let Err(payload) = self.runner.take_panic_error() {
				self.runner.reset_runner();
				panic::resume_unwind(payload);
			}

			if let Some(_code) = self.runner.exit_code() {
				break;
			}

			if self.runner.interrupt_msg_dispatch.get() {
				break;
			}
		}
	}

	fn exit_code(&self) -> Option<i32> {
		self.runner.exit_code()
	}
}

impl Drop for EventLoop {
	fn drop(&mut self) {
		unsafe {
			DestroyWindow(self.runner.thread_msg_target);
		}
	}
}

fn main_thread_id() -> u32 {
	static mut MAIN_THREAD_ID: u32 = 0;

	#[unsafe(link_section = ".CRT$XCU")]
	static INIT_MAIN_THREAD_ID: unsafe extern "C" fn() = {
		unsafe extern "C" fn initer() {
			unsafe {
				MAIN_THREAD_ID = GetCurrentThreadId();
			}
		}
		initer
	};

	unsafe { MAIN_THREAD_ID }
}

fn min_timeout(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
	a.map_or(b, |a_timeout| b.map_or(Some(a_timeout), |b_timeout| Some(a_timeout.min(b_timeout))))
}

fn dur2timeout(dur: Duration) -> u32 {
	dur.as_secs()
		.checked_mul(1000)
		.and_then(|ms| ms.checked_add((dur.subsec_nanos() as u64) / 1_000_000))
		.and_then(
			|ms| {
				if dur.subsec_nanos() % 1_000_000 > 0 { ms.checked_add(1) } else { Some(ms) }
			},
		)
		.map(|ms| if ms > u32::MAX as u64 { INFINITE } else { ms as u32 })
		.unwrap_or(INFINITE)
}

const FIFTY_DAYS: Duration = Duration::from_secs(50_u64 * 24 * 60 * 60);

const MIN_WAIT: Duration = Duration::from_nanos(100);

fn create_high_resolution_timer() -> Option<OwnedHandle> {
	unsafe {
		let handle: HANDLE = CreateWaitableTimerExW(
			ptr::null(),
			ptr::null(),
			CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
			TIMER_ALL_ACCESS,
		);

		if handle.is_null() { None } else { Some(OwnedHandle::from_raw_handle(handle)) }
	}
}

unsafe fn set_high_resolution_timer(timer: RawHandle, timeout: Duration) -> Result<(), u32> {
	const INTERVAL_NS: u32 = MIN_WAIT.subsec_nanos();
	const INTERVALS_IN_SEC: u64 = (Duration::from_secs(1).as_nanos() / INTERVAL_NS as u128) as u64;
	let intervals_to_wait: u64 =
		timeout.as_secs() * INTERVALS_IN_SEC + u64::from(timeout.subsec_nanos() / INTERVAL_NS);
	debug_assert!(intervals_to_wait < i64::MAX as u64, "Must be called with smaller duration");
	let due_time: i64 = -(intervals_to_wait as i64);
	unsafe {
		let set_result = SetWaitableTimer(timer as HANDLE, &due_time, 0, None, ptr::null(), FALSE);
		if set_result != FALSE { Ok(()) } else { Err(GetLastError()) }
	}
}

fn wait_for_messages_impl(
	high_resolution_timer: &mut Option<OwnedHandle>,
	control_flow: ControlFlow,
	timeout: Option<Duration>,
) {
	let timeout = {
		let control_flow_timeout = match control_flow {
			ControlFlow::Wait => None,
			ControlFlow::Poll => Some(Duration::ZERO),
			ControlFlow::WaitUntil(wait_deadline) => {
				let start = Instant::now();
				Some(wait_deadline.saturating_duration_since(start))
			}
		};
		let timeout = min_timeout(timeout, control_flow_timeout);
		if timeout == Some(Duration::ZERO) {
			return;
		}
		timeout.map(|t| t.min(FIFTY_DAYS)).map(|t| t.max(MIN_WAIT))
	};

	if timeout.is_some() && high_resolution_timer.is_none() {
		*high_resolution_timer = create_high_resolution_timer();
	}

	let high_resolution_timer: Option<RawHandle> =
		high_resolution_timer.as_ref().map(OwnedHandle::as_raw_handle);

	let use_timer: bool;
	if let (Some(handle), Some(timeout)) = (high_resolution_timer, timeout) {
		let res = unsafe { set_high_resolution_timer(handle, timeout) };
		if let Err(error_code) = res {
			trace!("Failed to set high resolution timer: last error {}", error_code);
			use_timer = false;
		} else {
			use_timer = true;
		}
	} else {
		use_timer = false;
	}

	unsafe {
		let wait_duration_ms = timeout.map(dur2timeout).unwrap_or(INFINITE);
		let (num_handles, raw_handles) =
			if use_timer { (1, [high_resolution_timer.unwrap()]) } else { (0, [ptr::null_mut()]) };

		let result = MsgWaitForMultipleObjectsEx(
			num_handles,
			raw_handles.as_ptr() as *const _,
			wait_duration_ms,
			QS_ALLINPUT,
			MWMO_INPUTAVAILABLE,
		);
		if result == WAIT_FAILED {
			warn!("Failed to MsgWaitForMultipleObjectsEx: error code {}", GetLastError(),);
		}
	}
}

pub(crate) struct EventLoopRunner {
	pub(super) thread_id: u32,
	pub(super) thread_msg_target: HWND,
	pub(super) interrupt_msg_dispatch: Cell<bool>,

	control_flow: Cell<ControlFlow>,
	exit: Cell<Option<i32>>,
	runner_state: Cell<RunnerState>,
	last_events_cleared: Cell<Instant>,
	event_handler: Rc<EventHandler>,
	event_buffer: RefCell<VecDeque<Event>>,

	pub(super) drag_state: RefCell<Option<DragState>>,
	pub(crate) source_drag: Cell<Option<SourceDrag>>,
	pub(super) pending_drag: RefCell<Option<PendingDrag>>,

	pending_source_drag_cleanup: Cell<Option<DataTransferId>>,

	panic_error: Cell<Option<PanicError>>,
}

impl fmt::Debug for EventLoopRunner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("EventLoopRunner")
			.field("thread_msg_target", &self.thread_msg_target)
			.finish_non_exhaustive()
	}
}

pub type PanicError = Box<dyn Any + Send + 'static>;

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
	WakeUp,
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
			drag_state: RefCell::new(None),
			source_drag: Cell::new(None),
			pending_drag: RefCell::new(None),
			pending_source_drag_cleanup: Cell::new(None),
		}
	}
}
