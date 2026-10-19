extern crate core;

pub mod capability;
pub mod cursor;
pub mod error;
pub mod event;
pub mod event_loop;
pub mod geometry;
pub mod keyboard;

use std::time::{Duration, Instant};

pub enum Platform {
    Win32,
    Android,
}

#[derive(Default, Copy, Clone)]
pub enum ControlFlow {
    Poll,
    #[default]
    Wait,
    WaitUntil(Instant),
}

impl ControlFlow {
    pub fn wait_duration(timeout: Duration) -> Self {
        match Instant::now().checked_add(timeout) {
            Some(instant) => Self::WaitUntil(instant),
            None => Self::Wait,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum StartCause {
    ResumeTimeReached {
        start: Instant,
        requested_resume: Instant,
    },
    WaitCancelled {
        start: Instant,
        requested_resume: Option<Instant>,
    },

    Poll,
    Init,
}

pub enum PumpStatus {
    Continue,
    Exit(i32),
}

pub enum Theme {
    Light,
    Dark,
}
