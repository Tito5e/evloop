//! The IME session of the Android backend (see `evloop_core::ime` for the shared semantics).
//!
//! GameTextInput keeps an editor buffer on the Java side. evloop mirrors what the application
//! holds in [`ImeSession::committed`] and turns every editor state reported by the IME into
//! `Ime::Preedit` / `Ime::Commit` / `Ime::DeleteSurrounding` (see [`crate::text`]).
//!
//! - With the `surrounding_text` capability the buffer is the application's text around the
//!   cursor, set from `ImeRequest::{Enable, Update}`.
//! - Without it the buffer only contains what was typed during the session. It is cleared when
//!   it grows large and nothing is being composed; key events (e.g. `Backspace`) are delivered for
//!   edits beyond the buffer.

use std::collections::VecDeque;

use evloop_core::{
	error::ImeRequestError,
	ime::{Ime, ImeCapabilities, ImeHint, ImePurpose, ImeRequest, ImeRequestData},
};

use crate::{
	glue::AndroidApp,
	platform::TextInputState,
	text::{EditorState, TextEdit, diff, utf8_to_utf16},
};

/// Clear the buffer (when not composing and without surrounding text) past this many bytes.
const MAX_SCRATCH_BYTES: usize = 4000;

#[derive(Debug, Default)]
pub(crate) struct ImeSession {
	capabilities: Option<ImeCapabilities>,
	/// The text outside of the composing region, as the application holds it.
	committed: String,
	/// UTF-8 offset of the cursor in `committed`.
	committed_cursor: usize,
	/// Whether a non-empty `Ime::Preedit` was the last preedit reported.
	preedit_active: bool,
	/// Events produced by requests, delivered by the event loop.
	pending: VecDeque<Ime>,
	/// The last state reported with `WindowEvent::TextInputStateChanged`.
	last_reported: Option<TextInputState>,
}

impl ImeSession {
	pub(crate) fn capabilities(&self) -> Option<ImeCapabilities> {
		self.capabilities
	}
}

/// Handles `Window::request_ime_update`.
pub(crate) fn request(app: &AndroidApp, request: ImeRequest) -> Result<(), ImeRequestError> {
	match request {
		ImeRequest::Enable(enable) => {
			let (capabilities, data) = enable.into_raw();
			{
				let mut session = app.ime_session();
				if session.capabilities.is_some() {
					return Err(ImeRequestError::AlreadyEnabled);
				}
				session.capabilities = Some(capabilities);
				session.committed.clear();
				session.committed_cursor = 0;
				session.preedit_active = false;
				session.pending.push_back(Ime::Enabled);
			}

			let (input_type, action, options) =
				editor_info(data.hint_and_purpose.filter(|_| capabilities.hint_and_purpose()));
			app.set_ime_editor_info(input_type, action, options);
			apply_surrounding_text(app, capabilities, &data, true);
			app.restart_input();
			app.show_soft_input();
		}
		ImeRequest::Update(data) => {
			let capabilities = app.ime_session().capabilities.ok_or(ImeRequestError::NotEnabled)?;
			if let Some(hint_and_purpose) =
				data.hint_and_purpose.filter(|_| capabilities.hint_and_purpose())
			{
				let (input_type, action, options) = editor_info(Some(hint_and_purpose));
				app.set_ime_editor_info(input_type, action, options);
				app.restart_input();
			}
			apply_surrounding_text(app, capabilities, &data, false);
		}
		ImeRequest::Disable => {
			{
				let mut session = app.ime_session();
				session.capabilities.take().ok_or(ImeRequestError::NotEnabled)?;
				if std::mem::take(&mut session.preedit_active) {
					session.pending.push_back(Ime::Preedit(String::new(), None));
				}
				session.committed.clear();
				session.committed_cursor = 0;
				session.pending.push_back(Ime::Disabled);
			}
			app.set_text_input_state("", (0, 0));
			app.hide_soft_input();
		}
	}
	app.wake();
	Ok(())
}

