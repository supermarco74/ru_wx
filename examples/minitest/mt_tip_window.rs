//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `TipWindow` — a transient non-activating popup hint
//! (`wxTipWindow`).
//!
//! Demonstrates:
//! - Creating a `TipWindow` anchored to a screen rect.
//! - Reading / replacing the tip text.
//! - Closing the tip explicitly.
//!
//! The tip is short-lived by design (it auto-closes when the user
//! clicks elsewhere) so this example is interactive: click the
//! button to show the tip, then move the mouse and click outside
//! to dismiss it.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_tip_window
//! ```

#![windows_subsystem = "windows"]

use std::cell::RefCell;

use ru_wx::{App, BoxSizer, Button, Frame, Rect, StatusBar, TipWindow};

thread_local! {
    static TIP: RefCell<Option<TipWindow>> = const { RefCell::new(None) };
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TipWindow")
        .with_size(540, 360)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click a button to show a tip popup.", 0);

    // Anchor the tip near the top-left corner of the main frame's
    // client area (a representative rect; the TipWindow pops up at
    // screen coordinates).
    let anchor = Rect {
        x: 80,
        y: 60,
        width: 180,
        height: 32,
    };

    // Show button
    let status_for_show = status.clone();
    let frame_for_show = frame.clone();
    let btn_show = Button::new(&frame, "Show tip 'Hello, world!'");
    btn_show.on_click(&frame, move || {
        let tip = TipWindow::new(&frame_for_show, anchor, "Hello, world!");
        status_for_show.set_status_text(&format!("Tip shown; text = {:?}", tip.text()), 0);
        // Hold the tip in a thread-local slot so it isn't dropped
        // (and thus destroyed) immediately. In a real app you'd
        // store the `TipWindow` in app state and drop / `close()`
        // it when appropriate.
        TIP.with(|cell| cell.borrow_mut().replace(tip));
    });

    // Update text button
    let btn_update = Button::new(&frame, "Update tip text");
    btn_update.on_click(&frame, move || {
        TIP.with(|cell| {
            if let Some(tip) = cell.borrow_mut().as_mut() {
                tip.set_text("Updated hint!");
            }
        });
    });

    // Close button
    let status_for_close = status.clone();
    let btn_close = Button::new(&frame, "Close tip");
    btn_close.on_click(&frame, move || {
        TIP.with(|cell| {
            if let Some(tip) = cell.borrow_mut().take() {
                tip.close();
                status_for_close.set_status_text("Tip closed", 0);
            } else {
                status_for_close.set_status_text("(no tip to close)", 0);
            }
        });
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_show.as_widget_ref());
    sizer.add(btn_update.as_widget_ref());
    sizer.add(btn_close.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}


