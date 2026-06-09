//! Minitest: `Animation` / `AnimationCtrl` — GIF playback.
//!
//! Demonstrates:
//! 1. A `wxAnimation` data container loaded from an asset file
//!    (GIF on disk).
//! 2. An `AnimationCtrl` widget bound to the loaded animation.
//! 3. Play / Stop / Restart controls driven by ordinary
//!    `Button`s.
//! 4. Frame-info readout (current frame index, frame count).
//! 5. A fallback "static image" animation built from a PNG, so
//!    the demo works even when no GIF asset is present.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_animation
//! ```

#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::time::Duration;

use ru_wx::{Animation, AnimationCtrl, App, BoxSizer, Button, Frame, StaticText, Timer};

fn locate_gif() -> Option<PathBuf> {
    // Look for a GIF first in the assets folder, then next to
    // the executable. We don't fail hard if it isn't there: the
    // demo still works using a static PNG fallback below.
    let candidates = [
        "assets/icons/anim_sample.gif",
        "../assets/icons/anim_sample.gif",
        "anim_sample.gif",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Animation / AnimationCtrl")
        .with_size(420, 320)
        .build();

    // ── Status / info labels ──────────────────────────────────────
    let info = StaticText::new(&frame, "loading…");
    let frame_info = StaticText::new(&frame, "frame: 0 / 0");
    let status = StaticText::new(&frame, "idle");

    // ── The animation data ───────────────────────────────────────
    let mut anim = Animation::new();
    if let Some(path) = locate_gif() {
        if anim.load_file(&path).is_ok() && anim.is_loaded() {
            let (w, h) = anim.size();
            info.set_label(&format!(
                "Loaded GIF: {} ({}×{}, {} frames)",
                path.display(),
                w,
                h,
                anim.frame_count()
            ));
        } else {
            info.set_label("Failed to decode GIF — using a static fallback");
        }
    } else {
        info.set_label("No GIF found in assets — using a static fallback");
    }

    // Fallback: a 1-frame animation built from inline PNG bytes
    // so the demo still has *something* to show.
    if !anim.is_loaded() {
        let png = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x40, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x4B, 0x6D, 0x29, 0xDC, 0x00, 0x00, 0x00, 0x16, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0xED, 0xC1, 0x01, 0x0D, 0x00, 0x00, 0x00, 0xC2, 0xA0, 0xF7, 0x4F, 0x6D, 0x0E,
            0x37, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0xB7, 0x01, 0x4B, 0x6D, 0x0E,
            0x37, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x1B, 0xC1, 0x66, 0x00, 0x01, 0x6A,
            0xE0, 0x9C, 0xF1, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60,
            0x82,
        ];
        let _ = anim.load_from_memory(&png);
    }

    // ── The widget ────────────────────────────────────────────────
    let (anim_w, anim_h) = anim.size();
    let ctrl = AnimationCtrl::with_size(&frame, anim_w.max(64), anim_h.max(64));
    ctrl.set_animation(anim.clone());

    // ── Buttons ──────────────────────────────────────────────────
    let play_btn = Button::new(&frame, "Play");
    let stop_btn = Button::new(&frame, "Stop");
    let reset_btn = Button::new(&frame, "Restart");

    let ctrl_for_play = ctrl.clone();
    play_btn.on_click(&frame, move || {
        ctrl_for_play.play();
    });
    let ctrl_for_stop = ctrl.clone();
    let status_for_stop = status.clone();
    stop_btn.on_click(&frame, move || {
        ctrl_for_stop.stop();
        status_for_stop.set_label("stopped");
    });
    let ctrl_for_reset = ctrl.clone();
    let status_for_reset = status.clone();
    reset_btn.on_click(&frame, move || {
        ctrl_for_reset.stop();
        ctrl_for_reset.play();
        status_for_reset.set_label("restarted");
    });

    // ── A timer to keep the frame-info label in sync ─────────────
    let refresh = Timer::new(&frame);
    let ctrl_for_tick = ctrl.clone();
    let frame_info_for_tick = frame_info.clone();
    let anim_for_tick = anim.clone();
    refresh.on_tick(move || {
        frame_info_for_tick.set_label(&format!(
            "frame: {} / {}",
            ctrl_for_tick.current_frame(),
            anim_for_tick.frame_count()
        ));
    });
    refresh.start(Duration::from_millis(100));
    let _ = status; // silence unused

    // ── Layout ───────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(info.as_widget_ref());
    sizer.add(ctrl.as_widget_ref());
    sizer.add(frame_info.as_widget_ref());
    sizer.add(play_btn.as_widget_ref());
    sizer.add(stop_btn.as_widget_ref());
    sizer.add(reset_btn.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
