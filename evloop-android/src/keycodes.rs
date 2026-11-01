use android_activity::AndroidApp;
use android_activity::input::{KeyAction, KeyEvent, KeyMapChar, Keycode};
use evloop_core::keyboard::{KeyCode, KeyLocation, LogicalKey, NamedKey, PhysicalKey};

// from winit-android

pub fn to_physical_key(keycode: Keycode) -> PhysicalKey {
	PhysicalKey::Code(match keycode {
		Keycode::A => KeyCode::KeyA,
		Keycode::B => KeyCode::KeyB,
		Keycode::C => KeyCode::KeyC,
		Keycode::D => KeyCode::KeyD,
		Keycode::E => KeyCode::KeyE,
		Keycode::F => KeyCode::KeyF,
		Keycode::G => KeyCode::KeyG,
		Keycode::H => KeyCode::KeyH,
		Keycode::I => KeyCode::KeyI,
		Keycode::J => KeyCode::KeyJ,
		Keycode::K => KeyCode::KeyK,
		Keycode::L => KeyCode::KeyL,
		Keycode::M => KeyCode::KeyM,
		Keycode::N => KeyCode::KeyN,
		Keycode::O => KeyCode::KeyO,
		Keycode::P => KeyCode::KeyP,
		Keycode::Q => KeyCode::KeyQ,
		Keycode::R => KeyCode::KeyR,
		Keycode::S => KeyCode::KeyS,
		Keycode::T => KeyCode::KeyT,
		Keycode::U => KeyCode::KeyU,
		Keycode::V => KeyCode::KeyV,
		Keycode::W => KeyCode::KeyW,
		Keycode::X => KeyCode::KeyX,
		Keycode::Y => KeyCode::KeyY,
		Keycode::Z => KeyCode::KeyZ,

		Keycode::Keycode0 => KeyCode::Digit0,
		Keycode::Keycode1 => KeyCode::Digit1,
		Keycode::Keycode2 => KeyCode::Digit2,
		Keycode::Keycode3 => KeyCode::Digit3,
		Keycode::Keycode4 => KeyCode::Digit4,
		Keycode::Keycode5 => KeyCode::Digit5,
		Keycode::Keycode6 => KeyCode::Digit6,
		Keycode::Keycode7 => KeyCode::Digit7,
		Keycode::Keycode8 => KeyCode::Digit8,
		Keycode::Keycode9 => KeyCode::Digit9,

		Keycode::Numpad0 => KeyCode::Numpad0,
		Keycode::Numpad1 => KeyCode::Numpad1,
		Keycode::Numpad2 => KeyCode::Numpad2,
		Keycode::Numpad3 => KeyCode::Numpad3,
		Keycode::Numpad4 => KeyCode::Numpad4,
		Keycode::Numpad5 => KeyCode::Numpad5,
		Keycode::Numpad6 => KeyCode::Numpad6,
		Keycode::Numpad7 => KeyCode::Numpad7,
		Keycode::Numpad8 => KeyCode::Numpad8,
		Keycode::Numpad9 => KeyCode::Numpad9,

		Keycode::NumpadAdd => KeyCode::NumpadAdd,
		Keycode::NumpadSubtract => KeyCode::NumpadSubtract,
		Keycode::NumpadMultiply => KeyCode::NumpadMultiply,
		Keycode::NumpadDivide => KeyCode::NumpadDivide,
		Keycode::NumpadEnter => KeyCode::NumpadEnter,
		Keycode::NumpadEquals => KeyCode::NumpadEqual,
		Keycode::NumpadComma => KeyCode::NumpadComma,
		Keycode::NumpadDot => KeyCode::NumpadDecimal,
		Keycode::NumLock => KeyCode::NumLock,

		Keycode::DpadLeft => KeyCode::ArrowLeft,
		Keycode::DpadRight => KeyCode::ArrowRight,
		Keycode::DpadUp => KeyCode::ArrowUp,
		Keycode::DpadDown => KeyCode::ArrowDown,

		Keycode::F1 => KeyCode::F1,
		Keycode::F2 => KeyCode::F2,
		Keycode::F3 => KeyCode::F3,
		Keycode::F4 => KeyCode::F4,
		Keycode::F5 => KeyCode::F5,
		Keycode::F6 => KeyCode::F6,
		Keycode::F7 => KeyCode::F7,
		Keycode::F8 => KeyCode::F8,
		Keycode::F9 => KeyCode::F9,
		Keycode::F10 => KeyCode::F10,
		Keycode::F11 => KeyCode::F11,
		Keycode::F12 => KeyCode::F12,

		Keycode::Space => KeyCode::Space,
		Keycode::Escape => KeyCode::Escape,
		Keycode::Enter => KeyCode::Enter, // not on the Numpad
		Keycode::Tab => KeyCode::Tab,

		Keycode::PageUp => KeyCode::PageUp,
		Keycode::PageDown => KeyCode::PageDown,
		Keycode::MoveHome => KeyCode::Home,
		Keycode::MoveEnd => KeyCode::End,
		Keycode::Insert => KeyCode::Insert,

		Keycode::Del => KeyCode::Backspace, // Backspace (above Enter)
		Keycode::ForwardDel => KeyCode::Delete, // Delete (below Insert)

		Keycode::Copy => KeyCode::Copy,
		Keycode::Paste => KeyCode::Paste,
		Keycode::Cut => KeyCode::Cut,

		Keycode::VolumeUp => KeyCode::AudioVolumeUp,
		Keycode::VolumeDown => KeyCode::AudioVolumeDown,
		Keycode::VolumeMute => KeyCode::AudioVolumeMute,
		// Keycode::Mute => None, // Microphone mute
		Keycode::MediaPlayPause => KeyCode::MediaPlayPause,
		Keycode::MediaStop => KeyCode::MediaStop,
		Keycode::MediaNext => KeyCode::MediaTrackNext,
		Keycode::MediaPrevious => KeyCode::MediaTrackPrevious,
		Keycode::MediaEject => KeyCode::Eject,

		Keycode::Plus => KeyCode::Equal,
		Keycode::Minus => KeyCode::Minus,
		// Winit doesn't differentiate both '+' and '=', considering they are usually
		// on the same physical key
		Keycode::Equals => KeyCode::Equal,
		Keycode::Semicolon => KeyCode::Semicolon,
		Keycode::Slash => KeyCode::Slash,
		Keycode::Backslash => KeyCode::Backslash,
		Keycode::Comma => KeyCode::Comma,
		Keycode::Period => KeyCode::Period,
		Keycode::Apostrophe => KeyCode::Quote,
		Keycode::Grave => KeyCode::Backquote,

		// Winit doesn't expose a SysRq code, so map to PrintScreen since it's
		// usually the same physical key
		Keycode::Sysrq => KeyCode::PrintScreen,
		// These are usually the same (Pause/Break)
		Keycode::Break => KeyCode::Pause,
		// These are exactly the same
		Keycode::ScrollLock => KeyCode::ScrollLock,

		Keycode::Eisu => KeyCode::Lang2,
		Keycode::Muhenkan => KeyCode::NonConvert,
		Keycode::Henkan => KeyCode::Convert,
		Keycode::Yen => KeyCode::IntlYen,
		Keycode::Ro => KeyCode::IntlRo,
		Keycode::Kana => KeyCode::Lang1,
		Keycode::KatakanaHiragana => KeyCode::KanaMode,

		Keycode::CtrlLeft => KeyCode::ControlLeft,
		Keycode::CtrlRight => KeyCode::ControlRight,

		Keycode::ShiftLeft => KeyCode::ShiftLeft,
		Keycode::ShiftRight => KeyCode::ShiftRight,

		Keycode::AltLeft => KeyCode::AltLeft,
		Keycode::AltRight => KeyCode::AltRight,

		Keycode::MetaLeft => KeyCode::MetaLeft,
		Keycode::MetaRight => KeyCode::MetaRight,

		Keycode::LeftBracket => KeyCode::BracketLeft,
		Keycode::RightBracket => KeyCode::BracketRight,

		Keycode::Power => KeyCode::Power,
		Keycode::Sleep => KeyCode::Sleep, // what about SoftSleep?
		Keycode::Wakeup => KeyCode::WakeUp,

		Keycode::CapsLock => KeyCode::CapsLock,
		Keycode::Help => KeyCode::Help,

		Keycode::Back => KeyCode::BrowserBack,
		Keycode::Forward => KeyCode::BrowserForward,
		Keycode::Refresh => KeyCode::BrowserRefresh,
		Keycode::Search => KeyCode::BrowserSearch,

		keycode => return PhysicalKey::Unidentified,
	})
}

