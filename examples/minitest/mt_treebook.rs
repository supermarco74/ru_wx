//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Treebook` — a `TreeCtrl`-driven notebook with a
//! hierarchical, icon-decorated strip and real page content.
//!
//! Demonstrates:
//! - A passive `Treebook` driven by a caller-owned `TreeCtrl`
//! - `ImageList` + SVG icons on the tree nodes (`add_root_with_image`,
//!   `append_item_with_image`)
//! - A two-level hierarchy (root page with child pages)
//! - Pages with real content (Gauge, CheckBoxes, Slider, TextCtrl)
//! - Nested sizers (`add_sizer`)
//! - Expand / collapse buttons for the strip
//!
//! Run with:
//! ```bash
//! cargo run --example mt_treebook
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Frame, Gauge, ImageList, Panel, Slider, StaticText,
    StatusBar, TextCtrl, TreeCtrl, TreeItem, Treebook,
};

// 16×16 SVG glyphs for the tree strip.
const SVG_HOME: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#CE6A28" stroke-width="2"><path d="m3 11 9-8 9 8v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/></svg>"##;
const SVG_GEAR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9 17 7M7 17l-2.1 2.1"/></svg>"##;
const SVG_BRUSH: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#9B59B6" stroke-width="2"><path d="M2 22s2-1 4-1 4-7 9-12a4 4 0 0 1 6 6c-5 5-11 7-12 9s-1 4-1 4z" fill="#E6D6F0"/></svg>"##;
const SVG_WRENCH: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#4FA464" stroke-width="2"><path d="M14.7 6.3a5 5 0 0 0-6.6 6.6L3 18v3h3l5.1-5.1a5 5 0 0 0 6.6-6.6L14 13l-3-3z"/></svg>"##;

const IDX_HOME: i32 = 0;
const IDX_GEAR: i32 = 1;
const IDX_BRUSH: i32 = 2;
const IDX_WRENCH: i32 = 3;

// Content area, to the right of the tree strip.
const PAGE_X: i32 = 220;
const PAGE_Y: i32 = 10;
const PAGE_W: u32 = 510;
const PAGE_H: u32 = 350;

