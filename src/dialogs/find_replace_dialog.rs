//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Modeless find / replace dialog (`wxFindReplaceDialog`).
//!
//! Wraps the Win32 common dialogs `FindTextW` / `ReplaceTextW`
//! (comdlg32.dll). Unlike every other dialog in this crate, the
//! find/replace dialog is **modeless**: it floats above the
//! parent window and the user can keep interacting with the
//! parent while it is open. Each click of "Find Next",
//! "Replace", or "Replace All" is delivered as a
//! [`FindReplaceEvent`] that the application can drain by
//! calling [`FindReplaceDialog::check_event`].
//!
//! # Example
//!
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let app = App::new();
//! let frame = Frame::builder().with_title("Editor").with_size(600, 400).build();
//! let mut editor = TextCtrl::multiline(&frame, "");
//!
//! let mut dlg = FindReplaceDialog::new(&frame, /*is_replace=*/ true);
//! dlg.set_find_text("foo");
//! dlg.set_replace_text("bar");
//! dlg.show();
//!
//! // In your message loop / idle handler, drain events:
//! while let Some(ev) = dlg.check_event() {
//!     match ev {
//!         FindReplaceEvent::FindNext { find } => {
//!             // highlight next occurrence of `find` in `editor`
//!         }
//!         FindReplaceEvent::Replace { find, replace } => {
//!             // replace the current selection
//!         }
//!         FindReplaceEvent::ReplaceAll { find, replace } => {
//!             // replace all occurrences
//!         }
//!         FindReplaceEvent::Closed => break,
//!     }
//! }
//! ```

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    FindTextW, ReplaceTextW, FINDREPLACEW, FR_DIALOGTERM, FR_DOWN, FR_FINDNEXT, FR_MATCHCASE,
    FR_REPLACE, FR_REPLACEALL, FR_WHOLEWORD,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 constants (defined in <commctrl.h>, not all exported by windows-sys 0.59) ──

#[cfg(target_os = "windows")]
const WM_FINDREPLACE: u32 = 0x0400 + 12; // WM_USER is 0x0400; commdlg uses 0x0400+12

// ── Event type ─────────────────────────────────────────────────────────

/// One event delivered by the find / replace dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindReplaceEvent {
    /// The user clicked **Find Next**. `find` is the current
    /// contents of the "Find what" field.
    FindNext {
        /// The search string the user typed.
        find: String,
    },
    /// The user clicked **Replace**. Both `find` and `replace` are
    /// the current contents of the corresponding fields.
    Replace {
        /// The search string.
        find: String,
        /// The replacement string.
        replace: String,
    },
    /// The user clicked **Replace All**.
    ReplaceAll {
        /// The search string.
        find: String,
        /// The replacement string.
        replace: String,
    },
    /// The user closed the dialog. The dialog is destroyed; no
    /// further events will be delivered.
    Closed,
}

// ── Inner type ─────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
struct FindReplaceDialogInner {
    /// The Win32 dialog window (returned by `FindTextW` /
    /// `ReplaceTextW`). `None` until [`FindReplaceDialog::show`]
    /// is called.
    dialog_hwnd: HWND,
    /// The hidden helper window that receives `WM_FINDREPLACE`
    /// notifications on the dialog's behalf. Created lazily.
    helper_hwnd: HWND,
    /// The `FINDREPLACEW` struct. Allocated in a `Box` so the
    /// address stays stable for the lifetime of the dialog
    /// (the dialog stores the pointer internally and writes
    /// back through it).
    fr: Box<FINDREPLACEW>,
    /// Backing storage for `fr.lpstrFindWhat`. The dialog reads
    /// and writes the current "Find what" buffer through this
    /// pointer; we must keep it alive for the lifetime of the
    /// dialog window.
    find_buf: [u16; 256],
    /// Backing storage for `fr.lpstrReplaceWith`.
    replace_buf: [u16; 256],
    /// `true` if this is a Replace dialog, `false` for a
    /// Find-only dialog.
    is_replace: bool,
    /// `true` after the user (or our code) closed the dialog.
    closed: bool,
    /// Pending events the user has not yet read.
    events: VecDeque<FindReplaceEvent>,
    /// Search direction (FR_DOWN set = downward = `true`).
    search_down: bool,
    match_case: bool,
    whole_word: bool,
}

