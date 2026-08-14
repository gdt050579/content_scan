use std::path::Path;
use std::ffi::OsString;

/// A path or address identifying a piece of content.
///
/// In the common case this is a filesystem path, but it can also hold a
/// synthetic address such as `archive.zip://inner/file.txt` or
/// `C:\proc.exe::1000::module.dll`. A `ContentPath` always exposes a
/// valid UTF-8 [`printable string`](Self::as_printable_string) for
/// display and filtering; the *true* bytes are preserved separately when
/// the underlying OS path is not valid UTF-8, so the original file can
/// still be opened.
///
/// Construct one with [`from_str`](Self::from_str) for caller-supplied or
/// synthetic paths, or [`from_os`](Self::from_os) for real OS paths.
pub struct ContentPath {
    /// Always valid UTF-8. For a non-UTF-8 OS path this is the *lossy*
    /// rendering (invalid sequences replaced with U+FFFD); the faithful
    /// bytes live in `os`.
    path: String,
    /// Present only when `path` is a lossy view of a non-UTF-8 OS path.
    /// Holds the authoritative, openable path. `None` ⇒ `path` is exact.
    os: Option<OsString>,
}

impl ContentPath {
    /// Builds a `ContentPath` from a UTF-8 string.
    ///
    /// Use this for synthetic addresses (`zip://…`, `…::pid::…`) and for
    /// any path you already know is valid UTF-8. The result is always
    /// [`lossless`](Self::is_lossless).
    ///
    /// Do **not** stringify a real OS path yourself and pass it here — a
    /// non-UTF-8 filesystem path would lose the bytes needed to reopen it.
    /// Route real paths through [`from_os`](Self::from_os) instead.
    #[inline]
    pub fn from_str(s: &str) -> Self {
        Self { path: s.to_string(), os: None }
    }

    /// Builds a `ContentPath` from a real OS path.
    ///
    /// When the path is valid UTF-8 (the overwhelmingly common case) only
    /// the string is stored. When it is not — non-UTF-8 bytes on Unix, or
    /// unpaired surrogates on Windows — the original [`OsString`] is kept
    /// alongside a lossy string view, so the path stays both displayable
    /// and openable.
    pub fn from_os(p: &Path) -> Self {
        let os = p.as_os_str();
        match os.to_str() {
            Some(s) => Self { path: s.to_string(), os: None },
            None => Self {
                path: os.to_string_lossy().into_owned(),
                os: Some(os.to_owned()),
            },
        }
    }

    /// An empty `ContentPath` (exact, zero-length).
    ///
    /// Useful as a reusable buffer target; see [`clear`](Self::clear) and
    /// [`set_from_os`](Self::set_from_os).
    #[inline]
    pub fn empty() -> Self {
        Self { path: String::new(), os: None }
    }

    /// Resets to an empty, exact path, retaining the string's capacity.
    #[inline]
    pub fn clear(&mut self) {
        self.path.clear();
        self.os = None;
    }

    /// Overwrites this path in place from a UTF-8 string, reusing the
    /// existing allocation.
    ///
    /// Mirrors the reuse pattern of [`Entry::update`](crate::Entry::update)
    /// so hot loops (e.g. directory walks) don't reallocate per item.
    #[inline]
    pub fn set_from_str(&mut self, s: &str) {
        self.path.clear();
        self.path.push_str(s);
        self.os = None;
    }

    /// Overwrites this path in place from an OS path, reusing the string
    /// allocation on the common UTF-8 branch.
    pub fn set_from_os(&mut self, p: &Path) {
        let os = p.as_os_str();
        self.path.clear();
        match os.to_str() {
            Some(s) => {
                self.path.push_str(s);
                self.os = None;
            }
            None => {
                self.path.push_str(&os.to_string_lossy());
                self.os = Some(os.to_owned());
            }
        }
    }

    /// Borrows this path as an [`OsStr`](std::ffi::OsStr)-backed
    /// [`Path`] suitable for filesystem calls.
    ///
    /// Total on every platform. For a lossless path the string is
    /// reinterpreted; for a non-UTF-8 path the preserved [`OsString`] is
    /// used, so the returned `&Path` always names the original file.
    ///
    /// Note this also returns a `&Path` for synthetic addresses like
    /// `zip://…`; that is intentional — the caller (e.g. the file opener)
    /// will simply get an OS error when such a path is not a real file.
    #[inline]
    pub fn as_path(&self) -> &Path {
        match &self.os {
            Some(os) => Path::new(os),
            None => Path::new(&self.path),
        }
    }

    /// Borrows the bytes for inspection, filtering, and matching.
    ///
    /// On Unix these are the *faithful* path bytes (from the preserved
    /// [`OsString`] when the path is non-UTF-8, otherwise the string
    /// bytes). On Windows the underlying `OsString` encoding is not
    /// exposable, so this returns the (possibly lossy) string bytes — the
    /// best available representation there.
    ///
    /// For a [`lossless`](Self::is_lossless) path these bytes are exactly
    /// [`as_printable_string`](Self::as_printable_string)'s bytes; for a
    /// non-UTF-8 Unix path they may differ.
    #[cfg(unix)]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        use std::os::unix::ffi::OsStrExt;
        match &self.os {
            Some(os) => os.as_bytes(),
            None => self.path.as_bytes(),
        }
    }

    /// Borrows the bytes for inspection, filtering, and matching.
    ///
    /// See the Unix documentation on this method. On Windows the
    /// `OsString` encoding (WTF-8) is not exposable, so this returns the
    /// string bytes, which are lossy for non-UTF-8 paths.
    #[cfg(windows)]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.path.as_bytes()
    }

    /// Borrows a valid UTF-8 string safe to print or log.
    ///
    /// Always available and never fails. For a non-UTF-8 path this is the
    /// lossy rendering (U+FFFD in place of invalid sequences); use
    /// [`is_lossless`](Self::is_lossless) to tell whether it is exact.
    #[inline]
    pub fn as_printable_string(&self) -> &str {
        &self.path
    }

    /// Returns `true` when the printable string is a faithful, exact
    /// representation of the path.
    ///
    /// `false` means the path had bytes that cannot be represented in
    /// valid UTF-8, so [`as_printable_string`](Self::as_printable_string)
    /// is a lossy stand-in and should not be used as an identity or key.
    /// The exact path is still available via [`as_path`](Self::as_path)
    /// (and, on Unix, [`as_bytes`](Self::as_bytes)).
    #[inline]
    pub fn is_lossless(&self) -> bool {
        self.os.is_none()
    }
}

impl std::fmt::Debug for ContentPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Show the printable form and flag lossiness so debugging a
        // non-UTF-8 path is not silently misleading.
        if self.is_lossless() {
            write!(f, "ContentPath({:?})", self.path)
        } else {
            write!(f, "ContentPath({:?}, lossy)", self.path)
        }
    }
}

/// Ergonomic construction: accept `&str`, `&Path`, or an owned
/// `ContentPath` wherever a path argument is taken.
///
/// Each impl routes to the constructor that does the cheapest correct
/// thing for its input, so `&str` never pays a validity scan and `&Path`
/// gets the platform-correct handling.
pub trait IntoContentPath {
    fn into_content_path(self) -> ContentPath;
}
impl IntoContentPath for &str {
    #[inline]
    fn into_content_path(self) -> ContentPath { ContentPath::from_str(self) }
}
impl IntoContentPath for &Path {
    #[inline]
    fn into_content_path(self) -> ContentPath { ContentPath::from_os(self) }
}
impl IntoContentPath for ContentPath {
    #[inline]
    fn into_content_path(self) -> ContentPath { self }
}