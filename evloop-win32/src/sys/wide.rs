use std::{
	ffi::{OsStr, OsString},
	os::windows::ffi::{OsStrExt, OsStringExt},
};

pub fn encode_wide(string: impl AsRef<OsStr>) -> Vec<u16> {
	// 明示的な終端文字を追加して返す
	string.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn decode_wide(mut wide_c_string: &[u16]) -> OsString {
	// TODO: 終端文字の検索を高速化する
	if let Some(null_pos) = wide_c_string.iter().position(|c| *c == 0) {
		wide_c_string = &wide_c_string[..null_pos];
	}

	OsString::from_wide(wide_c_string)
}
