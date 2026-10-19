// Debug build of smoke_full — *no* windows subsystem so the println!
// output is visible.

use ru_wx::platform::winui3::{
    App, BoxSizer, Frame, Menu, MenuBar, Panel, Slider, StatusBar, TextCtrl, TreeCtrl,
};

fn main() {
    println!("1: starting");
    let app = App::new();
    println!("2: app ok");
    let mut frame = Frame::new("smoke_dbg", 640, 400);
    println!("3: frame ok");
    frame.show();
    println!("4: frame.show() ok");

    let mut mb = MenuBar::new();
    let mut file_menu = Menu::new();
    file_menu.append_text(1001, "About");
    file_menu.append_separator();
    file_menu.append_text(0xFFFF, "Quit");
    mb.append("&File", file_menu);
    frame.set_menu_bar(mb);
    println!("5: menu ok");

    let mut tree = TreeCtrl::new(&frame);
    tree.add_root("Project");
    println!("6: tree ok");

    let panel = Panel::new(&frame);
    let _text = TextCtrl::new(&panel, "Editable text");
    let slider = Slider::new(&panel, 0, 100);
    println!("7: panel ok");

    let mut status = StatusBar::new(&frame);
    status.set_part_count(2);
    status.set_part_text(0, "Ready");
    status.set_part_text(1, "smoke_dbg");

    let mut sizer = BoxSizer::horizontal();
    sizer.add(tree.hwnd());
    sizer.add(panel.hwnd());
    sizer.layout(frame.hwnd());

    println!("8: about to construct AcceleratorTable...");
    use ru_wx::platform::winui3::{AcceleratorKey, AcceleratorTable};
    let accel = AcceleratorTable::new(&[AcceleratorKey::parse("Ctrl+S", 1001).unwrap()]);
    println!("9: accelerator entries = {}", accel.entries().len());

    println!("10: entering run");
    let _ = slider;
    app.run(&frame, || {});
}