/// Tries to map the `key_event` to a `KeyMapChar` containing a unicode character or dead key accent
///
/// This takes a `KeyEvent` and looks up its corresponding `KeyCharacterMap` and
/// uses that to try and map the `key_code` + `meta_state` to a unicode
/// character or a dead key that can be combined with the next key press.
pub fn character_map_and_combine_key(
	app: &AndroidApp,
	key_event: &KeyEvent<'_>,
	combining_accent: &mut Option<char>,
) -> Option<KeyMapChar> {
	let device_id = key_event.device_id();

	let key_map = match app.device_key_character_map(device_id) {
		Ok(key_map) => key_map,
		Err(err) => {
			tracing::warn!("Failed to look up `KeyCharacterMap` for device {device_id}: {err:?}");
			return None;
		}
	};

	match key_map.get(key_event.key_code(), key_event.meta_state()) {
		Ok(KeyMapChar::Unicode(unicode)) => {
			// Only do dead key combining on key down
			if key_event.action() == KeyAction::Down {
				let combined_unicode = if let Some(accent) = combining_accent {
					match key_map.get_dead_char(*accent, unicode) {
						Ok(Some(key)) => Some(key),
						Ok(None) => None,
						Err(err) => {
							tracing::warn!(
								"KeyEvent: Failed to combine 'dead key' accent '{accent}' with \
                                 '{unicode}': {err:?}"
							);
							None
						}
					}
				} else {
					Some(unicode)
				};
				*combining_accent = None;
				combined_unicode.map(KeyMapChar::Unicode)
			} else {
				Some(KeyMapChar::Unicode(unicode))
			}
		}
		Ok(KeyMapChar::CombiningAccent(accent)) => {
			if key_event.action() == KeyAction::Down {
				*combining_accent = Some(accent);
			}
			Some(KeyMapChar::CombiningAccent(accent))
		}
		Ok(KeyMapChar::None) => {
			// Leave any combining_accent state in tact (seems to match how other
			// Android apps work)
			None
		}
		Err(err) => {
			tracing::warn!("KeyEvent: Failed to get key map character: {err:?}");
			*combining_accent = None;
			None
		}
	}
}

