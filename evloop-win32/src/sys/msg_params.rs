#[inline(always)]
pub(crate) const fn get_xbutton_wparam(x: u32) -> u16 {
	hiword(x)
}

#[inline(always)]
pub(crate) const fn get_x_lparam(x: u32) -> i16 {
	loword(x) as _
}

#[inline(always)]
pub(crate) const fn get_y_lparam(x: u32) -> i16 {
	hiword(x) as _
}

#[inline(always)]
pub(crate) const fn primarylangid(lgid: u16) -> u16 {
	lgid & 0x3ff
}

#[inline(always)]
pub(crate) const fn loword(x: u32) -> u16 {
	(x & 0xffff) as u16
}

#[inline(always)]
pub(crate) const fn hiword(x: u32) -> u16 {
	((x >> 16) & 0xffff) as u16
}
