use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::UI::WindowsAndMessaging::RegisterWindowMessageA;

pub struct LazyMessageId {
	id: AtomicU32,
	name: &'static str,
}

impl LazyMessageId {
	const fn new(name: &'static str) -> Self {
		Self { id: AtomicU32::new(INVALID_ID), name }
	}

	pub fn get(&self) -> u32 {
		// Load the ID.
		let id = self.id.load(Ordering::Relaxed);

		if id != INVALID_ID {
			return id;
		}

		assert!(self.name.ends_with('\0'));
		let new_id = unsafe { RegisterWindowMessageA(self.name.as_ptr()) };

		assert_ne!(
			new_id,
			0,
			"RegisterWindowMessageA returned zero for '{}': {}",
			self.name,
			std::io::Error::last_os_error()
		);

		self.id.store(new_id, Ordering::Relaxed);

		new_id
	}
}

const INVALID_ID: u32 = 0x0;

pub(crate) static USER_EVENT_MSG_ID: LazyMessageId = LazyMessageId::new("Winit::WakeupMsg\0");

pub(crate) static EXEC_MSG_ID: LazyMessageId = LazyMessageId::new("Winit::ExecMsg\0");

pub(crate) static DESTROY_MSG_ID: LazyMessageId = LazyMessageId::new("Winit::DestroyMsg\0");

pub(crate) static SET_RETAIN_STATE_ON_SIZE_MSG_ID: LazyMessageId =
	LazyMessageId::new("Winit::SetRetainMaximized\0");

/// broadcasts this message to all top-level windows <https://docs.microsoft.com/en-us/windows/win32/shell/taskbar#taskbar-creation-notification>
pub(crate) static TASKBAR_CREATED: LazyMessageId = LazyMessageId::new("TaskbarCreated\0");
