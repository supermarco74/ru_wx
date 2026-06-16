//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `GLCanvas` — minimal OpenGL rendering surface.
//!
//! Demonstrates:
//! 1. Creating a `GLCanvas` of a given pixel size with
//!    [`GLCanvas::with_size`].
//! 2. Binding the GL context to the calling thread with
//!    [`GLCanvas::set_current`].
//! 3. Setting up a 2D orthographic projection and clearing the
//!    color buffer on each frame.
//! 4. Drawing a simple animated triangle (rotating) with
//!    OpenGL 1.1 fixed-function calls re-exported via
//!    [`ru_wx::gl11`].
//! 5. Swapping front/back buffers with [`GLCanvas::swap_buffers`].
//! 6. A `Timer`-driven animation loop and a "Pause / Resume"
//!    `Button` that toggles the timer without recreating the
//!    canvas.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_gl_canvas
//! ```
//!
//! # Notes
//!
//! * The demo uses the OpenGL 1.1 fixed-function pipeline. This is
//!   the highest version `windows-sys` re-exports directly; for
//!   modern OpenGL (2.0+ with shaders / VBOs) you would need to
//!   load entry points at runtime via `wglGetProcAddress`.
//! * On non-Windows targets `GLCanvas` is a no-op stub: this
//!   example still compiles, but the timer fires against an
//!   inert widget.

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

#[cfg(target_os = "windows")]
use ru_wx::gl11::{
    glBegin, glClear, glClearColor, glColor3f, glEnd, glLoadIdentity, glMatrixMode,
    glRotatef, glVertex2f, glViewport, GL_COLOR_BUFFER_BIT, GL_MODELVIEW, GL_TRIANGLES,
};
use ru_wx::{App, BoxSizer, Button, Frame, GLCanvas, StaticText, Timer};

