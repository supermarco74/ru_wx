//! Minitest: `TextCtrl` → `StatusBar` — type into a text field, click
//! a button, see the value appear in the status bar.
//!
//! This is a regression test for the bug where the `StatusBar` fields
//! were computed with the parent frame's client rect = 0×0 (because
//! the frame was not yet shown at construction time) and never
//! re-computed on `WM_SIZE`, so each field ended up a few pixels wide
//! and could only display one character. The fix added a resize
//! handler in [`StatusBar::new`] that re-applies the field widths on
//! every `WM_SIZE`.
//!
//! Demonstrates:
//! - `TextCtrl::get_value` round-tripping into `StatusBar::set_status_text`
//! - A long, deliberately wide payload that would visibly truncate
//!   if the field-width fix were missing
//! - Pre-baked sample buttons so the test can be exercised without
//!   having to type into the input field
//! - 4 fields side by side so each field has a clear width target
//!
//! Run with:
//! ```bash
//! cargo run --example mt_status_bar_input
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StaticText, StatusBar, TextCtrl};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TextCtrl → StatusBar")
        .with_size(1000, 600)
        .build();

    // 4 fields so each one gets ≈ 1/4 of the bar. Field 0 is the
    // "main" one for the text-input round-trip; fields 1–3 are
    // pre-populated with reference labels so the visual width of
    // each field is obvious.
    let status = StatusBar::new(&frame, 4);
    status.set_status_text("(empty)", 0);
    status.set_status_text("Field 1 — fixed label", 1);
    status.set_status_text("Field 2 — fixed label", 2);
    status.set_status_text("Field 3 — fixed label", 3);

    // ── Section 1: free-form input ────────────────────────────────────
    let lbl_input = StaticText::new(&frame, "Type some text, then click \"Set status\":");
    let input = TextCtrl::new(&frame, "Hello, world!");

    // ── Section 2: presets (long payloads) ───────────────────────────
    // These buttons write deliberately long strings so the test
    // visibly fails if the field-width fix regresses — a long string
    // would be clipped to one character without the fix.
    let s_for_btn = status.clone();
    let btn_send_input = Button::new(&frame, "Set status ← input box");
    let input_for_btn = input.clone();
    btn_send_input.on_click(&frame, move || {
        let value = input_for_btn.get_value();
        s_for_btn.set_status_text(&value, 0);
    });

    let s = status.clone();
    let btn_long = Button::new(
        &frame,
        "Preset: long string (80+ chars, no truncation)",
    );
    btn_long.on_click(&frame, move || {
        s.set_status_text(
            "The quick brown fox jumps over the lazy dog (1234567890)",
            0,
        );
    });

    let s = status.clone();
    let btn_short = Button::new(&frame, "Preset: short (\"OK\")");
    btn_short.on_click(&frame, move || {
        s.set_status_text("OK", 0);
    });

    let s = status.clone();
    let btn_clear = Button::new(&frame, "Clear status field 0");
    btn_clear.on_click(&frame, move || {
        s.set_status_text("", 0);
    });

    // ── Layout ────────────────────────────────────────────────────────
    // The 22 px spacer at the end of the sizer reserves room for the
    // status bar at the bottom of the client area. The status bar is
    // *not* part of this sizer (it is positioned by its own resize
    // handler via MoveWindow), so without the spacer the sizer would
    // lay out the last button on top of the status bar.
    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl_input.as_widget_ref());
    sizer.add(input.as_widget_ref());
    sizer.add(btn_send_input.as_widget_ref());
    sizer.add(btn_long.as_widget_ref());
    sizer.add(btn_short.as_widget_ref());
    sizer.add(btn_clear.as_widget_ref());
    sizer.add_spacer(22);
    frame.set_sizer(sizer);

    app.run(frame);
}
