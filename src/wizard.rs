//! Multi-page navigation dialog (`wxWizard`).
//!
//! A [`Wizard`] is a top-level window that hosts a sequence of
//! user-supplied [`Panel`] pages, plus four auto-managed navigation
//! buttons (Back, Next, Finish, Cancel) at the bottom of the client
//! area. It mirrors a subset of `wxWizard` / `wxWizardPage` from
//! wxWidgets and is the typical way to step the user through a
//! multi-screen input flow (e.g. install wizards, "set up your
//! account" flows, etc.).
//!
//! # Typical usage
//!
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let app = App::new();
//!
//! let mut wiz = Wizard::new("Setup Wizard", 500, 360);
//!
//! // Page 1: a panel that the wizard will host
//! let p1 = Panel::new(wiz.frame());
//! StaticText::new(&p1, "Welcome to the wizard!");
//! wiz.add_page("Welcome", p1);
//!
//! // Page 2
//! let p2 = Panel::new(wiz.frame());
//! StaticText::new(&p2, "Page two — fill in your details.");
//! wiz.add_page("Details", p2);
//!
//! // Page 3 (last)
//! let p3 = Panel::new(wiz.frame());
//! StaticText::new(&p3, "Click Finish to complete the setup.");
//! wiz.add_page("Finish", p3);
//!
//! match wiz.run() {
//!     WizardResult::Finished => println!("Wizard completed!"),
//!     WizardResult::Cancelled => println!("Wizard cancelled."),
//! }
//! ```
//!
//! # Page lifecycle
//!
//! The user constructs each page as a normal [`Panel`] parented to the
//! wizard's underlying [`Frame`] (obtainable via [`Wizard::frame`]) and
//! configures it (child widgets, sizer, background colour, …) *before*
//! handing it to [`Wizard::add_page`]. The wizard:
//!
//! * shows the first page on entry, hides every other page,
//! * shows / hides pages automatically as the user navigates with
//!   Back / Next / Finish,
//! * positions the active page in the area above the button row,
//! * re-positions the active page and the four buttons every time the
//!   wizard's frame is resized (via [`Frame::on_resize`]).
//!
//! # Cross-platform behaviour
//!
//! The constructor is reachable on every platform; on non-Windows
//! hosts it stores the requested geometry in the returned
//! [`Wizard`] but does not actually create a window. Methods that
//! would require a live `HWND` (e.g. [`Wizard::run`]) are
//! `#[cfg]`-gated to Windows.

use std::cell::RefCell;
use std::rc::Rc;

use crate::button::Button;
use crate::frame::Frame;
use crate::panel::Panel;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, IsDialogMessageW, MSG, ShowWindow, SW_SHOW,
    TranslateMessage,
};

// ─── Layout constants ───────────────────────────────────────────────────

/// Width of each of the four navigation buttons (logical pixels).
const WIZARD_BUTTON_WIDTH: i32 = 80;
/// Height of each of the four navigation buttons (logical pixels).
const WIZARD_BUTTON_HEIGHT: i32 = 28;
/// Horizontal gap between adjacent navigation buttons.
const WIZARD_BUTTON_GAP: i32 = 8;
/// Total vertical space reserved for the button row at the bottom of
/// the client area. The page panel fills the area above this.
const WIZARD_BUTTONS_AREA: i32 = 40;

// ─── Public types ───────────────────────────────────────────────────────

/// A single page hosted by the [`Wizard`].
///
/// A `WizardPage` pairs a `title` (used as the dialog title prefix —
/// see [`Wizard::set_title`]) with a `panel` (the page's content
/// area). The caller typically builds the panel with
/// `Panel::new(wizard.frame())`, populates it with the desired child
/// widgets, and then hands the result to [`Wizard::add_page`].
#[derive(Clone)]
pub struct WizardPage {
    /// Page title.
    pub title: String,
    /// The page's content panel. Must already be parented to the
    /// wizard's [`Frame`] (obtainable via [`Wizard::frame`]).
    pub panel: Panel,
}

/// Outcome of [`Wizard::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardResult {
    /// The user clicked Finish on the last page.
    Finished,
    /// The user clicked Cancel or closed the wizard window.
    Cancelled,
}

// ─── Internal state shared with the button / resize closures ──────────