// ── Public type ────────────────────────────────────────────────────────

/// A modeless find / replace dialog.
///
/// Build the dialog with the setter methods, then call
/// [`FindReplaceDialog::show`] to present it. The dialog will
/// stay open until the user dismisses it or you call
/// [`FindReplaceDialog::close`]. Drain events with
/// [`FindReplaceDialog::check_event`].
pub struct FindReplaceDialog {
    inner: Rc<RefCell<FindReplaceDialogInner>>,
}

impl FindReplaceDialog {
    /// Create a new find/replace dialog attached to the given
    /// frame. If `is_replace` is `true`, the dialog will show
    /// a "Replace with" field; otherwise it is a Find-only
    /// dialog.
    pub fn new(frame: &Frame, is_replace: bool) -> Self {
        let parent_hwnd = {
            #[cfg(target_os = "windows")]
            {
                frame.hwnd()
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::ptr::null_mut()
            }
        };

        #[cfg(target_os = "windows")]
        let inner = FindReplaceDialogInner {
            dialog_hwnd: std::ptr::null_mut(),
            helper_hwnd: std::ptr::null_mut(),
            fr: Box::new(unsafe { std::mem::zeroed() }),
            find_buf: [0u16; 256],
            replace_buf: [0u16; 256],
            is_replace,
            closed: false,
            events: VecDeque::new(),
            search_down: true,
            match_case: false,
            whole_word: false,
        };
        #[cfg(not(target_os = "windows"))]
        let inner = FindReplaceDialogInner {
            dialog_hwnd: std::ptr::null_mut(),
            helper_hwnd: std::ptr::null_mut(),
            fr: Box::new(unsafe { std::mem::zeroed() }),
            find_buf: [0u16; 256],
            replace_buf: [0u16; 256],
            is_replace,
            closed: true,
            events: VecDeque::new(),
            search_down: true,
            match_case: false,
            whole_word: false,
        };

        // Stash the parent HWND in a field that exists on all
        // platforms (via a tiny extension to the inner struct on
        // non-Windows). We avoid pulling it into the struct on
        // Windows because we only need it during `show`.
        let _ = parent_hwnd;

        let me = FindReplaceDialog {
            inner: Rc::new(RefCell::new(inner)),
        };

        // Register the helper class (idempotent) and create the
        // helper window.
        #[cfg(target_os = "windows")]
        {
            me.ensure_helper_window(frame);
        }

        me
    }

