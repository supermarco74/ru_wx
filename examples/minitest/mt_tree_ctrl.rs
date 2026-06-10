//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `TreeCtrl` — hierarchical item tree with per-item SVG
//! icons (`set_image_list` / `append_item_with_image`).
//!
//! Demonstrates:
//! - `ImageList` + inline SVG icons on tree nodes
//! - `add_root_with_image` / `append_item_with_image`
//! - `get_item_text` in the selection callback
//! - Expand all / collapse all buttons
//! - Adding and deleting nodes at runtime
//!
//! Run with:
//! ```bash
//! cargo run --example mt_tree_ctrl
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, ImageList, StaticText, StatusBar, TreeCtrl};

// Small monochrome glyphs, rasterised by resvg at 16×16.
const SVG_FOLDER: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#E8B339" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="#E8B339"/></svg>"##;
const SVG_RUST: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#CE6A28" stroke-width="2"><circle cx="12" cy="12" r="9"/><path d="M8 16V8h5a2.5 2.5 0 0 1 0 5H8m5 0 3 3"/></svg>"##;
const SVG_IMAGE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5" fill="#3A86C8"/><path d="m21 15-5-5L5 21"/></svg>"##;
const SVG_DOC: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#4FA464" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M9 13h6M9 17h6"/></svg>"##;

const IDX_FOLDER: i32 = 0;
const IDX_RUST: i32 = 1;
const IDX_IMAGE: i32 = 2;
const IDX_DOC: i32 = 3;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TreeCtrl + icons")
        .with_size(460, 560)
        .with_modern_style().build();

    let _hint = StaticText::new(
        &frame,
        "Click a node — its text appears in the status bar.",
    );

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Select a node…", 0);

    let tree = TreeCtrl::new(&frame);

    // ── Image list: 4 SVG glyphs at 16×16 ───────────────────────────
    let icons = ImageList::new(16, 16);
    icons.add_svg_bytes(SVG_FOLDER);
    icons.add_svg_bytes(SVG_RUST);
    icons.add_svg_bytes(SVG_IMAGE);
    icons.add_svg_bytes(SVG_DOC);
    tree.set_image_list(&icons);
    status.set_status_text(&format!("{} icons loaded", icons.count()), 1);

    // ── Project-style tree with per-node icons ──────────────────────
    let root = tree.add_root_with_image("Project", IDX_FOLDER);

    let src = tree.append_item_with_image(root, "src", IDX_FOLDER);
    tree.append_item_with_image(src, "main.rs", IDX_RUST);
    tree.append_item_with_image(src, "lib.rs", IDX_RUST);
    let modules = tree.append_item_with_image(src, "modules", IDX_FOLDER);
    tree.append_item_with_image(modules, "auth.rs", IDX_RUST);
    tree.append_item_with_image(modules, "db.rs", IDX_RUST);
    tree.append_item_with_image(modules, "ui.rs", IDX_RUST);

    let assets = tree.append_item_with_image(root, "assets", IDX_FOLDER);
    tree.append_item_with_image(assets, "logo.png", IDX_IMAGE);
    tree.append_item_with_image(assets, "banner.jpg", IDX_IMAGE);

    let docs = tree.append_item_with_image(root, "docs", IDX_FOLDER);
    tree.append_item_with_image(docs, "README.md", IDX_DOC);
    tree.append_item_with_image(docs, "CHANGELOG.md", IDX_DOC);

    tree.expand(root);
    tree.expand(src);

    // ── Selection → status bar (uses get_item_text) ─────────────────
    let s = status.clone();
    let tree_for_sel = tree.clone();
    tree.on_selection_change(&frame, move |item| match item {
        Some(it) => {
            let text = tree_for_sel
                .get_item_text(it)
                .unwrap_or_else(|| format!("handle {}", it.0));
            s.set_status_text(&format!("Selected: {text}"), 0);
        }
        None => s.set_status_text("(no selection)", 0),
    });

    // ── Runtime operations ───────────────────────────────────────────
    let btn_expand = Button::new(&frame, "Expand all");
    let tree_for_expand = tree.clone();
    btn_expand.on_click(&frame, move || tree_for_expand.expand_all());

    let btn_collapse = Button::new(&frame, "Collapse all");
    let tree_for_collapse = tree.clone();
    btn_collapse.on_click(&frame, move || tree_for_collapse.collapse_all());

    // Add a new numbered file under the selected folder (or root).
    let btn_add = Button::new(&frame, "Add node under selection");
    let tree_for_add = tree.clone();
    let s_add = status.clone();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0u32));
    btn_add.on_click(&frame, move || {
        let parent = tree_for_add.get_selection().unwrap_or(root);
        counter.set(counter.get() + 1);
        let name = format!("new_file_{}.rs", counter.get());
        let item = tree_for_add.append_item_with_image(parent, &name, IDX_RUST);
        tree_for_add.expand(parent);
        tree_for_add.select_item(item);
        s_add.set_status_text(&format!("Added {name}"), 0);
    });

    let btn_delete = Button::new(&frame, "Delete selection");
    let tree_for_del = tree.clone();
    let s_del = status.clone();
    btn_delete.on_click(&frame, move || {
        if let Some(item) = tree_for_del.get_selection() {
            if item != root {
                tree_for_del.delete_item(item);
                s_del.set_status_text("Node deleted", 0);
            } else {
                s_del.set_status_text("Refusing to delete the root", 0);
            }
        }
    });

    // ── Layout ───────────────────────────────────────────────────────
    let mut buttons = BoxSizer::horizontal();
    buttons.add(btn_expand.as_widget_ref());
    buttons.add(btn_collapse.as_widget_ref());
    buttons.add(btn_add.as_widget_ref());
    buttons.add(btn_delete.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    sizer.add_with_proportion(tree.as_widget_ref(), 1);
    sizer.add_sizer(buttons);
    frame.set_sizer(sizer);

    app.run(frame);
}
