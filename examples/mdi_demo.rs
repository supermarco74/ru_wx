//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! MDI Demo - Multiple Document Interface completo per macOS
//!
//! Mostra MDIParentFrame con child windows e controlli MDI.
//! Nota: I controlli sono nel pannello di controllo separato perche'
//! MDIChildFrame non puo' avere widget figli.
//!
//! Run with:
//! ```bash
//! cargo run --example mdi_demo
//! ```

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::cell::RefCell;

use ru_wx::{App, BoxSizer, Button, Frame, MDIChildFrame, MDIParentFrame, StaticText};

thread_local! {
    static MDI_STATE: RefCell<Option<(MDIParentFrame, Vec<MDIChildFrame>)>> =
        const { RefCell::new(None) };
}

fn main() {
    let app = App::new();

    // ── Control Panel ─────────────────────────────────────────────
    let control_panel = Frame::builder()
        .with_title("MDI Demo Controls")
        .with_size(320, 520)
        .build();

    // Hint text
    let hint = StaticText::new(
        &control_panel,
        "MDI Demo\n\nOpen MDI window and arrange\nchildren with buttons below.",
    );

    // ── Open MDI ─────────────────────────────────────────────────
    let btn_open = Button::new(&control_panel, "Open MDI Window");
    btn_open.on_click(&control_panel, move || {
        // Close existing MDI
        MDI_STATE.with(|cell| {
            if let Some((old_mdi, _)) = cell.borrow_mut().take() {
                old_mdi.destroy();
            }
        });

        // Create MDI parent
        let mdi = MDIParentFrame::new(None, "MDI Children", 900, 650);

        // Create child windows
        let mut children: Vec<MDIChildFrame> = Vec::new();

        let child1 = mdi.add_child("Document 1 - Notes", 20, 20, 380, 260);
        children.push(child1);

        let child2 = mdi.add_child("Document 2 - Source", 60, 60, 380, 260);
        children.push(child2);

        let child3 = mdi.add_child("Document 3 - Docs", 100, 100, 380, 260);
        children.push(child3);

        let child4 = mdi.add_child("Document 4 - Output", 140, 140, 380, 260);
        children.push(child4);

        let child5 = mdi.add_child("Document 5 - Debug", 180, 180, 380, 260);
        children.push(child5);

        // Store state
        MDI_STATE.with(|cell| cell.borrow_mut().replace((mdi, children.clone())));
    });

    // ── MDI Operations ────────────────────────────────────────────
    let btn_cascade = Button::new(&control_panel, "Cascade");
    btn_cascade.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((mdi, _)) = cell.borrow_mut().as_mut() {
                mdi.cascade_children();
            }
        });
    });

    let btn_tile_v = Button::new(&control_panel, "Tile Vertical");
    btn_tile_v.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((mdi, _)) = cell.borrow_mut().as_mut() {
                mdi.tile_children(false);
            }
        });
    });

    let btn_tile_h = Button::new(&control_panel, "Tile Horizontal");
    btn_tile_h.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((mdi, _)) = cell.borrow_mut().as_mut() {
                mdi.tile_children(true);
            }
        });
    });

    let btn_close = Button::new(&control_panel, "Close All Children");
    btn_close.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((mdi, _)) = cell.borrow_mut().as_mut() {
                mdi.close_all_children();
            }
        });
    });

    // ── Per-Child Operations ─────────────────────────────────────
    let btn_max = Button::new(&control_panel, "Maximize Doc 1");
    btn_max.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((_, ref children)) = cell.borrow_mut().as_mut() {
                if !children.is_empty() {
                    children[0].maximize();
                }
            }
        });
    });

    let btn_restore = Button::new(&control_panel, "Restore Doc 1");
    btn_restore.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((_, ref children)) = cell.borrow_mut().as_mut() {
                if !children.is_empty() {
                    children[0].restore();
                }
            }
        });
    });

    let btn_activate = Button::new(&control_panel, "Activate Doc 2");
    btn_activate.on_click(&control_panel, move || {
        MDI_STATE.with(|cell| {
            if let Some((_, ref children)) = cell.borrow_mut().as_mut() {
                if children.len() > 1 {
                    children[1].activate();
                }
            }
        });
    });

    // ── Layout ──────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(btn_open.as_widget_ref());
    sizer.add(btn_cascade.as_widget_ref());
    sizer.add(btn_tile_v.as_widget_ref());
    sizer.add(btn_tile_h.as_widget_ref());
    sizer.add(btn_close.as_widget_ref());
    sizer.add(btn_max.as_widget_ref());
    sizer.add(btn_restore.as_widget_ref());
    sizer.add(btn_activate.as_widget_ref());

    control_panel.set_sizer(sizer);

    app.run(control_panel);
}
