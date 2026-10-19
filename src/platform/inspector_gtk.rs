//! Debug inspector — backend GTK4/Linux (Livello 3).
//!
//! Pendant Linux di `inspector_windows.rs` / `inspector_macos.rs`: cammina
//! l'albero GTK4 dei widget, legge i metadati della finestra via X11,
//! inietta azioni sui widget e cattura lo schermo con `XGetImage`.
//!
//! Gli id dei nodi sono gli handle runtime ru_wx (assegnati da
//! `alloc_widget_handle`); i widget interni non registrati usano un id
//! sintetico `gtk:0x..` con `source: inferred`, come `mac:0x..` su macOS.
//!
//! Le chiamate X11 usano una connessione dedicata (non quella di GDK) con
//! un error handler che ignora gli errori di protocollo: `XGetImage` su un
//! rettangolo fuori dalla root window non deve abortire il processo.

use std::collections::HashMap;
use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};

use gdk4_x11::X11Surface;
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::*;
use gtk4::{CheckButton, DropDown, Label, ListBox, Notebook, Switch, TextView, Window};

use super::declared;
use super::gtk as backend;
use super::inspector::{source, Bounds, ComponentNode, InjectedEvent, Selector, WindowInfo};
use super::NativeHandle;

const MAX_DEPTH: usize = 24;
const MAX_NODES: usize = 4000;
// Budget della tabella in `extra`, come `read_listview` su Windows (§6.4).
const MAX_TABLE_ROWS: usize = 200;
const MAX_TABLE_COLS: usize = 32;

// ── X11: FFI manuale su libX11 ──────────────────────────────────────

// Duplicata (ABI-equivalente, tipi opachi distinti) rispetto a
// `gtk::screen_x11`: gating diverso (questo modulo è debug/feature).
#[allow(clashing_extern_declarations)]
mod x11 {
    use super::*;

    #[repr(C)]
    pub struct Display {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct XErrorEvent {
        _private: [u8; 0],
    }

    /// Solo i campi usati dalla conversione pixel; la struct reale prosegue
    /// con la tabella delle funzioni (ABI stabile da decenni).
    #[repr(C)]
    pub struct XImage {
        pub width: c_int,
        pub height: c_int,
        pub xoffset: c_int,
        pub format: c_int,
        pub data: *mut c_char,
        pub byte_order: c_int,
        pub bitmap_unit: c_int,
        pub bitmap_bit_order: c_int,
        pub bitmap_pad: c_int,
        pub depth: c_int,
        pub bytes_per_line: c_int,
        pub bits_per_pixel: c_int,
        pub red_mask: c_ulong,
        pub green_mask: c_ulong,
        pub blue_mask: c_ulong,
        pub obdata: *mut c_void,
    }

    pub type ErrorHandler = Option<extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>;

    #[link(name = "X11")]
    extern "C" {
        pub fn XOpenDisplay(name: *const c_char) -> *mut Display;
        pub fn XCloseDisplay(dpy: *mut Display) -> c_int;
        pub fn XDefaultScreen(dpy: *mut Display) -> c_int;
        pub fn XDefaultRootWindow(dpy: *mut Display) -> c_ulong;
        pub fn XDisplayWidth(dpy: *mut Display, screen: c_int) -> c_int;
        pub fn XDisplayHeight(dpy: *mut Display, screen: c_int) -> c_int;
        pub fn XGetGeometry(
            dpy: *mut Display,
            drawable: c_ulong,
            root_return: *mut c_ulong,
            x_return: *mut c_int,
            y_return: *mut c_int,
            width_return: *mut c_uint,
            height_return: *mut c_uint,
            border_width_return: *mut c_uint,
            depth_return: *mut c_uint,
        ) -> c_int;
        pub fn XGetImage(
            dpy: *mut Display,
            drawable: c_ulong,
            x: c_int,
            y: c_int,
            width: c_uint,
            height: c_uint,
            plane_mask: c_ulong,
            format: c_int,
        ) -> *mut XImage;
        pub fn XDestroyImage(image: *mut XImage) -> c_int;
        pub fn XSync(dpy: *mut Display, discard: c_int) -> c_int;
        pub fn XSetErrorHandler(handler: ErrorHandler) -> ErrorHandler;
    }

