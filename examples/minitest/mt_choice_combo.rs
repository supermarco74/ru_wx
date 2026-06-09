//! Minitest: `Choice` and `ComboBox` — drop-down selection controls.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_choice_combo
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Choice, ComboBox, Frame, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Choice / ComboBox")
        .with_size(460, 320)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Pick or type a value.", 0);

    // Choice — read-only dropdown
    let lbl1 = StaticText::new(&frame, "Choice (read-only):");
    let choice = Choice::new(&frame);
    for fruit in ["Apples", "Oranges", "Bananas", "Grapes", "Pineapples"] {
        choice.append(fruit);
    }
    choice.set_selection(0);
    let choice_for_cb = choice.clone();
    let s = status.clone();
    choice.on_selection_change(&frame, move || {
        if let Some(i) = choice_for_cb.get_selection() {
            if let Some(label) = choice_for_cb.get_string(i) {
                s.set_status_text(&format!("Choice → {label}"), 0);
            }
        }
    });

    // ComboBox — editable dropdown
    let lbl2 = StaticText::new(&frame, "ComboBox (editable):");
    let combo = ComboBox::new(&frame);
    for c in ["Red", "Green", "Blue", "Magenta", "Cyan"] {
        combo.append(c);
    }
    combo.set_selection(2);

    // Button to read out the current ComboBox value
    let combo_for_btn = combo.clone();
    let s = status.clone();
    let btn = Button::new(&frame, "Show ComboBox value");
    btn.on_click(&frame, move || {
        s.set_status_text(
            &format!("ComboBox text = {:?}", combo_for_btn.get_value()),
            0,
        );
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl1.as_widget_ref());
    sizer.add(choice.as_widget_ref());
    sizer.add(lbl2.as_widget_ref());
    sizer.add(combo.as_widget_ref());
    sizer.add(btn.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
