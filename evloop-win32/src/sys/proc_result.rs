use windows_sys::Win32::Foundation::WPARAM;

#[derive(Clone, Copy)]
pub(crate) enum ProcResult {
	DefWindowProc(WPARAM),
	Value(isize),
}
