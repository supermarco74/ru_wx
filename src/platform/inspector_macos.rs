//! macOS — implementazione Livello 3 delle primitive dell'inspector
//! (cfr. `PIANO_DEBUG_INSPECTOR.md` §9).
//!
//! Gira **sul main thread** (garantito dal marshaling descritto in §5) ed
//! è l'unico punto, insieme ad `appkit.rs`, in cui si usa `objc2`. Gli
//! accessor ai registri privati restano in `appkit.rs`: qui si compone
//! soltanto la struttura dati `ComponentNode`.

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{msg_send, sel};
use objc2_app_kit::{NSBitmapImageFileType, NSColor, NSColorSpace, NSView, NSWindow};
use objc2_foundation::{NSArray, NSDictionary, NSIndexSet, NSRect, NSString};

use crate::platform::appkit;
use crate::platform::declared;
use crate::platform::inspector::{
    source, Bounds, ColorInfo, ComponentNode, InjectedEvent, Selector, WindowInfo,
};
use crate::platform::NativeHandle;

/// Limiti di sicurezza per alberi patologici.
const MAX_DEPTH: usize = 24;
const MAX_NODES: usize = 4000;

fn rect_to_bounds(r: crate::core::geometry::Rect) -> Bounds {
    Bounds::new(r.x as f64, r.y as f64, r.width as f64, r.height as f64)
}

/// Legge l'intero albero a partire dalla finestra radice.
pub fn dump_tree(root: NativeHandle) -> ComponentNode {
    let window = appkit::inspector_window(root);
    let content = appkit::inspector_content_view(root);

    let mut node = ComponentNode::new(root.to_string(), "NSWindow", "window");
    node.text = window.as_deref().map(|w| w.title().to_string());
    node.bounds = rect_to_bounds(appkit::mac_window_frame_rect(root));
    node.visible = window.as_deref().map(|w| w.isVisible()).unwrap_or(true);
    node.enabled = true;
    node.focused = true;
    node.source = source::NATIVE.to_string();

    if let Some(content) = content {
        let mut budget = MAX_NODES;
        for child in subviews_of(&content) {
            node.children
                .push(node_for_view(&child, window.as_deref(), 0, &mut budget));
        }
    } else {
        node.source = source::MISSING.to_string();
    }
    node
}

/// Metadati della finestra radice (`GET /window`).
pub fn window_info(root: NativeHandle) -> WindowInfo {
    let window = appkit::inspector_window(root);
    let frame = appkit::mac_window_frame_rect(root);
    let (client_w, client_h) = appkit::frame_client_size(root);
    let (screen_w, screen_h) = appkit::mac_screen_size();

    WindowInfo {
        id: root.to_string(),
        title: window
            .as_deref()
            .map(|w| w.title().to_string())
            .unwrap_or_default(),
        class: window
            .as_deref()
            .map(|_| "NSWindow".to_string())
            .unwrap_or_else(|| source::MISSING.to_string()),
        bounds: rect_to_bounds(frame),
        client_w,
        client_h,
        visible: window.as_deref().map(|w| w.isVisible()).unwrap_or(false),
        scale: window
            .as_deref()
            .map(|w| w.backingScaleFactor())
            .unwrap_or(1.0),
        dpi: 96,
        screen_w,
        screen_h,
        source: source::NATIVE.to_string(),
    }
}

// ── Screenshot (cfr. §9.3, Fase B5) ─────────────────────────────────

/// PNG della finestra (selettore vuoto) o del nodo indicato da `sel.id`.
///
/// Esegue `cacheDisplayInRect:toBitmapImageRep:` sulla view bersaglio:
/// così il ritaglio su un nodo non richiede conversioni di coordinate.
pub fn node_screenshot(root: NativeHandle, sel: &Selector) -> Result<Vec<u8>, String> {
    let target = match sel.id.as_deref() {
        Some(id) => find_view(root, id).ok_or_else(|| format!("nessun nodo con id `{id}`"))?,
        None => appkit::inspector_content_view(root)
            .ok_or_else(|| "content view non disponibile".to_string())?,
    };

    let rect = target.bounds();
    if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
        return Err("il nodo ha dimensioni nulle".to_string());
    }

    let rep = target
        .bitmapImageRepForCachingDisplayInRect(rect)
        .ok_or_else(|| "impossibile allocare il buffer immagine".to_string())?;
    target.cacheDisplayInRect_toBitmapImageRep(rect, &rep);

    let properties = NSDictionary::new();
    // SAFETY: `PNG` è una costante valida di `NSBitmapImageFileType`; la
    // mappa delle proprietà è vuota e `rep` resta vivo per la chiamata.
    let data =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties) }
            .ok_or_else(|| "codifica PNG fallita".to_string())?;
    Ok(data.to_vec())
}

