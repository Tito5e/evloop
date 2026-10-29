use core::{fmt, hash};
use std::{ffi::c_void, ptr};

pub struct EventLoopAttributes {
	pub any_thread: bool,
	pub dpi_aware: bool,
	pub msg_hook: Option<Box<dyn FnMut(*const c_void) -> bool + 'static>>,
}

impl fmt::Debug for EventLoopAttributes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("EventLoopAttributes")
			.field("any_thread", &self.any_thread)
			.field("dpi_aware", &self.dpi_aware)
			.finish_non_exhaustive()
	}
}

impl Default for EventLoopAttributes {
	fn default() -> Self {
		Self { any_thread: false, dpi_aware: true, msg_hook: None }
	}
}

impl PartialEq for EventLoopAttributes {
	fn eq(&self, other: &Self) -> bool {
		self.any_thread.eq(&other.any_thread)
			&& self.dpi_aware.eq(&other.dpi_aware)
			&& match (&self.msg_hook, &other.msg_hook) {
				(Some(this), Some(other)) => ptr::eq(&this, &other),
				(None, None) => true,
				_ => false,
			}
	}
}

impl Eq for EventLoopAttributes {}

impl hash::Hash for EventLoopAttributes {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.any_thread.hash(state);
		self.dpi_aware.hash(state);
		ptr::hash(&self.msg_hook, state);
	}
}
