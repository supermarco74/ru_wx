//! Minitest: `Tab` — notebook with three pages, each containing buttons.
//!
//! Demonstrates:
//! - Creating a tab control as a child of the frame
//! - Building Panel pages and adding child widgets to them
//! - Adding multiple pages to the notebook
//! - Reacting to page-selection changes
//!
//! Run with:
//! ```bash
//! cargo run --example mt_tab
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, Panel, StaticText, StatusBar, Tab};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Tab")
        .with_size(540, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Switch tabs and click any button.", 0);

    let notebook = Tab::new(&frame);

    // ── Page 1 ───────────────────────────────────────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Page 1 — alpha");
    let btn1a = Button::new(&page1, "Alpha 1");
    let btn1b = Button::new(&page1, "Alpha 2");
    let s1 = status.clone();
    btn1a.on_click(&frame, move || s1.set_status_text("Page1 → Alpha 1", 0));
    let s2 = status.clone();
    btn1b.on_click(&frame, move || s2.set_status_text("Page1 → Alpha 2", 0));
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add(btn1a.as_widget_ref());
    sz1.add(btn1b.as_widget_ref());
    page1.set_sizer(sz1);

    // ── Page 2 ───────────────────────────────────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Page 2 — beta");
    let btn2a = Button::new(&page2, "Beta 1");
    let btn2b = Button::new(&page2, "Beta 2");
    let btn2c = Button::new(&page2, "Beta 3");
    let s3 = status.clone();
    btn2a.on_click(&frame, move || s3.set_status_text("Page2 → Beta 1", 0));
    let s4 = status.clone();
    btn2b.on_click(&frame, move || s4.set_status_text("Page2 → Beta 2", 0));
    let s5 = status.clone();
    btn2c.on_click(&frame, move || s5.set_status_text("Page2 → Beta 3", 0));
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(btn2a.as_widget_ref());
    sz2.add(btn2b.as_widget_ref());
    sz2.add(btn2c.as_widget_ref());
    page2.set_sizer(sz2);

    // ── Page 3 ───────────────────────────────────────────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "Page 3 — gamma");
    let btn3a = Button::new(&page3, "Gamma A");
    let btn3b = Button::new(&page3, "Gamma B");
    let s6 = status.clone();
    btn3a.on_click(&frame, move || s6.set_status_text("Page3 → Gamma A", 0));
    let s7 = status.clone();
    btn3b.on_click(&frame, move || s7.set_status_text("Page3 → Gamma B", 0));
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add(btn3a.as_widget_ref());
    sz3.add(btn3b.as_widget_ref());
    page3.set_sizer(sz3);

    // ── Add the pages ────────────────────────────────────────────────
    notebook.add_page("Alpha", &page1);
    notebook.add_page("Beta", &page2);
    notebook.add_page("Gamma", &page3);

    let s_sel = status.clone();
    notebook.on_selection_change(&frame, move |idx| {
        s_sel.set_status_text(&format!("Selected tab: {idx}"), 0);
    });

    app.run(frame);
}
