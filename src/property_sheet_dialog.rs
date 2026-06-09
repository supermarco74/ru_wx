//! Multi-tab settings dialog (`wxPropertySheetDialog`).
//!
//! A [`PropertySheetDialog`] is a top-level window that hosts a
//! [`Tab`] notebook of user-supplied [`Panel`] pages plus three
//! auto-managed buttons (OK, Cancel, Apply) along the bottom of the
//! client area. It mirrors a subset of `wxPropertySheetDialog` from
//! wxWidgets and is the typical way to present a category-style
//! "settings / preferences" dialog where the user steps through
//! related option pages.
//!
//! # Typical usage
//!
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let app = App::new();
//! let parent = Frame::builder().with_title("Main").with_size(400, 300).build();
//!
//! let mut dlg = PropertySheetDialog::new(&parent, "Settings", 500, 400);
//!
//! // Page 1: General
//! let p1 = Panel::new(&dlg.frame());
//! StaticText::new(&p1, "General settings go here.");
//! dlg.add_page("General", p1);
//!
//! // Page 2: Advanced
//! let p2 = Panel::new(&dlg.frame());
//! StaticText::new(&p2, "Advanced settings go here.");
//! dlg.add_page("Advanced", p2);
//!
//! dlg.on_apply(|| {
//!     println!("Apply clicked (no page change committed yet).");
//! });
//!
//! match dlg.show_modal() {
//!     PropertySheetDialogResult::Ok => println!("OK"),
//!     PropertySheetDialogResult::Cancelled => println!("Cancelled"),
//! }
//! ```
//!
//! # Modal flow
//!
//! [`PropertySheetDialog::show_modal`] disables the parent window,
//! shows the dialog, and runs a local message loop. The loop exits
//! when:
//!
//! * the user clicks **OK** — [`PropertySheetDialogResult::Ok`]
//!   is returned (the parent is re-enabled first),
//! * the user clicks **Cancel** or closes the window —
//!   [`PropertySheetDialogResult::Cancelled`] is returned,
//! * clicking **Apply** does **not** close the dialog; it just
//!   fires the user's `on_apply` callback (if any). This mirrors
//!   the wxWidgets convention where Apply is a transient
//!   action that validates / commits the current page's values
//!   without dismissing the dialog.
//!
//! # Cross-platform behaviour
//!
//! The constructor and `add_page` are reachable on every platform;
//! on non-Windows hosts the dialog stores the requested geometry
//! but does not actually create windows. `show_modal` is a no-op
//! on non-Windows and returns `Cancelled`.
//!
//! [`PropertySheetDialogResult::Ok`]: enum.PropertySheetDialogResult.html#variant.Ok
//! [`PropertySheetDialogResult::Cancelled`]: enum.PropertySheetDialogResult.html#variant.Cancelled

use std::cell::RefCell;
use std::rc::Rc;

use crate::button::Button;
use crate::frame::Frame;
use crate::panel::Panel;
use crate::tab::Tab;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, IsDialogMessageW, MSG, TranslateMessage,
};

// ─── Layout constants ───────────────────────────────────────────────────

/// Width of each of the three navigation buttons (logical pixels).
const PSD_BUTTON_WIDTH: i32 = 80;
/// Height of each of the three navigation buttons (logical pixels).
const PSD_BUTTON_HEIGHT: i32 = 28;
/// Horizontal gap between adjacent buttons.
const PSD_BUTTON_GAP: i32 = 8;
/// Total vertical space reserved for the button row at the bottom of
/// the client area. The notebook fills the area above this.
const PSD_BUTTONS_AREA: i32 = 40;

// ─── Public types ───────────────────────────────────────────────────────

/// Outcome of [`PropertySheetDialog::show_modal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertySheetDialogResult {
    /// The user clicked OK.
    Ok,
    /// The user clicked Cancel or closed the dialog window.
    Cancelled,
}

// ─── Internal state ─────────────────────────────────────────────────────

