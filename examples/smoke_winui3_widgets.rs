//! Smoke test for the winui3 backend.
//!
//! Run with:
//! ```bash
//! cargo run --release --example smoke_winui3_widgets --features winui3
//! ```

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{
    events, set_frame_icon, svg_to_hicon, AcceleratorKey, AcceleratorTable, App, BoxSizer, Frame,
    Menu, MenuBar, Panel, Slider, StatusBar, TextCtrl, Timer, TreeCtrl,
};

fn main() {
    let app = App::new();
    let mut frame = Frame::new("winui3 smoke test", 960, 640);
    frame.center();
    frame.show();

    // --- Icon ---
    let svg: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><circle cx='16' cy='16' r='12' fill='%234f46e5'/></svg>";
    if let Some(h) = svg_to_hicon(svg, 32) {
        set_frame_icon(frame.hwnd(), h);
    }

    // --- Menu bar ---
    let mut mb = MenuBar::new();
    let mut file_menu = Menu::new();
    let id_about: u16 = 1001;
    file_menu.append_text(id_about, "About");
    file_menu.append_separator();
    file_menu.append_text(0xFFFF, "Quit");
    mb.append("&File", file_menu);
    frame.set_menu_bar(mb);

    // --- Tree Ctrl on the left (uses the frame as parent) ---
    let mut tree = TreeCtrl::new(&frame);
    let root = tree.add_root("Project");
    let src = tree.append_item(root, "src/");
    tree.append_item(src, "ui/");
    tree.append_item(src, "net.rs");
    tree.append_item(root, "Cargo.toml");
    tree.select_item(root);

    // --- Panel + slider on the right ---
    let panel = Panel::new(&frame);
    let _text = TextCtrl::new(&panel, "Editable text");
    let slider = Slider::new(&panel, 0, 100);

    // --- Status bar ---
    let mut status = StatusBar::new(&frame);
    status.set_part_count(2);
    status.set_part_text(0, "Ready");
    status.set_part_text(1, "smoke test");

    // --- Callbacks ---
    events::register_command(frame.hwnd(), id_about, |_| println!("About clicked"));
    slider.on_change(&frame, |v| println!("slider = {v}"));
    frame.set_on_close(|| println!("close requested"));

    // --- Layout: side-by-side tree | panel ---
    let mut sizer = BoxSizer::horizontal();
    sizer.add(tree.hwnd());
    sizer.add(panel.hwnd());
    sizer.layout(frame.hwnd());

    // --- Timer + accelerator ---
    let _timer = Timer::new(&frame, 500, || println!("tick"));
    let accel = AcceleratorTable::new(&[
        AcceleratorKey::parse("Ctrl+S", id_about).unwrap(),
        AcceleratorKey::parse("Alt+X", 0xFFFF).unwrap(),
    ]);
    println!("accel entries = {}", accel.entries().len());

    println!("Smoke test ready. Close window to exit.");
    app.run(&frame, || {});
}
