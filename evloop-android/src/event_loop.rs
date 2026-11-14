use std::{
	cell::Cell,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::{Duration, Instant},
};

use evloop_core::{
	error::EventLoopError,
	event::{DeviceId, ElementState, EvloopEvent},
	event_loop::{ControlFlow, PumpStatus, StartCause},
	pointer::{ButtonSource, FingerId, Force, PointerKind, PointerSource},
	window::{PhysicalInsets, Theme},
};
use geometry_types::{PhysicalPosition, PhysicalSize};
use ndk::configuration::UiModeNight;
use tracing::trace;

use crate::{
	HAS_FOCUS, RedrawRequester, SharedFlag,
	application::ApplicationHandler,
	event::{KeyEvent, WindowEvent},
	glue::{
		AndroidApp, AndroidAppWaker, MainEvent,
		input::{InputEvent, KeyAction, MotionAction, MotionEvent, ToolType},
	},
	keycodes, min_timeout, scale_factor,
	window::Window,
};

pub struct EventLoop {
	android_app: AndroidApp,
	active_loop: ActiveEventLoop,
	cause: StartCause,

	loop_running: bool,

	pending_redraw: bool,
	redraw_flag: SharedFlag,
	running: bool,

	primary_pointer: Option<FingerId>,

	combining_accent: Option<char>,

	/// Last values seen, to report only actual changes on `ConfigChanged`.
	last_scale_factor: f64,
	last_theme: Option<Theme>,
	last_safe_area: PhysicalInsets,
	last_ime_occlusion: PhysicalInsets,
}

impl EventLoop {
	pub fn new(attributes: &EventLoopAttributes) -> Result<Self, EventLoopError> {
		let android_app = attributes
			.android_app
			.as_ref()
			.expect("`AndroidApp` is required to create `EventLoop` on Android platform");

		let event_loop_proxy = EventLoopProxy::new(android_app.create_waker());

		let redraw_flag = SharedFlag::new();

		let active_loop = ActiveEventLoop {
			app: android_app.clone(),
			control_flow: Cell::new(ControlFlow::default()),
			exit: Cell::new(false),
			redraw_requester: RedrawRequester::new(&redraw_flag, android_app.create_waker()),
			event_loop_proxy,
		};
		let last_theme = active_loop.system_theme();

		Ok(Self {
			android_app: android_app.clone(),
			active_loop,
			cause: StartCause::Init,

			loop_running: false,

			pending_redraw: false,
			redraw_flag,
			running: false,

			primary_pointer: None,

			combining_accent: None,

			last_scale_factor: scale_factor(android_app),
			last_theme,
			last_safe_area: PhysicalInsets::ZERO,
			last_ime_occlusion: PhysicalInsets::ZERO,
		})
	}

