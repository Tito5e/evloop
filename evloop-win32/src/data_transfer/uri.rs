use std::{
	borrow::Cow,
	path::{Component, Path, PathBuf, Prefix},
};

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, percent_encode};

const URI_SEGMENT: &AsciiSet = &CONTROLS
	.add(b' ')
	.add(b'"')
	.add(b'#')
	.add(b'%')
	.add(b'<')
	.add(b'>')
	.add(b'?')
	.add(b'`')
	.add(b'{')
	.add(b'}');

pub(crate) fn file_path_to_uri(path: &Path) -> Option<String> {
	let mut components = path.components();
	let mut uri = String::from("file://");

	match components.next()? {
		Component::Prefix(prefix) => match prefix.kind() {
			Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
				uri.push('/');
				uri.push(letter.to_ascii_uppercase() as char);
				uri.push(':');
			}
			Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
				uri.extend(percent_encode(server.to_str()?.as_bytes(), URI_SEGMENT));
				uri.push('/');
				uri.extend(percent_encode(share.to_str()?.as_bytes(), URI_SEGMENT));
			}
			_ => return None,
		},
		_ => return None,
	}

	if !matches!(components.next(), Some(Component::RootDir)) {
		return None;
	}
	for component in components {
		let Component::Normal(segment) = component else { return None };
		uri.push('/');
		uri.extend(percent_encode(segment.to_str()?.as_bytes(), URI_SEGMENT));
	}

	Some(uri)
}

pub(crate) fn uri_to_file_path(uri: &str) -> Option<PathBuf> {
	fn decode(segment: &str) -> Option<Cow<'_, str>> {
		percent_decode_str(segment).decode_utf8().ok()
	}

	let rest = uri.strip_prefix("file://")?;
	let (host, path) = rest.split_once('/').unwrap_or((rest, ""));

	let mut out = String::new();
	if host.is_empty() {
		let mut segments = path.split('/');
		let drive = decode(segments.next()?)?;
		let bytes = drive.as_bytes();
		if bytes.len() != 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
			return None;
		}
		out.push_str(&drive);
		for segment in segments {
			out.push('\\');
			out.push_str(&decode(segment)?);
		}
	} else {
		out.push_str("\\\\");
		out.push_str(&decode(host)?);
		for segment in path.split('/').filter(|s| !s.is_empty()) {
			out.push('\\');
			out.push_str(&decode(segment)?);
		}
	}

	Some(PathBuf::from(out))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn disk_path_round_trips_through_uri() {
		let uri = file_path_to_uri(Path::new(r"C:\Users\me\my file.txt")).unwrap();
		assert_eq!(uri, "file:///C:/Users/me/my%20file.txt");
		assert_eq!(uri_to_file_path(&uri).unwrap(), PathBuf::from(r"C:\Users\me\my file.txt"));
	}

	#[test]
	fn unc_path_round_trips_through_uri() {
		let uri = file_path_to_uri(Path::new(r"\\server\share\dir\file.txt")).unwrap();
		assert_eq!(uri, "file://server/share/dir/file.txt");
		assert_eq!(uri_to_file_path(&uri).unwrap(), PathBuf::from(r"\\server\share\dir\file.txt"));
	}

	#[test]
	fn relative_path_has_no_uri() {
		assert_eq!(file_path_to_uri(Path::new(r"me\file.txt")), None);
	}

	#[test]
	fn percent_and_unicode_round_trip() {
		let uri = file_path_to_uri(Path::new(r"C:\100% café")).unwrap();
		assert_eq!(uri, "file:///C:/100%25%20caf%C3%A9");
		assert_eq!(uri_to_file_path(&uri).unwrap(), PathBuf::from(r"C:\100% café"));
	}
}
