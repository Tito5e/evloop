//! Shared helpers of the Android backend.

use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use crate::glue::{AndroidApp, AndroidAppWaker};

pub(crate) static HAS_FOCUS: AtomicBool = AtomicBool::new(true);

#[derive(Clone)]
pub(crate) struct SharedFlagSetter {
	flag: Arc<AtomicBool>,
}

impl SharedFlagSetter {
	pub(crate) fn set(&self) -> bool {
		self.flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok()
	}
}

pub(crate) struct SharedFlag {
	flag: Arc<AtomicBool>,
}

impl SharedFlag {
	pub(crate) fn new() -> Self {
		Self { flag: Arc::new(AtomicBool::new(false)) }
	}

	pub(crate) fn setter(&self) -> SharedFlagSetter {
		SharedFlagSetter { flag: self.flag.clone() }
	}

	pub(crate) fn get_and_reset(&self) -> bool {
		self.flag.swap(false, Ordering::AcqRel)
	}
}

#[derive(Clone)]
pub(crate) struct RedrawRequester {
	flag: SharedFlagSetter,
	waker: AndroidAppWaker,
}

impl RedrawRequester {
	pub(crate) fn new(flag: &SharedFlag, waker: AndroidAppWaker) -> Self {
		Self { flag: flag.setter(), waker }
	}

	pub(crate) fn request_redraw(&self) {
		if self.flag.set() {
			self.waker.wake();
		}
	}
}

pub(crate) fn scale_factor(app: &AndroidApp) -> f64 {
	app.config().density().map(|dpi| dpi as f64 / 160.).unwrap_or(1.0)
}

pub(crate) fn min_timeout(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
	a.map_or(b, |a_timeout| b.map_or(Some(a_timeout), |b_timeout| Some(a_timeout.min(b_timeout))))
}
