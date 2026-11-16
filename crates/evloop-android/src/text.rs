//! Translation between Android's editor-state IME model and evloop's preedit / commit model.
//!
//! Android's `InputConnection` (through GameTextInput) reports the whole editor state: the text,
//! the selection, and the composing region, with UTF-16 offsets. evloop applications own their
//! text and receive `Ime::Preedit` / `Ime::Commit` / `Ime::DeleteSurrounding` with UTF-8 byte
//! offsets. This module computes the events that turn one state into the next.
//!
//! Pure std code, so it is also compiled (and tested) on non-Android hosts.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use std::ops::Range;

/// Converts a UTF-16 code unit offset into a UTF-8 byte offset of `text`, clamped to the text and
/// rounded down to a character boundary.
pub(crate) fn utf16_to_utf8(text: &str, utf16: i32) -> usize {
	let Ok(target) = usize::try_from(utf16) else {
		return 0;
	};
	let mut units = 0;
	for (byte, ch) in text.char_indices() {
		if units + ch.len_utf16() > target {
			return byte;
		}
		units += ch.len_utf16();
	}
	text.len()
}

/// Converts a UTF-8 byte offset of `text` (rounded down to a character boundary) into a UTF-16
/// code unit offset.
pub(crate) fn utf8_to_utf16(text: &str, utf8: usize) -> i32 {
	let mut end = utf8.min(text.len());
	while !text.is_char_boundary(end) {
		end -= 1;
	}
	text[..end].encode_utf16().count() as i32
}

/// The editor state in UTF-8 byte offsets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EditorState {
	pub text: String,
	/// Normalized (`start <= end`).
	pub selection: Range<usize>,
	/// Normalized; `None` when there is no (non-empty) composing region.
	pub compose: Option<Range<usize>>,
}

impl EditorState {
	/// Builds the state from UTF-16 offsets, as reported by Java.
	pub(crate) fn from_utf16(
		text: String,
		selection: (i32, i32),
		compose: Option<(i32, i32)>,
	) -> Self {
		let range = |(a, b): (i32, i32)| {
			let (a, b) = (utf16_to_utf8(&text, a), utf16_to_utf8(&text, b));
			a.min(b)..a.max(b)
		};
		let selection = range(selection);
		let compose = compose.map(range).filter(|r| !r.is_empty());
		Self { text, selection, compose }
	}

	/// The text outside of the composing region: what the application should hold.
	pub(crate) fn committed(&self) -> String {
		match &self.compose {
			Some(c) => format!("{}{}", &self.text[..c.start], &self.text[c.end..]),
			None => self.text.clone(),
		}
	}

	/// The composing text and the cursor range inside it (UTF-8 offsets relative to the preedit).
	pub(crate) fn preedit(&self) -> Option<(String, Option<(usize, usize)>)> {
		let c = self.compose.clone()?;
		let preedit = self.text[c.clone()].to_owned();
		let clamp = |v: usize| v.clamp(c.start, c.end) - c.start;
		let cursor = (self.selection.start >= c.start && self.selection.end <= c.end)
			.then(|| (clamp(self.selection.start), clamp(self.selection.end)));
		Some((preedit, cursor))
	}

	/// Where the cursor is in [`Self::committed`] text (the composing region is collapsed).
	pub(crate) fn committed_cursor(&self) -> usize {
		let cursor = self.selection.end;
		match &self.compose {
			Some(c) if cursor >= c.end => cursor - (c.end - c.start),
			Some(c) if cursor > c.start => c.start,
			_ => cursor,
		}
	}
}

/// An edit of the application's (committed) text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextEdit {
	/// `Ime::DeleteSurrounding`: bytes before / after the cursor.
	Delete { before_bytes: usize, after_bytes: usize },
	/// `Ime::Commit`: inserted at the cursor.
	Commit(String),
}

