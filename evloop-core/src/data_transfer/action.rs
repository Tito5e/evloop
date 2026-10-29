#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DndAction {
	Move,
	Copy,
	Link,
	Ask,
	Private,
}
