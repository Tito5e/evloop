use bitflags::bitflags;

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct WindowButtons: u8 {
		const CLOSE = 1 << 0;
		const MINIMIZE = 1 << 1;
		const MAXIMIZE = 1 << 2;
	}
}
