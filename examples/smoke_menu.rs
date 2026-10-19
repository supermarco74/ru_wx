//! smoke_menu: Frame + MenuBar only.

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{App, Frame, Menu, MenuBar};

fn main() {
    let app = App::new();
    let mut frame = Frame::new("smoke_menu", 640, 400);
    frame.show();

    let mut mb = MenuBar::new();
    let mut file_menu = Menu::new();
    file_menu.append_text(1001, "About");
    file_menu.append_separator();
    file_menu.append_text(0xFFFF, "Quit");
    mb.append("&File", file_menu);
    frame.set_menu_bar(mb);

    println!("entering run");
    app.run(&frame, || {});
}
