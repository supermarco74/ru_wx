//! Minitest: `MiniFrame` — a small caption frame (`wxMiniFrame`).
//!
//! Demonstrates:
//! - Creating a `MiniFrame` parented to the main frame.
//! - Showing / hiding it on demand.
//! - Setting / reading its title.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_mini_frame
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{App, BoxSizer, Button, Frame, MiniFrame, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — MiniFrame")
        .with_size(540, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Use the buttons to control the mini-frame.", 0);

    // The mini-frame: a small caption frame (WS_EX_TOOLWINDOW) parented
    // to the main frame. It can hold its own widgets; for this minitest
    // we just demonstrate show / hide / set-title which is the core
    // public API of `MiniFrame`.
    let mini = MiniFrame::new(&frame, "Mini-Frame", 280, 160);

    // Track the current visibility state so the toggle button can
    // actually flip it. We start as `true` to match the
    // `mini.show(true)` at the bottom of `main`.
    let visible: Rc<Cell<bool>> = Rc::new(Cell::new(true));

    // Show / Hide toggle
    let mini_for_toggle = mini.clone();
    let btn_toggle = Button::new(&frame, "Show / Hide mini-frame");
    let status_for_toggle = status.clone();
    let visible_for_toggle = visible.clone();
    btn_toggle.on_click(&frame, move || {
        let new_state = !visible_for_toggle.get();
        visible_for_toggle.set(new_state);
        mini_for_toggle.show(new_state);
        status_for_toggle.set_status_text(
            if new_state {
                "MiniFrame shown"
            } else {
                "MiniFrame hidden"
            },
            0,
        );
    });

    // Hide explicitly
    let mini_for_hide = mini.clone();
    let btn_hide = Button::new(&frame, "Hide mini-frame");
    btn_hide.on_click(&frame, move || {
        mini_for_hide.show(false);
    });

    // Set a new title
    let mini_for_rename = mini.clone();
    let btn_rename = Button::new(&frame, "Rename mini-frame");
    btn_rename.on_click(&frame, move || {
        mini_for_rename.set_title("Renamed!");
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_toggle.as_widget_ref());
    sizer.add(btn_hide.as_widget_ref());
    sizer.add(btn_rename.as_widget_ref());
    frame.set_sizer(sizer);

    // Make the mini-frame visible initially.
    mini.show(true);

    app.run(frame);
}
