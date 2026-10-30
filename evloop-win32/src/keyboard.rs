use core::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalRawKey(u16);

impl fmt::Debug for LogicalRawKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let raw = self.0;
		f.debug_tuple("LogicalRawKey").field(&format_args!("0x{raw:04X}")).finish()
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalRawKey(u16);

impl fmt::Debug for PhysicalRawKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let raw = self.0;
		f.debug_tuple("PhysicalRawKey").field(&format_args!("0x{raw:04X}")).finish()
	}
}