/// Per-dialog mutable state held behind an `Rc<RefCell<>>` so that the
/// click and resize closures can reach it.
pub(crate) struct PropertySheetDialogData {
    /// The dialog's underlying [`Frame`]. A clone is also kept so
    /// Finish / Cancel / Apply handlers can call `Frame::close` to
    /// break the modal message loop without having to thread the
    /// frame through every helper.
    pub frame: Frame,
    /// The dialog's notebook (a [`Tab`]).
    pub tab: Tab,
    /// Auto-created "OK" button. Closes the dialog with
    /// [`PropertySheetDialogResult::Ok`].
    pub button_ok: Button,
    /// Auto-created "Cancel" button. Closes the dialog with
    /// [`PropertySheetDialogResult::Cancelled`].
    pub button_cancel: Button,
    /// Auto-created "Apply" button. Does **not** close the dialog;
    /// it just fires the user-supplied `on_apply` callback (if any).
    pub button_apply: Button,
    /// Parent window HWND; disabled while the dialog is modal and
    /// re-enabled on close.
    #[cfg(target_os = "windows")]
    pub parent_hwnd: windows_sys::Win32::Foundation::HWND,
    /// `true` while the local message loop inside
    /// [`PropertySheetDialog::show_modal`] is pumping.
    pub modal_running: bool,
    /// The result that will be returned from `show_modal`. Set by OK
    /// (`Ok`) or by Cancel / window-close (`Cancelled`).
    pub result: Option<PropertySheetDialogResult>,
    /// User-supplied Apply callback (fired every time the user
    /// clicks the Apply button). Apply does **not** close the
    /// dialog, so the callback is fired inline during the modal
    /// loop.
    pub on_apply: Option<Box<dyn FnMut()>>,
    /// Cached client-area size. Updated on every `on_resize` so the
    /// `layout_dialog` helper can position the notebook and the
    /// three buttons correctly.
    pub client_w: u32,
    pub client_h: u32,
}

/// A `wxPropertySheetDialog`-style top-level dialog hosting a tabbed
/// notebook of settings pages plus OK / Cancel / Apply buttons.
///
/// The dialog owns a [`Frame`] internally (reachable via
/// [`PropertySheetDialog::frame`]), a [`Tab`] notebook
/// ([`PropertySheetDialog::tab`]) and three navigation buttons.
/// Pages are added via [`PropertySheetDialog::add_page`]; the
/// modal flow is driven by [`PropertySheetDialog::show_modal`].
pub struct PropertySheetDialog {
    data: Rc<RefCell<PropertySheetDialogData>>,
}

// ─── Constructor & public API ──────────────────────────────────────────

