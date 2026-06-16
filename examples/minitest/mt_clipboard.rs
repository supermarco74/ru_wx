//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Clipboard, Frame, StatusBar, TextCtrl};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Clipboard")
        .with_size(480, 240)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Copy / paste via wxClipboard.", 0);
    let input = TextCtrl::new(&frame, "Hello clipboard");
    let btn_copy = Button::new(&frame, "Copy to clipboard");
    let btn_paste = Button::new(&frame, "Paste from clipboard");
    let input_paste = input.clone();
    let s_copy = status.clone();
    btn_copy.on_click(&frame, move || {
        let text = input_paste.get_value();
        let ok = Clipboard::set_text(&text);
        s_copy.set_status_text(
            if ok { "Copied." } else { "Copy failed." },
            0,
        );
    });
    let input_paste = input.clone();
    let s_paste = status.clone();
    btn_paste.on_click(&frame, move || {
        if let Some(text) = Clipboard::get_text() {
            input_paste.set_value(&text);
            s_paste.set_status_text("Pasted.", 0);
        } else {
            s_paste.set_status_text("Clipboard empty or unavailable.", 0);
        }
    });
    let mut sizer = BoxSizer::vertical();
    sizer.add(input.as_widget_ref());
    sizer.add(btn_copy.as_widget_ref());
    sizer.add(btn_paste.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