    pub const Z_PIXMAP: c_int = 2;
}

/// Error handler che ignora ogni errore di protocollo X (solo per la
/// nostra connessione dedicata, ripristinato alla chiusura).
extern "C" fn ignore_x_error(_d: *mut x11::Display, _e: *mut x11::XErrorEvent) -> c_int {
    0
}

/// Connessione X11 dedicata all'inspector, con error handler silenzioso.
struct X11Conn {
    dpy: *mut x11::Display,
    prev: x11::ErrorHandler,
}

impl X11Conn {
    fn open() -> Option<Self> {
        unsafe {
            let dpy = x11::XOpenDisplay(std::ptr::null());
            if dpy.is_null() {
                return None;
            }
            let prev = x11::XSetErrorHandler(Some(ignore_x_error));
            Some(Self { dpy, prev })
        }
    }

    /// Geometria di una finestra: per una top-level il parent è la root
    /// window, quindi `x`/`y` sono già coordinate schermo.
    fn geometry(&self, xid: c_ulong) -> Option<(i32, i32, u32, u32)> {
        unsafe {
            let mut root = 0 as c_ulong;
            let mut x: c_int = 0;
            let mut y: c_int = 0;
            let mut w: c_uint = 0;
            let mut h: c_uint = 0;
            let mut border: c_uint = 0;
            let mut depth: c_uint = 0;
            let ok = x11::XGetGeometry(
                self.dpy,
                xid,
                &mut root,
                &mut x,
                &mut y,
                &mut w,
                &mut h,
                &mut border,
                &mut depth,
            );
            x11::XSync(self.dpy, 0);
            if ok == 0 {
                None
            } else {
                Some((x, y, w, h))
            }
        }
    }

    fn screen_size(&self) -> (u32, u32) {
        unsafe {
            let screen = x11::XDefaultScreen(self.dpy);
            let w = x11::XDisplayWidth(self.dpy, screen);
            let h = x11::XDisplayHeight(self.dpy, screen);
            (w.max(1) as u32, h.max(1) as u32)
        }
    }

