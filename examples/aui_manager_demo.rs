//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Standalone demo for the `wxAuiManager` family — docking layout,
//! themed panes, icon toolbars, and an interactive **Home** notebook
//! page to tweak tab icons, dock colours, and sash metrics live.
//!
//! Run with:
//! ```bash
//! cargo run --example aui_manager_demo
//! ```

#![windows_subsystem = "windows"]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg(target_os = "macos")]
use ru_wx::platform::appkit::{TabControlSize, TabViewChrome};
use ru_wx::{
    App, AuiDockArtGradient, AuiDockSide, AuiManager, AuiNotebook, AuiPaneButton, AuiPaneInfo,
    AuiToolBar, BoxSizer, Button, CheckBox, Frame, ImageList, Panel, Slider, StaticText, TextCtrl,
};

// ── Toolbar icons (40×40, filled SVG) ─────────────────────────────
const ICON_NEW: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#4F46E5"/><path d="M14 6H8a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V10z" fill="none" stroke="white" stroke-width="1.6"/><path d="M14 6v4h4 M11 16h2 M12 13v5" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg>"##;
const ICON_OPEN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#10B981"/><path d="M3 10a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#F59E0B"/><path d="M6 4h10l3 3v13H5z M9 4v5h6V4 M8 13h8v7H8z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;

const ICON_HELP: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="#EF4444"/><path d="M12 8v4m0 5v.01" stroke="white" stroke-width="2" stroke-linecap="round"/></svg>"##;
const ICON_EXIT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#6B7280"/><path d="M16 8l-8 8m0-8l8 8" stroke="white" stroke-width="2" stroke-linecap="round"/></svg>"##;

// ── Bottom-toolbar icons (40×40, media-style) ──────────────────
const ICON_PLAY: &[u8] = br##"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><circle cx='12' cy='12' r='10' fill='#22C55E'/><polygon points='10,7 17,12 10,17' fill='white'/></svg>"##;
const ICON_PAUSE: &[u8] = br##"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><circle cx='12' cy='12' r='10' fill='#F59E0B'/><rect x='9' y='7' width='2.5' height='10' rx='0.5' fill='white'/><rect x='12.5' y='7' width='2.5' height='10' rx='0.5' fill='white'/></svg>"##;
const ICON_STOP: &[u8] = br##"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><circle cx='12' cy='12' r='10' fill='#EF4444'/><rect x='9' y='9' width='6' height='6' rx='1' fill='white'/></svg>"##;
const ICON_RECORD: &[u8] = br##"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><circle cx='12' cy='12' r='10' fill='#EF4444'/><circle cx='12' cy='12' r='4' fill='white'/></svg>"##;

// ── Tab strip icons (32×32, filled) ─────────────────────────────
const SVG_TAB_HOME: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="4" fill="#4F46E5"/><path d="M12 6.5 6 11v8h4v-5h4v5h4v-8z" fill="white"/></svg>"##;
const SVG_TAB_EDIT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="4" fill="#10B981"/><path d="M14.5 6.5 17.5 9.5 9 18H6v-3z" fill="white"/></svg>"##;
const SVG_TAB_LOG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="4" fill="#F59E0B"/><path d="M8 8h8v2H8zm0 4h8v2H8zm0 4h5v2H8z" fill="white"/></svg>"##;
const SVG_TAB_STAR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="4" fill="#9B59B6"/><path d="m12 7 2 4.5 5 .7-3.6 3.5.9 5.2L12 18.8 7.7 20.9l.9-5.2L5 12.2l5-.7z" fill="white"/></svg>"##;
const SVG_TAB_SETTINGS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="4" fill="#06B6D4"/><circle cx="12" cy="12" r="3" fill="white"/><path d="M12 2v3m0 14v3M2 12h3m14 0h3M4.9 4.9l2.1 2.1m9.9 9.9 2.1 2.1M4.9 19.1l2.1-2.1m9.9-9.9 2.1-2.1" stroke="white" stroke-width="2"/></svg>"##;
const SVG_TAB_DOCS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="1" y="1" width="22" height="22" rx="4" fill="#EC4899"/><path d="M7 4h8l4 4v12H7z" fill="white"/><path d="M15 4v4h4" fill="none" stroke="white"/></svg>"##;

const HOME_TITLES: &[&str] = &["Home", "Dashboard", "Workspace", "Start"];
const TAB_ICON_CYCLE: [i32; 6] = [0, 1, 2, 3, 4, -1];

const ID_TOOL_NEW: u16 = 2001;
const ID_TOOL_OPEN: u16 = 2002;
const ID_TOOL_SAVE: u16 = 2003;
const ID_TOOL_HELP: u16 = 2004;
const ID_TOOL_EXIT: u16 = 2005;

const ID_BOTTOM_PLAY: u16 = 2101;
const ID_BOTTOM_PAUSE: u16 = 2102;
const ID_BOTTOM_STOP: u16 = 2103;
const ID_BOTTOM_RECORD: u16 = 2104;

