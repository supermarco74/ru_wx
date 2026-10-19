//! Windows — implementazione Livello 3 delle primitive dell'inspector
//! (cfr. `PIANO_DEBUG_INSPECTOR.md` §10).
//!
//! Backend `win32_aui` / `winui3`: i widget sono **HWND** reali, quindi
//! l'albero si legge con `GetWindow(GW_CHILD/GW_HWNDNEXT)` e le API
//! Win32 classiche. Il backend `winui3_xaml` / `winui3_reactor` (element
//! tree `windows-reactor`, senza HWND) è gestito in
//! `crate::platform::winui3_xaml::element_tree`: `dump_tree` prova prima
//! il registro XAML e ricade su questo percorso Win32.
//!
//! Va chiamato **sul thread UI** (è il caso garantito dal marshaling §5).

use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetBkColor, GetDC,
    GetDIBits, GetTextColor, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::UI::Controls::{LVCOLUMNW, LVITEMW};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetSystemMetrics, GetWindow, GetWindowLongW, IsWindowVisible,
    PostMessageW, SendMessageW, SetWindowTextW, GWL_STYLE, GW_CHILD, GW_HWNDNEXT, GW_OWNER,
    PW_RENDERFULLCONTENT, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, WM_CLOSE,
};
// Serve solo al fallback screenshot del backend XAML (§10.1).
#[cfg(feature = "winui3_xaml")]
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
#[cfg(feature = "winui3_xaml")]
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

use crate::platform::inspector::{
    source, Bounds, ColorInfo, ComponentNode, InjectedEvent, Selector, WindowInfo,
};
use crate::platform::{as_hwnd, declared, win32, NativeHandle};

const MAX_DEPTH: usize = 24;
const MAX_NODES: usize = 4000;
/// Righe/colonne massime lette da una ListView (limita il payload JSON).
const MAX_TABLE_ROWS: usize = 200;
const MAX_TABLE_COLS: usize = 32;

// ── Costanti di stile / messaggi non esposti comodamente ────────────

const BS_TYPEMASK: i32 = 0x000F;
const BS_CHECKBOX: i32 = 0x0002;
const BS_AUTOCHECKBOX: i32 = 0x0003;
const BS_RADIOBUTTON: i32 = 0x0004;
const BS_3STATE: i32 = 0x0005;
const BS_AUTO3STATE: i32 = 0x0006;
const BS_GROUPBOX: i32 = 0x0007;
const BS_AUTORADIOBUTTON: i32 = 0x0009;
/// Stili statici che mostrano un'immagine (omologo di `NSImageView`).
const SS_TYPEMASK: i32 = 0x001F;
const SS_ICON: i32 = 0x0003;
const SS_BITMAP: i32 = 0x000E;
const SS_ENHMETAFILE: i32 = 0x000F;

const BM_GETCHECK: u32 = 0x00F0;
const BM_CLICK: u32 = 0x00F5;
const LB_GETCOUNT: u32 = 0x018B;
const LB_GETTEXT: u32 = 0x0189;
const LB_GETTEXTLEN: u32 = 0x018A;
const LB_GETCURSEL: u32 = 0x0188;
const LB_GETSELCOUNT: u32 = 0x0190;
const LB_SETCURSEL: u32 = 0x0186;
const CB_GETCOUNT: u32 = 0x0146;
const CB_GETLBTEXT: u32 = 0x0148;
const CB_GETLBTEXTLEN: u32 = 0x0149;
const CB_GETCURSEL: u32 = 0x0147;
const CB_SETCURSEL: u32 = 0x014E;

const LVM_FIRST: u32 = 0x1000;
const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
const LVM_GETITEMTEXTW: u32 = LVM_FIRST + 115;
const LVM_GETSELECTEDCOUNT: u32 = LVM_FIRST + 50;
const LVM_GETCOLUMNW: u32 = LVM_FIRST + 95;
const LVM_SETITEMSTATE: u32 = LVM_FIRST + 43;
const LVM_GETITEMSTATE: u32 = LVM_FIRST + 44;
const LVCF_WIDTH: u32 = 0x0002;
const LVCF_TEXT: u32 = 0x0004;
const LVIF_TEXT: u32 = 0x0001;
const LVIS_FOCUSED: usize = 0x0001;
const LVIS_SELECTED: usize = 0x0002;

