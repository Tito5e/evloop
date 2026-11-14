use super::TypeHint;

/// Data received from a data transfer, already parsed. Which variant is delivered is determined
/// by the requested [`TypeHint`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypedData {
	/// The result of [`TypeHint::Text`]: converted to UTF-8 by evloop.
	String(String),
	/// The result of [`TypeHint::Uris`].
	Uris(Vec<String>),
	/// The result of [`TypeHint::Raw`]: the bytes as the platform delivered them, unprocessed
	/// (PNG stays PNG, a Windows DIB stays a DIB; nothing is decoded or converted).
	Bytes(Vec<u8>),
}

impl TypedData {
	/// Whether this is the variant delivered for `hint`.
	pub fn matches(&self, hint: &TypeHint) -> bool {
		matches!(
			(self, hint),
			(Self::String(_), TypeHint::Text)
				| (Self::Uris(_), TypeHint::Uris)
				| (Self::Bytes(_), TypeHint::Raw(_))
		)
	}
}
