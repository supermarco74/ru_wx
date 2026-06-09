//! Minitest: `WindowCornerPreference` — Windows 11 rounded corners.
//!
//! Demonstrates:
//! - `TopLevelWindow::set_window_corner_preference` (the Win32
//!   `DWMWA_WINDOW_CORNER_PREFERENCE` DWM attribute)
//! - `TopLevelWindow::get_window_corner_preference` round-trip
//! - All four preference variants: `Default`, `DoNotRound`, `Round`,
//!   `RoundSmall`
//!
//! Click any of the four "apply" buttons to change the corner
//! preference of the running window. The change takes effect
//! immediately (the DWM redraws the frame with the new corner shape).
//! The "Show current" button reads back the value the DWM reports and
//! writes it to the status bar — useful for verifying the round-trip
//! and for detecting older Windows releases where the DWM call is
//! accepted but the attribute is ignored (in which case the read-back
//! still returns the value that was just written).
//!
//! Run with:
//! ```bash
//! cargo run --example mt_window_corners
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StatusBar, TopLevelWindow, WindowCornerPreference};

fn main() {
    let app = App::new();

    // Single TopLevelWindow — the inner Frame is what we attach
    // controls to, and the wrapper is what gives us the corner-
    // preference API.
    let window = TopLevelWindow::new(
        "Minitest — WindowCornerPreference (Win11 rounded corners)",
        540,
        360,
    );
    let frame: Frame = window.frame().clone();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text(
        "Click an apply button to change the window's corner preference.",
        0,
    );

    // ── Apply buttons ─────────────────────────────────────────────────
    // Each button sets the corner preference on the running window.
    // On Windows 11 the DWM redraws the frame with the new corner
    // shape immediately; on older Windows releases the call still
    // succeeds but the compositor ignores it (window stays rectangular).
    let btn_default = Button::new(&frame, "Apply: Default (let system decide)");
    let s = status.clone();
    let w = window.clone();
    btn_default.on_click(&frame, move || {
        let ok = w.set_window_corner_preference(WindowCornerPreference::Default);
        s.set_status_text(
            if ok {
                "Applied: Default (let system decide)"
            } else {
                "Failed to apply: Default"
            },
            0,
        );
    });

    let btn_donot = Button::new(&frame, "Apply: DoNotRound (rectangular)");
    let s = status.clone();
    let w = window.clone();
    btn_donot.on_click(&frame, move || {
        let ok = w.set_window_corner_preference(WindowCornerPreference::DoNotRound);
        s.set_status_text(
            if ok {
                "Applied: DoNotRound (rectangular)"
            } else {
                "Failed to apply: DoNotRound"
            },
            0,
        );
    });

    let btn_round = Button::new(&frame, "Apply: Round (large rounded corners)");
    let s = status.clone();
    let w = window.clone();
    btn_round.on_click(&frame, move || {
        let ok = w.set_window_corner_preference(WindowCornerPreference::Round);
        s.set_status_text(
            if ok {
                "Applied: Round (large rounded corners)"
            } else {
                "Failed to apply: Round"
            },
            0,
        );
    });

    let btn_small = Button::new(&frame, "Apply: RoundSmall (small rounded corners)");
    let s = status.clone();
    let w = window.clone();
    btn_small.on_click(&frame, move || {
        let ok = w.set_window_corner_preference(WindowCornerPreference::RoundSmall);
        s.set_status_text(
            if ok {
                "Applied: RoundSmall (small rounded corners)"
            } else {
                "Failed to apply: RoundSmall"
            },
            0,
        );
    });

    // ── Show-current button ───────────────────────────────────────────
    // Reads back the value the DWM reports and writes it to the
    // status bar. Verifies the round-trip and detects older Windows
    // releases where the DWM call is accepted but the attribute is
    // ignored (the read-back then still returns the value that was
    // just written).
    let btn_show = Button::new(&frame, "Show current preference (DWM read-back)");
    let s = status.clone();
    let w = window.clone();
    btn_show.on_click(&frame, move || match w.get_window_corner_preference() {
        Some(p) => s.set_status_text(&format!("Current preference: {:?}", p), 0),
        None => s.set_status_text(
            "DWM did not report a corner preference (not supported on this OS)",
            0,
        ),
    });

    // ── Apply-all / round-trip test button ────────────────────────────
    // Cycles through all four preferences, applies each one in turn
    // and immediately reads it back, so a single click verifies the
    // full set of (apply, read-back) round-trips. The status bar
    // shows a pass/fail summary.
    let btn_all = Button::new(&frame, "Round-trip test: apply all 4 + read back");
    let s = status.clone();
    let w = window.clone();
    btn_all.on_click(&frame, move || {
        let prefs = [
            ("Default", WindowCornerPreference::Default),
            ("DoNotRound", WindowCornerPreference::DoNotRound),
            ("Round", WindowCornerPreference::Round),
            ("RoundSmall", WindowCornerPreference::RoundSmall),
        ];
        let mut ok_count = 0usize;
        let mut mismatch: Option<String> = None;
        for (label, p) in prefs.iter() {
            if !w.set_window_corner_preference(*p) {
                continue;
            }
            match w.get_window_corner_preference() {
                Some(read_back) if read_back == *p => ok_count += 1,
                Some(read_back) => {
                    mismatch = Some(format!("{label}: wrote {:?}, read {:?}", p, read_back));
                }
                None => {
                    mismatch = Some(format!("{label}: write ok, read returned None"));
                }
            }
        }
        let msg = if let Some(m) = mismatch {
            format!("Round-trip: {ok_count}/4 ok, mismatch: {m}")
        } else {
            format!("Round-trip: {ok_count}/4 ok (all preferences applied and read back)")
        };
        s.set_status_text(&msg, 0);
    });

    // ── Layout ────────────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_default.as_widget_ref());
    sizer.add(btn_donot.as_widget_ref());
    sizer.add(btn_round.as_widget_ref());
    sizer.add(btn_small.as_widget_ref());
    sizer.add(btn_show.as_widget_ref());
    sizer.add(btn_all.as_widget_ref());
    sizer.add_spacer(22);
    frame.set_sizer(sizer);

    // Set the default Win11 corner shape on startup so the user can
    // see the rounded corners immediately (the OS default for new
    // top-level app windows is already large-rounded, but setting it
    // explicitly makes the test self-contained).
    window.set_window_corner_preference(WindowCornerPreference::Default);

    app.run(frame);
}
