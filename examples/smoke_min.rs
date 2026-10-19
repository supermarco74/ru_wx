//! Tiny smoke: just Frame + Slider + StatusBar.

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{App, BoxSizer, Frame, Slider, StatusBar};

fn main() {
    println!("starting");
    let app = App::new();
    println!("app ok");
    let frame = Frame::new("smoke_min", 640, 400);
    println!("frame ok");
    frame.show();
    println!("frame.show() ok");

    let slider = Slider::new(&frame, 0, 100);
    let status = StatusBar::new(&frame);

    let mut sizer = BoxSizer::vertical();
    sizer.add(slider.hwnd());
    sizer.add(status.hwnd());
    sizer.layout(frame.hwnd());

    println!("entering app.run");
    app.run(&frame, || {});
}
