//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `esempio2` — Mini Editor with a colourful dockable toolbar, native
//! Win32 tooltips on every tool button, per-widget [`ToolTip`]s on the
//! rest of the UI, and Windows 11 rounded corners via DWM.
//!
//! Run with:
//! ```bash
//! cargo run --example esempio2
//! ```
//! On Windows 11, build with the Common Controls v6 manifest (see
//! `build_with_manifest.ps1`) for PerMonitorV2 scaling and modern
//! control theming.

#![windows_subsystem = "windows"]

use ru_wx::{
    App, AuiDockSide, AuiToolBar, BitmapBundle, Button, CheckBox, ImageList, StaticText,
    StatusBar, TextCtrl, ToolTip, TopLevelWindow, WindowCornerPreference,
};

// Colourful inline SVG icons (24×24 viewBox) with filled backgrounds so
// they stand out on the light-grey toolbar. White strokes on top.
const ICON_NEW: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#4F46E5"/><path d="M14 6H8a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V10z" fill="none" stroke="white" stroke-width="1.6"/><path d="M14 6v4h4 M11 16h2 M12 13v5" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg>"##;
const ICON_OPEN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#10B981"/><path d="M3 10a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#F59E0B"/><path d="M6 4h10l3 3v13H5z M9 4v5h6V4 M8 13h8v7H8z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_CUT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#EF4444"/><circle cx="7" cy="8" r="2.2" fill="white"/><circle cx="7" cy="16" r="2.2" fill="white"/><path d="M9 9.5L20 20 M9 14.5L20 4" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_COPY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#8B5CF6"/><rect x="9" y="9" width="11" height="11" rx="1.5" fill="none" stroke="white" stroke-width="1.6"/><path d="M15 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h3" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_PASTE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#06B6D4"/><rect x="7" y="5" width="12" height="15" rx="1.5" fill="none" stroke="white" stroke-width="1.6"/><path d="M9 5V3h6v2" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;

const ID_TOOL_NEW: u16 = 2001;
const ID_TOOL_OPEN: u16 = 2002;
const ID_TOOL_SAVE: u16 = 2003;
const ID_TOOL_CUT: u16 = 2004;
const ID_TOOL_COPY: u16 = 2005;
const ID_TOOL_PASTE: u16 = 2006;

/// Vertical space reserved by the docked AuiToolBar (see `set_toolbar_height`).
const TOOLBAR_H: i32 = 52;

