//! Minimal reproducer — 4 buttons + status bar (no StaticText/TextCtrl)
//! Tests if 4th button is the trigger.
#![windows_subsystem = "windows"]
use ru_wx::{App, Button, Frame, StatusBar};
fn main() {
    let _app = App::new();
    let frame = Frame::builder()
        .with_title("Repro 4 — 4 buttons, no StaticText/TextCtrl")
        .with_size(1000, 600)
        .build();
    let status = StatusBar::new(&frame, 4);
    status.set_status_text("(empty)", 0);
    status.set_status_text("Field 1 — fixed label", 1);
    status.set_status_text("Field 2 — fixed label", 2);
    status.set_status_text("Field 3 — fixed label", 3);
    let s = status.clone();
    let b1 = Button::new(&frame, "Button 1");
    b1.on_click(&frame, move || { s.set_status_text("1", 0); });
    eprintln!("b1.on_click registered");
    let s = status.clone();
    let b2 = Button::new(&frame, "Button 2");
    b2.on_click(&frame, move || { s.set_status_text("2", 0); });
    eprintln!("b2.on_click registered");
    let s = status.clone();
    let b3 = Button::new(&frame, "Button 3");
    b3.on_click(&frame, move || { s.set_status_text("3", 0); });
    eprintln!("b3.on_click registered");
    let s = status.clone();
    eprintln!("About to register b4.on_click");
    let b4 = Button::new(&frame, "Button 4");
    b4.on_click(&frame, move || { s.set_status_text("4", 0); });
    eprintln!("b4.on_click registered");
    eprintln!("All on_clicks registered successfully");
}
