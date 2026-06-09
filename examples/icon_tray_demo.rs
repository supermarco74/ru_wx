//! Demo: a window with a system-tray (notification area) icon — the
//! ru_wx port of `wxTaskBarIcon`.
//!
//! Demonstrates:
//! - `IconTray::new` — creates a tray icon from embedded SVG bytes
//! - Tooltip (set with `set_tooltip`)
//! - Left-click / double-click / right-click / balloon-click callbacks
//! - Right-click context menu (shown automatically when a `Menu` is
//!   attached with `set_menu`)
//! - Balloon / toast notification via `show_balloon`
//! - "Hide / Show" buttons to toggle the icon's visibility
//!
//! Run with:
//! ```bash
//! cargo run --example icon_tray_demo
//! ```
//!
//! The tray icon will appear in the system notification area as soon as
//! the window opens. Right-click it to see the context menu. Click the
//! "Show notification" button (or pick the same item in the context
//! menu) to fire a balloon.

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::rc::Rc;

use ru_wx::{App, BalloonIcon, BoxSizer, Button, Frame, IconTray, Menu, StaticText};

// Embedded Bootstrap Icons SVG files
const STAR_SVG: &[u8] = include_bytes!("../assets/icons/star.svg");
const INFO_SVG: &[u8] = include_bytes!("../assets/icons/info.svg");

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx · IconTray Demo")
        .with_size(520, 360)
        .build();

    // --- Status label: shows the most recent tray event ---
    let status = StaticText::new(&frame, "Status: tray icon is active. Right-click it.");

    // --- Build and configure the tray icon ---
    let mut tray =
        IconTray::new(&frame, STAR_SVG, 16).expect("failed to create tray icon from STAR_SVG");
    tray.set_tooltip("ru_wx · IconTray demo");

    // --- Context menu attached to the tray icon ---
    let mut tray_menu = Menu::new("TrayMenu");
    let status_for_menu = status.clone();
    tray_menu.append_with_svg_icon("Show &notification", INFO_SVG, 16, &frame, move || {
        status_for_menu.set_label("Status: notification fired from context menu.");
    });
    tray_menu.append_disabled("&About");
    tray.set_menu(tray_menu);

    // --- Tray event handlers (closures capture only the status label) ---
    let status_for_left = status.clone();
    tray.on_left_click(move || {
        status_for_left.set_label("Status: left-click on tray icon!");
    });

    let status_for_dbl = status.clone();
    tray.on_left_double_click(move || {
        status_for_dbl.set_label("Status: double-click on tray icon!");
    });

    let status_for_right = status.clone();
    tray.on_right_click(move || {
        status_for_right.set_label("Status: right-click — context menu shown.");
    });

    let status_for_balloon_click = status.clone();
    tray.on_balloon_click(move || {
        status_for_balloon_click.set_label("Status: balloon was clicked!");
    });

    // --- Wrap the tray in Rc<RefCell<_>> so the buttons can also use
    //     it (hide / show / show_balloon). The tray is kept alive in
    //     `main`'s scope and properly dropped (removing the OS tray
    //     entry) when the program exits. ---
    let tray: Rc<RefCell<IconTray>> = Rc::new(RefCell::new(tray));

    // --- "Show notification" button ---
    let tray_for_balloon = tray.clone();
    let status_for_balloon = status.clone();
    let balloon_btn = Button::new(&frame, "Show notification");
    balloon_btn.on_click(&frame, move || {
        let t = tray_for_balloon.borrow();
        let ok = t.show_balloon(
            "ru_wx",
            "Hello from the system tray! Click me to fire on_balloon_click.",
            BalloonIcon::Info,
        );
        let msg = if ok {
            "Status: balloon shown."
        } else {
            "Status: failed to show balloon."
        };
        status_for_balloon.set_label(msg);
    });

    // --- "Hide tray icon" button ---
    let tray_for_hide = tray.clone();
    let status_for_hide = status.clone();
    let hide_btn = Button::new(&frame, "Hide tray icon");
    hide_btn.on_click(&frame, move || {
        let mut t = tray_for_hide.borrow_mut();
        t.hide();
        status_for_hide.set_label("Status: tray icon hidden (click 'Show' to bring it back).");
    });

    // --- "Show tray icon" button ---
    let tray_for_show = tray.clone();
    let status_for_show = status.clone();
    let show_btn = Button::new(&frame, "Show tray icon");
    show_btn.on_click(&frame, move || {
        let mut t = tray_for_show.borrow_mut();
        let ok = t.show();
        let msg = if ok {
            "Status: tray icon re-shown."
        } else {
            "Status: failed to re-show tray icon."
        };
        status_for_show.set_label(msg);
    });

    // --- Vertical BoxSizer ---
    let mut sizer = BoxSizer::vertical();
    sizer.add(status.as_widget_ref());
    sizer.add(balloon_btn.as_widget_ref());
    sizer.add(hide_btn.as_widget_ref());
    sizer.add(show_btn.as_widget_ref());
    frame.set_sizer(sizer);

    // --- Keep a strong reference to the tray in main's scope. The
    //     `app.run` call below enters the Win32 message loop; when the
    //     window is closed, `app.run` returns and `tray` (and the
    //     cloned `Rc`s held by the closures) is dropped, which removes
    //     the tray icon from the notification area. ---
    let _tray_ref = tray;

    // --- Run ---
    app.run(frame);
}
