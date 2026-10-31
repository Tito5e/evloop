use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceEventStrategy {
	Always,
	#[default]
	WhenFocused,
	Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpStatus {
	Continue,
	Exit(i32),
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartCause {
	ResumeTimeReached { start: Instant, requested_resume: Instant },
	WaitCancelled { start: Instant, requested_resume: Option<Instant> },
	Poll,
	Init,
}
