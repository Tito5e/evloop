/// The edge or corner of a window that `Window::drag_resize_window` resizes from.
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
