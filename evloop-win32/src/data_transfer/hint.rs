#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ReceivedData {
	/// MIME: application/*
	Binary,

	/// MIME: audio/*
	Audio,

	/// MIME: image/*
	Image,

	/// MIME: text/plain
	TextPlain,

	/// MIME: text/html
	TextHtml,
}