	fn single_iter<A: ApplicationHandler>(&mut self, main_event: Option<MainEvent>, app: &mut A) {
		trace!("Mainloop Iter");

		let cause = self.cause;
		let mut resized = false;
		let mut pending_redraw = self.pending_redraw;

		app.enter_events(&self.active_loop, cause);
		if cause == StartCause::Init {
			// Same order as the Win32 backend: `enter_events(Init)` then `started(Init)`.
			app.started(&self.active_loop, cause);
		}

		// Requests made outside of the event loop's callbacks (e.g. from another thread).
		self.dispatch_pending_ime_events(app);

		if let Some(event) = main_event {
			trace!("Handling MainEvent: {:?}", event);

			match event {
				MainEvent::InitWindow => {
					app.surface_available(&self.active_loop, Window::new(&self.active_loop));
				}
				MainEvent::TerminateWindow => {
					app.surface_lost(&self.active_loop);
				}
				MainEvent::WindowResized => {
					resized = true;
				}
				MainEvent::RedrawNeeded => {
					pending_redraw = true;
				}
				MainEvent::InsetsChanged => {
					app.window_event(&self.active_loop, WindowEvent::InsetsChanged);

					let safe_area = crate::window::safe_area(&self.android_app);
					if safe_area != self.last_safe_area {
						self.last_safe_area = safe_area;
						app.window_event(
							&self.active_loop,
							WindowEvent::SafeAreaChanged(safe_area),
						);
					}

					let ime_occlusion = crate::window::ime_occlusion(&self.android_app);
					if ime_occlusion != self.last_ime_occlusion {
						self.last_ime_occlusion = ime_occlusion;
						let event = WindowEvent::ImeOcclusionChanged(ime_occlusion);
						app.window_event(&self.active_loop, event);
					}
				}
				// The content rect of the activity is described by the insets.
				MainEvent::ContentRectChanged => {}
				MainEvent::GainedFocus => {
					HAS_FOCUS.store(true, Ordering::Relaxed);
					let event = WindowEvent::Focused(true);
					app.window_event(&self.active_loop, event);
				}
				MainEvent::LostFocus => {
					HAS_FOCUS.store(false, Ordering::Relaxed);
					let event = WindowEvent::Focused(false);
					app.window_event(&self.active_loop, event);
				}
				MainEvent::ConfigChanged => {
					let scale_factor = scale_factor(&self.android_app);
					if scale_factor != self.last_scale_factor {
						self.last_scale_factor = scale_factor;
						let event = WindowEvent::ScaleFactorChanged(scale_factor);
						app.window_event(&self.active_loop, event);
					}

					let theme = self.active_loop.system_theme();
					if theme != self.last_theme {
						self.last_theme = theme;
						if let Some(theme) = theme {
							app.window_event(&self.active_loop, WindowEvent::ThemeChanged(theme));
						}
					}
				}
				MainEvent::LowMemory => {
					let event = EvloopEvent::MemoryWarning;
					app.evloop_event(&self.active_loop, event);
				}
				MainEvent::Start => {
					self.running = true;

					let event = WindowEvent::Occluded(false);
					app.window_event(&self.active_loop, event);
				}
				MainEvent::Resume => {
					app.resumed(&self.active_loop);
				}
				MainEvent::Pause => {
					app.paused(&self.active_loop);
				}
				MainEvent::Stop => {
					self.running = false;

					let event = WindowEvent::Occluded(true);
					app.window_event(&self.active_loop, event);
				}
				MainEvent::Destroy => {
					app.stopped(&self.active_loop);
					// The activity is gone: `android_main` has to return so the system can finish
					// destroying it (and possibly start a new instance).
					self.active_loop.exit();
				}
				MainEvent::SoftKeyboardVisibilityChanged => {
					let visible = self.android_app.soft_keyboard_visible();
					let event = WindowEvent::SoftKeyboardVisibilityChanged(visible);
					app.window_event(&self.active_loop, event);
				}
				MainEvent::TextInputChanged => {
					let (state, events) = crate::ime::on_text_input(&self.android_app);
					if let Some(state) = state {
						let event = WindowEvent::TextInputStateChanged(state);
						app.window_event(&self.active_loop, event);
					}
					for event in events {
						app.window_event(&self.active_loop, WindowEvent::Ime(event));
					}
				}
				MainEvent::EditorAction(action) => {
					let event = WindowEvent::ImeAction(action.into());
					app.window_event(&self.active_loop, event);
				}
			}
		}

		for input in self.android_app.take_input() {
			match input {
				InputEvent::Motion(motion) => self.handle_motion_event(&motion, app),
				InputEvent::Key(key) => self.handle_key_event(&key, app),
			}
		}

		// Events produced by `Window::request_ime_update` during this iteration (`Enabled`,
		// `Disabled`, ...), so they precede the editor events that follow the request.
		self.dispatch_pending_ime_events(app);

		if self.active_loop.event_loop_proxy.take_wake_up() {
			let event = EvloopEvent::ProxyWakeUp;
			app.evloop_event(&self.active_loop, event);
		}

		if self.running {
			if resized {
				let size = if let Some(native_window) = self.android_app.native_window().as_ref() {
					let width = native_window.width() as u32;
					let height = native_window.height() as u32;
					PhysicalSize::new(width, height)
				} else {
					PhysicalSize::new(0, 0)
				};
				let event = WindowEvent::Resized(size);
				app.window_event(&self.active_loop, event);
			}

			pending_redraw |= self.redraw_flag.get_and_reset();

			if pending_redraw {
				pending_redraw = false;
				let event = WindowEvent::RedrawRequested;
				app.window_event(&self.active_loop, event);
			}
		}

		app.exit_events(&self.active_loop);

		self.pending_redraw = pending_redraw;

		if main_event == Some(MainEvent::TerminateWindow) {
			// The application has released the surface; let `onNativeWindowDestroyed` return.
			self.android_app.window_terminated();
		}
	}

