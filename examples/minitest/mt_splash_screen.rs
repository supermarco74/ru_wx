//! Minitest: `SplashScreen` — a top-most borderless bitmap window
//! (`wxSplashScreen`).
//!
//! Demonstrates:
//! - Building a `Bitmap` (here a blank 320×160 of opaque pixels —
//!   in a real app this would be your logo or product image).
//! - Constructing a `SplashScreen` parented to the main frame with
//!   an auto-close timer.
//! - Manually closing the splash before the timer fires.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_splash_screen
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, Bitmap, BoxSizer, Button, Frame, SplashScreen, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — SplashScreen")
        .with_size(540, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Use the buttons to control the splash.", 0);

    // Build a small blank bitmap for the splash content. Real apps
    // would use a loaded PNG / SVG of the application logo.
    let bitmap = Bitmap::new(320, 160);

    // Create a splash with a 3-second auto-close timer.
    let splash = SplashScreen::new(&frame, bitmap, 3_000);
    splash.show();

    // Manually close the splash before the timer fires.
    let splash_for_close = splash.clone();
    let btn_close = Button::new(&frame, "Close splash now");
    let status_for_close = status.clone();
    btn_close.on_click(&frame, move || {
        splash_for_close.close();
        status_for_close.set_status_text("Splash closed manually", 0);
    });

    // Re-show the splash (re-uses the same bitmap + timer).
    let splash_for_show = splash.clone();
    let btn_show = Button::new(&frame, "Re-show splash");
    let status_for_show = status.clone();
    btn_show.on_click(&frame, move || {
        splash_for_show.show();
        status_for_show.set_status_text("Splash shown (3 s auto-close)", 0);
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_close.as_widget_ref());
    sizer.add(btn_show.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
