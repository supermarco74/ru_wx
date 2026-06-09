//! Minitest: `Listbook` — a `ListBox`-driven notebook.
//!
//! Demonstrates:
//! - Creating a passive `Listbook` (no internal widget; the caller
//!   owns the `ListBox` that drives it).
//! - Adding pages (panels parented to the frame) with labels.
//! - Wiring the listbox's selection change to the book.
//! - Reacting to page changes via the book's callback.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_listbook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{App, BoxSizer, Button, Frame, ListBox, Listbook, Panel, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Listbook")
        .with_size(540, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click a page in the list.", 0);

    // The listbox is the "strip" that drives the book.
    let list = ListBox::new(&frame);
    for name in ["Alpha", "Beta", "Gamma", "Delta"] {
        list.append(name);
    }

    // The book is passive — it just stores pages and shows/hides them.
    // Wrap it in `Rc<_>` so the listbox's selection-change closure
    // can borrow it (`Listbook` doesn't implement `Clone` itself).
    let book: Rc<Listbook> = Rc::new(Listbook::new());

    // ── Page 1 — Alpha ──────────────────────────────────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Page 1 — alpha content");
    let btn1 = Button::new(&page1, "Alpha action");
    let s1 = status.clone();
    btn1.on_click(&frame, move || s1.set_status_text("Page1 → Alpha action", 0));
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add(btn1.as_widget_ref());
    page1.set_sizer(sz1);
    book.add_page("Alpha", page1);

    // ── Page 2 — Beta ───────────────────────────────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Page 2 — beta content");
    let btn2 = Button::new(&page2, "Beta action");
    let s2 = status.clone();
    btn2.on_click(&frame, move || s2.set_status_text("Page2 → Beta action", 0));
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(btn2.as_widget_ref());
    page2.set_sizer(sz2);
    book.add_page("Beta", page2);

    // ── Page 3 — Gamma ──────────────────────────────────────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "Page 3 — gamma content");
    let btn3 = Button::new(&page3, "Gamma action");
    let s3 = status.clone();
    btn3.on_click(&frame, move || s3.set_status_text("Page3 → Gamma action", 0));
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add(btn3.as_widget_ref());
    page3.set_sizer(sz3);
    book.add_page("Gamma", page3);

    // ── Page 4 — Delta ──────────────────────────────────────────────────
    let page4 = Panel::new(&frame);
    let lbl4 = StaticText::new(&page4, "Page 4 — delta content");
    let mut sz4 = BoxSizer::vertical();
    sz4.add(lbl4.as_widget_ref());
    page4.set_sizer(sz4);
    book.add_page("Delta", page4);

    // Wire listbox selection to book.
    let book_for_list = book.clone();
    let list_for_cb = list.clone();
    list.on_selection_change(&frame, move || {
        if let Some(idx) = list_for_cb.get_selection() {
            book_for_list.select(idx);
        }
    });
    // Initialize: select page 0 (the book auto-shows the first page,
    // but we make the wiring explicit).
    list.set_selection(0);

    // React to the book changing selection via its own callback.
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        s_sel.set_status_text(&format!("Listbook → page {idx}"), 0);
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(list.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
