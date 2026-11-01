use windows_sys::Win32::{
	Foundation::HWND, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::PostMessageW,
};

use crate::sys::message_id::EXEC_MSG_ID;

#[derive(Debug)]
pub(crate) struct EventLoopThreadExecutor {
	pub(crate) thread_id: u32,
	pub(crate) target_window: HWND,
}

unsafe impl Send for EventLoopThreadExecutor {}

unsafe impl Sync for EventLoopThreadExecutor {}

impl EventLoopThreadExecutor {
	pub(crate) fn in_event_loop_thread(&self) -> bool {
		let current_thread_id = unsafe { GetCurrentThreadId() };
		self.thread_id == current_thread_id
	}

	pub(crate) fn execute_in_thread<F>(&self, mut function: F)
	where
		F: FnMut() + Send + 'static,
	{
		unsafe {
			if self.in_event_loop_thread() {
				function();
			} else {
				let boxed2: ThreadExecFn = Box::new(Box::new(function));
				let raw = Box::into_raw(boxed2);

				let res = PostMessageW(self.target_window, EXEC_MSG_ID.get(), raw as usize, 0);
				assert!(res != false.into(), "PostMessage failed");
			}
		}
	}
}

pub(crate) type ThreadExecFn = Box<Box<dyn FnMut()>>;
