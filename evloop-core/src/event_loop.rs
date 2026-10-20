use std::time::{Duration, Instant};

pub enum DeviceEventStrategy {
	Always,
	WhenFocused,
	Never,
}

pub enum PumpStatus {
	Continue,
	Exit(i32),
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
	ResumeTimeReached { start: Instant, requested_resume: Instant },
	WaitCancelled { start: Instant, requested_resume: Option<Instant> },

	Poll,
	Init,
}
