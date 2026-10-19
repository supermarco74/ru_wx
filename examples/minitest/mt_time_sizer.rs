//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: TimePickerCtrl + StdDialogButtonSizer (fase F1).
//!
//! Demonstrates:
//! 1. [`TimePickerCtrl`] (`new` = HH:mm:ss, `new_hm` = HH:mm) with
//!    `on_time_change` keeping a live label + `StatusBar` in sync.
//! 2. [`StdDialogButtonSizer`] arranging the standard dialog
//!    buttons in the per-OS order (Windows: `[OK] [Cancel]`,
//!    macOS: `[Cancel] [OK]`), nested into the frame's `BoxSizer`.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_time_sizer
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Button, Frame, StaticText, StatusBar, StdDialogButton, StdDialogButtonSizer,
    Time, TimePickerCtrl,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — time picker + std dialog button sizer")
        .with_size(520, 240)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Change a time…", 0);

    // ── Time pickers: HH:mm:ss + HH:mm ────────────────────────────
    let time_label = StaticText::new(&frame, "Time (HH:mm:ss):");
    let time_picker = TimePickerCtrl::new(&frame);
    time_picker.set_value(Some(Time::new(14, 30, 0)));
    let time_value = StaticText::new(&frame, "14:30:00");

    let hm_label = StaticText::new(&frame, "Hour/minute:");
    let time_hm = TimePickerCtrl::new_hm(&frame);
    time_hm.set_value(Some(Time::new(9, 5, 0)));
    let hm_value = StaticText::new(&frame, "09:05");

    let value_for_time = time_value.clone();
    let status_for_time = status.clone();
    time_picker.on_time_change(&frame, move |t: Option<Time>| {
        let text = match t {
            Some(t) => format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second),
            None => String::from("(no time)"),
        };
        value_for_time.set_label(&text);
        status_for_time.set_status_text(&format!("Time changed: {text}"), 0);
    });

    let hm_for_value = hm_value.clone();
    time_hm.on_time_change(&frame, move |t: Option<Time>| {
        let text = match t {
            Some(t) => format!("{:02}:{:02}", t.hour, t.minute),
            None => String::from("(no time)"),
        };
        hm_for_value.set_label(&text);
    });

    let mut row_time = BoxSizer::horizontal();
    row_time.add(time_label.as_widget_ref());
    row_time.add(time_picker.as_widget_ref());
    row_time.add_spacer(8);
    row_time.add(time_value.as_widget_ref());

    let mut row_hm = BoxSizer::horizontal();
    row_hm.add(hm_label.as_widget_ref());
    row_hm.add(time_hm.as_widget_ref());
    row_hm.add_spacer(8);
    row_hm.add(hm_value.as_widget_ref());

    // ── Standard dialog buttons, ordered per-OS ───────────────────
    // Registered deliberately "out of order": `realize` applies the
    // platform convention (affirmative left on Windows, negative
    // left on macOS) while keeping the insertion order inside each
    // group.
    let btn_ok = Button::new(&frame, "OK");
    let btn_cancel = Button::new(&frame, "Cancel");
    let btn_help = Button::new(&frame, "Help");
    let mut buttons = StdDialogButtonSizer::new();
    buttons.add_button(&btn_help, StdDialogButton::Help);
    buttons.add_button(&btn_cancel, StdDialogButton::Cancel);
    buttons.add_button(&btn_ok, StdDialogButton::Ok);

    // ── Layout ────────────────────────────────────────────────────
    let header = StaticText::new(&frame, "TimePickerCtrl (C1) + StdDialogButtonSizer (C3):");
    let mut sizer = BoxSizer::vertical();
    sizer.add(header.as_widget_ref());
    sizer.add_spacer(4);
    sizer.add_sizer(row_time);
    sizer.add_sizer(row_hm);
    sizer.add_spacer(12);
    sizer.add_sizer(buttons.realize());
    frame.set_sizer(sizer);

    app.run(frame);
}