    /// Cattura un rettangolo della root window in RGBA (PNG-encoded dal
    /// chiamante).
    fn capture(&self, x: i32, y: i32, w: u32, h: u32) -> Result<image::RgbaImage, String> {
        unsafe {
            let root = x11::XDefaultRootWindow(self.dpy);
            let img = x11::XGetImage(self.dpy, root, x, y, w, h, !0 as c_ulong, x11::Z_PIXMAP);
            x11::XSync(self.dpy, 0);
            if img.is_null() {
                return Err("cattura X11 fallita".to_string());
            }
            let result = image_from_ximage(img);
            x11::XDestroyImage(img);
            result
        }
    }
}

impl Drop for X11Conn {
    fn drop(&mut self) {
        unsafe {
            x11::XSync(self.dpy, 0);
            x11::XSetErrorHandler(self.prev);
            x11::XCloseDisplay(self.dpy);
        }
    }
}

/// Converte un `XImage` 32 bpp (ZPixmap) in `RgbaImage`: i canali sono
/// estratti dalle maschere RGB e l'alpha è forzata opaca.
///
/// # Sicurezza
/// `img` è un `XImage` valido fornito da `XGetImage`; i campi letti sono
/// i primi della struct (ABI stabile) e `data` copre `bytes_per_line*height`.
unsafe fn image_from_ximage(img: *mut x11::XImage) -> Result<image::RgbaImage, String> {
    unsafe {
        let w = (*img).width;
        let h = (*img).height;
        if w <= 0 || h <= 0 || (*img).data.is_null() {
            return Err("cattura X11: immagine vuota".to_string());
        }
        let bpp = (*img).bits_per_pixel;
        if bpp != 32 {
            return Err(format!("cattura X11: {bpp} bpp non supportato"));
        }
        let stride = (*img).bytes_per_line as usize;
        let data = std::slice::from_raw_parts((*img).data as *const u8, stride * h as usize);
        let red = (*img).red_mask as u32;
        let green = (*img).green_mask as u32;
        let blue = (*img).blue_mask as u32;
        let le = (*img).byte_order == 0; // LSBFirst

        let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
        for row in 0..h as usize {
            let line = &data[row * stride..];
            for col in 0..w as usize {
                let off = col * 4;
                let bytes = [line[off], line[off + 1], line[off + 2], line[off + 3]];
                let pixel = if le {
                    u32::from_le_bytes(bytes)
                } else {
                    u32::from_be_bytes(bytes)
                };
                rgba.push(channel(pixel, red));
                rgba.push(channel(pixel, green));
                rgba.push(channel(pixel, blue));
                rgba.push(255);
            }
        }
        image::RgbaImage::from_raw(w as u32, h as u32, rgba)
            .ok_or_else(|| "cattura X11: buffer immagine non valido".to_string())
    }
}

/// Estrae un canale da un pixel data la maschera, scala su 0..255.
fn channel(pixel: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = (mask >> shift) as u64;
    let value = ((pixel & mask) >> shift) as u64;
    ((value * 255 + max / 2) / max) as u8
}

// ── Mappa inversa widget → handle ───────────────────────────────────

/// Chiave di un widget GTK per la mappa inversa (puntatore GObject).
fn widget_key(widget: &gtk4::Widget) -> usize {
    let ptr: *mut gtk4::ffi::GtkWidget = widget.to_glib_none().0;
    ptr as usize
}

/// `handle runtime → widget` registrati in `gtk::VIEWS`, invertiti in
/// `puntatore → handle` per mappare i nodi del walk agli id ru_wx.
fn handle_map() -> HashMap<usize, NativeHandle> {
    backend::registered_views()
        .into_iter()
        .map(|(handle, widget)| (widget_key(&widget), handle))
        .collect()
}

// ── Finestra / geometria ────────────────────────────────────────────

/// XID della finestra GTK (solo backend X11).
fn window_xid(window: &Window) -> Option<c_ulong> {
    let surface = window.surface()?;
    let x11_surface = surface.downcast::<X11Surface>().ok()?;
    Some(x11_surface.xid())
}

/// Posizione (x, y) della finestra in coordinate schermo; `(0, 0)` come
/// fallback senza server X.
fn window_origin(window: &Window) -> (i32, i32) {
    let (Some(xid), Some(conn)) = (window_xid(window), X11Conn::open()) else {
        return (0, 0);
    };
    match conn.geometry(xid) {
        Some((x, y, _, _)) => (x, y),
        None => (0, 0),
    }
}

/// Bounds di un widget in coordinate schermo (origine finestra + bounds
/// relativi al widget finestra).
fn screen_bounds(widget: &gtk4::Widget, window: &Window, origin: (i32, i32)) -> Bounds {
    match widget.compute_bounds(window) {
        Some(rect) => Bounds::new(
            origin.0 as f64 + rect.x() as f64,
            origin.1 as f64 + rect.y() as f64,
            rect.width() as f64,
            rect.height() as f64,
        ),
        None => Bounds::default(),
    }
}

// ── Livello 3: primitive ────────────────────────────────────────────

/// Legge l'intero albero a partire dalla finestra radice.
pub fn dump_tree(root: NativeHandle) -> ComponentNode {
    let Some(window) = backend::frame_window_of(root) else {
        let mut node = ComponentNode::new(root.to_string(), "GtkWindow", "window");
        node.source = source::MISSING.to_string();
        return node;
    };
    let handles = handle_map();
    let origin = window_origin(&window);
    let root_widget = window.clone().upcast::<gtk4::Widget>();
    let mut walk = Walk {
        root,
        window: &window,
        origin,
        handles: &handles,
        budget: MAX_NODES,
    };
    walk.node(root_widget, 0, true)
}

/// Stato del walk (budget condiviso, come `inspector_windows.rs`).
struct Walk<'a> {
    root: NativeHandle,
    window: &'a Window,
    origin: (i32, i32),
    handles: &'a HashMap<usize, NativeHandle>,
    budget: usize,
}

