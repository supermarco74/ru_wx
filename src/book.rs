//! Alternative notebook controls: [`Listbook`], [`Choicebook`],
//! [`Treebook`], [`Toolbook`].
//!
//! All four are siblings of [`crate::tab::Tab`] — they are
//! "notebooks" of pages, each backed by a [`Panel`], but instead of
//! a `SysTabControl32` tab strip they use a different widget to
//! show the page labels:
//!
//! * [`Listbook`] — pages are entries in a [`ListBox`] (vertical
//!   strip on the left, content area on the right).
//! * [`Choicebook`] — pages are entries in a [`Choice`] drop-down
//!   (combobox on top, content area below).
//! * [`Treebook`] — pages are nodes in a [`TreeCtrl`] (hierarchical
//!   outline on the left, content area on the right).
//! * [`Toolbook`] — pages are buttons in a [`ToolBar`] (icon strip
//!   on the top, content area below).
//!
//! # Design
//!
//! The book is a *passive* container: it owns the page list, the
//! currently-selected page, and the visibility state of each page
//! panel, but it does **not** wire the control-strip widget's
//! selection events to the book. The caller does that explicitly:
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! use ru_wx::book::Listbook;
//! let frame = Frame::builder().with_title("book").build();
//! let _list = ListBox::new(&frame);
//! let book = Listbook::new();
//! book.add_page("Page 1", Panel::new(&frame));
//! book.add_page("Page 2", Panel::new(&frame));
//! // Caller wires the control-strip's selection change to `book.select(idx)`.
//! ```
//!
//! The reason for this is that the various `ru_wx` controls
//! (ListBox, Choice, TreeCtrl, ToolBar) each expose their own
//! selection-change API in their own way; the book does not try
//! to abstract over them. The book is "the list of pages and the
//! show/hide logic" — that's all. The user is in charge of the
//! rest.
//!
//! # Cross-platform behaviour
//!
//! On non-Windows the constructors are no-op stubs that store the
//! page list; the `current_selection` accessor always returns the
//! currently-selected index.

use std::cell::RefCell;
use std::rc::Rc;

use crate::panel::Panel;

// ─── BookCore (the shared state machine) ───────────────────────────────

/// Page bookkeeping shared by all book variants.
struct BookCore {
    /// Each entry is `(label, panel)`. Panels are stored in the
    /// order they are added. The book shows only the panel at
    /// `selected` (when `Some`) and hides the rest.
    pages: Vec<(String, Panel)>,
    /// Currently selected page (0-based). `None` means "no page is
    /// selected" (e.g. a freshly-built book with no pages).
    selected: Option<usize>,
    /// User callback for selection change.
    on_selection_change: Option<Box<dyn FnMut(usize)>>,
}

impl BookCore {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            selected: None,
            on_selection_change: None,
        }
    }

    fn add_page(&mut self, label: &str, panel: Panel) {
        let index = self.pages.len();
        self.pages.push((label.to_string(), panel));
        if self.selected.is_none() {
            self.select(index);
        }
    }

    fn select(&mut self, index: usize) {
        if index >= self.pages.len() {
            return;
        }
        if Some(index) == self.selected {
            return;
        }
        // Hide the previously-selected panel, show the new one.
        if let Some(prev) = self.selected {
            if let Some((_, p)) = self.pages.get(prev) {
                p.hide();
            }
        }
        self.selected = Some(index);
        if let Some((_, p)) = self.pages.get(index) {
            p.show();
        }
        if let Some(cb) = self.on_selection_change.as_mut() {
            cb(index);
        }
    }

    fn current_selection(&self) -> Option<usize> {
        self.selected
    }

    fn set_on_selection_change<F: FnMut(usize) + 'static>(&mut self, f: F) {
        self.on_selection_change = Some(Box::new(f));
    }

    fn page_count(&self) -> usize {
        self.pages.len()
    }
}

// ─── Listbook ──────────────────────────────────────────────────────────

/// A passive notebook backed by a [`ListBox`]. The user is
/// responsible for wiring the listbox's selection event to
/// [`Listbook::select`].
///
/// Sibling of `wxListbook` from wxWidgets.
pub struct Listbook {
    core: Rc<RefCell<BookCore>>,
}

impl Listbook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }
}

impl Default for Listbook {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Choicebook ────────────────────────────────────────────────────────

/// A passive notebook backed by a [`Choice`] drop-down. Caller
/// wires the choice's selection change to [`Choicebook::select`].
///
/// Sibling of `wxChoicebook` from wxWidgets.
pub struct Choicebook {
    core: Rc<RefCell<BookCore>>,
}

impl Choicebook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }
}

impl Default for Choicebook {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Treebook ──────────────────────────────────────────────────────────

/// A passive notebook backed by a [`TreeCtrl`]. The tree's node ids
/// are exposed via [`Treebook::item_for_page`] so the caller can
/// wire a `TreeCtrl::on_selection_change` closure to
/// [`Treebook::select`].
///
/// Sibling of `wxTreebook` from wxWidgets.
pub struct Treebook {
    core: Rc<RefCell<BookCore>>,
}

impl Treebook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }
}

impl Default for Treebook {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Toolbook ──────────────────────────────────────────────────────────

/// A passive notebook backed by a [`ToolBar`]. Caller wires the
/// tool-button's click to [`Toolbook::select`].
///
/// Sibling of `wxToolbook` from wxWidgets.
pub struct Toolbook {
    core: Rc<RefCell<BookCore>>,
}

impl Toolbook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }
}

impl Default for Toolbook {
    fn default() -> Self {
        Self::new()
    }
}