/// Background RGB used for `glClearColor`. We toggle this with
/// the "Toggle background" button.
const BG_DARK: (f32, f32, f32) = (0.08, 0.10, 0.16);
const BG_LIGHT: (f32, f32, f32) = (0.92, 0.90, 0.84);

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — GLCanvas (OpenGL 1.1)")
        .with_size(520, 460)
        .build();

    // ── Status / info labels ──────────────────────────────────────
    let info = StaticText::new(
        &frame,
        "OpenGL canvas — animated rotating triangle on a 2D ortho projection.",
    );

    // ── Animation state shared between the timer and the buttons ─
    // `angle_deg` is updated by the timer tick.
    // `paused` is toggled by the pause/resume button.
    // `use_dark_bg` is toggled by the bg button.
    let angle_deg: Rc<Cell<f32>> = Rc::new(Cell::new(0.0));
    let paused: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let use_dark_bg: Rc<Cell<bool>> = Rc::new(Cell::new(true));

    // ── The OpenGL canvas ─────────────────────────────────────────
    let gl = GLCanvas::with_size(&frame, 480, 320);

    // One-time GL setup. We need the context current before any
    // `gl*` call. On a non-Windows build this returns `false`,
    // and the `gl*` calls become inert (the `gl11` re-exports
    // under `cfg(target_os = "windows")` are not available
    // elsewhere, so we guard the GL setup with `cfg`).
    #[cfg(target_os = "windows")]
    {
        if !gl.set_current() {
            info.set_label("Failed to bind GL context — the GL demo is inert on this system.");
        } else {
            // Use a 2D ortho projection so we can draw in pixel-like
            // coordinates. We set the viewport to the canvas size
            // and pick a projection matching that size in pixels.
            let (w, h) = {
                let r = gl.as_widget_ref().borrow().rect();
                (r.width.max(1) as i32, r.height.max(1) as i32)
            };
            unsafe {
                glViewport(0, 0, w, h);
                glMatrixMode(GL_MODELVIEW);
                glLoadIdentity();
            }
            // Pick the initial clear color.
            let bg = if use_dark_bg.get() { BG_DARK } else { BG_LIGHT };
            unsafe {
                glClearColor(bg.0, bg.1, bg.2, 1.0);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        info.set_label("GLCanvas is a no-op stub on this platform.");
    }

    // ── Animation tick ────────────────────────────────────────────
    let timer = Timer::new(&frame);
    let gl_for_tick = gl.clone();
    let angle_for_tick = angle_deg.clone();
    let paused_for_tick = paused.clone();
    let use_dark_bg_for_tick = use_dark_bg.clone();
    timer.on_tick(move || {
        if paused_for_tick.get() {
            return;
        }
        // Advance the angle. ~60° per tick at 50ms ≈ 20°/s; we
        // pick 50ms (~20Hz) which is plenty smooth and leaves
        // CPU budget for the rest of the UI.
        let a = angle_for_tick.get() + 2.0;
        angle_for_tick.set(a);

        #[cfg(target_os = "windows")]
        {
            if !gl_for_tick.set_current() {
                return;
            }
            let bg = if use_dark_bg_for_tick.get() {
                BG_DARK
            } else {
                BG_LIGHT
            };
            // Draw a rotating triangle in the centre of the
            // canvas. We use a 200×200 unit space centred on the
            // canvas; `glRotatef` rotates the whole modelview
            // matrix.
            unsafe {
                glClearColor(bg.0, bg.1, bg.2, 1.0);
                glClear(GL_COLOR_BUFFER_BIT);
                glLoadIdentity();
                // Center the model around (0, 0) and rotate.
                glRotatef(a, 0.0, 0.0, 1.0);
                glBegin(GL_TRIANGLES);
                // Vertices form an equilateral-ish triangle
                // about 80 units from the origin.
                glColor3f(1.0, 0.4, 0.2); // red
                glVertex2f(0.0, 80.0);
                glColor3f(0.2, 1.0, 0.4); // green
                glVertex2f(-70.0, -60.0);
                glColor3f(0.3, 0.5, 1.0); // blue
                glVertex2f(70.0, -60.0);
                glEnd();
            }
            gl_for_tick.swap_buffers();
        }
    });
    timer.start(Duration::from_millis(50));

    // ── Pause / resume button ─────────────────────────────────────
    let pause_btn = Button::new(&frame, "Pause");
    let paused_for_btn = paused.clone();
    let pause_label_for_btn = pause_btn.clone();
    pause_btn.on_click(&frame, move || {
        paused_for_btn.set(!paused_for_btn.get());
        pause_label_for_btn.set_label(if paused_for_btn.get() {
            "Resume"
        } else {
            "Pause"
        });
    });

    // ── Toggle background button ──────────────────────────────────
    let bg_btn = Button::new(&frame, "Toggle background");
    let use_dark_bg_for_btn = use_dark_bg.clone();
    bg_btn.on_click(&frame, move || {
        use_dark_bg_for_btn.set(!use_dark_bg_for_btn.get());
    });

    // ── Reset angle button ────────────────────────────────────────
    let reset_btn = Button::new(&frame, "Reset angle");
    let angle_for_btn = angle_deg.clone();
    reset_btn.on_click(&frame, move || {
        angle_for_btn.set(0.0);
    });

    // ── Status readout ────────────────────────────────────────────
    let status = StaticText::new(&frame, "angle: 0°");

    // A second timer (slow, 100ms) just to keep the angle label
    // visible to the user without spamming repaints.
    let readout = Timer::new(&frame);
    let status_for_tick = status.clone();
    let angle_for_readout = angle_deg.clone();
    let paused_for_readout = paused.clone();
    readout.on_tick(move || {
        status_for_tick.set_label(&format!(
            "angle: {:.0}°   {}",
            angle_for_readout.get(),
            if paused_for_readout.get() {
                "(paused)"
            } else {
                ""
            }
        ));
    });
    readout.start(Duration::from_millis(100));

    // ── Layout ───────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(info.as_widget_ref());
    sizer.add(gl.as_widget_ref());
    sizer.add(status.as_widget_ref());
    sizer.add(pause_btn.as_widget_ref());
    sizer.add(bg_btn.as_widget_ref());
    sizer.add(reset_btn.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
