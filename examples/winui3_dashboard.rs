//! Minimal dashboard — step 1 of debugging.

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{App, Frame};

fn main() {
    let app = App::new();
    let frame = Frame::new("Dashboard debug", 800, 600);
    frame.show();
    println!("about to run");
    app.run(&frame, || {});
}
