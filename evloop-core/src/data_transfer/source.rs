use super::{TypeHint, TypedData};

/// Data offered by the application: written to the clipboard or dragged out of a window.
///
/// Holds one entry per format, with the same three kinds as received data. The data is stored
/// up front (no lazy rendering).
///
/// ```
/// # use evloop_core::data_transfer::DataSource;
/// let source = DataSource::new()
///     .with_text("hello")
///     .with_raw("text/html", b"<b>hello</b>".to_vec());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataSource {
	entries: Vec<(TypeHint, TypedData)>,
	sensitive: bool,
}

impl DataSource {
	pub fn new() -> Self {
		Self::default()
	}

	/// Offers a string ([`TypeHint::Text`]). Replaces a previous text entry.
	pub fn with_text(self, text: impl Into<String>) -> Self {
		self.with(TypeHint::Text, TypedData::String(text.into()))
	}

	/// Offers URIs ([`TypeHint::Uris`]). Replaces a previous URI entry.
	pub fn with_uris(self, uris: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.with(TypeHint::Uris, TypedData::Uris(uris.into_iter().map(Into::into).collect()))
	}

	/// Offers bytes under a platform format name ([`TypeHint::Raw`]); see the backend for which
	/// names it supports. Replaces a previous entry with the same name.
	pub fn with_raw(self, name: impl Into<String>, bytes: Vec<u8>) -> Self {
		self.with(TypeHint::Raw(name.into()), TypedData::Bytes(bytes))
	}

	/// Marks the data as sensitive (e.g. a password): the platform is asked not to keep it in
	/// clipboard history or show it in previews, where supported.
	pub fn sensitive(mut self, sensitive: bool) -> Self {
		self.sensitive = sensitive;
		self
	}

	pub fn is_sensitive(&self) -> bool {
		self.sensitive
	}

	/// The entries, in the order they were added.
	pub fn entries(&self) -> impl Iterator<Item = (&TypeHint, &TypedData)> {
		self.entries.iter().map(|(hint, data)| (hint, data))
	}

	pub fn get(&self, hint: &TypeHint) -> Option<&TypedData> {
		self.entries.iter().find(|(h, _)| h == hint).map(|(_, data)| data)
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	fn with(mut self, hint: TypeHint, data: TypedData) -> Self {
		debug_assert!(data.matches(&hint));
		match self.entries.iter_mut().find(|(h, _)| *h == hint) {
			Some(entry) => entry.1 = data,
			None => self.entries.push((hint, data)),
		}
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn entries_replace_same_hint() {
		let source = DataSource::new().with_text("a").with_raw("x", vec![1]).with_text("b");
		let hints: Vec<_> = source.entries().map(|(h, _)| h.clone()).collect();
		assert_eq!(hints, vec![TypeHint::Text, TypeHint::Raw("x".into())]);
		assert_eq!(source.get(&TypeHint::Text), Some(&TypedData::String("b".into())));
	}
}
