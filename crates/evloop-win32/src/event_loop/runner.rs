use core::fmt;

use windows_sys::Win32::Foundation::HWND;

/// アプリケーション全体の実行状態を管理する
pub(super) struct EventLoopRunner {
    pub(super) message_window: HWND,
}

impl fmt::Debug for EventLoopRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoopRunner")
            .field("message_window", &self.message_window)
            .finish_non_exhaustive()
    }
}

impl EventLoopRunner {
    pub(super) fn new(message_window: HWND) -> Self {
        Self { message_window }
    }
}
