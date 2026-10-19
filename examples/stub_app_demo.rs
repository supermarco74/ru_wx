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
    // Su Windows il backend stub (AppKit/GTK) non ha finestra nativa: prima
    // l'esempio usciva subito a vuoto (eprintln invisibile con subsystem
    // windows). Ora si usa l'API unificata sul backend Win32 reale, mostrando
    // gli stessi tipi del ramo non-Windows (direttiva §5).
    use ru_wx::{App, Button, Frame, Panel, StaticText};
    use std::cell::Cell;
    use std::rc::Rc;

    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx stub app (backend Win32)")
        .with_size(480, 320)
        .build();

    let panel = Panel::new(&frame);
    let label = StaticText::new(&panel, "Hello from Windows backend");
    label.set_label("Stessi tipi del demo stub — backend Win32 attivo");

    let clicked = Rc::new(Cell::new(false));
    let clicked_cb = Rc::clone(&clicked);
    let label_for_click = label.clone();
    let button = Button::new(&panel, "Click me");
    button.on_click(&frame, move || {
        clicked_cb.set(true);
        label_for_click.set_label("Click dispatch OK — WM_COMMAND gestita");
    });

    // Simula il click reale: sul backend Win32 il dispatch passa da
    // WM_COMMAND (la variante non-Windows usa `Frame::dispatch_command`).
    frame.post_raw_message(0x0111 /* WM_COMMAND */, button.id() as usize, 0);

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

    let _ = clicked; // usato dall'handler qui sopra
    app.run(frame);
}
