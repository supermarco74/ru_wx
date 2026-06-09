//! Test: 2 buttons + TextCtrl + status bar
//! If this passes, the bug requires the 3rd on_click
#![windows_subsystem = "windows"]
use ru_wx::{App, Button, Frame, StatusBar, TextCtrl};
fn main() {
    let _app = App::new();
    let frame = Frame::builder()
        .with_title("Repro 7 — 2 buttons + TextCtrl")
        .with_size(1000, 600)
        .build();
    let status = StatusBar::new(&frame, 4);
    status.set_status_text("(empty)", 0);
    let _input = TextCtrl::new(&frame, "Hello, world!");
    eprintln!("[T1] TextCtrl created");
    let s = status.clone();
    let b1 = Button::new(&frame, "Button 1");
    b1.on_click(&frame, move || { s.set_status_text("1", 0); });
    eprintln!("[T2] b1.on_click registered");
    let s = status.clone();
    eprintln!("[T3] About to call b2.on_click");
    let b2 = Button::new(&frame, "Button 2");
    b2.on_click(&frame, move || { s.set_status_text("2", 0); });
    eprintln!("[T4] b2.on_click registered - SUCCESS");
    eprintln!("=== Test passed ===");
}
