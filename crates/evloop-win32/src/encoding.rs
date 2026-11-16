use std::{
    ffi::{OsStr, OsString},
    iter::once,
    os::windows::ffi::{OsStrExt, OsStringExt},
};

pub(crate) fn encode_wide(string: impl AsRef<OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(once(0)).collect()
}

pub(crate) fn decode_wide(mut wide_c_string: &[u16]) -> OsString {
    if let Some(null_position) = wide_c_string.iter().position(|char| *char == 0) {
        wide_c_string = &wide_c_string[..null_position];
    }

    OsString::from_wide(wide_c_string)
}
