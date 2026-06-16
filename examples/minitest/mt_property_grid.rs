//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `PropertyGrid` — two-column property sheet with
//! in-place editors (`wxPropertyGrid`).
//!
//! Demonstrates:
//! - Building a `PropertyGrid` and appending properties of every
//!   supported value type (`String`, `Int`, `Float`, `Bool`, `Choice`).
//! - Registering an `on_change` callback fired every time the user
//!   commits an edit (clicking away, pressing Enter, or toggling a
//!   boolean).
//! - Reading values back via `get_value` and showing them in a
//!   message box.
//! - Programmatically updating a value via `set_value` (does **not**
//!   fire the `on_change` callback).
//! - Clearing the grid via `clear` and reporting the new row count.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_property_grid
//! ```

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, Frame, PropertyGrid, PropertyValue, StaticText, StatusBar,
    message_box, MessageBoxIcon, MessageBoxStyle,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — PropertyGrid")
        .with_size(480, 380)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click on any value cell to edit it.", 0);

    StaticText::new(
        &frame,
        "Edit values by clicking on the right column.\nUse the buttons below to inspect / mutate / clear.",
    );

    // ── Build the grid ─────────────────────────────────────────────
    // Wrap the grid in `Rc<RefCell<...>>` so several button closures
    // can borrow it mutably to read / write / clear it.
    let grid: Rc<RefCell<PropertyGrid>> = Rc::new(RefCell::new(PropertyGrid::new(&frame)));

    // Populate the initial set of properties.
    {
        let mut g = grid.borrow_mut();
        g.append("Name", PropertyValue::String("Alice".into()));
        g.append("Age", PropertyValue::Int(30));
        g.append("Score", PropertyValue::Float(0.85));
        g.append("Active", PropertyValue::Bool(true));
        g.append(
            "Role",
            PropertyValue::Choice {
                options: vec!["User".into(), "Admin".into(), "Owner".into()],
                selected: 1,
            },
        );
    }

    // React to user commits. The callback receives the property index
    // that just changed; we look up the new value via `get_value` and
    // write a summary to stdout + the status bar.
    let status_for_change = status.clone();
    grid.borrow_mut().on_change(move |idx| {
        let name = match idx {
            0 => "Name",
            1 => "Age",
            2 => "Score",
            3 => "Active",
            4 => "Role",
            _ => "<unknown>",
        };
        println!("[grid] property {idx} ({name}) changed");
        status_for_change.set_status_text(&format!("Changed: {name}"), 0);
    });

    // ── Button row ─────────────────────────────────────────────────
    // 1. Dump — read every property back and show the values in a
    //    modal message box.
    let frame_for_dump = frame.clone();
    let btn_dump = Button::new(&frame, "Dump all values");
    let grid_for_dump = grid.clone();
    btn_dump.on_click(&frame, move || {
        let g = grid_for_dump.borrow();
        let mut lines = Vec::new();
        for i in 0..g.len() {
            let label = match i {
                0 => "Name",
                1 => "Age",
                2 => "Score",
                3 => "Active",
                4 => "Role",
                _ => "?",
            };
            let value_str = match g.get_value(i) {
                Some(v) => format!("{v:?}"),
                None => "<out of range>".to_string(),
            };
            lines.push(format!("{label} = {value_str}"));
        }
        let dump = lines.join("\n");
        message_box(
            &frame_for_dump,
            &dump,
            "PropertyGrid — read-back",
            MessageBoxStyle::Ok,
            MessageBoxIcon::Information,
        );
    });

    // 2. Programmatic set — flip "Active" to its opposite and bump
    //    "Score" by 0.10. `set_value` does **not** fire the
    //    `on_change` callback, so the status bar doesn't get a
    //    notification — the only feedback is the repaint.
    let btn_set = Button::new(&frame, "Programmatic set (toggle Active, bump Score)");
    let grid_for_set = grid.clone();
    btn_set.on_click(&frame, move || {
        let mut g = grid_for_set.borrow_mut();
        let new_active = match g.get_value(3) {
            Some(PropertyValue::Bool(b)) => PropertyValue::Bool(!b),
            _ => PropertyValue::Bool(false),
        };
        let new_score = match g.get_value(2) {
            Some(PropertyValue::Float(f)) => PropertyValue::Float((f + 0.10).clamp(0.0, 1.0)),
            _ => PropertyValue::Float(0.0),
        };
        g.set_value(3, new_active);
        g.set_value(2, new_score);
    });

    // 3. Clear — wipe every property and report the new row count.
    let status_for_clear = status.clone();
    let btn_clear = Button::new(&frame, "Clear all properties");
    let grid_for_clear = grid.clone();
    btn_clear.on_click(&frame, move || {
        grid_for_clear.borrow_mut().clear();
        let n = grid_for_clear.borrow().len();
        status_for_clear.set_status_text(&format!("Cleared; {n} row(s) remain"), 0);
    });

    // 4. Re-populate — start over with the same five default values.
    let btn_repopulate = Button::new(&frame, "Re-populate with defaults");
    let grid_for_repopulate = grid.clone();
    btn_repopulate.on_click(&frame, move || {
        let mut g = grid_for_repopulate.borrow_mut();
        g.append("Name", PropertyValue::String("Alice".into()));
        g.append("Age", PropertyValue::Int(30));
        g.append("Score", PropertyValue::Float(0.85));
        g.append("Active", PropertyValue::Bool(true));
        g.append(
            "Role",
            PropertyValue::Choice {
                options: vec!["User".into(), "Admin".into(), "Owner".into()],
                selected: 1,
            },
        );
    });

    // ── Layout ─────────────────────────────────────────────────────
    // The grid and the buttons live in a vertical BoxSizer. The grid
    // takes the top, growing band; the four buttons stack below.
    let mut sizer = BoxSizer::vertical();
    sizer.add(grid.borrow().as_widget_ref());
    sizer.add(btn_dump.as_widget_ref());
    sizer.add(btn_set.as_widget_ref());
    sizer.add(btn_clear.as_widget_ref());
    sizer.add(btn_repopulate.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
