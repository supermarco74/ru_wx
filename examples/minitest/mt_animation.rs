//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Animation` / `AnimationCtrl` — GIF playback.
//!
//! Demonstrates:
//! 1. A multi-frame animated GIF **generated at runtime** (an
//!    orbiting coloured ball, encoded with the `image` crate) and
//!    loaded via [`Animation::load_from_memory`] — no asset files
//!    needed.
//! 2. Two `AnimationCtrl`s bound to the *same* animation data: one
//!    at the natural size, one stretched (StretchBlt scaling).
//! 3. Play / Stop / Restart buttons driving both controls.
//! 4. Live status in a `StatusBar`: playback state in field 0,
//!    frame index / count in field 1 (refreshed by a [`Timer`]).
//!
//! Run with:
//! ```bash
//! cargo run --example mt_animation
//! ```

#![windows_subsystem = "windows"]

use std::time::Duration;

use ru_wx::{Animation, AnimationCtrl, App, BoxSizer, Button, Frame, StaticText, StatusBar, Timer};

const SIDE: u32 = 96;
const FRAMES: u32 = 12;

/// Encode a small animated GIF entirely in memory: a coloured ball
/// orbiting the centre over a dark vignette background. Each frame
/// shifts the hue so playback is clearly visible.
fn build_gif_bytes() -> Vec<u8> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame, Rgba, RgbaImage};

    // Simple colour wheel for the ball: one (r, g, b) per frame.
    let palette: [(u8, u8, u8); FRAMES as usize] = [
        (230, 70, 60),
        (240, 130, 40),
        (245, 190, 40),
        (170, 210, 60),
        (90, 190, 90),
        (60, 200, 170),
        (60, 170, 220),
        (70, 120, 230),
        (120, 90, 230),
        (180, 80, 220),
        (225, 70, 180),
        (235, 70, 110),
    ];

    let centre = SIDE as f32 / 2.0;
    let orbit = SIDE as f32 * 0.30;
    let radius = SIDE as f32 * 0.13;

    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        let _ = encoder.set_repeat(Repeat::Infinite);
        for i in 0..FRAMES {
            let angle = i as f32 / FRAMES as f32 * std::f32::consts::TAU;
            let bx = centre + orbit * angle.cos();
            let by = centre + orbit * angle.sin();
            let (br, bg, bb) = palette[i as usize];
            let img = RgbaImage::from_fn(SIDE, SIDE, |x, y| {
                let dx = x as f32 - bx;
                let dy = y as f32 - by;
                if dx * dx + dy * dy <= radius * radius {
                    Rgba([br, bg, bb, 255])
                } else {
                    // Background: radial vignette around the centre.
                    let cx = x as f32 - centre;
                    let cy = y as f32 - centre;
                    let d = (cx * cx + cy * cy).sqrt() / centre;
                    let shade = (40.0 + 50.0 * (1.0 - d.min(1.0))) as u8;
                    Rgba([shade / 2, shade / 2, shade, 255])
                }
            });
            let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(80, 1));
            let _ = encoder.encode_frame(frame);
        }
    }
    bytes
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Animation / AnimationCtrl")
        .with_size(480, 420)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("idle", 0);

    // ── The animation data: a GIF generated in memory ─────────────
    let gif = build_gif_bytes();
    let mut anim = Animation::new();
    let info = StaticText::new(&frame, "building…");
    if anim.load_from_memory(&gif).is_ok() && anim.is_loaded() {
        let (w, h) = anim.size();
        info.set_label(&format!(
            "Runtime GIF: {}×{} px, {} frames, {} bytes encoded",
            w,
            h,
            anim.frame_count(),
            gif.len()
        ));
    } else {
        info.set_label("GIF encode/decode failed — controls stay empty");
    }

    // ── Two controls sharing the same data ────────────────────────
    let hint = StaticText::new(&frame, "Natural size (left) vs stretched (right):");
    let ctrl_native = AnimationCtrl::with_size(&frame, SIDE, SIDE);
    ctrl_native.set_animation(anim.clone());
    let ctrl_big = AnimationCtrl::with_size(&frame, SIDE * 2, SIDE * 2);
    ctrl_big.set_animation(anim.clone());

    let mut row_anim = BoxSizer::horizontal();
    row_anim.add(ctrl_native.as_widget_ref());
    row_anim.add_spacer(16);
    row_anim.add(ctrl_big.as_widget_ref());

    // ── Buttons drive BOTH controls ───────────────────────────────
    let play_btn = Button::new(&frame, "Play");
    let stop_btn = Button::new(&frame, "Stop");
    let reset_btn = Button::new(&frame, "Restart");

    let (c1, c2, s) = (ctrl_native.clone(), ctrl_big.clone(), status.clone());
    play_btn.on_click(&frame, move || {
        c1.play();
        c2.play();
        s.set_status_text("playing", 0);
    });
    let (c1, c2, s) = (ctrl_native.clone(), ctrl_big.clone(), status.clone());
    stop_btn.on_click(&frame, move || {
        c1.stop();
        c2.stop();
        s.set_status_text("stopped", 0);
    });
    let (c1, c2, s) = (ctrl_native.clone(), ctrl_big.clone(), status.clone());
    reset_btn.on_click(&frame, move || {
        c1.stop();
        c2.stop();
        c1.play();
        c2.play();
        s.set_status_text("restarted", 0);
    });

    let mut row_buttons = BoxSizer::horizontal();
    row_buttons.add(play_btn.as_widget_ref());
    row_buttons.add(stop_btn.as_widget_ref());
    row_buttons.add(reset_btn.as_widget_ref());

    // ── Timer keeps the status bar frame counter in sync ──────────
    let refresh = Timer::new(&frame);
    let ctrl_for_tick = ctrl_native.clone();
    let status_for_tick = status.clone();
    let total = anim.frame_count();
    refresh.on_tick(move || {
        status_for_tick.set_status_text(
            &format!(
                "frame {} / {} — {}",
                ctrl_for_tick.current_frame() + 1,
                total,
                if ctrl_for_tick.is_playing() { "running" } else { "paused" }
            ),
            1,
        );
    });
    refresh.start(Duration::from_millis(100));

    // ── Layout ────────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(info.as_widget_ref());
    sizer.add(hint.as_widget_ref());
    sizer.add_sizer(row_anim);
    sizer.add_spacer(8);
    sizer.add_sizer(row_buttons);
    frame.set_sizer(sizer);

    app.run(frame);
}
