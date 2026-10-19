//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `minieditor_esempio` — Mini Editor with a colourful dockable toolbar, native
//! Win32 tooltips on every tool button, per-widget [`ToolTip`]s on the
//! rest of the UI, and Windows 11 rounded corners via DWM.
//!
//! Run with:
//! ```bash
//! cargo run --example minieditor_esempio
//! ```
//! On Windows 11 the library embeds Common Controls v6 + PerMonitorV2 via
//! `build.rs` and applies Mica / dark title bar automatically on every
//! `Frame::builder().build()`.

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use ru_wx::{
    Accelerator, App, AuiDockSide, AuiToolBar, BitmapBundle, Button, CheckBox, FileDialog,
    FileDialogStyle, ImageList, Menu, MenuBar, MessageDialog, MessageDialogIcon,
    MessageDialogStyle, StaticText, StatusBar, TextCtrl, ToolTip, TopLevelWindow,
    WindowCornerPreference,
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

const TXT_WILDCARD: &str = "File di testo (*.txt)|*.txt|Tutti i file (*.*)|*.*";

#[cfg(target_os = "windows")]
const WM_CUT: u32 = 0x0300;
#[cfg(target_os = "windows")]
const WM_COPY: u32 = 0x0301;
#[cfg(target_os = "windows")]
const WM_PASTE: u32 = 0x0302;
#[cfg(not(target_os = "windows"))]
const WM_CUT: u32 = 0;
#[cfg(not(target_os = "windows"))]
const WM_COPY: u32 = 0;
#[cfg(not(target_os = "windows"))]
const WM_PASTE: u32 = 0;

/// Shared document state for menu bar, toolbar and status line.
struct DocSession {
    frame: ru_wx::Frame,
    editor: TextCtrl,
    status: StatusBar,
    path: RefCell<Option<PathBuf>>,
    dirty: RefCell<bool>,
    /// Suppress `on_change` while loading or clearing programmatically.
    loading: RefCell<bool>,
}

impl DocSession {
    fn refresh_chrome(&self) {
        let dirty = *self.dirty.borrow();
        let path = self.path.borrow();
        let title_name = path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Senza titolo");
        let star = if dirty { " *" } else { "" };
        self.frame
            .set_title(&format!("minieditor_esempio — {title_name}{star}"));

        let path_hint = path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Nuovo documento".to_string());
        drop(path);
        self.status.set_status_text(
            &format!(
                "{path_hint}{} — {} caratteri",
                if dirty { " (modificato)" } else { "" },
                self.editor.get_value().chars().count()
            ),
            0,
        );
    }

    fn mark_dirty(&self) {
        if *self.loading.borrow() {
            return;
        }
        if !*self.dirty.borrow() {
            *self.dirty.borrow_mut() = true;
            self.refresh_chrome();
        } else {
            self.refresh_chrome();
        }
    }

    fn apply_loaded(&self, text: String, path: Option<PathBuf>) {
        *self.loading.borrow_mut() = true;
        self.editor.set_value(&text);
        *self.path.borrow_mut() = path;
        *self.dirty.borrow_mut() = false;
        *self.loading.borrow_mut() = false;
        self.refresh_chrome();
    }

    fn new_document(&self) {
        self.apply_loaded(String::new(), None);
        self.status.set_status_text("Nuovo documento.", 0);
    }

    fn open_document(&self) {
        let mut dlg = FileDialog::new(&self.frame, FileDialogStyle::Open);
        dlg.set_title("Apri documento");
        dlg.set_wildcard(TXT_WILDCARD);
        if let Some(dir) = self.path.borrow().as_ref().and_then(|p| p.parent()) {
            dlg.set_directory(&dir.display().to_string());
        }
        let Some(path) = dlg.show_modal() else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.apply_loaded(text, Some(PathBuf::from(path)));
                self.status.set_status_text("File aperto.", 0);
            }
            Err(err) => {
                self.status
                    .set_status_text(&format!("Errore apertura: {err}"), 0);
            }
        }
    }

    fn save_document(&self) {
        if self.path.borrow().is_some() {
            self.save_to_known_path();
        } else {
            self.save_document_as();
        }
    }

    fn save_document_as(&self) {
        let mut dlg = FileDialog::new(&self.frame, FileDialogStyle::Save);
        dlg.set_title("Salva documento");
        dlg.set_wildcard(TXT_WILDCARD);
        if let Some(path) = self.path.borrow().as_ref() {
            dlg.set_directory(
                &path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            );
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                dlg.set_filename(name);
            }
        }
        let Some(path) = dlg.show_modal() else {
            return;
        };
        *self.path.borrow_mut() = Some(PathBuf::from(path));
        self.save_to_known_path();
    }

    fn save_to_known_path(&self) {
        let Some(path) = self.path.borrow().clone() else {
            self.save_document_as();
            return;
        };
        match std::fs::write(&path, self.editor.get_value()) {
            Ok(()) => {
                *self.dirty.borrow_mut() = false;
                self.refresh_chrome();
                self.status.set_status_text("File salvato.", 0);
            }
            Err(err) => {
                self.status
                    .set_status_text(&format!("Errore salvataggio: {err}"), 0);
            }
        }
    }

    fn cut(&self) {
        edit_command(&self.editor, WM_CUT);
        self.mark_dirty();
        self.status.set_status_text("Taglia (selezione editor).", 0);
    }

    fn copy(&self) {
        edit_command(&self.editor, WM_COPY);
        self.status.set_status_text("Copia (selezione editor).", 0);
    }

    fn paste(&self) {
        edit_command(&self.editor, WM_PASTE);
        self.mark_dirty();
        self.status.set_status_text("Incolla nell'editor.", 0);
    }

    fn undo(&self) {
        if self.editor.can_undo() {
            self.editor.undo();
            self.mark_dirty();
            self.status.set_status_text("Annulla ultima modifica.", 0);
        }
    }

    fn select_all(&self) {
        select_all_text(&self.editor);
        self.status.set_status_text("Seleziona tutto.", 0);
    }

    fn show_about(&self) {
        let dlg = MessageDialog::new(
            &self.frame,
            "Informazioni su minieditor_esempio",
            "Mini editor di testo ru_wx\n\n\
             • Menu File / Modifica con scorciatoie\n\
             • Toolbar colorata agganciabile (AuiToolBar)\n\
             • Tooltip nativi Win32 + tooltip ru_wx\n\
             • Angoli arrotondati Windows 11 (DWM)\n\n\
             https://ru_wx.easytaskflow.app/",
            MessageDialogStyle::Ok,
            MessageDialogIcon::Information,
        );
        let _ = dlg.show_modal();
    }
}

