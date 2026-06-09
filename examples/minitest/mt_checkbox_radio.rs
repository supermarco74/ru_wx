//! Minitest: `CheckBox`, `RadioBox` and `RadioButton` — selection controls.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_checkbox_radio
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, CheckBox, Frame, RadioBox, RadioButton, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — CheckBox / RadioBox / RadioButton")
        .with_size(460, 380)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Toggle any control.", 0);

    // ── CheckBox ────────────────────────────────────────────────────
    let lbl_chk = StaticText::new(&frame, "CheckBox:");
    let chk = CheckBox::new(&frame, "Enable feature X");
    chk.set_checked(true);
    let chk_clone = chk.clone();
    let s = status.clone();
    chk.on_toggle(&frame, move || {
        let v = chk_clone.is_checked();
        s.set_status_text(&format!("CheckBox: feature X = {v}"), 0);
    });

    // ── RadioBox ────────────────────────────────────────────────────
    let lbl_rb = StaticText::new(&frame, "RadioBox (grouped):");
    let radio = RadioBox::new(&frame, "Priority", &["Low", "Normal", "High", "Urgent"]);
    radio.set_selection(1);
    let s = status.clone();
    radio.on_select(&frame, move |idx| {
        let label = ["Low", "Normal", "High", "Urgent"]
            .get(idx)
            .copied()
            .unwrap_or("?");
        s.set_status_text(&format!("RadioBox: {label}"), 0);
    });

    // ── RadioButton group ───────────────────────────────────────────
    let lbl_rbtn = StaticText::new(&frame, "RadioButton group:");
    let rb1 = RadioButton::new(&frame, "Option A", true); // is_group_start = true
    let rb2 = RadioButton::new(&frame, "Option B", false);
    let rb3 = RadioButton::new(&frame, "Option C", false);
    let s = status.clone();
    rb1.on_select(&frame, move || {
        s.set_status_text("RadioButton: Option A", 0)
    });
    let s = status.clone();
    rb2.on_select(&frame, move || {
        s.set_status_text("RadioButton: Option B", 0)
    });
    let s = status.clone();
    rb3.on_select(&frame, move || {
        s.set_status_text("RadioButton: Option C", 0)
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl_chk.as_widget_ref());
    sizer.add(chk.as_widget_ref());
    sizer.add(lbl_rb.as_widget_ref());
    sizer.add(radio.as_widget_ref());
    sizer.add(lbl_rbtn.as_widget_ref());
    sizer.add(rb1.as_widget_ref());
    sizer.add(rb2.as_widget_ref());
    sizer.add(rb3.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