fn place_page(page: &Panel) {
    page.set_position(PAGE_X, PAGE_Y);
    page.set_size(PAGE_W, PAGE_H);
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Treebook + icons")
        .with_size(760, 470)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click a tree node to switch pages.", 0);

    // The tree is the "strip" that drives the book, with SVG icons.
    let tree = TreeCtrl::new(&frame);
    let icons = ImageList::new(16, 16);
    icons.add_svg_bytes(SVG_HOME);
    icons.add_svg_bytes(SVG_GEAR);
    icons.add_svg_bytes(SVG_BRUSH);
    icons.add_svg_bytes(SVG_WRENCH);
    tree.set_image_list(&icons);

    // Two-level hierarchy: a root "Overview" page plus a "Settings"
    // branch with two leaf pages.
    let n_root = tree.add_root_with_image("Overview", IDX_HOME);
    let n_settings = tree.append_item_with_image(n_root, "Settings", IDX_GEAR);
    let n_appearance = tree.append_item_with_image(n_settings, "Appearance", IDX_BRUSH);
    let n_advanced = tree.append_item_with_image(n_settings, "Advanced", IDX_WRENCH);
    tree.expand_all();

    // The book is passive — it just stores pages and shows/hides them.
    let book: Rc<Treebook> = Rc::new(Treebook::new());

    // ── Page 0 — Overview (Gauge + button) ───────────────────────────
    let page0 = Panel::new(&frame);
    let lbl0 = StaticText::new(&page0, "Overview — configuration completeness:");
    let gauge = Gauge::new(&page0, 100);
    gauge.set_value(20);
    let btn_step = Button::new(&page0, "Advance +15");
    let g_step = gauge.clone();
    let s_step = status.clone();
    btn_step.on_click(&frame, move || {
        let v = g_step.increment(15).min(100);
        s_step.set_status_text(&format!("Completeness: {v}%"), 0);
    });
    let mut sz0 = BoxSizer::vertical();
    sz0.add(lbl0.as_widget_ref());
    sz0.add(gauge.as_widget_ref());
    sz0.add(btn_step.as_widget_ref());
    sz0.add_stretch(1);
    page0.set_sizer(sz0);
    place_page(&page0);
    book.add_page("Overview", page0);

    // ── Page 1 — Settings (CheckBoxes) ───────────────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Settings — general behaviour:");
    let chk_updates = CheckBox::new(&page1, "Check for updates");
    let chk_tips = CheckBox::new(&page1, "Show tips at startup");
    chk_updates.set_checked(true);
    let cu = chk_updates.clone();
    let s_cu = status.clone();
    chk_updates.on_toggle(&frame, move || {
        s_cu.set_status_text(
            &format!("Updates: {}", if cu.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let ct = chk_tips.clone();
    let s_ct = status.clone();
    chk_tips.on_toggle(&frame, move || {
        s_ct.set_status_text(
            &format!("Tips: {}", if ct.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add(chk_updates.as_widget_ref());
    sz1.add(chk_tips.as_widget_ref());
    sz1.add_stretch(1);
    page1.set_sizer(sz1);
    place_page(&page1);
    book.add_page("Settings", page1);

    // ── Page 2 — Appearance (Slider for zoom) ────────────────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Appearance — UI zoom (50–200%):");
    let zoom = Slider::new(&page2, 50, 200, 100);
    let zoom_lbl = StaticText::new(&page2, "Zoom: 100%");
    let zoom_cb = zoom.clone();
    let zoom_lbl_cb = zoom_lbl.clone();
    let s_zoom = status.clone();
    zoom.on_value_change(&frame, move || {
        let v = zoom_cb.get_value();
        zoom_lbl_cb.set_label(&format!("Zoom: {v}%"));
        s_zoom.set_status_text(&format!("Zoom set to {v}%"), 0);
    });
    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(zoom.as_widget_ref());
    sz2.add(zoom_lbl.as_widget_ref());
    sz2.add_stretch(1);
    page2.set_sizer(sz2);
    place_page(&page2);
    book.add_page("Appearance", page2);

    // ── Page 3 — Advanced (TextCtrl with extra flags) ────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "Advanced — extra command-line flags:");
    let flags = TextCtrl::new(&page3, "--verbose");
    let btn_apply = Button::new(&page3, "Apply flags");
    let flags_c = flags.clone();
    let s_fl = status.clone();
    btn_apply.on_click(&frame, move || {
        s_fl.set_status_text(&format!("Applied flags: {}", flags_c.get_value()), 0);
    });
    let mut row3 = BoxSizer::horizontal();
    row3.add_with_proportion(flags.as_widget_ref(), 1);
    row3.add(btn_apply.as_widget_ref());
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add_sizer(row3);
    sz3.add_stretch(1);
    page3.set_sizer(sz3);
    place_page(&page3);
    book.add_page("Advanced", page3);

    status.set_status_text(&format!("{} pages", book.page_count()), 1);

    // ── Wire tree selection to book via an item → index map ──────────
    let mapping: Vec<(TreeItem, usize)> = vec![
        (n_root, 0),
        (n_settings, 1),
        (n_appearance, 2),
        (n_advanced, 3),
    ];
    let book_for_tree = book.clone();
    tree.on_selection_change(&frame, move |item| {
        if let Some(it) = item {
            if let Some(&(_, idx)) = mapping.iter().find(|(node, _)| *node == it) {
                book_for_tree.select(idx);
            }
        }
    });
    tree.select_item(n_root);

    // ── Book callback → status bar ───────────────────────────────────
    let page_names = ["Overview", "Settings", "Appearance", "Advanced"];
    let s_sel = status.clone();
    book.on_selection_change(move |idx| {
        let name = page_names.get(idx).copied().unwrap_or("?");
        s_sel.set_status_text(&format!("Treebook → page {idx} ({name})"), 0);
    });

    // ── Strip controls: expand / collapse the tree ───────────────────
    let btn_expand = Button::new(&frame, "Expand");
    let tree_exp = tree.clone();
    btn_expand.on_click(&frame, move || tree_exp.expand_all());
    let btn_collapse = Button::new(&frame, "Collapse");
    let tree_col = tree.clone();
    btn_collapse.on_click(&frame, move || tree_col.collapse_all());

    // ── Layout: tree strip + buttons on the left ─────────────────────
    let mut strip_btns = BoxSizer::horizontal();
    strip_btns.add(btn_expand.as_widget_ref());
    strip_btns.add(btn_collapse.as_widget_ref());

    let mut left = BoxSizer::vertical();
    left.add_with_proportion(tree.as_widget_ref(), 1);
    left.add_sizer(strip_btns);

    let mut sizer = BoxSizer::horizontal();
    sizer.add_sizer_with_proportion(left, 0);
    sizer.add_stretch(1);
    frame.set_sizer(sizer);

    app.run(frame);
}
