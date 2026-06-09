//! Minitest: `Toolbook` — a `ToolBar`-driven notebook.
//!
//! Demonstrates:
//! - Creating a passive `Toolbook` (the caller owns the `ToolBar`
//!   that drives it).
//! - Adding pages (panels parented to the frame) with labels.
//! - Wiring the toolbar's tool click to the book by id.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_toolbook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, Frame, Panel, StaticText, StatusBar, ToolBar, Toolbook,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Toolbook")
        .with_size(540, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click a tool to switch pages.", 0);

    // The toolbar is the "strip" that drives the book. We assign a
    // unique id to each tool and map id → page index manually.
    let toolbar = ToolBar::new(&frame);
    let id_alpha = 1001u16;
    let id_beta = 1002u16;
    let id_gamma = 1003u16;
    let id_delta = 1004u16;
    toolbar.add_tool(id_alpha, "Alpha", -1);
    toolbar.add_tool(id_beta, "Beta", -1);
    toolbar.add_tool(id_gamma, "Gamma", -1);
    toolbar.add_tool(id_delta, "Delta", -1);
    // Commit the buffered buttons to the native control so they are drawn.
    toolbar.realize();

    // The book is passive — it just stores pages and shows/hides them.
    let book: Rc<Toolbook> = Rc::new(Toolbook::new());

    // ── Page 1 — Alpha ──────────────────────────────────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Page 1 — Alpha content");
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
    let lbl2 = StaticText::new(&page2, "Page 2 — Beta content");
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
    let lbl3 = StaticText::new(&page3, "Page 3 — Gamma content");
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
    let lbl4 = StaticText::new(&page4, "Page 4 — Delta content");
    let mut sz4 = BoxSizer::vertical();
    sz4.add(lbl4.as_widget_ref());
    page4.set_sizer(sz4);
    book.add_page("Delta", page4);

    // Wire toolbar tool-click to book by id.
    let book_for_tb = book.clone();
    toolbar.on_tool_clicked(&frame, move |id| {
        let idx = match id {
            x if x == id_alpha => 0,
            x if x == id_beta => 1,
            x if x == id_gamma => 2,
            x if x == id_delta => 3,
            _ => return,
        };
        book_for_tb.select(idx);
    });

    // React to the book changing selection via its own callback.
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        s_sel.set_status_text(&format!("Toolbook → page {idx}"), 0);
    });

    // Layout.
    // The `ToolBar` is frame-attached (it owns its own row at the top of
    // the frame, like in wxWidgets), so it is NOT added to a sizer. We
    // install an empty sizer so the frame's client area is reserved for
    // the active book page.
    let sizer = BoxSizer::vertical();
    frame.set_sizer(sizer);

    app.run(frame);
}
