//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minimal reproducer - try without StaticText/TextCtrl
#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Repro — no StaticText/TextCtrl")
        .with_size(1000, 600)
        .build();

    let status = StatusBar::new(&frame, 4);
    status.set_status_text("(empty)", 0);
    status.set_status_text("Field 1 — fixed label", 1);
    status.set_status_text("Field 2 — fixed label", 2);
    status.set_status_text("Field 3 — fixed label", 3);

    let s = status.clone();
    let btn_send_input = Button::new(&frame, "Set status ← input box");
    btn_send_input.on_click(&frame, move || {
        s.set_status_text("foo", 0);
    });

    let s = status.clone();
    let btn_long = Button::new(
        &frame,
        "Preset: long string (80+ chars, no truncation)",
    );
    btn_long.on_click(&frame, move || {
        s.set_status_text("bar", 0);
    });

    let s = status.clone();
    let btn_short = Button::new(&frame, "Preset: short (\"OK\")");
    btn_short.on_click(&frame, move || {
        s.set_status_text("OK", 0);
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_send_input.as_widget_ref());
    sizer.add(btn_long.as_widget_ref());
    sizer.add(btn_short.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
