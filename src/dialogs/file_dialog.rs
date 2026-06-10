//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File Open/Save dialog wrappers.
//!
//! Wraps `GetOpenFileNameW` and `GetSaveFileNameW` from the Win32
//! Common Dialogs API (comdlg32.dll).

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::*;

/// Style of the file dialog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileDialogStyle {
    Open,
    Save,
}

/// A file picker dialog (open or save).
///
/// Build the dialog with setter methods, then call `show_modal()` to
/// present it to the user and retrieve the selected path.
///
/// For multi-file selection, set `multi_select(true)` before calling
/// [`FileDialog::show_modal_multi`] (which returns one or more paths).
/// Calling `show_modal()` with multi-select enabled still works but
/// will only return the first selected file.
pub struct FileDialog {
    #[cfg(target_os = "windows")]
    parent_hwnd: HWND,
    style: FileDialogStyle,
    title: String,
    default_dir: String,
    default_file: String,
    wildcard: String,
    multi_select: bool,
}

impl FileDialog {
    /// Create a new file dialog associated with the given frame.
    pub fn new(frame: &Frame, style: FileDialogStyle) -> Self {
        FileDialog {
            #[cfg(target_os = "windows")]
            parent_hwnd: frame.hwnd(),
            style,
            title: String::new(),
            default_dir: String::new(),
            default_file: String::new(),
            wildcard: String::new(),
            multi_select: false,
        }
    }

    /// Set the dialog title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Set the file filter in wxWidgets format.
    ///
    /// Format: `"Description|pattern|Description2|pattern2"`
    ///
    /// Example: `"Text files (*.txt)|*.txt|All files (*.*)|*.*"`
    pub fn set_wildcard(&mut self, wildcard: &str) {
        self.wildcard = wildcard.to_string();
    }

    /// Set the initial directory.
    pub fn set_directory(&mut self, dir: &str) {
        self.default_dir = dir.to_string();
    }

    /// Set the default filename.
    pub fn set_filename(&mut self, name: &str) {
        self.default_file = name.to_string();
    }

    /// Enable or disable multi-file selection (`OFN_ALLOWMULTISELECT`).
    ///
    /// When enabled, [`FileDialog::show_modal_multi`] returns a `Vec`
    /// of all files the user selected. When disabled (the default),
    /// the user can only pick a single file.
    ///
    /// **Note:** [`FileDialog::show_modal`] (single-file variant)
    /// ignores this flag and always returns the first selected file.
    /// To retrieve every selected file, use
    /// [`FileDialog::show_modal_multi`].
    ///
    /// This is a no-op on non-Windows platforms.
    pub fn set_multi_select(&mut self, enabled: bool) -> &mut Self {
        self.multi_select = enabled;
        self
    }

    /// Returns `true` if multi-file selection is enabled.
    pub fn is_multi_select(&self) -> bool {
        self.multi_select
    }

    /// Show the dialog modally and return the selected file path.
    ///
    /// Returns `Some(path)` if the user selected a file, or `None` if
    /// the dialog was cancelled.
    ///
    /// **Note:** this method always returns at most one path. If you
    /// enabled multi-select with [`FileDialog::set_multi_select`], use
    /// [`FileDialog::show_modal_multi`] instead to retrieve every
    /// selected file.
    pub fn show_modal(&mut self) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            // Convert wxWidgets wildcard format to Win32 filter format.
            // wxWidgets: "Desc1|*.ext1|Desc2|*.ext2"
            // Win32:     "Desc1\0*.ext1\0Desc2\0*.ext2\0\0"
            let filter_wide = self.wildcard_to_win32_filter();

            // Store wide strings in locals so they outlive the OPENFILENAMEW call
            let title_wide_vec;
            let title_ptr: PCWSTR = if self.title.is_empty() {
                std::ptr::null()
            } else {
                title_wide_vec = to_wide(&self.title);
                title_wide_vec.as_ptr()
            };

