//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minimal reproducer — 4 buttons + status bar (no StaticText/TextCtrl)
#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StatusBar};

fn main() {
    let app = App::new();
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
    b1.on_click(&frame, move || {
        s.set_status_text("1", 0);
    });

    let s = status.clone();
    let b2 = Button::new(&frame, "Button 2");
    b2.on_click(&frame, move || {
        s.set_status_text("2", 0);
    });

    let s = status.clone();
    let b3 = Button::new(&frame, "Button 3");
    b3.on_click(&frame, move || {
        s.set_status_text("3", 0);
    });

    let s = status.clone();
    let b4 = Button::new(&frame, "Button 4");
    b4.on_click(&frame, move || {
        s.set_status_text("4", 0);
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(b1.as_widget_ref());
    sizer.add(b2.as_widget_ref());
    sizer.add(b3.as_widget_ref());
    sizer.add(b4.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
