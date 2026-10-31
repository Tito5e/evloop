pub(crate) fn utf16_to_utf8_offset(s: &str, utf16_offset: usize) -> usize {
	let mut utf16_pos = 0;
	for (utf8_pos, ch) in s.char_indices() {
		if utf16_pos >= utf16_offset {
			return utf8_pos;
		}
		utf16_pos += ch.len_utf16();
		if utf16_pos > utf16_offset {
			return utf16_pos;
		}
	}
	s.len()
}