fn main() {
    let app = App::new();

    // TopLevelWindow gives us Win11 DWM helpers (rounded corners).
    let window = TopLevelWindow::new(
        "esempio2 — Mini Editor (Win11 + icone colorate)",
        780,
        560,
    );
    let _ = window.set_window_corner_preference(WindowCornerPreference::Default);
    let frame = window.frame().clone();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text(
        "Passa il mouse sui pulsanti della toolbar per i tooltip nativi Win32.",
        0,
    );

    // 40×40 colourful icons (HiDPI-friendly).
    let icon_sizes: [(u32, u32); 3] = [(32, 32), (40, 40), (48, 48)];

    let bundle_new = BitmapBundle::from_svg_bytes(ICON_NEW, &icon_sizes);
    let bundle_open = BitmapBundle::from_svg_bytes(ICON_OPEN, &icon_sizes);
    let bundle_save = BitmapBundle::from_svg_bytes(ICON_SAVE, &icon_sizes);
    let bundle_cut = BitmapBundle::from_svg_bytes(ICON_CUT, &icon_sizes);
    let bundle_copy = BitmapBundle::from_svg_bytes(ICON_COPY, &icon_sizes);
    let bundle_paste = BitmapBundle::from_svg_bytes(ICON_PASTE, &icon_sizes);

    let images = ImageList::new(40, 40);
    for bundle in [
        &bundle_new,
        &bundle_open,
        &bundle_save,
        &bundle_cut,
        &bundle_copy,
        &bundle_paste,
    ] {
        if let Some(bmp) = bundle.best_for_size((40, 40)) {
            images.add_bitmap(bmp.hbitmap);
        }
    }

    let aui = AuiToolBar::new(&frame);
    aui.set_toolbar_height(TOOLBAR_H);
    aui.set_image_list(&images);
    // Labels become native toolbar tooltips via TB_ADDSTRING in realize().
    aui.add_tool(ID_TOOL_NEW, "Nuovo documento (Ctrl+N)", 0);
    aui.add_tool(ID_TOOL_OPEN, "Apri file…", 1);
    aui.add_tool(ID_TOOL_SAVE, "Salva documento (Ctrl+S)", 2);
    aui.add_separator();
    aui.add_tool(ID_TOOL_CUT, "Taglia selezione", 3);
    aui.add_tool(ID_TOOL_COPY, "Copia selezione", 4);
    aui.add_tool(ID_TOOL_PASTE, "Incolla dagli appunti", 5);
    aui.realize();

    let status_for_tools = status.clone();
    aui.on_tool_clicked(&frame, move |id| {
        let label = match id {
            ID_TOOL_NEW => "Nuovo",
            ID_TOOL_OPEN => "Apri",
            ID_TOOL_SAVE => "Salva",
            ID_TOOL_CUT => "Taglia",
            ID_TOOL_COPY => "Copia",
            ID_TOOL_PASTE => "Incolla",
            _ => "?",
        };
        status_for_tools.set_status_text(&format!("Toolbar → {label}"), 0);
    });

    let status_for_dock = status.clone();
    aui.on_dock_state_change(move |side| {
        let label = match side {
            AuiDockSide::Top => "in alto",
            AuiDockSide::Bottom => "in basso",
            AuiDockSide::Left => "a sinistra",
            AuiDockSide::Right => "a destra",
            AuiDockSide::Floating => "flottante",
        };
        status_for_dock.set_status_text(&format!("Toolbar agganciata {label}"), 0);
    });

    // Layout: everything below the 52 px toolbar band.
    let content_top = TOOLBAR_H + 10;

    let lbl_hint = StaticText::new(&frame, "Scrivi il documento nell'editor qui sotto.");
    lbl_hint
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top);
    lbl_hint.as_widget_ref().borrow_mut().set_size(500, 22);
    ToolTip::new("Etichetta descrittiva — solo lettura.").attach(&lbl_hint.as_widget_ref());

    let editor = TextCtrl::multiline(
        &frame,
        "Benvenuto in esempio2!\n\n\
         Cosa provare:\n\
         • Passa il mouse sui pulsanti colorati della toolbar — ogni\n\
           pulsante mostra un tooltip nativo Win32 (TB_ADDSTRING).\n\
         • Clicca il gripper (≡) per staccare la toolbar; cliccalo di\n\
           nuovo per riagganciarla.\n\
         • La finestra usa angoli arrotondati Windows 11 (DWM).\n\
         • La casella \"Mostra tooltip\" abilita/disabilita i tooltip\n\
           ru_wx sugli altri controlli (editor, pulsanti, ecc.).\n",
    );
    editor
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top + 28);
    editor.as_widget_ref().borrow_mut().set_size(740, 175);
    ToolTip::new("Area di modifica del documento.").attach(&editor.as_widget_ref());

    let lbl_options = StaticText::new(&frame, "Opzioni:");
    lbl_options
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top + 218);
    lbl_options.as_widget_ref().borrow_mut().set_size(120, 22);

    let lbl_toolbar = StaticText::new(&frame, "Controlli toolbar:");
    lbl_toolbar
        .as_widget_ref()
        .borrow_mut()
        .set_position(300, content_top + 218);
    lbl_toolbar.as_widget_ref().borrow_mut().set_size(180, 22);

    let chk_tooltips = CheckBox::new(&frame, "Mostra tooltip (controlli ru_wx)");
    chk_tooltips
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top + 246);
    chk_tooltips.as_widget_ref().borrow_mut().set_size(240, 24);
    chk_tooltips.set_checked(true);
    ToolTip::new(
        "Abilita o disabilita i tooltip ru_wx su editor, pulsanti e label.\n\
         I tooltip nativi della toolbar restano sempre attivi.",
    )
    .attach(&chk_tooltips.as_widget_ref());

    let chk_for_cb = chk_tooltips.clone();
    let status_for_chk = status.clone();
    chk_tooltips.on_toggle(&frame, move || {
        let checked = chk_for_cb.is_checked();
        ToolTip::enable(checked);
        status_for_chk.set_status_text(
            if checked {
                "Tooltip ru_wx abilitati"
            } else {
                "Tooltip ru_wx disabilitati (toolbar nativa ancora attiva)"
            },
            0,
        );
    });

    let btn_float = Button::new(&frame, "Stacca");
    btn_float
        .as_widget_ref()
        .borrow_mut()
        .set_position(300, content_top + 244);
    btn_float.as_widget_ref().borrow_mut().set_size(90, 30);
    ToolTip::new("Stacca la toolbar in una finestra flottante.").attach(&btn_float.as_widget_ref());

    let btn_dock_top = Button::new(&frame, "Aggancia sopra");
    btn_dock_top
        .as_widget_ref()
        .borrow_mut()
        .set_position(398, content_top + 244);
    btn_dock_top.as_widget_ref().borrow_mut().set_size(110, 30);
    ToolTip::new("Riaggancia la toolbar al bordo superiore.").attach(&btn_dock_top.as_widget_ref());

    let btn_cycle = Button::new(&frame, "Cicla bordo");
    btn_cycle
        .as_widget_ref()
        .borrow_mut()
        .set_position(516, content_top + 244);
    btn_cycle.as_widget_ref().borrow_mut().set_size(110, 30);
    ToolTip::new("Ruota: alto → basso → sinistra → destra → alto.")
        .attach(&btn_cycle.as_widget_ref());

    let aui_for_float = aui.clone();
    btn_float.on_click(&frame, move || {
        aui_for_float.float_at(420, 220);
    });

    let aui_for_dock_top = aui.clone();
    btn_dock_top.on_click(&frame, move || {
        aui_for_dock_top.dock_to(AuiDockSide::Top);
    });

    use std::cell::RefCell;
    use std::rc::Rc;
    let cycle_idx: Rc<RefCell<u8>> = Rc::new(RefCell::new(0));
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

    let lbl_info = StaticText::new(
        &frame,
        "Toolbar 40×40 con icone colorate · tooltip nativi Win32 su ogni tool ·\n\
         angoli arrotondati Win11 (DWMWA_WINDOW_CORNER_PREFERENCE).",
    );
    lbl_info
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top + 290);
    lbl_info.as_widget_ref().borrow_mut().set_size(740, 40);
    ToolTip::new("Note tecniche su questo esempio.").attach(&lbl_info.as_widget_ref());

    app.run(frame);
}