/// Per-wizard mutable state held behind an `Rc<RefCell<>>` so that the
/// click and resize closures can reach it.
pub(crate) struct WizardData {
    /// A clone of the wizard's underlying [`Frame`]. Cheap (`Frame` is
    /// already an `Rc<RefCell<>>`), and lets the Finish / Cancel
    /// handlers call `Frame::close` to break the modal message loop
    /// without having to thread the frame through every helper.
    pub frame: Frame,
    /// All pages, in display order. The first page is shown
    /// initially; all others are hidden.
    pub pages: Vec<WizardPage>,
    /// Index of the currently-shown page in `pages`.
    pub current: usize,
    /// `true` while the local message loop inside [`Wizard::run`] is
    /// pumping. The four click handlers and the window-close handler
    /// set this to `false` to break out of the loop.
    pub running: bool,
    /// The result the wizard will return from [`Wizard::run`]. Set
    /// by Finish (`Finished`) or by Cancel / window-close
    /// (`Cancelled`).
    pub result: Option<WizardResult>,
    /// Auto-created "Back" button.
    pub button_back: Button,
    /// Auto-created "Next" button.
    pub button_next: Button,
    /// Auto-created "Finish" button. Hidden on every page except the
    /// last; on the last page `Next` is hidden and `Finish` is
    /// shown, mirroring the wxWidgets convention.
    pub button_finish: Button,
    /// Auto-created "Cancel" button. Always enabled.
    pub button_cancel: Button,
    /// User-supplied finish callback (fired by [`Wizard::run`] after
    /// the loop exits with [`WizardResult::Finished`]).
    pub on_finish: Option<Box<dyn FnMut()>>,
    /// User-supplied cancel callback (fired by [`Wizard::run`] after
    /// the loop exits with [`WizardResult::Cancelled`]).
    pub on_cancel: Option<Box<dyn FnMut()>>,
    /// User-supplied page-change callback (fired every time the
    /// user navigates to a different page).
    pub on_page_changed: Option<Box<dyn FnMut(usize)>>,
}

/// A `wxWizard`-style top-level dialog for multi-step user input.
///
/// The wizard owns a [`Frame`] internally (reachable via
/// [`Wizard::frame`]) and four navigation buttons auto-attached to
/// that frame. The user adds pages (panels) via [`Wizard::add_page`]
/// and starts the modal flow with [`Wizard::run`].
pub struct Wizard {
    data: Rc<RefCell<WizardData>>,
}

// ─── Constructor & public API ──────────────────────────────────────────

impl Wizard {
    /// Create a new wizard with the given title and initial client
    /// size (logical, 96-DPI pixels — the frame is DPI-scaled on
    /// Windows).
    ///
    /// The wizard is **not shown** until [`Wizard::run`] is called;
    /// until then it only registers the four buttons and the
    /// resize / click handlers on the underlying [`Frame`].
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        let frame = Frame::builder()
            .with_title(title)
            .with_size(width, height)
            .build();

        let data = Rc::new(RefCell::new(WizardData {
            frame: frame.clone(),
            pages: Vec::new(),
            current: 0,
            running: false,
            result: None,
            button_back: Button::new(&frame, "< Back"),
            button_next: Button::new(&frame, "Next >"),
            button_finish: Button::new(&frame, "Finish"),
            button_cancel: Button::new(&frame, "Cancel"),
            on_finish: None,
            on_cancel: None,
            on_page_changed: None,
        }));

        // Initial layout: position the four buttons along the bottom
        // of the requested client area. There are no pages yet, so
        // the page-positioning branch is a no-op; the `on_resize`
        // callback below will fire as soon as the frame is shown
        // (`WM_SIZE`) and position the current page correctly.
        layout_wizard(&data, width, height);

        // Re-layout on every frame resize.
        {
            let data_clone = data.clone();
            frame.on_resize(move |w, h| {
                layout_wizard(&data_clone, w, h);
            });
        }

        // The X button cancels the wizard (same as clicking Cancel).
        {
            let data_clone = data.clone();
            frame.on_close(move || {
                handle_cancel(&data_clone);
            });
        }

