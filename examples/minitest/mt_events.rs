//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, FocusEvent, Frame, MouseEventKind, SizeEvent, StaticText, StatusBar,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — input events")
        .with_size(480, 240)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Press a key or click the client area.", 0);
    let _hint = StaticText::new(&frame, "Key / mouse / focus / size / move events:");
    let s = status.clone();
    frame.on_key_down(move |ev| {
        s.set_status_text(&format!("Key down: {}", ev.key_code), 0);
    });
    let s2 = status.clone();
    frame.on_mouse(move |ev| {
        if ev.kind == MouseEventKind::LeftDown {
            s2.set_status_text(
                &format!("Click at {}, {}", ev.position.x, ev.position.y),
                0,
            );
        }
    });
    let s3 = status.clone();
    frame.on_focus(move |ev: &FocusEvent| {
        let msg = if ev.gained { "Focus gained" } else { "Focus lost" };
        s3.set_status_text(msg, 0);
    });
    let s4 = status.clone();
    frame.on_size_event(move |ev: &SizeEvent| {
        s4.set_status_text(
            &format!("Size {}×{}", ev.size.width, ev.size.height),
            0,
        );
    });
    frame.set_sizer(BoxSizer::vertical());
    app.run(frame);
}
