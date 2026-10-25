#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DataTransferId(i64);

impl DataTransferId {
	pub const fn into_raw(&self) -> i64 {
		self.0
	}

	pub const fn from_raw(id: i64) -> Self {
		Self(id)
	}
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DndAction {
	Move,
	Copy,
	Link,
	Ask,
	Private,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TypeHint {
	PlainText,
	UriList,
	Html,
	Rtf,
	Audio { extension_hint: Option<&'static str> },
	Image { extension_hint: Option<&'static str> },
}

impl TypeHint {
	pub fn matches(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::PlainText, Self::PlainText)
			| (Self::UriList, Self::UriList)
			| (Self::Html, Self::Html)
			| (Self::Rtf, Self::Rtf) => true,

			(
				Self::Audio { extension_hint: this_ext },
				Self::Audio { extension_hint: other_ext },
			)
			| (
				Self::Image { extension_hint: this_ext },
				Self::Image { extension_hint: other_ext },
			) => match (this_ext, other_ext) {
				(Some(this_ext), Some(other_ext)) => this_ext == other_ext,
				(None, _) | (_, None) => true,
			},

			_ => false,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SendData {
	Uris(Vec<String>),
	String(String),
	Bytes(Vec<u8>),
}

impl From<String> for SendData {
	fn from(value: String) -> Self {
		Self::String(value)
	}
}

impl From<Vec<u8>> for SendData {
	fn from(value: Vec<u8>) -> Self {
		Self::Bytes(value)
	}
}