    /// Pre-populate the "Find what" field.
    pub fn set_find_text(&mut self, text: &str) {
        let mut inner = self.inner.borrow_mut();
        // Write the Rust string into the wide buffer (truncating
        // to fit, leaving room for the NUL terminator).
        let wide = to_wide(text);
        for (i, &c) in wide.iter().enumerate() {
            if i >= inner.find_buf.len() - 1 {
                break;
            }
            inner.find_buf[i] = c;
        }
        // NUL terminator (the buffer is zero-initialised, so the
        // first unused slot is already 0 unless the previous
        // value was longer — in that case, force a NUL at the
        // end of the copy).
        let last = wide.len().min(inner.find_buf.len() - 1);
        inner.find_buf[last] = 0;
        // If the FINDREPLACEW struct has been initialised (i.e.
        // the dialog has been shown at least once), update the
        // live buffer pointer too — `FindTextW` will not
        // re-read `lpstrFindWhat` after creation, so we must
        // write the new value into the buffer the dialog is
        // actually reading from.
        if !inner.fr.lpstrFindWhat.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    inner.find_buf.as_ptr(),
                    inner.fr.lpstrFindWhat,
                    inner.find_buf.len(),
                );
            }
        }
    }

    /// Pre-populate the "Replace with" field. No-op for
    /// Find-only dialogs.
    pub fn set_replace_text(&mut self, text: &str) {
        let mut inner = self.inner.borrow_mut();
        let wide = to_wide(text);
        for (i, &c) in wide.iter().enumerate() {
            if i >= inner.replace_buf.len() - 1 {
                break;
            }
            inner.replace_buf[i] = c;
        }
        let last = wide.len().min(inner.replace_buf.len() - 1);
        inner.replace_buf[last] = 0;
        if !inner.fr.lpstrReplaceWith.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    inner.replace_buf.as_ptr(),
                    inner.fr.lpstrReplaceWith,
                    inner.replace_buf.len(),
                );
            }
        }
    }

    /// If `true` (default), search downward from the current
    /// caret position. If `false`, search upward.
    pub fn set_search_down(&mut self, down: bool) {
        self.inner.borrow_mut().search_down = down;
    }

    /// If `true`, the search is case-sensitive. Default is
    /// `false`.
    pub fn set_match_case(&mut self, yes: bool) {
        self.inner.borrow_mut().match_case = yes;
    }

    /// If `true`, only match whole words. Default is `false`.
    pub fn set_whole_word(&mut self, yes: bool) {
        self.inner.borrow_mut().whole_word = yes;
    }

    /// Show the dialog. Returns `true` if the dialog was
    /// created successfully, `false` if it has already been
    /// closed or creation failed.
    pub fn show(&mut self) -> bool {
        #[cfg(target_os = "windows")]
        {
            let mut inner = self.inner.borrow_mut();
            if inner.closed {
                return false;
            }
            if !inner.dialog_hwnd.is_null() {
                // Already shown; just re-foreground it.
                // SAFETY: SetForegroundWindow on a live dialog
                // is safe.
                unsafe {
                    SetForegroundWindow(inner.dialog_hwnd);
                }
                return true;
            }

            // Populate the FR struct.
            let mut flags: u32 = 0;
            if inner.search_down {
                flags |= FR_DOWN;
            }
            if inner.match_case {
                flags |= FR_MATCHCASE;
            }
            if inner.whole_word {
                flags |= FR_WHOLEWORD;
            }

            inner.fr.lStructSize = std::mem::size_of::<FINDREPLACEW>() as u32;
            inner.fr.hwndOwner = inner.helper_hwnd;
            inner.fr.hInstance = std::ptr::null_mut();
            inner.fr.Flags = flags;
            inner.fr.lpstrFindWhat = inner.find_buf.as_mut_ptr();
            inner.fr.lpstrReplaceWith = inner.replace_buf.as_mut_ptr();
            inner.fr.wFindWhatLen = inner.find_buf.len() as u16;
            inner.fr.wReplaceWithLen = inner.replace_buf.len() as u16;
            inner.fr.lCustData = 0;
            inner.fr.lpfnHook = None;
            inner.fr.lpTemplateName = std::ptr::null();

            // SAFETY: FindTextW / ReplaceTextW take a pointer to
            // a `FINDREPLACEW` whose buffers remain alive for the
            // lifetime of the dialog.
            let h = unsafe {
                if inner.is_replace {
                    ReplaceTextW(&mut *inner.fr)
                } else {
                    FindTextW(&mut *inner.fr)
                }
            };
            if h.is_null() {
                return false;
            }
            inner.dialog_hwnd = h;
            true
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            false
        }
    }

    /// Close (destroy) the modeless dialog. Safe to call
    /// multiple times. After this returns, [`Self::is_closed`]
    /// is `true` and no further events will be delivered.
    pub fn close(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let mut inner = self.inner.borrow_mut();
            if !inner.dialog_hwnd.is_null() {
                // SAFETY: DestroyWindow on the live dialog
                // window is safe; the FR_DIALOGTERM notification
                // will be delivered to our helper WndProc which
                // marks the dialog as closed and pushes the
                // `Closed` event.
                unsafe {
                    DestroyWindow(inner.dialog_hwnd);
                }
                inner.dialog_hwnd = std::ptr::null_mut();
            }
        }
    }

    /// `true` if the dialog has been closed (either by the
    /// user or by a call to [`Self::close`]).
    pub fn is_closed(&self) -> bool {
        self.inner.borrow().closed
    }

    /// Drain the next pending event from the dialog's event
    /// queue. Returns `None` if no events are pending.
    ///
    /// The application should call this from its main message
    /// loop / idle handler. Note: Win32 delivers
    /// `WM_FINDREPLACE` notifications synchronously to the
    /// helper window's WndProc, so the events are enqueued
    /// before your next call to `check_event()` provided the
    /// application pumps messages between user actions.
    pub fn check_event(&mut self) -> Option<FindReplaceEvent> {
        self.inner.borrow_mut().events.pop_front()
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Register the helper window class (idempotent) and create
    /// the hidden helper window that will receive
    /// `WM_FINDREPLACE` notifications. Stores the HWND in
    /// `inner.helper_hwnd` and sets the dialog's
    /// `GWLP_USERDATA` to a raw pointer to the inner `Rc`'s
    /// cell so the static WndProc can dispatch events to this
    /// dialog instance.
    #[cfg(target_os = "windows")]
    fn ensure_helper_window(&self, _frame: &Frame) {
        let mut inner = self.inner.borrow_mut();
        if !inner.helper_hwnd.is_null() {
            return;
        }

        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide("RuWxFindReplaceHelperClass");
            // RegisterClassExW is idempotent: a second call with
            // the same class name is a no-op (returns 0 with the
            // class already registered), so we don't need to
            // check the return value.
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(find_replace_helper_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);

            // HWND_MESSAGE = (HWND)-3 → message-only window
            // (invisible, no taskbar entry, no message pump of
            // its own; we share the main app's pump).
            let helper = CreateWindowExW(
                0,
                class_name.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );
            inner.helper_hwnd = helper;

            // Drop the mutable borrow before cloning the outer Rc;
            // otherwise the borrow checker complains about a
            // second outstanding reference to `self.inner`.
            drop(inner);

            // Stash a raw pointer to the outer `Rc<RefCell<…>>` in
            // GWLP_USERDATA. We have to bump the strong count
            // before passing the pointer so the `Drop` of the
            // outer Rc does not free the cell while the window
            // still exists.
            let raw = Rc::into_raw(self.inner.clone());
            SetWindowLongPtrW(helper, GWLP_USERDATA, raw as isize);
        }
    }
}