/// Pushes the application's surrounding text to the IME. While something is being composed the
/// IME's text includes the preedit, so it is not replaced (that would cancel the composition).
fn apply_surrounding_text(
	app: &AndroidApp,
	capabilities: ImeCapabilities,
	data: &ImeRequestData,
	enabling: bool,
) {
	let surrounding = data.surrounding_text.as_ref().filter(|_| capabilities.surrounding_text());
	let mut session = app.ime_session();
	match surrounding {
		Some(surrounding) => {
			if session.preedit_active {
				return;
			}
			let text = surrounding.text();
			let selection = (
				utf8_to_utf16(text, surrounding.anchor().min(surrounding.cursor())),
				utf8_to_utf16(text, surrounding.anchor().max(surrounding.cursor())),
			);
			session.committed = text.to_owned();
			session.committed_cursor = surrounding.cursor();
			drop(session);
			app.set_text_input_state(text, selection);
		}
		None if enabling => {
			drop(session);
			app.set_text_input_state("", (0, 0));
		}
		None => {}
	}
}

/// Events produced by `request` since the last call.
pub(crate) fn take_pending(app: &AndroidApp) -> Vec<Ime> {
	app.ime_session().pending.drain(..).collect()
}

/// Handles a new editor state from the IME: returns the raw state (`None` when it did not change)
/// and the `Ime` events to emit.
pub(crate) fn on_text_input(app: &AndroidApp) -> (Option<TextInputState>, Vec<Ime>) {
	let raw = app.raw_text_input_state();
	let state = EditorState::from_utf16(raw.text, raw.selection, raw.compose);
	let public = TextInputState {
		text: state.text.clone(),
		selection: state.selection.clone(),
		compose: state.compose.clone(),
	};

	let mut session = app.ime_session();
	if session.last_reported.as_ref() == Some(&public) {
		return (None, Vec::new());
	}
	session.last_reported = Some(public.clone());

	let mut events = Vec::new();
	let new_committed = state.committed();

	let enabled = session.capabilities.is_some();
	if enabled {
		let edits = diff(&session.committed, session.committed_cursor, &new_committed);
		let preedit = state.preedit();

		if session.preedit_active && (!edits.is_empty() || preedit.is_none()) {
			events.push(Ime::Preedit(String::new(), None));
			session.preedit_active = false;
		}
		for edit in edits {
			events.push(match edit {
				TextEdit::Delete { before_bytes, after_bytes } => {
					Ime::DeleteSurrounding { before_bytes, after_bytes }
				}
				TextEdit::Commit(text) => Ime::Commit(text),
			});
		}
		if let Some((text, cursor)) = preedit {
			events.push(Ime::Preedit(text, cursor));
			session.preedit_active = true;
		}
	}

	session.committed = new_committed;
	session.committed_cursor = state.committed_cursor();

	let clear_scratch = enabled
		&& state.compose.is_none()
		&& session.committed.len() > MAX_SCRATCH_BYTES
		&& !session.capabilities.is_some_and(|c| c.surrounding_text());
	if clear_scratch {
		session.committed.clear();
		session.committed_cursor = 0;
		drop(session);
		app.set_text_input_state("", (0, 0));
	}

	(Some(public), events)
}

// `EditorInfo` constants (`android.text.InputType` and `EditorInfo.IME_*`).
mod editor {
	pub const TYPE_CLASS_TEXT: u32 = 0x1;
	pub const TYPE_CLASS_NUMBER: u32 = 0x2;
	pub const TYPE_CLASS_PHONE: u32 = 0x3;
	pub const TYPE_CLASS_DATETIME: u32 = 0x4;

	pub const TYPE_TEXT_FLAG_CAP_CHARACTERS: u32 = 0x1000;
	pub const TYPE_TEXT_FLAG_CAP_WORDS: u32 = 0x2000;
	pub const TYPE_TEXT_FLAG_CAP_SENTENCES: u32 = 0x4000;
	pub const TYPE_TEXT_FLAG_AUTO_CORRECT: u32 = 0x8000;
	pub const TYPE_TEXT_FLAG_AUTO_COMPLETE: u32 = 0x1_0000;
	pub const TYPE_TEXT_FLAG_MULTI_LINE: u32 = 0x2_0000;
	pub const TYPE_TEXT_FLAG_NO_SUGGESTIONS: u32 = 0x8_0000;

