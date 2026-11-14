/// What happens to dragged data when it is dropped.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DndAction {
	/// The data is moved: the source deletes it after the drop.
	Move,
	/// The data is copied.
	Copy,
	/// A link (reference) to the data is created.
	Link,
	/// Reserved for platforms where the user chooses the action after the drop (e.g. a menu on
	/// Wayland). No current backend uses it.
	Ask,
	/// Reserved for an application-defined action. No current backend uses it.
	Private,
}