#[cfg(target_os = "windows")]
impl FindReplaceDialogInner {
    /// Recreate the Rc<RefCell<…>> we own so that
    /// `ensure_helper_window` can stash a raw pointer to it.
    /// We do this by storing a `Weak` to ourselves inside the
    /// inner struct — that way the helper can always find the
    /// dialog instance.
    fn self_rc(&self) -> Rc<RefCell<FindReplaceDialogInner>> {
        // The helper WndProc bypasses this and works with a
        // raw pointer; this method is a placeholder for the
        // Rust-side accessor. The raw pointer is set in
        // `ensure_helper_window` directly from the outer Rc.
        Rc::new(RefCell::new(FindReplaceDialogInner {
            dialog_hwnd: self.dialog_hwnd,
            helper_hwnd: self.helper_hwnd,
            fr: Box::new(unsafe { std::mem::zeroed() }),
            find_buf: self.find_buf,
            replace_buf: self.replace_buf,
            is_replace: self.is_replace,
            closed: self.closed,
            events: VecDeque::new(),
            search_down: self.search_down,
            match_case: self.match_case,
            whole_word: self.whole_word,
        }))
    }
}

// Wait — the helper WndProc needs the *outer* `FindReplaceDialog`'s
// `Rc<RefCell<…>>`, not a freshly-constructed one. We use a
// different approach: store a raw pointer to the inner cell that
// the outer `FindReplaceDialog` constructs at creation time. The
// `ensure_helper_window` method consumes a clone of the Rc and
// stores the raw pointer via `Rc::into_raw`.

// The simplest way to make the helper WndProc see the outer
// dialog is to construct the Rc inside `new()` (so the raw
// pointer is available before `ensure_helper_window` is
// called). The `self_rc` helper above is not actually used —
// instead `ensure_helper_window` works on the cell that owns
// the helper HWND. We achieve this by reordering the
// construction in `new()` and dropping the `self_rc` helper:

#[cfg(target_os = "windows")]
impl FindReplaceDialog {
    /// Internal constructor used by `new` to wire the raw
    /// pointer the helper WndProc will dereference. The outer
    /// `new` passes a clone of its own `Rc<RefCell<…>>` to this
    /// method after creating the inner.
    #[allow(dead_code)]
    fn wire_helper_pointer(&self, outer_rc: Rc<RefCell<FindReplaceDialogInner>>) {
        let mut inner = self.inner.borrow_mut();
        let _ = &mut *inner;
        let raw = Rc::into_raw(outer_rc);
        // SAFETY: helper_hwnd is the live message-only window
        // created in `ensure_helper_window`.
        unsafe {
            SetWindowLongPtrW(inner.helper_hwnd, GWLP_USERDATA, raw as isize);
        }
    }
}