pub fn to_logical(key_char: Option<KeyMapChar>, keycode: Keycode) -> LogicalKey {
	use android_activity::input::Keycode::*;

	match key_char {
		Some(KeyMapChar::Unicode(c)) => LogicalKey::Character(smol_str::SmolStr::from_iter([c])),
		Some(KeyMapChar::CombiningAccent(c)) => LogicalKey::Dead(Some(c)),
		None | Some(KeyMapChar::None) => match keycode {
			// Using `BrowserHome` instead of `GoHome` according to
			// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
			Home => LogicalKey::Named(NamedKey::BrowserHome),
			Back => LogicalKey::Named(NamedKey::BrowserBack),
			Call => LogicalKey::Named(NamedKey::Call),
			Endcall => LogicalKey::Named(NamedKey::EndCall),

			//-------------------------------------------------------------------------------
			// These should be redundant because they should have already been matched
			// as `KeyMapChar::Unicode`, but also matched here as a fallback
			Keycode0 => LogicalKey::Character("0".into()),
			Keycode1 => LogicalKey::Character("1".into()),
			Keycode2 => LogicalKey::Character("2".into()),
			Keycode3 => LogicalKey::Character("3".into()),
			Keycode4 => LogicalKey::Character("4".into()),
			Keycode5 => LogicalKey::Character("5".into()),
			Keycode6 => LogicalKey::Character("6".into()),
			Keycode7 => LogicalKey::Character("7".into()),
			Keycode8 => LogicalKey::Character("8".into()),
			Keycode9 => LogicalKey::Character("9".into()),
			Star => LogicalKey::Character("*".into()),
			Pound => LogicalKey::Character("#".into()),
			A => LogicalKey::Character("a".into()),
			B => LogicalKey::Character("b".into()),
			C => LogicalKey::Character("c".into()),
			D => LogicalKey::Character("d".into()),
			E => LogicalKey::Character("e".into()),
			F => LogicalKey::Character("f".into()),
			G => LogicalKey::Character("g".into()),
			H => LogicalKey::Character("h".into()),
			I => LogicalKey::Character("i".into()),
			J => LogicalKey::Character("j".into()),
			K => LogicalKey::Character("k".into()),
			L => LogicalKey::Character("l".into()),
			M => LogicalKey::Character("m".into()),
			N => LogicalKey::Character("n".into()),
			O => LogicalKey::Character("o".into()),
			P => LogicalKey::Character("p".into()),
			Q => LogicalKey::Character("q".into()),
			R => LogicalKey::Character("r".into()),
			S => LogicalKey::Character("s".into()),
			T => LogicalKey::Character("t".into()),
			U => LogicalKey::Character("u".into()),
			V => LogicalKey::Character("v".into()),
			W => LogicalKey::Character("w".into()),
			X => LogicalKey::Character("x".into()),
			Y => LogicalKey::Character("y".into()),
			Z => LogicalKey::Character("z".into()),
			Comma => LogicalKey::Character(",".into()),
			Period => LogicalKey::Character(".".into()),
			Grave => LogicalKey::Character("`".into()),
			Minus => LogicalKey::Character("-".into()),
			Equals => LogicalKey::Character("=".into()),
			LeftBracket => LogicalKey::Character("[".into()),
			RightBracket => LogicalKey::Character("]".into()),
			Backslash => LogicalKey::Character("\\".into()),
			Semicolon => LogicalKey::Character(";".into()),
			Apostrophe => LogicalKey::Character("'".into()),
			Slash => LogicalKey::Character("/".into()),
			At => LogicalKey::Character("@".into()),
			Plus => LogicalKey::Character("+".into()),
			//-------------------------------------------------------------------------------
			DpadUp => LogicalKey::Named(NamedKey::ArrowUp),
			DpadDown => LogicalKey::Named(NamedKey::ArrowDown),
			DpadLeft => LogicalKey::Named(NamedKey::ArrowLeft),
			DpadRight => LogicalKey::Named(NamedKey::ArrowRight),
			DpadCenter => LogicalKey::Named(NamedKey::Enter),

			VolumeUp => LogicalKey::Named(NamedKey::AudioVolumeUp),
			VolumeDown => LogicalKey::Named(NamedKey::AudioVolumeDown),
			Power => LogicalKey::Named(NamedKey::Power),
			Camera => LogicalKey::Named(NamedKey::Camera),
			Clear => LogicalKey::Named(NamedKey::Clear),

			AltLeft => LogicalKey::Named(NamedKey::Alt),
			AltRight => LogicalKey::Named(NamedKey::Alt),
			ShiftLeft => LogicalKey::Named(NamedKey::Shift),
			ShiftRight => LogicalKey::Named(NamedKey::Shift),
			Tab => LogicalKey::Named(NamedKey::Tab),
			Space => LogicalKey::Character(" ".into()),
			Sym => LogicalKey::Named(NamedKey::Symbol),
			Explorer => LogicalKey::Named(NamedKey::LaunchWebBrowser),
			Envelope => LogicalKey::Named(NamedKey::LaunchMail),
			Enter => LogicalKey::Named(NamedKey::Enter),
			Del => LogicalKey::Named(NamedKey::Backspace),

			// According to https://developer.android.com/reference/android/view/KeyEvent#KEYCODE_NUM
			Num => LogicalKey::Named(NamedKey::Alt),

			Headsethook => LogicalKey::Named(NamedKey::HeadsetHook),
			Focus => LogicalKey::Named(NamedKey::CameraFocus),

			Notification => LogicalKey::Named(NamedKey::Notification),
			Search => LogicalKey::Named(NamedKey::BrowserSearch),
			MediaPlayPause => LogicalKey::Named(NamedKey::MediaPlayPause),
			MediaStop => LogicalKey::Named(NamedKey::MediaStop),
			MediaNext => LogicalKey::Named(NamedKey::MediaTrackNext),
			MediaPrevious => LogicalKey::Named(NamedKey::MediaTrackPrevious),
			MediaRewind => LogicalKey::Named(NamedKey::MediaRewind),
			MediaFastForward => LogicalKey::Named(NamedKey::MediaFastForward),
			Mute => LogicalKey::Named(NamedKey::MicrophoneVolumeMute),
			PageUp => LogicalKey::Named(NamedKey::PageUp),
			PageDown => LogicalKey::Named(NamedKey::PageDown),

			Escape => LogicalKey::Named(NamedKey::Escape),
			ForwardDel => LogicalKey::Named(NamedKey::Delete),
			CtrlLeft => LogicalKey::Named(NamedKey::Control),
			CtrlRight => LogicalKey::Named(NamedKey::Control),
			CapsLock => LogicalKey::Named(NamedKey::CapsLock),
			ScrollLock => LogicalKey::Named(NamedKey::ScrollLock),
			MetaLeft => LogicalKey::Named(NamedKey::Meta),
			MetaRight => LogicalKey::Named(NamedKey::Meta),
			Function => LogicalKey::Named(NamedKey::Fn),
			Sysrq => LogicalKey::Named(NamedKey::PrintScreen),
			Break => LogicalKey::Named(NamedKey::Pause),
			MoveHome => LogicalKey::Named(NamedKey::Home),
			MoveEnd => LogicalKey::Named(NamedKey::End),
			Insert => LogicalKey::Named(NamedKey::Insert),
			Forward => LogicalKey::Named(NamedKey::BrowserForward),
			MediaPlay => LogicalKey::Named(NamedKey::MediaPlay),
			MediaPause => LogicalKey::Named(NamedKey::MediaPause),
			MediaClose => LogicalKey::Named(NamedKey::MediaClose),
			MediaEject => LogicalKey::Named(NamedKey::Eject),
			MediaRecord => LogicalKey::Named(NamedKey::MediaRecord),
			F1 => LogicalKey::Named(NamedKey::F1),
			F2 => LogicalKey::Named(NamedKey::F2),
			F3 => LogicalKey::Named(NamedKey::F3),
			F4 => LogicalKey::Named(NamedKey::F4),
			F5 => LogicalKey::Named(NamedKey::F5),
			F6 => LogicalKey::Named(NamedKey::F6),
			F7 => LogicalKey::Named(NamedKey::F7),
			F8 => LogicalKey::Named(NamedKey::F8),
			F9 => LogicalKey::Named(NamedKey::F9),
			F10 => LogicalKey::Named(NamedKey::F10),
			F11 => LogicalKey::Named(NamedKey::F11),
			F12 => LogicalKey::Named(NamedKey::F12),
			NumLock => LogicalKey::Named(NamedKey::NumLock),
			Numpad0 => LogicalKey::Character("0".into()),
			Numpad1 => LogicalKey::Character("1".into()),
			Numpad2 => LogicalKey::Character("2".into()),
			Numpad3 => LogicalKey::Character("3".into()),
			Numpad4 => LogicalKey::Character("4".into()),
			Numpad5 => LogicalKey::Character("5".into()),
			Numpad6 => LogicalKey::Character("6".into()),
			Numpad7 => LogicalKey::Character("7".into()),
			Numpad8 => LogicalKey::Character("8".into()),
			Numpad9 => LogicalKey::Character("9".into()),
			NumpadDivide => LogicalKey::Character("/".into()),
			NumpadMultiply => LogicalKey::Character("*".into()),
			NumpadSubtract => LogicalKey::Character("-".into()),
			NumpadAdd => LogicalKey::Character("+".into()),
			NumpadDot => LogicalKey::Character(".".into()),
			NumpadComma => LogicalKey::Character(",".into()),
			NumpadEnter => LogicalKey::Named(NamedKey::Enter),
			NumpadEquals => LogicalKey::Character("=".into()),
			NumpadLeftParen => LogicalKey::Character("(".into()),
			NumpadRightParen => LogicalKey::Character(")".into()),

			VolumeMute => LogicalKey::Named(NamedKey::AudioVolumeMute),
			Info => LogicalKey::Named(NamedKey::Info),
			ChannelUp => LogicalKey::Named(NamedKey::ChannelUp),
			ChannelDown => LogicalKey::Named(NamedKey::ChannelDown),
			ZoomIn => LogicalKey::Named(NamedKey::ZoomIn),
			ZoomOut => LogicalKey::Named(NamedKey::ZoomOut),
			Tv => LogicalKey::Named(NamedKey::TV),
			Guide => LogicalKey::Named(NamedKey::Guide),
			Dvr => LogicalKey::Named(NamedKey::DVR),
			Bookmark => LogicalKey::Named(NamedKey::BrowserFavorites),
			Captions => LogicalKey::Named(NamedKey::ClosedCaptionToggle),
			Settings => LogicalKey::Named(NamedKey::Settings),
			TvPower => LogicalKey::Named(NamedKey::TVPower),
			TvInput => LogicalKey::Named(NamedKey::TVInput),
			StbPower => LogicalKey::Named(NamedKey::STBPower),
			StbInput => LogicalKey::Named(NamedKey::STBInput),
			AvrPower => LogicalKey::Named(NamedKey::AVRPower),
			AvrInput => LogicalKey::Named(NamedKey::AVRInput),
			ProgRed => LogicalKey::Named(NamedKey::ColorF0Red),
			ProgGreen => LogicalKey::Named(NamedKey::ColorF1Green),
			ProgYellow => LogicalKey::Named(NamedKey::ColorF2Yellow),
			ProgBlue => LogicalKey::Named(NamedKey::ColorF3Blue),
			AppSwitch => LogicalKey::Named(NamedKey::AppSwitch),
			LanguageSwitch => LogicalKey::Named(NamedKey::GroupNext),
			MannerMode => LogicalKey::Named(NamedKey::MannerMode),
			Keycode3dMode => LogicalKey::Named(NamedKey::TV3DMode),
			Contacts => LogicalKey::Named(NamedKey::LaunchContacts),
			Calendar => LogicalKey::Named(NamedKey::LaunchCalendar),
			Music => LogicalKey::Named(NamedKey::LaunchMusicPlayer),
			Calculator => LogicalKey::Named(NamedKey::LaunchApplication2),
			ZenkakuHankaku => LogicalKey::Named(NamedKey::ZenkakuHankaku),
			Eisu => LogicalKey::Named(NamedKey::Eisu),
			Muhenkan => LogicalKey::Named(NamedKey::NonConvert),
			Henkan => LogicalKey::Named(NamedKey::Convert),
			KatakanaHiragana => LogicalKey::Named(NamedKey::HiraganaKatakana),
			Kana => LogicalKey::Named(NamedKey::KanjiMode),
			BrightnessDown => LogicalKey::Named(NamedKey::BrightnessDown),
			BrightnessUp => LogicalKey::Named(NamedKey::BrightnessUp),
			MediaAudioTrack => LogicalKey::Named(NamedKey::MediaAudioTrack),
			Sleep => LogicalKey::Named(NamedKey::Standby),
			Wakeup => LogicalKey::Named(NamedKey::WakeUp),
			Pairing => LogicalKey::Named(NamedKey::Pairing),
			MediaTopMenu => LogicalKey::Named(NamedKey::MediaTopMenu),
			LastChannel => LogicalKey::Named(NamedKey::MediaLast),
			TvDataService => LogicalKey::Named(NamedKey::TVDataService),
			VoiceAssist => LogicalKey::Named(NamedKey::VoiceDial),
			TvRadioService => LogicalKey::Named(NamedKey::TVRadioService),
			TvTeletext => LogicalKey::Named(NamedKey::Teletext),
			TvNumberEntry => LogicalKey::Named(NamedKey::TVNumberEntry),
			TvTerrestrialAnalog => LogicalKey::Named(NamedKey::TVTerrestrialAnalog),
			TvTerrestrialDigital => LogicalKey::Named(NamedKey::TVTerrestrialDigital),
			TvSatellite => LogicalKey::Named(NamedKey::TVSatellite),
			TvSatelliteBs => LogicalKey::Named(NamedKey::TVSatelliteBS),
			TvSatelliteCs => LogicalKey::Named(NamedKey::TVSatelliteCS),
			TvSatelliteService => LogicalKey::Named(NamedKey::TVSatelliteToggle),
			TvNetwork => LogicalKey::Named(NamedKey::TVNetwork),
			TvAntennaCable => LogicalKey::Named(NamedKey::TVAntennaCable),
			TvInputHdmi1 => LogicalKey::Named(NamedKey::TVInputHDMI1),
			TvInputHdmi2 => LogicalKey::Named(NamedKey::TVInputHDMI2),
			TvInputHdmi3 => LogicalKey::Named(NamedKey::TVInputHDMI3),
			TvInputHdmi4 => LogicalKey::Named(NamedKey::TVInputHDMI4),
			TvInputComposite1 => LogicalKey::Named(NamedKey::TVInputComposite1),
			TvInputComposite2 => LogicalKey::Named(NamedKey::TVInputComposite2),
			TvInputComponent1 => LogicalKey::Named(NamedKey::TVInputComponent1),
			TvInputComponent2 => LogicalKey::Named(NamedKey::TVInputComponent2),
			TvInputVga1 => LogicalKey::Named(NamedKey::TVInputVGA1),
			TvAudioDescription => LogicalKey::Named(NamedKey::TVAudioDescription),
			TvAudioDescriptionMixUp => LogicalKey::Named(NamedKey::TVAudioDescriptionMixUp),
			TvAudioDescriptionMixDown => LogicalKey::Named(NamedKey::TVAudioDescriptionMixDown),
			TvZoomMode => LogicalKey::Named(NamedKey::ZoomToggle),
			TvContentsMenu => LogicalKey::Named(NamedKey::TVContentsMenu),
			TvMediaContextMenu => LogicalKey::Named(NamedKey::TVMediaContext),
			TvTimerProgramming => LogicalKey::Named(NamedKey::TVTimer),
			Help => LogicalKey::Named(NamedKey::Help),
			NavigatePrevious => LogicalKey::Named(NamedKey::NavigatePrevious),
			NavigateNext => LogicalKey::Named(NamedKey::NavigateNext),
			NavigateIn => LogicalKey::Named(NamedKey::NavigateIn),
			NavigateOut => LogicalKey::Named(NamedKey::NavigateOut),
			MediaSkipForward => LogicalKey::Named(NamedKey::MediaSkipForward),
			MediaSkipBackward => LogicalKey::Named(NamedKey::MediaSkipBackward),
			MediaStepForward => LogicalKey::Named(NamedKey::MediaStepForward),
			MediaStepBackward => LogicalKey::Named(NamedKey::MediaStepBackward),
			Cut => LogicalKey::Named(NamedKey::Cut),
			Copy => LogicalKey::Named(NamedKey::Copy),
			Paste => LogicalKey::Named(NamedKey::Paste),
			Refresh => LogicalKey::Named(NamedKey::BrowserRefresh),

			_ => LogicalKey::Unidentified,
		},
	}
}