            let dir_wide_vec;
            let dir_ptr: PCWSTR = if self.default_dir.is_empty() {
                std::ptr::null()
            } else {
                dir_wide_vec = to_wide(&self.default_dir);
                dir_wide_vec.as_ptr()
            };

            // Buffer for the selected file path
            const MAX_PATH_BUF: u32 = 4096;
            let mut file_buf = vec![0u16; MAX_PATH_BUF as usize];

            // Set default filename into the buffer
            if !self.default_file.is_empty() {
                let default_wide: Vec<u16> = self.default_file.encode_utf16().collect();
                let copy_len = default_wide.len().min(file_buf.len() - 1);
                file_buf[..copy_len].copy_from_slice(&default_wide[..copy_len]);
            }

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = self.parent_hwnd;
            ofn.hInstance = std::ptr::null_mut();
            ofn.lpstrFilter = filter_wide.as_ptr();
            ofn.lpstrCustomFilter = std::ptr::null_mut();
            ofn.nMaxCustFilter = 0;
            ofn.nFilterIndex = 1;
            ofn.lpstrFile = file_buf.as_mut_ptr();
            ofn.nMaxFile = MAX_PATH_BUF;
            ofn.lpstrFileTitle = std::ptr::null_mut();
            ofn.nMaxFileTitle = 0;
            ofn.lpstrInitialDir = dir_ptr;
            ofn.lpstrTitle = title_ptr;
            ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
            ofn.nFileOffset = 0;
            ofn.nFileExtension = 0;
            ofn.lpstrDefExt = std::ptr::null();
            ofn.lCustData = 0;
            ofn.lpfnHook = None;
            ofn.lpTemplateName = std::ptr::null();
            ofn.pvReserved = std::ptr::null_mut();
            ofn.dwReserved = 0;
            ofn.FlagsEx = 0;

            let result = match self.style {
                // SAFETY: FFI call to GetOpenFileNameW; the dialog struct is fully initialised and the user callback is the matching Rust closure.
                FileDialogStyle::Open => unsafe { GetOpenFileNameW(&mut ofn) },
                // SAFETY: FFI call to GetSaveFileNameW; the dialog struct is fully initialised and the user callback is the matching Rust closure.
                FileDialogStyle::Save => unsafe { GetSaveFileNameW(&mut ofn) },
            };

