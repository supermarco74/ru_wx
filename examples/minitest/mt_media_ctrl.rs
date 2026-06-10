//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `MediaCtrl` — MCI-based audio / video playback.
//!
//! Demonstrates:
//! 1. Creating a headless `MediaCtrl` (no own HWND — MCI plays into
//!    the default audio / video device).
//! 2. Loading a media file from disk with [`MediaCtrl::load`].
//! 3. Play / Pause / Stop / Seek controls driven by ordinary
//!    `Button`s.
//! 4. Reading back state and position via [`MediaCtrl::state`],
//!    [`MediaCtrl::position_ms`], [`MediaCtrl::length_ms`].
//! 5. A graceful "no file found" state: the demo still runs and the
//!    UI stays responsive, even when no media asset is on disk.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_media_ctrl
//! ```
//!
//! # Asset
//!
//! The example looks for a small WAV or MP3 in `assets/` and
//! `../assets/`. If none is present, the UI shows a "no file"
//! message and the playback buttons are no-ops (or return
//! `Err("no media loaded")` from the inner API).

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use ru_wx::{
    App, BoxSizer, Button, FileDialog, FileDialogStyle, Frame, MediaCtrl, MediaState, StaticText,
    Timer,
};

/// Look for a media file we can play. We try a few common names
/// and a few locations. Returning `None` is fine — the demo will
/// show a "no file" message and the user can still click around
/// without crashing.
fn locate_media() -> Option<PathBuf> {
    let candidates = [
        "assets/media/sample.wav",
        "assets/media/sample.mp3",
        "../assets/media/sample.wav",
        "../assets/media/sample.mp3",
        "sample.wav",
        "sample.mp3",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn state_label(s: MediaState) -> &'static str {
    match s {
        MediaState::Stopped => "stopped",
        MediaState::Paused => "paused",
        MediaState::Playing => "playing",
    }
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — MediaCtrl (MCI)")
        .with_size(440, 280)
        .with_modern_style().build();

    // ── Status / info labels ──────────────────────────────────────
    let info = StaticText::new(&frame, "loading…");
    let state_label_widget = StaticText::new(&frame, "state: stopped");
    let pos_label = StaticText::new(&frame, "position: -");

    // ── The media control ─────────────────────────────────────────
    let media = MediaCtrl::new(&frame);
    let media: Rc<RefCell<MediaCtrl>> = Rc::new(RefCell::new(media));

    // Try to load a media file from one of the candidate paths.
    if let Some(path) = locate_media() {
        let m = media.borrow();
        match m.load(&path) {
            Ok(()) => {
                info.set_label(&format!("Loaded: {}", path.display()));
            }
            Err(e) => {
                info.set_label(&format!("MCI load error: {e}"));
            }
        }
    } else {
        info.set_label(
            "No media file found in assets/media/ — place a sample.wav or sample.mp3 there to test playback.",
        );
    }

    // ── File picker button (uses wxWidgets-style FileDialog) ──────
    let pick_btn = Button::new(&frame, "Load media file…");
    let frame_for_pick = frame.clone();
    let media_for_pick = media.clone();
    let info_for_pick = info.clone();
    pick_btn.on_click(&frame, move || {
        let mut dlg = FileDialog::new(&frame_for_pick, FileDialogStyle::Open);
        dlg.set_title("Choose a media file");
        dlg.set_wildcard(
            "Audio / video files|*.wav;*.mp3;*.mid;*.avi;*.mpg;*.mpeg;*.wma|\
             WAV (*.wav)|*.wav|\
             MP3 (*.mp3)|*.mp3|\
             MIDI (*.mid)|*.mid|\
             AVI (*.avi)|*.avi|\
             MPEG (*.mpg)|*.mpg|\
             All files (*.*)|*.*",
        );
        if let Some(path) = dlg.show_modal() {
            let m = media_for_pick.borrow();
            match m.load(std::path::Path::new(&path)) {
                Ok(()) => info_for_pick.set_label(&format!("Loaded: {path}")),
                Err(e) => info_for_pick.set_label(&format!("MCI load error: {e}")),
            }
        }
    });

    // ── Play / Pause / Stop / Seek buttons ────────────────────────
    let play_btn = Button::new(&frame, "Play");
    let pause_btn = Button::new(&frame, "Pause");
    let stop_btn = Button::new(&frame, "Stop");
    let rewind_btn = Button::new(&frame, "Rewind to start");

    let media_for_play = media.clone();
    let state_for_play = state_label_widget.clone();
    play_btn.on_click(&frame, move || {
        let m = media_for_play.borrow();
        if let Err(e) = m.play() {
            state_for_play.set_label(&format!("play: {e}"));
        }
    });
    let media_for_pause = media.clone();
    let state_for_pause = state_label_widget.clone();
    pause_btn.on_click(&frame, move || {
        let m = media_for_pause.borrow();
        if let Err(e) = m.pause() {
            state_for_pause.set_label(&format!("pause: {e}"));
        }
    });
    let media_for_stop = media.clone();
    let state_for_stop = state_label_widget.clone();
    stop_btn.on_click(&frame, move || {
        let m = media_for_stop.borrow();
        if let Err(e) = m.stop() {
            state_for_stop.set_label(&format!("stop: {e}"));
        }
    });
    let media_for_rewind = media.clone();
    let state_for_rewind = state_label_widget.clone();
    rewind_btn.on_click(&frame, move || {
        let m = media_for_rewind.borrow();
        if let Err(e) = m.seek_ms(0) {
            state_for_rewind.set_label(&format!("seek: {e}"));
        }
    });

    // ── Refresh timer — keeps the state / position labels in sync ─
    let refresh = Timer::new(&frame);
    let media_for_tick = media.clone();
    let state_for_tick = state_label_widget.clone();
    let pos_for_tick = pos_label.clone();
    refresh.on_tick(move || {
        let m = media_for_tick.borrow();
        state_for_tick.set_label(&format!("state: {}", state_label(m.state())));
        match (m.position_ms(), m.length_ms()) {
            (Some(p), Some(l)) => {
                pos_for_tick.set_label(&format!("position: {} ms / {} ms", p, l));
            }
            (Some(p), None) => {
                pos_for_tick.set_label(&format!("position: {} ms", p));
            }
            _ => {
                pos_for_tick.set_label("position: -");
            }
        }
    });
    refresh.start(Duration::from_millis(200));

    // ── Layout ───────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(info.as_widget_ref());
    sizer.add(pick_btn.as_widget_ref());
    sizer.add(state_label_widget.as_widget_ref());
    sizer.add(pos_label.as_widget_ref());
    sizer.add(play_btn.as_widget_ref());
    sizer.add(pause_btn.as_widget_ref());
    sizer.add(stop_btn.as_widget_ref());
    sizer.add(rewind_btn.as_widget_ref());
    frame.set_sizer(sizer);

    // Keep a strong reference to the media control alive for the
    // whole lifetime of the window. The cloned `Rc` inside the
    // closures would otherwise be the only owners; that's fine
    // too, but being explicit here makes the lifetime obvious.
    let _media_ref = media;

    app.run(frame);
}
