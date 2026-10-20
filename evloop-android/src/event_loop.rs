use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use android_activity::input::{InputEvent, KeyAction, Keycode, MotionAction};
use android_activity::ndk::configuration::UiModeNight;
use tracing::trace;
use evloop_core::error::{CreateWindowError, EventLoopError};
use evloop_core::event::{ButtonSource, DeviceId, ElementState, EvloopEvent, FingerId, Force, KeyEvent, PointerKind, PointerSource, WindowEvent};
use evloop_core::{Platform};
use evloop_core::capability::{CanExit, ControlFlowAvailable, PlatformAvailable, SystemThemeAvailable};
use evloop_core::event_loop::{ControlFlow, StartCause};
use evloop_core::geometry::{PhysicalPosition, PhysicalSize};
use evloop_core::window::Theme;
use crate::{content_rect, RedrawRequester, SharedFlag, HAS_FOCUS, scale_factor, keycodes, min_timeout, platform};
use crate::application::ApplicationHandler;
use crate::capability::CanGetWindow;
use crate::lifecycle::{Paused, Resumed, Running, Started, Stopped, SurfaceAvailable, SurfaceLost};
use crate::window::Window;

pub enum PumpStatus {
    Continue,
    Exit(i32),
}

pub struct EventLoop {
    android_app: AndroidApp,
    active_loop: ActiveEventLoop,
    cause: StartCause,

    loop_running: bool,

    pending_redraw: bool,
    redraw_flag: SharedFlag,
    running: bool,

    primary_pointer: Option<FingerId>,

    ignore_volume_keys: bool,
    combining_accent: Option<char>,
}

impl EventLoop {
    pub fn new(attributes: &EventLoopAttributes) -> Result<Self, EventLoopError> {
        let android_app = attributes
            .android_app
            .as_ref()
            .expect("`AndroidApp` is required to create `EventLoop` on Android platform");

        let event_loop_proxy = Arc::new(EventLoopProxy::new(android_app.create_waker()));

        let redraw_flag = SharedFlag::new();

        Ok(Self {
            android_app: android_app.clone(),
            active_loop: ActiveEventLoop {
                app: android_app.clone(),
                control_flow: Cell::new(ControlFlow::default()),
                exit: Cell::new(false),
                redraw_requester: RedrawRequester::new(&redraw_flag, android_app.create_waker()),
                event_loop_proxy,
            },
            cause: StartCause::Init,

            loop_running: false,

            pending_redraw: false,
            redraw_flag,
            running: false,

            primary_pointer: None,

            ignore_volume_keys: true,
            combining_accent: None,
        })
    }

