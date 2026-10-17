pub mod application;
pub mod keycodes;

pub use android_activity as activity;

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use android_activity::input::{KeyAction, Keycode, MotionAction};
use tracing::trace;
use evloop_core::error::EventLoopError;
use evloop_core::{ControlFlow, StartCause};
use evloop_core::event::{ButtonSource, DeviceId, ElementState, FingerId, Force, PointerKind, PointerSource, WindowEvent};
use evloop_core::geometry::{PhysicalInsets, PhysicalPosition, PhysicalSize};
use crate::activity::{AndroidApp, MainEvent, AndroidAppWaker, input::InputEvent, InputStatus};
use crate::application::ApplicationHandler;

static HAS_FOCUS: AtomicBool = AtomicBool::new(true);

struct SharedFlagSetter {
    flag: Arc<AtomicBool>,
}

impl SharedFlagSetter {
    fn set(&self) -> bool {
        self.flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok()
    }
}

struct SharedFlag {
    flag: Arc<AtomicBool>,
}

impl SharedFlag {
    fn new() -> Self {
        Self { flag: Arc::new(AtomicBool::new(false)), }
    }

    fn setter(&self) -> SharedFlagSetter {
        SharedFlagSetter { flag: self.flag.clone() }
    }

    fn get_and_reset(&self) -> bool {
        self.flag.swap(false, Ordering::AcqRel)
    }
}

pub struct EventLoop {
    android_app: AndroidApp,
    active_loop: ActiveEventLoop,
    cause: StartCause,

    pending_redraw: bool,
    redraw_flag: SharedFlag,
    running: bool,

    primary_pointer: Option<FingerId>,
}

impl EventLoop {
    pub fn new(attributes: &EventLoopAttributes) -> Result<Self, EventLoopError> {
        let android_app = attributes.android_app.as_ref().expect("`AndroidApp` is required to create `EventLoop` on Android platform");

        let event_loop_proxy = Arc::new(EventLoopProxy::new(android_app.create_waker()));

        let redraw_flag = SharedFlag::new();

        Ok(Self {
            android_app: android_app.clone(),
            active_loop: ActiveEventLoop {
                app: android_app.clone(),
                control_flow: Cell::new(ControlFlow::default()),
                exit: Cell::new(false),
                event_loop_proxy,
            },
            cause: StartCause::Init,

            pending_redraw: false,
            redraw_flag,
            running: false,

            primary_pointer: None,
        })
    }

    fn single_iter<A: ApplicationHandler>(
        &mut self,
        main_event: Option<MainEvent<'_>>,
        app: &mut A
    ) {
        trace!("Mainloop Iter");

        let cause = self.cause;
        let mut resized = false;
        let mut pending_redraw = self.pending_redraw;

        app.new_events(&self.active_loop, cause);

        if let Some(event) = main_event {
            trace!("Handling MainEvent: {:?}", event);

            match event {
                MainEvent::InitWindow { .. } => {
                    app.can_create_surfaces(&self.active_loop);
                },
                MainEvent::TerminateWindow { .. } => {
                    app.destroy_surfaces(&self.active_loop);
                },
                MainEvent::WindowResized { .. } => {
                    resized = true;
                },
                MainEvent::RedrawNeeded { .. } => {
                    pending_redraw = true;
                },
                MainEvent::ContentRectChanged { .. } => {
                    let content_rect = content_rect(&self.android_app);

                    let event = WindowEvent::ContentRectChanged(content_rect);
                    app.window_event(&self.active_loop, event);
                },
                MainEvent::GainedFocus => {
                    HAS_FOCUS.store(true, Ordering::Relaxed);
                    let event = WindowEvent::Focused(true);
                    app.window_event(&self.active_loop, event);
                },
                MainEvent::LostFocus => {
                    HAS_FOCUS.store(false, Ordering::Relaxed);
                    let event = WindowEvent::Focused(false);
                    app.window_event(&self.active_loop, event);
                },
                MainEvent::ConfigChanged { .. } => {
                    let scale_factor = scale_factor(&self.android_app);

                    let event = WindowEvent::ScaleFactorChanged(scale_factor);
                    app.window_event(&self.active_loop, event);
                },
                MainEvent::LowMemory => {
                    app.memory_warning(&self.active_loop);
                },
                MainEvent::Start => {
                    self.running = true;
                    app.resumed(&self.active_loop);
                },
                MainEvent::Resume { loader, .. } => {
                    let event = WindowEvent::Occluded(false);
                    app.window_event(&self.active_loop, event);
                },
                MainEvent::SaveState { .. } => {
                },
                MainEvent::Pause => {
                    let event = WindowEvent::Occluded(true);
                    app.window_event(&self.active_loop, event);
                },
                MainEvent::Stop => {
                    self.running = false;
                    app.suspended(&self.active_loop);
                },
                MainEvent::Destroy => {
                },
                MainEvent::InsetsChanged { .. } => {
                },
                _ => {}
            }
        }

        let android_app = self.android_app.clone();

        match android_app.input_events_iter() {
            Ok(mut input_iter) => loop {
                let read_event = input_iter.next(|event| self.handle_input_event(&android_app, event, app));

                if !read_event {
                    break;
                }
            },
            Err(err) => {
            }
        }

        if self.active_loop.event_loop_proxy.wake_up.swap(false, Ordering::Relaxed) {
            app.proxy_wake_up(&self.active_loop);
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
        }
    }

