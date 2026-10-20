use crate::lifecycle::{Paused, Resumed, Running, Started, Stopped, SurfaceAvailable, SurfaceLost};
use evloop_core::event::{EvloopEvent, WindowEvent};
use evloop_core::event_loop::StartCause;

pub trait ApplicationHandler {
    fn started(&mut self, event_loop: &impl Started, cause: StartCause);

    fn stopped(&mut self, event_loop: &impl Stopped) {
        let _ = event_loop;
    }

    fn surface_available(&mut self, event_loop: &impl SurfaceAvailable);

    fn surface_lost(&mut self, event_loop: &impl SurfaceLost);

    fn resumed(&mut self, event_loop: &impl Resumed) {
        let _ = event_loop;
    }

    fn paused(&mut self, event_loop: &impl Paused) {
        let _ = event_loop;
    }

    fn enter_events(&mut self, event_loop: &impl Running, cause: StartCause) {
        let _ = (event_loop, cause);
    }

    fn window_event(&mut self, event_loop: &impl Running, event: WindowEvent);

    fn evloop_event(&mut self, event_loop: &impl Running, event: EvloopEvent) {
        let _ = (event_loop, event);
    }

    fn exit_events(&mut self, event_loop: &impl Running) {
        let _ = event_loop;
    }
}

impl<A: ?Sized + ApplicationHandler> ApplicationHandler for &mut A {
    #[inline]
    fn started(&mut self, event_loop: &impl Started, cause: StartCause) {
        (**self).started(event_loop, cause);
    }

    #[inline]
    fn stopped(&mut self, event_loop: &impl Stopped) {
        (**self).stopped(event_loop);
    }

    #[inline]
    fn surface_available(&mut self, event_loop: &impl SurfaceAvailable) {
        (**self).surface_available(event_loop);
    }

    #[inline]
    fn surface_lost(&mut self, event_loop: &impl SurfaceLost) {
        (**self).surface_lost(event_loop);
    }

    #[inline]
    fn resumed(&mut self, event_loop: &impl Resumed) {
        (**self).resumed(event_loop);
    }

    #[inline]
    fn paused(&mut self, event_loop: &impl Paused) {
        (**self).paused(event_loop);
    }

    #[inline]
    fn enter_events(&mut self, event_loop: &impl Running, cause: StartCause) {
        (**self).enter_events(event_loop, cause);
    }

    #[inline]
    fn window_event(&mut self, event_loop: &impl Running, event: WindowEvent) {
        (**self).window_event(event_loop, event);
    }

    #[inline]
    fn evloop_event(&mut self, event_loop: &impl Running, event: EvloopEvent) {
        (**self).evloop_event(event_loop, event);
    }

    #[inline]
    fn exit_events(&mut self, event_loop: &impl Running) {
        (**self).exit_events(event_loop);
    }
}
