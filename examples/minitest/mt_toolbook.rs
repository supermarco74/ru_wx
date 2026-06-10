//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Toolbook` — a `ToolBar`-driven notebook with SVG tool
//! icons and real page content.
//!
//! Demonstrates:
//! - A passive `Toolbook` driven by a caller-owned `ToolBar`
//! - `ImageList` (24×24) + `ToolBar::set_image_list` + `add_tool` with
//!   per-tool icon indices, plus `add_separator`
//! - Pages with real content (Gauge, ListBox, CheckBox, TextCtrl)
//! - Nested sizers (`add_sizer`)
//! - `on_tool_clicked` → `Toolbook::select`, plus the book's own
//!   `on_selection_change` callback feeding the StatusBar
//!
//! Run with:
//! ```bash
//! cargo run --example mt_toolbook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Frame, Gauge, ImageList, ListBox, Panel, StaticText,
    StatusBar, TextCtrl, ToolBar, Toolbook,
};

// 24×24 SVG glyphs for the toolbar strip.
const SVG_HOME: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#CE6A28" stroke-width="2"><path d="m3 11 9-8 9 8v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/></svg>"##;
const SVG_LIST: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#4FA464" stroke-width="2"><path d="M8 6h13M8 12h13M8 18h13"/><circle cx="4" cy="6" r="1" fill="#4FA464"/><circle cx="4" cy="12" r="1" fill="#4FA464"/><circle cx="4" cy="18" r="1" fill="#4FA464"/></svg>"##;
const SVG_GEAR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9 17 7M7 17l-2.1 2.1"/></svg>"##;
const SVG_INFO: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#9B59B6" stroke-width="2"><circle cx="12" cy="12" r="9"/><path d="M12 16v-5M12 8h.01"/></svg>"##;

const IDX_HOME: i32 = 0;
const IDX_LIST: i32 = 1;
const IDX_GEAR: i32 = 2;
const IDX_INFO: i32 = 3;

// Content area, below the frame-attached toolbar.
const PAGE_X: i32 = 10;
const PAGE_Y: i32 = 50;
const PAGE_W: u32 = 520;
const PAGE_H: u32 = 280;

fn place_page(page: &Panel) {
    page.set_position(PAGE_X, PAGE_Y);
    page.set_size(PAGE_W, PAGE_H);
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Toolbook + icons")
        .with_size(560, 420)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click a tool icon to switch pages.", 0);

    // The toolbar is the "strip" that drives the book. Each tool gets
    // an SVG icon from the attached 24×24 image list.
    let toolbar = ToolBar::new(&frame);
    let icons = ImageList::new(24, 24);
    icons.add_svg_bytes(SVG_HOME);
    icons.add_svg_bytes(SVG_LIST);
    icons.add_svg_bytes(SVG_GEAR);
    icons.add_svg_bytes(SVG_INFO);
    toolbar.set_image_list(&icons);

    let id_home = 1001u16;
    let id_tasks = 1002u16;
    let id_options = 1003u16;
    let id_about = 1004u16;
    toolbar.add_tool(id_home, "Home", IDX_HOME);
    toolbar.add_tool(id_tasks, "Tasks", IDX_LIST);
    toolbar.add_separator();
    toolbar.add_tool(id_options, "Options", IDX_GEAR);
    toolbar.add_tool(id_about, "About", IDX_INFO);
    toolbar.realize();

    // The book is passive — it just stores pages and shows/hides them.
    let book: Rc<Toolbook> = Rc::new(Toolbook::new());

    // ── Page 0 — Home (Gauge + pulse) ────────────────────────────────
    let page0 = Panel::new(&frame);
    let lbl0 = StaticText::new(&page0, "Home — daily goal progress:");
    let gauge = Gauge::new(&page0, 100);
    gauge.set_value(60);
    let btn_bump = Button::new(&page0, "Log +5 minutes");
    let g_bump = gauge.clone();
    let s_bump = status.clone();
    btn_bump.on_click(&frame, move || {
        let v = g_bump.increment(5).min(100);
        s_bump.set_status_text(&format!("Daily goal: {v}%"), 0);
    });
    let mut sz0 = BoxSizer::vertical();
    sz0.add(lbl0.as_widget_ref());
    sz0.add(gauge.as_widget_ref());
    sz0.add(btn_bump.as_widget_ref());
    sz0.add_stretch(1);
    page0.set_sizer(sz0);
    place_page(&page0);
    book.add_page("Home", page0);

    // ── Page 1 — Tasks (ListBox + add row) ───────────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Tasks:");
    let tasks = ListBox::new(&page1);
    for t in ["Draw toolbar icons", "Wire up the book", "Test everything"] {
        tasks.append(t);
    }
    let input = TextCtrl::new(&page1, "");
    let btn_add = Button::new(&page1, "Add");
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
    let mut row1 = BoxSizer::horizontal();
    row1.add_with_proportion(input.as_widget_ref(), 1);
    row1.add(btn_add.as_widget_ref());
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add_with_proportion(tasks.as_widget_ref(), 1);
    sz1.add_sizer(row1);
    page1.set_sizer(sz1);
    place_page(&page1);
    book.add_page("Tasks", page1);

    // ── Page 2 — Options (CheckBoxes) ────────────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Options:");
    let chk_sound = CheckBox::new(&page2, "Play sounds");
    let chk_sync = CheckBox::new(&page2, "Sync on startup");
    chk_sync.set_checked(true);
    let cs = chk_sound.clone();
    let s_cs = status.clone();
    chk_sound.on_toggle(&frame, move || {
        s_cs.set_status_text(
            &format!("Sounds: {}", if cs.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let cy = chk_sync.clone();
    let s_cy = status.clone();
    chk_sync.on_toggle(&frame, move || {
        s_cy.set_status_text(
            &format!("Sync: {}", if cy.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(chk_sound.as_widget_ref());
    sz2.add(chk_sync.as_widget_ref());
    sz2.add_stretch(1);
    page2.set_sizer(sz2);
    place_page(&page2);
    book.add_page("Options", page2);

    // ── Page 3 — About (read-only TextCtrl) ──────────────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "About this minitest:");
    let about = TextCtrl::multiline(
        &page3,
        "Toolbook minitest\r\n\r\nA ToolBar with SVG icons drives a passive \
         Toolbook.\r\nEach tool id maps to one page index.",
    );
    about.set_readonly(true);
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add_with_proportion(about.as_widget_ref(), 1);
    page3.set_sizer(sz3);
    place_page(&page3);
    book.add_page("About", page3);

    status.set_status_text(&format!("{} pages", book.page_count()), 1);

    // ── Wire toolbar tool-click to book by id ────────────────────────
    let book_for_tb = book.clone();
    toolbar.on_tool_clicked(&frame, move |id| {
        let idx = match id {
            x if x == id_home => 0,
            x if x == id_tasks => 1,
            x if x == id_options => 2,
            x if x == id_about => 3,
            _ => return,
        };
        book_for_tb.select(idx);
    });

    // ── Book callback → status bar with the page label ───────────────
    let page_names = ["Home", "Tasks", "Options", "About"];
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        let name = page_names.get(idx).copied().unwrap_or("?");
        s_sel.set_status_text(&format!("Toolbook → page {idx} ({name})"), 0);
    });

    // Layout.
    // The `ToolBar` is frame-attached (it owns its own row at the top
    // of the frame), so it is NOT added to a sizer; the pages are
    // positioned manually below it. An empty sizer reserves the
    // frame's client area.
    let sizer = BoxSizer::vertical();
    frame.set_sizer(sizer);

    app.run(frame);
}
