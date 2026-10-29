#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeHint {
	PlainText,
	UriList,
	Html,
	Rtf,
	Raw(String),
}
