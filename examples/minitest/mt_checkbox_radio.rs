//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `CheckBox`, `RadioBox` and `RadioButton` — selection controls.
//!
//! Demonstrates:
//! - A "Select all" master checkbox driving three feature checkboxes
//!   (`set_checked` / `is_checked` / `on_toggle`)
//! - A live summary of all checkbox states in the `StatusBar`
//! - `RadioBox` selection rewriting a `StaticText` label
//! - A `RadioButton` group reported in a second status field
//! - Tooltips on the master checkbox
//!
//! Run with:
//! ```bash
//! cargo run --example mt_checkbox_radio
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, CheckBox, Frame, RadioBox, RadioButton, StaticText, StatusBar, ToolTip,
};

const FEATURES: [&str; 3] = ["Autosave", "Spell check", "Dark mode"];
const PRIORITIES: [&str; 4] = ["Low", "Normal", "High", "Urgent"];

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — CheckBox / RadioBox / RadioButton")
        .with_size(480, 460)
        .build();

    // Field 0 = checkbox summary, field 1 = radio button group.
    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Toggle any control.", 1);

    // ── CheckBoxes with live summary ────────────────────────────────
    let lbl_chk = StaticText::new(&frame, "Features (live summary in status bar):");

    let chk_all = CheckBox::new(&frame, "Select all");
    ToolTip::new("Checks / unchecks every feature below").attach(&chk_all.as_widget_ref());

    let chk_auto = CheckBox::new(&frame, FEATURES[0]);
    let chk_spell = CheckBox::new(&frame, FEATURES[1]);
    let chk_dark = CheckBox::new(&frame, FEATURES[2]);
    chk_auto.set_checked(true);

    let boxes = [chk_auto.clone(), chk_spell.clone(), chk_dark.clone()];

    // Rebuild the "enabled: ..." summary from the real checkbox states.
    let summary = {
        let boxes = boxes.clone();
        let status = status.clone();
        move || {
            let on: Vec<&str> = FEATURES
                .iter()
                .zip(boxes.iter())
                .filter(|(_, b)| b.is_checked())
                .map(|(name, _)| *name)
                .collect();
            let text = if on.is_empty() {
                "Enabled: (none)".to_string()
            } else {
                format!("Enabled: {} ({}/{})", on.join(", "), on.len(), FEATURES.len())
            };
            status.set_status_text(&text, 0);
        }
    };
    summary(); // seed the initial state

    // Each feature checkbox refreshes the summary and keeps the
    // master "Select all" in sync with reality.
    for chk in &boxes {
        let summary = summary.clone();
        let boxes = boxes.clone();
        let master = chk_all.clone();
        chk.on_toggle(&frame, move || {
            master.set_checked(boxes.iter().all(|b| b.is_checked()));
            summary();
        });
    }

    // Master checkbox: propagate to the three feature checkboxes.
    // (`set_checked` does not re-fire `on_toggle`, so refresh manually.)
    {
        let master = chk_all.clone();
        let boxes = boxes.clone();
        let summary = summary.clone();
        chk_all.on_toggle(&frame, move || {
            let v = master.is_checked();
            for b in &boxes {
                b.set_checked(v);
            }
            summary();
        });
    }

    // ── RadioBox → StaticText ───────────────────────────────────────
    let lbl_rb = StaticText::new(&frame, "RadioBox (grouped):");
    let radio = RadioBox::new(&frame, "Priority", &PRIORITIES);
    radio.set_selection(1);
    let priority_label = StaticText::new(&frame, "Current priority: Normal");
    {
        let target = priority_label.clone();
        radio.on_select(&frame, move |idx| {
            let label = PRIORITIES.get(idx).copied().unwrap_or("?");
            target.set_label(&format!("Current priority: {label}"));
        });
    }

    // ── RadioButton group → status field 1 ──────────────────────────
    let lbl_rbtn = StaticText::new(&frame, "RadioButton group (delivery):");
    let rb1 = RadioButton::new(&frame, "Standard", true); // is_group_start = true
    let rb2 = RadioButton::new(&frame, "Express", false);
    let rb3 = RadioButton::new(&frame, "Overnight", false);
    rb1.set_selected(true);
    for (rb, name) in [(&rb1, "Standard"), (&rb2, "Express"), (&rb3, "Overnight")] {
        let s = status.clone();
        rb.on_select(&frame, move || {
            s.set_status_text(&format!("Delivery: {name}"), 1);
        });
    }

    // Layout: radio buttons on one nested horizontal row.
    let mut rb_row = BoxSizer::horizontal();
    rb_row.add(rb1.as_widget_ref());
    rb_row.add(rb2.as_widget_ref());
    rb_row.add(rb3.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl_chk.as_widget_ref());
    sizer.add(chk_all.as_widget_ref());
    sizer.add(chk_auto.as_widget_ref());
    sizer.add(chk_spell.as_widget_ref());
    sizer.add(chk_dark.as_widget_ref());
    sizer.add(lbl_rb.as_widget_ref());
    sizer.add(radio.as_widget_ref());
    sizer.add(priority_label.as_widget_ref());
    sizer.add(lbl_rbtn.as_widget_ref());
    sizer.add_sizer(rb_row);
    frame.set_sizer(sizer);

    app.run(frame);
}
