//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: Windows 11 modern appearance on a plain `Frame`.
//!
//! Demonstrates:
//! - `FrameBuilder::with_modern_style()` — dark title bar (follows
//!   the OS theme), Mica backdrop and rounded corners in one call
//! - `Frame::set_dark_title_bar` / `dark_title_bar` round-trip
//! - `Frame::set_backdrop` with every `BackdropType` variant
//! - `Frame::set_corner_preference` / `corner_preference`
//! - `Appearance::system_is_dark` OS theme detection
//!
//! Every DWM call degrades gracefully: on Windows 10 the backdrop
//! and corner calls are rejected/ignored and the status bar says so.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_modern_style
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, Appearance, BackdropType, BoxSizer, Button, CheckBox, Frame, RadioBox, StaticText,
    StatusBar, WindowCornerPreference,
};

fn main() {
    let app = App::new();

    // `with_modern_style()` applies dark-titlebar + Mica + rounded
    // corners as soon as the HWND exists.
    let frame = Frame::builder()
        .with_title("Minitest — Windows 11 modern style")
        .with_size(560, 420)
        .with_modern_style()
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Modern style applied at startup.", 0);
    status.set_status_text(
        if Appearance::system_is_dark() {
            "OS theme: dark"
        } else {
            "OS theme: light"
        },
        1,
    );

    let _hint = StaticText::new(
        &frame,
        "Toggle the DWM attributes below — changes are immediate.",
    );

    // ── Dark title bar ───────────────────────────────────────────────
    let chk_dark = CheckBox::new(&frame, "Dark title bar");
    chk_dark.set_checked(Appearance::system_is_dark());
    let frame_for_dark = frame.clone();
    let s = status.clone();
    let chk_dark_cb = chk_dark.clone();
    chk_dark.on_toggle(&frame, move || {
        let want = chk_dark_cb.is_checked();
        let ok = frame_for_dark.set_dark_title_bar(want);
        let read_back = frame_for_dark.dark_title_bar();
        s.set_status_text(
            &format!("set_dark_title_bar({want}) -> {ok}, read back: {read_back:?}"),
            0,
        );
    });

    // ── Backdrop material ────────────────────────────────────────────
    let backdrops = [
        ("Auto", BackdropType::Auto),
        ("None", BackdropType::None),
        ("Mica", BackdropType::Mica),
        ("Acrylic", BackdropType::Acrylic),
        ("Mica Alt", BackdropType::MicaAlt),
    ];
    let radio_backdrop = RadioBox::new(
        &frame,
        "Backdrop (Win11 22H2+)",
        &backdrops.map(|(name, _)| name),
    );
    radio_backdrop.set_selection(2); // Mica, applied by with_modern_style()
    let frame_for_bd = frame.clone();
    let s = status.clone();
    radio_backdrop.on_select(&frame, move |idx| {
        if let Some((name, bd)) = backdrops.get(idx) {
            let ok = frame_for_bd.set_backdrop(*bd);
            s.set_status_text(
                &format!(
                    "set_backdrop({name}) -> {}",
                    if ok { "accepted" } else { "rejected (pre-22H2 Windows)" }
                ),
                0,
            );
        }
    });

    // ── Corner preference ────────────────────────────────────────────
    let corners = [
        ("Default", WindowCornerPreference::Default),
        ("DoNotRound", WindowCornerPreference::DoNotRound),
        ("Round", WindowCornerPreference::Round),
        ("RoundSmall", WindowCornerPreference::RoundSmall),
    ];
    let radio_corners = RadioBox::new(&frame, "Corners (Win11)", &corners.map(|(name, _)| name));
    radio_corners.set_selection(0);
    let frame_for_corner = frame.clone();
    let s = status.clone();
    radio_corners.on_select(&frame, move |idx| {
        if let Some((name, pref)) = corners.get(idx) {
            let ok = frame_for_corner.set_corner_preference(*pref);
            let read_back = frame_for_corner.corner_preference();
            s.set_status_text(
                &format!("set_corner_preference({name}) -> {ok}, read back: {read_back:?}"),
                0,
            );
        }
    });

    // ── Re-apply everything ──────────────────────────────────────────
    let btn_reapply = Button::new(&frame, "Re-apply full modern style");
    let frame_for_apply = frame.clone();
    let s = status.clone();
    btn_reapply.on_click(&frame, move || {
        let ok = frame_for_apply.apply_modern_style();
        s.set_status_text(
            &format!(
                "apply_modern_style() -> {}",
                if ok { "ok" } else { "not supported on this OS" }
            ),
            0,
        );
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    sizer.add(chk_dark.as_widget_ref());
    sizer.add(radio_backdrop.as_widget_ref());
    sizer.add(radio_corners.as_widget_ref());
    sizer.add(btn_reapply.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
