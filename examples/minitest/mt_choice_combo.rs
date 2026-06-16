//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Choice`, `ComboBox` and `BitmapComboBox` — drop-down
//! selection controls.
//!
//! Demonstrates:
//! - `Choice` and `ComboBox` with two-way selection sync
//! - `BitmapComboBox` with per-row SVG icons
//!   (`ImageList` + `set_image_list` + `append_with_image`)
//! - Adding new entries to all three controls at runtime
//! - Selection feedback in a two-field `StatusBar`
//!
//! Run with:
//! ```bash
//! cargo run --example mt_choice_combo
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{
    App, BitmapComboBox, BoxSizer, Button, Choice, ComboBox, Frame, ImageList, StaticText,
    StatusBar, ToolTip,
};

// NB: SVG literals containing `#RRGGBB` colours need `br##"..."##`.
const SVG_APPLE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#D9486A" stroke="#D9486A"><circle cx="12" cy="14" r="7"/><path d="M12 7c0-3 2-4 4-4" fill="none" stroke-width="2"/></svg>"##;
const SVG_ORANGE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#E8902A" stroke="#E8902A"><circle cx="12" cy="13" r="8"/><path d="M12 5l3-3" fill="none" stroke-width="2"/></svg>"##;
const SVG_GRAPE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#7B4FA4" stroke="#7B4FA4"><circle cx="9" cy="10" r="3.5"/><circle cx="15" cy="10" r="3.5"/><circle cx="12" cy="16" r="3.5"/></svg>"##;
const SVG_LEAF: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#4FA464" stroke="#4FA464"><path d="M4 20C4 10 10 4 20 4c0 10-6 16-16 16z"/><path d="M4 20C9 15 13 11 18 6" fill="none" stroke="#2C6E3F" stroke-width="1.5"/></svg>"##;

const FRUITS: [&str; 5] = ["Apples", "Oranges", "Bananas", "Grapes", "Pineapples"];

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Choice / ComboBox / BitmapComboBox")
        .with_size(500, 420)
        .build();

    // Field 0 = current selection, field 1 = item count.
    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Pick or type a value.", 0);

    // ── Choice + ComboBox kept in sync ──────────────────────────────
    let lbl1 = StaticText::new(&frame, "Choice and ComboBox stay in sync:");
    let choice = Choice::new(&frame);
    let combo = ComboBox::new(&frame);
    for fruit in FRUITS {
        choice.append(fruit);
        combo.append(fruit);
    }
    choice.set_selection(0);
    combo.set_selection(0);

    // Guard against feedback loops while we mirror a selection.
    let syncing = Rc::new(Cell::new(false));

    {
        let choice = choice.clone();
        let combo = combo.clone();
        let syncing = syncing.clone();
        let s = status.clone();
        choice.clone().on_selection_change(&frame, move || {
            if syncing.get() {
                return;
            }
            if let Some(i) = choice.get_selection() {
                syncing.set(true);
                combo.set_selection(i);
                syncing.set(false);
                let label = choice.get_string(i).unwrap_or_default();
                s.set_status_text(&format!("Choice → {label} (combo mirrored)"), 0);
            }
        });
    }
    {
        let choice = choice.clone();
        let combo = combo.clone();
        let syncing = syncing.clone();
        let s = status.clone();
        combo.clone().on_selection_change(&frame, move || {
            if syncing.get() {
                return;
            }
            if let Some(i) = combo.get_selection() {
                syncing.set(true);
                choice.set_selection(i);
                syncing.set(false);
                let label = combo.get_string(i).unwrap_or_default();
                s.set_status_text(&format!("ComboBox → {label} (choice mirrored)"), 0);
            }
        });
    }

    // ── BitmapComboBox with SVG icons ───────────────────────────────
    let lbl2 = StaticText::new(&frame, "BitmapComboBox (rows carry SVG icons):");
    let icons = ImageList::new(16, 16);
    icons.add_svg_bytes(SVG_APPLE);
    icons.add_svg_bytes(SVG_ORANGE);
    icons.add_svg_bytes(SVG_GRAPE);
    icons.add_svg_bytes(SVG_LEAF);

    let bitmap_combo = BitmapComboBox::new(&frame);
    bitmap_combo.set_image_list(&icons);
    bitmap_combo.append_with_image("Apple (red)", 0);
    bitmap_combo.append_with_image("Orange (orange)", 1);
    bitmap_combo.append_with_image("Grape (purple)", 2);
    bitmap_combo.append_with_image("Mint (green)", 3);
    bitmap_combo.append_with_image("Plain row (no icon)", -1);
    bitmap_combo.set_selection(0);
    ToolTip::new("Open the dropdown to see icons on every row")
        .attach(&bitmap_combo.as_widget_ref());

    {
        let bc = bitmap_combo.clone();
        let s = status.clone();
        bitmap_combo.on_selection_change(&frame, move || {
            if let Some(i) = bc.get_selection() {
                s.set_status_text(&format!("BitmapComboBox → {}", bc.get_string(i)), 0);
            }
        });
    }

    // ── Runtime growth: one button feeds all three controls ─────────
    let counter = Rc::new(Cell::new(0u32));
    let btn_add = Button::new(&frame, "Add item to all dropdowns");
    {
        let choice = choice.clone();
        let combo = combo.clone();
        let bc = bitmap_combo.clone();
        let s = status.clone();
        let counter = counter.clone();
        btn_add.on_click(&frame, move || {
            counter.set(counter.get() + 1);
            let name = format!("Custom fruit #{}", counter.get());
            choice.append(&name);
            combo.append(&name);
            bc.append_with_image(&name, 3); // reuse the leaf icon
            s.set_status_text(&format!("Added '{name}' to all three controls"), 0);
            s.set_status_text(&format!("{} items each", choice.get_count()), 1);
        });
    }

    let combo_for_btn = combo.clone();
    let s = status.clone();
    let btn_value = Button::new(&frame, "Show ComboBox text");
    btn_value.on_click(&frame, move || {
        s.set_status_text(&format!("ComboBox text = {:?}", combo_for_btn.get_value()), 0);
    });

    status.set_status_text(&format!("{} items each", choice.get_count()), 1);

    // Layout: the two action buttons share a nested horizontal row.
    let mut buttons = BoxSizer::horizontal();
    buttons.add(btn_add.as_widget_ref());
    buttons.add(btn_value.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl1.as_widget_ref());
    sizer.add(choice.as_widget_ref());
    sizer.add(combo.as_widget_ref());
    sizer.add(lbl2.as_widget_ref());
    sizer.add(bitmap_combo.as_widget_ref());
    sizer.add_sizer(buttons);
    frame.set_sizer(sizer);

    app.run(frame);
}