            if result != 0 {
                // Success — extract the file path from the buffer
                let len = file_buf
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(file_buf.len());
                if len > 0 {
                    Some(String::from_utf16_lossy(&file_buf[..len]))
                } else {
                    None
                }
            } else {
                None
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            None
        }
    }

    /// Show the dialog modally and return all selected file paths.
    ///
    /// Returns a `Vec` of file paths if the user selected one or more
    /// files, or an empty `Vec` if the dialog was cancelled.
    ///
    /// This method enables multi-select on the Win32 dialog
    /// (`OFN_ALLOWMULTISELECT`) regardless of whether
    /// [`FileDialog::set_multi_select`] was called — callers do not
    /// need to set the flag manually.
    ///
    /// For single-file selection prefer the simpler
    /// [`FileDialog::show_modal`] which returns `Option<String>`.
    ///
    /// On non-Windows platforms, returns an empty `Vec`.
    pub fn show_modal_multi(&mut self) -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            // Multi-select buffers can grow large; allocate a generous
            // working buffer. 32 KiB (in u16 chars, i.e. 64 KiB) is
            // what the Win32 documentation recommends.
            const MAX_MULTI_BUF: u32 = 32 * 1024;
            let mut file_buf: Vec<u16> = vec![0u16; MAX_MULTI_BUF as usize];

            // Set default filename into the buffer (only if single-select)
            if !self.default_file.is_empty() && !self.multi_select {
                let default_wide: Vec<u16> = self.default_file.encode_utf16().collect();
                let copy_len = default_wide.len().min(file_buf.len() - 1);
                file_buf[..copy_len].copy_from_slice(&default_wide[..copy_len]);
            }

            let filter_wide = self.wildcard_to_win32_filter();

            let title_wide_vec;
            let title_ptr: PCWSTR = if self.title.is_empty() {
                std::ptr::null()
            } else {
                title_wide_vec = to_wide(&self.title);
                title_wide_vec.as_ptr()
            };

            let dir_wide_vec;
            let dir_ptr: PCWSTR = if self.default_dir.is_empty() {
                std::ptr::null()
            } else {
                dir_wide_vec = to_wide(&self.default_dir);
                dir_wide_vec.as_ptr()
            };

            // SAFETY: Win32 FFI call with validated arguments and a buffer large enough for the output.
            let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = self.parent_hwnd;
            ofn.hInstance = std::ptr::null_mut();
            ofn.lpstrFilter = filter_wide.as_ptr();
            ofn.lpstrCustomFilter = std::ptr::null_mut();
            ofn.nMaxCustFilter = 0;
            ofn.nFilterIndex = 1;
            ofn.lpstrFile = file_buf.as_mut_ptr();
            ofn.nMaxFile = MAX_MULTI_BUF;
            ofn.lpstrFileTitle = std::ptr::null_mut();
            ofn.nMaxFileTitle = 0;
            ofn.lpstrInitialDir = dir_ptr;
            ofn.lpstrTitle = title_ptr;
            ofn.Flags = OFN_FILEMUSTEXIST
                | OFN_PATHMUSTEXIST
                | OFN_NOCHANGEDIR
                | OFN_ALLOWMULTISELECT
                | OFN_EXPLORER;
            ofn.nFileOffset = 0;
            ofn.nFileExtension = 0;
            ofn.lpstrDefExt = std::ptr::null();
            ofn.lCustData = 0;
            ofn.lpfnHook = None;
            ofn.lpTemplateName = std::ptr::null();
            ofn.pvReserved = std::ptr::null_mut();
            ofn.dwReserved = 0;
            ofn.FlagsEx = 0;

            // Save dialogs do not support multi-select; bail out.
            if matches!(self.style, FileDialogStyle::Save) {
                return Vec::new();
            }

            // SAFETY: FFI call to GetOpenFileNameW; the dialog struct is fully initialised.
            let result = unsafe { GetOpenFileNameW(&mut ofn) };

            if result != 0 {
                parse_multiselect_buffer(&file_buf, ofn.nFileOffset as usize)
            } else {
                Vec::new()
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            Vec::new()
        }
    }

    /// Convert wxWidgets wildcard format to Win32 filter format.
    ///
    /// wxWidgets uses `|` to separate pairs: `"Desc1|*.ext1|Desc2|*.ext2"`
    /// Win32 uses null-terminated pairs with a double-null terminator:
    /// `"Desc1\0*.ext1\0Desc2\0*.ext2\0\0"`
    #[cfg(target_os = "windows")]
    fn wildcard_to_win32_filter(&self) -> Vec<u16> {
        if self.wildcard.is_empty() {
            // Return a double-null terminated empty filter
            return vec![0, 0];
        }

        let parts: Vec<&str> = self.wildcard.split('|').collect();
        let mut result = Vec::new();

        // Parts come in pairs: description, pattern, description, pattern, ...
        let mut i = 0;
        while i + 1 < parts.len() {
            // Encode description
            result.extend(parts[i].encode_utf16());
            result.push(0); // null separator
                            // Encode pattern
            result.extend(parts[i + 1].encode_utf16());
            result.push(0); // null separator
            i += 2;
        }

        // Final null terminator
        result.push(0);
        result
    }
}

