//! Minitest: minimal status bar visibility check.
//!
//! Creates a frame with a single 4-field status bar and a long
//! preset text in field 0. Used to verify that the status bar
//! renders at the bottom of the frame and is not occluded by
//! sibling controls.

#![windows_subsystem = "windows"]

use ru_wx::{App, Frame, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StatusBar only")
        .with_size(800, 200)
        .build();

    let status = StatusBar::new(&frame, 4);
    status.set_status_text(
        ">>> This is field 0 with a long string to verify full text fits <<<",
        0,
    );
    status.set_status_text("Field 1", 1);
    status.set_status_text("Field 2", 2);
    status.set_status_text("Field 3", 3);

    app.run(frame);
}