fn append_log(log: &TextCtrl, line: &str) {
    let mut text = log.get_value();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(line);
    text.push('\n');
    log.set_value(&text);
}

fn build_toolbar_images() -> ImageList {
    let images = ImageList::new(40, 40);
    for svg in [ICON_NEW, ICON_OPEN, ICON_SAVE, ICON_HELP, ICON_EXIT] {
        images.add_svg_bytes(svg);
    }
    images
}

fn build_tab_images() -> ImageList {
    let icons = ImageList::new(32, 32);
    icons.add_svg_bytes(SVG_TAB_HOME);
    icons.add_svg_bytes(SVG_TAB_EDIT);
    icons.add_svg_bytes(SVG_TAB_LOG);
    icons.add_svg_bytes(SVG_TAB_STAR);
    icons.add_svg_bytes(SVG_TAB_SETTINGS);
    icons.add_svg_bytes(SVG_TAB_DOCS);
    icons
}

fn apply_dock_theme(aui: &mut AuiManager, preset: &str) {
    let art = aui.dock_art_mut();
    match preset {
        "ocean" => {
            art.set_gradient(AuiDockArtGradient::Horizontal);
            art.set_sash_width(6);
            art.set_pane_padding(3);
            art.set_active_caption(0x00E8_F5F3);
            art.set_sash(0x00A0_C8C0);
        }
        "forest" => {
            art.set_gradient(AuiDockArtGradient::None);
            art.set_sash_width(3);
            art.set_pane_padding(2);
            art.set_active_caption(0x00D4_EDDA);
            art.set_sash(0x0080_A080);
        }
        _ => {
            art.set_gradient(AuiDockArtGradient::Vertical);
            art.set_sash_width(4);
            art.set_pane_padding(2);
            art.set_active_caption(0x00D8_E9F8);
            art.set_sash(0x00C8_C8C8);
        }
    }
}

fn theme_summary(aui: &AuiManager) -> String {
    let art = aui.dock_art();
    let grad = match art.gradient() {
        AuiDockArtGradient::None => "flat",
        AuiDockArtGradient::Vertical => "vertical",
        AuiDockArtGradient::Horizontal => "horizontal",
    };
    format!(
        "Dock theme: gradient={grad}, sash={}px, padding={}px",
        art.sash_width(),
        art.pane_padding()
    )
}

fn relayout_notebook(aui: &AuiManager, notebook: &AuiNotebook) {
    if let Some(center_rect) = aui.get_pane("main_notebook").map(|p| p.rect) {
        notebook.set_size(center_rect.width as i32, center_rect.height as i32);
        notebook.relayout();
    }
}

// ── Build top toolbar using AuiToolBar (native NSToolbar in title bar) ─
fn build_aui_toolbar(frame: &Frame, log_ctrl: &TextCtrl) -> AuiToolBar {
    let icons = build_toolbar_images();
    let toolbar = AuiToolBar::new(frame);
    toolbar.set_toolbar_height(50);
    toolbar.set_image_list(&icons);
    toolbar.add_tool(ID_TOOL_NEW, "New", 0);
    toolbar.add_tool(ID_TOOL_OPEN, "Open", 1);
    toolbar.add_tool(ID_TOOL_SAVE, "Save", 2);
    toolbar.add_separator();
    toolbar.add_tool(ID_TOOL_HELP, "Help", 3);
    toolbar.add_tool(ID_TOOL_EXIT, "Exit", 4);
    toolbar.realize_native();

    let log = log_ctrl.clone();
    toolbar.on_tool_clicked(frame, move |id| match id {
        ID_TOOL_NEW => append_log(&log, "[top-toolbar] New clicked"),
        ID_TOOL_OPEN => append_log(&log, "[top-toolbar] Open clicked"),
        ID_TOOL_SAVE => append_log(&log, "[top-toolbar] Save clicked"),
        ID_TOOL_HELP => append_log(&log, "[top-toolbar] Help clicked"),
        ID_TOOL_EXIT => append_log(&log, "[top-toolbar] Exit clicked"),
        _ => append_log(&log, &format!("[top-toolbar] Unknown tool {id} clicked")),
    });

    toolbar
}

// ── Build bottom toolbar as an AuiManager pane (regular AuiToolBar) ─
fn build_bottom_toolbar_images() -> ImageList {
    let images = ImageList::new(40, 40);
    for svg in [ICON_PLAY, ICON_PAUSE, ICON_STOP, ICON_RECORD] {
        images.add_svg_bytes(svg);
    }
    images
}