fn rect_to_bounds(r: crate::core::geometry::Rect) -> Bounds {
    Bounds::new(r.x as f64, r.y as f64, r.width as f64, r.height as f64)
}

/// Legge l'intero albero a partire dalla finestra radice.
pub fn dump_tree(root: NativeHandle) -> ComponentNode {
    // Backend XAML/reactor: i widget non sono HWND, il "handle" è il
    // `frame_id` del frame registrato in `winui3_xaml::element_tree` (§10.1).
    #[cfg(feature = "winui3_xaml")]
    {
        let frame_id = root as u64;
        if super::winui3_xaml::element_tree::is_registered(frame_id) {
            if let Some(node) = super::winui3_xaml::element_tree::dump_tree(frame_id) {
                return node;
            }
        }
    }

    let hwnd = as_hwnd(root);
    if hwnd.is_null() {
        let mut n = ComponentNode::new(root.to_string(), "HWND", "window");
        n.source = source::MISSING.to_string();
        return n;
    }
    let mut budget = MAX_NODES;
    let owned = owned_windows_map();
    node_for_hwnd(hwnd, root, 0, &mut budget, true, &owned)
}

/// Mappa `proprietario → finestre possedute` (cfr. C8, finestra tip).
///
/// Le finestre popup possedute (`GW_OWNER`, es. finestre di dialogo e la
/// finestra tip) **non** sono figlie nel walk `GW_CHILD`: senza questo
/// aggancio non comparirebbero in `/tree` né in `/find?role=dialog`.
fn owned_windows_map() -> std::collections::HashMap<HWND, Vec<HWND>> {
    let mut map: std::collections::HashMap<HWND, Vec<HWND>> = std::collections::HashMap::new();

    // SAFETY: `lparam` è il `*mut HashMap` passato sotto, vivo e non
    // aliasato per tutta l'enumerazione.
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let map = unsafe { &mut *(lparam as *mut std::collections::HashMap<HWND, Vec<HWND>>) };
        // SAFETY: `GetWindow(GW_OWNER)` non legge buffer e ritorna l'owner.
        let owner = unsafe { GetWindow(hwnd, GW_OWNER) };
        if !owner.is_null() {
            map.entry(owner).or_default().push(hwnd);
        }
        1 // continua l'enumerazione
    }

    // SAFETY: `enum_proc` rispetta il contratto di `WNDENUMPROC`.
    unsafe { EnumWindows(Some(enum_proc), &mut map as *mut _ as LPARAM) };
    map
}

/// Metadati della finestra radice (`GET /window`).
///
/// Backend XAML/reactor: id/title/class vengono dal registro
/// `winui3_xaml::element_tree`, mentre `bounds`, `client_*`, `visible`,
/// `scale` e `dpi` sono letti dalla finestra top-level Win32 che ospita
/// la UI XAML: il reactor non espone rect per-elemento senza un layout
/// pass (§6.3, §10.1), quindi solo la radice ha geometria reale.
pub fn window_info(root: NativeHandle) -> WindowInfo {
    #[cfg(feature = "winui3_xaml")]
    {
        let frame_id = root as u64;
        if super::winui3_xaml::element_tree::is_registered(frame_id) {
            if let Some(mut info) = super::winui3_xaml::element_tree::window_info(frame_id) {
                if let Some(hwnd) = find_own_top_level_window() {
                    let rect = win32::window_rect(hwnd);
                    let (client_w, client_h) = win32::client_size(hwnd);
                    // DPI per-finestra, come nel percorso Win32 (§10.2, §12).
                    let dpi = crate::core::dpi::get_dpi_for_window(hwnd).value();
                    info.bounds = rect_to_bounds(rect);
                    info.client_w = client_w;
                    info.client_h = client_h;
                    info.visible = is_visible(hwnd);
                    info.scale = dpi as f64 / 96.0;
                    info.dpi = dpi;
                }
                // Spazio virtuale unificato (multi-monitor), non il solo primario.
                info.screen_w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1) as u32 };
                info.screen_h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1) as u32 };
                return info;
            }
        }
    }

    let hwnd = as_hwnd(root);
    if hwnd.is_null() {
        return WindowInfo {
            source: source::MISSING.to_string(),
            ..Default::default()
        };
    }
    let rect = win32::window_rect(hwnd);
    let (client_w, client_h) = win32::client_size(hwnd);
    // DPI per-finestra (`GetDpiForWindow`): corretto anche su monitor
    // multipli con scaling diverso (§10.2, §12).
    let dpi = crate::core::dpi::get_dpi_for_window(hwnd).value();
    let scale = dpi as f64 / 96.0;
    WindowInfo {
        id: root.to_string(),
        title: win32::read_window_text(hwnd),
        class: class_name(hwnd),
        bounds: rect_to_bounds(rect),
        client_w,
        client_h,
        visible: is_visible(hwnd),
        scale,
        dpi,
        // Spazio virtuale unificato (multi-monitor), non il solo primario.
        screen_w: unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1) as u32 },
        screen_h: unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1) as u32 },
        source: source::NATIVE.to_string(),
    }
}

