//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `PopupMenu` — context menu shown by clicking a button.
//!
//! Demonstrates:
//! - Building a popup with plain, separator, checkable and icon items
//! - Showing the popup at the cursor (`PopupMenu::popup`)
//! - Showing the popup at fixed coordinates (`PopupMenu::popup_at`)
//!
//! Run with:
//! ```bash
//! cargo run --example mt_context_menu
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Colour, Frame, PopupMenu, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Context (popup) menu")
        .with_size(460, 280)
        .build();

    let _hint = StaticText::new(
        &frame,
        "Click either button to open a context menu. The status bar shows the chosen item.",
    );

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Ready — try the buttons below.", 0);

    // Button 1 — popup at cursor position
    let frame_for_btn1 = frame.clone();
    let status_for_btn1 = status.clone();
    let btn_at_cursor = Button::new(&frame, "Open menu at cursor");
    btn_at_cursor.on_click(&frame, move || {
        let mut popup = PopupMenu::new();

        let s = status_for_btn1.clone();
        popup.append("Cut", &frame_for_btn1, move || {
            s.set_status_text("Popup → Cut", 0);
        });
        let s = status_for_btn1.clone();
        popup.append("Copy", &frame_for_btn1, move || {
            s.set_status_text("Popup → Copy", 0);
        });
        let s = status_for_btn1.clone();
        popup.append("Paste", &frame_for_btn1, move || {
            s.set_status_text("Popup → Paste", 0);
        });

        popup.append_separator();

        let s = status_for_btn1.clone();
        popup.append_with_colour_icon(
            "Mark in red",
            Colour::new(220, 60, 60, 255),
            &frame_for_btn1,
            move || s.set_status_text("Popup → Marked in red", 0),
        );

        let s = status_for_btn1.clone();
        popup.append_check_item("Pin to top", &frame_for_btn1, move || {
            s.set_status_text("Popup → Pin toggled", 0);
        });

        popup.append_separator();

        popup.append_disabled("Disabled item");

        popup.popup(&frame_for_btn1);
    });

    // Button 2 — popup at fixed screen position (top-left corner)
    let frame_for_btn2 = frame.clone();
    let status_for_btn2 = status.clone();
    let btn_at_pos = Button::new(&frame, "Open menu at (100, 100)");
    btn_at_pos.on_click(&frame, move || {
        let mut popup = PopupMenu::new();
        let s = status_for_btn2.clone();
        popup.append("Anchored item A", &frame_for_btn2, move || {
            s.set_status_text("Anchored A", 0);
        });
        let s = status_for_btn2.clone();
        popup.append("Anchored item B", &frame_for_btn2, move || {
            s.set_status_text("Anchored B", 0);
        });
        popup.popup_at(&frame_for_btn2, 100, 100);
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_at_cursor.as_widget_ref());
    sizer.add(btn_at_pos.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
