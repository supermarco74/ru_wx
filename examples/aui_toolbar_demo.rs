//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Standalone demo for `wxAuiToolBar` — a dockable toolbar that can be
//! detached to a floating window and re-docked to the frame's edges.
//!
//! This version shows off bigger, more colorful icons (40×40) and a
//! live click counter so it's obvious at a glance whether tool
//! clicks are being delivered.
//!
//! Try the following:
//!
//! 1. Click the gripper (≡) on the leading edge of the toolbar — the
//!    toolbar detaches and becomes a floating top-level window.
//! 2. Drag the floating window around the screen, then click the
//!    gripper again, double-click its title bar, or close it — the
//!    toolbar re-docks at the top of the frame.
//! 3. Use the buttons below to dock to the top/bottom/left/right
//!    edge or float at a specific screen position.
//! 4. Click any of the colourful tool buttons — the click counter
//!    below the toolbar ticks up so you can see exactly when a tool
//!    fires.
//!
//! Run with:
//! ```bash
//! cargo run --example aui_toolbar_demo
//! ```

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::rc::Rc;

use ru_wx::{
    App, AuiDockSide, AuiToolBar, BitmapBundle, Button, Frame, ImageList, StaticText, StatusBar,
    ToolTip,
};

// Colorful inline SVG icons (24×24 viewBox) with **filled** coloured
// backgrounds so they stand out against the standard light-grey
// toolbar surface. The white outline is drawn on top of the fill.
//
// We use `br##"..."##` (double hash) as the raw byte string
// delimiter because the SVG bodies contain `fill="#RRGGBB"` colour
// values which would otherwise prematurely close a single-hash
// `br#"..."#` literal.
const ICON_NEW: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#4F46E5"/><path d="M14 6H8a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V10z" fill="none" stroke="white" stroke-width="1.6"/><path d="M14 6v4h4 M11 16h2 M12 13v5" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg>"##;
const ICON_OPEN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#10B981"/><path d="M3 10a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#F59E0B"/><path d="M6 4h10l3 3v13H5z M9 4v5h6V4 M8 13h8v7H8z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_CUT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#EF4444"/><circle cx="7" cy="8" r="2.2" fill="white"/><circle cx="7" cy="16" r="2.2" fill="white"/><path d="M9 9.5L20 20 M9 14.5L20 4" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_COPY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#8B5CF6"/><rect x="9" y="9" width="11" height="11" rx="1.5" fill="none" stroke="white" stroke-width="1.6"/><path d="M15 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h3" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;

// User-chosen tool ids. We dispatch on these in the click handler
// below.
const ID_TOOL_NEW: u16 = 1001;
const ID_TOOL_OPEN: u16 = 1002;
const ID_TOOL_SAVE: u16 = 1003;
const ID_TOOL_CUT: u16 = 1004;
const ID_TOOL_COPY: u16 = 1005;

