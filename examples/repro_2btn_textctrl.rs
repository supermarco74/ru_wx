//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Test: 2 buttons + TextCtrl + status bar
#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StatusBar, TextCtrl};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Repro 7 — 2 buttons + TextCtrl")
        .with_size(1000, 600)
        .build();

    let status = StatusBar::new(&frame, 4);
    status.set_status_text("(empty)", 0);

    let input = TextCtrl::new(&frame, "Hello, world!");

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

    let mut sizer = BoxSizer::vertical();
    sizer.add(input.as_widget_ref());
    sizer.add(b1.as_widget_ref());
    sizer.add(b2.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