// ── Walk dell'albero ────────────────────────────────────────────────

fn subviews_of(view: &NSView) -> Vec<Retained<NSView>> {
    let subs = view.subviews();
    let count = subs.count();
    (0..count).map(|i| subs.objectAtIndex(i)).collect()
}

/// Identificatore stabile di una view (stesso schema di `dump_tree`).
fn view_id(view: &NSView) -> String {
    match appkit::inspector_handle_for_view(view) {
        Some(h) => h.to_string(),
        None => format!("mac:0x{:x}", view as *const NSView as usize),
    }
}

fn node_for_view(
    view: &NSView,
    window: Option<&NSWindow>,
    depth: usize,
    budget: &mut usize,
) -> ComponentNode {
    *budget = budget.saturating_sub(1);

    let id = view_id(view);
    let class = appkit::inspector_class_name(view);
    let role = role_for_class(&class, view);

    let mut node = ComponentNode::new(id, class.clone(), role);
    node.text = read_text(view, &class);
    node.bounds = rect_to_bounds(appkit::inspector_view_screen_rect(view));
    node.visible = !appkit::inspector_view_hidden(view);
    node.enabled = read_enabled(view);
    node.focused = is_focused(view, window);
    node.colors = read_colors(view);
    node.state = read_state(view);
    node.extra = read_extra(view);
    if *budget == 0 {
        node.source = source::MISSING.to_string();
        return node;
    }
    if depth < MAX_DEPTH {
        for child in subviews_of(view) {
            node.children
                .push(node_for_view(&child, window, depth + 1, budget));
        }
    }
    node
}

// ── Reader per-widget (cfr. §6.4) ───────────────────────────────────

/// `true` se `obj` risponde a `selector` (`respondsToSelector:`).
fn responds<T: objc2::Message>(obj: &T, selector: Sel) -> bool {
    // SAFETY: `respondsToSelector:` è un metodo standard di NSObject e
    // non altera lo stato dell'oggetto.
    unsafe { msg_send![obj, respondsToSelector: selector] }
}

fn read_text(view: &NSView, class: &str) -> Option<String> {
    // `NSButton.stringValue` restituisce un valore di stato (es. "0"),
    // non l'etichetta visibile: per i pulsanti il testo è `title`.
    if class.contains("Button") {
        if let Some(text) = read_title(view) {
            return Some(text);
        }
    }
    if let Some(text) = read_string_value(view) {
        return Some(text);
    }
    read_title(view)
}

fn read_string_value(view: &NSView) -> Option<String> {
    if !responds(view, sel!(stringValue)) {
        return None;
    }
    // SAFETY: selezione verificata; ritorna `NSString` (o nil).
    let value: Option<Retained<NSString>> = unsafe { msg_send![view, stringValue] };
    value.map(|v| v.to_string()).filter(|text| !text.is_empty())
}

fn read_title(view: &NSView) -> Option<String> {
    if !responds(view, sel!(title)) {
        return None;
    }
    // SAFETY: selezione verificata; ritorna `NSString` (o nil).
    let value: Option<Retained<NSString>> = unsafe { msg_send![view, title] };
    value.map(|v| v.to_string()).filter(|text| !text.is_empty())
}

fn read_enabled(view: &NSView) -> bool {
    if responds(view, sel!(isEnabled)) {
        // SAFETY: selezione verificata; ritorna un BOOL.
        unsafe { msg_send![view, isEnabled] }
    } else {
        true
    }
}

fn read_editable(view: &NSView) -> bool {
    if responds(view, sel!(isEditable)) {
        // SAFETY: selezione verificata; ritorna un BOOL.
        unsafe { msg_send![view, isEditable] }
    } else {
        false
    }
}

