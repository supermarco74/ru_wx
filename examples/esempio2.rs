//! `esempio2` — Mini Editor that combines the **newly created controls**:
//!
//! - [`AuiToolBar`] — a dockable toolbar that can be detached to a
//!   floating window by clicking the `≡` gripper. Re-dock by clicking
//!   the gripper again, double-clicking the floating title bar, or
//!   closing the floating window.
//! - [`ToolTip`] — per-widget tooltips attached to every interactive
//!   control in the window. A `Show tooltips` checkbox toggles the
//!   global enable flag.
//! - [`StaticText`] — labels that describe what each region of the
//!   window does.
//!
//! Plus pre-existing controls used for context:
//! - [`Frame`], [`StatusBar`], [`BoxSizer`], [`TextCtrl`], [`Button`],
//!   [`CheckBox`], [`ImageList`].
//!
//! Run with:
//! ```bash
//! cargo run --example esempio2
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, AuiDockSide, AuiToolBar, BitmapBundle, BoxSizer, Button, CheckBox, Frame, ImageList,
    StaticText, StatusBar, TextCtrl, ToolTip,
};

// ── Inline SVG icons (Bootstrap-Icon-style, 24×24 viewBox) ──────────
const ICON_NEW: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M12 18v-6M9 15h6"/></svg>"#;
const ICON_OPEN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>"#;
const ICON_SAVE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M5 3h11l3 3v15H5z M8 3v6h7V3 M8 14h8v7H8z"/></svg>"#;
const ICON_CUT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M8.12 8.12L20 20 M8.12 15.88L20 4"/></svg>"#;
const ICON_COPY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="8" y="8" width="13" height="13"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/></svg>"#;
const ICON_PASTE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="6" y="4" width="12" height="16"/><path d="M9 4V2h6v2"/></svg>"#;

// Toolbar command ids. We dispatch on these in the click handler.
const ID_TOOL_NEW: u16 = 2001;
const ID_TOOL_OPEN: u16 = 2002;
const ID_TOOL_SAVE: u16 = 2003;
const ID_TOOL_CUT: u16 = 2004;
const ID_TOOL_COPY: u16 = 2005;
const ID_TOOL_PASTE: u16 = 2006;