        // Register the four button click handlers. The closures
        // capture a clone of the `Rc<RefCell<WizardData>>`; the
        // underlying `Frame` is reachable through `data.frame`.
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_back.on_click(&frame, move || {
                handle_back(&data_clone);
            });
        }
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_next.on_click(&frame, move || {
                handle_next(&data_clone);
            });
        }
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_finish.on_click(&frame, move || {
                handle_finish(&data_clone);
            });
        }
        {
            let d = data.borrow();
            let data_clone = data.clone();
            d.button_cancel.on_click(&frame, move || {
                handle_cancel(&data_clone);
            });
        }

        Wizard { data }
    }

    /// Borrow the wizard's underlying [`Frame`]. Use this to create
    /// child controls (page panels, sizers) before adding them to
    /// the wizard.
    pub fn frame(&self) -> Frame {
        self.data.borrow().frame.clone()
    }

    /// Append a new page to the wizard.
    ///
    /// `panel` is taken by value and must already be parented to the
    /// wizard's [`Frame`] (typically by creating it with
    /// `Panel::new(wizard.frame())`). The first page added is shown
    /// initially; every subsequent page is hidden until the user
    /// navigates to it.
    ///
    /// Returns the page's index.
    pub fn add_page(&mut self, title: &str, panel: Panel) -> usize {
        let idx = self.data.borrow().pages.len();
        self.data.borrow_mut().pages.push(WizardPage {
            title: title.to_string(),
            panel,
        });
        if idx != 0 {
            // Hide non-current pages immediately so the user doesn't
            // see them stacked under the current one.
            if let Some(page) = self.data.borrow().pages.get(idx) {
                page.panel.hide();
            }
        }
        // Re-evaluate which buttons are enabled / visible.
        update_buttons(&self.data);
        idx
    }

    /// Register a callback fired by [`Wizard::run`] when the user
    /// clicks Finish (i.e. the wizard completes successfully).
    pub fn on_finish<F: FnMut() + 'static>(&mut self, f: F) {
        self.data.borrow_mut().on_finish = Some(Box::new(f));
    }

    /// Register a callback fired by [`Wizard::run`] when the user
    /// clicks Cancel or closes the wizard window.
    pub fn on_cancel<F: FnMut() + 'static>(&mut self, f: F) {
        self.data.borrow_mut().on_cancel = Some(Box::new(f));
    }

    /// Register a callback fired every time the user navigates to a
    /// different page. Receives the new page index.
    pub fn on_page_changed<F: FnMut(usize) + 'static>(&mut self, f: F) {
        self.data.borrow_mut().on_page_changed = Some(Box::new(f));
    }

    /// Show the wizard and run a local modal message loop. Returns
    /// when the user finishes or cancels the wizard.
    pub fn run(&mut self) -> WizardResult {
        // Mark the wizard as running and clear any leftover result
        // from a previous `run()` call.
        {
            let mut d = self.data.borrow_mut();
            d.running = true;
            d.result = None;
        }
        // Make sure the four buttons reflect the state of the first
        // page (Back disabled, Cancel/Next enabled, etc.).
        update_buttons(&self.data);

        #[cfg(target_os = "windows")]
        {
            let hwnd = self.data.borrow().frame.hwnd();
            // SAFETY: Win32 FFI call with validated arguments (HWND is the live
            // frame HWND owned by this `Wizard`).
            unsafe {
                ShowWindow(hwnd, SW_SHOW);
            }

            // Local modal message loop. Modelled on
            // `Dialog::show_modal`: pump messages with `GetMessageW`
            // (which returns -1 on error and 0 on `WM_QUIT`); for
            // every other message route it through
            // `IsDialogMessageW` (so Tab / accelerators work) and
            // fall through to the normal `TranslateMessage` /
            // `DispatchMessageW` path. The loop exits when either
            // the OS posts `WM_QUIT` or one of the click / close
            // handlers flips `data.running` to `false`.
            let hwnd = self.data.borrow().frame.hwnd();
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
                if ret <= 0 {
                    break;
                }
                if !self.data.borrow().running {
                    break;
                }
                // SAFETY: Win32 FFI call with validated arguments (`hwnd` is
                // a live window owned by this `Wizard`; `&msg` is a valid
                // `MSG` slot).
                unsafe {
                    if IsDialogMessageW(hwnd, &msg) == 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }

        // Take the result. Default to `Cancelled` if the message
        // loop exited without an explicit result (e.g. WM_QUIT from
        // an outer pump).
        let result = self
            .data
            .borrow()
            .result
            .unwrap_or(WizardResult::Cancelled);

        // Fire the appropriate user callback, if any. We `take()` the
        // callback out of the slot so the closure (which may capture
        // `&mut` references into user data) does not have to coexist
        // with the still-borrowed `data`.
        match result {
            WizardResult::Finished => {
                if let Some(mut cb) = self.data.borrow_mut().on_finish.take() {
                    cb();
                }
            }
            WizardResult::Cancelled => {
                if let Some(mut cb) = self.data.borrow_mut().on_cancel.take() {
                    cb();
                }
            }
        }

        result
    }
}

// ─── Internal helpers ───────────────────────────────────────────────────

