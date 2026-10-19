//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minimal reproducer — StaticText only + 3 buttons + status bar
#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Repro 5 — StaticText + 3 buttons")
        .with_size(1000, 600)
        .build();

    let status = StatusBar::new(&frame, 4);
    status.set_status_text("(empty)", 0);
    status.set_status_text("Field 1 — fixed label", 1);
    status.set_status_text("Field 2 — fixed label", 2);
    status.set_status_text("Field 3 — fixed label", 3);

    let lbl = StaticText::new(&frame, "Type some text, then click \"Set status\":");

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

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl.as_widget_ref());
    sizer.add(b1.as_widget_ref());
    sizer.add(b2.as_widget_ref());
    sizer.add(b3.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