	pub const TYPE_TEXT_VARIATION_URI: u32 = 0x10;
	pub const TYPE_TEXT_VARIATION_EMAIL_ADDRESS: u32 = 0x20;
	pub const TYPE_TEXT_VARIATION_PASSWORD: u32 = 0x80;
	pub const TYPE_NUMBER_VARIATION_PASSWORD: u32 = 0x10;
	pub const TYPE_DATETIME_VARIATION_DATE: u32 = 0x10;
	pub const TYPE_DATETIME_VARIATION_TIME: u32 = 0x20;

	pub const IME_ACTION_UNSPECIFIED: u32 = 0x0;
	pub const IME_ACTION_NONE: u32 = 0x1;

	pub const IME_FLAG_NO_PERSONALIZED_LEARNING: u32 = 0x100_0000;
	pub const IME_FLAG_NO_FULLSCREEN: u32 = 0x200_0000;
	pub const IME_FLAG_NO_EXTRACT_UI: u32 = 0x1000_0000;
	pub const IME_FLAG_FORCE_ASCII: u32 = 0x8000_0000;
}

/// Maps the hint and purpose to `(inputType, imeAction, imeOptions)`.
///
/// The surface is drawn by the application, so the IME never takes the screen over
/// (`IME_FLAG_NO_FULLSCREEN | IME_FLAG_NO_EXTRACT_UI`).
fn editor_info(hint_and_purpose: Option<(ImeHint, ImePurpose)>) -> (u32, u32, u32) {
	use editor::*;

	let mut options = IME_FLAG_NO_FULLSCREEN | IME_FLAG_NO_EXTRACT_UI;
	let Some((hint, purpose)) = hint_and_purpose else {
		return (TYPE_CLASS_TEXT, IME_ACTION_UNSPECIFIED, options);
	};

	let mut input_type = match purpose {
		ImePurpose::Normal => TYPE_CLASS_TEXT,
		ImePurpose::Password => TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_PASSWORD,
		ImePurpose::Terminal => TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_NO_SUGGESTIONS,
		ImePurpose::Number => TYPE_CLASS_NUMBER,
		ImePurpose::Phone => TYPE_CLASS_PHONE,
		ImePurpose::Url => TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_URI,
		ImePurpose::Email => TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
		ImePurpose::Pin => TYPE_CLASS_NUMBER | TYPE_NUMBER_VARIATION_PASSWORD,
		ImePurpose::Date => TYPE_CLASS_DATETIME | TYPE_DATETIME_VARIATION_DATE,
		ImePurpose::Time => TYPE_CLASS_DATETIME | TYPE_DATETIME_VARIATION_TIME,
		ImePurpose::DateTime => TYPE_CLASS_DATETIME,
	};

	if input_type & 0xf == TYPE_CLASS_TEXT {
		let flags = [
			(ImeHint::COMPLETION, TYPE_TEXT_FLAG_AUTO_COMPLETE),
			(ImeHint::SPELLCHECK, TYPE_TEXT_FLAG_AUTO_CORRECT),
			(ImeHint::AUTO_CAPITALIZATION, TYPE_TEXT_FLAG_CAP_SENTENCES),
			(ImeHint::UPPERCASE, TYPE_TEXT_FLAG_CAP_CHARACTERS),
			(ImeHint::TITLECASE, TYPE_TEXT_FLAG_CAP_WORDS),
			(ImeHint::MULTILINE, TYPE_TEXT_FLAG_MULTI_LINE),
		];
		for (h, flag) in flags {
			if hint.contains(h) {
				input_type |= flag;
			}
		}
		if hint.contains(ImeHint::HIDDEN_TEXT) && input_type & 0xff0 == 0 {
			input_type |= TYPE_TEXT_VARIATION_PASSWORD;
		}
	}

	if hint.contains(ImeHint::SENSITIVE_DATA) {
		options |= IME_FLAG_NO_PERSONALIZED_LEARNING;
	}
	if hint.contains(ImeHint::LATIN) {
		options |= IME_FLAG_FORCE_ASCII;
	}

	// In a multi-line field the action key inserts a new line.
	let action =
		if hint.contains(ImeHint::MULTILINE) { IME_ACTION_NONE } else { IME_ACTION_UNSPECIFIED };

	(input_type, action, options)
}