impl Walk<'_> {
    fn node(&mut self, widget: gtk4::Widget, depth: usize, is_root: bool) -> ComponentNode {
        self.budget = self.budget.saturating_sub(1);

        let component_type = widget.type_().name().to_string();
        let handle = if is_root {
            Some(self.root)
        } else {
            self.handles.get(&widget_key(&widget)).copied()
        };
        let id = match handle {
            Some(handle) => handle.to_string(),
            None => format!("gtk:0x{:x}", widget_key(&widget)),
        };

        let mut node = ComponentNode::new(id, component_type, role_of(&widget, handle));
        node.text = text_of(&widget).filter(|t| !t.is_empty());
        node.bounds = screen_bounds(&widget, self.window, self.origin);
        node.visible = widget.is_visible();
        node.enabled = widget.is_sensitive();
        node.focused = widget.has_focus();
        node.state = state_of(&widget, handle);
        node.extra = extra_of(&widget, handle);
        node.source = if handle.is_some() {
            source::NATIVE.to_string()
        } else {
            source::INFERRED.to_string()
        };

        if self.budget == 0 {
            node.source = source::MISSING.to_string();
            return node;
        }
        if depth >= MAX_DEPTH {
            return node;
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            let next = current.next_sibling();
            self.budget = self.budget.saturating_sub(1);
            let mut child_node = self.node(current, depth + 1, false);
            if self.budget == 0 {
                child_node.source = source::MISSING.to_string();
                node.children.push(child_node);
                break;
            }
            node.children.push(child_node);
            child = next;
        }
        node
    }
}

/// Metadati della finestra radice (`GET /window`).
pub fn window_info(root: NativeHandle) -> WindowInfo {
    let Some(window) = backend::frame_window_of(root) else {
        return WindowInfo {
            source: source::MISSING.to_string(),
            ..Default::default()
        };
    };

    let scale = window.scale_factor().max(1) as f64;
    let dpi = (96.0 * scale).round() as u32;
    let surface = window.surface();
    let client_w = surface.as_ref().map(|s| s.width()).unwrap_or(0).max(0) as u32;
    let client_h = surface.as_ref().map(|s| s.height()).unwrap_or(0).max(0) as u32;

    let mut info = WindowInfo {
        id: root.to_string(),
        title: window.title().map(|t| t.to_string()).unwrap_or_default(),
        class: window.type_().name().to_string(),
        bounds: Bounds::default(),
        client_w,
        client_h,
        visible: window.is_visible(),
        scale,
        dpi,
        screen_w: 0,
        screen_h: 0,
        source: source::NATIVE.to_string(),
    };

    match (window_xid(&window), X11Conn::open()) {
        (Some(xid), Some(conn)) => {
            if let Some((x, y, w, h)) = conn.geometry(xid) {
                info.bounds = Bounds::new(x as f64, y as f64, w as f64, h as f64);
                if info.client_w == 0 {
                    info.client_w = w;
                }
                if info.client_h == 0 {
                    info.client_h = h;
                }
            }
            let (sw, sh) = conn.screen_size();
            info.screen_w = sw;
            info.screen_h = sh;
        }
        _ => {
            // Niente server X: almeno l'area della superficie GTK.
            info.bounds = Bounds::new(0.0, 0.0, client_w as f64, client_h as f64);
        }
    }
    info
}

/// PNG della finestra (selettore vuoto o id della radice) o del nodo.
pub fn node_screenshot(root: NativeHandle, sel: &Selector) -> Result<Vec<u8>, String> {
    let window = backend::frame_window_of(root).ok_or("finestra non disponibile")?;
    let conn = X11Conn::open().ok_or("nessun server X disponibile".to_string())?;
    let origin = window_origin(&window);

    let rect = match sel.id.as_deref() {
        // Selettore vuoto (o id della radice): l'intera finestra.
        None => {
            let xid = window_xid(&window).ok_or("finestra X11 non disponibile")?;
            conn.geometry(xid)
                .ok_or("geometria della finestra non disponibile")?
        }
        Some(id) => {
            let raw: isize = id.parse().map_err(|_| format!("id non valido: `{id}`"))?;
            if raw == root {
                let xid = window_xid(&window).ok_or("finestra X11 non disponibile")?;
                conn.geometry(xid)
                    .ok_or("geometria della finestra non disponibile")?
            } else {
                let widget = backend::widget_of(raw)
                    .ok_or_else(|| format!("nessuna finestra con id `{id}`"))?;
                let bounds = screen_bounds(&widget, &window, origin);
                if !bounds.is_valid() {
                    return Err(format!("geometria del nodo `{id}` non disponibile"));
                }
                (
                    bounds.x.floor() as i32,
                    bounds.y.floor() as i32,
                    bounds.w.ceil().max(1.0) as u32,
                    bounds.h.ceil().max(1.0) as u32,
                )
            }
        }
    };

    let (x, y, w, h) = rect;
    let image = conn.capture(x, y, w.max(1), h.max(1))?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    Ok(cursor.into_inner())
}