impl PropertySheetDialog {
    /// Create a new property-sheet dialog.
    ///
    /// `parent` is the owning window; it is disabled while the
    /// dialog is modal and re-enabled when `show_modal` returns.
    /// The dialog itself is shown as a top-level window (it does
    /// not appear in the taskbar as a child of `parent`).
    pub fn new(parent: &Frame, title: &str, width: u32, height: u32) -> Self {
        let frame = Frame::builder()
            .with_title(title)
            .with_size(width, height)
            .build();

        // The notebook is parented to the dialog's frame so it can
        // host normal `Panel` page bodies via `Tab::add_page`.
        let tab = Tab::new(&frame);

        let data = Rc::new(RefCell::new(PropertySheetDialogData {
            frame: frame.clone(),
            tab: tab.clone(),
            button_ok: Button::new(&frame, "OK"),
            button_cancel: Button::new(&frame, "Cancel"),
            button_apply: Button::new(&frame, "Apply"),
            #[cfg(target_os = "windows")]
            parent_hwnd: parent.hwnd(),
            modal_running: false,
            result: None,
            on_apply: None,
            client_w: width,
            client_h: height,
        }));

        // Initial layout: position the notebook and the three
        // buttons. The `on_resize` callback registered below will
        // re-fire as soon as the frame is shown (`WM_SIZE`) and
        // adjust everything to the actual client area.
        layout_dialog(&data);

        // Re-layout on every frame resize.
        {
            let data_clone = data.clone();
            frame.on_resize(move |w, h| {
                let mut d = data_clone.borrow_mut();
                d.client_w = w;
                d.client_h = h;
                drop(d);
                layout_dialog(&data_clone);
            });
        }

        // The X button cancels the dialog (same as clicking Cancel).
        {
            let data_clone = data.clone();
            frame.on_close(move || {
                handle_cancel(&data_clone);
            });
        }

        // Register the three button click handlers. The closures
        // capture a clone of the `Rc<RefCell<...>>`; the underlying
        // `Frame` is reachable through `data.frame`.
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_ok.on_click(&frame, move || {
                handle_ok(&data_clone);
            });
        }
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_cancel.on_click(&frame, move || {
                handle_cancel(&data_clone);
            });
        }
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_apply.on_click(&frame, move || {
                handle_apply(&data_clone);
            });
        }

        PropertySheetDialog { data }
    }

    /// Borrow the dialog's underlying [`Frame`]. Use this to create
    /// page panels (parented to the frame) before handing them to
    /// [`PropertySheetDialog::add_page`].
    pub fn frame(&self) -> Frame {
        self.data.borrow().frame.clone()
    }

    /// Borrow the dialog's underlying [`Tab`] notebook. The Tab is
    /// also accessible indirectly: every page added through
    /// [`PropertySheetDialog::add_page`] is forwarded to the
    /// notebook.
    pub fn tab(&self) -> Tab {
        self.data.borrow().tab.clone()
    }

    /// Append a new page to the dialog's notebook.
    ///
    /// `panel` is taken by value and must already be parented to the
    /// dialog's [`Frame`] (typically by creating it with
    /// `Panel::new(dlg.frame())`). The first page added is shown
    /// initially; every subsequent page is hidden until the user
    /// clicks its tab.
    ///
    /// Returns the page's zero-based index (matching the underlying
    /// [`Tab::add_page`]).
    pub fn add_page(&mut self, title: &str, panel: Panel) -> i32 {
        let tab = self.data.borrow().tab.clone();
        tab.add_page(title, &panel)
    }

    /// Register a callback fired every time the user clicks Apply.
    /// Apply does **not** close the dialog; it just runs this
    /// callback (if any) and lets the user keep editing.
    pub fn on_apply<F: FnMut() + 'static>(&mut self, f: F) {
        self.data.borrow_mut().on_apply = Some(Box::new(f));
    }

    /// Show the dialog modally.
    ///
    /// Disables the parent window, shows the dialog, and runs a
    /// local message loop until the user clicks OK, Cancel, or
    /// closes the window. Apply does **not** exit the loop.
    pub fn show_modal(&mut self) -> PropertySheetDialogResult {
        // Reset the modal state.
        {
            let mut d = self.data.borrow_mut();
            d.modal_running = true;
            d.result = None;
        }

        #[cfg(target_os = "windows")]
        {
            // Disable the parent window so the user can't interact
            // with it while the dialog is up.
            let (hwnd, parent_hwnd) = {
                let d = self.data.borrow();
                (d.frame.hwnd(), d.parent_hwnd)
            };
            // SAFETY: `parent_hwnd` is a live window owned by the
            // caller. `EnableWindow` is a no-op if the handle is
            // already disabled / re-enabled, so the existing
            // wizard / dialog code is not affected by repeated
            // calls.
            unsafe {
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    parent_hwnd,
                    0,
                );
            }

            // Local modal message loop, modelled on
            // `Dialog::show_modal` and `Wizard::run`: pump with
            // `GetMessageW`, route through `IsDialogMessageW`
            // (so Tab / accelerators work), and fall through to
            // the normal `TranslateMessage` / `DispatchMessageW`
            // path. The loop exits on `WM_QUIT` (return 0),
            // error (return -1), or when one of the click /
            // close handlers flips `data.modal_running` to
            // `false`.
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
                if ret <= 0 {
                    break;
                }
                if !self.data.borrow().modal_running {
                    break;
                }
                // SAFETY: `hwnd` is a live dialog HWND owned by
                // this `PropertySheetDialog`; `&msg` is a valid
                // `MSG` slot.
                unsafe {
                    if IsDialogMessageW(hwnd, &msg) == 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }

            // Re-enable the parent window.
            let parent_hwnd = self.data.borrow().parent_hwnd;
            // SAFETY: see `EnableWindow(parent_hwnd, 0)` above.
            unsafe {
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    parent_hwnd,
                    1,
                );
            }
            // Bring the parent back to the foreground so the user
            // can keep working with it.
            // SAFETY: `parent_hwnd` is a live top-level window.
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(parent_hwnd);
            }
        }

        // Return the recorded result. Default to `Cancelled` if
        // the message loop exited without an explicit result
        // (e.g. an outer `WM_QUIT`).
        self.data
            .borrow()
            .result
            .unwrap_or(PropertySheetDialogResult::Cancelled)
    }
}

