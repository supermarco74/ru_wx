//! smoke_tree: add TreeCtrl on top of smoke_min.

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{App, BoxSizer, Frame, Slider, StatusBar, TreeCtrl};

fn main() {
    let app = App::new();
    let frame = Frame::new("smoke_tree", 640, 400);
    frame.show();

    let mut tree = TreeCtrl::new(&frame);
    let root = tree.add_root("Project");
    let src = tree.append_item(root, "src/");
    tree.append_item(src, "ui/");
    tree.append_item(src, "net.rs");
    tree.append_item(root, "Cargo.toml");
    tree.select_item(root);

    let slider = Slider::new(&frame, 0, 100);
    let status = StatusBar::new(&frame);

    let mut sizer = BoxSizer::vertical();
    sizer.add(tree.hwnd());
    sizer.add(slider.hwnd());
    sizer.add(status.hwnd());
    sizer.layout(frame.hwnd());

    println!("entering app.run");
    app.run(&frame, || {});
}
