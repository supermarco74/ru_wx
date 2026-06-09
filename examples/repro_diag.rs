//! Diagnostic — probe the FrameData RefCell state after TextCtrl::new
#![windows_subsystem = "windows"]
use std::cell::RefCell;
use std::rc::Rc;
use ru_wx::{App, Button, Frame, StatusBar, TextCtrl};

// We need to access the inner RefCell. Use a minimal test.
fn main() {
    eprintln!("=== Test: probe RefCell state ===");
    let _app = App::new();
    let frame = Frame::builder()
        .with_title("Diagnostic")
        .with_size(1000, 600)
        .build();

    eprintln!("[1] Frame built, status bar next");
    let status = StatusBar::new(&frame, 4);
    eprintln!("[2] StatusBar created");
    status.set_status_text("(empty)", 0);
    eprintln!("[3] set_status_text(0) done");
    status.set_status_text("Field 1", 1);
    status.set_status_text("Field 2", 2);
    status.set_status_text("Field 3", 3);
    eprintln!("[4] All set_status_text done");

    eprintln!("[5] About to create TextCtrl");
    let _input = TextCtrl::new(&frame, "Hello, world!");
    eprintln!("[6] TextCtrl created");

    // After TextCtrl::new, try a sanity check using known refcount pattern.
    // We don't have direct access to inner, but we can call on_click and see
    // if it succeeds. (b1 uses Button::new + on_click)
    eprintln!("[7] About to create Button b1");
    let s = status.clone();
    let b1 = Button::new(&frame, "Button 1");
    eprintln!("[8] b1 Button::new done");
    b1.on_click(&frame, move || { s.set_status_text("1", 0); });
    eprintln!("[9] b1.on_click registered");

    let s = status.clone();
    let b2 = Button::new(&frame, "Button 2");
    eprintln!("[10] b2 Button::new done");
    b2.on_click(&frame, move || { s.set_status_text("2", 0); });
    eprintln!("[11] b2.on_click registered");

    eprintln!("[12] About to create Button b3");
    let s = status.clone();
    let b3 = Button::new(&frame, "Button 3");
    eprintln!("[13] b3 Button::new done");
    b3.on_click(&frame, move || { s.set_status_text("3", 0); });
    eprintln!("[14] b3.on_click registered - SUCCESS");

    eprintln!("=== Test complete ===");
}
