//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `MDIParentFrame` and `MDIChildFrame` — multiple-document
//! interface (`wxMDIParentFrame` / `wxMDIChildFrame`).
//!
//! Demonstrates:
//! - Creating an `MDIParentFrame` as a self-contained top-level
//!   window.
//! - Adding several `MDIChildFrame` children via `add_child`.
//! - Cascading, tiling (horizontal and vertical), and closing all
//!   children from buttons on the launcher.
//! - Maximising, restoring, and activating individual children.
//!
//! Note: MDI child frames are **not** `Frame`s, so they cannot host
//! child widgets like `Button` (the `Window` trait isn't implemented
//! for them). The control bar therefore lives on the launcher frame
//! and operates on the MDI parent + children through a thread-local
//! handle. In a real application the controls would be wired to a
//! menu bar attached to the MDI parent (the common wxWidgets
//! pattern).
//!
//! Run with:
//! ```bash
//! cargo run --example mt_mdi
//! ```

#![windows_subsystem = "windows"]

use std::cell::RefCell;

use ru_wx::{App, BoxSizer, Button, Frame, MDIChildFrame, MDIParentFrame, StaticText, StatusBar};

thread_local! {
    /// Holds the live MDI parent + child handles so the launcher
    /// control buttons can operate on them after the "Open" click
    /// returns.
    static MDI: RefCell<Option<(MDIParentFrame, MDIChildFrame, MDIChildFrame)>> =
        const { RefCell::new(None) };
}

fn main() {
    let app = App::new();

    // Launcher / control-panel frame. All MDI operations are wired
    // here because `MDIChildFrame` doesn't implement the `Window`
    // trait and therefore cannot host child buttons.
    let launcher = Frame::builder()
        .with_title("Minitest — MDI control panel")
        .with_size(420, 360)
        .with_modern_style().build();
    let status = StatusBar::new(&launcher, 1);
    status.set_status_text("Click 'Open MDI parent' to spawn an MDI window.", 0);

    let hint = StaticText::new(
        &launcher,
        "Use the buttons below to open the MDI parent and operate\non its children. The control bar is a *separate* frame\nbecause MDI children cannot host child widgets.",
    );

    // ── Open MDI parent ────────────────────────────────────────────
    let status_for_open = status.clone();
    let btn_open = Button::new(&launcher, "Open MDI parent");
    btn_open.on_click(&launcher, move || {
        // Replace any previously opened MDI parent so we do not leak HWNDs.
        MDI.with(|cell| {
            if let Some((old, _, _)) = cell.borrow_mut().take() {
                old.destroy();
            }
        });

        // Build the parent — a self-contained top-level MDI host.
        let mdi = MDIParentFrame::new(None, "Minitest — MDI parent", 720, 480);

        // Track the children for individual operations.
        let child_a = mdi.add_child("Doc A", 0, 0, 280, 200);
        let child_b = mdi.add_child("Doc B", 60, 40, 280, 200);
        // The 3rd child is just here so the user sees the cascade / tile
        // effects on more than 2 windows — the per-child operations below
        // only need `child_a` and `child_b`.
        let _child_c = mdi.add_child("Doc C", 120, 80, 280, 200);
        let _bar = mdi.add_child("Controls", 0, 280, 700, 160);

        // Stash the MDI + 2 children for the per-child buttons.
        MDI.with(|cell| cell.borrow_mut().replace((mdi, child_a, child_b)));
        status_for_open.set_status_text("MDI parent opened with 4 children.", 0);
    });

    // ── Cascade / tile / close-all ──────────────────────────────────
    let status_for_cascade = status.clone();
    let btn_cascade = Button::new(&launcher, "Cascade children");
    btn_cascade.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((mdi, _, _)) = cell.borrow_mut().as_mut() {
                mdi.cascade_children();
                status_for_cascade.set_status_text("Cascade: done", 0);
            } else {
                status_for_cascade.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    let status_for_tile_v = status.clone();
    let btn_tile_v = Button::new(&launcher, "Tile vertically");
    btn_tile_v.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((mdi, _, _)) = cell.borrow_mut().as_mut() {
                mdi.tile_children(false);
                status_for_tile_v.set_status_text("Tile V: done", 0);
            } else {
                status_for_tile_v.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    let status_for_tile_h = status.clone();
    let btn_tile_h = Button::new(&launcher, "Tile horizontally");
    btn_tile_h.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((mdi, _, _)) = cell.borrow_mut().as_mut() {
                mdi.tile_children(true);
                status_for_tile_h.set_status_text("Tile H: done", 0);
            } else {
                status_for_tile_h.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    let status_for_close_all = status.clone();
    let btn_close_all = Button::new(&launcher, "Close all children");
    btn_close_all.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((mdi, _, _)) = cell.borrow_mut().as_mut() {
                mdi.close_all_children();
                status_for_close_all.set_status_text("Close all: done", 0);
            } else {
                status_for_close_all.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    // ── Per-child controls ─────────────────────────────────────────
    let status_for_max = status.clone();
    let btn_max = Button::new(&launcher, "Maximise 'Doc A'");
    btn_max.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((_, child_a, _)) = cell.borrow_mut().as_mut() {
                child_a.maximize();
                status_for_max.set_status_text("Maximised 'Doc A'", 0);
            } else {
                status_for_max.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    let status_for_restore = status.clone();
    let btn_restore = Button::new(&launcher, "Restore 'Doc A'");
    btn_restore.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((_, child_a, _)) = cell.borrow_mut().as_mut() {
                child_a.restore();
                status_for_restore.set_status_text("Restored 'Doc A'", 0);
            } else {
                status_for_restore.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    let status_for_activate = status.clone();
    let btn_activate = Button::new(&launcher, "Activate 'Doc B'");
    btn_activate.on_click(&launcher, move || {
        MDI.with(|cell| {
            if let Some((_, _, child_b)) = cell.borrow_mut().as_mut() {
                child_b.activate();
                status_for_activate.set_status_text("Activated 'Doc B'", 0);
            } else {
                status_for_activate.set_status_text("(open the MDI parent first)", 0);
            }
        });
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(btn_open.as_widget_ref());
    sizer.add(btn_cascade.as_widget_ref());
    sizer.add(btn_tile_v.as_widget_ref());
    sizer.add(btn_tile_h.as_widget_ref());
    sizer.add(btn_close_all.as_widget_ref());
    sizer.add(btn_max.as_widget_ref());
    sizer.add(btn_restore.as_widget_ref());
    sizer.add(btn_activate.as_widget_ref());
    launcher.set_sizer(sizer);

    app.run(launcher);
}
