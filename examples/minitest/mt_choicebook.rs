//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Choicebook` — a `Choice` drop-down driven notebook
//! with themed pages and real content.
//!
//! Demonstrates:
//! - A passive `Choicebook` driven by a caller-owned `Choice` drop-down
//! - Pages with real content (Slider + Gauge, CheckBoxes, TextCtrl)
//! - Per-page background colours (`Panel::set_background_colour`)
//! - Nested sizers (`add_sizer`)
//! - Prev / Next buttons that drive both the drop-down and the book
//!
//! Run with:
//! ```bash
//! cargo run --example mt_choicebook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Choice, Choicebook, Colour, Frame, Gauge, Panel, Slider,
    StaticText, StatusBar, TextCtrl,
};

// Content area below the drop-down strip.
const PAGE_X: i32 = 10;
const PAGE_Y: i32 = 50;
const PAGE_W: u32 = 520;
const PAGE_H: u32 = 290;

fn place_page(page: &Panel) {
    page.set_position(PAGE_X, PAGE_Y);
    page.set_size(PAGE_W, PAGE_H);
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Choicebook")
        .with_size(560, 440)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Pick a page in the drop-down.", 0);

    // The choice is the "strip" that drives the book.
    let choice = Choice::new(&frame);
    let page_names = ["Red", "Green", "Blue"];
    for name in page_names {
        choice.append(name);
    }

    // The book is passive — it just stores pages and shows/hides them.
    let book: Rc<Choicebook> = Rc::new(Choicebook::new());

    // ── Page 1 — Red (Slider → Gauge) ────────────────────────────────
    let page1 = Panel::new(&frame);
    page1.set_background_colour(Colour::new(250, 225, 225, 255));
    let lbl1 = StaticText::new(&page1, "Red page — move the slider to fill the gauge:");
    let slider = Slider::new(&page1, 0, 100, 25);
    let gauge = Gauge::new(&page1, 100);
    gauge.set_value(25);
    let slider_cb = slider.clone();
    let gauge_cb = gauge.clone();
    let s_sld = status.clone();
    slider.on_value_change(&frame, move || {
        let v = slider_cb.get_value();
        gauge_cb.set_value(v);
        s_sld.set_status_text(&format!("Red level: {v}%"), 0);
    });
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add(slider.as_widget_ref());
    sz1.add(gauge.as_widget_ref());
    sz1.add_stretch(1);
    page1.set_sizer(sz1);
    place_page(&page1);
    book.add_page("Red", page1);

    // ── Page 2 — Green (CheckBoxes) ──────────────────────────────────
    let page2 = Panel::new(&frame);
    page2.set_background_colour(Colour::new(225, 245, 225, 255));
    let lbl2 = StaticText::new(&page2, "Green page — pick your options:");
    let chk1 = CheckBox::new(&page2, "Recycle");
    let chk2 = CheckBox::new(&page2, "Compost");
    let chk3 = CheckBox::new(&page2, "Cycle to work");
    let chks = [chk1.clone(), chk2.clone(), chk3.clone()];
    let s_eco = status.clone();
    let report = move || {
        let n = chks.iter().filter(|c| c.is_checked()).count();
        s_eco.set_status_text(&format!("{n}/3 green habits enabled"), 0);
    };
    let r1 = report.clone();
    chk1.on_toggle(&frame, r1);
    let r2 = report.clone();
    chk2.on_toggle(&frame, r2);
    let r3 = report;
    chk3.on_toggle(&frame, r3);
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(chk1.as_widget_ref());
    sz2.add(chk2.as_widget_ref());
    sz2.add(chk3.as_widget_ref());
    sz2.add_stretch(1);
    page2.set_sizer(sz2);
    place_page(&page2);
    book.add_page("Green", page2);

    // ── Page 3 — Blue (TextCtrl + echo button) ───────────────────────
    let page3 = Panel::new(&frame);
    page3.set_background_colour(Colour::new(225, 235, 250, 255));
    let lbl3 = StaticText::new(&page3, "Blue page — type a message and echo it:");
    let input = TextCtrl::new(&page3, "Hello, Choicebook!");
    let btn_echo = Button::new(&page3, "Echo to status bar");
    let echo = StaticText::new(&page3, "(nothing echoed yet)");
    let input_c = input.clone();
    let echo_c = echo.clone();
    let s_echo = status.clone();
    btn_echo.on_click(&frame, move || {
        let text = input_c.get_value();
        echo_c.set_label(&text);
        s_echo.set_status_text(&format!("Echoed {} char(s)", text.chars().count()), 0);
    });
    let mut row3 = BoxSizer::horizontal();
    row3.add_with_proportion(input.as_widget_ref(), 1);
    row3.add(btn_echo.as_widget_ref());
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add_sizer(row3);
    sz3.add(echo.as_widget_ref());
    sz3.add_stretch(1);
    page3.set_sizer(sz3);
    place_page(&page3);
    book.add_page("Blue", page3);

    status.set_status_text(&format!("{} pages", book.page_count()), 1);

    // ── Wire choice selection to book ────────────────────────────────
    let book_for_choice = book.clone();
    let choice_for_cb = choice.clone();
    choice.on_selection_change(&frame, move || {
        if let Some(idx) = choice_for_cb.get_selection() {
            book_for_choice.select(idx);
        }
    });
    choice.set_selection(0);

    // ── Prev / Next navigation drives both strip and book ────────────
    let btn_prev = Button::new(&frame, "◀ Prev");
    let btn_next = Button::new(&frame, "Next ▶");
    let book_prev = book.clone();
    let choice_prev = choice.clone();
    btn_prev.on_click(&frame, move || {
        let count = book_prev.page_count();
        if count == 0 {
            return;
        }
        let cur = book_prev.current_selection().unwrap_or(0);
        let prev = (cur + count - 1) % count;
        choice_prev.set_selection(prev);
        book_prev.select(prev);
    });
    let book_next = book.clone();
    let choice_next = choice.clone();
    btn_next.on_click(&frame, move || {
        let count = book_next.page_count();
        if count == 0 {
            return;
        }
        let cur = book_next.current_selection().unwrap_or(0);
        let next = (cur + 1) % count;
        choice_next.set_selection(next);
        book_next.select(next);
    });

    // ── Book callback → status bar with the page label ───────────────
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        let name = page_names.get(idx).copied().unwrap_or("?");
        s_sel.set_status_text(&format!("Choicebook → page {idx} ({name})"), 0);
    });

    // ── Layout: drop-down on top, nav buttons at the bottom ──────────
    // (The choice's sizer slot reserves the drop-down's full height;
    // the manually-placed pages cover the unused part of that slot.)
    let mut nav = BoxSizer::horizontal();
    nav.add(btn_prev.as_widget_ref());
    nav.add(btn_next.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(choice.as_widget_ref());
    sizer.add_stretch(1);
    sizer.add_sizer(nav);
    frame.set_sizer(sizer);

    app.run(frame);
}