/// Position the current page panel (filling the area above the button
/// row) and the four navigation buttons (along the bottom of the
/// client area).
///
/// Called once with the requested `with_size` geometry, then on
/// every frame resize via the `on_resize` callback.
fn layout_wizard(data: &Rc<RefCell<WizardData>>, w: u32, h: u32) {
    let d = data.borrow();

    // Page panel: from (0, 0) to (w, h - BUTTONS_AREA). The active
    // page is shown (it may have been added while the wizard was
    // already laid out at a different size, in which case its old
    // position / size is stale).
    if let Some(page) = d.pages.get(d.current) {
        let pref = page.panel.as_widget_ref();
        pref.borrow_mut().set_position(0, 0);
        pref.borrow_mut().set_size(w, h.saturating_sub(WIZARD_BUTTONS_AREA as u32).max(1));
        if !page.panel.is_visible() {
            page.panel.show();
        }
    }

    // Buttons: right-aligned row at the bottom of the client area.
    // The order from right to left is: Cancel, Finish, Next, Back.
    let button_y = h as i32 - WIZARD_BUTTON_HEIGHT - 6;
    let mut x = w as i32 - WIZARD_BUTTON_WIDTH - 8; // rightmost: Cancel
    d.button_cancel
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
    x -= WIZARD_BUTTON_WIDTH + WIZARD_BUTTON_GAP;
    d.button_finish
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
    x -= WIZARD_BUTTON_WIDTH + WIZARD_BUTTON_GAP;
    d.button_next
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
    x -= WIZARD_BUTTON_WIDTH + WIZARD_BUTTON_GAP;
    d.button_back
        .as_widget_ref()
        .borrow_mut()
        .set_position(x, button_y);
}

/// Update the enabled / visible state of the four navigation
/// buttons to match the current page index:
/// * `Back` is disabled on the first page, enabled otherwise.
/// * `Next` is visible + enabled on every page except the last.
/// * `Finish` is visible + enabled only on the last page.
/// * `Cancel` is always visible and enabled.
///
/// If no pages have been added yet, both `Next` and `Finish` are
/// disabled.
fn update_buttons(data: &Rc<RefCell<WizardData>>) {
    let d = data.borrow();
    let n = d.pages.len();
    let cur = d.current;
    let has_pages = n > 0;
    let is_last = has_pages && cur + 1 == n;
    let is_first = cur == 0;

    // Enable / disable
    d.button_back
        .as_widget_ref()
        .borrow_mut()
        .set_enabled(!is_first);
    d.button_next
        .as_widget_ref()
        .borrow_mut()
        .set_enabled(has_pages && !is_last);
    d.button_finish
        .as_widget_ref()
        .borrow_mut()
        .set_enabled(has_pages && is_last);
    // Cancel is always enabled (the underlying button defaults to
    // `enabled = true`).

    // Show / hide Next vs Finish depending on whether the current
    // page is the last one (mirroring the wxWidgets convention).
    d.button_next
        .as_widget_ref()
        .borrow_mut()
        .set_visible(!is_last);
    d.button_finish
        .as_widget_ref()
        .borrow_mut()
        .set_visible(is_last);
}

/// Switch the active page to `new_idx`. No-op if the index is out of
/// range. Hides every other page, shows the new active page, updates
/// the navigation buttons and fires the user-supplied
/// `on_page_changed` callback (if any).
fn show_page(data: &Rc<RefCell<WizardData>>, new_idx: usize) {
    {
        let d = data.borrow();
        if new_idx >= d.pages.len() {
            return;
        }
    }
    // Hide every non-active page first.
    {
        let d = data.borrow();
        for (i, p) in d.pages.iter().enumerate() {
            if i != new_idx && p.panel.is_visible() {
                p.panel.hide();
            }
        }
    }
    // Update the active index and show the new page.
    {
        let mut d = data.borrow_mut();
        d.current = new_idx;
        if let Some(page) = d.pages.get(new_idx) {
            if !page.panel.is_visible() {
                page.panel.show();
            }
        }
    }
    update_buttons(data);

    // Fire the user callback (if any) *after* the data borrow is
    // released, so the callback can freely re-enter `&self` methods
    // on the wizard.
    if let Some(mut cb) = data.borrow_mut().on_page_changed.take() {
        cb(new_idx);
        data.borrow_mut().on_page_changed = Some(cb);
    }
}

// ─── Button / close handlers ───────────────────────────────────────────

fn handle_back(data: &Rc<RefCell<WizardData>>) {
    let cur = data.borrow().current;
    if cur > 0 {
        show_page(data, cur - 1);
    }
}

fn handle_next(data: &Rc<RefCell<WizardData>>) {
    let cur = data.borrow().current;
    let n = data.borrow().pages.len();
    if cur + 1 < n {
        show_page(data, cur + 1);
    }
}

fn handle_finish(data: &Rc<RefCell<WizardData>>) {
    {
        let mut d = data.borrow_mut();
        if !d.running {
            return;
        }
        d.running = false;
        d.result = Some(WizardResult::Finished);
    }
    // Close the frame so any messages still queued for the wizard
    // window do not linger; the `run()` loop will exit on the next
    // iteration because `running` is now `false`.
    data.borrow().frame.close();
}

fn handle_cancel(data: &Rc<RefCell<WizardData>>) {
    {
        let mut d = data.borrow_mut();
        if !d.running {
            return;
        }
        d.running = false;
        d.result = Some(WizardResult::Cancelled);
    }
    data.borrow().frame.close();
}