/// Inietta un'azione su un nodo identificato da `event.id`.
///
/// Convenzioni allineate a `inspector_windows.rs`: niente `id` → errore
/// coordinate; id non numerico → `id non valido`; handle sconosciuto →
/// `nessuna finestra con id`.
pub fn inject_event(_root: NativeHandle, event: &InjectedEvent) -> Result<(), String> {
    let id = event
        .id
        .as_deref()
        .ok_or_else(|| "evento senza `id`: le coordinate non sono ancora supportate".to_string())?;
    let raw: isize = id.parse().map_err(|_| format!("id non valido: `{id}`"))?;

    // L'id può essere una finestra top-level (close/focus della radice).
    let window = backend::frame_window_of(raw);
    let widget = match &window {
        Some(window) => window.clone().upcast::<gtk4::Widget>(),
        None => backend::widget_of(raw).ok_or_else(|| format!("nessuna finestra con id `{id}`"))?,
    };
    let component_type = widget.type_().name().to_string();

    match event.action.as_str() {
        "click" => {
            if let Some(button) = widget.downcast_ref::<gtk4::Button>() {
                button.emit_clicked();
            } else {
                widget.activate();
            }
            Ok(())
        }
        "focus" => {
            widget.grab_focus();
            Ok(())
        }
        "set_text" => {
            let text = event.text.as_deref().unwrap_or("");
            if backend::get_widget_text(raw).is_some() {
                backend::set_widget_text(raw, text);
                Ok(())
            } else {
                Err(format!("`set_text` non supportato per `{component_type}`"))
            }
        }
        // `GtkComboBoxText` è deprecato da GTK 4.10 ma `ComboBox::new` lo
        // usa ancora: è l'unico widget GTK4 che unisce lista a tendina e
        // campo di testo editabile (cfr. `gtk.rs::create_combo_box`).
        #[allow(deprecated)]
        "select" => {
            let index = event
                .key
                .as_deref()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| "`select` richiede `key` con un indice numerico".to_string())?;
            // Widget dichiarato nel registro (es. il guscio `tabs`).
            if declared::select(raw, index) {
                return Ok(());
            }
            if let Some(combo) = widget.downcast_ref::<gtk4::ComboBoxText>() {
                combo.set_active(Some(index as u32));
                return Ok(());
            }
            if let Some(drop) = widget.downcast_ref::<DropDown>() {
                drop.set_selected(index as u32);
                return Ok(());
            }
            if let Some(book) = widget.downcast_ref::<Notebook>() {
                if index as u32 >= book.n_pages() {
                    return Err(format!(
                        "pagina {index} fuori range (0..{})",
                        book.n_pages()
                    ));
                }
                book.set_current_page(Some(index as u32));
                return Ok(());
            }
            if let Some(list) = widget.downcast_ref::<ListBox>() {
                let Some(row) = list.row_at_index(index as i32) else {
                    return Err(format!("riga {index} fuori range"));
                };
                list.select_row(Some(&row));
                return Ok(());
            }
            // Tabella (`ListCtrl` = `GtkTreeView` + `ListStore`): stessa
            // semantica di `select_listview_row` su Windows — valida il
            // range, deseleziona tutto, seleziona la riga (§19.5).
            if component_type == "GtkTreeView" {
                let count = backend::listctrl_get_item_count(raw);
                if index >= count {
                    return Err(format!("riga {index} fuori range (0..{count})"));
                }
                backend::listctrl_clear_selection(raw);
                backend::listctrl_select(raw, index);
                return Ok(());
            }
            Err(format!("`select` non supportato per `{component_type}`"))
        }
        "close" => {
            // Finestra radice, finestra di dialogo o la top-level del widget.
            let target = window.or_else(|| widget.root()?.downcast::<Window>().ok());
            match target {
                Some(window) => {
                    window.close();
                    Ok(())
                }
                None => Err(format!("nessuna finestra con id `{id}`")),
            }
        }
        other => Err(format!("azione non supportata: `{other}`")),
    }
}

