//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `ListCtrl` (report view) — columns, rows with SVG icons,
//! selection events and runtime add/remove.
//!
//! Demonstrates:
//! - `ListCtrl::set_image_list` + `insert_item_with_image` (per-row icons)
//! - Multi-column report view (`insert_column` / `set_item_text`)
//! - `on_item_selected` callback wired to the status bar
//! - Adding / deleting rows at runtime with nested button row
//!
//! Run with:
//! ```bash
//! cargo run --example mt_list_ctrl
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Button, Frame, ImageList, ListCtrl, ListCtrlStyle, StaticText, StatusBar,
};

const SVG_OK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3FA34D" stroke-width="3"><path d="M4 13l5 5L20 6"/></svg>"##;
const SVG_WARN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#E0A100" stroke-width="2"><path d="M12 3 2 21h20z" fill="#F7D154"/><path d="M12 10v5M12 18h.01"/></svg>"##;
const SVG_ERR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#D64545" stroke-width="3"><circle cx="12" cy="12" r="9"/><path d="M8 8l8 8M16 8l-8 8"/></svg>"##;

const IDX_OK: i32 = 0;
const IDX_WARN: i32 = 1;
const IDX_ERR: i32 = 2;

/// (icon, service, status text, latency)
const ROWS: &[(i32, &str, &str, &str)] = &[
    (IDX_OK, "web-frontend", "healthy", "12 ms"),
    (IDX_OK, "auth-service", "healthy", "8 ms"),
    (IDX_WARN, "search-index", "degraded", "230 ms"),
    (IDX_OK, "payments", "healthy", "18 ms"),
    (IDX_ERR, "email-relay", "down", "—"),
    (IDX_WARN, "thumbnailer", "degraded", "510 ms"),
    (IDX_OK, "database", "healthy", "3 ms"),
];

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ListCtrl report + icons")
        .with_size(560, 480)
        .with_modern_style().build();

    let _hint = StaticText::new(&frame, "Service monitor — click a row, or add/remove rows.");

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Select a service…", 0);

    let list = ListCtrl::new(&frame, ListCtrlStyle::Report);
    list.insert_column(0, "Service", 180);
    list.insert_column(1, "Status", 120);
    list.insert_column(2, "Latency", 100);

    // ── Per-row icons from inline SVG ────────────────────────────────
    let icons = ImageList::new(16, 16);
    icons.add_svg_bytes(SVG_OK);
    icons.add_svg_bytes(SVG_WARN);
    icons.add_svg_bytes(SVG_ERR);
    list.set_image_list(&icons);

    for (i, (icon, name, state, latency)) in ROWS.iter().enumerate() {
        list.insert_item_with_image(i, name, *icon);
        list.set_item_text(i, 1, state);
        list.set_item_text(i, 2, latency);
    }
    status.set_status_text(&format!("{} services", list.get_item_count()), 1);

    // ── Selection → status bar ───────────────────────────────────────
    let s = status.clone();
    list.on_item_selected(&frame, move |sel| match sel {
        Some(idx) => s.set_status_text(&format!("Selected row {idx}"), 0),
        None => s.set_status_text("(no selection)", 0),
    });

    // ── Runtime add / remove ─────────────────────────────────────────
    let btn_add = Button::new(&frame, "Add service");
    let list_for_add = list.clone();
    let s_add = status.clone();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0u32));
    btn_add.on_click(&frame, move || {
        counter.set(counter.get() + 1);
        let n = list_for_add.get_item_count();
        let name = format!("worker-{:02}", counter.get());
        list_for_add.insert_item_with_image(n, &name, IDX_OK);
        list_for_add.set_item_text(n, 1, "starting");
        list_for_add.set_item_text(n, 2, "…");
        s_add.set_status_text(&format!("{} services", n + 1), 1);
    });

    let btn_del = Button::new(&frame, "Delete selected");
    let list_for_del = list.clone();
    let s_del = status.clone();
    btn_del.on_click(&frame, move || {
        let sel = list_for_del.get_selected_items();
        // Delete bottom-up so the remaining indices stay valid.
        for idx in sel.into_iter().rev() {
            list_for_del.delete_item(idx);
        }
        s_del.set_status_text(&format!("{} services", list_for_del.get_item_count()), 1);
    });

    let btn_fail = Button::new(&frame, "Mark selected as down");
    let list_for_fail = list.clone();
    btn_fail.on_click(&frame, move || {
        for idx in list_for_fail.get_selected_items() {
            list_for_fail.set_item_text(idx, 1, "down");
            list_for_fail.set_item_text(idx, 2, "—");
        }
    });

    // ── Layout: list fills, button row at the bottom ─────────────────
    let mut buttons = BoxSizer::horizontal();
    buttons.add(btn_add.as_widget_ref());
    buttons.add(btn_del.as_widget_ref());
    buttons.add(btn_fail.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    sizer.add_with_proportion(list.as_widget_ref(), 1);
    sizer.add_sizer(buttons);
    frame.set_sizer(sizer);

    app.run(frame);
}