// ── Screenshot (cfr. §10.2, Fase B5) ────────────────────────────────

/// PNG della finestra (selettore vuoto) o del controllo indicato da `sel.id`.
///
/// Usa `PrintWindow` con `PW_RENDERFULLCONTENT` per catturare anche i
/// contenuti composited, poi converte il DIB BGRA in PNG (crate `image`).
///
/// Backend XAML/reactor: i widget non hanno un HWND e gli id del registro
/// hanno forma `xaml:*`; in quel caso si cattura la finestra top-level
/// WinUI3 del processo, che ospita l'intera UI XAML. La cattura per-nodo
/// resta indisponibile: senza layout pass non esiste un rect per elemento
/// (§6.3, §10.1).
pub fn node_screenshot(root: NativeHandle, sel: &Selector) -> Result<Vec<u8>, String> {
    #[cfg(feature = "winui3_xaml")]
    {
        let is_xaml = super::winui3_xaml::element_tree::is_registered(root as u64)
            || sel.id.as_deref().is_some_and(|id| id.starts_with("xaml:"));
        if is_xaml {
            let hwnd = find_own_top_level_window()
                .ok_or_else(|| "finestra XAML non trovata".to_string())?;
            return screenshot_hwnd(hwnd);
        }
    }

    let hwnd = match sel.id.as_deref() {
        Some(id) => {
            let raw: isize = id.parse().map_err(|_| format!("id non valido: `{id}`"))?;
            as_hwnd(raw)
        }
        None => as_hwnd(root),
    };
    if hwnd.is_null() {
        return Err("finestra non disponibile".to_string());
    }
    screenshot_hwnd(hwnd)
}

/// Cattura `hwnd` con `PrintWindow` e la codifica in PNG.
///
/// `PrintWindow` produce BGRA: i canali vengono riordinati in RGBA e
/// l'alpha forzata opaca.
fn screenshot_hwnd(hwnd: HWND) -> Result<Vec<u8>, String> {
    let rect = win32::window_rect(hwnd);
    let width = (rect.width as i32).max(1);
    let height = (rect.height as i32).max(1);

    // SAFETY: le risorse GDI sono rilasciate in ogni percorso di
    // `capture_bgra`; l'handler interno non tocca la GUI.
    let mut rgba = unsafe { capture_bgra(hwnd, width, height) }
        .ok_or_else(|| "cattura della finestra fallita".to_string())?;

    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
        px[3] = 255;
    }

    let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| "buffer immagine non valido".to_string())?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    Ok(cursor.into_inner())
}

