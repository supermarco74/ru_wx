//! Cross-platform stub demo — AppKit (macOS) or GTK (Linux/others) placeholders.
//!
//! Exercises the five stub types without a native windowing system:
//! `App` / `Frame` / `Panel` / `Button` / `StaticText`.

use std::cell::Cell;
use std::rc::Rc;

#[cfg(target_os = "macos")]
use ru_wx::{AppKitApp, AppKitButton, AppKitFrame, AppKitPanel, AppKitStaticText, StubBackend};

#[cfg(not(target_os = "macos"))]
use ru_wx::{GtkApp, GtkButton, GtkFrame, GtkPanel, GtkStaticText, StubBackend};

fn main() {
    #[cfg(target_os = "macos")]
    run_appkit_demo();

    #[cfg(not(target_os = "macos"))]
    run_gtk_demo();

    // Su Windows il demo stub è headless (nessuna finestra nativa): si mostra
    // comunque una finestra Win32 reale con l'esito, così l'esempio resta
    // eseguibile e visibile anche su Windows (direttiva §5).
    #[cfg(target_os = "windows")]
    show_result_window();
}

/// Finestra Win32 di riepilogo (solo Windows): riporta l'esito del demo stub,
/// che altrimenti terminerebbe immediatamente senza finestra da catturare.
#[cfg(target_os = "windows")]
fn show_result_window() {
    use ru_wx::{App, Frame, Panel, StaticText};

    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx cross-platform stubs — esito demo")
        .with_size(640, 320)
        .build();

    let panel = Panel::new(&frame);
    let label = StaticText::new(&panel, "Demo stub completato");
    label.set_label("GTK stub OK — asserzioni superate, backend Win32 attivo");

    // Resta visibile per qualche secondo prima di chiudersi da sola.
    // (Su Win32 `request_close` non esiste: si manda WM_CLOSE.)
    let ticks = Rc::new(Cell::new(0u32));
    let frame_for_close = frame.clone();
    let ticks_for_close = Rc::clone(&ticks);
    frame.on_idle(move |_| {
        let t = ticks_for_close.get() + 1;
        ticks_for_close.set(t);
        if t >= 300 {
            frame_for_close.post_raw_message(0x0010 /* WM_CLOSE */, 0, 0);
        }
    });

    app.run(frame);
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