    fn single_iter<A: ApplicationHandler>(
        &mut self,
        main_event: Option<MainEvent<'_>>,
        app: &mut A,
    ) {
        trace!("Mainloop Iter");

        let cause = self.cause;
        let mut resized = false;
        let mut pending_redraw = self.pending_redraw;

        app.enter_events(&self.active_loop, cause);

        if let Some(event) = main_event {
            trace!("Handling MainEvent: {:?}", event);

            match event {
                MainEvent::InitWindow { .. } => {
                    app.surface_available(&self.active_loop);
                }
                MainEvent::TerminateWindow { .. } => {
                    app.surface_lost(&self.active_loop);
                }
                MainEvent::WindowResized { .. } => {
                    resized = true;
                }
                MainEvent::RedrawNeeded { .. } => {
                    pending_redraw = true;
                }
                MainEvent::ContentRectChanged { .. } => {
                    let content_rect = content_rect(&self.android_app);

                    let event = WindowEvent::ContentRectChanged(content_rect);
                    app.window_event(&self.active_loop, event);
                }
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
                MainEvent::ConfigChanged { .. } => {
                    let scale_factor = scale_factor(&self.android_app);

                    let event = WindowEvent::ScaleFactorChanged(scale_factor);
                    app.window_event(&self.active_loop, event);
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
                MainEvent::Resume {
                    loader: _loader, ..
                } => {
                    app.resumed(&self.active_loop);
                }
                MainEvent::SaveState { .. } => {}
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
                }
                MainEvent::InsetsChanged { .. } => {}
                _ => {}
            }
        }

        let android_app = self.android_app.clone();

        match android_app.input_events_iter() {
            Ok(mut input_iter) => loop {
                let read_event =
                    input_iter.next(|event| self.handle_input_event(&android_app, event, app));

                if !read_event {
                    break;
                }
            },
            Err(_err) => {}
        }

        if self
            .active_loop
            .event_loop_proxy
            .wake_up
            .swap(false, Ordering::Relaxed)
        {
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
                let event = WindowEvent::SurfaceResized(size);
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
    }

    fn handle_input_event<A: ApplicationHandler>(
        &mut self,
        android_app: &AndroidApp,
        event: &InputEvent,
        app: &mut A,
    ) -> InputStatus {
        let mut input_status = InputStatus::Handled;
        match event {
            InputEvent::MotionEvent(motion_event) => {
                let device_id = Some(DeviceId::from_raw(motion_event.device_id() as i64));
                let action = motion_event.action();

                let pointers: Option<Box<dyn Iterator<Item = platform::activity::input::Pointer>>> =
                    match action {
                        MotionAction::Down
                        | MotionAction::PointerDown
                        | MotionAction::Up
                        | MotionAction::PointerUp => Some(Box::new(std::iter::once(
                            motion_event.pointer_at_index(motion_event.pointer_index()),
                        ))),
                        MotionAction::Move | MotionAction::Cancel => {
                            Some(Box::new(motion_event.pointers()))
                        }
                        _ => None,
                    };

                for pointer in pointers.into_iter().flatten() {
                    let tool_type = pointer.tool_type();
                    let position = PhysicalPosition {
                        x: pointer.x() as i32,
                        y: pointer.y() as i32,
                    };
                    trace!(
                        "Input event {device_id:?}, {action:?}, loc={position:?}, \
                    pointer={pointer:?}, tool_type={tool_type:?}"
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
                                    platform::activity::input::ToolType::Finger => {
                                        PointerKind::Touch(finger_id)
                                    }
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
                                    platform::activity::input::ToolType::Finger => {
                                        ButtonSource::Touch { finger_id, force }
                                    }
                                    _ => ButtonSource::Unknown(0),
                                },
                                is_macos_activation_click: false,
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
                                    platform::activity::input::ToolType::Finger => {
                                        PointerSource::Touch { finger_id, force }
                                    }
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
                                        platform::activity::input::ToolType::Finger => {
                                            ButtonSource::Touch { finger_id, force }
                                        }
                                        _ => ButtonSource::Unknown(0),
                                    },
                                    is_macos_activation_click: false,
                                };
                                app.window_event(&self.active_loop, event);
                            }

                            let event = WindowEvent::PointerLeft {
                                device_id,
                                primary,
                                position: Some(position),
                                kind: match tool_type {
                                    platform::activity::input::ToolType::Finger => {
                                        PointerKind::Touch(finger_id)
                                    }
                                    _ => PointerKind::Unknown,
                                },
                            };
                            app.window_event(&self.active_loop, event);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            InputEvent::KeyEvent(key) => match key.key_code() {
                Keycode::VolumeUp | Keycode::VolumeDown | Keycode::VolumeMute
                if self.ignore_volume_keys =>
                    {
                        input_status = InputStatus::Unhandled
                    }
                keycode => {
                    let state = match key.action() {
                        KeyAction::Down => ElementState::Pressed,
                        KeyAction::Up => ElementState::Released,
                        _ => ElementState::Released,
                    };

                    let key_char = keycodes::character_map_and_combine_key(
                        android_app,
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
            },
            _ => {}
        }

        input_status
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
            && (self.pending_redraw
            || self
            .active_loop
            .event_loop_proxy
            .wake_up
            .load(Ordering::Relaxed))
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

        let android_app = self.android_app.clone();
        android_app.poll_events(timeout, |poll_event| {
            let mut main_event = None;

            match poll_event {
                PollEvent::Wake => {
                    self.pending_redraw |= self.redraw_flag.get_and_reset();
                    if !self.running
                        || (!self.pending_redraw
                        && !self
                        .active_loop
                        .event_loop_proxy
                        .wake_up
                        .load(Ordering::Relaxed))
                    {
                        return;
                    }
                }
                PollEvent::Timeout => {}
                PollEvent::Main(event) => {
                    main_event = Some(event);
                }
                unknown_event => {
                    // TODO: Handling
                }
            }

            self.cause = match self.control_flow() {
                ControlFlow::Poll => StartCause::Poll,
                ControlFlow::Wait => StartCause::WaitCancelled {
                    start,
                    requested_resume: None,
                },
                ControlFlow::WaitUntil(deadline) => {
                    if Instant::now() < deadline {
                        StartCause::WaitCancelled {
                            start,
                            requested_resume: Some(deadline),
                        }
                    } else {
                        StartCause::ResumeTimeReached {
                            start,
                            requested_resume: deadline,
                        }
                    }
                }
            };

            self.single_iter(main_event, app);
        })
    }

    fn control_flow(&self) -> ControlFlow {
        self.active_loop.control_flow()
    }

    pub fn set_control_flow(&self, control_flow: ControlFlow) {
        self.active_loop.set_control_flow(control_flow);
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
        Ok(EventLoop::new(&self.attributes)?)
    }
}

pub struct ActiveEventLoop {
    pub(crate) app: AndroidApp,
    control_flow: Cell<ControlFlow>,
    exit: Cell<bool>,
    pub(crate) redraw_requester: RedrawRequester,
    event_loop_proxy: Arc<EventLoopProxy>,
}

impl ActiveEventLoop {
    fn clear_exit(&self) {
        self.exit.set(false);
    }
}

impl CanGetWindow for ActiveEventLoop {
    fn get_window(&self) -> Result<Window, CreateWindowError> {
        Ok(Window::new(self)?)
    }
}

impl CanExit for ActiveEventLoop {
    fn exit(&self) {
        self.exit.set(true);
    }

    fn exiting(&self) -> bool {
        self.exit.get()
    }
}

impl ControlFlowAvailable for ActiveEventLoop {
    fn set_control_flow(&self, control_flow: ControlFlow) {
        self.control_flow.set(control_flow);
    }

    fn control_flow(&self) -> ControlFlow {
        self.control_flow.get()
    }
}

impl SystemThemeAvailable for ActiveEventLoop {
    fn system_theme(&self) -> Option<Theme> {
        match self.app.config().ui_mode_night() {
            UiModeNight::No => Some(Theme::Light),
            UiModeNight::Yes => Some(Theme::Dark),
            _ => None,
        }
    }
}

impl PlatformAvailable for ActiveEventLoop {
    fn platform(&self) -> Platform {
        Platform::Android
    }
}

impl Started for ActiveEventLoop {}

impl Stopped for ActiveEventLoop {}

impl SurfaceAvailable for ActiveEventLoop {}

impl SurfaceLost for ActiveEventLoop {}

impl Resumed for ActiveEventLoop {}

impl Paused for ActiveEventLoop {}

impl Running for ActiveEventLoop {}

pub struct EventLoopProxy {
    wake_up: AtomicBool,
    waker: AndroidAppWaker,
}

impl EventLoopProxy {
    fn new(waker: AndroidAppWaker) -> Self {
        Self {
            wake_up: AtomicBool::new(false),
            waker,
        }
    }
}

pub struct EventLoopAttributes {
    pub android_app: Option<AndroidApp>,
}

impl Default for EventLoopAttributes {
    fn default() -> Self {
        Self { android_app: None }
    }
}