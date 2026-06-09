//! Minitest: `StaticLine` — horizontal and vertical separators.
//!
//! Demonstrates the two `StaticLineOrientation` variants placed
//! between two pieces of text. On Windows the line is drawn as
//! an `SS_ETCHEDHORZ` / `SS_ETCHEDVERT` `STATIC` control, so the
//! orientation property is also exposed at runtime via
//! `orientation()`.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_static_line
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Frame, StaticLine, StaticLineOrientation, StaticText};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StaticLine")
        .with_size(420, 260)
        .build();

    // (1) The default constructor mirrors `new_horizontal`
    // when given `Default::default()`. Verify both forms.
    let line_h_explicit = StaticLine::new(&frame, StaticLineOrientation::Horizontal);
    assert_eq!(line_h_explicit.orientation(), StaticLineOrientation::Horizontal);

    let line_h_sugar = StaticLine::new_horizontal(&frame);
    assert_eq!(line_h_sugar.orientation(), StaticLineOrientation::Horizontal);

    let line_v = StaticLine::new_vertical(&frame);
    assert_eq!(line_v.orientation(), StaticLineOrientation::Vertical);

    // (2) Pack the demo widgets. The first `StaticText` is the
    // header, then the horizontal line, then a sub-section, then
    // the vertical line, then a right-hand block.
    let header = StaticText::new(&frame, "Above the horizontal line");
    let sub = StaticText::new(&frame, "Below the horizontal line");

    let left = StaticText::new(&frame, "Left");
    let right = StaticText::new(&frame, "Right");

    let mut sizer = BoxSizer::vertical();
    sizer.add(header.as_widget_ref());
    sizer.add(line_h_explicit.as_widget_ref());
    sizer.add(sub.as_widget_ref());

    // The vertical line is laid out next to two texts.
    sizer.add(left.as_widget_ref());
    sizer.add(line_v.as_widget_ref());
    sizer.add(right.as_widget_ref());

    frame.set_sizer(sizer);

    app.run(frame);
}
