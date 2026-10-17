pub mod application;
pub mod event;

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
pub use android_activity as activity;
use tracing::trace;
use evloop_core::error::EventLoopError;
use evloop_core::{ControlFlow, StartCause};
use evloop_core::event::WindowEvent;
use crate::activity::{AndroidApp, MainEvent};
use crate::application::ApplicationHandler;
use crate::event::PlatformEvent;

static HAS_FOCUS: AtomicBool = AtomicBool::new(true);

pub struct EventLoop {
    android_app: AndroidApp,
    active_loop: ActiveEventLoop,
    cause: StartCause,
}

const GLOBAL_WINDOW: WindowId = WindowId::from_raw(0);

impl EventLoop {
    pub fn new(attributes: &EventLoopAttributes) -> Result<Self, EventLoopError> {
        let android_app = attributes.android_app.as_ref().expect("`AndroidApp` is required to create `EventLoop` on Android platform");

        Ok(Self {
            android_app: android_app.clone(),
            active_loop: ActiveEventLoop {
                app: android_app.clone(),
                control_flow: Cell::new(ControlFlow::default()),
                exit: Cell::new(false),
            },
            cause: StartCause::Init,
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
                MainEvent::InitWindow => {
                    app.can_create_surfaces(&self.active_loop);
                },
                MainEvent::TerminateWindow => {
                    app.destroy_surfaces(&self.active_loop);
                },
                MainEvent::WindowResized => {
                    resized = true;
                },
                MainEvent::RedrawNeeded => {
                    pending_redraw = true;
                },
                MainEvent::ContentRectChanged => {
                    let event = PlatformEvent::ContentRectChanged;
                    app.platform_event(&self.active_loop, GLOBAL_WINDOW, event);
                },
                MainEvent::GainedFocus => {
                    HAS_FOCUS.store(true, Ordering::Relaxed);
                    let event = WindowEvent::Focused(true);
                    app.window_event(&self.active_loop, GLOBAL_WINDOW, event);
                },
                MainEvent::LostFocus => {
                    HAS_FOCUS.store(false, Ordering::Relaxed);
                    let event = WindowEvent::Focused(false);
                    app.window_event(&self.active_loop, GLOBAL_WINDOW, event);
                },
                MainEvent::ConfigChanged => {
                    let scale_factor = scale_factor(&self.android_app);

                    let event = WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        suggested_size: None,
                    };
                    app.window_event(&self.active_loop, GLOBAL_WINDOW, event);
                }
            }
        }
    }
}

pub struct ActiveEventLoop {
    app: AndroidApp,
    control_flow: Cell<ControlFlow>,
    exit: Cell<bool>,
}

pub struct EventLoopAttributes {
    pub android_app: Option<AndroidApp>,
}

impl Default for EventLoopAttributes {
    fn default() -> Self {
        Self { android_app: None }
    }
}
