use windows_sys::Win32::Foundation::HWND;

#[derive(Debug)]
pub(crate) struct EventLoopThreadExecutor {
	pub(crate) thread_id: u32,
	pub(crate) target_window: HWND,
}

unsafe impl Send for EventLoopThreadExecutor {}

unsafe impl Sync for EventLoopThreadExecutor {}

impl EventLoopThreadExecutor {}

pub(crate) type ThreadExecFn = Box<Box<dyn FnMut()>>;