fn build_bottom_toolbar(frame: &Frame, log_ctrl: &TextCtrl) -> AuiToolBar {
    let icons = build_bottom_toolbar_images();
    let toolbar = AuiToolBar::new(frame);
    toolbar.set_toolbar_height(50);
    toolbar.set_image_list(&icons);
    toolbar.add_tool(ID_BOTTOM_PLAY, "Play", 0);
    toolbar.add_tool(ID_BOTTOM_PAUSE, "Pause", 1);
    toolbar.add_tool(ID_BOTTOM_STOP, "Stop", 2);
    toolbar.add_separator();
    toolbar.add_tool(ID_BOTTOM_RECORD, "Record", 3);
    toolbar.realize();

    let log = log_ctrl.clone();
    toolbar.on_tool_clicked(frame, move |id| match id {
        ID_BOTTOM_PLAY => append_log(&log, "[bottom-toolbar] Play clicked"),
        ID_BOTTOM_PAUSE => append_log(&log, "[bottom-toolbar] Pause clicked"),
        ID_BOTTOM_STOP => append_log(&log, "[bottom-toolbar] Stop clicked"),
        ID_BOTTOM_RECORD => append_log(&log, "[bottom-toolbar] Record clicked"),
        _ => append_log(&log, &format!("[bottom-toolbar] Unknown tool {id} clicked")),
    });

    toolbar
}

