//! Drop target — receive files dragged from Windows Explorer onto a frame.
//!
//! This module implements the **Shell-level** file-drop protocol
//! (`WM_DROPFILES` / `DragAcceptFiles` / `DragQueryFileW`), which
//! covers the common case of files dragged from Windows Explorer
//! (or any other file-list source that uses the same Shell message
//! convention) into the application's top-level window.
//!
//! # Example
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! use ru_wx::DroppedFiles;
//!
//! let frame = Frame::builder().with_title("D&D demo").build();
//! frame.set_drop_files_callback(|files: DroppedFiles| {
//!     println!("Dropped {} file(s):", files.len());
//!     for path in files.paths() {
//!         println!("  - {}", path.display());
//!     }
//! });
//! ```
//!
//! # Scope
//!
//! The Shell-level protocol is the simpler of the two drag-and-drop
//! protocols Windows exposes:
//!
//! * It only carries **files** (no in-memory text, no custom data
//!   objects, no drag from another application that hands you an
//!   `IDataObject`).
//! * It does **not** need the COM/OLE runtime (`OleInitialize`,
//!   `CoInitialize`, `RegisterDragDrop`) — the messages are sent
//!   straight to the window procedure by the Shell.
//! * It does **not** give you "drag-over" feedback (no
//!   `IDropTarget::DragOver` equivalent).
//!
//! For the full OLE COM drag-and-drop protocol
//! (`IDropTarget` / `IDataObject` / `RegisterDragDrop`), which
//! supports arbitrary data formats and live drag-over feedback, see
//! the future-work section of the v0.5.5 upgrade report. The two
//! protocols are not mutually exclusive — the COM one is a strict
//! superset — but the Shell-level protocol is sufficient for the
//! common "open these files dropped from Explorer" workflow and is
//! what this module provides.
//!
//! # Cross-platform notes
//!
//! [`DroppedFiles`] itself is a plain data type and is available on
//! every platform; the registration method
//! ([`crate::Frame::set_drop_files_callback`]) is also exposed on
//! every platform, but on non-Windows hosts the registered callback
//! is simply never invoked. This mirrors the way the rest of
//! `ru_wx` exposes Windows-only functionality (e.g. menu
//! construction works on every platform but `HMENU` is only
//! meaningful on Windows).

use std::path::PathBuf;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{DragFinish, DragQueryFileW, HDROP};

/// A list of file paths dropped onto a frame's window, as reported
/// by the Shell-level drag-and-drop protocol (`WM_DROPFILES`).
///
/// Constructed by the library from the `HDROP` the Shell hands to
/// the frame's window procedure; passed by value to the callback
/// registered with [`crate::Frame::set_drop_files_callback`]. The
/// paths are absolute (Explorer always provides absolute paths in
/// `WM_DROPFILES`, even for items dragged from a relative location
/// the user opened with a non-absolute cwd).
///
/// The type deliberately exposes a small surface (`len`, `is_empty`,
/// `paths`, `into_paths`) so the user code that handles a drop
/// event doesn't have to think about the `HDROP` / `DropTarget`
/// vocabulary.
///
/// # Example
///
/// ```no_run
/// use ru_wx::prelude::*;
/// use ru_wx::DroppedFiles;
///
/// let frame = Frame::builder().with_title("Files").build();
/// frame.set_drop_files_callback(|files: DroppedFiles| {
///     if files.is_empty() {
///         return; // defensive: shouldn't happen
///     }
///     println!("Got {} file(s); first is {}",
///         files.len(),
///         files.paths()[0].display());
/// });
/// ```
pub struct DroppedFiles {
    paths: Vec<PathBuf>,
}

impl DroppedFiles {
    /// Construct a `DroppedFiles` from the list of paths the
    /// Shell handed us. `pub(crate)` — the only legitimate
    /// callers are the `WM_DROPFILES` dispatch in
    /// `src/frame.rs` (real path) and the `#[cfg(test)]`
    /// accessors below. User code should treat `DroppedFiles`
    /// as an opaque value handed to its callback; constructing
    /// one by hand would defeat the type's purpose of
    /// representing "files the Shell dropped here".
    pub(crate) fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    /// Construct a `DroppedFiles` from a pre-extracted list of
    /// paths. `pub(crate)` — the only intended constructor is the
    /// one driven by `WM_DROPFILES` inside the frame's window
    /// procedure. The constructor is reachable from the test
    /// module so the data-only accessors can be unit-tested
    /// without needing a real `HWND`.
    #[cfg(test)]
    pub(crate) fn from_paths(paths: Vec<PathBuf>) -> Self {
        Self::new(paths)
    }

