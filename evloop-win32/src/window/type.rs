#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum WindowType {
	#[default]
	Window,
	Popup,
}
