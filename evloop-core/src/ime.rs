use bitflags::bitflags;

#[derive(Default)]
pub struct ImeCapabilities(ImeCapabilitiesFlags);

impl ImeCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn with_hint_and_purpose(self) -> Self {
        Self(self.0.union(ImeCapabilitiesFlags::HINT_AND_PURPOSE))
    }

    pub const fn without_hint_and_purpose(self) -> Self {
        Self(self.0.difference(ImeCapabilitiesFlags::HINT_AND_PURPOSE))
    }

    pub const fn hint_and_purpose(&self) -> bool {
        self.0.contains(ImeCapabilitiesFlags::HINT_AND_PURPOSE)
    }

    pub const fn with_cursor_area(self) -> Self {
        Self(self.0.union(ImeCapabilitiesFlags::CURSOR_AREA))
    }

    pub const fn without_cursor_area(self) -> Self {
        Self(self.0.difference(ImeCapabilitiesFlags::CURSOR_AREA))
    }

    pub const fn cursor_area(&self) -> bool {
        self.0.contains(ImeCapabilitiesFlags::CURSOR_AREA)
    }

    pub const fn with_surrounding_text(self) -> Self {
        Self(self.0.union(ImeCapabilitiesFlags::SURROUNDING_TEXT))
    }

    pub const fn without_surrounding_text(self) -> Self {
        Self(self.0.difference(ImeCapabilitiesFlags::SURROUNDING_TEXT))
    }

    pub const fn surrounding_text(&self) -> bool {
        self.0.contains(ImeCapabilitiesFlags::SURROUNDING_TEXT)
    }
}

bitflags! {
    #[derive(Default)]
    pub(crate) struct ImeCapabilitiesFlags: u8 {
        const HINT_AND_PURPOSE = 1 << 0;
        const CURSOR_AREA = 1 << 1;
        const SURROUNDING_TEXT = 1 << 2;
    }
}
