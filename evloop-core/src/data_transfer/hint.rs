/// Identifies a format a data transfer offers. A plain value type.
///
/// # Invariants
///
/// - Every native format maps to **exactly one** `TypeHint`. A native format reported as
///   [`Text`](Self::Text) or [`Uris`](Self::Uris) never also appears as [`Raw`](Self::Raw), and
///   several native formats may map to the same `Text` / `Uris` (e.g. the Windows text formats).
/// - The variant of the [`TypedData`](super::TypedData) delivered for a request is determined by
///   the requested hint alone: `Text` → `String`, `Uris` → `Uris`, `Raw` → `Bytes`.
/// - evloop does not guarantee that the content matches the name: it is what the source declared.
///   Validating the content is the receiver's responsibility.
///
/// The mapping of each backend's native formats is documented in that backend.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeHint {
	/// A string. Fetching it delivers [`TypedData::String`](super::TypedData::String), already
	/// converted from the platform's encoding to UTF-8.
	Text,
	/// A list of URIs; files are `file:` URIs (or platform-specific URIs such as Android
	/// `content:` URIs). Fetching it delivers [`TypedData::Uris`](super::TypedData::Uris).
	Uris,
	/// Any other format, by the name the platform reports for it (a MIME type, a Windows
	/// clipboard format name, ...). Not portable: the same content may have different names, and
	/// different encodings, on different platforms. Fetching it delivers
	/// [`TypedData::Bytes`](super::TypedData::Bytes).
	Raw(String),
}
