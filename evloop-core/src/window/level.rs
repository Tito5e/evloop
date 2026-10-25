#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
pub enum WindowLevel {
	AlwaysOnBottom,

	#[default]
	Normal,

	AlwaysOnTop,
}