/// Computes the edits turning `old` (with the cursor at `old_cursor`) into `new`.
///
/// The changed span is found by removing the common prefix and suffix. It is expressed as a
/// deletion around the cursor followed by a commit, which is what the preedit / commit model can
/// represent. When the changed span does not contain the cursor, the cursor is assumed to be at
/// the start of the change (the IME moved it there).
pub(crate) fn diff(old: &str, old_cursor: usize, new: &str) -> Vec<TextEdit> {
	if old == new {
		return Vec::new();
	}

	let prefix = common_prefix(old, new);
	let suffix = common_suffix(&old[prefix..], &new[prefix..]);
	let removed = prefix..old.len() - suffix;
	let inserted = &new[prefix..new.len() - suffix];

	let cursor = if removed.contains(&old_cursor) || old_cursor == removed.end {
		old_cursor
	} else {
		removed.start
	};

	let mut edits = Vec::new();
	let (before_bytes, after_bytes) = (cursor - removed.start, removed.end - cursor);
	if before_bytes > 0 || after_bytes > 0 {
		edits.push(TextEdit::Delete { before_bytes, after_bytes });
	}
	if !inserted.is_empty() {
		edits.push(TextEdit::Commit(inserted.to_owned()));
	}
	edits
}

fn common_prefix(a: &str, b: &str) -> usize {
	a.char_indices()
		.zip(b.chars())
		.find(|((_, ca), cb)| ca != cb)
		.map_or(a.len().min(b.len()), |((i, _), _)| i)
}

fn common_suffix(a: &str, b: &str) -> usize {
	a.chars()
		.rev()
		.zip(b.chars().rev())
		.take_while(|(ca, cb)| ca == cb)
		.map(|(c, _)| c.len_utf8())
		.sum()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn utf16_offsets() {
		let s = "a😀b日";
		assert_eq!(utf16_to_utf8(s, 0), 0);
		assert_eq!(utf16_to_utf8(s, 1), 1);
		// Inside the surrogate pair: rounded down.
		assert_eq!(utf16_to_utf8(s, 2), 1);
		assert_eq!(utf16_to_utf8(s, 3), 5);
		assert_eq!(utf16_to_utf8(s, 5), s.len());
		assert_eq!(utf16_to_utf8(s, 99), s.len());
		assert_eq!(utf16_to_utf8(s, -1), 0);
		assert_eq!(utf8_to_utf16(s, 5), 3);
		assert_eq!(utf8_to_utf16(s, 3), 1);
		assert_eq!(utf8_to_utf16(s, s.len()), 5);
	}

	#[test]
	fn state_from_utf16() {
		// "にほんご" with "ほん" composing, cursor at the end of the composition.
		let state = EditorState::from_utf16("にほんご".into(), (3, 3), Some((1, 3)));
		assert_eq!(state.compose, Some(3..9));
		assert_eq!(state.committed(), "にご");
		assert_eq!(state.preedit(), Some(("ほん".into(), Some((6, 6)))));
		assert_eq!(state.committed_cursor(), 3);
	}

	#[test]
	fn empty_compose_is_none() {
		let state = EditorState::from_utf16("abc".into(), (1, 1), Some((1, 1)));
		assert_eq!(state.compose, None);
		assert_eq!(state.preedit(), None);
	}

	#[test]
	fn diff_append() {
		assert_eq!(diff("ab", 2, "abc"), vec![TextEdit::Commit("c".into())]);
	}

	#[test]
	fn diff_backspace() {
		assert_eq!(
			diff("abc", 3, "ab"),
			vec![TextEdit::Delete { before_bytes: 1, after_bytes: 0 }]
		);
	}

	#[test]
	fn diff_replace_word_before_cursor() {
		// Autocorrect "teh" -> "the" with the cursor after the word.
		assert_eq!(
			diff("teh ", 3, "the "),
			vec![
				TextEdit::Delete { before_bytes: 2, after_bytes: 0 },
				TextEdit::Commit("he".into())
			]
		);
	}

	#[test]
	fn diff_multibyte() {
		assert_eq!(diff("日本", 6, "日本語"), vec![TextEdit::Commit("語".into())]);
		assert_eq!(
			diff("日本語", 9, "日本"),
			vec![TextEdit::Delete { before_bytes: 3, after_bytes: 0 }]
		);
	}

	#[test]
	fn diff_unchanged() {
		assert!(diff("same", 2, "same").is_empty());
	}

	#[test]
	fn diff_delete_after_cursor() {
		assert_eq!(
			diff("abcd", 1, "acd"),
			vec![TextEdit::Delete { before_bytes: 0, after_bytes: 1 }]
		);
	}
}
