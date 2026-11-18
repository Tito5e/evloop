use core::fmt;
use std::{any::Any, cell::Cell, panic};

use windows_sys::Win32::Foundation::HWND;

type PanicError = Box<dyn Any + Send + 'static>;

/// アプリケーション全体の実行状態を管理する
pub(super) struct EventLoopRunner {
    pub(super) message_window: HWND,

    panic_error: Cell<Option<PanicError>>,
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
        Self {
            message_window,
            panic_error: Cell::new(None),
        }
    }

    pub(crate) fn catch_unwind<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
        let panic_error = self.panic_error.take();
        if panic_error.is_none() {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(f));

            match self.panic_error.take() {
                None => match result {
                    Ok(r) => Some(r),
                    Err(e) => {
                        self.panic_error.set(Some(e));
                        None
                    }
                },
                Some(e) => {
                    self.panic_error.set(Some(e));
                    None
                }
            }
        } else {
            self.panic_error.set(panic_error);
            None
        }
    }
}