/// Stato specifico del widget (`checked`, `editable`, `secure`, …).
fn read_state(view: &NSView) -> serde_json::Value {
    let class = appkit::inspector_class_name(view);
    let mut map = serde_json::Map::new();

    if responds(view, sel!(isEditable)) {
        map.insert(
            "editable".into(),
            serde_json::Value::Bool(read_editable(view)),
        );
    }
    if responds(view, sel!(isSelectable)) {
        // SAFETY: selezione verificata; ritorna un BOOL.
        let v: bool = unsafe { msg_send![view, isSelectable] };
        map.insert("selectable".into(), serde_json::Value::Bool(v));
    }
    if class.contains("NSButton") && responds(view, sel!(state)) {
        // SAFETY: selezione verificata; ritorna NSControlStateValue (isize).
        let v: isize = unsafe { msg_send![view, state] };
        map.insert("checked".into(), serde_json::Value::Bool(v != 0));
    }
    if class.contains("SecureTextField") {
        map.insert("secure".into(), serde_json::Value::Bool(true));
    }
    if let Some(placeholder) = read_placeholder(view) {
        map.insert("placeholder".into(), serde_json::Value::String(placeholder));
    }
    // Selezione dei widget senza classe nativa propria (es. il guscio di
    // `Simplebook`, ruolo `tabs`): è il registro `declared` a detenerla.
    if let Some(handle) = appkit::inspector_handle_for_view(view) {
        if let Some(selection) = declared::selection(handle) {
            // Il guscio di `RadioBox` è marcato `radio` (§6 del piano di
            // portabilità) e la stessa selezione va letta come
            // `checked_index`; per gli altri (book, simplebook) resta
            // `selection`.
            let key = if declared::role(handle) == Some("radio") {
                "checked_index"
            } else {
                "selection"
            };
            map.insert(key.into(), serde_json::json!(selection));
        }
    }

    if map.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(map)
    }
}

// ── Contenuti complessi (cfr. §6.4, Fase B3) ────────────────────────

/// Contenuti specifici del widget: scroll, tabelle, liste/combo.
///
/// I test usano `respondsToSelector:` sul selettore caratteristico e non
/// il nome della classe: le view *interne* di AppKit hanno nomi che
/// contengono quello del contenitore (es. `_NSScrollViewContentBackgroundView`
/// contiene `"NSScrollView"`) e un test per sottostringa le tratterebbe
/// erroneamente come NSScrollView → `msg_send!` su metodo inesistente.
fn read_extra(view: &NSView) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    if responds(view, sel!(documentVisibleRect)) {
        map.insert("scroll".into(), read_scroll(view));
    }
    if responds(view, sel!(numberOfRows)) && responds(view, sel!(tableColumns)) {
        map.insert("table".into(), read_table(view));
    }
    if responds(view, sel!(numberOfItems)) && responds(view, sel!(indexOfSelectedItem)) {
        map.insert("choice".into(), read_choice(view));
    }

    if map.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(map)
    }
}

fn read_scroll(view: &NSView) -> serde_json::Value {
    // SAFETY: `NSScrollView` espone `documentVisibleRect` (NSRect).
    let rect: NSRect = unsafe { msg_send![view, documentVisibleRect] };
    serde_json::json!({
        "x": rect.origin.x,
        "y": rect.origin.y,
        "w": rect.size.width,
        "h": rect.size.height,
    })
}

fn string_array(values: &NSArray<NSString>) -> Vec<serde_json::Value> {
    let count = values.count();
    (0..count)
        .map(|i| serde_json::Value::String(values.objectAtIndex(i).to_string()))
        .collect()
}

/// Espande un `NSIndexSet` in un vettore di indici.
///
/// `firstIndex`/`lastIndex` ritornano `NSNotFound` (che AppKit definisce
/// come `NSIntegerMax`, non `usize::MAX`) quando l'insieme è vuoto: senza
/// il controllo su `count()` un set vuoto produrrebbe un elemento
/// `9223372036854775807` invece di `[]`.
fn index_set_to_vec(set: &NSIndexSet) -> Vec<usize> {
    const NS_NOT_FOUND: usize = isize::MAX as usize;
    let mut out = Vec::new();
    if set.count() == 0 {
        return out;
    }
    let last = set.lastIndex();
    let mut idx = set.firstIndex();
    while idx != NS_NOT_FOUND {
        out.push(idx);
        if idx >= last {
            break;
        }
        idx = set.indexGreaterThanIndex(idx);
    }
    out
}

