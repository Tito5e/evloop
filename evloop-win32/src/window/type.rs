#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorGrabMode {
	None,
	Confined,
	Locked,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ResizeDirection {
	East,
	North,
	NorthEast,
	NorthWest,
	South,
	SouthEast,
	SouthWest,
	West,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UserAttentionType {
	Critical,
	#[default]
	Informational,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum WindowType {
	#[default]
	Window,
	Popup,
}