fn main() {
    let app = App::new();

    let frame = Frame::builder()
        .with_title("esempio2 — Mini Editor (AuiToolBar + ToolTip + StaticText)")
        .with_size(760, 520)
        .build();

    // ── Status bar at the bottom ────────────────────────────────────
    let status = StatusBar::new(&frame, 1);
    status.set_status_text(
        "Hover any control to see its tooltip. Click ≡ to detach the toolbar.",
        0,
    );

    // ── Image list for the AuiToolBar (24×24) ───────────────────────
    let icon_sizes: [(u32, u32); 3] = [(16, 16), (20, 20), (24, 24)];

    let bundle_new = BitmapBundle::from_svg_bytes(ICON_NEW, &icon_sizes);
    let bundle_open = BitmapBundle::from_svg_bytes(ICON_OPEN, &icon_sizes);
    let bundle_save = BitmapBundle::from_svg_bytes(ICON_SAVE, &icon_sizes);
    let bundle_cut = BitmapBundle::from_svg_bytes(ICON_CUT, &icon_sizes);
    let bundle_copy = BitmapBundle::from_svg_bytes(ICON_COPY, &icon_sizes);
    let bundle_paste = BitmapBundle::from_svg_bytes(ICON_PASTE, &icon_sizes);

    let images = ImageList::new(24, 24);
    if let Some(bmp) = bundle_new.best_for_size((24, 24)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_open.best_for_size((24, 24)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_save.best_for_size((24, 24)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_cut.best_for_size((24, 24)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_copy.best_for_size((24, 24)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_paste.best_for_size((24, 24)) {
        images.add_bitmap(bmp.hbitmap);
    }

    // ── The AuiToolBar (NEW CONTROL #1) ─────────────────────────────
    // The AuiToolBar docks itself to the top of the frame automatically;
    // we don't add it to the BoxSizer below — its size is managed
    // independently.
    let aui = AuiToolBar::new(&frame);
    aui.set_image_list(&images);
    aui.add_tool(ID_TOOL_NEW, "New", 0);
    aui.add_tool(ID_TOOL_OPEN, "Open", 1);
    aui.add_tool(ID_TOOL_SAVE, "Save", 2);
    aui.add_separator();
    aui.add_tool(ID_TOOL_CUT, "Cut", 3);
    aui.add_tool(ID_TOOL_COPY, "Copy", 4);
    aui.add_tool(ID_TOOL_PASTE, "Paste", 5);
    aui.realize();

    // Tool click → update the status bar.
    let status_for_tools = status.clone();
    aui.on_tool_clicked(&frame, move |id| {
        let label = match id {
            ID_TOOL_NEW => "New",
            ID_TOOL_OPEN => "Open",
            ID_TOOL_SAVE => "Save",
            ID_TOOL_CUT => "Cut",
            ID_TOOL_COPY => "Copy",
            ID_TOOL_PASTE => "Paste",
            _ => "?",
        };
        status_for_tools.set_status_text(&format!("AuiToolBar → {label}"), 0);
    });

    // Dock-state-change → update the status bar.
    let status_for_dock = status.clone();
    aui.on_dock_state_change(move |side| {
        let label = match side {
            AuiDockSide::Top => "top",
            AuiDockSide::Bottom => "bottom",
            AuiDockSide::Left => "left",
            AuiDockSide::Right => "right",
            AuiDockSide::Floating => "floating",
        };
        status_for_dock.set_status_text(&format!("AuiToolBar is now docked to: {label}"), 0);
    });

    // ── StaticText (NEW CONTROL #2) — labels for the three regions ─
    // AuiToolBar reserves the top ~28 px of the client area, so the
    // sizer's content starts a bit further down. We position the
    // StaticText labels at fixed coordinates so the layout matches
    // what users would expect from a typical editor window.
    let lbl_hint = StaticText::new(&frame, "📝  Type your document in the editor below.");
    lbl_hint.as_widget_ref().borrow_mut().set_position(12, 42);
    lbl_hint.as_widget_ref().borrow_mut().set_size(400, 20);

    let lbl_options = StaticText::new(&frame, "⚙  Options:");
    lbl_options
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, 252);
    lbl_options.as_widget_ref().borrow_mut().set_size(120, 20);

    let lbl_toolbar = StaticText::new(&frame, "🧰  Toolbar controls:");
    lbl_toolbar
        .as_widget_ref()
        .borrow_mut()
        .set_position(280, 252);
    lbl_toolbar.as_widget_ref().borrow_mut().set_size(160, 20);

    let lbl_info = StaticText::new(
        &frame,
        "ℹ  This example combines AuiToolBar + ToolTip + StaticText — the three\n\
         controls most recently added to ru_wx.",
    );
    lbl_info.as_widget_ref().borrow_mut().set_position(12, 360);
    lbl_info.as_widget_ref().borrow_mut().set_size(720, 40);

    // ── ToolTip (NEW CONTROL #3) — attach to every interactive widget ─
    // The ToolTip API works against the platform-independent `WidgetRef`
    // so we can attach it to anything (StaticText, Button, AuiToolBar,
    // TextCtrl, ...).

    // StaticText labels: just a hint of what they are
    ToolTip::new("Descriptive label — read-only.").attach(&lbl_hint.as_widget_ref());
    ToolTip::new("Options section header.").attach(&lbl_options.as_widget_ref());
    ToolTip::new("Toolbar controls section header.").attach(&lbl_toolbar.as_widget_ref());
    ToolTip::new("A short note about this example.").attach(&lbl_info.as_widget_ref());

    // The AuiToolBar itself: explain how to detach
    ToolTip::new("Click the ≡ gripper to detach me, then click it again to re-dock.")
        .attach(&aui.as_widget_ref());

    // ── Multiline TextCtrl — the "document" being edited ───────────
    let editor = TextCtrl::multiline(
        &frame,
        "Welcome to esempio2 — the Mini Editor!\n\n\
         Try the following:\n\
         • Click any toolbar button (or hover it for a tooltip).\n\
         • Click the ≡ gripper on the toolbar to detach it to a\n\
           floating window; click it again (or close the floating\n\
           window) to re-dock.\n\
         • Toggle the \"Show tooltips\" checkbox below to globally\n\
           enable or disable every tooltip in this window.\n\
         • Use the \"Float\" / \"Dock Top\" buttons to programmatically\n\
           detach and re-dock the toolbar.\n",
    );
    editor.as_widget_ref().borrow_mut().set_position(12, 68);
    editor.as_widget_ref().borrow_mut().set_size(720, 175);

    ToolTip::new("The document you are editing. Type freely.").attach(&editor.as_widget_ref());

    // ── Options row: CheckBox + buttons ────────────────────────────
    let chk_tooltips = CheckBox::new(&frame, "Show tooltips");
    chk_tooltips
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, 280);
    chk_tooltips.as_widget_ref().borrow_mut().set_size(140, 24);
    chk_tooltips.set_checked(true);

    ToolTip::new("Globally enable or disable every tooltip in this window. Try unchecking me!")
        .attach(&chk_tooltips.as_widget_ref());

    // Wire the checkbox to toggle the global tooltip state and update
    // the status bar. `on_toggle` receives no arguments — we read the
    // new state from the checkbox via `is_checked()` inside the closure.
    // We clone the CheckBox so the closure owns a copy (the `on_toggle`
    // call borrows the original via `&self`).
    let chk_for_cb = chk_tooltips.clone();
    let status_for_chk = status.clone();
    chk_tooltips.on_toggle(&frame, move || {
        let checked = chk_for_cb.is_checked();
        ToolTip::enable(checked);
        status_for_chk.set_status_text(
            if checked {
                "Tooltips enabled"
            } else {
                "Tooltips disabled"
            },
            0,
        );
    });

    // ── Toolbar-control buttons ────────────────────────────────────
    let btn_float = Button::new(&frame, "Float");
    btn_float
        .as_widget_ref()
        .borrow_mut()
        .set_position(280, 278);
    btn_float.as_widget_ref().borrow_mut().set_size(90, 30);
    ToolTip::new("Detach the AuiToolBar to a floating window at a fixed position.")
        .attach(&btn_float.as_widget_ref());

    let btn_dock_top = Button::new(&frame, "Dock Top");
    btn_dock_top
        .as_widget_ref()
        .borrow_mut()
        .set_position(378, 278);
    btn_dock_top.as_widget_ref().borrow_mut().set_size(90, 30);
    ToolTip::new("Re-dock the AuiToolBar to the top edge of the frame.")
        .attach(&btn_dock_top.as_widget_ref());

    let btn_cycle = Button::new(&frame, "Cycle Dock");
    btn_cycle
        .as_widget_ref()
        .borrow_mut()
        .set_position(476, 278);
    btn_cycle.as_widget_ref().borrow_mut().set_size(100, 30);
    ToolTip::new("Cycle the toolbar through Top → Bottom → Left → Right → Top.")
        .attach(&btn_cycle.as_widget_ref());

    // ── Wire up the buttons ────────────────────────────────────────
    let aui_for_float = aui.clone();
    btn_float.on_click(&frame, move || {
        aui_for_float.float_at(420, 220);
    });

    let aui_for_dock_top = aui.clone();
    btn_dock_top.on_click(&frame, move || {
        aui_for_dock_top.dock_to(AuiDockSide::Top);
    });

    // Cycle: top → bottom → left → right → top. We can't read the
    // current dock side from the closure without an Rc, so we keep a
    // local mutable index behind an Rc<RefCell<...>>.
    use std::cell::RefCell;
    let cycle_idx: std::rc::Rc<RefCell<u8>> = std::rc::Rc::new(RefCell::new(0));
    let aui_for_cycle = aui.clone();
    let cycle_idx_for_cb = cycle_idx.clone();
    btn_cycle.on_click(&frame, move || {
        let next = {
            let mut i = cycle_idx_for_cb.borrow_mut();
            let cur = *i;
            *i = (*i + 1) % 4;
            cur
        };
        let side = match next {
            0 => AuiDockSide::Top,
            1 => AuiDockSide::Bottom,
            2 => AuiDockSide::Left,
            _ => AuiDockSide::Right,
        };
        aui_for_cycle.dock_to(side);
    });

    // ── Vertical sizer for the bottom controls (purely demonstrative —
    // we manually positioned them so the editor has absolute
    // coordinates, but this sizer shows how a real app would compose
    // them).
    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl_hint.as_widget_ref());
    sizer.add_stretch(1);
    sizer.add(editor.as_widget_ref());
    sizer.add(lbl_info.as_widget_ref());
    let _ = sizer; // intentionally not applied to the frame

    app.run(frame);
}