/// Trova l'HWND della finestra top-level di questo processo.
///
/// Serve al backend XAML/reactor, i cui widget non hanno HWND: si individua
/// la finestra WinUI3 che li ospita. Viene preferita la prima finestra
/// visibile con titolo non vuoto; in mancanza, la prima visibile.
#[cfg(feature = "winui3_xaml")]
fn find_own_top_level_window() -> Option<HWND> {
    struct Ctx {
        pid: u32,
        found: HWND,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` è il `*mut Ctx` passato a `EnumWindows`, vivo per
        // tutta l'enumerazione e non aliasato altrove.
        let ctx = unsafe { &mut *(lparam as *mut Ctx) };
        let mut win_pid: u32 = 0;
        // SAFETY: `win_pid` è un out-param valido per questa chiamata.
        unsafe { GetWindowThreadProcessId(hwnd, &mut win_pid) };
        if win_pid != ctx.pid || !is_visible(hwnd) {
            return 1; // continua l'enumerazione
        }
        if ctx.found.is_null() {
            ctx.found = hwnd; // fallback: prima finestra visibile
        }
        if !win32::read_window_text(hwnd).is_empty() {
            ctx.found = hwnd;
            return 0; // finestra "principale" trovata: stop
        }
        1
    }

    let mut ctx = Ctx {
        // SAFETY: `GetCurrentProcessId` non ha precondizioni.
        pid: unsafe { GetCurrentProcessId() },
        found: std::ptr::null_mut(),
    };
    // SAFETY: `enum_proc` rispetta il contratto di `WNDENUMPROC`.
    unsafe { EnumWindows(Some(enum_proc), &mut ctx as *mut Ctx as LPARAM) };
    if ctx.found.is_null() {
        None
    } else {
        Some(ctx.found)
    }
}

/// Cattura l'area della finestra in un buffer BGRA (righe dall'alto).
///
/// Ritorna `None` se una risorsa GDI non è disponibile o `PrintWindow`
/// fallisce; in tal caso tutte le risorse create vengono rilasciate.
unsafe fn capture_bgra(hwnd: HWND, width: i32, height: i32) -> Option<Vec<u8>> {
    let hdc_window = GetDC(hwnd);
    if hdc_window.is_null() {
        return None;
    }
    let hdc_mem = CreateCompatibleDC(hdc_window);
    let hbm = if hdc_mem.is_null() {
        std::ptr::null_mut()
    } else {
        CreateCompatibleBitmap(hdc_window, width, height)
    };
    if hdc_mem.is_null() || hbm.is_null() {
        if !hbm.is_null() {
            DeleteObject(hbm);
        }
        if !hdc_mem.is_null() {
            DeleteDC(hdc_mem);
        }
        ReleaseDC(hwnd, hdc_window);
        return None;
    }
    let old_bitmap = SelectObject(hdc_mem, hbm);

    // `PrintWindow` disegna la finestra (anche composited) nel DC di memoria.
    let printed = PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT);

    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height, // negativo = DIB top-down
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let lines = GetDIBits(
        hdc_mem,
        hbm,
        0,
        height as u32,
        buffer.as_mut_ptr() as *mut core::ffi::c_void,
        &mut info,
        DIB_RGB_COLORS,
    );

    SelectObject(hdc_mem, old_bitmap);
    DeleteObject(hbm);
    DeleteDC(hdc_mem);
    ReleaseDC(hwnd, hdc_window);

    if printed == 0 || lines == 0 {
        return None;
    }
    Some(buffer)
}

// ── Walk dell'albero ────────────────────────────────────────────────

fn node_for_hwnd(
    hwnd: HWND,
    handle: NativeHandle,
    depth: usize,
    budget: &mut usize,
    is_root: bool,
    owned: &std::collections::HashMap<HWND, Vec<HWND>>,
) -> ComponentNode {
    *budget = budget.saturating_sub(1);

    let class = class_name(hwnd);
    let role = role_for_class(&class, hwnd);
    let id = if is_root {
        handle.to_string()
    } else {
        (hwnd as isize).to_string()
    };

    let mut node = ComponentNode::new(id, class, role);
    node.text = Some(win32::read_window_text(hwnd)).filter(|t| !t.is_empty());
    node.bounds = rect_to_bounds(win32::window_rect(hwnd));
    node.visible = is_visible(hwnd);
    node.enabled = is_enabled(hwnd);
    node.focused = unsafe { GetFocus() == hwnd };
    node.colors = read_colors(hwnd);
    node.state = read_state(hwnd, &node.component_type);
    node.extra = read_extra(hwnd, &node.component_type);
    node.source = source::NATIVE.to_string();

    if *budget == 0 {
        node.source = source::MISSING.to_string();
        return node;
    }
    if depth < MAX_DEPTH {
        let mut child = unsafe { GetWindow(hwnd, GW_CHILD) };
        while !child.is_null() {
            node.children.push(node_for_hwnd(
                child,
                child as isize,
                depth + 1,
                budget,
                false,
                owned,
            ));
            if *budget == 0 {
                break;
            }
            child = unsafe { GetWindow(child, GW_HWNDNEXT) };
        }
        // Finestre possedute (popup/modali, cfr. C8): figlie "di_OWNER",
        // invisibili a `GW_CHILD` ma necessarie a `/find?role=dialog`.
        if *budget > 0 {
            if let Some(list) = owned.get(&hwnd) {
                for &popup in list {
                    node.children.push(node_for_hwnd(
                        popup,
                        popup as isize,
                        depth + 1,
                        budget,
                        false,
                        owned,
                    ));
                    if *budget == 0 {
                        break;
                    }
                }
            }
        }
    }
    node
}

// ── Reader per-widget (cfr. §6.4) ───────────────────────────────────

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    // SAFETY: buffer dimensionato e terminato da GetClassNameW.
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn is_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd) != 0 }
}

fn is_enabled(hwnd: HWND) -> bool {
    unsafe { IsWindowEnabled(hwnd) != 0 }
}

fn style(hwnd: HWND) -> i32 {
    unsafe { GetWindowLongW(hwnd, GWL_STYLE) }
}

fn read_colors(hwnd: HWND) -> ColorInfo {
    let mut info = ColorInfo::default();
    // SAFETY: GetDC/ReleaseDC bilanciati; hdc nullo gestito.
    unsafe {
        let hdc = GetDC(hwnd);
        if !hdc.is_null() {
            info.text = colorref_to_hex(GetTextColor(hdc));
            info.background = colorref_to_hex(GetBkColor(hdc));
            ReleaseDC(hwnd, hdc);
        }
    }
    info
}

/// `COLORREF` (0x00BBGGRR) → `#RRGGBBAA`.
fn colorref_to_hex(c: u32) -> Option<String> {
    let r = c & 0xFF;
    let g = (c >> 8) & 0xFF;
    let b = (c >> 16) & 0xFF;
    Some(format!("#{r:02X}{g:02X}{b:02X}FF"))
}

fn read_state(hwnd: HWND, class: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    // Selezione dei widget senza classe nativa propria (es. il guscio di
    // `Simplebook`, ruolo `tabs`): è il registro `declared` a detenerla.
    if let Some(selection) = declared::selection(hwnd as NativeHandle) {
        // Il guscio di `RadioBox` è marcato `radio` (§6 del piano di
        // portabilità) e la stessa selezione va letta come `checked_index`;
        // per tutti gli altri (book, simplebook) resta `selection`.
        let key = if declared::role(hwnd as NativeHandle) == Some("radio") {
            "checked_index"
        } else {
            "selection"
        };
        map.insert(key.into(), serde_json::json!(selection));
    }
    if class == "Button" {
        let style = style(hwnd) & BS_TYPEMASK;
        let is_check = matches!(
            style,
            BS_CHECKBOX
                | BS_AUTOCHECKBOX
                | BS_RADIOBUTTON
                | BS_AUTORADIOBUTTON
                | BS_3STATE
                | BS_AUTO3STATE
        );
        if is_check {
            let state = unsafe { SendMessageW(hwnd, BM_GETCHECK, 0, 0) };
            map.insert("checked".into(), serde_json::Value::Bool(state != 0));
        }
    }
    if map.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(map)
    }
}

fn read_extra(hwnd: HWND, class: &str) -> serde_json::Value {
    match class {
        "ListBox" => read_listbox(hwnd),
        "ComboBox" => read_combobox(hwnd),
        "SysListView32" => read_listview(hwnd),
        _ => serde_json::Value::Null,
    }
}

fn send_text(hwnd: HWND, msg: u32, index: usize, len_msg: u32) -> Option<String> {
    let len = unsafe { SendMessageW(hwnd, len_msg, index, 0) };
    if len <= 0 {
        return Some(String::new());
    }
    let mut buf = vec![0u16; len as usize + 1];
    let copied = unsafe { SendMessageW(hwnd, msg, index, buf.as_mut_ptr() as isize) };
    if copied < 0 {
        return None;
    }
    let n = (copied as usize).min(buf.len() - 1);
    Some(String::from_utf16_lossy(&buf[..n]))
}

fn read_listbox(hwnd: HWND) -> serde_json::Value {
    let count = unsafe { SendMessageW(hwnd, LB_GETCOUNT, 0, 0) }.max(0) as usize;
    let mut items = Vec::with_capacity(count.min(MAX_TABLE_ROWS));
    for i in 0..count.min(MAX_TABLE_ROWS) {
        items.push(serde_json::Value::String(
            send_text(hwnd, LB_GETTEXT, i, LB_GETTEXTLEN).unwrap_or_default(),
        ));
    }
    let cur = unsafe { SendMessageW(hwnd, LB_GETCURSEL, 0, 0) };
    let multi = unsafe { SendMessageW(hwnd, LB_GETSELCOUNT, 0, 0) } > 0;
    let mut map = serde_json::Map::new();
    map.insert("items".into(), serde_json::Value::Array(items));
    map.insert("count".into(), serde_json::json!(count));
    map.insert("multi".into(), serde_json::Value::Bool(multi));
    map.insert(
        "selected".into(),
        if cur >= 0 {
            serde_json::json!([cur])
        } else {
            serde_json::json!([])
        },
    );
    serde_json::Value::Object(map)
}

fn read_combobox(hwnd: HWND) -> serde_json::Value {
    let count = unsafe { SendMessageW(hwnd, CB_GETCOUNT, 0, 0) }.max(0) as usize;
    let mut items = Vec::with_capacity(count.min(MAX_TABLE_ROWS));
    for i in 0..count.min(MAX_TABLE_ROWS) {
        items.push(serde_json::Value::String(
            send_text(hwnd, CB_GETLBTEXT, i, CB_GETLBTEXTLEN).unwrap_or_default(),
        ));
    }
    let cur = unsafe { SendMessageW(hwnd, CB_GETCURSEL, 0, 0) };
    let mut map = serde_json::Map::new();
    map.insert("items".into(), serde_json::Value::Array(items));
    map.insert("count".into(), serde_json::json!(count));
    map.insert(
        "selected".into(),
        if cur >= 0 {
            serde_json::json!([cur])
        } else {
            serde_json::json!([])
        },
    );
    serde_json::Value::Object(map)
}

fn read_listview(hwnd: HWND) -> serde_json::Value {
    let row_count = unsafe { SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0) }.max(0) as usize;
    let selected = unsafe { SendMessageW(hwnd, LVM_GETSELECTEDCOUNT, 0, 0) }.max(0) as usize;

    // Quali righe sono selezionate: `LVM_GETITEMSTATE` per riga (§6.4).
    let mut selected_rows = Vec::new();
    for r in 0..row_count.min(MAX_TABLE_ROWS) {
        let state =
            unsafe { SendMessageW(hwnd, LVM_GETITEMSTATE, r, LVIS_SELECTED as isize) } as usize;
        if state & LVIS_SELECTED != 0 {
            selected_rows.push(r);
        }
    }

    // Colonne: titolo + larghezza via LVM_GETCOLUMNW.
    let mut columns = Vec::new();
    for col in 0..MAX_TABLE_COLS {
        let mut title_buf = [0u16; 128];
        let mut item: LVCOLUMNW = unsafe { std::mem::zeroed() };
        item.mask = LVCF_TEXT | LVCF_WIDTH;
        item.pszText = title_buf.as_mut_ptr();
        item.cchTextMax = title_buf.len() as i32;
        let res = unsafe { SendMessageW(hwnd, LVM_GETCOLUMNW, col, &mut item as *mut _ as isize) };
        if res == 0 {
            break;
        }
        let title = String::from_utf16_lossy(&title_buf)
            .trim_end_matches('\0')
            .to_string();
        columns.push(serde_json::json!({ "title": title, "width": item.cx }));
    }

    // Righe: solo le prime `MAX_TABLE_ROWS`, tutte le colonne.
    let n_cols = columns.len().min(MAX_TABLE_COLS);
    let mut rows = Vec::new();
    for r in 0..row_count.min(MAX_TABLE_ROWS) {
        let mut cells = Vec::with_capacity(n_cols);
        for c in 0..n_cols {
            let mut buf = [0u16; 256];
            let mut item: LVITEMW = unsafe { std::mem::zeroed() };
            item.mask = LVIF_TEXT;
            item.iItem = r as i32;
            item.iSubItem = c as i32;
            item.pszText = buf.as_mut_ptr();
            item.cchTextMax = buf.len() as i32;
            let _ =
                unsafe { SendMessageW(hwnd, LVM_GETITEMTEXTW, r, &mut item as *mut _ as isize) };
            cells.push(serde_json::Value::String(
                String::from_utf16_lossy(&buf)
                    .trim_end_matches('\0')
                    .to_string(),
            ));
        }
        rows.push(serde_json::Value::Array(cells));
    }

    let header_text = columns
        .iter()
        .filter_map(|c| c.get("title").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join(" | ");

    serde_json::json!({
        "columns": columns,
        "rows": rows,
        "row_count": row_count,
        "selected_count": selected,
        "selected_rows": selected_rows,
        "header_text": header_text,
        "truncated": row_count > MAX_TABLE_ROWS,
    })
}

/// Normalizza la classe nativa in un ruolo logico condiviso (§6.1).
fn role_for_class(class: &str, hwnd: HWND) -> String {
    // Ruolo dichiarato (registro `declared`): ha la precedenza
    // sull'euristica di classe — il guscio di `Simplebook` è un
    // `RuWxPanel` che altrimenti cade in "view" (stesso ordine di
    // `inspector_macos.rs`, §17.1 di `PIANO_DEBUG_INSPECTOR.md`).
    if let Some(role) = declared::role(hwnd as NativeHandle) {
        return role.to_string();
    }
    match class {
        "Button" => {
            let style = style(hwnd) & BS_TYPEMASK;
            match style {
                BS_CHECKBOX | BS_AUTOCHECKBOX | BS_3STATE | BS_AUTO3STATE => "check",
                BS_RADIOBUTTON | BS_AUTORADIOBUTTON => "radio",
                BS_GROUPBOX => "group",
                _ => "button",
            }
        }
        "Static" => {
            // SS_ICON/SS_BITMAP/SS_ENHMETAFILE = immagine (come NSImageView
            // su AppKit): senza il check tutti i Static finivano in "label"
            // e /find?role=image non trovava le icone Win32.
            let st = style(hwnd) & SS_TYPEMASK;
            match st {
                SS_ICON | SS_BITMAP | SS_ENHMETAFILE => "image",
                _ => "label",
            }
        }
        "Edit" | "RichEdit20W" => "input",
        "ComboBox" | "ComboBoxEx32" => "choice",
        "ListBox" => "list",
        "SysListView32" => "table",
        "SysTreeView32" => "tree",
        "SysTabControl32" => "tabs",
        "msctls_trackbar32" | "msctls_updown32" | "msctls_progress32" => "value",
        // Allineato ai ruoli XAML: DatePicker/TimePicker → "picker".
        // NOTA: la classe registrata è `SysDateTimePick32`
        // (DATETIMEPICK_CLASS in commctrl.h), non `msctls_datetimepick32`.
        "SysDateTimePick32" => "picker",
        "ScrollBar" => "scroll",
        c if c.starts_with("RichEdit") => "input",
        c if c.contains("WebView") => "webview",
        _ => "view",
    }
    .to_string()
}

// ── Azioni sui controlli (cfr. §11, Fase B4) ────────────────────────
//
// Preferiamo `SendMessageW` diretto al controllo (non `SendInput`): così
// il cursore reale non si muove e l'azione è deterministica.

/// Esegue un'azione su un controllo identificato da `event.id`.
///
/// Backend XAML/reactor: gli id hanno forma `xaml:*` e l'azione viene
/// eseguita invocando l'handler `pub` del widget corrispondente nel
/// registro (§11): nessun messaggio grezzo viene inviato all'OS.
pub fn inject_event(_root: NativeHandle, event: &InjectedEvent) -> Result<(), String> {
    #[cfg(feature = "winui3_xaml")]
    {
        let frame_id = _root as u64;
        if super::winui3_xaml::element_tree::is_registered(frame_id) {
            return super::winui3_xaml::element_tree::inject_event(frame_id, event);
        }
        if event
            .id
            .as_deref()
            .is_some_and(|id| id.starts_with("xaml:"))
        {
            return Err(format!("frame XAML {frame_id} non registrato"));
        }
    }

    let id = event
        .id
        .as_deref()
        .ok_or_else(|| "evento senza `id`: le coordinate non sono ancora supportate".to_string())?;
    let raw: isize = id.parse().map_err(|_| format!("id non valido: `{id}`"))?;
    let hwnd = as_hwnd(raw);
    if hwnd.is_null() {
        return Err(format!("nessuna finestra con id `{id}`"));
    }
    let class = class_name(hwnd);

    match event.action.as_str() {
        "click" => {
            // SAFETY: `BM_CLICK` è la simulazione di click dei pulsanti.
            unsafe { SendMessageW(hwnd, BM_CLICK, 0, 0) };
            Ok(())
        }
        "focus" => {
            // SAFETY: `SetFocus` su una finestra viva del thread UI.
            unsafe { SetFocus(hwnd) };
            Ok(())
        }
        "set_text" => {
            let text = event.text.as_deref().unwrap_or("");
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            wide.push(0);
            // SAFETY: `wide` è NUL-terminato e vivo per la chiamata.
            let ok = unsafe { SetWindowTextW(hwnd, wide.as_ptr()) };
            if ok != 0 {
                Ok(())
            } else {
                Err("SetWindowTextW ha fallito".to_string())
            }
        }
        "select" => {
            let index = event
                .key
                .as_deref()
                .and_then(|s| s.parse::<isize>().ok())
                .ok_or_else(|| "`select` richiede `key` con un indice numerico".to_string())?;
            // Widget dichiarato (es. il guscio `tabs` di `Simplebook`):
            // il selettore registrato cambia pagina senza messaggi nativi.
            if declared::select(raw, index as usize) {
                return Ok(());
            }
            match class.as_str() {
                "ComboBox" => {
                    // SAFETY: messaggio di selezione con indice valido sul controllo.
                    unsafe { SendMessageW(hwnd, CB_SETCURSEL, index as usize, 0) };
                    Ok(())
                }
                "ListBox" => {
                    // SAFETY: messaggio di selezione con indice valido sul controllo.
                    unsafe { SendMessageW(hwnd, LB_SETCURSEL, index as usize, 0) };
                    Ok(())
                }
                // Tabella report: selezione singola via `LVM_SETITEMSTATE`.
                "SysListView32" => select_listview_row(hwnd, index),
                other => Err(format!("`select` non supportato per `{other}`")),
            }
        }
        "close" => {
            // Finestre native (dialog, popup tip C8, finestre del sistema):
            // `WM_CLOSE` avvia la normale sequenza di chiusura.
            // SAFETY: `PostMessageW` è asincrona e non legge il buffer.
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            Ok(())
        }
        other => Err(format!("azione non supportata: `{other}`")),
    }
}

/// Seleziona (e dà il focus a) una singola riga di una `SysListView32`.
///
/// Deseleziona tutte le righe e imposta `LVIS_SELECTED | LVIS_FOCUSED`
/// sulla riga richiesta, così `/tree` riporta `selected_rows:[index]`.
fn select_listview_row(hwnd: HWND, index: isize) -> Result<(), String> {
    if index < 0 {
        return Err("indice di selezione negativo".to_string());
    }
    let count = unsafe { SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0) }.max(0) as isize;
    if index >= count {
        return Err(format!("riga {index} fuori range (0..{count})"));
    }
    for i in 0..count {
        let mut item: LVITEMW = unsafe { std::mem::zeroed() };
        item.stateMask = (LVIS_SELECTED | LVIS_FOCUSED) as u32;
        item.state = if i == index {
            (LVIS_SELECTED | LVIS_FOCUSED) as u32
        } else {
            0
        };
        // SAFETY: `LVM_SETITEMSTATE` accetta l'indice in wParam e un
        // `LVITEMW` valido (solo `state`/`stateMask` significativi) in lParam.
        unsafe {
            SendMessageW(
                hwnd,
                LVM_SETITEMSTATE,
                i as usize,
                &mut item as *mut _ as isize,
            );
        }
    }
    Ok(())
}
