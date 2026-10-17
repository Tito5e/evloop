use evloop_core::event::{DeviceEvent, DeviceId, WindowEvent};
use evloop_core::StartCause;
use crate::ActiveEventLoop;

pub trait ApplicationHandler {
    fn can_create_surfaces(&mut self, event_loop: &ActiveEventLoop);

    fn destroy_surfaces(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        let _ = (event_loop, cause);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent);

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: Option<DeviceId>, event: DeviceEvent) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop);

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn proxy_wake_up(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}