//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `StatusBar` — 4 fields, 4 features exercised by buttons.
//!
//! Demonstrates the four main things you can do with a `StatusBar`:
//! 1. **Set field text** — write a distinct string into each of the
//!    4 fields via 4 dedicated buttons.
//! 2. **Get field text** — read back all 4 fields and show the
//!    result in a modal message box.
//! 3. **Get field count** — query `get_fields_count()` and show it.
//! 4. **Show / Hide** — toggle the visibility of the whole bar.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_status_bar
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    message_box, App, BoxSizer, Button, Frame, MessageBoxIcon, MessageBoxStyle, StatusBar,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StatusBar")
        .with_size(560, 360)
        .build();

    // 4 fields → "ready/field-name/coords/state" style layout.
    let status = StatusBar::new(&frame, 4);
    status.set_status_text("Ready", 0);
    status.set_status_text("Field 1", 1);
    status.set_status_text("Field 2", 2);
    status.set_status_text("Field 3", 3);

    // ── (1) Set field text ────────────────────────────────────────────
    // One button per field writes a recognisable payload into it.
    let s = status.clone();
    let btn_set0 = Button::new(&frame, "Set field 0 = \"ALPHA\"");
    btn_set0.on_click(&frame, move || s.set_status_text("ALPHA", 0));

    let s = status.clone();
    let btn_set1 = Button::new(&frame, "Set field 1 = \"BETA\"");
    btn_set1.on_click(&frame, move || s.set_status_text("BETA", 1));

    let s = status.clone();
    let btn_set2 = Button::new(&frame, "Set field 2 = \"GAMMA\"");
    btn_set2.on_click(&frame, move || s.set_status_text("GAMMA", 2));

    let s = status.clone();
    let btn_set3 = Button::new(&frame, "Set field 3 = \"DELTA\"");
    btn_set3.on_click(&frame, move || s.set_status_text("DELTA", 3));

    // ── (2) Get field text ────────────────────────────────────────────
    // Round-trip: read every field, format the values, show in a dialog.
    let s = status.clone();
    let frame_for_box = frame.clone();
    let btn_get = Button::new(&frame, "Read all 4 fields");
    btn_get.on_click(&frame, move || {
        let dump = format!(
            "Field 0 = {:?}\nField 1 = {:?}\nField 2 = {:?}\nField 3 = {:?}",
            s.get_status_text(0),
            s.get_status_text(1),
            s.get_status_text(2),
            s.get_status_text(3),
        );
        message_box(
            &frame_for_box,
            &dump,
            "StatusBar — read-back",
            MessageBoxStyle::Ok,
            MessageBoxIcon::Information,
        );
    });

    // ── (3) Get field count ──────────────────────────────────────────
    let s = status.clone();
    let frame_for_box = frame.clone();
    let btn_count = Button::new(&frame, "Show field count");
    btn_count.on_click(&frame, move || {
        message_box(
            &frame_for_box,
            &format!("StatusBar has {} field(s).", s.get_fields_count()),
            "StatusBar — field count",
            MessageBoxStyle::Ok,
            MessageBoxIcon::Information,
        );
    });

    // ── (4) Show / Hide toggle ────────────────────────────────────────
    // `StatusBar` has direct `is_visible` / `set_visible` shortcuts that
    // delegate to the underlying `Widget` trait. Clicking flips the
    // bar's visibility in place.
    let s = status.clone();
    let btn_toggle = Button::new(&frame, "Show / Hide status bar");
    btn_toggle.on_click(&frame, move || {
        s.set_visible(!s.is_visible());
    });

    // ── Layout ────────────────────────────────────────────────────────
    // A single vertical BoxSizer stacks the 7 buttons top-to-bottom.
    // (Sizers cannot be nested in `ru_wx` — only Widgets can sit inside
    // a sizer — so we keep the layout flat.)
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_set0.as_widget_ref());
    sizer.add(btn_set1.as_widget_ref());
    sizer.add(btn_set2.as_widget_ref());
    sizer.add(btn_set3.as_widget_ref());
    sizer.add(btn_get.as_widget_ref());
    sizer.add(btn_count.as_widget_ref());
    sizer.add(btn_toggle.as_widget_ref());

    frame.set_sizer(sizer);

    app.run(frame);
}
