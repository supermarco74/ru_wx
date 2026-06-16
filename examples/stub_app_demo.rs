//! Unified `ru_wx` API on the AppKit / GTK stub backend (non-Windows).
//!
//! Uses `App`, `Frame`, `Panel`, `Button`, and `StaticText` — the same types as
//! on Windows, backed by the in-memory placeholder instead of Win32.

#[cfg(not(target_os = "windows"))]
fn main() {
    use ru_wx::{App, Button, Frame, Panel, StaticText};
    use std::cell::Cell;
    use std::rc::Rc;

    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx stub app")
        .with_size(480, 320)
        .build();

    let panel = Panel::new(&frame);
    let label = StaticText::new(&panel, "Hello from stub backend");
    label.set_label("Panel + StaticText OK");

    let clicked = Rc::new(Cell::new(false));
    let clicked_cb = Rc::clone(&clicked);
    let button = Button::new(&panel, "Click me");
    button.on_click(&frame, move || clicked_cb.set(true));

    assert!(
        frame.dispatch_command(button.id()),
        "button command should dispatch through the frame"
    );
    assert!(clicked.get());

    let frame_for_close = frame.clone();
    frame.on_idle(move |_| frame_for_close.request_close());

    app.run(frame);
    println!("stub_app_demo OK — backend active on this target");
}

#[cfg(target_os = "windows")]
fn main() {
    eprintln!(
        "stub_app_demo targets the AppKit/GTK stub backend (non-Windows).\n\
         On Windows use `window_with_button` or `cross_platform_stubs`."
    );
}