// OFN flags used by our dialog
#[cfg(target_os = "windows")]
const OFN_FILEMUSTEXIST: OPEN_FILENAME_FLAGS = 0x00001000;
#[cfg(target_os = "windows")]
const OFN_PATHMUSTEXIST: OPEN_FILENAME_FLAGS = 0x00000800;
#[cfg(target_os = "windows")]
const OFN_NOCHANGEDIR: OPEN_FILENAME_FLAGS = 0x00000008;
#[cfg(target_os = "windows")]
const OFN_ALLOWMULTISELECT: OPEN_FILENAME_FLAGS = 0x00000200;
#[cfg(target_os = "windows")]
const OFN_EXPLORER: OPEN_FILENAME_FLAGS = 0x00080000;

/// Parse a Win32 multi-select buffer produced by `GetOpenFileNameW`
/// with `OFN_ALLOWMULTISELECT` set.
///
/// The buffer layout is:
///
/// - **single file selection** (no `OFN_ALLOWMULTISELECT`):
///   `"full path\0\0"` — one null-terminated string.
/// - **multi file selection**:
///   `"dir\0file1\0file2\0...\0\0"` — a directory prefix
///   (terminated by a null) followed by one null-terminated file
///   name per selection, then a second null marking the end of the
///   list.
///
/// `file_offset` is the `nFileOffset` value written by
/// `GetOpenFileNameW`. It is the index (in `u16` code units) of the
/// first character of the first file name. We accept it for API
/// parity with the Win32 call but do **not** use it for path
/// reconstruction: the first null-terminated string in the buffer
/// is, by definition, the full directory, and we just prepend it to
/// each filename. (wxWidgets does the same.)
///
/// Returns:
/// - an empty `Vec` if the buffer contains no paths,
/// - a single-element `Vec` for a single-file selection,
/// - a `Vec` with one element per selected file for multi-select.
#[cfg(target_os = "windows")]
pub(crate) fn parse_multiselect_buffer(buf: &[u16], _file_offset: usize) -> Vec<String> {
    // Walk the buffer and collect null-terminated strings until we
    // hit an empty string (= double null, end of list).
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0usize;
    let mut end_of_list = false;
    for (i, &c) in buf.iter().enumerate() {
        if c == 0 {
            if i == start {
                // Two consecutive nulls: end of the list.
                end_of_list = true;
                break;
            }
            let s = String::from_utf16_lossy(&buf[start..i]);
            parts.push(s);
            start = i + 1;
        }
    }
    if !end_of_list && start < buf.len() {
        // Buffer did not contain a trailing null — treat the
        // remaining tail as a final path. (Defensive: Win32 always
        // terminates the list, but we handle the corner case.)
        let s = String::from_utf16_lossy(&buf[start..]);
        if !s.is_empty() {
            parts.push(s);
        }
    }

    if parts.is_empty() {
        return Vec::new();
    }
    if parts.len() == 1 {
        return parts;
    }

    // Multi-select: first entry is the directory, the rest are
    // filenames. The directory never has a trailing separator in
    // the Win32 buffer, so add one before joining.
    let dir = &parts[0];
    let dir_with_sep = if dir.is_empty() || dir.ends_with('\\') || dir.ends_with('/') {
        dir.clone()
    } else {
        format!("{dir}\\")
    };

    parts
        .iter()
        .skip(1)
        .filter(|name| !name.is_empty())
        .map(|name| {
            // If the filename is already absolute (UNC root or
            // drive letter) keep it as-is — the user selected it
            // explicitly and it isn't in the directory above.
            let is_absolute =
                name.starts_with("\\\\") || (name.len() >= 2 && name.as_bytes()[1] == b':');
            if is_absolute || dir.is_empty() {
                name.clone()
            } else {
                format!("{dir_with_sep}{name}")
            }
        })
        .collect()
}