// ── Lettura delle proprietà dei widget ──────────────────────────────

/// Ruolo logico normalizzato: il registro `declared` ha la precedenza,
/// poi il mapping dal tipo GTK (§7.1 del piano).
fn role_of(widget: &gtk4::Widget, handle: Option<NativeHandle>) -> String {
    if let Some(handle) = handle {
        if let Some(role) = declared::role(handle) {
            return role.to_string();
        }
    }
    let type_name = widget.type_().name();
    match type_name {
        "GtkWindow" => "window",
        "GtkDialog" | "GtkAboutDialog" | "GtkAssistant" => "dialog",
        "GtkButton" | "GtkLinkButton" | "GtkMenuButton" | "GtkToggleButton" | "GtkColorButton"
        | "GtkFontButton" => "button",
        "GtkCheckButton" | "GtkSwitch" => "check",
        "GtkLabel" | "GtkAccelLabel" | "GtkShortcutLabel" => "label",
        "GtkEntry" | "GtkPasswordEntry" | "GtkTextView" | "GtkSpinButton" => "input",
        "GtkComboBox" | "GtkComboBoxText" | "GtkComboBoxWithEntry" | "GtkDropDown" => "choice",
        "GtkListBox" | "GtkFlowBox" | "GtkListView" | "GtkGridView" => "list",
        "GtkColumnView" | "GtkTableView" | "GtkTreeListView" => "table",
        "GtkTreeView" => "tree",
        "GtkNotebook" | "GtkStackSwitcher" => "tabs",
        "GtkScrolledWindow" | "GtkScrollbar" => "scroll",
        "GtkProgressBar" | "GtkScale" | "GtkLevelBar" | "GtkVolumeButton" => "value",
        "GtkPicture" | "GtkImage" => "image",
        "GtkFrame" | "GtkExpander" | "GtkListBoxRow" => "group",
        "GtkBox" | "GtkGrid" | "GtkFixed" | "GtkPaned" | "GtkOverlay" | "GtkStack"
        | "GtkHeaderBar" | "GtkActionBar" | "GtkCenterBox" | "GtkToolbar" => "container",
        t if t.contains("WebKit") => "webview",
        _ => "view",
    }
    .to_string()
}

/// Testo visibile del widget (label, campo di testo, pulsante).
fn text_of(widget: &gtk4::Widget) -> Option<String> {
    if let Some(label) = widget.downcast_ref::<Label>() {
        return Some(label.text().to_string());
    }
    if let Some(entry) = widget.downcast_ref::<gtk4::Entry>() {
        return Some(entry.text().to_string());
    }
    if let Some(check) = widget.downcast_ref::<CheckButton>() {
        if let Some(label) = check.label() {
            return Some(label.to_string());
        }
    }
    if let Some(button) = widget.downcast_ref::<gtk4::Button>() {
        if let Some(label) = button.label() {
            return Some(label.to_string());
        }
    }
    if let Some(view) = widget.downcast_ref::<TextView>() {
        let buffer = view.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        return Some(text.to_string());
    }
    None
}

