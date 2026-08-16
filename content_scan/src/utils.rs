#[inline(always)]
pub(crate) fn get_file_name(path: &[u8]) -> &[u8] {
    if let Some(ofs) = path.iter().rposition(|&b| b == b'/' || b == b'\\') {
        &path[ofs + 1..]
    } else {
        path
    }
}

#[inline(always)]
pub(crate) fn get_extension(file_name: &[u8]) -> &[u8] {
    if let Some(ofs) = file_name.iter().rposition(|&b| b == b'.') {
        &file_name[ofs + 1..]
    } else {
        &[]
    }
}
pub(crate) fn contains_uppercase(buf: &[u8]) -> bool {
    for b in buf {
        if *b >= b'A' && *b <= b'Z' {
            return true;
        }
    }
    return false;
}
pub(crate) fn copy_lowercase<'a>(source: &[u8], output: &'a mut Vec<u8>) -> &'a [u8] {
    output.clear();
    output.extend_from_slice(source);
    output.make_ascii_lowercase();
    output.as_slice()
}
/// ASCII-lowercases a `'static` pattern for the matcher.
///
/// Already-lowercase strings are returned as-is (no allocation).
/// Mixed-case patterns are leaked once at filter build time.
pub(crate) fn ascii_lower_static(s: &'static str) -> &'static [u8] {
    let bytes = s.as_bytes();
    if !contains_uppercase(bytes) {
        bytes
    } else {
        let mut owned = bytes.to_vec();
        owned.make_ascii_lowercase();
        Box::leak(owned.into_boxed_slice())
    }
}