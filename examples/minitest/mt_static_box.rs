//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `StaticBox` — labelled group boxes around *working*
//! controls.
//!
//! Win32 `BS_GROUPBOX` semantics: the box is purely decorative and the
//! grouped controls are **siblings** of the box (children of the same
//! frame), positioned over it. That keeps their `WM_COMMAND` routing
//! to the frame intact, so every radio / checkbox callback fires.
//!
//! Demonstrates:
//! - "Theme" box: a 3-way `RadioButton` group whose selection updates
//!   a preview label and the StatusBar
//! - "Options" box: three `CheckBox`es feeding a live summary label
//! - "Result" box framing the two live labels
//! - `set_label` / `get_label` round-trip via a rename button
//! - Constructors: `new`, `with_size`, `new_empty` + late `set_label`
//!
//! Run with:
//! ```bash
//! cargo run --example mt_static_box
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Frame, RadioButton, StaticBox, StaticText, StatusBar,
    WidgetRef,
};

/// Absolutely place a widget (the grouped controls do not live in the
/// frame sizer — they sit on top of their decorative group box).
fn place(widget: &WidgetRef, x: i32, y: i32, w: u32, h: u32) {
    let mut b = widget.borrow_mut();
    b.set_size(w, h);
    b.set_position(x, y);
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StaticBox (live groups)")
        .with_size(560, 440)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Theme: Light — 0 options enabled", 0);

    let hint = StaticText::new(&frame, "Toggle the controls — the Result box updates live.");

    // ── Boxes first (created before their sibling controls so the
    //    controls stay above them in the z-order) ─────────────────────────
    let box_theme = StaticBox::with_size(&frame, "Theme", 250, 150);
    place(&box_theme.as_widget_ref(), 10, 35, 250, 150);

    let box_options = StaticBox::new(&frame, "Options");
    place(&box_options.as_widget_ref(), 275, 35, 250, 150);

    // `new_empty` + late label: the round-trip is shown live in the
    // Result box title.
    let box_result = StaticBox::new_empty(&frame);
    assert_eq!(box_result.get_label(), "");
    box_result.set_label("Result");
    place(&box_result.as_widget_ref(), 10, 195, 515, 90);

    // ── Theme radio group (siblings of the box) ──────────────────────────
    let lbl_preview = StaticText::new(&frame, "Theme: Light");
    place(&lbl_preview.as_widget_ref(), 30, 220, 470, 22);
    let lbl_summary = StaticText::new(&frame, "Options: (none)");
    place(&lbl_summary.as_widget_ref(), 30, 248, 470, 22);

    let themes = ["Light", "Dark", "System"];
    for (i, name) in themes.iter().enumerate() {
        let radio = RadioButton::new(&frame, name, i == 0);
        place(&radio.as_widget_ref(), 30, 65 + i as i32 * 28, 200, 24);
        if i == 0 {
            radio.set_selected(true);
        }
        let lbl = lbl_preview.clone();
        let s = status.clone();
        let name = name.to_string();
        radio.on_select(&frame, move || {
            lbl.set_label(&format!("Theme: {name}"));
            s.set_status_text(&format!("Theme changed to {name}"), 0);
        });
    }

    // ── Options checkboxes (siblings of the box) ─────────────────────────
    let options = ["Autosave", "Line numbers", "Word wrap"];
    let mut checks = Vec::new();
    for (i, name) in options.iter().enumerate() {
        let check = CheckBox::new(&frame, name);
        place(&check.as_widget_ref(), 295, 65 + i as i32 * 28, 200, 24);
        checks.push(check);
    }
    let checks = Rc::new(checks);
    for check in checks.iter() {
        let checks = checks.clone();
        let lbl = lbl_summary.clone();
        let s = status.clone();
        check.on_toggle(&frame, move || {
            let enabled: Vec<String> = checks
                .iter()
                .filter(|c| c.is_checked())
                .map(|c| c.get_label())
                .collect();
            if enabled.is_empty() {
                lbl.set_label("Options: (none)");
            } else {
                lbl.set_label(&format!("Options: {}", enabled.join(", ")));
            }
            s.set_status_text(&format!("{} option(s) enabled", enabled.len()), 0);
        });
    }

    // ── Rename button: set_label / get_label round-trip ──────────────────
    let btn_rename = Button::new(&frame, "Rename groups");
    place(&btn_rename.as_widget_ref(), 10, 300, 140, 30);
    let b1 = box_theme.clone();
    let b2 = box_options.clone();
    let s = status.clone();
    btn_rename.on_click(&frame, move || {
        let renamed = b1.get_label().ends_with('*');
        if renamed {
            b1.set_label("Theme");
            b2.set_label("Options");
        } else {
            b1.set_label("Theme*");
            b2.set_label("Options*");
        }
        s.set_status_text(
            &format!("Boxes renamed to '{}' / '{}'", b1.get_label(), b2.get_label()),
            0,
        );
    });

    // ── Frame sizer: the hint rides at the top; a spacer reserves the
    //    region used by the absolutely-placed group boxes. ────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add_spacer(320);
    frame.set_sizer(sizer);

    app.run(frame);
}