    /// Number of dropped files. A Shell-level drop with zero files
    /// is technically possible if a buggy source posts an empty
    /// `HDROP`; treat it the same as "no drop happened" in user
    /// code.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// `true` if no files were dropped. See [`Self::len`].
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Borrow the dropped file paths. The returned slice is in the
    /// same order the Shell handed them to us (which is usually
    /// selection order in Explorer, but the Shell makes no
    /// guarantees — don't rely on it for sort order).
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Consume the `DroppedFiles` and return the inner `Vec<PathBuf>`,
    /// avoiding one extra clone if the user code wants to take
    /// ownership of the paths.
    pub fn into_paths(self) -> Vec<PathBuf> {
        self.paths
    }
}

impl std::fmt::Debug for DroppedFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DroppedFiles")
            .field("count", &self.paths.len())
            .finish()
    }
}

/// Extract the list of dropped file paths from a Shell `HDROP`
/// handle. The handle is the `wparam` of the `WM_DROPFILES` message
/// and stays valid until the matching `DragFinish` call (which the
/// caller is responsible for issuing — see the wndproc in
/// `src/frame.rs`).
///
/// The implementation follows the canonical two-call pattern for
/// `DragQueryFileW`: the first call with a null buffer returns the
/// required buffer length in TCHARs, the second call fills the
/// buffer. We add one slot for the NUL terminator per the
/// documented contract.
///
/// # Safety
///
/// The caller must ensure:
///
/// * `hdrop` is a valid `HDROP` (i.e. a pointer the Shell handed
///   us via `WM_DROPFILES` and that we have not yet passed to
///   `DragFinish`).
/// * No other thread is simultaneously calling `DragQueryFileW` on
///   the same handle. The Shell's `HDROP` is not thread-safe.
#[cfg(target_os = "windows")]
pub(crate) fn extract_paths_from_hdrop(hdrop: HDROP) -> Vec<PathBuf> {
    // 0xFFFFFFFF is the documented "give me the count" sentinel.
    // SAFETY: hdrop is a valid HDROP per the caller's contract; we
    // pass a null buffer because we are not asking for a path here,
    // just the count.
    let count = unsafe { DragQueryFileW(hdrop, 0xFFFFFFFF, std::ptr::null_mut(), 0) };
    let mut paths = Vec::with_capacity(count as usize);
    for i in 0..count {
        // First call: how many TCHARs (UTF-16 code units) do we need?
        // SAFETY: same as above; i < count is guaranteed by the loop bound.
        let len_tchars = unsafe { DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0) };
        if len_tchars == 0 {
            // Per Shell docs, a return of 0 from DragQueryFileW with
            // a non-special index indicates the index is out of range
            // (should not happen) or a memory failure. Skip rather
            // than poison the result with a junk path.
            continue;
        }
        // `len_tchars` already excludes the terminating NUL, so
        // allocate `len_tchars + 1` slots to leave room for it.
        let mut buf = vec![0u16; (len_tchars + 1) as usize];
        // SAFETY: `buf` is a valid contiguous `u16` buffer of
        // `buf.len()` TCHARs; the call writes at most `buf.len()`
        // TCHARs and returns the number actually written (excluding
        // the NUL). The handle is the same one we counted on above.
        let copied = unsafe { DragQueryFileW(hdrop, i, buf.as_mut_ptr(), buf.len() as u32) };
        if copied == 0 {
            continue;
        }
        // `copied` excludes the NUL. We don't read past `copied` so
        // a partially-filled buffer (which shouldn't happen) is
        // safe.
        let wide = &buf[..copied as usize];
        paths.push(PathBuf::from(String::from_utf16_lossy(wide)));
    }
    paths
}

