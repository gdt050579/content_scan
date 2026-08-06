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