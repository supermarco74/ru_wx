//! Minitest: `TreeCtrl` — hierarchical item tree.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_tree_ctrl
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Frame, StaticText, StatusBar, TreeCtrl};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TreeCtrl")
        .with_size(420, 460)
        .build();

    let _hint = StaticText::new(&frame, "Click an item to see its label in the status bar.");

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Select a node…", 0);

    let tree = TreeCtrl::new(&frame);

    // Build a small project-style tree
    let root = tree.add_root("Project");

    let src = tree.append_item(root, "src");
    tree.append_item(src, "main.rs");
    tree.append_item(src, "lib.rs");
    let modules = tree.append_item(src, "modules");
    tree.append_item(modules, "auth.rs");
    tree.append_item(modules, "db.rs");
    tree.append_item(modules, "ui.rs");

    let assets = tree.append_item(root, "assets");
    tree.append_item(assets, "logo.png");
    tree.append_item(assets, "styles.css");

    let docs = tree.append_item(root, "docs");
    tree.append_item(docs, "README.md");
    tree.append_item(docs, "CHANGELOG.md");

    tree.expand(root);
    tree.expand(src);

    // `TreeCtrl` doesn't expose `get_item_text`; report the raw handle.
    let s = status.clone();
    tree.on_selection_change(&frame, move |item| match item {
        Some(it) => s.set_status_text(&format!("Selected item handle: {}", it.0), 0),
        None => s.set_status_text("(no selection)", 0),
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(tree.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