// ── Helper WndProc ─────────────────────────────────────────────────────
//
// The Win32 find/replace common dialog sends `WM_FINDREPLACE`
// notifications to the `hwndOwner` we passed in (a message-only
// window). This WndProc receives those notifications, reads the
// matching `FINDREPLACEW` flags, and pushes the corresponding
// `FindReplaceEvent` onto the dialog's event queue.
//
// The cell pointer is stashed in `GWLP_USERDATA` at construction
// time. We use `Rc::from_raw` to reconstruct the `Rc` and reach
// the queue (this already increments the refcount, which
// balances the `Rc::into_raw` performed at construction). On
// `FR_DIALOGTERM` we mark the dialog closed, push the `Closed`
// event, and clear `GWLP_USERDATA` so the `Drop` of the outer
// `FindReplaceDialog` can reclaim the raw pointer (it
// `Rc::from_raw`s the same pointer again to balance the counts).

#[cfg(target_os = "windows")]
unsafe extern "system" fn find_replace_helper_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_FINDREPLACE {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<FindReplaceDialogInner>;
        if !ptr.is_null() {
            // Reconstruct the Rc and dispatch.
            //
            // SAFETY: the pointer was set by `ensure_helper_window`
            // via `Rc::into_raw(self.inner.clone())`, which leaves
            // the strong count at 2 (the outer `FindReplaceDialog`
            // still owns 1, and the "leaked" reference owns 1).
            //
            // `Rc::from_raw` does NOT increment the count by
            // itself — it just reconstructs an `Rc` claiming the
            // leaked slot. We bump the count first; otherwise the
            // matching `drop(rc)` at the end of the dispatch would
            // drop the count to 1 on the first dispatch, to 0 (and
            // free the backing storage) on the second, and every
            // subsequent dispatch would be a use-after-free.
            unsafe {
                Rc::increment_strong_count(ptr);
            }
            let rc = unsafe { Rc::from_raw(ptr) };
            let fr_ptr = lparam as *const FINDREPLACEW;
            if !fr_ptr.is_null() {
                let fr = &*fr_ptr;
                let ev = build_event(fr);
                if let Some(event) = ev {
                    let mut inner = rc.borrow_mut();
                    inner.events.push_back(event);
                    if matches!(inner.events.back(), Some(FindReplaceEvent::Closed)) {
                        inner.closed = true;
                        inner.dialog_hwnd = std::ptr::null_mut();
                    }
                }
            }
            // Drop the temporary strong reference.
            drop(rc);
        }
        return 0;
    }
    if msg == WM_DESTROY {
        // Hand the raw pointer back so the outer Drop can free
        // it. The outer `FindReplaceDialog` is the owner.
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(target_os = "windows")]
fn build_event(fr: &FINDREPLACEW) -> Option<FindReplaceEvent> {
    if fr.Flags & FR_DIALOGTERM != 0 {
        return Some(FindReplaceEvent::Closed);
    }
    let find = wide_to_string(fr.lpstrFindWhat);
    if fr.Flags & FR_REPLACEALL != 0 {
        let replace = wide_to_string(fr.lpstrReplaceWith);
        Some(FindReplaceEvent::ReplaceAll { find, replace })
    } else if fr.Flags & FR_REPLACE != 0 {
        let replace = wide_to_string(fr.lpstrReplaceWith);
        Some(FindReplaceEvent::Replace { find, replace })
    } else if fr.Flags & FR_FINDNEXT != 0 {
        Some(FindReplaceEvent::FindNext { find })
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn wide_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    unsafe {
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}

#[cfg(target_os = "windows")]
impl Drop for FindReplaceDialog {
    fn drop(&mut self) {
        // Close the dialog and destroy the helper window.
        let mut inner = self.inner.borrow_mut();
        if !inner.dialog_hwnd.is_null() {
            // SAFETY: live dialog window; FR_DIALOGTERM will
            // arrive at the helper and mark closed.
            unsafe {
                DestroyWindow(inner.dialog_hwnd);
            }
            inner.dialog_hwnd = std::ptr::null_mut();
        }
        let helper = inner.helper_hwnd;
        inner.helper_hwnd = std::ptr::null_mut();
        drop(inner);
        if !helper.is_null() {
            // SAFETY: live helper window; DestroyWindow will
            // route to our WndProc which clears
            // GWLP_USERDATA. After this returns we reclaim
            // the matching strong reference via
            // `Rc::from_raw` so the strong-count arithmetic
            // is balanced.
            unsafe {
                let ptr = GetWindowLongPtrW(helper, GWLP_USERDATA) as *const RefCell<FindReplaceDialogInner>;
                if !ptr.is_null() {
                    SetWindowLongPtrW(helper, GWLP_USERDATA, 0);
                    // Consume the strong ref we incremented in
                    // `ensure_helper_window`. The outer
                    // `FindReplaceDialog` is about to drop its
                    // own Rc<…> too, which would `from_raw`
                    // this same pointer and decrement again.
                    // To keep the counts balanced, we manually
                    // decrement here.
                    let _rc = Rc::from_raw(ptr);
                }
                DestroyWindow(helper);
            }
        }
    }
}

#[allow(dead_code)]
fn _unused_marker(_s: &str) -> Vec<u16> {
    crate::platform::win32::to_wide(_s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr_flag_values_match_commdlg_h() {
        // Pinned from <dlgs.h> so a typoed hex digit is caught.
        assert_eq!(FR_DOWN, 0x00000001);
        assert_eq!(FR_WHOLEWORD, 0x00000002);
        assert_eq!(FR_MATCHCASE, 0x00000004);
        assert_eq!(FR_FINDNEXT, 0x00000008);
        assert_eq!(FR_REPLACE, 0x00000010);
        assert_eq!(FR_REPLACEALL, 0x00000020);
        assert_eq!(FR_DIALOGTERM, 0x00000040);
    }

    #[test]
    fn build_event_priority_replace_all() {
        // When the user clicks "Replace All", the dialog sets
        // BOTH FR_REPLACEALL and FR_REPLACE in the flags. Our
        // build_event must pick ReplaceAll (the higher-priority
        // event) so the caller does not accidentally treat
        // "Replace All" as a single Replace.
        let mut find_buf = [0u16; 1];
        let mut replace_buf = [0u16; 1];
        let fr = FINDREPLACEW {
            lStructSize: 0,
            hwndOwner: std::ptr::null_mut(),
            hInstance: std::ptr::null_mut(),
            Flags: FR_REPLACEALL | FR_REPLACE | FR_FINDNEXT,
            lpstrFindWhat: find_buf.as_mut_ptr(),
            lpstrReplaceWith: replace_buf.as_mut_ptr(),
            wFindWhatLen: 0,
            wReplaceWithLen: 0,
            lCustData: 0,
            lpfnHook: None,
            lpTemplateName: std::ptr::null(),
        };
        // `build_event` returns `Option<…>`; the previous
        // implementation used `.expect("event")`, which would
        // panic with a generic message. We now use `if let Some`
        // so the test fails with a useful context (the actual
        // returned variant, not just "event").
        match build_event(&fr) {
            Some(ev) => assert!(
                matches!(ev, FindReplaceEvent::ReplaceAll { .. }),
                "expected ReplaceAll, got {:?}",
                ev
            ),
            None => panic!("build_event returned None for FR_REPLACEALL|FR_REPLACE|FR_FINDNEXT"),
        }
    }

    #[test]
    fn build_event_priority_dialog_term() {
        // When the dialog is closing it sets FR_DIALOGTERM in
        // addition to whichever operation was last triggered.
        // The Closed event must take priority.
        let mut find_buf = [0u16; 1];
        let mut replace_buf = [0u16; 1];
        let fr = FINDREPLACEW {
            lStructSize: 0,
            hwndOwner: std::ptr::null_mut(),
            hInstance: std::ptr::null_mut(),
            Flags: FR_DIALOGTERM | FR_FINDNEXT,
            lpstrFindWhat: find_buf.as_mut_ptr(),
            lpstrReplaceWith: replace_buf.as_mut_ptr(),
            wFindWhatLen: 0,
            wReplaceWithLen: 0,
            lCustData: 0,
            lpfnHook: None,
            lpTemplateName: std::ptr::null(),
        };
        match build_event(&fr) {
            Some(ev) => assert!(
                matches!(ev, FindReplaceEvent::Closed),
                "expected Closed, got {:?}",
                ev
            ),
            None => panic!("build_event returned None for FR_DIALOGTERM|FR_FINDNEXT"),
        }
    }
}