pub fn to_location(keycode: Keycode) -> KeyLocation {
	use android_activity::input::Keycode::*;

	match keycode {
		AltLeft => KeyLocation::Left,
		AltRight => KeyLocation::Right,
		ShiftLeft => KeyLocation::Left,
		ShiftRight => KeyLocation::Right,

		// According to https://developer.android.com/reference/android/view/KeyEvent#KEYCODE_NUM
		Num => KeyLocation::Left,

		CtrlLeft => KeyLocation::Left,
		CtrlRight => KeyLocation::Right,
		MetaLeft => KeyLocation::Left,
		MetaRight => KeyLocation::Right,

		NumLock => KeyLocation::Numpad,
		Numpad0 => KeyLocation::Numpad,
		Numpad1 => KeyLocation::Numpad,
		Numpad2 => KeyLocation::Numpad,
		Numpad3 => KeyLocation::Numpad,
		Numpad4 => KeyLocation::Numpad,
		Numpad5 => KeyLocation::Numpad,
		Numpad6 => KeyLocation::Numpad,
		Numpad7 => KeyLocation::Numpad,
		Numpad8 => KeyLocation::Numpad,
		Numpad9 => KeyLocation::Numpad,
		NumpadDivide => KeyLocation::Numpad,
		NumpadMultiply => KeyLocation::Numpad,
		NumpadSubtract => KeyLocation::Numpad,
		NumpadAdd => KeyLocation::Numpad,
		NumpadDot => KeyLocation::Numpad,
		NumpadComma => KeyLocation::Numpad,
		NumpadEnter => KeyLocation::Numpad,
		NumpadEquals => KeyLocation::Numpad,
		NumpadLeftParen => KeyLocation::Numpad,
		NumpadRightParen => KeyLocation::Numpad,

		_ => KeyLocation::Standard,
	}
}