	fn handle_motion_event<A: ApplicationHandler>(
		&mut self,
		motion_event: &MotionEvent,
		app: &mut A,
	) {
		let device_id = Some(DeviceId::from_raw(motion_event.device_id() as i64));
		let action = motion_event.action();

		let pointers: Vec<_> = match action {
			MotionAction::Down
			| MotionAction::PointerDown
			| MotionAction::Up
			| MotionAction::PointerUp => {
				let index = motion_event.pointer_index();
				if index < motion_event.pointer_count() {
					vec![motion_event.pointer_at_index(index)]
				} else {
					Vec::new()
				}
			}
			MotionAction::Move | MotionAction::Cancel => motion_event.pointers().collect(),
			_ => Vec::new(),
		};

		for pointer in pointers {
			let tool_type = pointer.tool_type();
			let position = PhysicalPosition { x: pointer.x() as i32, y: pointer.y() as i32 };
			trace!(
				"Input event {device_id:?}, {action:?}, loc={position:?}, pointer={pointer:?}, \
				 tool_type={tool_type:?}"
			);
			let finger_id = FingerId::from_raw(pointer.pointer_id() as usize);
			let force = Some(Force::Normalized(pointer.pressure() as f64));

			match action {
				MotionAction::Down | MotionAction::PointerDown => {
					let primary = action == MotionAction::Down;
					if primary {
						self.primary_pointer = Some(finger_id);
					}
					let event = WindowEvent::PointerEntered {
						device_id,
						primary,
						position,
						kind: match tool_type {
							ToolType::Finger => PointerKind::Touch(finger_id),
							_ => PointerKind::Unknown,
						},
					};
					app.window_event(&self.active_loop, event);
					let event = WindowEvent::PointerButton {
						device_id,
						primary,
						state: ElementState::Pressed,
						position,
						button: match tool_type {
							ToolType::Finger => ButtonSource::Touch { finger_id, force },
							_ => ButtonSource::Unknown(0),
						},
					};
					app.window_event(&self.active_loop, event);
				}
				MotionAction::Move => {
					let primary = self.primary_pointer == Some(finger_id);
					let event = WindowEvent::PointerMoved {
						device_id,
						primary,
						position,
						source: match tool_type {
							ToolType::Finger => PointerSource::Touch { finger_id, force },
							_ => PointerSource::Unknown,
						},
					};
					app.window_event(&self.active_loop, event);
				}
				MotionAction::Up | MotionAction::PointerUp | MotionAction::Cancel => {
					let primary = action == MotionAction::Up
						|| (action == MotionAction::Cancel
							&& self.primary_pointer == Some(finger_id));

					if primary {
						self.primary_pointer = None;
					}

					if let MotionAction::Up | MotionAction::PointerUp = action {
						let event = WindowEvent::PointerButton {
							device_id,
							primary,
							state: ElementState::Released,
							position,
							button: match tool_type {
								ToolType::Finger => ButtonSource::Touch { finger_id, force },
								_ => ButtonSource::Unknown(0),
							},
						};
						app.window_event(&self.active_loop, event);
					}

					let event = WindowEvent::PointerLeft {
						device_id,
						primary,
						position: Some(position),
						kind: match tool_type {
							ToolType::Finger => PointerKind::Touch(finger_id),
							_ => PointerKind::Unknown,
						},
					};
					app.window_event(&self.active_loop, event);
				}
				_ => unreachable!(),
			}
		}
	}