#[cfg(target_os = "windows")]
fn edit_command(editor: &TextCtrl, msg: u32) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    let hwnd = editor.as_widget_ref().borrow().native_handle() as HWND;
    // SAFETY: `hwnd` is a valid multiline EDIT control; standard edit messages.
    unsafe {
        SendMessageW(hwnd, msg, 0, 0);
    }
}

#[cfg(not(target_os = "windows"))]
fn edit_command(_editor: &TextCtrl, _msg: u32) {}

#[cfg(target_os = "windows")]
fn select_all_text(editor: &TextCtrl) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Controls::EM_SETSEL;
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    let hwnd = editor.as_widget_ref().borrow().native_handle() as HWND;
    // SAFETY: standard EM_SETSEL on EDIT control.
    unsafe {
        SendMessageW(hwnd, EM_SETSEL, 0, -1);
    }
}

#[cfg(not(target_os = "windows"))]
fn select_all_text(_editor: &TextCtrl) {}

fn main() {
    let app = App::new();

    let window = TopLevelWindow::new("minieditor_esempio — Senza titolo", 780, 580);
    let _ = window.set_window_corner_preference(WindowCornerPreference::Default);
    let frame = window.frame().clone();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Pronto — usa File / Modifica o la toolbar.", 0);

    let editor = TextCtrl::multiline(
        &frame,
        "Benvenuto in minieditor_esempio!\n\n\
         Prova:\n\
         • File → Apri / Salva oppure i pulsanti della toolbar\n\
         • Modifica → Taglia / Copia / Incolla (anche Ctrl+X/C/V)\n\
         • Seleziona del testo nell'editor prima di tagliare o copiare\n\
         • Gripper toolbar (≡) per staccare e riagganciare\n",
    );

    let session = Rc::new(DocSession {
        frame: frame.clone(),
        editor: editor.clone(),
        status: status.clone(),
        path: RefCell::new(None),
        dirty: RefCell::new(false),
        loading: RefCell::new(false),
    });

    let session_dirty = session.clone();
    editor.on_change(&frame, move || session_dirty.mark_dirty());

    // ── Menu bar ────────────────────────────────────────────────────
    let mut file_menu = Menu::new("&File");
    let s = session.clone();
    file_menu.append_with_shortcut(
        "&Nuovo",
        Accelerator::parse("Ctrl+N").unwrap(),
        &frame,
        move || s.new_document(),
    );
    let s = session.clone();
    file_menu.append_with_shortcut(
        "&Apri…",
        Accelerator::parse("Ctrl+O").unwrap(),
        &frame,
        move || s.open_document(),
    );
    let s = session.clone();
    file_menu.append_with_shortcut(
        "&Salva",
        Accelerator::parse("Ctrl+S").unwrap(),
        &frame,
        move || s.save_document(),
    );
    let s = session.clone();
    file_menu.append("Salva &con nome…", &frame, move || s.save_document_as());
    file_menu.append_separator();
    let frame_exit = frame.clone();
    file_menu.append("E&sci", &frame, move || frame_exit.close());

    let mut edit_menu = Menu::new("&Modifica");
    let s = session.clone();
    edit_menu.append_with_shortcut(
        "Annulla",
        Accelerator::parse("Ctrl+Z").unwrap(),
        &frame,
        move || s.undo(),
    );
    edit_menu.append_separator();
    let s = session.clone();
    edit_menu.append_with_shortcut(
        "Taglia",
        Accelerator::parse("Ctrl+X").unwrap(),
        &frame,
        move || s.cut(),
    );
    let s = session.clone();
    edit_menu.append_with_shortcut(
        "&Copia",
        Accelerator::parse("Ctrl+C").unwrap(),
        &frame,
        move || s.copy(),
    );
    let s = session.clone();
    edit_menu.append_with_shortcut(
        "&Incolla",
        Accelerator::parse("Ctrl+V").unwrap(),
        &frame,
        move || s.paste(),
    );
    edit_menu.append_separator();
    let s = session.clone();
    edit_menu.append_with_shortcut(
        "Seleziona &tutto",
        Accelerator::parse("Ctrl+A").unwrap(),
        &frame,
        move || s.select_all(),
    );

    let mut help_menu = Menu::new("&?");

    let s = session.clone();
    help_menu.append("&Informazioni…", &frame, move || s.show_about());

    let mut menu_bar = MenuBar::new();
    menu_bar.append(file_menu);
    menu_bar.append(edit_menu);
    menu_bar.append(help_menu);
    frame.set_menu_bar(menu_bar);

    // ── Colourful toolbar ───────────────────────────────────────────
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
    aui.add_tool(ID_TOOL_NEW, "Nuovo documento (Ctrl+N)", 0);
    aui.add_tool(ID_TOOL_OPEN, "Apri file… (Ctrl+O)", 1);
    aui.add_tool(ID_TOOL_SAVE, "Salva documento (Ctrl+S)", 2);
    aui.add_separator();
    aui.add_tool(ID_TOOL_CUT, "Taglia selezione (Ctrl+X)", 3);
    aui.add_tool(ID_TOOL_COPY, "Copia selezione (Ctrl+C)", 4);
    aui.add_tool(ID_TOOL_PASTE, "Incolla (Ctrl+V)", 5);
    aui.realize();

    let session_tools = session.clone();
    aui.on_tool_clicked(&frame, move |id| match id {
        ID_TOOL_NEW => session_tools.new_document(),
        ID_TOOL_OPEN => session_tools.open_document(),
        ID_TOOL_SAVE => session_tools.save_document(),
        ID_TOOL_CUT => session_tools.cut(),
        ID_TOOL_COPY => session_tools.copy(),
        ID_TOOL_PASTE => session_tools.paste(),
        _ => {}
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

    let content_top = TOOLBAR_H + 10;

    let lbl_hint = StaticText::new(&frame, "Editor — seleziona del testo per Taglia/Copia.");
    lbl_hint
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top);
    lbl_hint.as_widget_ref().borrow_mut().set_size(500, 22);
    ToolTip::new("L'area di modifica supporta le operazioni standard Win32 sull'EDIT.")
        .attach(&lbl_hint.as_widget_ref());

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

    use std::cell::RefCell as RcCell;
    let cycle_idx: Rc<RcCell<u8>> = Rc::new(RcCell::new(0));
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
        "Menu File/Modifica funzionanti · titolo con asterisco se modificato ·\n\
         conteggio caratteri nella status bar · Salva con nome per file nuovi.",
    );
    lbl_info
        .as_widget_ref()
        .borrow_mut()
        .set_position(12, content_top + 290);
    lbl_info.as_widget_ref().borrow_mut().set_size(740, 40);

    session.refresh_chrome();
    app.run(frame);
}
