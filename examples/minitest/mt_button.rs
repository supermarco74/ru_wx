//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Button` — various forms inside a single window.
//!
//! Demonstrates:
//! - Plain text button
//! - Coloured bitmap button (`new_with_bitmap`)
//! - A row of SVG-icon buttons (`new_with_svg_bytes`) with tooltips,
//!   nested via `BoxSizer::add_sizer` (horizontal row in vertical column)
//! - SVG-icon button loaded from an asset file
//! - A button that enables / disables another button
//! - A shared click counter reported in the `StatusBar`
//! - Self-updating button label
//!
//! Run with:
//! ```bash
//! cargo run --example mt_button
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{App, BoxSizer, Button, Colour, Frame, StaticText, StatusBar, ToolTip};

const STAR_SVG: &[u8] = include_bytes!("../../assets/icons/star.svg");
// NB: SVG literals containing `#RRGGBB` colours need `br##"..."##`.
const SVG_HOME: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><path d="M3 11 12 3l9 8"/><path d="M5 10v10h5v-6h4v6h5V10"/></svg>"##;
const SVG_HEART: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#D9486A" stroke="#D9486A" stroke-width="1"><path d="M12 21s-7.5-4.9-9.5-9.5C1 7.5 3.5 4 7 4c2 0 3.6 1.1 5 3 1.4-1.9 3-3 5-3 3.5 0 6 3.5 4.5 7.5C19.5 16.1 12 21 12 21z"/></svg>"##;
const SVG_GEAR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#4FA464" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3h.1a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9v.1a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>"##;
const SVG_BOLT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#E8B339" stroke="#E8B339" stroke-width="1"><path d="M13 2 3 14h7l-1 8 11-13h-7z"/></svg>"##;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Button forms")
        .with_size(460, 460)
        .build();

    // Status bar: field 0 = last action, field 1 = shared click counter.
    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click any button…", 0);
    status.set_status_text("Total clicks: 0", 1);

    let label = StaticText::new(&frame, "Every button updates the status bar below.");

    // Shared counter incremented by every button in the window.
    let clicks = Rc::new(Cell::new(0u32));
    let bump = {
        let clicks = clicks.clone();
        let status = status.clone();
        move || {
            clicks.set(clicks.get() + 1);
            status.set_status_text(&format!("Total clicks: {}", clicks.get()), 1);
        }
    };

    // 1. Plain text button
    let btn_plain = Button::new(&frame, "Plain text button");
    {
        let s = status.clone();
        let bump = bump.clone();
        btn_plain.on_click(&frame, move || {
            bump();
            s.set_status_text("Plain text button clicked", 0);
        });
    }

    // 2. Coloured bitmap button (red 16×16)
    let btn_bmp =
        Button::new_with_bitmap(&frame, "Red bitmap", Colour::new(220, 60, 60, 255), 16, 16);
    {
        let s = status.clone();
        let bump = bump.clone();
        btn_bmp.on_click(&frame, move || {
            bump();
            s.set_status_text("Coloured bitmap button clicked", 0);
        });
    }

    // 3. A horizontal row of SVG icon buttons, each with its own tooltip.
    let icon_row_label = StaticText::new(&frame, "SVG icon row (hover for tooltips):");
    let mut icon_row = BoxSizer::horizontal();
    let icons: [(&[u8], &str); 4] = [
        (SVG_HOME, "Home (blue)"),
        (SVG_HEART, "Favourite (pink)"),
        (SVG_GEAR, "Settings (green)"),
        (SVG_BOLT, "Boost (yellow)"),
    ];
    for (svg, name) in icons {
        let btn = Button::new_with_svg_bytes(&frame, svg, 24);
        ToolTip::new(name).attach(&btn.as_widget_ref());
        let s = status.clone();
        let bump = bump.clone();
        let name = name.to_string();
        btn.on_click(&frame, move || {
            bump();
            s.set_status_text(&format!("Icon button: {name}"), 0);
        });
        icon_row.add(btn.as_widget_ref());
    }

    // 4. SVG icon loaded from an asset file (Bootstrap-Icons star)
    let btn_svg_file = Button::new_with_svg_bytes(&frame, STAR_SVG, 24);
    ToolTip::new("Star icon loaded from assets/icons/star.svg").attach(&btn_svg_file.as_widget_ref());
    {
        let s = status.clone();
        let bump = bump.clone();
        btn_svg_file.on_click(&frame, move || {
            bump();
            s.set_status_text("Asset-SVG (★) button clicked", 0);
        });
    }

    // 5. Enable / disable interplay: the toggle button controls the target.
    let btn_target = Button::new(&frame, "Target button");
    {
        let s = status.clone();
        let bump = bump.clone();
        btn_target.on_click(&frame, move || {
            bump();
            s.set_status_text("Target button clicked (so it must be enabled!)", 0);
        });
    }
    let btn_toggle = Button::new(&frame, "Disable target");
    ToolTip::new("Enables / disables the button on the right").attach(&btn_toggle.as_widget_ref());
    {
        let s = status.clone();
        let bump = bump.clone();
        let target = btn_target.clone();
        let toggle = btn_toggle.clone();
        let enabled = Rc::new(Cell::new(true));
        btn_toggle.on_click(&frame, move || {
            bump();
            let now = !enabled.get();
            enabled.set(now);
            target.as_widget_ref().borrow_mut().set_enabled(now);
            toggle.set_label(if now { "Disable target" } else { "Enable target" });
            s.set_status_text(
                if now { "Target button enabled" } else { "Target button disabled" },
                0,
            );
        });
    }
    let mut toggle_row = BoxSizer::horizontal();
    toggle_row.add(btn_toggle.as_widget_ref());
    toggle_row.add(btn_target.as_widget_ref());

    // 6. Self-updating: changes its own label on each click
    let btn_self = Button::new(&frame, "Click me!");
    {
        let btn_self_clone = btn_self.clone();
        let bump = bump.clone();
        let mine = Rc::new(Cell::new(0u32));
        btn_self.on_click(&frame, move || {
            bump();
            mine.set(mine.get() + 1);
            btn_self_clone.set_label(&format!("Clicked {} time(s)", mine.get()));
        });
    }

    // Layout: vertical column with two nested horizontal rows.
    let mut sizer = BoxSizer::vertical();
    sizer.add(label.as_widget_ref());
    sizer.add(btn_plain.as_widget_ref());
    sizer.add(btn_bmp.as_widget_ref());
    sizer.add(icon_row_label.as_widget_ref());
    sizer.add_sizer(icon_row);
    sizer.add(btn_svg_file.as_widget_ref());
    sizer.add_sizer(toggle_row);
    sizer.add(btn_self.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
