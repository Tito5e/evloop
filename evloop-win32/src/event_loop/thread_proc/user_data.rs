pub(crate) struct ThreadTargetUserData {
	pub(crate) event_loop_runner: Rc<EventLoopRunner>,
}

impl ThreadTargetUserData {
	pub(crate) fn send_wakeup(&self) {
		self.event_loop_runner.send_event(Event::WakeUp);
	}

	pub(crate) fn send_device_event(&self, device_id: DeviceId, event: DeviceEvent) {
		self.event_loop_runner.send_event(Event::Device { device_id, event });
	}
}
