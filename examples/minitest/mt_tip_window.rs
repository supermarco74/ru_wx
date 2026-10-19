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
//! - Showing a tip *series* with `show_tip`/`TipProvider` (C8): the
//!   popup anchors below the calling window, wraps the text and uses
//!   Enter/Space for the next tip, Esc (or a click) to close.
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

use ru_wx::{show_tip, App, BoxSizer, Button, Frame, Rect, StatusBar, TipProvider, TipWindow};

thread_local! {
    static TIP: RefCell<Option<TipWindow>> = const { RefCell::new(None) };
}

/// Serie di suggerimenti erogati dalla finestra tip (C8).
struct Tips {
    items: Vec<String>,
    idx: usize,
}

impl TipProvider for Tips {
    fn next_tip(&mut self, max_length: usize) -> Option<String> {
        let tip = self.items.get(self.idx)?.clone();
        self.idx += 1;
        Some(tip.chars().take(max_length).collect())
    }
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TipWindow")
        .with_size(540, 360)
        .build();

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

    // Tip series button (C8): opens the modal tip window anchored
    // below the frame and reports whether at least one tip was read.
    let status_for_series = status.clone();
    let frame_for_series = frame.clone();
    let btn_series = Button::new(&frame, "Show tips series (TipProvider)");
    btn_series.on_click(&frame, move || {
        let mut tips = Tips {
            items: vec![
                "Press Enter or Space for the next tip.".into(),
                "Long tips wrap inside the popup: this sentence is deliberately long so the window has to break it across several lines without clipping the text.".into(),
                "Press Esc (or click) to close the tip window.".into(),
            ],
            idx: 0,
        };
        let read = show_tip(&frame_for_series, "Tip series", &mut tips);
        status_for_series.set_status_text(
            if read {
                "Tip series: at least one tip read"
            } else {
                "Tip series: nothing read"
            },
            0,
        );
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_show.as_widget_ref());
    sizer.add(btn_update.as_widget_ref());
    sizer.add(btn_close.as_widget_ref());
    sizer.add(btn_series.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
