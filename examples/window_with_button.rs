//! Demo: a window with native Win32 widgets laid out by a BoxSizer.
//!
//! Demonstrates:
//! - App + Frame with title and size
//! - StaticText label
//! - Button with click callback that updates the label
//! - SVG icon button (using Bootstrap Icons rasterised via resvg)
//! - Vertical BoxSizer for layout
//! - MenuBar with File (New, Open, Exit) and Help (About, disabled)
//! - Menu items with SVG icons via `append_with_svg_icon`
//!
//! Run with:
//! ```bash
//! cargo run --example window_with_button
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, Menu, MenuBar, StaticText};

// Embed Bootstrap Icons SVG files at compile time
const STAR_SVG: &[u8] = include_bytes!("../assets/icons/star.svg");
const FILE_NEW_SVG: &[u8] = include_bytes!("../assets/icons/file-new.svg");
const FOLDER_OPEN_SVG: &[u8] = include_bytes!("../assets/icons/folder-open.svg");
const EXIT_SVG: &[u8] = include_bytes!("../assets/icons/exit.svg");

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx Native Demo")
        .with_size(500, 350)
        .build();

    // --- StaticText ---
    let label = StaticText::new(&frame, "Hello from native Win32 widgets!");

    // --- Button (clones label so the callback can update it) ---
    let label_for_click = label.clone();
    let button = Button::new(&frame, "Click Me!");
    button.on_click(&frame, move || {
        label_for_click.set_label("Button was clicked!");
    });

    // --- SVG icon button (Bootstrap Icons star) ---
    let label_for_svg = label.clone();
    let svg_button = Button::new_with_svg_bytes(&frame, STAR_SVG, 24);
    svg_button.on_click(&frame, move || {
        label_for_svg.set_label("SVG icon button clicked!");
    });

    // --- Vertical BoxSizer ---
    let mut sizer = BoxSizer::vertical();
    sizer.add(label.as_widget_ref());
    sizer.add(button.as_widget_ref());
    sizer.add(svg_button.as_widget_ref());
    frame.set_sizer(sizer);

    // --- MenuBar ---
    // File menu with SVG icon items
    let mut file_menu = Menu::new("&File");

    let frame_for_new = frame.clone();
    file_menu.append_with_svg_icon("&New", FILE_NEW_SVG, 16, &frame, move || {
        frame_for_new.set_title("New file created");
    });

    let frame_for_open = frame.clone();
    file_menu.append_with_svg_icon("&Open", FOLDER_OPEN_SVG, 16, &frame, move || {
        frame_for_open.set_title("File opened");
    });

    let frame_for_exit = frame.clone();
    file_menu.append_with_svg_icon("E&xit", EXIT_SVG, 16, &frame, move || {
        frame_for_exit.close();
    });

    // Help menu with About (disabled)
    let mut help_menu = Menu::new("&Help");
    help_menu.append_disabled("&About");

    let mut menubar = MenuBar::new();
    menubar.append(file_menu);
    menubar.append(help_menu);
    frame.set_menu_bar(menubar);

    // --- Run ---
    app.run(frame);
}