    fn handle_input_event<A: ApplicationHandler>(&mut self, android_app: &AndroidApp, event: &InputEvent, app: &mut A) -> InputStatus {
        let mut input_status = InputStatus::Handled;
        match event {
            InputEvent::MotionEvent(motion_event) => {
                let device_id = Some(DeviceId::from_raw(motion_event.device_id() as i64));
                let action = motion_event.action();

                let pointers: Option<Box<dyn Iterator<Item = activity::input::Pointer>>> = match action {
                    MotionAction::Down | MotionAction::PointerDown | MotionAction::Up | MotionAction::PointerUp => Some(Box::new(std::iter::once(
                        motion_event.pointer_at_index(motion_event.pointer_index())
                    ))),
                    MotionAction::Move | MotionAction::Cancel => {
                        Some(Box::new(motion_event.pointers()))
                    },
                    _ => None,
                };

                for pointer in pointers.into_iter().flatten() {
                    let tool_type = pointer.tool_type();
                    let position = PhysicalPosition {
                        x: pointer.x() as i32,
                        y: pointer.y() as i32,
                    };
                    trace!("Input event {device_id:?}, {action:?}, loc={position:?}, \
                    pointer={pointer:?}, tool_type={tool_type:?}");
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
                                    activity::input::ToolType::Finger => {
                                        PointerKind::Touch(finger_id)
                                    },
                                    _ => PointerKind::Unknown
                                }
                            };
                            app.window_event(&self.active_loop, event);
                            let event = WindowEvent::PointerButton {
                                device_id,
                                primary,
                                state: ElementState::Pressed,
                                position,
                                button: match tool_type {
                                    activity::input::ToolType::Finger => {
                                        ButtonSource::Touch { finger_id, force }
                                    },
                                    _ => ButtonSource::Unknown(0)
                                },
                                is_macos_activation_click: false,
                            };
                            app.window_event(&self.active_loop, event);
                        },
                        MotionAction::Move => {
                            let primary = self.primary_pointer == Some(finger_id);
                            let event = WindowEvent::PointerMoved {
                                device_id,
                                primary,
                                position,
                                source: match tool_type {
                                    activity::input::ToolType::Finger => {
                                        PointerSource::Touch { finger_id, force }
                                    },
                                    _ => PointerSource::Unknown,
                                }
                            };
                            app.window_event(&self.active_loop, event);
                        },
                        MotionAction::Up | MotionAction::PointerUp | MotionAction::Cancel => {
                            let primary = action == MotionAction::Up || (action == MotionAction::Cancel && self.primary_pointer == Some(finger_id));

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
                                        activity::input::ToolType::Finger => {
                                            ButtonSource::Touch { finger_id, force }
                                        },
                                        _ => ButtonSource::Unknown(0)
                                    },
                                    is_macos_activation_click: false
                                };
                                app.window_event(&self.active_loop, event);
                            }

                            let event = WindowEvent::PointerLeft {
                                device_id,
                                primary,
                                position: Some(position),
                                kind: match tool_type {
                                    activity::input::ToolType::Finger => {
                                        PointerKind::Touch(finger_id)
                                    },
                                    _ => PointerKind::Unknown,
                                }
                            };
                            app.window_event(&self.active_loop, event);
                        },
                        _ => unreachable!(),
                    },
                }
            },
            InputEvent::KeyEvent(key) => {
                match key.key_code() {
                    Keycode::VolumeUp | Keycode::VolumeDown | Keycode::VolumeMute if self.ignore_volume_keys => {
                        input_status = InputStatus::Unhandled
                    },
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
                            is_synthetic: false
                        };

                        app.window_event(&self.active_loop, event);
                    }
                }
            },
            _ => {

            }
        }

        input_status
    }
}

pub struct ActiveEventLoop {
    app: AndroidApp,
    control_flow: Cell<ControlFlow>,
    exit: Cell<bool>,
    event_loop_proxy: Arc<EventLoopProxy>,
}

pub struct EventLoopProxy {
    wake_up: AtomicBool,
    waker: AndroidAppWaker,
}

pub struct EventLoopAttributes {
    pub android_app: Option<AndroidApp>,
}

impl Default for EventLoopAttributes {
    fn default() -> Self {
        Self { android_app: None }
    }
}

fn content_rect(app: &AndroidApp) -> PhysicalInsets {
    let content_rect = app.content_rect();

    PhysicalInsets {
        bottom: content_rect.bottom as u32,
        left: content_rect.left as u32,
        top: content_rect.top as u32,
        right: content_rect.right as u32,
    }
}

fn scale_factor(app: &AndroidApp) -> f64 {
    app.config().density().map(|dpi| dpi as f64 / 160.).unwrap_or(1.0)
}
