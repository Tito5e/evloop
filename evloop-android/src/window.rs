use std::sync::Mutex;

use android_activity::{AndroidApp, ConfigurationRef, Rect};
use evloop_core::{error::CreateWindowError, ime::ImeCapabilities};
use raw_window_handle::{AndroidDisplayHandle, HandleError, RawDisplayHandle};

use crate::{ActiveEventLoop, RedrawRequester};

pub struct Window {
    app: AndroidApp,
    ime_capabilities: Mutex<Option<ImeCapabilities>>,
    redraw_requester: RedrawRequester,
}

impl Window {
    pub(crate) fn new(event_loop: &ActiveEventLoop) -> Result<Self, CreateWindowError> {
        Ok(Self {
            app: event_loop.app.clone(),
            ime_capabilities: Default::default(),
            redraw_requester: event_loop.redraw_requester.clone(),
        })
    }

    pub(crate) fn config(&self) -> ConfigurationRef {
        self.app.config()
    }

    pub(crate) fn content_rect(&self) -> Rect {
        self.app.content_rect()
    }

    fn raw_display_handle_rwh_06(&self) -> Result<RawDisplayHandle, HandleError> {
        Ok(RawDisplayHandle::Android(AndroidDisplayHandle::new()))
    }
}
