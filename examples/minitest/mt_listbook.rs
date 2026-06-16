//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Listbook` — a `ListBox`-driven notebook with real content.
//!
//! Demonstrates:
//! - A passive `Listbook` driven by a caller-owned `ListBox` strip
//! - Pages with real, interactive content (Gauge, ListBox, CheckBox, TextCtrl)
//! - Nested sizers (`add_sizer` / `add_sizer_with_proportion`)
//! - Prev / Next navigation buttons that drive both the strip and the book
//! - `on_selection_change`, `current_selection` and `page_count`
//!
//! Run with:
//! ```bash
//! cargo run --example mt_listbook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Frame, Gauge, ListBox, Listbook, Panel, StaticText,
    StatusBar, TextCtrl,
};

// Where the page panels live (to the right of the list strip).
const PAGE_X: i32 = 220;
const PAGE_Y: i32 = 10;
const PAGE_W: u32 = 510;
const PAGE_H: u32 = 330;

/// Position a freshly-built page in the content area.
fn place_page(page: &Panel) {
    page.set_position(PAGE_X, PAGE_Y);
    page.set_size(PAGE_W, PAGE_H);
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Listbook")
        .with_size(760, 440)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click a page in the list, or use Prev/Next.", 0);

    // The listbox is the "strip" that drives the book.
    let list = ListBox::new(&frame);
    let page_names = ["Dashboard", "Tasks", "Options", "Notes"];
    for name in page_names {
        list.append(name);
    }

    // The book is passive — it just stores pages and shows/hides them.
    let book: Rc<Listbook> = Rc::new(Listbook::new());

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
    book.add_page("Dashboard", page1);

    // ── Page 2 — Tasks (ListBox + add row) ───────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Tasks — type and add:");
    let tasks = ListBox::new(&page2);
    for t in ["Fix the sizer bug", "Add listbook icons", "Release"] {
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
    book.add_page("Tasks", page2);

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
            &format!("Line numbers: {}", if c1.is_checked() { "ON" } else { "OFF" }),
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
    book.add_page("Options", page3);

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
    book.add_page("Notes", page4);

    status.set_status_text(&format!("{} pages", book.page_count()), 1);

    // ── Wire the listbox strip to the book ───────────────────────────
    let book_for_list = book.clone();
    let list_for_cb = list.clone();
    list.on_selection_change(&frame, move || {
        if let Some(idx) = list_for_cb.get_selection() {
            book_for_list.select(idx);
        }
    });
    list.set_selection(0);

    // ── Prev / Next buttons drive both the strip and the book ────────
    let btn_prev = Button::new(&frame, "◀ Prev");
    let btn_next = Button::new(&frame, "Next ▶");
    let book_prev = book.clone();
    let list_prev = list.clone();
    btn_prev.on_click(&frame, move || {
        let count = book_prev.page_count();
        if count == 0 {
            return;
        }
        let cur = book_prev.current_selection().unwrap_or(0);
        let prev = (cur + count - 1) % count;
        list_prev.set_selection(prev);
        book_prev.select(prev);
    });
    let book_next = book.clone();
    let list_next = list.clone();
    btn_next.on_click(&frame, move || {
        let count = book_next.page_count();
        if count == 0 {
            return;
        }
        let cur = book_next.current_selection().unwrap_or(0);
        let next = (cur + 1) % count;
        list_next.set_selection(next);
        book_next.select(next);
    });

    // ── Book callback → status bar (with the page label) ─────────────
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        let name = page_names.get(idx).copied().unwrap_or("?");
        s_sel.set_status_text(&format!("Listbook → page {idx} ({name})"), 0);
    });

    // ── Layout: list strip + nav buttons on the left ─────────────────
    let mut nav = BoxSizer::horizontal();
    nav.add(btn_prev.as_widget_ref());
    nav.add(btn_next.as_widget_ref());

    let mut left = BoxSizer::vertical();
    left.add_with_proportion(list.as_widget_ref(), 1);
    left.add_sizer(nav);

    let mut sizer = BoxSizer::horizontal();
    sizer.add_sizer_with_proportion(left, 0);
    sizer.add_stretch(1);
    frame.set_sizer(sizer);

    app.run(frame);
}