#[cfg(test)]
impl FileDialog {
    /// Construct a `FileDialog` for unit tests without a real `Frame`.
    ///
    /// The parent `HWND` is set to null, which is harmless for
    /// exercising the wildcard / multi-select state code paths. The
    /// `show_modal` / `show_modal_multi` calls themselves are **not**
    /// exercised by these tests — those require a real Win32 dialog
    /// and are covered by the windowed smoke test in
    /// `examples/showcase_all.rs`.
    pub(crate) fn new_for_test(multi_select: bool) -> Self {
        Self {
            #[cfg(target_os = "windows")]
            parent_hwnd: std::ptr::null_mut(),
            style: FileDialogStyle::Open,
            title: String::new(),
            default_dir: String::new(),
            default_file: String::new(),
            wildcard: String::new(),
            multi_select,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------- parse_multiselect_buffer --------

    fn to_wide_vec(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn parse_empty_buffer_returns_empty() {
        let buf = vec![0u16, 0, 0, 0];
        let out = parse_multiselect_buffer(&buf, 0);
        assert!(out.is_empty(), "empty buffer must yield empty Vec");
    }

    #[test]
    fn parse_all_zero_buffer_returns_empty() {
        let buf = vec![0u16; 64];
        let out = parse_multiselect_buffer(&buf, 0);
        assert!(out.is_empty(), "all-zero buffer must yield empty Vec");
    }

    #[test]
    fn parse_single_file_returns_single_path() {
        // "C:\foo.txt\0\0"
        let mut buf = to_wide_vec(r"C:\foo.txt");
        buf.push(0);
        buf.push(0);
        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out, vec![r"C:\foo.txt".to_string()]);
    }

    #[test]
    fn parse_multi_select_two_files() {
        // "C:\dir\0a.txt\0b.txt\0\0"
        let mut buf = to_wide_vec(r"C:\dir");
        buf.push(0);
        buf.extend(to_wide_vec("a.txt"));
        buf.push(0);
        buf.extend(to_wide_vec("b.txt"));
        buf.push(0);
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(
            out,
            vec![r"C:\dir\a.txt".to_string(), r"C:\dir\b.txt".to_string()]
        );
    }

    #[test]
    fn parse_multi_select_three_files() {
        let mut buf = to_wide_vec(r"C:\data");
        buf.push(0);
        for name in ["one.txt", "two.txt", "three.txt"] {
            buf.extend(to_wide_vec(name));
            buf.push(0);
        }
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], r"C:\data\one.txt");
        assert_eq!(out[1], r"C:\data\two.txt");
        assert_eq!(out[2], r"C:\data\three.txt");
    }

    #[test]
    fn parse_multi_select_with_trailing_backslash_in_dir() {
        // Directory already ends in "\", must NOT get a second "\".
        let mut buf = to_wide_vec(r"C:\dir\");
        buf.push(0);
        buf.extend(to_wide_vec("a.txt"));
        buf.push(0);
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out, vec![r"C:\dir\a.txt".to_string()]);
    }

    #[test]
    fn parse_multi_select_offset_does_not_alter_output() {
        // The full directory "C:\Users\Me" is in the buffer but the
        // shared prefix according to nFileOffset is only "C:\" (3
        // chars). Path reconstruction always uses the *whole*
        // directory (parts[0]) regardless of file_offset, so the
        // output is the same as if the offset were 0 or 1000.
        let mut buf = to_wide_vec(r"C:\Users\Me");
        buf.push(0);
        buf.extend(to_wide_vec("a.txt"));
        buf.push(0);
        buf.extend(to_wide_vec("b.txt"));
        buf.push(0);
        buf.push(0);

        let with_offset = parse_multiselect_buffer(&buf, r"C:\".len());
        let without_offset = parse_multiselect_buffer(&buf, 0);
        let huge_offset = parse_multiselect_buffer(&buf, 9999);
        assert_eq!(with_offset, without_offset);
        assert_eq!(with_offset, huge_offset);
        assert_eq!(with_offset.len(), 2);
        // Whole dir is the prefix, not just the shared "C:\".
        assert!(with_offset[0].starts_with(r"C:\Users\Me"));
    }

    #[test]
    fn parse_multi_select_keeps_absolute_filenames() {
        // Filename that already has a drive letter must not be
        // prepended with the directory.
        let mut buf = to_wide_vec(r"C:\dir");
        buf.push(0);
        buf.extend(to_wide_vec(r"D:\other\x.txt"));
        buf.push(0);
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out, vec![r"D:\other\x.txt".to_string()]);
    }