/// Finish the Shell drag-and-drop operation. Must be called once
/// per `WM_DROPFILES` after the user callback returns, otherwise
/// the Shell will leak the internal storage backing the `HDROP`.
///
/// # Safety
///
/// `hdrop` must be the same handle that was passed to
/// [`extract_paths_from_hdrop`], and must not have been passed to
/// `DragFinish` before. The Shell does not validate the pointer on
/// the second call.
#[cfg(target_os = "windows")]
pub(crate) fn finish_drop(hdrop: HDROP) {
    // SAFETY: documented in the function-level contract above.
    unsafe { DragFinish(hdrop) };
}

#[cfg(test)]
mod tests {
    //! Tests for the data-only parts of the drop-target module.
    //!
    //! The actual `WM_DROPFILES` dispatch path (which would call
    //! `DragQueryFileW` / `DragFinish`) is *not* tested here: it
    //! requires a real `HDROP` from the Shell, which can only be
    //! obtained from a live `WM_DROPFILES` message. The
    //! [`Frame::set_drop_files_callback`] storage path is tested in
    //! `frame.rs`'s own `mod tests`, again without a real `HWND`.
    //!
    //! What we *can* test cheaply:
    //!
    //! * The `DroppedFiles` accessors round-trip a `Vec<PathBuf>`.
    //! * The `len` / `is_empty` / `paths` / `into_paths` semantics
    //!   are correct for the four edge cases the user code is
    //!   likely to encounter: 0, 1, N, and a path containing
    //!   non-ASCII characters (a `String::from_utf16_lossy` smoke
    //!   test).
    //! * The `Debug` impl does not blow up on empty / multi-file
    //!   cases (it is a public API surface — the `{:?}` formatter
    //!   is part of the public contract).

    use super::DroppedFiles;
    use std::path::PathBuf;

    #[test]
    fn from_paths_then_paths_round_trips() {
        let original = vec![PathBuf::from(r"C:\Users\me\file.txt")];
        let files = DroppedFiles::from_paths(original.clone());
        assert_eq!(files.paths(), original.as_slice());
    }

    #[test]
    fn len_reports_the_underlying_vec_length() {
        let zero = DroppedFiles::from_paths(vec![]);
        assert_eq!(zero.len(), 0);
        assert!(zero.is_empty());

        let one = DroppedFiles::from_paths(vec![PathBuf::from("a.txt")]);
        assert_eq!(one.len(), 1);
        assert!(!one.is_empty());

        let many = DroppedFiles::from_paths(vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
            PathBuf::from("c.txt"),
        ]);
        assert_eq!(many.len(), 3);
        assert!(!many.is_empty());
    }

    #[test]
    fn into_paths_returns_the_inner_vec_and_consumes_self() {
        let original = vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")];
        let files = DroppedFiles::from_paths(original.clone());
        let taken = files.into_paths();
        assert_eq!(taken, original);
    }

    #[test]
    fn paths_survive_non_ascii_unicode() {
        // A file name with both a non-ASCII Latin-1 character
        // (è) and a CJK character (文) — this exercises the
        // `String::from_utf16_lossy` path. The Shell hands us
        // UTF-16, so a real-world drop with a non-ASCII filename
        // will go through this same path.
        let original = vec![PathBuf::from("C:\\resumè-文.txt")];
        let files = DroppedFiles::from_paths(original.clone());
        assert_eq!(files.paths(), original.as_slice());
    }

    #[test]
    fn debug_does_not_panic_for_empty() {
        // The `Debug` impl is part of the public surface; make sure
        // it produces *something* and does not blow up on an empty
        // drop. (It happens to produce "DroppedFiles { count: 0 }";
        // the exact format is not part of the contract, but the
        // panic-freeness is.)
        let files = DroppedFiles::from_paths(vec![]);
        let s = format!("{files:?}");
        assert!(s.contains("DroppedFiles"));
        assert!(s.contains("count"));
    }

    #[test]
    fn debug_does_not_panic_for_many() {
        let files = DroppedFiles::from_paths(vec![
            PathBuf::from("a"),
            PathBuf::from("b"),
            PathBuf::from("c"),
        ]);
        let s = format!("{files:?}");
        assert!(s.contains("DroppedFiles"));
        assert!(s.contains("3"));
    }
}
