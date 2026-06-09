//! Minitest: `Choicebook` — a `Choice` drop-down driven notebook.
//!
//! Demonstrates:
//! - Creating a passive `Choicebook` (the caller owns the `Choice`
//!   that drives it).
//! - Adding pages (panels parented to the frame) with labels.
//! - Wiring the choice's selection change to the book.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_choicebook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{App, BoxSizer, Button, Choice, Choicebook, Frame, Panel, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Choicebook")
        .with_size(540, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Pick a page in the drop-down.", 0);

    // The choice is the "strip" that drives the book.
    let choice = Choice::new(&frame);
    for name in ["Red", "Green", "Blue", "Yellow"] {
        choice.append(name);
    }

    // The book is passive — it just stores pages and shows/hides them.
    let book: Rc<Choicebook> = Rc::new(Choicebook::new());

    // ── Page 1 — Red ────────────────────────────────────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Page 1 — Red");
    let btn1 = Button::new(&page1, "Red action");
    let s1 = status.clone();
    btn1.on_click(&frame, move || s1.set_status_text("Page1 → Red action", 0));
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add(btn1.as_widget_ref());
    page1.set_sizer(sz1);
    book.add_page("Red", page1);

    // ── Page 2 — Green ──────────────────────────────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Page 2 — Green");
    let btn2 = Button::new(&page2, "Green action");
    let s2 = status.clone();
    btn2.on_click(&frame, move || s2.set_status_text("Page2 → Green action", 0));
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(btn2.as_widget_ref());
    page2.set_sizer(sz2);
    book.add_page("Green", page2);

    // ── Page 3 — Blue ───────────────────────────────────────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "Page 3 — Blue");
    let btn3 = Button::new(&page3, "Blue action");
    let s3 = status.clone();
    btn3.on_click(&frame, move || s3.set_status_text("Page3 → Blue action", 0));
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add(btn3.as_widget_ref());
    page3.set_sizer(sz3);
    book.add_page("Blue", page3);

    // ── Page 4 — Yellow ─────────────────────────────────────────────────
    let page4 = Panel::new(&frame);
    let lbl4 = StaticText::new(&page4, "Page 4 — Yellow");
    let mut sz4 = BoxSizer::vertical();
    sz4.add(lbl4.as_widget_ref());
    page4.set_sizer(sz4);
    book.add_page("Yellow", page4);

    // Wire choice selection to book.
    let book_for_choice = book.clone();
    let choice_for_cb = choice.clone();
    choice.on_selection_change(&frame, move || {
        if let Some(idx) = choice_for_cb.get_selection() {
            book_for_choice.select(idx);
        }
    });

    // Initialize the choice to the first page (mirrors the pattern
    // in `mt_listbook` so the test is deterministic — without this
    // the initial selection is `None` and no page is shown).
    choice.set_selection(0);

    // React to the book changing selection via its own callback.
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        s_sel.set_status_text(&format!("Choicebook → page {idx}"), 0);
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(choice.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