    #[test]
    fn parse_multi_select_unc_prefix_kept() {
        // UNC path: \\server\share
        let mut buf = to_wide_vec(r"\\server\share");
        buf.push(0);
        buf.extend(to_wide_vec("x.txt"));
        buf.push(0);
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out, vec![r"\\server\share\x.txt".to_string()]);
    }

    #[test]
    fn parse_multi_select_skips_empty_filenames() {
        // An empty filename (consecutive nulls in the middle) is the
        // terminator — anything past it must be ignored.
        let mut buf = to_wide_vec(r"C:\dir");
        buf.push(0);
        buf.extend(to_wide_vec("a.txt"));
        buf.push(0);
        buf.push(0); // <-- terminator
        buf.extend(to_wide_vec("should_be_ignored.txt"));
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out, vec![r"C:\dir\a.txt".to_string()]);
    }

    #[test]
    fn parse_multi_select_offset_clamps_to_dir_length() {
        // If the caller passes an absurdly large file_offset, we
        // clamp it to the directory length so we never panic.
        let mut buf = to_wide_vec(r"C:\dir");
        buf.push(0);
        buf.extend(to_wide_vec("a.txt"));
        buf.push(0);
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 9999);
        assert_eq!(out, vec![r"C:\dir\a.txt".to_string()]);
    }

    #[test]
    fn parse_multi_select_with_forward_slash_dir() {
        // Defensive: a directory ending in "/" must not get a "\"
        // appended to it.
        let mut buf = to_wide_vec("C:/dir/");
        buf.push(0);
        buf.extend(to_wide_vec("a.txt"));
        buf.push(0);
        buf.push(0);

        let out = parse_multiselect_buffer(&buf, 0);
        assert_eq!(out, vec!["C:/dir/a.txt".to_string()]);
    }

    // -------- wildcard_to_win32_filter --------

    #[test]
    fn wildcard_empty_returns_double_null() {
        let dlg = FileDialog::new_for_test(false);
        let f = dlg.wildcard_to_win32_filter();
        assert_eq!(f, vec![0u16, 0]);
    }

    #[test]
    fn wildcard_single_pair() {
        let mut dlg = FileDialog::new_for_test(false);
        dlg.set_wildcard("Text files (*.txt)|*.txt");
        let f = dlg.wildcard_to_win32_filter();
        // Expected: "Text files (*.txt)\0*.txt\0\0"
        let mut expected = to_wide_vec("Text files (*.txt)");
        expected.push(0);
        expected.extend(to_wide_vec("*.txt"));
        expected.push(0);
        expected.push(0);
        assert_eq!(f, expected);
    }

    #[test]
    fn wildcard_two_pairs() {
        let mut dlg = FileDialog::new_for_test(false);
        dlg.set_wildcard("Text files (*.txt)|*.txt|All files (*.*)|*.*");
        let f = dlg.wildcard_to_win32_filter();
        let mut expected = to_wide_vec("Text files (*.txt)");
        expected.push(0);
        expected.extend(to_wide_vec("*.txt"));
        expected.push(0);
        expected.extend(to_wide_vec("All files (*.*)"));
        expected.push(0);
        expected.extend(to_wide_vec("*.*"));
        expected.push(0);
        expected.push(0);
        assert_eq!(f, expected);
    }

    #[test]
    fn wildcard_odd_parts_ignored() {
        // 3 parts = one full pair + a dangling description that has
        // no pattern. Must be ignored, not mis-interpreted.
        let mut dlg = FileDialog::new_for_test(false);
        dlg.set_wildcard("A|*.a|B");
        let f = dlg.wildcard_to_win32_filter();
        let mut expected = to_wide_vec("A");
        expected.push(0);
        expected.extend(to_wide_vec("*.a"));
        expected.push(0);
        expected.push(0);
        assert_eq!(f, expected);
    }

    // -------- multi_select state --------

    #[test]
    fn default_multi_select_is_false() {
        let dlg = FileDialog::new_for_test(false);
        assert!(!dlg.is_multi_select());
    }

    #[test]
    fn new_for_test_true_sets_multi_select() {
        let dlg = FileDialog::new_for_test(true);
        assert!(dlg.is_multi_select());
    }

    #[test]
    fn set_multi_select_enables_flag() {
        let mut dlg = FileDialog::new_for_test(false);
        dlg.set_multi_select(true);
        assert!(dlg.is_multi_select());
    }

    #[test]
    fn set_multi_select_disables_flag() {
        let mut dlg = FileDialog::new_for_test(true);
        dlg.set_multi_select(false);
        assert!(!dlg.is_multi_select());
    }

    #[test]
    fn set_multi_select_returns_mut_ref_for_builder() {
        let mut dlg = FileDialog::new_for_test(false);
        let returned: &mut FileDialog = dlg.set_multi_select(true);
        returned.set_multi_select(false);
        assert!(!dlg.is_multi_select());
    }

    // -------- OFN constant values (sanity) --------

    #[cfg(target_os = "windows")]
    #[test]
    fn ofn_constant_values_match_win32_headers() {
        // Pinned from <commdlg.h> so a typoed hex digit gets caught.
        // `OPEN_FILENAME_FLAGS` is a type alias for `u32`, so direct `assert_eq!`
        // against a `u32` literal works without any cast.
        let must_exist: u32 = OFN_FILEMUSTEXIST;
        let path_must_exist: u32 = OFN_PATHMUSTEXIST;
        let no_change_dir: u32 = OFN_NOCHANGEDIR;
        let allow_multi: u32 = OFN_ALLOWMULTISELECT;
        let explorer: u32 = OFN_EXPLORER;
        assert_eq!(must_exist, 0x00001000);
        assert_eq!(path_must_exist, 0x00000800);
        assert_eq!(no_change_dir, 0x00000008);
        assert_eq!(allow_multi, 0x00000200);
        assert_eq!(explorer, 0x00080000);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ofn_flags_are_all_distinct() {
        // Guards against accidentally aliasing two flags.
        let flags: [u32; 5] = [
            OFN_FILEMUSTEXIST,
            OFN_PATHMUSTEXIST,
            OFN_NOCHANGEDIR,
            OFN_ALLOWMULTISELECT,
            OFN_EXPLORER,
        ];
        for i in 0..flags.len() {
            for j in (i + 1)..flags.len() {
                assert_ne!(
                    flags[i], flags[j],
                    "OFN flags at {i} and {j} collide: 0x{:x}",
                    flags[i]
                );
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ofn_combined_flags_dont_drop_bits() {
        // Spot-check that the OR we use in show_modal_multi doesn't
        // accidentally produce zero (i.e. no flag is a duplicate of 0).
        let combined = OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_ALLOWMULTISELECT
            | OFN_EXPLORER;
        assert_ne!(combined, 0, "OR of all OFN flags must not be zero");
        // Each individual flag should survive a bitwise AND with itself.
        for &flag in &[
            OFN_FILEMUSTEXIST,
            OFN_PATHMUSTEXIST,
            OFN_NOCHANGEDIR,
            OFN_ALLOWMULTISELECT,
            OFN_EXPLORER,
        ] {
            assert_eq!(flag & flag, flag);
        }
    }

    #[test]
    fn file_dialog_style_open_and_save_are_distinct() {
        assert_ne!(FileDialogStyle::Open, FileDialogStyle::Save);
    }

    #[test]
    fn file_dialog_style_is_debug_and_copy() {
        // Pin the derives so a regression in the enum changes a loud test.
        let s = FileDialogStyle::Open;
        let _copy = s; // requires Copy
        let _dbg = format!("{:?}", s); // requires Debug
    }
}
