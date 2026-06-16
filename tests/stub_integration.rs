//! Cross-platform stub integration tests (AppKit / GTK placeholder backends).
//!
//! Runs on Linux/macOS CI and on Windows (GTK path on non-macOS hosts).

use std::cell::Cell;
use std::rc::Rc;

#[cfg(not(target_os = "windows"))]
#[test]
fn unified_frame_button_command_dispatch() {
    use ru_wx::{Button, Frame};

    let frame = Frame::builder().with_title("stub-integration").build();
    let btn = Button::new(&frame, "OK");
    let clicked = Rc::new(Cell::new(false));
    let clicked_cb = Rc::clone(&clicked);
    btn.on_click(&frame, move || clicked_cb.set(true));
    assert!(frame.dispatch_command(btn.id()));
    assert!(clicked.get());
}

#[test]
fn stub_backend_direct_api() {
    use ru_wx::{
        GtkApp, GtkButton, GtkFrame, GtkPanel, GtkStaticText, StubBackend,
    };

    #[cfg(target_os = "macos")]
    use ru_wx::{
        AppKitApp, AppKitButton, AppKitFrame, AppKitPanel, AppKitStaticText,
    };

    #[cfg(target_os = "macos")]
    {
        let mut app = AppKitApp::new();
        assert_eq!(app.backend(), StubBackend::AppKit);
        let frame = AppKitFrame::with_size("test", 400, 300);
        frame.show();
        let _panel = AppKitPanel::new(&frame);
        let label = AppKitStaticText::new(&frame, "hi");
        label.set_label("appkit");
        let clicked = Rc::new(Cell::new(false));
        let cb = Rc::clone(&clicked);
        frame.register_command_handler(1, Box::new(move || cb.set(true)));
        let btn = AppKitButton::new(&frame, 1, "Go");
        assert!(btn.simulate_click(&frame));
        assert!(clicked.get());
        app.run();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let mut app = GtkApp::new();
        assert_eq!(app.backend(), StubBackend::Gtk);
        let frame = GtkFrame::with_size("test", 400, 300);
        frame.show();
        let _panel = GtkPanel::new(&frame);
        let label = GtkStaticText::new(&frame, "hi");
        label.set_label("gtk");
        let clicked = Rc::new(Cell::new(false));
        let cb = Rc::clone(&clicked);
        frame.register_command_handler(1, Box::new(move || cb.set(true)));
        let btn = GtkButton::new(&frame, 1, "Go");
        assert!(btn.simulate_click(&frame));
        assert!(clicked.get());
        app.run();
    }
}
