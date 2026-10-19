//! smoke_panel: Frame + MenuBar + Panel + Slider + TreeCtrl

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{
    App, BoxSizer, Frame, Menu, MenuBar, Panel, Slider, TextCtrl, TreeCtrl,
};

fn main() {
    let app = App::new();
    let mut frame = Frame::new("smoke_panel", 640, 400);
    frame.show();

    let mut mb = MenuBar::new();
    let mut file_menu = Menu::new();
    file_menu.append_text(1001, "About");
    file_menu.append_separator();
    file_menu.append_text(0xFFFF, "Quit");
    mb.append("&File", file_menu);
    frame.set_menu_bar(mb);

    let mut tree = TreeCtrl::new(&frame);
    let root = tree.add_root("Project");
    let src = tree.append_item(root, "src/");
    tree.append_item(src, "ui/");
    tree.append_item(src, "net.rs");
    tree.append_item(root, "Cargo.toml");
    tree.select_item(root);

    let panel = Panel::new(&frame);
    let _text = TextCtrl::new(&panel, "Editable text");
    let _slider = Slider::new(&panel, 0, 100);

    let mut sizer = BoxSizer::horizontal();
    sizer.add(tree.hwnd());
    sizer.add(panel.hwnd());
    sizer.layout(frame.hwnd());

    println!("entering run");
    app.run(&frame, || {});
}