fn read_table(view: &NSView) -> serde_json::Value {
    // SAFETY: metodi standard di `NSTableView`; i tipi di ritorno sono
    // quelli dichiarati dalle API AppKit.
    let rows: isize = unsafe { msg_send![view, numberOfRows] };
    let cols: isize = unsafe { msg_send![view, numberOfColumns] };
    let columns: Retained<NSArray<objc2::runtime::AnyObject>> =
        unsafe { msg_send![view, tableColumns] };
    let count = columns.count();
    let mut titles = Vec::with_capacity(count);
    for i in 0..count {
        let column = columns.objectAtIndex(i);
        let title: Retained<NSString> = unsafe { msg_send![&*column, title] };
        titles.push(serde_json::Value::String(title.to_string()));
    }
    let selected: Retained<NSIndexSet> = unsafe { msg_send![view, selectedRowIndexes] };
    serde_json::json!({
        "row_count": rows,
        "col_count": cols,
        "columns": titles,
        "selected_rows": index_set_to_vec(&selected),
        // Le celle arrivano dal dataSource di ru_wx, non dalle view (§17).
        "cells": serde_json::Value::Null,
    })
}

fn read_choice(view: &NSView) -> serde_json::Value {
    let class = appkit::inspector_class_name(view);
    // SAFETY: `numberOfItems` / `indexOfSelectedItem` sono comuni a
    // `NSPopUpButton` e `NSComboBox`.
    let item_count: isize = unsafe { msg_send![view, numberOfItems] };
    let selected_index: isize = unsafe { msg_send![view, indexOfSelectedItem] };
    let items = if class.contains("NSPopUpButton") {
        // SAFETY: solo `NSPopUpButton` espone `itemTitles`.
        let titles: Retained<NSArray<NSString>> = unsafe { msg_send![view, itemTitles] };
        string_array(&titles)
    } else {
        Vec::new()
    };
    serde_json::json!({
        "item_count": item_count,
        "selected_index": selected_index,
        "items": items,
    })
}

fn read_placeholder(view: &NSView) -> Option<String> {
    if !responds(view, sel!(placeholderString)) {
        return None;
    }
    // SAFETY: selezione verificata; può ritornare nil.
    let value: Option<Retained<NSString>> = unsafe { msg_send![view, placeholderString] };
    value.map(|v| v.to_string()).filter(|s| !s.is_empty())
}

fn is_focused(view: &NSView, window: Option<&NSWindow>) -> bool {
    let Some(window) = window else {
        return false;
    };
    let Some(responder) = window.firstResponder() else {
        return false;
    };
    let target = view as *const NSView as usize;
    if &*responder as *const _ as usize == target {
        return true;
    }
    // I campi di testo sono modificati tramite un "field editor" distinto:
    // in quel caso il `delegate` del first responder è il controllo vero.
    if responds(&*responder, sel!(delegate)) {
        // SAFETY: selezione verificata; può ritornare nil.
        let delegate: Option<Retained<objc2::runtime::AnyObject>> =
            unsafe { msg_send![&*responder, delegate] };
        if let Some(delegate) = delegate {
            return &*delegate as *const _ as usize == target;
        }
    }
    false
}

fn read_colors(view: &NSView) -> ColorInfo {
    let mut info = ColorInfo::default();
    if responds(view, sel!(textColor)) {
        // SAFETY: selezione verificata; può ritornare nil.
        let color: Option<Retained<NSColor>> = unsafe { msg_send![view, textColor] };
        info.text = color.as_deref().and_then(color_to_hex);
    }
    if responds(view, sel!(backgroundColor)) {
        // SAFETY: selezione verificata; può ritornare nil.
        let color: Option<Retained<NSColor>> = unsafe { msg_send![view, backgroundColor] };
        info.background = color.as_deref().and_then(color_to_hex);
    }
    info
}

