use std::time::{Duration, Instant};

/// When device events (`ApplicationHandler::device_event`) are delivered.
///
/// Passed to `set_device_event_strategy` on backends that have device events. The device event type
/// itself is backend-specific.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceEventStrategy {
	/// Deliver device events regardless of window focus.
	Always,
	/// Deliver device events only while one of the application's windows has focus.
	#[default]
	WhenFocused,
	/// Never deliver device events.
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

/// A token identifying one asynchronous request, so that its result event can be matched with
/// the call that started it.
///
/// Every call that returns an `AsyncRequestSerial` returns a value never returned before during
/// the lifetime of the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsyncRequestSerial(u64);

impl AsyncRequestSerial {
	/// Allocates a new, unique serial. Intended for backends.
	pub fn next() -> Self {
		use std::sync::atomic::{AtomicU64, Ordering};
		static CURRENT: AtomicU64 = AtomicU64::new(0);
		Self(CURRENT.fetch_add(1, Ordering::Relaxed))
	}

	pub const fn into_raw(self) -> u64 {
		self.0
	}
}