	fn dispatch_pending_ime_events<A: ApplicationHandler>(&mut self, app: &mut A) {
		for event in crate::ime::take_pending(&self.android_app) {
			app.window_event(&self.active_loop, WindowEvent::Ime(event));
		}
	}

	fn handle_key_event<A: ApplicationHandler>(
		&mut self,
		key: &crate::glue::input::KeyEvent,
		app: &mut A,
	) {
		let keycode = key.key_code();
		let state = match key.action() {
			KeyAction::Down => ElementState::Pressed,
			_ => ElementState::Released,
		};

		let key_char = keycodes::character_map_and_combine_key(
			&self.android_app,
			key,
			&mut self.combining_accent,
		);

		let logical_key = keycodes::to_logical(key_char, keycode);
		let text = if state == ElementState::Pressed {
			logical_key.to_text().map(smol_str::SmolStr::new)
		} else {
			None
		};

		let event = WindowEvent::KeyboardInput {
			device_id: Some(DeviceId::from_raw(key.device_id() as i64)),
			event: KeyEvent {
				state,
				raw_key: keycode,
				physical_key: keycodes::to_physical_key(keycode),
				logical_key,
				location: keycodes::to_location(keycode),
				repeat: key.repeat_count() > 0,
				text: text.clone(),
				text_with_all_modifiers: text,
				key_without_modifiers: keycodes::to_logical(key_char, keycode),
			},
			is_synthetic: false,
		};

		app.window_event(&self.active_loop, event);
	}

	pub fn run_app<A: ApplicationHandler + 'static>(
		mut self,
		mut app: A,
	) -> Result<(), EventLoopError> {
		let result = self.run_app_on_demand(&mut app);
		drop(app);
		result
	}

	pub fn run_app_on_demand<A: ApplicationHandler>(
		&mut self,
		mut app: A,
	) -> Result<(), EventLoopError> {
		self.active_loop.clear_exit();
		loop {
			match self.pump_app_events(None, &mut app) {
				PumpStatus::Exit(0) => {
					break Ok(());
				}
				PumpStatus::Exit(code) => {
					break Err(EventLoopError::ExitFailure(code));
				}
				_ => {
					continue;
				}
			}
		}
	}

	pub fn pump_app_events<A: ApplicationHandler>(
		&mut self,
		timeout: Option<Duration>,
		mut app: A,
	) -> PumpStatus {
		if !self.loop_running {
			self.loop_running = true;

			self.pending_redraw = false;
			self.cause = StartCause::Init;

			self.single_iter(None, &mut app);
		}

		if !self.exiting() {
			self.poll_events_with_timeout(timeout, &mut app);
		}
		if self.exiting() {
			self.loop_running = false;

			PumpStatus::Exit(0)
		} else {
			PumpStatus::Continue
		}
	}

	fn poll_events_with_timeout<A: ApplicationHandler>(
		&mut self,
		mut timeout: Option<Duration>,
		app: &mut A,
	) {
		let start = Instant::now();

		self.pending_redraw |= self.redraw_flag.get_and_reset();

		timeout = if self.running
			&& (self.pending_redraw || self.active_loop.event_loop_proxy.is_woken())
		{
			Some(Duration::ZERO)
		} else {
			let control_flow_timeout = match self.control_flow() {
				ControlFlow::Wait => None,
				ControlFlow::Poll => Some(Duration::ZERO),
				ControlFlow::WaitUntil(wait_deadline) => {
					Some(wait_deadline.saturating_duration_since(start))
				}
			};

			min_timeout(control_flow_timeout, timeout)
		};

		let timed_out = self.android_app.wait(timeout);

		self.cause = match self.control_flow() {
			ControlFlow::Poll => StartCause::Poll,
			ControlFlow::Wait => StartCause::WaitCancelled { start, requested_resume: None },
			ControlFlow::WaitUntil(deadline) => {
				if Instant::now() < deadline {
					StartCause::WaitCancelled { start, requested_resume: Some(deadline) }
				} else {
					StartCause::ResumeTimeReached { start, requested_resume: deadline }
				}
			}
		};

		// One iteration per main event, in order.
		let mut handled_main_event = false;
		while let Some(event) = self.android_app.take_main_event() {
			handled_main_event = true;
			self.single_iter(Some(event), app);
			if self.exiting() {
				return;
			}
		}

		if handled_main_event {
			return;
		}

		// Woken without a main event: iterate only if there is something to do.
		self.pending_redraw |= self.redraw_flag.get_and_reset();
		let proxy_woken = self.active_loop.event_loop_proxy.is_woken();
		if timed_out
			|| self.android_app.has_pending_input()
			|| (self.running && (self.pending_redraw || proxy_woken))
		{
			self.single_iter(None, app);
		}
	}

	fn control_flow(&self) -> ControlFlow {
		self.active_loop.control_flow()
	}

	pub fn set_control_flow(&self, control_flow: ControlFlow) {
		self.active_loop.set_control_flow(control_flow);
	}

	pub fn create_proxy(&self) -> EventLoopProxy {
		self.active_loop.create_proxy()
	}

	#[inline]
	pub fn builder() -> EventLoopBuilder {
		EventLoopBuilder::default()
	}

	fn exiting(&self) -> bool {
		self.active_loop.exiting()
	}
}