/// Converte un `NSColor` in `#RRGGBBAA`, passando per lo spazio sRGB.
/// Ritorna `None` se il colore non è convertibile (named/pattern color).
fn color_to_hex(color: &NSColor) -> Option<String> {
    let space = NSColorSpace::sRGBColorSpace();
    let converted = color.colorUsingColorSpace(&space)?;
    let (mut r, mut g, mut b, mut a) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    // SAFETY: `converted` è in sRGB; i quattro puntatori sono validi e
    // allineati.
    unsafe { converted.getRed_green_blue_alpha(&mut r, &mut g, &mut b, &mut a) };
    let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    Some(format!("#{:02X}{:02X}{:02X}{:02X}", q(r), q(g), q(b), q(a)))
}

/// Normalizza la classe nativa in un ruolo logico condiviso (§6.1).
fn role_for_class(class: &str, view: &NSView) -> String {
    // Il ruolo dichiarato dal widget (Livello 2) ha priorità sull'euristica
    // sulla classe: è l'unico modo per distinguere checkbox/radio, che su
    // macOS sono `NSButton` senza `buttonType` (cfr. PIANO §17.1).
    if let Some(role) = appkit::inspector_declared_role(view) {
        return role.to_string();
    }
    // Registro `declared` (sempre compilato): copre i container senza
    // classe nativa propria — il guscio di `Simplebook` (ruolo `tabs`).
    if let Some(handle) = appkit::inspector_handle_for_view(view) {
        if let Some(role) = declared::role(handle) {
            return role.to_string();
        }
    }
    if class.contains("NSWindow") {
        "window"
    } else if class.contains("NSSecureTextField") {
        "input"
    } else if class.contains("NSComboBox") || class.contains("NSPopUpButton") {
        "choice"
    } else if class.contains("NSTableView")
        || class.contains("TableView")
        || class.contains("ListCtrlView")
    {
        "table"
    } else if class.contains("ListBoxView") {
        "list"
    } else if class.contains("TreeView") {
        "tree"
    } else if class.contains("NSScrollView") {
        "scroll"
    } else if class.contains("NSSlider")
        || class.contains("NSStepper")
        || class.contains("NSProgressIndicator")
        || class.contains("NSLevelIndicator")
    {
        "value"
    } else if class.contains("NSTabView") {
        "tabs"
    } else if class.contains("NSButton") {
        "button"
    } else if class.contains("NSTextView") {
        "input"
    } else if class.contains("NSTextField") {
        if read_editable(view) {
            "input"
        } else {
            "label"
        }
    } else if class.contains("NSImageView") {
        "image"
    } else if class.contains("NSBox") || class.contains("Panel") {
        "container"
    } else {
        "view"
    }
    .to_string()
}

// ── Azioni sui controlli (cfr. §11, Fase B4) ────────────────────────
//
// L'azione è espressa come chiamata ai metodi nativi del controllo
// (`performClick:`, `setStringValue:`, `makeFirstResponder:`, …), non
// come evento sintetico di basso livello: così non servono i permessi di
// Accessibilità e il comportamento è identico a quello dell'utente.

fn find_view(root: NativeHandle, id: &str) -> Option<Retained<NSView>> {
    let content = appkit::inspector_content_view(root)?;
    find_rec(&content, 0, id)
}

fn find_rec(view: &NSView, depth: usize, id: &str) -> Option<Retained<NSView>> {
    if view_id(view) == id {
        // SAFETY: `view` appartiene all'albero della finestra, vivo per
        // tutta la durata dell'evento; incrementiamo il refcount.
        return unsafe { Retained::retain(view as *const NSView as *mut NSView) };
    }
    if depth >= MAX_DEPTH {
        return None;
    }
    for child in subviews_of(view) {
        if let Some(found) = find_rec(&child, depth + 1, id) {
            return Some(found);
        }
    }
    None
}