/// Stato specifico del widget (`checked`, `selected`, …).
fn state_of(widget: &gtk4::Widget, handle: Option<NativeHandle>) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    // Selezione dei widget senza classe nativa propria (gusci `tabs`):
    // è il registro `declared` a detenerla (chiavi come su Windows).
    // Alla tabella il registro serve solo per il ruolo: la selezione è
    // `selected_rows` (sotto), mentre `selection` resterebbe il 0
    // implicito di `mark`.
    if let Some(handle) = handle {
        let role = declared::role(handle);
        if role != Some("table") {
            if let Some(selection) = declared::selection(handle) {
                let key = if role == Some("radio") {
                    "checked_index"
                } else {
                    "selection"
                };
                map.insert(key.into(), serde_json::json!(selection));
            }
        }
    }

    if let Some(check) = widget.downcast_ref::<CheckButton>() {
        map.insert("checked".into(), serde_json::json!(check.is_active()));
    } else if let Some(switch) = widget.downcast_ref::<Switch>() {
        map.insert("checked".into(), serde_json::json!(switch.is_active()));
    }

    // Vedi la nota sul braccio `select`: il combo può essere un
    // `GtkComboBoxText` deprecato da GTK 4.10.
    #[allow(deprecated)]
    if let Some(combo) = widget.downcast_ref::<gtk4::ComboBoxText>() {
        if let Some(active) = combo.active() {
            map.insert("selected".into(), serde_json::json!(active));
        }
    } else if let Some(drop) = widget.downcast_ref::<DropDown>() {
        map.insert("selected".into(), serde_json::json!(drop.selected()));
    } else if let Some(book) = widget.downcast_ref::<Notebook>() {
        if let Some(page) = book.current_page() {
            map.insert("selected".into(), serde_json::json!(page));
        }
    } else if let Some(list) = widget.downcast_ref::<ListBox>() {
        if let Some(row) = list.selected_row() {
            map.insert("selected".into(), serde_json::json!([row.index().max(0)]));
        }
    }

    // Tabella (`ListCtrl` = `GtkTreeView`): righe selezionate con la
    // stessa chiave `selected_rows` di `read_listview`/`read_table`
    // (§6.4, §19.6).
    if widget.type_().name() == "GtkTreeView" {
        if let Some(handle) = handle {
            map.insert(
                "selected_rows".into(),
                serde_json::json!(backend::listctrl_get_selected_items(handle)),
            );
        }
    }

    if map.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(map)
    }
}

/// Contenuti complessi (tabella, conteggi di pagine/righe).
fn extra_of(widget: &gtk4::Widget, handle: Option<NativeHandle>) -> serde_json::Value {
    if widget.type_().name() == "GtkTreeView" {
        if let Some(handle) = handle {
            return table_extra(handle);
        }
    }
    if let Some(book) = widget.downcast_ref::<Notebook>() {
        return serde_json::json!({
            "count": book.n_pages(),
            "selected": book.current_page(),
        });
    }
    if let Some(list) = widget.downcast_ref::<ListBox>() {
        let mut count = 0u32;
        let mut row = list.first_child();
        while let Some(current) = row {
            count += 1;
            row = current.next_sibling();
        }
        return serde_json::json!({ "count": count });
    }
    serde_json::Value::Null
}

/// Contenuti della tabella (`ListCtrl`): stesse chiavi di `read_listview`
/// su Windows — `columns`, `rows`, `row_count`, `selected_count`,
/// `selected_rows`, `header_text`, `truncated` (§6.4).
fn table_extra(handle: NativeHandle) -> serde_json::Value {
    let row_count = backend::listctrl_get_item_count(handle);
    let columns: Vec<serde_json::Value> = backend::listctrl_column_info(handle)
        .into_iter()
        .take(MAX_TABLE_COLS)
        .map(|(title, width)| serde_json::json!({ "title": title, "width": width }))
        .collect();
    let n_cols = columns.len();

    let mut rows = Vec::new();
    for r in 0..row_count.min(MAX_TABLE_ROWS) {
        let cells: Vec<serde_json::Value> = (0..n_cols)
            .map(|c| serde_json::Value::String(backend::listctrl_get_item_text(handle, r, c)))
            .collect();
        rows.push(serde_json::Value::Array(cells));
    }

    let selected = backend::listctrl_get_selected_items(handle);
    let header_text = columns
        .iter()
        .filter_map(|c| c.get("title").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join(" | ");

    serde_json::json!({
        "columns": columns,
        "rows": rows,
        "row_count": row_count,
        "selected_count": selected.len(),
        "selected_rows": selected,
        "header_text": header_text,
        "truncated": row_count > MAX_TABLE_ROWS,
    })
}
