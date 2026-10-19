//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Simplebook` — a page container with no tab bar: only the
//! current page is visible, driven by Prev / Next buttons.
//!
//! Demonstrates:
//! - The shell (`Simplebook::new`) built *before* the pages, then sized
//!   to the first page's area by `add_page`
//! - Pages with real, interactive content (Gauge, ListBox, CheckBox, TextCtrl)
//! - `set_selection` / `selection` / `show_page` navigation
//! - Prev / Next buttons cycling through the pages
//!
//! Run with:
//! ```bash
//! cargo run --example mt_simplebook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Frame, Gauge, ListBox, Panel, Simplebook, StaticText,
    StatusBar, TextCtrl,
};

// Where the page panels live.
const PAGE_X: i32 = 20;
const PAGE_Y: i32 = 10;
const PAGE_W: u32 = 710;
const PAGE_H: u32 = 330;

/// Position a freshly-built page in the content area.
fn place_page(page: &Panel) {
    page.set_position(PAGE_X, PAGE_Y);
    page.set_size(PAGE_W, PAGE_H);
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Simplebook")
        .with_size(760, 440)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Use Prev/Next to flip the pages.", 0);

    // The shell must exist before the pages: `add_page` sizes it to the
    // first page's area so the two overlap exactly.
    let book: Rc<Simplebook> = Rc::new(Simplebook::new(&frame));

    // ── Page 1 — Dashboard (Gauge + increment buttons) ──────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Dashboard — overall progress:");
    let gauge = Gauge::new(&page1, 100);
    gauge.set_value(35);
    let btn_plus = Button::new(&page1, "+10");
    let btn_minus = Button::new(&page1, "-10");
    let g_plus = gauge.clone();
    let s_plus = status.clone();
    btn_plus.on_click(&frame, move || {
        let v = g_plus.increment(10);
        s_plus.set_status_text(&format!("Progress: {v}%"), 0);
    });
    let g_minus = gauge.clone();
    let s_minus = status.clone();
    btn_minus.on_click(&frame, move || {
        let v = (g_minus.get_value() - 10).max(0);
        g_minus.set_value(v);
        s_minus.set_status_text(&format!("Progress: {v}%"), 0);
    });
    let mut row1 = BoxSizer::horizontal();
    row1.add(btn_plus.as_widget_ref());
    row1.add(btn_minus.as_widget_ref());
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add(gauge.as_widget_ref());
    sz1.add_sizer(row1);
    sz1.add_stretch(1);
    page1.set_sizer(sz1);
    place_page(&page1);
    book.add_page("Dashboard", page1.as_widget_ref());

    // ── Page 2 — Tasks (ListBox + add row) ───────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Tasks — type and add:");
    let tasks = ListBox::new(&page2);
    for t in ["Fix the sizer bug", "Add simplebook test", "Release"] {
        tasks.append(t);
    }
    let input = TextCtrl::new(&page2, "");
    let btn_add = Button::new(&page2, "Add task");
    let tasks_c = tasks.clone();
    let input_c = input.clone();
    let s_add = status.clone();
    btn_add.on_click(&frame, move || {
        let text = input_c.get_value();
        if !text.trim().is_empty() {
            tasks_c.append(text.trim());
            input_c.set_value("");
            s_add.set_status_text(&format!("{} task(s)", tasks_c.get_count()), 0);
        }
    });
    let mut row2 = BoxSizer::horizontal();
    row2.add_with_proportion(input.as_widget_ref(), 1);
    row2.add(btn_add.as_widget_ref());
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add_with_proportion(tasks.as_widget_ref(), 1);
    sz2.add_sizer(row2);
    page2.set_sizer(sz2);
    place_page(&page2);
    book.add_page("Tasks", page2.as_widget_ref());

    // ── Page 3 — Options (CheckBoxes) ────────────────────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "Options:");
    let chk1 = CheckBox::new(&page3, "Show line numbers");
    let chk2 = CheckBox::new(&page3, "Word wrap");
    chk1.set_checked(true);
    let c1 = chk1.clone();
    let s_c1 = status.clone();
    chk1.on_toggle(&frame, move || {
        s_c1.set_status_text(
            &format!(
                "Line numbers: {}",
                if c1.is_checked() { "ON" } else { "OFF" }
            ),
            0,
        );
    });
    let c2 = chk2.clone();
    let s_c2 = status.clone();
    chk2.on_toggle(&frame, move || {
        s_c2.set_status_text(
            &format!("Word wrap: {}", if c2.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add(chk1.as_widget_ref());
    sz3.add(chk2.as_widget_ref());
    sz3.add_stretch(1);
    page3.set_sizer(sz3);
    place_page(&page3);
    book.add_page("Options", page3.as_widget_ref());

    // ── Page 4 — Notes (multiline TextCtrl) ──────────────────────────
    let page4 = Panel::new(&frame);
    let lbl4 = StaticText::new(&page4, "Notes:");
    let notes = TextCtrl::multiline(&page4, "Free-form notes go here.\r\n");
    let btn_clear = Button::new(&page4, "Clear notes");
    let notes_c = notes.clone();
    btn_clear.on_click(&frame, move || notes_c.clear());
    let mut sz4 = BoxSizer::vertical();
    sz4.add(lbl4.as_widget_ref());
    sz4.add_with_proportion(notes.as_widget_ref(), 1);
    sz4.add(btn_clear.as_widget_ref());
    page4.set_sizer(sz4);
    place_page(&page4);
    book.add_page("Notes", page4.as_widget_ref());

    status.set_status_text(&format!("{} pages", book.page_count()), 1);

    // ── Prev / Next buttons flip the pages ──────────────────────────
    let btn_prev = Button::new(&frame, "◀ Prev");
    let btn_next = Button::new(&frame, "Next ▶");
    let book_prev = book.clone();
    let s_prev = status.clone();
    btn_prev.on_click(&frame, move || {
        let count = book_prev.page_count();
        if count == 0 {
            return;
        }
        let cur = book_prev.selection();
        let prev = (cur + count - 1) % count;
        book_prev.set_selection(prev);
        s_prev.set_status_text(&format!("Simplebook → page {prev}"), 0);
    });
    let book_next = book.clone();
    let s_next = status.clone();
    btn_next.on_click(&frame, move || {
        let count = book_next.page_count();
        if count == 0 {
            return;
        }
        let cur = book_next.selection();
        let next = (cur + 1) % count;
        book_next.set_selection(next);
        s_next.set_status_text(&format!("Simplebook → page {next}"), 0);
    });

    // ── Layout: nav buttons on top of the page area ─────────────────
    let mut nav = BoxSizer::horizontal();
    nav.add(btn_prev.as_widget_ref());
    nav.add(btn_next.as_widget_ref());
    nav.add_stretch(1);

    let mut sizer = BoxSizer::vertical();
    sizer.add_sizer(nav);
    sizer.add_stretch(1);
    frame.set_sizer(sizer);

    app.run(frame);
}
