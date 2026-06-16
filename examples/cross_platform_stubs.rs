//! Cross-platform stub demo — AppKit (macOS) or GTK (Linux/others) placeholders.
//!
//! Exercises the five stub types without a native windowing system:
//! `App` / `Frame` / `Panel` / `Button` / `StaticText`.

use std::cell::Cell;
use std::rc::Rc;

#[cfg(target_os = "macos")]
use ru_wx::{
    AppKitApp, AppKitButton, AppKitFrame, AppKitPanel, AppKitStaticText, StubBackend,
};

#[cfg(not(target_os = "macos"))]
use ru_wx::{GtkApp, GtkButton, GtkFrame, GtkPanel, GtkStaticText, StubBackend};

fn main() {
    #[cfg(target_os = "macos")]
    run_appkit_demo();

    #[cfg(not(target_os = "macos"))]
    run_gtk_demo();
}

#[cfg(target_os = "macos")]
fn run_appkit_demo() {
    let mut app = AppKitApp::new();
    assert_eq!(app.backend(), StubBackend::AppKit);

    let frame = AppKitFrame::with_size("ru_wx AppKit stub", 640, 480);
    frame.show();

    let _panel = AppKitPanel::new(&frame);
    let label = AppKitStaticText::new(&frame, "Hello from AppKit stub");
    label.set_label("AppKit / ru_wx");

    let clicked = Rc::new(Cell::new(false));
    let clicked_cb = Rc::clone(&clicked);
    frame.register_command_handler(1, Box::new(move || clicked_cb.set(true)));
    let button = AppKitButton::new(&frame, 1, "Click me");
    assert!(button.simulate_click(&frame));
    assert!(clicked.get(), "button should dispatch through the frame");

    app.run();
    println!(
        "AppKit stub demo OK — title={:?}, label={:?}, button={:?}",
        frame.title(),
        label.label(),
        button.label()
    );
}

#[cfg(not(target_os = "macos"))]
fn run_gtk_demo() {
    let mut app = GtkApp::new();
    assert_eq!(app.backend(), StubBackend::Gtk);

    let frame = GtkFrame::with_size("ru_wx GTK stub", 640, 480);
    frame.show();

    let _panel = GtkPanel::new(&frame);
    let label = GtkStaticText::new(&frame, "Hello from GTK stub");
    label.set_label("GTK / ru_wx");

    let clicked = Rc::new(Cell::new(false));
    let clicked_cb = Rc::clone(&clicked);
    frame.register_command_handler(1, Box::new(move || clicked_cb.set(true)));
    let button = GtkButton::new(&frame, 1, "Click me");
    assert!(button.simulate_click(&frame));
    assert!(clicked.get(), "button should dispatch through the frame");

    app.run();
    println!(
        "GTK stub demo OK — title={:?}, label={:?}, button={:?}",
        frame.title(),
        label.label(),
        button.label()
    );
}