fn main() {
    let app = App::new();

    // Main frame. The AuiToolBar is positioned at the top of the
    // client area automatically; we lay out the rest of the content
    // by hand below it.
    let frame = Frame::builder()
        .with_title("wxAuiToolBar demo — big colourful icons + click counter")
        .with_size(760, 480)
        .build();

    // Status bar at the bottom — used to report tool clicks and
    // dock-state changes.
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click the gripper to detach the toolbar", 0);

    // ---- Image list (40×40) — rasterise the SVG icons at multiple
    // sizes for HiDPI and pick the 40×40 ones for the toolbar.
    let icon_sizes: [(u32, u32); 3] = [(32, 32), (40, 40), (48, 48)];

    let bundle_new = BitmapBundle::from_svg_bytes(ICON_NEW, &icon_sizes);
    let bundle_open = BitmapBundle::from_svg_bytes(ICON_OPEN, &icon_sizes);
    let bundle_save = BitmapBundle::from_svg_bytes(ICON_SAVE, &icon_sizes);
    let bundle_cut = BitmapBundle::from_svg_bytes(ICON_CUT, &icon_sizes);
    let bundle_copy = BitmapBundle::from_svg_bytes(ICON_COPY, &icon_sizes);

    let images = ImageList::new(40, 40);
    if let Some(bmp) = bundle_new.best_for_size((40, 40)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_open.best_for_size((40, 40)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_save.best_for_size((40, 40)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_cut.best_for_size((40, 40)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_copy.best_for_size((40, 40)) {
        images.add_bitmap(bmp.hbitmap);
    }

    // ---- The dockable toolbar ----
    let aui = AuiToolBar::new(&frame);
    // Grow the bar so 40×40 icons fit with breathing room.
    aui.set_toolbar_height(52);
    aui.set_image_list(&images);
    aui.add_tool(ID_TOOL_NEW, "New document", 0);
    aui.add_tool(ID_TOOL_OPEN, "Open file…", 1);
    aui.add_tool(ID_TOOL_SAVE, "Save document", 2);
    aui.add_separator();
    aui.add_tool(ID_TOOL_CUT, "Cut selection", 3);
    aui.add_tool(ID_TOOL_COPY, "Copy selection", 4);
    aui.realize();

    // ---- Shared click counter. The single closure bumps the
    // counter and rewrites the label on each call. Because the
    // closure is `FnMut`, the captured `Rc<RefCell<u32>>` lets us
    // mutate state across multiple tool buttons while still being
    // shared via clone.
    let click_count: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
    let last_tool: Rc<RefCell<String>> = Rc::new(RefCell::new(String::from("(none)")));

    let lbl_count =
        StaticText::new(&frame, "Total tool clicks: 0   |   Last tool: (none)");
    lbl_count
        .as_widget_ref()
        .borrow_mut()
        .set_position(10, 120);
    lbl_count.as_widget_ref().borrow_mut().set_size(720, 24);

    // ---- Wire up tool clicks. The single callback fires for every
    // tool with the tool's id; we look up its human label, bump the
    // counter, update the last-tool string and rewrite the label.
    let label_for_id = |id: u16| -> &'static str {
        match id {
            ID_TOOL_NEW => "New",
            ID_TOOL_OPEN => "Open",
            ID_TOOL_SAVE => "Save",
            ID_TOOL_CUT => "Cut",
            ID_TOOL_COPY => "Copy",
            _ => "?",
        }
    };

    let status_for_tools = status.clone();
    let lbl_for_tools = lbl_count.clone();
    let count_for_tools = click_count.clone();
    let last_for_tools = last_tool.clone();
    aui.on_tool_clicked(&frame, move |id| {
        let label = label_for_id(id);
        *count_for_tools.borrow_mut() += 1;
        *last_for_tools.borrow_mut() = label.to_string();
        let n = *count_for_tools.borrow();
        let last = last_for_tools.borrow().clone();
        lbl_for_tools.set_label(&format!(
            "Total tool clicks: {n}   |   Last tool: {last}"
        ));
        status_for_tools.set_status_text(&format!("Tool clicked: {label}"), 0);
    });

    // ---- Dock-state-change callback. Fires whenever the toolbar
    // moves between docked edges and the floating state.
    let status_for_dock = status.clone();
    aui.on_dock_state_change(move |side| {
        let label = match side {
            AuiDockSide::Top => "top",
            AuiDockSide::Bottom => "bottom",
            AuiDockSide::Left => "left",
            AuiDockSide::Right => "right",
            AuiDockSide::Floating => "floating",
        };
        status_for_dock.set_status_text(&format!("Toolbar is now docked to: {label}"), 0);
    });

    // ---- Hint label and dock-control buttons. The AuiToolBar
    // reserves the top 52 px of the client area (because of
    // `set_toolbar_height(52)`); we put everything else at y ≥ 70.
    let lbl_hint = StaticText::new(
        &frame,
        "Click the gripper (≡) to detach, or use the buttons below:",
    );
    lbl_hint.as_widget_ref().borrow_mut().set_position(10, 70);
    lbl_hint.as_widget_ref().borrow_mut().set_size(500, 20);

    let btn_top = Button::new(&frame, "Dock Top");
    btn_top.as_widget_ref().borrow_mut().set_position(10, 160);
    btn_top.as_widget_ref().borrow_mut().set_size(110, 30);

    let btn_bottom = Button::new(&frame, "Dock Bottom");
    btn_bottom
        .as_widget_ref()
        .borrow_mut()
        .set_position(130, 160);
    btn_bottom.as_widget_ref().borrow_mut().set_size(110, 30);

    let btn_left = Button::new(&frame, "Dock Left");
    btn_left.as_widget_ref().borrow_mut().set_position(250, 160);
    btn_left.as_widget_ref().borrow_mut().set_size(110, 30);

    let btn_right = Button::new(&frame, "Dock Right");
    btn_right.as_widget_ref().borrow_mut().set_position(370, 160);
    btn_right.as_widget_ref().borrow_mut().set_size(110, 30);

    let btn_float = Button::new(&frame, "Float");
    btn_float.as_widget_ref().borrow_mut().set_position(490, 160);
    btn_float.as_widget_ref().borrow_mut().set_size(110, 30);

    // Per-widget ru_wx tooltips on the regular buttons (tooltips_class32).
    ToolTip::new("Aggancia la toolbar in alto").attach(&btn_top.as_widget_ref());
    ToolTip::new("Aggancia la toolbar in basso").attach(&btn_bottom.as_widget_ref());
    ToolTip::new("Aggancia la toolbar a sinistra").attach(&btn_left.as_widget_ref());
    ToolTip::new("Aggancia la toolbar a destra").attach(&btn_right.as_widget_ref());
    ToolTip::new("Stacca la toolbar come finestra flottante")
        .attach(&btn_float.as_widget_ref());

    // ---- Wire up the buttons. Each closure captures a clone of the
    // AuiToolBar (cheap) so the toolbar can be moved/docked even
    // after the button is clicked.
    let aui_for_top = aui.clone();
    btn_top.on_click(&frame, move || {
        aui_for_top.dock_to(AuiDockSide::Top);
    });

    let aui_for_bottom = aui.clone();
    btn_bottom.on_click(&frame, move || {
        aui_for_bottom.dock_to(AuiDockSide::Bottom);
    });

    let aui_for_left = aui.clone();
    btn_left.on_click(&frame, move || {
        aui_for_left.dock_to(AuiDockSide::Left);
    });

    let aui_for_right = aui.clone();
    btn_right.on_click(&frame, move || {
        aui_for_right.dock_to(AuiDockSide::Right);
    });

    let aui_for_float = aui.clone();
    btn_float.on_click(&frame, move || {
        aui_for_float.float_at(420, 220);
    });

    // A small info label below the buttons so the demo window has
    // some extra content.
    let info = StaticText::new(
        &frame,
        "Dock sides: Top / Bottom / Left / Right / Floating\n\
         User interactions: click the gripper, double-click the floating\n\
         title bar, or close the floating window — all re-dock to Top.\n\
         Click counter at top ticks up on every tool click so you can see\n\
         in real time whether tool events are being delivered.",
    );
    info.as_widget_ref().borrow_mut().set_position(10, 210);
    info.as_widget_ref().borrow_mut().set_size(720, 80);

    // A second, separate "Reset counter" button to demonstrate that
    // the counter can also be reset.
    let btn_reset = Button::new(&frame, "Reset counter");
    btn_reset
        .as_widget_ref()
        .borrow_mut()
        .set_position(620, 160);
    btn_reset.as_widget_ref().borrow_mut().set_size(120, 30);

    let count_for_reset = click_count.clone();
    let last_for_reset = last_tool.clone();
    let lbl_for_reset = lbl_count.clone();
    btn_reset.on_click(&frame, move || {
        *count_for_reset.borrow_mut() = 0;
        *last_for_reset.borrow_mut() = "(none)".to_string();
        lbl_for_reset.set_label("Total tool clicks: 0   |   Last tool: (none)");
    });

    // Run the app.
    app.run(frame);
}
