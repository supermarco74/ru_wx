//! Minitest: `Button` — various forms inside a single window.
//!
//! Demonstrates:
//! - Plain text button
//! - Coloured bitmap button (`new_with_bitmap`)
//! - SVG-icon button from embedded bytes (`new_with_svg_bytes`)
//! - SVG-icon button loaded from an asset file
//! - Disabled button
//! - A button that updates its own label and a shared status label
//!
//! Run with:
//! ```bash
//! cargo run --example mt_button
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Colour, Frame, StaticText};

const STAR_SVG: &[u8] = include_bytes!("../../assets/icons/star.svg");
const INFO_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>"#;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Button forms")
        .with_size(420, 380)
        .build();

    let status = StaticText::new(&frame, "Click any button…");

    // 1. Plain text button
    let status_for_plain = status.clone();
    let btn_plain = Button::new(&frame, "Plain text button");
    btn_plain.on_click(&frame, move || {
        status_for_plain.set_label("Plain text button clicked");
    });

    // 2. Coloured bitmap button (red 16×16)
    let status_for_bmp = status.clone();
    let btn_bmp =
        Button::new_with_bitmap(&frame, "Red bitmap", Colour::new(220, 60, 60, 255), 16, 16);
    btn_bmp.on_click(&frame, move || {
        status_for_bmp.set_label("Coloured bitmap button clicked");
    });

    // 3. SVG icon from embedded inline bytes
    let status_for_svg_inline = status.clone();
    let btn_svg_inline = Button::new_with_svg_bytes(&frame, INFO_SVG, 24);
    btn_svg_inline.on_click(&frame, move || {
        status_for_svg_inline.set_label("Inline-SVG button clicked");
    });

    // 4. SVG icon loaded from an asset file (Bootstrap-Icons star)
    let status_for_svg_file = status.clone();
    let btn_svg_file = Button::new_with_svg_bytes(&frame, STAR_SVG, 24);
    btn_svg_file.on_click(&frame, move || {
        status_for_svg_file.set_label("Asset-SVG (★) button clicked");
    });

    // 5. Disabled button — never fires
    let btn_disabled = Button::new(&frame, "Disabled button");
    btn_disabled.as_widget_ref().borrow_mut().set_enabled(false);

    // 6. Self-updating: changes its own label on each click
    let btn_self = Button::new(&frame, "Click me!");
    let btn_self_clone = btn_self.clone();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0u32));
    btn_self.on_click(&frame, move || {
        counter.set(counter.get() + 1);
        btn_self_clone.set_label(&format!("Clicked {} time(s)", counter.get()));
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(status.as_widget_ref());
    sizer.add(btn_plain.as_widget_ref());
    sizer.add(btn_bmp.as_widget_ref());
    sizer.add(btn_svg_inline.as_widget_ref());
    sizer.add(btn_svg_file.as_widget_ref());
    sizer.add(btn_disabled.as_widget_ref());
    sizer.add(btn_self.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
