//! Minitest: `MenuBar` — File / Edit / Help with all common item kinds.
//!
//! Demonstrates:
//! - Plain menu item
//! - Disabled item
//! - Item with SVG icon
//! - Item with keyboard shortcut (accelerator)
//! - Checkable item
//! - Radio item group
//! - Separators
//!
//! Run with:
//! ```bash
//! cargo run --example mt_menu
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{Accelerator, App, Frame, Menu, MenuBar, StaticText, StatusBar};

const FILE_NEW_SVG: &[u8] = include_bytes!("../../assets/icons/file-new.svg");
const FOLDER_OPEN_SVG: &[u8] = include_bytes!("../../assets/icons/folder-open.svg");
const EXIT_SVG: &[u8] = include_bytes!("../../assets/icons/exit.svg");
const INFO_SVG: &[u8] = include_bytes!("../../assets/icons/info.svg");
const STAR_SVG: &[u8] = include_bytes!("../../assets/icons/star.svg");

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — MenuBar")
        .with_size(520, 320)
        .build();

    let _label = StaticText::new(
        &frame,
        "Open the menus and pick items. Status bar reports the action.",
    );
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Ready.", 0);

    // ── File menu ───────────────────────────────────────────────────
    let mut file_menu = Menu::new("&File");

    let s = status.clone();
    file_menu.append_with_svg_icon("&New", FILE_NEW_SVG, 16, &frame, move || {
        s.set_status_text("File → New", 0);
    });

    let s = status.clone();
    file_menu.append_with_shortcut(
        "&Open…",
        Accelerator::parse("Ctrl+O").unwrap(),
        &frame,
        move || s.set_status_text("File → Open (Ctrl+O)", 0),
    );

    let s = status.clone();
    file_menu.append_with_svg_icon("Open &recent", FOLDER_OPEN_SVG, 16, &frame, move || {
        s.set_status_text("File → Open recent", 0);
    });

    let s = status.clone();
    file_menu.append_with_svg_icon("Star (favourite)", STAR_SVG, 16, &frame, move || {
        s.set_status_text("File → Star", 0);
    });

    file_menu.append_separator();

    file_menu.append_disabled("Save (disabled)");

    let frame_for_exit = frame.clone();
    file_menu.append_with_svg_icon("E&xit", EXIT_SVG, 16, &frame, move || {
        frame_for_exit.close();
    });

    // ── Edit menu — checkable + radio group ─────────────────────────
    let mut edit_menu = Menu::new("&Edit");

    let s = status.clone();
    edit_menu.append_check_item("Word &wrap", &frame, move || {
        s.set_status_text("Edit → Word wrap toggled", 0);
    });

    edit_menu.append_separator();

    // Radio group: zoom level
    let s = status.clone();
    edit_menu.append_radio_item("Zoom &50%", &frame, move || {
        s.set_status_text("Zoom: 50%", 0);
    });
    let s = status.clone();
    edit_menu.append_radio_item("Zoom &100%", &frame, move || {
        s.set_status_text("Zoom: 100%", 0);
    });
    let s = status.clone();
    edit_menu.append_radio_item("Zoom &200%", &frame, move || {
        s.set_status_text("Zoom: 200%", 0);
    });

    // ── Help menu ───────────────────────────────────────────────────
    let mut help_menu = Menu::new("&Help");

    let s = status.clone();
    help_menu.append_with_svg_icon("&About…", INFO_SVG, 16, &frame, move || {
        s.set_status_text("Help → About (this is a minitest)", 0);
    });

    // ── Wire it all up ──────────────────────────────────────────────
    let mut menubar = MenuBar::new();
    menubar.append(file_menu);
    menubar.append(edit_menu);
    menubar.append(help_menu);
    frame.set_menu_bar(menubar);

    app.run(frame);
}
