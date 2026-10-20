pub mod application;
pub mod capability;
pub mod keycodes;
pub mod lifecycle;
pub mod window;
pub mod event_loop;
pub mod platform;

use crate::platform::activity::{AndroidApp, AndroidAppWaker};
use evloop_core::geometry::{PhysicalInsets};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration};

static HAS_FOCUS: AtomicBool = AtomicBool::new(true);

#[derive(Clone)]
struct SharedFlagSetter {
    flag: Arc<AtomicBool>,
}

impl SharedFlagSetter {
    fn set(&self) -> bool {
        self.flag
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }
}

struct SharedFlag {
    flag: Arc<AtomicBool>,
}

impl SharedFlag {
    fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    fn setter(&self) -> SharedFlagSetter {
        SharedFlagSetter {
            flag: self.flag.clone(),
        }
    }

    fn get_and_reset(&self) -> bool {
        self.flag.swap(false, Ordering::AcqRel)
    }
}

#[derive(Clone)]
struct RedrawRequester {
    flag: SharedFlagSetter,
    waker: AndroidAppWaker,
}

impl RedrawRequester {
    fn new(flag: &SharedFlag, waker: AndroidAppWaker) -> Self {
        Self {
            flag: flag.setter(),
            waker,
        }
    }

    fn request_redraw(&self) {
        if self.flag.set() {
            self.waker.wake();
        }
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
    app.config()
        .density()
        .map(|dpi| dpi as f64 / 160.)
        .unwrap_or(1.0)
}

fn min_timeout(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    a.map_or(b, |a_timeout| {
        b.map_or(Some(a_timeout), |b_timeout| Some(a_timeout.min(b_timeout)))
    })
}