#[derive(Default)]
pub struct EventLoopBuilder {
	attributes: EventLoopAttributes,
}

impl EventLoopBuilder {
	#[inline]
	pub fn build(&mut self, app: AndroidApp) -> Result<EventLoop, EventLoopError> {
		self.attributes.android_app = Some(app);

		EventLoop::new(&self.attributes)
	}
}

pub struct ActiveEventLoop {
	pub(crate) app: AndroidApp,
	control_flow: Cell<ControlFlow>,
	exit: Cell<bool>,
	pub(crate) redraw_requester: RedrawRequester,
	event_loop_proxy: EventLoopProxy,
}

impl ActiveEventLoop {
	fn clear_exit(&self) {
		self.exit.set(false);
	}

	pub fn exit(&self) {
		self.exit.set(true);
	}

	pub fn exiting(&self) -> bool {
		self.exit.get()
	}

	pub fn set_control_flow(&self, control_flow: ControlFlow) {
		self.control_flow.set(control_flow);
	}

	pub fn control_flow(&self) -> ControlFlow {
		self.control_flow.get()
	}

	pub fn system_theme(&self) -> Option<Theme> {
		match self.app.config().ui_mode_night() {
			UiModeNight::No => Some(Theme::Light),
			UiModeNight::Yes => Some(Theme::Dark),
			_ => None,
		}
	}

	/// The `AndroidApp` this event loop runs on.
	pub fn android_app(&self) -> &AndroidApp {
		&self.app
	}

	pub fn create_proxy(&self) -> EventLoopProxy {
		self.event_loop_proxy.clone()
	}
}

/// Wakes the event loop from any thread; the application then receives
/// `EvloopEvent::ProxyWakeUp`.
#[derive(Debug, Clone)]
pub struct EventLoopProxy {
	inner: Arc<ProxyInner>,
}

#[derive(Debug)]
struct ProxyInner {
	wake_up: AtomicBool,
	waker: AndroidAppWaker,
}

impl EventLoopProxy {
	fn new(waker: AndroidAppWaker) -> Self {
		Self { inner: Arc::new(ProxyInner { wake_up: AtomicBool::new(false), waker }) }
	}

	pub fn wake_up(&self) {
		self.inner.wake_up.store(true, Ordering::Release);
		self.inner.waker.wake();
	}

	fn is_woken(&self) -> bool {
		self.inner.wake_up.load(Ordering::Acquire)
	}

	fn take_wake_up(&self) -> bool {
		self.inner.wake_up.swap(false, Ordering::AcqRel)
	}
}

#[derive(Default)]
pub struct EventLoopAttributes {
	pub android_app: Option<AndroidApp>,
}
