//! Android smoke test: logs every event to logcat (tag `RustStdoutStderr`).
//!
//! A tap toggles an IME session: in the upper half without surrounding text, in the lower half
//! with surrounding text. The edited text is printed after every IME event.
//!
//! Build and run with `examples/android-smoke/run.sh`.

#[cfg(target_os = "android")]
mod app {
	use evloop_android::{
		application::ApplicationHandler,
		event::WindowEvent,
		event_loop::{ActiveEventLoop, EventLoop},
		platform::AndroidApp,
		window::Window,
	};
	use evloop_core::{
		event::{ElementState, EvloopEvent},
		event_loop::StartCause,
		ime::{
			Ime, ImeCapabilities, ImeEnableRequest, ImeHint, ImePurpose, ImeRequest,
			ImeRequestData, ImeSurroundingText,
		},
		keyboard::{LogicalKey, NamedKey},
	};

	#[derive(Default)]
	struct App {
		window: Option<Window>,
		/// The application's text and cursor (UTF-8 offset).
		text: String,
		cursor: usize,
		preedit: String,
		surrounding: bool,
	}

	impl App {
		fn describe(window: &Window) -> String {
			format!(
				"surface_size={:?} scale_factor={} safe_area={:?} content_rect={:?} ime_occlusion={:?}",
				window.surface_size(),
				window.scale_factor(),
				window.safe_area(),
				window.content_rect(),
				window.ime_occlusion(),
			)
		}

		fn toggle_ime(&mut self, surrounding: bool) {
			let Some(window) = &self.window else { return };
			if window.ime_capabilities().is_some() {
				println!(
					"[smoke] disable ime: {:?}",
					window.request_ime_update(ImeRequest::Disable)
				);
				return;
			}

			self.surrounding = surrounding;
			let mut capabilities = ImeCapabilities::new().with_hint_and_purpose();
			let mut data = ImeRequestData::default().with_hint_and_purpose(
				ImeHint::COMPLETION | ImeHint::SPELLCHECK,
				ImePurpose::Normal,
			);
			if surrounding {
				capabilities = capabilities.with_surrounding_text();
				data = data.with_surrounding_text(self.surrounding_text());
			}
			let request = ImeEnableRequest::new(capabilities, data).unwrap();
			println!(
				"[smoke] enable ime (surrounding={surrounding}): {:?}",
				window.request_ime_update(ImeRequest::Enable(request))
			);
		}

		fn surrounding_text(&self) -> ImeSurroundingText {
			ImeSurroundingText::new(self.text.clone(), self.cursor, self.cursor).unwrap()
		}

		fn apply_ime(&mut self, ime: &Ime) {
			match ime {
				Ime::Preedit(text, _) => self.preedit = text.clone(),
				Ime::Commit(text) => {
					self.text.insert_str(self.cursor, text);
					self.cursor += text.len();
				}
				Ime::DeleteSurrounding { before_bytes, after_bytes } => {
					let start = self.cursor.saturating_sub(*before_bytes);
					let end = (self.cursor + after_bytes).min(self.text.len());
					self.text.replace_range(start..end, "");
					self.cursor = start;
				}
				Ime::Enabled | Ime::Disabled => {}
			}
			if matches!(ime, Ime::Commit(_) | Ime::DeleteSurrounding { .. }) && self.surrounding {
				if let Some(window) = &self.window {
					let data =
						ImeRequestData::default().with_surrounding_text(self.surrounding_text());
					let _ = window.request_ime_update(ImeRequest::Update(data));
				}
			}
			println!(
				"[smoke]   text={:?} cursor={} preedit={:?}",
				self.text, self.cursor, self.preedit
			);
		}
	}

	impl ApplicationHandler for App {
		fn started(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
			println!("[smoke] started: {cause:?} theme={:?}", event_loop.system_theme());
		}

		fn stopped(&mut self, _event_loop: &ActiveEventLoop) {
			println!("[smoke] stopped");
		}

		fn surface_available(&mut self, _event_loop: &ActiveEventLoop, window: Window) {
			println!("[smoke] surface_available: {}", Self::describe(&window));
			window.request_redraw();
			self.window = Some(window);
		}

		fn surface_lost(&mut self, _event_loop: &ActiveEventLoop) {
			println!("[smoke] surface_lost");
			self.window = None;
		}

		fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
			println!("[smoke] resumed");
		}

		fn paused(&mut self, _event_loop: &ActiveEventLoop) {
			println!("[smoke] paused");
		}

		fn window_event(&mut self, _event_loop: &ActiveEventLoop, event: WindowEvent) {
			match &event {
				WindowEvent::RedrawRequested | WindowEvent::PointerMoved { .. } => {}
				_ => println!("[smoke] window: {event:?}"),
			}

			match event {
				WindowEvent::PointerButton { state: ElementState::Released, position, .. } => {
					let height = self.window.as_ref().map_or(0, |w| w.surface_size().height) as i32;
					self.toggle_ime(position.y > height / 2);
				}
				WindowEvent::Ime(ime) => self.apply_ime(&ime),
				WindowEvent::KeyboardInput { event, .. }
					if event.state == ElementState::Pressed =>
				{
					if event.logical_key == LogicalKey::Named(NamedKey::Backspace)
						&& self.cursor > 0
					{
						let start =
							self.text[..self.cursor].char_indices().last().map_or(0, |(i, _)| i);
						self.text.replace_range(start..self.cursor, "");
						self.cursor = start;
						println!("[smoke]   text={:?} cursor={} (key)", self.text, self.cursor);
					}
				}
				WindowEvent::SafeAreaChanged(_) | WindowEvent::Resized(_) => {
					if let Some(window) = &self.window {
						println!("[smoke]   {}", Self::describe(window));
					}
				}
				_ => {}
			}
		}

		fn evloop_event(&mut self, _event_loop: &ActiveEventLoop, event: EvloopEvent) {
			println!(
				"[smoke] evloop: {}",
				match event {
					EvloopEvent::ProxyWakeUp => "ProxyWakeUp",
					EvloopEvent::MemoryWarning => "MemoryWarning",
				}
			);
		}
	}

	pub fn main(app: AndroidApp) {
		println!("[smoke] android_main: sdk={}", app.sdk_version());
		let event_loop = EventLoop::builder().build(app).expect("failed to create event loop");

		// Exercise the proxy from another thread.
		let proxy = event_loop.create_proxy();
		std::thread::spawn(move || {
			std::thread::sleep(std::time::Duration::from_secs(1));
			proxy.wake_up();
		});

		event_loop.run_app(App::default()).expect("event loop failed");
		println!("[smoke] android_main returned");
	}
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: evloop_android::platform::AndroidApp) {
	app::main(app);
}
