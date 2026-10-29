#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypedData {
	Text(String),
	Uris(Vec<String>),
	Bytes(Vec<u8>),
}

impl From<String> for TypedData {
	fn from(value: String) -> Self {
		Self::Text(value)
	}
}

impl From<Vec<u8>> for TypedData {
	fn from(value: Vec<u8>) -> Self {
		Self::Bytes(value)
	}
}
