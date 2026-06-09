//! Minitest: `StaticBox` — labelled box container.
//!
//! Demonstrates the three `StaticBox` constructors (`new`,
//! `new_empty`, `with_size`) plus the `set_label` / `get_label`
//! round-trip. Children can be reparented to a `StaticBox` by
//! passing it as the parent in their own constructor; here we
//! reparent a `StaticText` for the visual grouping effect.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_static_box
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Frame, StaticBox, StaticText};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StaticBox")
        .with_size(420, 280)
        .build();

    // (1) Box with an initial label and a child StaticText
    // reparented to the box.
    let box1 = StaticBox::new(&frame, "Group A");
    assert_eq!(box1.get_label(), "Group A");
    let _child_a = StaticText::new(&box1, "(child of Group A)");

    // (2) Empty constructor + late label.
    let box2 = StaticBox::new_empty(&frame);
    assert_eq!(box2.get_label(), "");
    box2.set_label("Group B (renamed)");
    assert_eq!(box2.get_label(), "Group B (renamed)");
    let _child_b = StaticText::new(&box2, "(child of Group B)");

    // (3) Explicit size constructor.
    let box3 = StaticBox::with_size(&frame, "Group C (sized)", 200, 80);
    assert_eq!(box3.get_label(), "Group C (sized)");

    // (4) Reuse a label after the fact (round-trip).
    let box4 = StaticBox::new(&frame, "Original");
    assert_eq!(box4.get_label(), "Original");
    box4.set_label("Updated");
    assert_eq!(box4.get_label(), "Updated");

    // Layout: stack the four boxes vertically. The reparented
    // child texts ride along via the box's HWND parent.
    let mut sizer = BoxSizer::vertical();
    sizer.add(box1.as_widget_ref());
    sizer.add(box2.as_widget_ref());
    sizer.add(box3.as_widget_ref());
    sizer.add(box4.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