/// Esegue un'azione su un widget identificato da `event.id`.
pub fn inject_event(root: NativeHandle, event: &InjectedEvent) -> Result<(), String> {
    let id = event
        .id
        .as_deref()
        .ok_or_else(|| "evento senza `id`: le coordinate non sono ancora supportate".to_string())?;

    // `close` può bersagliare la finestra radice stessa (un dialogo, cfr.
    // C8): in quel caso non esiste una view con quell'id, si chiude la
    // `NSWindow` direttamente.
    if event.action == "close" {
        return close_window(root, id);
    }

    let view = find_view(root, id).ok_or_else(|| format!("nessun nodo con id `{id}`"))?;

    match event.action.as_str() {
        "click" => perform_click(&view),
        "focus" => set_focus(root, &view),
        "set_text" => set_text(&view, event.text.as_deref().unwrap_or("")),
        "select" => select(&view, event),
        other => Err(format!("azione non supportata: `{other}`")),
    }
}

/// Chiude la finestra che ospita il nodo (`NSWindow.close`).
///
/// L'id può essere la finestra radice stessa oppure una view al suo
/// interno: in entrambi i casi si arriva alla `NSWindow` da chiudere.
fn close_window(root: NativeHandle, id: &str) -> Result<(), String> {
    let window = if id == root.to_string() {
        appkit::inspector_window(root).ok_or_else(|| "finestra non disponibile".to_string())?
    } else {
        let view = find_view(root, id).ok_or_else(|| format!("nessun nodo con id `{id}`"))?;
        // SAFETY: `window` ritorna (aautorelease) la finestra che possiede
        // la view; l'oggetto è gestito da AppKit per tutta la chiamata.
        let window: Option<Retained<NSWindow>> = unsafe { msg_send![&*view, window] };
        window.ok_or_else(|| "la view non appartiene a nessuna finestra".to_string())?
    };
    // SAFETY: `close` è il metodo standard di `NSWindow` (senza salvataggi).
    unsafe {
        let _: () = msg_send![&*window, close];
    }
    Ok(())
}

fn perform_click(view: &NSView) -> Result<(), String> {
    if !responds(view, sel!(performClick:)) {
        return Err("il widget non implementa `performClick:`".to_string());
    }
    // SAFETY: selezione verificata; `performClick:` simula il click del
    // controllo e accetta lo stesso controllo come `sender`.
    unsafe {
        let _: () = msg_send![view, performClick: view];
    };
    Ok(())
}

fn set_focus(root: NativeHandle, view: &NSView) -> Result<(), String> {
    let window =
        appkit::inspector_window(root).ok_or_else(|| "finestra non disponibile".to_string())?;
    // SAFETY: `makeFirstResponder:` è l'API standard di AppKit.
    let ok: bool = unsafe { msg_send![&*window, makeFirstResponder: view] };
    if ok {
        Ok(())
    } else {
        Err("il controllo ha rifiutato il focus".to_string())
    }
}

fn set_text(view: &NSView, text: &str) -> Result<(), String> {
    if !responds(view, sel!(setStringValue:)) {
        return Err("il widget non ha `setStringValue:`".to_string());
    }
    let value = NSString::from_str(text);
    // SAFETY: selezione verificata; `value` resta vivo per la chiamata.
    unsafe {
        let _: () = msg_send![view, setStringValue: &*value];
    };
    Ok(())
}

fn select(view: &NSView, event: &InjectedEvent) -> Result<(), String> {
    let index = event
        .key
        .as_deref()
        .and_then(|s| s.parse::<isize>().ok())
        .ok_or_else(|| "`select` richiede `key` con un indice numerico".to_string())?;

    // Widget dichiarato (es. il guscio `tabs` di `Simplebook`): il
    // selettore registrato cambia pagina senza chiamate native.
    if index >= 0 {
        if let Some(handle) = appkit::inspector_handle_for_view(view) {
            if declared::select(handle, index as usize) {
                return Ok(());
            }
        }
    }

    if responds(view, sel!(selectItemAtIndex:)) {
        // SAFETY: selezione verificata; `index` è l'indice di menu/combo.
        unsafe {
            let _: () = msg_send![view, selectItemAtIndex: index];
        };
        return Ok(());
    }
    if index >= 0 && responds(view, sel!(selectRowIndexes:byExtendingSelection:)) {
        let set = NSIndexSet::indexSetWithIndex(index as usize);
        // SAFETY: selezione verificata; `set` resta vivo per la chiamata.
        unsafe {
            let _: () = msg_send![view, selectRowIndexes: &*set, byExtendingSelection: false];
        }
        return Ok(());
    }
    Err("il widget non supporta la selezione per indice".to_string())
}