// ─── Internal helpers ───────────────────────────────────────────────────

/// Position the notebook (filling the area above the button row) and
/// the three navigation buttons (along the bottom-right of the
/// client area). Called once with the requested `with_size`
/// geometry, then on every frame resize via the `on_resize`
/// callback.
fn layout_dialog(data: &Rc<RefCell<PropertySheetDialogData>>) {
    let d = data.borrow();
    let w = d.client_w;
    let h = d.client_h;

    // Notebook: from (0, 0) to (w, h - BUTTONS_AREA).
    let tab = d.tab.as_widget_ref();
    let page_h = h.saturating_sub(PSD_BUTTONS_AREA as u32).max(1);
    tab.borrow_mut().set_position(0, 0);
    tab.borrow_mut().set_size(w, page_h);

    // Buttons: right-aligned row at the bottom of the client area.
    // The order from right to left is: OK, Cancel, Apply.
    let button_y = h as i32 - PSD_BUTTON_HEIGHT - 6;
    let mut x = w as i32 - PSD_BUTTON_WIDTH - 8; // rightmost: OK
    d.button_ok
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
    x -= PSD_BUTTON_WIDTH + PSD_BUTTON_GAP;
    d.button_cancel
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
    x -= PSD_BUTTON_WIDTH + PSD_BUTTON_GAP;
    d.button_apply
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
}

// ─── Button / close handlers ───────────────────────────────────────────

fn handle_ok(data: &Rc<RefCell<PropertySheetDialogData>>) {
    {
        let mut d = data.borrow_mut();
        if !d.modal_running {
            return;
        }
        d.modal_running = false;
        d.result = Some(PropertySheetDialogResult::Ok);
    }
    // Close the frame so any messages still queued for the
    // dialog window do not linger; the `show_modal` loop will
    // exit on the next iteration because `modal_running` is
    // now `false`.
    data.borrow().frame.close();
}

fn handle_cancel(data: &Rc<RefCell<PropertySheetDialogData>>) {
    {
        let mut d = data.borrow_mut();
        if !d.modal_running {
            return;
        }
        d.modal_running = false;
        d.result = Some(PropertySheetDialogResult::Cancelled);
    }
    data.borrow().frame.close();
}

fn handle_apply(data: &Rc<RefCell<PropertySheetDialogData>>) {
    // Apply does **not** close the dialog — it just fires the
    // user callback (if any). Take the callback out, run it,
    // and put it back so the closure can re-enter `&self` /
    // `&mut self` methods on the dialog without conflicting
    // with the `data` borrow.
    if let Some(mut cb) = data.borrow_mut().on_apply.take() {
        cb();
        data.borrow_mut().on_apply = Some(cb);
    }
}