fn main() {
    let app = App::new();

    let frame = Frame::builder()
        .with_title("wxAuiManager demo — docking, tabs, icons & live theme")
        .with_size(1100, 720)
        .with_position(80, 0)
        .build();

    let aui: Rc<RefCell<AuiManager>> = Rc::new(RefCell::new(AuiManager::new()));
    {
        let mut aui_ref = aui.borrow_mut();
        let art = aui_ref.dock_art_mut();
        art.set_gradient(AuiDockArtGradient::Vertical);
        art.set_sash_width(4);
        art.set_pane_padding(2);
    }

    // ── Top toolbar with AuiToolBar (standalone, auto-positions) ─────
    let bottom_text = TextCtrl::new(&frame, "");
    bottom_text.set_value(
        "[INFO ] AuiManager demo ready\n\
         [INFO ] Open the Home tab to change tab icons and dock theme\n",
    );

    // Manteniamo vivo il riferimento: l'inner è già preservato dagli
    // handler registrati, ma la toolbar ha bisogno dell'oggetto vivo
    // finché la finestra esiste.
    let top_toolbar = build_aui_toolbar(&frame, &bottom_text);

    // La toolbar deve vivere come pane dell'AuiManager: da sola si
    // posizionava a y=0 del client, esattamente dove partono i pannelli
    // docked, e restava nascosta sotto notebook/albero/inspector.
    let top_toolbar_pane = AuiPaneInfo::toolbar_with_height("top_toolbar", 50)
        .with_dock_side(AuiDockSide::Top)
        .with_window_ref(&top_toolbar.as_widget_ref())
        .with_button(AuiPaneButton::Pin);
    aui.borrow_mut().add_pane(top_toolbar_pane);

    // ── Left project tree (mock) ────────────────────────────────
    let left_label = StaticText::new(
        &frame,
        "Project tree (mock)\n\n▸ src/\n▸ examples/\n▸ resources/",
    );
    let left_pane = AuiPaneInfo::left("project_tree")
        .with_caption("Project")
        .with_window_ref(&left_label.as_widget_ref())
        .with_best_size(180, 0)
        .with_min_size(120, 0)
        .with_button(AuiPaneButton::Pin)
        .with_auto_hide(true);
    aui.borrow_mut().add_pane(left_pane);

    // ── Center notebook ───────────────────────────────────────────
    let center_host = Panel::new_child(&frame);
    let center_host_ref = center_host.as_widget_ref();
    let notebook = Rc::new(RefCell::new(AuiNotebook::new(
        &*center_host_ref.borrow(),
        0,
        0,
        100,
        100,
    )));

    let tab_icons = build_tab_images();
    notebook.borrow().set_image_list(&tab_icons);

    // ── Home page: live tab + dock theme playground ─────────────
    let page_home = Panel::new_child(&center_host);
    let home_title_idx = Rc::new(Cell::new(0usize));
    let home_icon_idx = Rc::new(Cell::new(0usize));

    let lbl_home_hdr = StaticText::new(
        &page_home,
        "Home — tweak tab strip shape/icons and dock theme, then Apply.",
    );
    let lbl_tabs = StaticText::new(&page_home, "Tab strip (AuiNotebook):");
    let home_status = StaticText::new(
        &page_home,
        "Tab 0 icon index: 0 · title: Home · shape: top bezel",
    );
    let btn_rename = Button::new(&page_home, "Rename Home tab");
    let btn_next_icon = Button::new(&page_home, "Next Home icon");
    let btn_icons_on = Button::new(&page_home, "Icons on all tabs");
    let btn_icons_off = Button::new(&page_home, "Remove tab icons");

    let lbl_tab_shape = StaticText::new(&page_home, "Tab shape / size:");
    let btn_tab_top = Button::new(&page_home, "Tabs top");
    let btn_tab_bottom = Button::new(&page_home, "Tabs bottom");
    let btn_tab_left = Button::new(&page_home, "Tabs left");
    let btn_tab_right = Button::new(&page_home, "Tabs right");
    let btn_tab_compact = Button::new(&page_home, "Compact tabs");
    let btn_tab_large = Button::new(&page_home, "Large tabs");
    let btn_tab_line = Button::new(&page_home, "Line border");
    let btn_tab_plain = Button::new(&page_home, "Plain (no tabs)");

    let lbl_dock = StaticText::new(&page_home, "Dock chrome (AuiDockArt):");
    let btn_grad_v = Button::new(&page_home, "Gradient ▼");
    let btn_grad_h = Button::new(&page_home, "Gradient ═");
    let btn_grad_flat = Button::new(&page_home, "Gradient flat");
    let btn_theme_def = Button::new(&page_home, "Theme: Default");
    let btn_theme_ocean = Button::new(&page_home, "Theme: Ocean");
    let btn_theme_forest = Button::new(&page_home, "Theme: Forest");
    let sash_slider = Slider::new(&page_home, 2, 10, 4);
    let lbl_sash = StaticText::new(&page_home, "Sash width: 4 px");
    let btn_apply = Button::new(&page_home, "Apply layout");
    let chk_caption = CheckBox::new(&page_home, "Tall captions (28 px)");
    chk_caption.set_checked(false);

    let lbl_mini = StaticText::new(
        &page_home,
        "Sample icon buttons (same glyphs as the top toolbar):",
    );
    let mini_new = Button::new(&page_home, "[🆕]");
    let mini_open = Button::new(&page_home, "[📂]");
    let mini_save = Button::new(&page_home, "[💾]");
    let mini_status = StaticText::new(&page_home, "Click an icon button above.");

    let mut row_tabs = BoxSizer::horizontal();
    row_tabs.add(btn_rename.as_widget_ref());
    row_tabs.add(btn_next_icon.as_widget_ref());
    row_tabs.add(btn_icons_on.as_widget_ref());
    row_tabs.add(btn_icons_off.as_widget_ref());

    let mut row_shape = BoxSizer::horizontal();
    row_shape.add(btn_tab_top.as_widget_ref());
    row_shape.add(btn_tab_bottom.as_widget_ref());
    row_shape.add(btn_tab_left.as_widget_ref());
    row_shape.add(btn_tab_right.as_widget_ref());

    let mut row_shape2 = BoxSizer::horizontal();
    row_shape2.add(btn_tab_compact.as_widget_ref());
    row_shape2.add(btn_tab_large.as_widget_ref());
    row_shape2.add(btn_tab_line.as_widget_ref());
    row_shape2.add(btn_tab_plain.as_widget_ref());

    let mut row_grad = BoxSizer::horizontal();
    row_grad.add(btn_grad_v.as_widget_ref());
    row_grad.add(btn_grad_h.as_widget_ref());
    row_grad.add(btn_grad_flat.as_widget_ref());

    let mut row_theme = BoxSizer::horizontal();
    row_theme.add(btn_theme_def.as_widget_ref());
    row_theme.add(btn_theme_ocean.as_widget_ref());
    row_theme.add(btn_theme_forest.as_widget_ref());

    let mut row_mini = BoxSizer::horizontal();
    row_mini.add(mini_new.as_widget_ref());
    row_mini.add(mini_open.as_widget_ref());
    row_mini.add(mini_save.as_widget_ref());

    let mut home_sizer = BoxSizer::vertical();
    home_sizer.add(lbl_home_hdr.as_widget_ref());
    home_sizer.add_spacer(6);
    home_sizer.add(lbl_tabs.as_widget_ref());
    home_sizer.add(home_status.as_widget_ref());
    home_sizer.add_sizer(row_tabs);
    home_sizer.add_spacer(6);
    home_sizer.add(lbl_tab_shape.as_widget_ref());
    home_sizer.add_sizer(row_shape);
    home_sizer.add_sizer(row_shape2);
    home_sizer.add_spacer(8);
    home_sizer.add(lbl_dock.as_widget_ref());
    home_sizer.add_sizer(row_grad);
    home_sizer.add_sizer(row_theme);
    home_sizer.add(lbl_sash.as_widget_ref());
    home_sizer.add(sash_slider.as_widget_ref());
    home_sizer.add(chk_caption.as_widget_ref());
    home_sizer.add(btn_apply.as_widget_ref());
    home_sizer.add_spacer(8);
    home_sizer.add(lbl_mini.as_widget_ref());
    home_sizer.add_sizer(row_mini);
    home_sizer.add(mini_status.as_widget_ref());
    home_sizer.add_stretch(1);
    page_home.set_sizer(home_sizer);

    // ── Tab Settings page ─────────────────────────────────────────
    let page_settings = Panel::new_child(&center_host);

    let lbl_settings_hdr = StaticText::new(&page_settings, "Tab Settings — change tab icons");

    let lbl_change_icon = StaticText::new(&page_settings, "Change icon for tab:");
    let btn_icon_0 = Button::new(&page_settings, "Tab 0 → Home");
    let btn_icon_1 = Button::new(&page_settings, "Tab 1 → Edit");
    let btn_icon_2 = Button::new(&page_settings, "Tab 2 → Log");
    let btn_icon_3 = Button::new(&page_settings, "Tab 3 → Star");
    let btn_icon_4 = Button::new(&page_settings, "Tab 4 → Settings");
    let btn_icon_5 = Button::new(&page_settings, "Tab 5 → Docs");

    let settings_status =
        StaticText::new(&page_settings, "Ready. Click buttons to change tab icons.");

    let settings_nb4 = Rc::clone(&notebook);
    let settings_log4 = bottom_text.clone();
    let settings_lbl0 = settings_status.clone();

    btn_icon_0.on_click(&frame, move || {
        settings_nb4.borrow().set_page_image(0, 0);
        settings_lbl0.set_label("Tab 0 icon → Home (index 0)");
        append_log(&settings_log4, "[tab] Tab 0 icon → Home");
    });

    let settings_nb5 = Rc::clone(&notebook);
    let settings_log5 = bottom_text.clone();
    let settings_lbl1 = settings_status.clone();

    btn_icon_1.on_click(&frame, move || {
        settings_nb5.borrow().set_page_image(1, 1);
        settings_lbl1.set_label("Tab 1 icon → Edit (index 1)");
        append_log(&settings_log5, "[tab] Tab 1 icon → Edit");
    });

    let settings_nb6 = Rc::clone(&notebook);
    let settings_log6 = bottom_text.clone();
    let settings_lbl2 = settings_status.clone();

    btn_icon_2.on_click(&frame, move || {
        settings_nb6.borrow().set_page_image(2, 2);
        settings_lbl2.set_label("Tab 2 icon → Log (index 2)");
        append_log(&settings_log6, "[tab] Tab 2 icon → Log");
    });

    let settings_nb7 = Rc::clone(&notebook);
    let settings_log7 = bottom_text.clone();
    let settings_lbl7 = settings_status.clone();

    btn_icon_3.on_click(&frame, move || {
        settings_nb7.borrow().set_page_image(3, 3);
        settings_lbl7.set_label("Tab 3 icon → Star (index 3)");
        append_log(&settings_log7, "[tab] Tab 3 icon → Star");
    });

    let settings_nb8 = Rc::clone(&notebook);
    let settings_log8 = bottom_text.clone();
    let settings_lbl8 = settings_status.clone();

    btn_icon_4.on_click(&frame, move || {
        settings_nb8.borrow().set_page_image(4, 4);
        settings_lbl8.set_label("Tab 4 icon → Settings (index 4)");
        append_log(&settings_log8, "[tab] Tab 4 icon → Settings");
    });

    let settings_nb9 = Rc::clone(&notebook);
    let settings_log9 = bottom_text.clone();
    let settings_lbl9 = settings_status.clone();

    btn_icon_5.on_click(&frame, move || {
        settings_nb9.borrow().set_page_image(5, 5);
        settings_lbl9.set_label("Tab 5 icon → Docs (index 5)");
        append_log(&settings_log9, "[tab] Tab 5 icon → Docs");
    });

    let mut row_icon1 = BoxSizer::horizontal();
    row_icon1.add(btn_icon_0.as_widget_ref());
    row_icon1.add(btn_icon_1.as_widget_ref());
    row_icon1.add(btn_icon_2.as_widget_ref());

    let mut row_icon2 = BoxSizer::horizontal();
    row_icon2.add(btn_icon_3.as_widget_ref());
    row_icon2.add(btn_icon_4.as_widget_ref());
    row_icon2.add(btn_icon_5.as_widget_ref());

    let mut settings_sizer = BoxSizer::vertical();
    settings_sizer.add(lbl_settings_hdr.as_widget_ref());
    settings_sizer.add_spacer(8);
    settings_sizer.add(lbl_change_icon.as_widget_ref());
    settings_sizer.add_sizer(row_icon1);
    settings_sizer.add_sizer(row_icon2);
    settings_sizer.add_spacer(8);
    settings_sizer.add(settings_status.as_widget_ref());
    settings_sizer.add_stretch(1);
    page_settings.set_sizer(settings_sizer);

    // ── Editor page ─────────────────────────────────────────────
    let page_edit = Panel::new_child(&center_host);
    let lbl_edit = StaticText::new(&page_edit, "Editor — sample document:");
    let editor = TextCtrl::new(&page_edit, "");
    editor.set_value(
        "This page shows a TextCtrl hosted inside an AuiNotebook tab.\n\
         Use the Home tab to rename tabs, swap icons, and recolour the dock chrome.\n\
         Use the Settings tab to add/remove tabs and change icons.\n",
    );
    let mut edit_sizer = BoxSizer::vertical();
    edit_sizer.add(lbl_edit.as_widget_ref());
    edit_sizer.add_with_proportion(editor.as_widget_ref(), 1);
    page_edit.set_sizer(edit_sizer);

    // ── Log page (in-notebook event list) ─────────────────────────
    let page_log = Panel::new_child(&center_host);
    let lbl_log = StaticText::new(&page_log, "Event log (notebook scope):");
    let log_page = TextCtrl::new(&page_log, "");
    log_page.set_value("[notebook] Home, Settings, Editor and Log tabs ready.\n");
    let mut log_sizer = BoxSizer::vertical();
    log_sizer.add(lbl_log.as_widget_ref());
    log_sizer.add_with_proportion(log_page.as_widget_ref(), 1);
    page_log.set_sizer(log_sizer);

    {
        let home_h = page_home.as_widget_ref().borrow().native_handle();
        let settings_h = page_settings.as_widget_ref().borrow().native_handle();
        let edit_h = page_edit.as_widget_ref().borrow().native_handle();
        let log_h = page_log.as_widget_ref().borrow().native_handle();
        let mut nb = notebook.borrow_mut();
        nb.add_page_handle("Home", home_h);
        nb.add_page_handle("Settings", settings_h);
        nb.add_page_handle("Editor", edit_h);
        nb.add_page_handle("Log", log_h);
        nb.set_page_image(0, 0);
        nb.set_page_image(1, 4); // Settings icon
        nb.set_page_image(2, 1);
        nb.set_page_image(3, 2);
        nb.set_selection(0);
        nb.relayout();
    }

    let center_pane = AuiPaneInfo::center("main_notebook")
        .with_caption("Workspace")
        .with_window_ref(&center_host.as_widget_ref());
    aui.borrow_mut().add_pane(center_pane);

    // ── Right inspector (live theme readout) ──────────────────────
    let right_label = StaticText::new(
        &frame,
        &format!(
            "Inspector\n\n{}\n\nPane count: 4\nCenter: main_notebook",
            theme_summary(&aui.borrow())
        ),
    );
    let right_pane = AuiPaneInfo::right("inspector")
        .with_caption("Inspector")
        .with_window_ref(&right_label.as_widget_ref())
        .with_best_size(220, 0)
        .with_min_size(160, 0)
        .with_button(AuiPaneButton::Maximize);
    aui.borrow_mut().add_pane(right_pane);

    // ── Bottom toolbar (AuiToolBar inside an AuiManager pane) ────
    let bottom_toolbar = build_bottom_toolbar(&frame, &bottom_text);
    let bottom_toolbar_pane = AuiPaneInfo::bottom("bottom_toolbar")
        .with_dock_side(AuiDockSide::Bottom)
        .with_window_ref(&bottom_toolbar.as_widget_ref())
        .with_caption("Toolbar")
        .with_caption_visible(false)
        .with_best_size(0, 50)
        .with_min_size(0, 40)
        .with_button(AuiPaneButton::Pin)
        .with_auto_hide(true);
    aui.borrow_mut().add_pane(bottom_toolbar_pane);

    // ── Bottom log (global) ─────────────────────────────────────
    let bottom_pane = AuiPaneInfo::bottom("log")
        .with_caption("Log")
        .with_window_ref(&bottom_text.as_widget_ref())
        .with_best_size(0, 100)
        .with_min_size(0, 60);
    aui.borrow_mut().add_pane(bottom_pane);

    aui.borrow_mut().update(&frame);
    relayout_notebook(&aui.borrow(), &notebook.borrow());
    {
        let nb = notebook.borrow();
        for (page, icon) in [(0, 0), (1, 4), (2, 1), (3, 2)] {
            nb.set_page_image(page, icon);
        }
    }

    // ── Home tab handlers ─────────────────────────────────────────
    {
        let nb = Rc::clone(&notebook);
        let status = home_status.clone();
        let idx = Rc::clone(&home_title_idx);
        let log = log_page.clone();
        btn_rename.on_click(&frame, move || {
            let next = (idx.get() + 1) % HOME_TITLES.len();
            idx.set(next);
            let title = HOME_TITLES[next];
            nb.borrow_mut().set_page_text(0, title);
            status.set_label(&format!("Tab 0 title renamed to \"{title}\""));
            append_log(&log, &format!("[tab] rename Home → {title}"));
        });
    }

    {
        let nb = Rc::clone(&notebook);
        let status = home_status.clone();
        let idx = Rc::clone(&home_icon_idx);
        let log = log_page.clone();
        btn_next_icon.on_click(&frame, move || {
            let next = (idx.get() + 1) % TAB_ICON_CYCLE.len();
            idx.set(next);
            let icon = TAB_ICON_CYCLE[next];
            nb.borrow().set_page_image(0, icon);
            let msg = if icon < 0 {
                "Tab 0 icon removed".to_string()
            } else {
                format!("Tab 0 icon index: {icon}")
            };
            status.set_label(&msg);
            append_log(&log, &format!("[tab] Home icon → {icon}"));
        });
    }

    {
        let nb = Rc::clone(&notebook);
        let log = log_page.clone();
        btn_icons_on.on_click(&frame, move || {
            for (page, icon) in (0..4).zip(0..4) {
                nb.borrow().set_page_image(page, icon);
            }
            append_log(&log, "[tab] icons enabled on all pages");
        });
    }

    {
        let nb = Rc::clone(&notebook);
        let log = log_page.clone();
        btn_icons_off.on_click(&frame, move || {
            for page in 0..4 {
                nb.borrow().set_page_image(page, -1);
            }
            append_log(&log, "[tab] icons removed from all pages");
        });
    }

    #[cfg(target_os = "macos")]
    {
        macro_rules! tab_shape_btn {
            ($btn:expr, $label:expr, $action:expr) => {{
                let nb = Rc::clone(&notebook);
                let status = home_status.clone();
                let log = log_page.clone();
                let label: &'static str = $label;
                $btn.on_click(&frame, move || {
                    $action(&nb.borrow());
                    status.set_label(&format!("Tab shape: {label}"));
                    append_log(&log, &format!("[tab] shape → {label}"));
                });
            }};
        }

        tab_shape_btn!(btn_tab_top, "top bezel", |nb: &AuiNotebook| nb
            .set_tab_chrome(TabViewChrome::TopTabsBezel));
        tab_shape_btn!(btn_tab_bottom, "bottom bezel", |nb: &AuiNotebook| nb
            .set_tab_chrome(TabViewChrome::BottomTabs));
        tab_shape_btn!(btn_tab_left, "left bezel", |nb: &AuiNotebook| nb
            .set_tab_chrome(TabViewChrome::LeftTabs));
        tab_shape_btn!(btn_tab_right, "right bezel", |nb: &AuiNotebook| nb
            .set_tab_chrome(TabViewChrome::RightTabs));
        tab_shape_btn!(btn_tab_compact, "compact size", |nb: &AuiNotebook| nb
            .set_tab_control_size(TabControlSize::Small));
        tab_shape_btn!(btn_tab_large, "large size", |nb: &AuiNotebook| nb
            .set_tab_control_size(TabControlSize::Large));
        tab_shape_btn!(btn_tab_line, "line border (no tabs)", |nb: &AuiNotebook| nb
            .set_tab_chrome(TabViewChrome::NoTabsLine));
        tab_shape_btn!(btn_tab_plain, "plain (no border)", |nb: &AuiNotebook| {
            nb.set_tab_chrome(TabViewChrome::NoTabsNoBorder);
            nb.set_tab_draws_background(false);
        });
    }

    let apply_layout: Rc<dyn Fn()> = {
        let aui = Rc::clone(&aui);
        let frame_apply = frame.clone();
        let nb = Rc::clone(&notebook);
        let inspector = right_label.clone();
        let bottom = bottom_text.clone();
        let sash_lbl = lbl_sash.clone();
        let sash = sash_slider.clone();
        let tall = chk_caption.clone();
        Rc::new(move || {
            aui.borrow_mut()
                .dock_art_mut()
                .set_sash_width(sash.get_value() as u32);
            aui.borrow_mut()
                .dock_art_mut()
                .set_caption_height(if tall.is_checked() { 28 } else { 21 });
            aui.borrow_mut().update(&frame_apply);
            relayout_notebook(&aui.borrow(), &nb.borrow());
            inspector.set_label(&format!(
                "Inspector\n\n{}\n\nPane count: {}\nCenter: main_notebook",
                theme_summary(&aui.borrow()),
                aui.borrow().pane_count()
            ));
            sash_lbl.set_label(&format!("Sash width: {} px", sash.get_value()));
            append_log(&bottom, "[dock] layout updated");
        })
    };

    {
        let aui = Rc::clone(&aui);
        let log = log_page.clone();
        let apply = Rc::clone(&apply_layout);
        btn_grad_v.on_click(&frame, move || {
            aui.borrow_mut()
                .dock_art_mut()
                .set_gradient(AuiDockArtGradient::Vertical);
            append_log(&log, "[dock] gradient → vertical");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let log = log_page.clone();
        let apply = Rc::clone(&apply_layout);
        btn_grad_h.on_click(&frame, move || {
            aui.borrow_mut()
                .dock_art_mut()
                .set_gradient(AuiDockArtGradient::Horizontal);
            append_log(&log, "[dock] gradient → horizontal");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let log = log_page.clone();
        let apply = Rc::clone(&apply_layout);
        btn_grad_flat.on_click(&frame, move || {
            aui.borrow_mut()
                .dock_art_mut()
                .set_gradient(AuiDockArtGradient::None);
            append_log(&log, "[dock] gradient → flat");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let log = log_page.clone();
        let apply = Rc::clone(&apply_layout);
        btn_theme_def.on_click(&frame, move || {
            apply_dock_theme(&mut aui.borrow_mut(), "default");
            append_log(&log, "[dock] theme → default");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let log = log_page.clone();
        let apply = Rc::clone(&apply_layout);
        btn_theme_ocean.on_click(&frame, move || {
            apply_dock_theme(&mut aui.borrow_mut(), "ocean");
            append_log(&log, "[dock] theme → ocean");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let log = log_page.clone();
        let apply = Rc::clone(&apply_layout);
        btn_theme_forest.on_click(&frame, move || {
            apply_dock_theme(&mut aui.borrow_mut(), "forest");
            append_log(&log, "[dock] theme → forest");
            apply();
        });
    }
    {
        let apply = Rc::clone(&apply_layout);
        btn_apply.on_click(&frame, move || apply());
    }
    {
        let lbl = lbl_sash.clone();
        let sash = sash_slider.clone();
        sash_slider.on_value_change(&frame, move || {
            lbl.set_label(&format!("Sash width: {} px", sash.get_value()));
        });
    }

    {
        let log = log_page.clone();
        let status = mini_status.clone();
        mini_new.on_click(&frame, move || {
            append_log(&log, "[mini-toolbar] New clicked");
            status.set_label("Mini toolbar: New");
        });
    }
    {
        let log = log_page.clone();
        let status = mini_status.clone();
        mini_open.on_click(&frame, move || {
            append_log(&log, "[mini-toolbar] Open clicked");
            status.set_label("Mini toolbar: Open");
        });
    }
    {
        let log = log_page.clone();
        let status = mini_status.clone();
        mini_save.on_click(&frame, move || {
            append_log(&log, "[mini-toolbar] Save clicked");
            status.set_label("Mini toolbar: Save");
        });
    }

    {
        let nb = Rc::clone(&notebook);
        let log = log_page.clone();
        notebook
            .borrow_mut()
            .on_selection_change(&frame, move |index| {
                append_log(&log, &format!("[tab] selection → {index}"));
                let _ = nb.borrow().selection();
            });
    }

    // ── Bottom toolbar: layout buttons ─────────────────────────────
    // NB: non usare `bottom_toolbar` come nome/id per questo pannello:
    // shadowava l'AuiToolBar omonima (riga ~531) creando **due panelli
    // con lo stesso id "bottom_toolbar"** sovrapposti in basso, con i
    // bottoni alti 200 px dentro un pane di 66 px.
    let actions_panel = Panel::new(&frame);
    let btn_hide_left = Button::new(&actions_panel, "Hide left");
    let btn_show_left = Button::new(&actions_panel, "Show left");
    let btn_save = Button::new(&actions_panel, "Save perspective");
    let btn_print = Button::new(&actions_panel, "Print pane count");

    let mut bt_sizer = BoxSizer::horizontal();
    bt_sizer.add(btn_hide_left.as_widget_ref());
    bt_sizer.add_spacer(5);
    bt_sizer.add(btn_show_left.as_widget_ref());
    bt_sizer.add_spacer(10);
    bt_sizer.add(btn_save.as_widget_ref());
    bt_sizer.add_spacer(5);
    bt_sizer.add(btn_print.as_widget_ref());
    actions_panel.set_sizer(bt_sizer);

    let actions_pane = AuiPaneInfo::bottom("actions")
        .with_caption("Actions")
        .with_window_ref(&actions_panel.as_widget_ref())
        .with_best_size(0, 45)
        .with_min_size(0, 40);
    aui.borrow_mut().add_pane(actions_pane);

    {
        let aui = Rc::clone(&aui);
        let apply = Rc::clone(&apply_layout);
        btn_hide_left.on_click(&frame, move || {
            aui.borrow_mut().hide_pane("project_tree");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let apply = Rc::clone(&apply_layout);
        btn_show_left.on_click(&frame, move || {
            aui.borrow_mut().show_pane("project_tree");
            apply();
        });
    }
    {
        let aui = Rc::clone(&aui);
        let bottom = bottom_text.clone();
        btn_save.on_click(&frame, move || {
            let snap = aui.borrow().save_perspective();
            append_log(
                &bottom,
                &format!("[perspective] saved ({} bytes)", snap.len()),
            );
        });
    }
    {
        let aui = Rc::clone(&aui);
        let bottom = bottom_text.clone();
        btn_print.on_click(&frame, move || {
            append_log(
                &bottom,
                &format!("[info] AuiManager has {} panes", aui.borrow().pane_count()),
            );
        });
    }

    println!("AuiManager demo ready — open the Home tab to explore tab & dock styling.");

    {
        let aui = Rc::clone(&aui);
        let frame_for_resize = frame.clone();
        let nb = Rc::clone(&notebook);
        frame.on_resize(move |_width, _height| {
            aui.borrow_mut().update(&frame_for_resize);
            relayout_notebook(&aui.borrow(), &nb.borrow());
        });
    }

    app.run(frame);
}
