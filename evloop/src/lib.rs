use std::marker::PhantomData;

use evloop_core::error::EventLoopError;

#[cfg(target_os = "windows")]
use evloop_win32 as platform;

#[cfg(target_os = "android")]
use evloop_android as platform;
use evloop_android::activity::AndroidApp;

pub struct EventLoop {
    inner: platform::EventLoop,
    _marker: PhantomData<*mut ()>,
}

#[derive(Default)]
pub struct EventLoopBuilder {
    attributes: platform::EventLoopAttributes,
}

impl EventLoopBuilder {
    #[inline]
    pub fn build(&mut self) -> Result<EventLoop, EventLoopError> {
        Ok(EventLoop {
            inner: platform::EventLoop::new(&self.attributes)?,
            _marker: PhantomData,
        })
    }

    #[cfg(target_os = "android")]
    #[inline]
    pub fn with_android_app(mut self, app: AndroidApp) -> Self {
        self.attributes.android_app = Some(app);
        self
    }
}
