//! Backend GTK4 per Linux — finestre e widget GTK reali collegati
//! all'API unificata `Frame` / `Panel` / `Button` / `StaticText` /
//! `TextCtrl`.
//!
//! Il backend segue la stessa architettura di `appkit.rs`:
//!
//! * le mappe `thread_local!` tengono il collegamento handle nativo →
//!   widget GTK, handle → genitore e handle frame → `Weak<FrameData>`;
//! * `gtk4::init()` viene tentato pigramente e **solo dal thread
//!   principale** del processo: GTK4 è main-thread-only e inizializzarlo
//!   da un thread diverso fa crashare `g_type_create_instance`;
//! * in ambiente headless (test CI senza display) `ensure_ready()`
//!   restituisce `false` e ogni `create_*` degrada all'handle stub
//!   in memoria, così `tests/stub_integration.rs` continua a passare.
//!
//! Nota: con la feature `v4_10` gtk-rs marca come deprecated tutta la
//! famiglia `TreeView` / `ListStore` / `TreeSelection` (GTK 4.10
//! suggerisce `ColumnView` / `GridView`). Il `ListCtrl` è basato su
//! `TreeView` perché è il widget che mappa 1:1 la semantica "report"
//! della libreria: gli avvisi di deprecation sono quindi silenziati
//! a livello di modulo.
#![allow(deprecated)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use gdk4_x11::X11Surface;
use gtk4::glib::Propagation;
use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk4::{
    Box as GtkBox, Button, CssProvider, Dialog, Entry, EventControllerKey, Fixed, Label,
    PopoverMenuBar, ResponseType, ScrolledWindow, TextView, Window,
};

use crate::core::accelerator::{Accelerator, VirtualKey};
use crate::core::geometry::{Colour, Rect};
use crate::dialogs::message_box::{MessageBoxIcon, MessageBoxResult, MessageBoxStyle};
use crate::platform::stub_backend::alloc_widget_handle;
use crate::platform::NativeHandle;
use crate::window::frame::{Frame, FrameData};
use crate::window::menu::MenuItemKind;

/// Riga di menu condivisa con il costruttore della menu bar
/// (stessa forma della controparte appkit).
pub type MenuInstallRow = (u16, String, bool, MenuItemKind, bool, Option<Accelerator>);

// ── Inizializzazione (solo dal thread principale) ──────────────────

/// `true` se il thread corrente è quello principale del processo.
///
/// GTK4 è main-thread-only: `gtk_init_check` va chiamata una volta
/// sola e i widget vanno creati dal thread che l'ha inizializzata.
/// Usarla da un thread diverso — come fanno i thread che `cargo test`
/// crea per ogni caso — termina in SIGSEGV dentro
/// `g_type_create_instance`. Su Linux il TID del thread principale
/// coincide con il PID.
fn is_main_thread() -> bool {
    // SAFETY: `getpid` e `gettid` sono chiamate thread-safe e senza
    // effetti collaterali.
    unsafe { libc::syscall(libc::SYS_gettid) == libc::getpid() as libc::c_long }
}

/// Inizializza GTK4 se possibile.
///
/// Restituisce `true` solo se il GTK è davvero inizializzato sul
/// thread principale. In ambiente headless `gtk_init_check` fallisce e
/// la funzione restituisce `false` senza panicare; da un thread non
/// principale restituisce `false` senza tentare l'init, così i widget
/// degradano agli stub in-memory invece di far crashare il processo.
pub fn init() -> bool {
    if !is_main_thread() {
        return false;
    }
    if gtk4::is_initialized_main_thread() {
        return true;
    }
    gtk4::init().is_ok()
}

/// `true` se i widget GTK reali possono essere creati sul thread corrente.
pub fn gtk_available() -> bool {
    gtk4::is_initialized_main_thread()
}

/// Prova a rendere disponibile GTK (idempotente).
fn ensure_ready() -> bool {
    gtk_available() || init()
}

// ── Registri handle → widget ────────────────────────────────────────

thread_local! {
    /// Handle nativo → widget GTK.
    static VIEWS: RefCell<HashMap<NativeHandle, gtk4::Widget>> =
        RefCell::new(HashMap::new());
    /// Handle nativo → handle del genitore.
    static VIEW_PARENT: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
    /// Handle frame → contenitore `Fixed` dell'area client.
    static FRAME_CONTENT: RefCell<HashMap<NativeHandle, Fixed>> =
        RefCell::new(HashMap::new());
    /// Handle frame → `gtk::Box` radice (menu bar + contenuto).
    static FRAME_ROOT: RefCell<HashMap<NativeHandle, GtkBox>> =
        RefCell::new(HashMap::new());
    /// Handle frame → finestra GTK.
    static FRAME_WINDOWS: RefCell<HashMap<NativeHandle, Window>> =
        RefCell::new(HashMap::new());
    /// Handle frame → `FrameData` debole.
    static FRAME_DATA_WEAK: RefCell<HashMap<NativeHandle, Weak<RefCell<FrameData>>>> =
        RefCell::new(HashMap::new());
    /// Handle frame → menu bar GTK installata.
    static FRAME_MENUBARS: RefCell<HashMap<NativeHandle, PopoverMenuBar>> =
        RefCell::new(HashMap::new());
    /// Handle frame → gruppo delle azioni della menu bar.
    static FRAME_ACTION_GROUPS: RefCell<HashMap<NativeHandle, gio::SimpleActionGroup>> =
        RefCell::new(HashMap::new());
    /// Handle frame → modello menu (per reinstallare su cambio shortcut).
    static FRAME_MENUS: RefCell<HashMap<NativeHandle, Vec<(String, Vec<MenuInstallRow>)>>> =
        RefCell::new(HashMap::new());
    /// Handle frame → tabella acceleratori.
    static FRAME_ACCELS: RefCell<HashMap<NativeHandle, Vec<(Accelerator, u16)>>> =
        RefCell::new(HashMap::new());
}

pub(crate) fn widget_of(handle: NativeHandle) -> Option<gtk4::Widget> {
    VIEWS.with(|map| map.borrow().get(&handle).cloned())
}

/// Size minima (larghezza, altezza) che GTK impone davvero al widget
/// per l'handle dato — `MAX(contenuto/CSS, size_request)`.
///
/// Il sizer la usa come size effettiva degli item: se la nostra stima
/// (lo stub) è più piccola del minimo GTK, il contenitore stimerebbe
/// meno spazio di quello che GTK assegnerà, spingendo il `Fixed` a
/// richiedere una finestra più alta a ogni ciclo (loop di crescita).
/// Restituisce `None` per gli handle senza widget GTK (stub puri).
pub(crate) fn view_min_size(handle: NativeHandle) -> Option<(u32, u32)> {
    let widget = widget_of(handle)?;
    let (_, min_h, _, _) = widget.measure(gtk4::Orientation::Vertical, -1);
    let (min_w, _, _, _) = widget.measure(gtk4::Orientation::Horizontal, -1);
    Some((min_w.max(0) as u32, min_h.max(0) as u32))
}

/// Finestra GTK registrata per l'handle di una frame (l'handle radice
/// dell'inspector è quello stub di `Frame::hwnd()`, non un puntatore).
pub(crate) fn frame_window_of(handle: NativeHandle) -> Option<Window> {
    FRAME_WINDOWS.with(|map| map.borrow().get(&handle).cloned())
}

/// Enumera le finestre top-level create (handle frame → finestra),
/// per il walk dell'inspector (Livello 3).
pub(crate) fn frame_windows() -> Vec<(NativeHandle, Window)> {
    FRAME_WINDOWS.with(|map| {
        map.borrow()
            .iter()
            .map(|(handle, window)| (*handle, window.clone()))
            .collect()
    })
}

/// Enumera le viste registrate (handle runtime → widget): serve
/// all'inspector per mappare i puntatori GTK agli id decimali.
pub(crate) fn registered_views() -> Vec<(NativeHandle, gtk4::Widget)> {
    VIEWS.with(|map| {
        map.borrow()
            .iter()
            .map(|(handle, widget)| (*handle, widget.clone()))
            .collect()
    })
}

/// Contenitore `Fixed` che ospita l'handle dato (frame o panel).
fn fixed_for(handle: NativeHandle) -> Option<Fixed> {
    if let Some(fixed) = FRAME_CONTENT.with(|map| map.borrow().get(&handle).cloned()) {
        return Some(fixed);
    }
    widget_of(handle)?.downcast::<Fixed>().ok()
}

/// Inserisce un widget figlio nel contenitore del genitore.
fn attach(parent: NativeHandle, handle: NativeHandle, widget: &gtk4::Widget, rect: Rect) {
    VIEWS.with(|map| map.borrow_mut().insert(handle, widget.clone()));
    record_parent(handle, parent);
    widget.set_size_request(rect.width as i32, rect.height as i32);
    if let Some(fixed) = fixed_for(parent) {
        // Se il widget è già ospitato altrove, staccalo prima: `put`
        // richiede un widget senza genitore.
        if widget.parent().is_some() {
            widget.unparent();
        }
        fixed.put(widget, rect.x as f64, rect.y as f64);
    }
}

fn record_parent(child: NativeHandle, parent: NativeHandle) {
    VIEW_PARENT.with(|map| map.borrow_mut().insert(child, parent));
}

/// Risale la catena di genitori fino a trovare l'handle di una frame.
pub fn resolve_frame_handle(mut handle: NativeHandle) -> NativeHandle {
    for _ in 0..64 {
        if FRAME_CONTENT.with(|map| map.borrow().contains_key(&handle)) {
            return handle;
        }
        if let Some(parent) = VIEW_PARENT.with(|map| map.borrow().get(&handle).copied()) {
            handle = parent;
        } else {
            break;
        }
    }
    handle
}

// ── Dispatch eventi verso la Frame ──────────────────────────────────

/// Registra la frame debole sotto il suo handle stub (l'handle di
/// `Frame::hwnd()` su questo target).
pub fn register_frame_weak(handle: NativeHandle, frame: &Frame) {
    FRAME_DATA_WEAK.with(|map| {
        map.borrow_mut().insert(handle, Rc::downgrade(&frame.inner));
    });
}

fn frame_data(frame_handle: NativeHandle) -> Option<Rc<RefCell<FrameData>>> {
    FRAME_DATA_WEAK.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .and_then(|weak| weak.upgrade())
    })
}

/// Invoca il command handler registrato per `command_id`.
pub fn dispatch_command(frame_handle: NativeHandle, command_id: u16) {
    let frame_handle = resolve_frame_handle(frame_handle);
    if let Some(data) = frame_data(frame_handle) {
        let handler = data.borrow_mut().command_handlers.remove(&command_id);
        if let Some(mut handler) = handler {
            handler();
            data.borrow_mut()
                .command_handlers
                .insert(command_id, handler);
        }
    }
}

/// Protocollo di chiusura: `on_query_close` (veto) → `on_close` →
/// `stub.visible = false`. Restituisce `true` se la chiusura procede.
fn request_close_via_data(frame_handle: NativeHandle) -> bool {
    let Some(data) = frame_data(frame_handle) else {
        return true;
    };

    // Preleva i callback in un borrow breve e separato: il loro corpo
    // può a sua volta accedere alla RefCell della frame.
    let event = crate::CloseEvent::new();
    let query = data.borrow_mut().on_query_close.take();
    if let Some(mut cb) = query {
        cb(&event);
        data.borrow_mut().on_query_close = Some(cb);
    }
    if event.is_vetoed() {
        return false;
    }

    let on_close = {
        let mut borrow = data.borrow_mut();
        borrow.stub.visible = false;
        borrow.on_close.take()
    };
    if let Some(mut cb) = on_close {
        cb();
        data.borrow_mut().on_close = Some(cb);
    }
    true
}

fn cleanup_frame_maps(frame_handle: NativeHandle) {
    FRAME_CONTENT.with(|map| map.borrow_mut().remove(&frame_handle));
    FRAME_ROOT.with(|map| map.borrow_mut().remove(&frame_handle));
    FRAME_MENUBARS.with(|map| map.borrow_mut().remove(&frame_handle));
    FRAME_ACTION_GROUPS.with(|map| map.borrow_mut().remove(&frame_handle));
    FRAME_MENUS.with(|map| map.borrow_mut().remove(&frame_handle));
    FRAME_ACCELS.with(|map| map.borrow_mut().remove(&frame_handle));
}

fn on_close_request(frame_handle: NativeHandle) -> bool {
    if !request_close_via_data(frame_handle) {
        return false;
    }
    FRAME_WINDOWS.with(|map| map.borrow_mut().remove(&frame_handle));
    cleanup_frame_maps(frame_handle);
    true
}

// ── Frame ───────────────────────────────────────────────────────────

/// Crea la finestra GTK reale e la collega all'handle stub della frame.
///
/// Restituisce `false` se GTK non è disponibile (headless): la frame
/// continua a vivere solo come stub in memoria.
pub fn create_frame_window(
    frame_handle: NativeHandle,
    title: &str,
    width: u32,
    height: u32,
) -> bool {
    if FRAME_WINDOWS.with(|map| map.borrow().contains_key(&frame_handle)) {
        return true;
    }
    if !ensure_ready() {
        return false;
    }

    let window = Window::new();
    window.set_title(Some(title));
    window.set_default_size(width as i32, height as i32);

    // Layout verticale: [menu bar opzionale] + area client `Fixed`.
    let root = GtkBox::new(gtk4::Orientation::Vertical, 0);
    let content = Fixed::new();
    content.set_hexpand(true);
    content.set_vexpand(true);
    root.append(&content);
    window.set_child(Some(&root));

    let close_handle = frame_handle;
    window.connect_close_request(move |_| {
        if on_close_request(close_handle) {
            Propagation::Proceed
        } else {
            Propagation::Stop
        }
    });

    let accel_handle = frame_handle;
    let keys = EventControllerKey::new();
    keys.connect_key_pressed(move |_controller, key, _code, modifiers| {
        if accelerator_activated(accel_handle, key, modifiers) {
            Propagation::Stop
        } else {
            Propagation::Proceed
        }
    });
    window.add_controller(keys);

    FRAME_WINDOWS.with(|map| map.borrow_mut().insert(frame_handle, window));
    FRAME_ROOT.with(|map| map.borrow_mut().insert(frame_handle, root));
    FRAME_CONTENT.with(|map| map.borrow_mut().insert(frame_handle, content));
    true
}

/// Mostra e porta in primo piano la finestra della frame.
pub fn present_frame(frame_handle: NativeHandle) {
    if let Some(window) = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned()) {
        window.present();
    }
}

/// Nasconde la finestra della frame.
pub fn hide_frame(frame_handle: NativeHandle) {
    if let Some(window) = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned()) {
        window.set_visible(false);
    }
}

/// Chiude la finestra al termine del loop di `Frame::show()`.
///
/// Se l'utente ha già chiuso la finestra (X) le mappe sono vuote e la
/// chiamata è un no-op; se l'utente l'ha solo nascosta, la finestra
/// resta viva per un successivo `present_frame`.
pub fn end_show(frame_handle: NativeHandle) {
    let window = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned());
    if let Some(window) = window {
        if window.is_visible() {
            FRAME_WINDOWS.with(|map| map.borrow_mut().remove(&frame_handle));
            cleanup_frame_maps(frame_handle);
            // `destroy()` non emette `close-request`: niente doppio
            // passaggio di `on_close`.
            window.destroy();
        }
    }
}

/// Aggiorna il titolo della finestra.
pub fn set_frame_title(frame_handle: NativeHandle, title: &str) {
    if let Some(window) = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned()) {
        window.set_title(Some(title));
    }
}

/// Ridimensiona la finestra (dimensione esterna, come `set_default_size`).
pub fn set_frame_size(frame_handle: NativeHandle, width: u32, height: u32) {
    if let Some(window) = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned()) {
        window.set_default_size(width as i32, height as i32);
        window.set_size_request(width as i32, height as i32);
    }
}

/// Sfondo dell'area client via CSS inline (una classe per frame).
pub fn frame_set_content_background(frame_handle: NativeHandle, colour: Colour) {
    if !ensure_ready() {
        return;
    }
    let Some(content) = FRAME_CONTENT.with(|map| map.borrow().get(&frame_handle).cloned()) else {
        return;
    };
    let name = format!("ruwx-client-{}", frame_handle);
    content.set_widget_name(&name);
    let css = format!(
        "#{} {{ background-color: rgba({}, {}, {}, {:.3}); }}",
        name,
        colour.r,
        colour.g,
        colour.b,
        colour.a as f32 / 255.0
    );
    let provider = CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Sfondo generico di un contenitore (panel).
pub fn set_container_background(handle: NativeHandle, colour: Colour) {
    if !ensure_ready() {
        return;
    }
    let Some(widget) = widget_of(handle) else {
        return;
    };
    let name = format!("ruwx-container-{}", handle);
    widget.set_widget_name(&name);
    let css = format!(
        "#{} {{ background-color: rgba({}, {}, {}, {:.3}); }}",
        name,
        colour.r,
        colour.g,
        colour.b,
        colour.a as f32 / 255.0
    );
    let provider = CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

// ── Loop degli eventi ───────────────────────────────────────────────

/// Drena la coda GTK e sincronizza la geometria della frame.
///
/// Da chiamare a ogni iterazione del loop in `Frame::show()`.
pub fn pump(frame_handle: NativeHandle) {
    if !gtk_available() {
        return;
    }
    let context = glib::MainContext::default();
    while context.pending() {
        context.iteration(false);
    }
    poll_frame_size(frame_handle);
}

/// Rileva il resize reale dell'area client e aggiorna stub + sizer +
/// callback `on_resize` (equivalente della `WM_SIZE` di Win32).
fn poll_frame_size(frame_handle: NativeHandle) {
    let size = FRAME_CONTENT.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .map(|content| (content.allocated_width(), content.allocated_height()))
    });
    let Some((width, height)) = size else {
        return;
    };
    if width <= 0 || height <= 0 {
        return;
    }
    let Some(data) = frame_data(frame_handle) else {
        return;
    };
    let changed = {
        let borrow = data.borrow();
        borrow.stub.rect.width != width as u32 || borrow.stub.rect.height != height as u32
    };
    if !changed {
        return;
    }
    data.borrow_mut().stub.rect.width = width as u32;
    data.borrow_mut().stub.rect.height = height as u32;
    data.borrow_mut()
        .layout_client_area(width as u32, height as u32);

    let mut callbacks = std::mem::take(&mut data.borrow_mut().on_resize);
    for cb in callbacks.iter_mut() {
        cb(width as u32, height as u32);
    }
    data.borrow_mut().on_resize = callbacks;
}

// ── Acceleratori ────────────────────────────────────────────────────

/// Pubblica la tabella acceleratori della frame (da chiamare in `show()`).
pub fn set_frame_accelerators(frame_handle: NativeHandle, accels: &[(Accelerator, u16)]) {
    FRAME_ACCELS.with(|map| map.borrow_mut().insert(frame_handle, accels.to_vec()));
}

/// Nome GDK di una tastiera virtuale, se supportata.
fn virtual_key_name(key: VirtualKey) -> Option<&'static str> {
    Some(match key {
        VirtualKey::F1 => "F1",
        VirtualKey::F2 => "F2",
        VirtualKey::F3 => "F3",
        VirtualKey::F4 => "F4",
        VirtualKey::F5 => "F5",
        VirtualKey::F6 => "F6",
        VirtualKey::F7 => "F7",
        VirtualKey::F8 => "F8",
        VirtualKey::F9 => "F9",
        VirtualKey::F10 => "F10",
        VirtualKey::F11 => "F11",
        VirtualKey::F12 => "F12",
        VirtualKey::Escape => "Escape",
        VirtualKey::Tab => "Tab",
        VirtualKey::Enter => "Return",
        VirtualKey::Space => "space",
        VirtualKey::Backspace => "BackSpace",
        VirtualKey::Delete => "Delete",
        VirtualKey::Insert => "Insert",
        VirtualKey::Home => "Home",
        VirtualKey::End => "End",
        VirtualKey::PageUp => "Page_Up",
        VirtualKey::PageDown => "Page_Down",
        VirtualKey::Left => "Left",
        VirtualKey::Right => "Right",
        VirtualKey::Up => "Up",
        VirtualKey::Down => "Down",
        VirtualKey::Char(_) => return None,
    })
}

fn accelerator_activated(
    frame_handle: NativeHandle,
    key: gdk::Key,
    modifiers: gdk::ModifierType,
) -> bool {
    let accels = FRAME_ACCELS.with(|map| map.borrow().get(&frame_handle).cloned());
    let Some(accels) = accels else {
        return false;
    };

    let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
    let alt = modifiers.contains(gdk::ModifierType::ALT_MASK);
    let shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);

    for (accelerator, command_id) in accels {
        if accelerator.modifiers.ctrl() != ctrl
            || accelerator.modifiers.alt() != alt
            || accelerator.modifiers.shift() != shift
        {
            continue;
        }
        let matched = match accelerator.key {
            VirtualKey::Char(expected) => key
                .to_unicode()
                .map(|pressed| pressed.to_ascii_uppercase() == expected.to_ascii_uppercase())
                .unwrap_or(false),
            other => match virtual_key_name(other) {
                Some(name) => gdk::Key::from_name(name) == Some(key),
                None => false,
            },
        };
        if matched {
            dispatch_command(frame_handle, command_id);
            return true;
        }
    }
    false
}

// ── Widget: creazione ───────────────────────────────────────────────

/// Crea un panel (`Fixed` annidato) dentro una frame o un altro panel.
pub fn create_panel(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let panel = Fixed::new();
    attach(
        parent,
        handle,
        &panel.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Crea un pulsante che, al click, invia `command_id` alla frame.
pub fn create_button(
    parent: NativeHandle,
    title: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let button = Button::with_label(title);
    let target = resolve_frame_handle(parent);
    button.connect_clicked(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &button.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Crea un label statico.
pub fn create_static_text(parent: NativeHandle, text: &str, initial_rect: Rect) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let label = Label::new(Some(text));
    attach(
        parent,
        handle,
        &label.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Crea una singola riga di testo editabile che notifica `command_id`.
pub fn create_text_field(
    parent: NativeHandle,
    initial_text: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let entry = Entry::new();
    entry.set_text(initial_text);
    let target = resolve_frame_handle(parent);
    entry.connect_changed(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &entry.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Crea un campo di testo con la maschera di password.
pub fn create_secure_text_field(
    parent: NativeHandle,
    initial_text: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let entry = Entry::new();
    entry.set_text(initial_text);
    entry.set_visibility(false);
    let target = resolve_frame_handle(parent);
    entry.connect_changed(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &entry.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Crea un controllo multilinea con scrollbar che notifica `command_id`.
pub fn create_multiline_text_ctrl(
    parent: NativeHandle,
    initial_text: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let view = TextView::new();
    view.buffer().set_text(initial_text);
    let target = resolve_frame_handle(parent);
    view.buffer()
        .connect_changed(move |_| dispatch_command(target, command_id));
    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&view));
    attach(
        parent,
        handle,
        &scrolled.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

// ── CheckBox ───────────────────────────────────────────────────────

/// Il `gtk::CheckButton` registrato per l'handle.
fn check_box_of(handle: NativeHandle) -> Option<gtk4::CheckButton> {
    widget_of(handle)?.downcast::<gtk4::CheckButton>().ok()
}

/// Crea un checkbox che, al toggle, invia `command_id` alla frame.
pub fn create_check_box(
    parent: NativeHandle,
    label: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let check = gtk4::CheckButton::with_label(label);
    let target = resolve_frame_handle(parent);
    check.connect_toggled(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &check.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Stato selezionato del checkbox.
pub fn check_box_is_checked(handle: NativeHandle) -> bool {
    check_box_of(handle).is_some_and(|check| check.is_active())
}

/// Imposta lo stato selezionato del checkbox.
pub fn check_box_set_checked(handle: NativeHandle, checked: bool) {
    if let Some(check) = check_box_of(handle) {
        check.set_active(checked);
    }
}

/// Imposta l'etichetta del checkbox.
pub fn check_box_set_label(handle: NativeHandle, label: &str) {
    if let Some(check) = check_box_of(handle) {
        check.set_label(Some(label));
    }
}

// ── RadioButton ─────────────────────────────────────────────────────

thread_local! {
    /// Handle del leader del gruppo radio corrente (l'ultimo `is_group_start`).
    static RADIO_GROUP_LEADER: std::cell::Cell<Option<NativeHandle>> =
        const { std::cell::Cell::new(None) };
}

/// Crea un radio button che, al toggle, invia `command_id` alla frame.
///
/// `is_group_start` apre un nuovo gruppo (equivalente di `WS_GROUP`); in caso
/// contrario il pulsante si unisce al gruppo aperto più di recente.
pub fn create_radio_button(
    parent: NativeHandle,
    label: &str,
    command_id: u16,
    is_group_start: bool,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let radio = gtk4::CheckButton::with_label(label);
    let leader = RADIO_GROUP_LEADER.with(|l| l.get());
    match leader {
        // Gruppo nuovo (o leader andato): il pulsante diventa il leader.
        None => {
            radio.set_group(None::<&gtk4::CheckButton>);
            RADIO_GROUP_LEADER.with(|l| l.set(Some(handle)));
        }
        Some(_) if is_group_start => {
            radio.set_group(None::<&gtk4::CheckButton>);
            RADIO_GROUP_LEADER.with(|l| l.set(Some(handle)));
        }
        Some(leader_handle) => match check_box_of(leader_handle) {
            // Unisciti al gruppo corrente.
            Some(leader_widget) => radio.set_group(Some(&leader_widget)),
            // Il leader non esiste più: apre un gruppo fresco.
            None => {
                radio.set_group(None::<&gtk4::CheckButton>);
                RADIO_GROUP_LEADER.with(|l| l.set(Some(handle)));
            }
        },
    }
    let target = resolve_frame_handle(parent);
    radio.connect_toggled(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &radio.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Stato selezionato del radio button.
pub fn radio_button_is_selected(handle: NativeHandle) -> bool {
    check_box_of(handle).is_some_and(|radio| radio.is_active())
}

/// Imposta lo stato selezionato del radio button.
pub fn radio_button_set_selected(handle: NativeHandle, selected: bool) {
    if let Some(radio) = check_box_of(handle) {
        radio.set_active(selected);
    }
}

// ── ToggleButton ─────────────────────────────────────────────────────

/// Il `gtk::ToggleButton` registrato per l'handle.
fn toggle_button_of(handle: NativeHandle) -> Option<gtk4::ToggleButton> {
    widget_of(handle)?.downcast::<gtk4::ToggleButton>().ok()
}

/// Crea un toggle button che, al toggle, invia `command_id` alla frame.
pub fn create_toggle_button(
    parent: NativeHandle,
    label: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let button = gtk4::ToggleButton::with_label(label);
    let target = resolve_frame_handle(parent);
    button.connect_toggled(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &button.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Stato premuto del toggle button.
pub fn toggle_button_is_pressed(handle: NativeHandle) -> bool {
    toggle_button_of(handle).is_some_and(|button| button.is_active())
}

/// Imposta lo stato premuto del toggle button.
pub fn toggle_button_set_pressed(handle: NativeHandle, pressed: bool) {
    if let Some(button) = toggle_button_of(handle) {
        button.set_active(pressed);
    }
}

/// Imposta l'etichetta del toggle button.
pub fn toggle_button_set_label(handle: NativeHandle, label: &str) {
    if let Some(button) = toggle_button_of(handle) {
        button.set_label(label);
    }
}

// ── ComboBox ────────────────────────────────────────────────────────

/// Il `gtk::ComboBoxText` registrato per l'handle.
///
/// `GtkComboBoxText` è deprecato da GTK 4.10 ma resta l'unico widget
/// GTK4 che combina lista a tendina e campo di testo editabile.
#[allow(deprecated)]
fn combo_box_of(handle: NativeHandle) -> Option<gtk4::ComboBoxText> {
    widget_of(handle)?.downcast::<gtk4::ComboBoxText>().ok()
}

/// Il modello (`GtkListStore`) alla base del combo: fonte unica per
/// conteggio e testo delle righe.
#[allow(deprecated)]
fn combo_model(handle: NativeHandle) -> Option<gtk4::TreeModel> {
    combo_box_of(handle)?.model()
}

/// Crea un combo box (`wxComboBox`); `editable = true` equivale a
/// `CBS_DROPDOWN` (campo digitabile), `false` a `CBS_DROPDOWNLIST`.
/// Notifica `command_id` alla frame quando la selezione cambia.
#[allow(deprecated)]
pub fn create_combo_box(
    parent: NativeHandle,
    command_id: u16,
    editable: bool,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let combo = if editable {
        gtk4::ComboBoxText::with_entry()
    } else {
        gtk4::ComboBoxText::new()
    };
    let target = resolve_frame_handle(parent);
    combo.connect_changed(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &combo.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Aggiunge un elemento in coda.
#[allow(deprecated)]
pub fn combo_box_append(handle: NativeHandle, item: &str) {
    if let Some(combo) = combo_box_of(handle) {
        combo.append_text(item);
    }
}

/// Inserisce un elemento alla posizione indicata (append se fuori range).
#[allow(deprecated)]
pub fn combo_box_insert(handle: NativeHandle, index: usize, item: &str) {
    if let Some(combo) = combo_box_of(handle) {
        let position = index.min(combo_box_get_count(handle)) as i32;
        combo.insert_text(position, item);
    }
}

/// Rimuove l'elemento alla posizione indicata (no-op se fuori range).
///
/// Rimuovere l'elemento attivo emette `changed` in modo sincrono:
/// qui non è vivo alcun `RefCell::borrow`, quindi la re-entrancy è sicura.
#[allow(deprecated)]
pub fn combo_box_remove(handle: NativeHandle, index: usize) {
    if index >= combo_box_get_count(handle) {
        return;
    }
    if let Some(combo) = combo_box_of(handle) {
        combo.remove(index as i32);
    }
}

/// Rimuove tutti gli elementi.
#[allow(deprecated)]
pub fn combo_box_clear(handle: NativeHandle) {
    if let Some(combo) = combo_box_of(handle) {
        combo.remove_all();
    }
}

/// Numero di elementi del combo.
#[allow(deprecated)]
pub fn combo_box_get_count(handle: NativeHandle) -> usize {
    combo_model(handle)
        .map(|model| model.iter_n_children(None).max(0) as usize)
        .unwrap_or(0)
}

/// Testo dell'elemento alla posizione indicata (`None` se fuori range).
#[allow(deprecated)]
pub fn combo_box_get_string(handle: NativeHandle, index: usize) -> Option<String> {
    let model = combo_model(handle)?;
    if index >= model.iter_n_children(None).max(0) as usize {
        return None;
    }
    let iter = model.iter_nth_child(None, index.min(i32::MAX as usize) as i32)?;
    model.get_value(&iter, 0).get::<String>().ok()
}

/// Indice dell'elemento selezionato (`None` se nessuna selezione).
#[allow(deprecated)]
pub fn combo_box_get_selection(handle: NativeHandle) -> Option<usize> {
    let combo = combo_box_of(handle)?;
    let active = combo.property::<i32>("active");
    if active < 0 {
        None
    } else {
        Some(active as usize)
    }
}

/// Seleziona l'elemento alla posizione indicata.
#[allow(deprecated)]
pub fn combo_box_set_selection(handle: NativeHandle, index: usize) {
    if index >= combo_box_get_count(handle) {
        return;
    }
    if let Some(combo) = combo_box_of(handle) {
        // Il segnale `changed` parte in sincrono durante la chiamata:
        // handle e conteggio sono già estratti, nessun borrow è vivo.
        combo.set_property("active", index as i32);
    }
}

/// Testo del campo di testo (combo editabile) oppure, per il combo a
/// sola selezione, il testo dell'elemento attivo (vuoto se nessuno).
#[allow(deprecated)]
pub fn combo_box_get_value(handle: NativeHandle) -> String {
    let Some(combo) = combo_box_of(handle) else {
        return String::new();
    };
    combo
        .active_text()
        .map(|text| text.to_string())
        .unwrap_or_default()
}

/// Imposta il testo del campo di testo; per il combo a sola selezione
/// seleziona l'elemento il cui testo coincide.
#[allow(deprecated)]
pub fn combo_box_set_value(handle: NativeHandle, text: &str) {
    let Some(combo) = combo_box_of(handle) else {
        return;
    };
    if let Some(entry) = combo
        .child()
        .and_then(|child| child.downcast::<Entry>().ok())
    {
        entry.set_text(text);
        return;
    }
    let found = (0..combo_box_get_count(handle))
        .find(|&index| combo_box_get_string(handle, index).as_deref() == Some(text));
    if let Some(index) = found {
        combo.set_property("active", index as i32);
    }
}

// ── ListBox / Choice ────────────────────────────────────────────────

/// Il `gtk::ListBox` registrato per l'handle.
fn list_box_of(handle: NativeHandle) -> Option<gtk4::ListBox> {
    widget_of(handle)?.downcast::<gtk4::ListBox>().ok()
}

/// Il `gtk::DropDown` registrato per l'handle.
fn choice_of(handle: NativeHandle) -> Option<gtk4::DropDown> {
    widget_of(handle)?.downcast::<gtk4::DropDown>().ok()
}

/// Il modello `StringList` dietro un `Choice`.
fn choice_model(handle: NativeHandle) -> Option<gtk4::StringList> {
    choice_of(handle)?
        .model()?
        .downcast::<gtk4::StringList>()
        .ok()
}

/// Riga testuale di un `gtk::ListBox`.
fn list_row(text: &str) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_child(Some(&Label::new(Some(text))));
    row
}

/// Crea un list box (`wxListBox`) a selezione singola o multipla che
/// notifica `command_id` alla frame quando la selezione cambia.
pub fn create_list_box(
    parent: NativeHandle,
    multi_select: bool,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let list = gtk4::ListBox::new();
    list.set_selection_mode(if multi_select {
        gtk4::SelectionMode::Multiple
    } else {
        gtk4::SelectionMode::Single
    });
    let target = resolve_frame_handle(parent);
    list.connect_selected_rows_changed(move |_| dispatch_command(target, command_id));
    attach(parent, handle, &list.upcast::<gtk4::Widget>(), initial_rect);
    handle
}

/// Aggiunge un elemento in coda al list box.
pub fn list_box_append(handle: NativeHandle, item: &str) {
    if let Some(list) = list_box_of(handle) {
        list.append(&list_row(item));
    }
}

/// Inserisce un elemento alla posizione indicata (append se fuori range).
pub fn list_box_insert(handle: NativeHandle, index: usize, item: &str) {
    if let Some(list) = list_box_of(handle) {
        let position = index.min(i32::MAX as usize) as i32;
        if position >= list_box_count(&list) as i32 {
            list.append(&list_row(item));
        } else {
            list.insert(&list_row(item), position);
        }
    }
}

/// Conta le righe dirette di un `gtk::ListBox`.
fn list_box_count(list: &gtk4::ListBox) -> usize {
    let mut count = 0usize;
    let mut child = list.first_child();
    while let Some(widget) = child {
        count += 1;
        child = widget.next_sibling();
    }
    count
}

/// Rimuove l'elemento alla posizione indicata (no-op se fuori range).
pub fn list_box_remove(handle: NativeHandle, index: usize) {
    if let Some(list) = list_box_of(handle) {
        if let Some(row) = list.row_at_index(index.min(i32::MAX as usize) as i32) {
            list.remove(&row);
        }
    }
}

/// Rimuove tutti gli elementi.
pub fn list_box_clear(handle: NativeHandle) {
    if let Some(list) = list_box_of(handle) {
        // `ListBox::remove_all` richiede la feature `v4_12`: con `v4_10`
        // si svuota la lista rimuovendo una riga alla volta.
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
    }
}

/// Numero di elementi del list box.
pub fn list_box_get_count(handle: NativeHandle) -> usize {
    list_box_of(handle)
        .map(|list| list_box_count(&list))
        .unwrap_or(0)
}

/// Testo dell'elemento alla posizione indicata (`None` se fuori range).
pub fn list_box_get_string(handle: NativeHandle, index: usize) -> Option<String> {
    let list = list_box_of(handle)?;
    let row = list.row_at_index(index.min(i32::MAX as usize) as i32)?;
    let label = row.child()?.downcast::<Label>().ok()?;
    Some(label.text().to_string())
}

/// Indice dell'elemento selezionato (`None` se nessuna selezione).
pub fn list_box_get_selection(handle: NativeHandle) -> Option<usize> {
    let row = list_box_of(handle)?.selected_row()?;
    Some(row.index().max(0) as usize)
}

/// Indici di tutti gli elementi selezionati (multi-selezione).
pub fn list_box_get_selections(handle: NativeHandle) -> Vec<usize> {
    match list_box_of(handle) {
        Some(list) => list
            .selected_rows()
            .iter()
            .map(|row| row.index().max(0) as usize)
            .collect(),
        None => Vec::new(),
    }
}

/// Seleziona l'elemento alla posizione indicata.
pub fn list_box_set_selection(handle: NativeHandle, index: usize) {
    if let Some(list) = list_box_of(handle) {
        if let Some(row) = list.row_at_index(index.min(i32::MAX as usize) as i32) {
            list.select_row(Some(&row));
        }
    }
}

/// Crea un `Choice` (`wxChoice`) con modello stringhe vuoto; notifica
/// `command_id` alla frame quando la selezione cambia.
pub fn create_choice(parent: NativeHandle, command_id: u16, initial_rect: Rect) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let model = gtk4::StringList::new(&[]);
    let drop_down = gtk4::DropDown::new(Some(model), None::<gtk4::Expression>);
    let target = resolve_frame_handle(parent);
    drop_down.connect_selected_notify(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &drop_down.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Aggiunge un elemento in coda al modello del `Choice`.
pub fn choice_append(handle: NativeHandle, item: &str) {
    if let Some(model) = choice_model(handle) {
        model.append(item);
    }
}

/// Inserisce un elemento alla posizione indicata (append se fuori range).
pub fn choice_insert(handle: NativeHandle, index: usize, item: &str) {
    if let Some(model) = choice_model(handle) {
        let position = index.min(model.n_items() as usize) as u32;
        model.splice(position, 0, &[item]);
    }
}

/// Rimuove l'elemento alla posizione indicata (no-op se fuori range).
pub fn choice_remove(handle: NativeHandle, index: usize) {
    if let Some(model) = choice_model(handle) {
        if (index as u64) < model.n_items() as u64 {
            model.remove(index as u32);
        }
    }
}

/// Rimuove tutti gli elementi del `Choice`.
pub fn choice_clear(handle: NativeHandle) {
    if let Some(model) = choice_model(handle) {
        model.splice(0, model.n_items(), &[]);
    }
}

/// Numero di elementi del `Choice`.
pub fn choice_get_count(handle: NativeHandle) -> usize {
    choice_model(handle)
        .map(|model| model.n_items() as usize)
        .unwrap_or(0)
}

/// Testo dell'elemento alla posizione indicata (`None` se fuori range).
pub fn choice_get_string(handle: NativeHandle, index: usize) -> Option<String> {
    choice_model(handle)?
        .string(index.min(u32::MAX as usize) as u32)
        .map(|text| text.to_string())
}

/// Indice dell'elemento selezionato (`None` = posizione invalida).
pub fn choice_get_selection(handle: NativeHandle) -> Option<usize> {
    let selected = choice_of(handle)?.selected();
    if selected == gtk4::INVALID_LIST_POSITION {
        None
    } else {
        Some(selected as usize)
    }
}

/// Seleziona l'elemento alla posizione indicata.
pub fn choice_set_selection(handle: NativeHandle, index: usize) {
    if let Some(drop_down) = choice_of(handle) {
        if (index as u64) < choice_get_count(handle) as u64 {
            drop_down.set_selected(index.min(u32::MAX as usize) as u32);
        }
    }
}

// ── Gauge (ProgressBar) ─────────────────────────────────────────────

thread_local! {
    /// Handle gauge → timer dell'animazione marquee (modo indeterminato).
    static GAUGE_TIMERS: RefCell<HashMap<NativeHandle, glib::SourceId>> =
        RefCell::new(HashMap::new());
}

/// Il `gtk::ProgressBar` registrato per l'handle.
fn gauge_of(handle: NativeHandle) -> Option<gtk4::ProgressBar> {
    widget_of(handle)?.downcast::<gtk4::ProgressBar>().ok()
}

/// Crea una progress bar; se `indeterminate` avvia subito il marquee.
pub fn create_gauge(
    parent: NativeHandle,
    initial_rect: Rect,
    vertical: bool,
    indeterminate: bool,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let bar = gtk4::ProgressBar::new();
    if vertical {
        bar.set_orientation(gtk4::Orientation::Vertical);
    }
    if indeterminate {
        bar.set_pulse_step(0.08);
    }
    attach(parent, handle, &bar.upcast::<gtk4::Widget>(), initial_rect);
    if indeterminate {
        gauge_pulse(handle);
    }
    handle
}

/// Porta la barra a `value / range` (frazione clampata in `[0, 1]`).
///
/// GTK non ha il concetto di range: la posizione è sempre una frazione,
/// quindi il valore va convertito qui. `range <= 0` equivale a barra vuota.
pub fn gauge_set_state(handle: NativeHandle, value: i32, range: i32) {
    let fraction = if range > 0 {
        (value as f64 / range as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if let Some(bar) = gauge_of(handle) {
        bar.set_fraction(fraction);
    }
}

/// Posizione corrente della barra, riconvertita in valore intero.
pub fn gauge_get_value(handle: NativeHandle, range: i32) -> i32 {
    gauge_of(handle)
        .map(|bar| (bar.fraction() * range as f64).round() as i32)
        .unwrap_or(0)
}

/// Avvia (o prosegue) l'animazione marquee dell'indeterminate bar.
///
/// GTK4 non ha la marquee nativa di Win32: il `pulse()` va ripetuto con
/// un timer sul main loop. Un secondo `pulse()` sullo stesso handle è
/// ignorato, così il timer resta singolo.
pub fn gauge_pulse(handle: NativeHandle) {
    if !ensure_ready() {
        return;
    }
    if gauge_of(handle).is_none() {
        return;
    }
    let running = GAUGE_TIMERS.with(|map| map.borrow().contains_key(&handle));
    if running {
        return;
    }
    let id = glib::timeout_add_local(std::time::Duration::from_millis(40), move || {
        let Some(bar) = gauge_of(handle) else {
            // Widget distrutto: il timer si spegne da solo.
            GAUGE_TIMERS.with(|map| map.borrow_mut().remove(&handle));
            return glib::ControlFlow::Break;
        };
        bar.pulse();
        glib::ControlFlow::Continue
    });
    GAUGE_TIMERS.with(|map| map.borrow_mut().insert(handle, id));
}

/// Ferma l'animazione marquee e riporta la barra a zero.
pub fn gauge_stop_pulse(handle: NativeHandle) {
    if let Some(id) = GAUGE_TIMERS.with(|map| map.borrow_mut().remove(&handle)) {
        id.remove();
    }
    if let Some(bar) = gauge_of(handle) {
        bar.set_fraction(0.0);
    }
}

// ── Slider (Scale) ─────────────────────────────────────────────────

/// Il `gtk::Scale` registrato per l'handle.
fn scale_of(handle: NativeHandle) -> Option<gtk4::Scale> {
    widget_of(handle)?.downcast::<gtk4::Scale>().ok()
}

/// Crea uno slider che, al cambio di valore, invia `command_id` alla frame.
pub fn create_slider(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    min: i32,
    max: i32,
    initial: i32,
    vertical: bool,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let orientation = if vertical {
        gtk4::Orientation::Vertical
    } else {
        gtk4::Orientation::Horizontal
    };
    let scale = gtk4::Scale::with_range(orientation, min as f64, max as f64, 1.0);
    scale.set_digits(0);
    // wxSlider non mostra la "bubble" del valore: solo la ghiera.
    scale.set_draw_value(false);
    // Imposta PRIMA del connect: così la creazione non genera eventi.
    scale.set_value(initial as f64);
    let target = resolve_frame_handle(parent);
    scale.connect_value_changed(move |_| dispatch_command(target, command_id));
    attach(
        parent,
        handle,
        &scale.upcast::<gtk4::Widget>(),
        initial_rect,
    );
    handle
}

/// Aggiorna il range dello slider (GTK ricalcola internamente il valore).
pub fn slider_set_range(handle: NativeHandle, min: i32, max: i32) {
    if let Some(scale) = scale_of(handle) {
        scale.set_range(min as f64, max as f64);
    }
}

/// Imposta il valore corrente dello slider.
pub fn slider_set_value(handle: NativeHandle, value: i32) {
    if let Some(scale) = scale_of(handle) {
        scale.set_value(value as f64);
    }
}

/// Valore corrente letto direttamente dal widget.
pub fn slider_get_value(handle: NativeHandle) -> i32 {
    scale_of(handle)
        .map(|scale| scale.value().round() as i32)
        .unwrap_or(0)
}

/// Tacche intermedie ogni `freq` unità (equivalente di `TBS_AUTOTICKS`).
///
/// GTK4 non ha un "tick frequency": le tacche sono mark singoli che qui
/// vengono ricostruiti da zero sul range corrente del widget.
pub fn slider_set_tick_freq(handle: NativeHandle, freq: i32) {
    let Some(scale) = scale_of(handle) else {
        return;
    };
    scale.clear_marks();
    if freq <= 0 {
        return;
    }
    let position = if scale.orientation() == gtk4::Orientation::Vertical {
        gtk4::PositionType::Left
    } else {
        gtk4::PositionType::Bottom
    };
    let adjustment = scale.adjustment();
    let lower = adjustment.lower();
    let upper = adjustment.upper();
    let step = freq as f64;
    let mut v = (lower / step).ceil() * step;
    while v <= upper {
        if v >= lower {
            scale.add_mark(v, position, None);
        }
        v += step;
    }
}

/// Passi dei tasti frecce (`line`) e PagSu/PagGiù (`page`).
pub fn slider_set_increments(handle: NativeHandle, line: i32, page: i32) {
    if let Some(scale) = scale_of(handle) {
        scale.set_increments(line as f64, page as f64);
    }
}

// ── SpinCtrl (SpinButton) ───────────────────────────────────────────

/// Il `gtk::SpinButton` registrato per l'handle.
fn spin_button_of(handle: NativeHandle) -> Option<gtk4::SpinButton> {
    widget_of(handle)?.downcast::<gtk4::SpinButton>().ok()
}

/// Crea uno stepper numerico che, al cambio di valore, invia
/// `command_id` alla frame.
pub fn create_spin_ctrl(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    min: i32,
    max: i32,
    initial: i32,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let spin = gtk4::SpinButton::with_range(min as f64, max as f64, 1.0);
    // Valori interi (wxSpinCtrl non ha cifre decimali).
    spin.set_digits(0);
    spin.set_numeric(true);
    spin.set_increments(1.0, 10.0);
    // Imposta PRIMA del connect: così la creazione non genera eventi.
    spin.set_value(initial.clamp(min, max) as f64);
    let target = resolve_frame_handle(parent);
    spin.connect_value_changed(move |_| dispatch_command(target, command_id));
    attach(parent, handle, &spin.upcast::<gtk4::Widget>(), initial_rect);
    handle
}

/// Aggiorna il range dello stepper (GTK ricalcola internamente il valore).
pub fn spin_set_range(handle: NativeHandle, min: i32, max: i32) {
    if let Some(spin) = spin_button_of(handle) {
        spin.set_range(min as f64, max as f64);
    }
}

/// Imposta il valore corrente dello stepper.
pub fn spin_set_value(handle: NativeHandle, value: i32) {
    if let Some(spin) = spin_button_of(handle) {
        spin.set_value(value as f64);
    }
}

/// Valore corrente letto direttamente dal widget.
pub fn spin_get_value(handle: NativeHandle) -> i32 {
    spin_button_of(handle)
        .map(|spin| spin.value().round() as i32)
        .unwrap_or(0)
}

// ── Widget: ListCtrl (TreeView + ListStore) ─────────────────────────

/// Codice di notifica allineato con il backend macOS: la selezione
/// (o il contenuto) di un elemento è cambiato.
pub const LISTCTRL_NOTIFY_ITEM_CHANGED: u32 = 0xFFFFFF9B;

/// Colonne testuali disponibili nel modello sottostante: le colonne
/// del report view si agganciano alle colonne `0..N` del `ListStore`.
const LISTCTRL_MODEL_COLUMNS: usize = 64;

/// Massimo numero di righe materializzate nel `ListStore` da
/// [`listctrl_set_virtual_count`]: oltre questo limite il modello
/// reallocherebbe decine di migliaia di righe ad ogni cambio di
/// conteggio (il round-trip `set_item_count` / `get_item_count`
/// resta comunque consistente perché la frame traccia `is_virtual`).
const LISTCTRL_MAX_MODEL_ROWS: u32 = 100_000;

fn tree_view_of(handle: NativeHandle) -> Option<gtk4::TreeView> {
    widget_of(handle)?.downcast::<gtk4::TreeView>().ok()
}

/// `ListStore` testuale sottostante a un ListCtrl.
fn list_store_of(handle: NativeHandle) -> Option<gtk4::ListStore> {
    tree_view_of(handle)?
        .model()
        .and_then(|model| model.downcast::<gtk4::ListStore>().ok())
}

/// Crea una lista in report view (TreeView + ListStore, selezione
/// multipla). Il cambio di selezione invia
/// `LISTCTRL_NOTIFY_ITEM_CHANGED` alla frame; il doppio click su una
/// riga invia `dispatch_lv_item_activate` con l'indice di riga.
#[allow(deprecated)]
pub fn create_list_ctrl(parent: NativeHandle, command_id: u16, initial_rect: Rect) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let store = gtk4::ListStore::new(&[glib::Type::STRING; LISTCTRL_MODEL_COLUMNS]);
    let view = gtk4::TreeView::with_model(&store);
    view.set_headers_visible(true);
    let selection = view.selection();
    selection.set_mode(gtk4::SelectionMode::Multiple);
    // Risolvi la frame PRIMA dell'attach: le closure catturano handle
    // e id per valore (Copy), così il dispatch non ha bisogno di
    // nessun registro condiviso durante i segnali sincroni.
    let target = resolve_frame_handle(parent);
    selection.connect_changed(move |_| {
        dispatch_notify(target, command_id, LISTCTRL_NOTIFY_ITEM_CHANGED);
    });
    view.connect_row_activated(move |_view, path, _column| {
        let row = path.indices().first().copied().unwrap_or(0);
        dispatch_lv_item_activate(target, command_id, row as isize);
    });
    attach(parent, handle, &view.upcast::<gtk4::Widget>(), initial_rect);
    // Su GTK il `ListCtrl` è un `GtkTreeView`, che `role_of` normalizza
    // come `tree`: il registro `declared` lo espone come `table`, come su
    // macOS/Windows (`RuWxListTableView` / `SysListView32`), così
    // `/find?role=table` trova le tabelle (§19.6 di PIANO_DEBUG_INSPECTOR.md).
    crate::platform::declared::mark(handle, "table");
    handle
}

/// Inoltra il codice di notifica `code` all'hub `notify_handlers`
/// registrato dalla frame per `control_id` (remove → invoca →
/// reinsert, stesso schema di [`dispatch_command`]).
pub fn dispatch_notify(frame_handle: NativeHandle, control_id: u16, code: u32) {
    let frame_handle = resolve_frame_handle(frame_handle);
    if let Some(data) = frame_data(frame_handle) {
        let handler = data.borrow_mut().notify_handlers.remove(&control_id);
        if let Some(mut handler) = handler {
            handler(code);
            data.borrow_mut()
                .notify_handlers
                .insert(control_id, handler);
        }
    }
}

/// Inoltra l'attivazione della riga `row` (doppio click) all'hub
/// `lv_item_activate_handlers` registrato dalla frame per
/// `control_id`.
pub fn dispatch_lv_item_activate(frame_handle: NativeHandle, control_id: u16, row: isize) {
    let frame_handle = resolve_frame_handle(frame_handle);
    if let Some(data) = frame_data(frame_handle) {
        let handler = data
            .borrow_mut()
            .lv_item_activate_handlers
            .remove(&control_id);
        if let Some(mut handler) = handler {
            handler(row);
            data.borrow_mut()
                .lv_item_activate_handlers
                .insert(control_id, handler);
        }
    }
}

/// Aggiunge una colonna di report view; il renderer di testo si
/// aggancia alla colonna `index` del modello.
pub fn listctrl_insert_column(handle: NativeHandle, index: u32, title: &str, width: i32) {
    if index as usize >= LISTCTRL_MODEL_COLUMNS {
        return;
    }
    let Some(view) = tree_view_of(handle) else {
        return;
    };
    let column = gtk4::TreeViewColumn::new();
    column.set_title(title);
    column.set_fixed_width(width.max(0));
    let renderer = gtk4::CellRendererText::new();
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", index as i32);
    view.insert_column(&column, index as i32);
}

/// Titolo e larghezza delle colonne di report view, in ordine visibile:
/// alimenta `extra.columns` dell'inspector (§6.4, pendant Linux di
/// `LVM_GETCOLUMNW` su Windows).
pub fn listctrl_column_info(handle: NativeHandle) -> Vec<(String, i32)> {
    let Some(view) = tree_view_of(handle) else {
        return Vec::new();
    };
    view.columns()
        .iter()
        .map(|column| {
            // `width()` è la larghezza corrente; finché la vista non è
            // ridimensionata resta -1 e si cade sulla larghezza fissa.
            let current = column.width();
            let width = if current >= 0 {
                current
            } else {
                column.fixed_width()
            };
            let title = column.title().to_string();
            (title, width)
        })
        .collect()
}

/// Inserisce una riga con il testo della prima colonna. Un `index`
/// oltre la fine appende in coda (come su macOS/Windows). L'eventuale
/// `image_index` non ha una controparte nel modello testuale.
pub fn listctrl_insert_item(
    handle: NativeHandle,
    index: usize,
    text: &str,
    _image_index: Option<i32>,
) -> usize {
    let Some(store) = list_store_of(handle) else {
        return 0;
    };
    let count = store.iter_n_children(None).max(0) as usize;
    let position = index.min(count);
    store.insert_with_values(
        Some(position as u32),
        &[(0, &text as &dyn glib::value::ToValue)],
    );
    position
}

/// Scrive il testo della cella `(riga, colonna)`.
pub fn listctrl_set_item_text(handle: NativeHandle, item_index: usize, column: usize, text: &str) {
    if column >= LISTCTRL_MODEL_COLUMNS {
        return;
    }
    let Some(store) = list_store_of(handle) else {
        return;
    };
    let Some(iter) = store.iter_nth_child(None, item_index as i32) else {
        return;
    };
    store.set(
        &iter,
        &[(column as u32, &text as &dyn glib::value::ToValue)],
    );
}

/// Legge il testo della cella `(riga, colonna)`; stringa vuota se la
/// cella o la riga non esistono.
pub fn listctrl_get_item_text(handle: NativeHandle, item_index: usize, column: usize) -> String {
    if column >= LISTCTRL_MODEL_COLUMNS {
        return String::new();
    }
    let Some(store) = list_store_of(handle) else {
        return String::new();
    };
    let Some(iter) = store.iter_nth_child(None, item_index as i32) else {
        return String::new();
    };
    store
        .get_value(&iter, column as i32)
        .get::<String>()
        .unwrap_or_default()
}

/// Numero di righe materializzate nel modello.
pub fn listctrl_get_item_count(handle: NativeHandle) -> usize {
    list_store_of(handle)
        .map(|store| store.iter_n_children(None).max(0) as usize)
        .unwrap_or(0)
}

/// Elimina la riga `index`; fuori intervallo è un no-op.
pub fn listctrl_delete_item(handle: NativeHandle, index: usize) {
    let Some(store) = list_store_of(handle) else {
        return;
    };
    if let Some(iter) = store.iter_nth_child(None, index as i32) {
        let _ = store.remove(&iter);
    }
}

/// Elimina tutte le righe.
pub fn listctrl_delete_all_items(handle: NativeHandle) {
    if let Some(store) = list_store_of(handle) {
        store.clear();
    }
}

/// Prima riga selezionata, oppure `None`.
pub fn listctrl_get_selected_item(handle: NativeHandle) -> Option<usize> {
    let view = tree_view_of(handle)?;
    let (paths, _model) = view.selection().selected_rows();
    let first = paths.first()?;
    first.indices().first().map(|i| *i as usize)
}

/// Tutte le righe selezionate, in ordine crescente.
pub fn listctrl_get_selected_items(handle: NativeHandle) -> Vec<usize> {
    let Some(view) = tree_view_of(handle) else {
        return Vec::new();
    };
    let (paths, _model) = view.selection().selected_rows();
    let mut rows: Vec<usize> = paths
        .iter()
        .filter_map(|path| path.indices().first().map(|i| *i as usize))
        .collect();
    rows.sort_unstable();
    rows
}

/// Seleziona la riga `index`.
pub fn listctrl_select(handle: NativeHandle, index: usize) {
    if let Some(view) = tree_view_of(handle) {
        let path = gtk4::TreePath::from_indices(&[index as i32]);
        view.selection().select_path(&path);
    }
}

/// Deseleziona la riga `index`.
pub fn listctrl_deselect(handle: NativeHandle, index: usize) {
    if let Some(view) = tree_view_of(handle) {
        let path = gtk4::TreePath::from_indices(&[index as i32]);
        view.selection().unselect_path(&path);
    }
}

/// Rimuove la selezione da tutte le righe.
pub fn listctrl_clear_selection(handle: NativeHandle) {
    if let Some(view) = tree_view_of(handle) {
        view.selection().unselect_all();
    }
}

/// Seleziona tutte le righe.
pub fn listctrl_select_all(handle: NativeHandle) {
    if let Some(view) = tree_view_of(handle) {
        view.selection().select_all();
    }
}

/// La riga `index` è selezionata?
pub fn listctrl_is_selected(handle: NativeHandle, index: usize) -> bool {
    tree_view_of(handle)
        .map(|view| {
            let path = gtk4::TreePath::from_indices(&[index as i32]);
            view.selection().path_is_selected(&path)
        })
        .unwrap_or(false)
}

/// Invalida l'intero widget: GTK non ha un ridisegno di righe
/// parziale, quindi `redraw_items` (usato in modalità virtuale) fa
/// un `queue_draw` completo.
pub fn listctrl_redraw_items(handle: NativeHandle) {
    if let Some(view) = tree_view_of(handle) {
        view.queue_draw();
    }
}

/// Allinea il modello a `count` righe (righe vuote, colonne testuali).
/// Usato dal passaggio in modalità virtuale per rendere consistente il
/// round-trip `set_item_count` / `get_item_count`.
pub fn listctrl_set_virtual_count(handle: NativeHandle, count: u32) {
    let Some(store) = list_store_of(handle) else {
        return;
    };
    let current = store.iter_n_children(None).max(0) as u32;
    if count == current {
        return;
    }
    if count > current {
        if count > LISTCTRL_MAX_MODEL_ROWS {
            return;
        }
        for _ in current..count {
            store.insert_with_values(None, &[(0, &"" as &dyn glib::value::ToValue)]);
        }
    } else {
        for _ in count..current {
            match store.iter_nth_child(None, count as i32) {
                Some(iter) => {
                    if !store.remove(&iter) {
                        break;
                    }
                }
                None => break,
            }
        }
    }
}

// ── Widget: Grid (TreeView + ListStore) ─────────────────────────────
//
// Pendant Linux del backend AppKit (`appkit_grid_table.rs`): lo stesso
// widget nativo del ListCtrl ma con la semantica del `Grid` — colonne
// con allineamento, selezione multipla, notifica di selezione,
// attivazione riga (doppio click) e click sull'intestazione.

/// La selezione è cambiata: stesso valore di `GRID_NOTIFY_ITEM_CHANGED`
/// del backend macOS (`appkit_grid_table.rs`).
pub const GRID_NOTIFY_ITEM_CHANGED: u32 = LISTCTRL_NOTIFY_ITEM_CHANGED;
/// Doppio click / Invio su una riga: la coppia `(riga, colonna)` è
/// pendente in [`grid_take_activate`].
pub const GRID_NOTIFY_ACTIVATE: u32 = 0x4752_0001;
/// Click sull'intestazione di colonna: la colonna è pendente in
/// [`grid_take_header_click`].
pub const GRID_NOTIFY_HEADER_CLICK: u32 = 0x4752_0002;

thread_local! {
    /// Contesto di dispatch `(frame, id)` per ciascun Grid creato:
    /// le closure delle intestazioni lo leggono per inoltrare gli eventi.
    static GRID_CTX: RefCell<HashMap<NativeHandle, (NativeHandle, u16)>> =
        RefCell::new(HashMap::new());
    /// Attivazione pendente `(riga, colonna)` per handle.
    static GRID_PENDING_ACTIVATE: RefCell<HashMap<NativeHandle, (usize, usize)>> =
        RefCell::new(HashMap::new());
    /// Click su intestazione pendente per handle.
    static GRID_PENDING_HEADER: RefCell<HashMap<NativeHandle, usize>> =
        RefCell::new(HashMap::new());
    /// Seleziona programmatica in corso: sopprime la notifica di
    /// selezione sintetica così i callback non scattano due volte.
    static GRID_SELECTING: Cell<bool> = Cell::new(false);
}

/// Crea un Grid (TreeView + ListStore, selezione multipla). Il cambio
/// di selezione invia `GRID_NOTIFY_ITEM_CHANGED` alla frame; il doppio
/// click su una riga invia `GRID_NOTIFY_ACTIVATE` con la coppia
/// `(riga, colonna)` pendente.
#[allow(deprecated)]
pub fn create_grid_ctrl(parent: NativeHandle, command_id: u16, initial_rect: Rect) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let store = gtk4::ListStore::new(&[glib::Type::STRING; LISTCTRL_MODEL_COLUMNS]);
    let view = gtk4::TreeView::with_model(&store);
    view.set_headers_visible(true);
    let selection = view.selection();
    selection.set_mode(gtk4::SelectionMode::Multiple);
    // Risolvi la frame PRIMA dell'attach: le closure catturano handle
    // e id per valore (Copy), così il dispatch non ha bisogno di
    // nessun registro condiviso durante i segnali sincroni.
    let target = resolve_frame_handle(parent);
    GRID_CTX.with(|c| c.borrow_mut().insert(handle, (target, command_id)));
    selection.connect_changed(move |_| {
        // La selezione programmatica di `grid_select_row` gestisce i
        // callback in-memory: qui passano solo i click dell'utente.
        if GRID_SELECTING.with(|s| s.get()) {
            return;
        }
        dispatch_notify(target, command_id, GRID_NOTIFY_ITEM_CHANGED);
    });
    view.connect_row_activated(move |view, path, column| {
        let row = path.indices().first().copied().unwrap_or(0) as usize;
        // gtk4 non espone `get_index` su TreeViewColumn: la posizione
        // si ricava confrontando l'oggetto con l'elenco delle colonne.
        let col = column
            .map(|c| view.columns().iter().position(|x| x == c).unwrap_or(0))
            .unwrap_or(0);
        GRID_PENDING_ACTIVATE.with(|p| p.borrow_mut().insert(handle, (row, col)));
        dispatch_notify(target, command_id, GRID_NOTIFY_ACTIVATE);
    });
    attach(parent, handle, &view.upcast::<gtk4::Widget>(), initial_rect);
    // Come il ListCtrl: esposto come `table` dall'inspector (§19.6 di
    // PIANO_DEBUG_INSPECTOR.md).
    crate::platform::declared::mark(handle, "table");
    handle
}

/// Attivazione pendente `(riga, colonna)` dell'ultimo doppio click.
pub fn grid_take_activate(handle: NativeHandle) -> Option<(usize, usize)> {
    GRID_PENDING_ACTIVATE.with(|p| p.borrow_mut().remove(&handle))
}

/// Intestazione cliccata pendente (indice di colonna).
pub fn grid_take_header_click(handle: NativeHandle) -> Option<usize> {
    GRID_PENDING_HEADER.with(|p| p.borrow_mut().remove(&handle))
}

/// Aggiunge una colonna del Grid: titolo, larghezza fissa e
/// allineamento (`0` sinistra, `1` destra, `2` centro — stesso codice
/// di `ColumnAlign::as_grid_align`). Il click sull'intestazione
/// inoltra `GRID_NOTIFY_HEADER_CLICK` con la colonna pendente.
pub fn grid_append_column(handle: NativeHandle, title: &str, width: i32, align: i32) {
    let Some(view) = tree_view_of(handle) else {
        return;
    };
    // La colonna viene appenduta in coda: la sua posizione è anche la
    // colonna del `ListStore` a cui il renderer di testo si aggancia.
    let index = view.columns().len() as i32;
    if index as usize >= LISTCTRL_MODEL_COLUMNS {
        return;
    }
    let xalign: f32 = match align {
        1 => 1.0,
        2 => 0.5,
        _ => 0.0,
    };
    let column = gtk4::TreeViewColumn::new();
    column.set_title(title);
    column.set_fixed_width(width.max(0));
    column.set_alignment(xalign);
    let renderer = gtk4::CellRendererText::new();
    renderer.set_xalign(xalign);
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", index);
    if let Some((target, id)) = GRID_CTX.with(|c| c.borrow().get(&handle).copied()) {
        let col = index as usize;
        column.connect_clicked(move |_| {
            GRID_PENDING_HEADER.with(|p| p.borrow_mut().insert(handle, col));
            dispatch_notify(target, id, GRID_NOTIFY_HEADER_CLICK);
        });
    }
    view.insert_column(&column, index);
}

/// Rimuove la colonna `index`; `false` se la colonna non esiste.
pub fn grid_remove_column(handle: NativeHandle, index: usize) -> bool {
    let Some(view) = tree_view_of(handle) else {
        return false;
    };
    match view.column(index as i32) {
        Some(column) => {
            view.remove_column(&column);
            true
        }
        None => false,
    }
}

/// Allinea il modello a `count` righe (righe vuote, colonne testuali).
pub fn grid_set_row_count(handle: NativeHandle, count: usize) {
    listctrl_set_virtual_count(handle, count as u32);
}

/// Numero di righe materializzate nel modello.
pub fn grid_native_row_count(handle: NativeHandle) -> usize {
    listctrl_get_item_count(handle)
}

/// Aggiunge una riga vuota in coda.
pub fn grid_append_row(handle: NativeHandle) {
    if let Some(store) = list_store_of(handle) {
        store.insert_with_values(None, &[(0, &"" as &dyn glib::value::ToValue)]);
    }
}

/// Elimina la riga `row` (indice di visualizzazione); fuori
/// intervallo è un no-op.
pub fn grid_delete_row(handle: NativeHandle, row: usize) {
    listctrl_delete_item(handle, row);
}

/// Scrive il testo della cella `(riga di visualizzazione, colonna)`.
pub fn grid_set_cell(handle: NativeHandle, display_row: usize, col: usize, text: &str) {
    listctrl_set_item_text(handle, display_row, col, text);
}

/// Larghezza della colonna `col` in pixel; `width <= 0` nasconde la
/// colonna (pendant del comportamento Win32 con larghezza 0).
pub fn grid_set_column_width(handle: NativeHandle, col: usize, width: i32) {
    let Some(view) = tree_view_of(handle) else {
        return;
    };
    let Some(column) = view.column(col as i32) else {
        return;
    };
    if width <= 0 {
        column.set_visible(false);
    } else {
        column.set_visible(true);
        column.set_fixed_width(width);
    }
}

/// Re-applica tutte le larghezze `(colonna, larghezza)` in pixel.
pub fn grid_apply_column_widths(handle: NativeHandle, widths: &[(usize, i32)]) {
    for &(col, width) in widths {
        grid_set_column_width(handle, col, width);
    }
}

/// Ridisegna l'intera vista (GTK non ha invalidazione di righe parziali).
pub fn grid_redraw_view(handle: NativeHandle) {
    listctrl_redraw_items(handle);
}

/// Seleziona `row` come unica riga selezionata e la scorre in vista.
/// La notifica di selezione viene soppressa durante l'operazione: il
/// chiamante (backend di `Grid::select_row`) gestisce i callback.
pub fn grid_select_row(handle: NativeHandle, row: usize) {
    let Some(view) = tree_view_of(handle) else {
        return;
    };
    if row >= listctrl_get_item_count(handle) {
        return;
    }
    GRID_SELECTING.with(|s| s.set(true));
    let selection = view.selection();
    selection.unselect_all();
    let path = gtk4::TreePath::from_indices(&[row as i32]);
    selection.select_path(&path);
    view.scroll_to_cell(Some(&path), None::<&gtk4::TreeViewColumn>, false, 0.0, 0.0);
    GRID_SELECTING.with(|s| s.set(false));
}

/// Scorre la riga `row` in vista senza cambiare la selezione.
pub fn grid_ensure_row_visible(handle: NativeHandle, row: usize) {
    let Some(view) = tree_view_of(handle) else {
        return;
    };
    if row >= listctrl_get_item_count(handle) {
        return;
    }
    let path = gtk4::TreePath::from_indices(&[row as i32]);
    view.scroll_to_cell(Some(&path), None::<&gtk4::TreeViewColumn>, false, 0.0, 0.0);
}

/// Prima riga selezionata, oppure `None`.
pub fn grid_get_selected_row(handle: NativeHandle) -> Option<usize> {
    listctrl_get_selected_item(handle)
}

/// Tutte le righe selezionate, in ordine crescente.
pub fn grid_get_selected_rows(handle: NativeHandle) -> Vec<usize> {
    listctrl_get_selected_items(handle)
}

// ── TreeCtrl: TreeView GTK + modello in-memory ──────────────────────
//
// Pendant Linux del backend `appkit` (`platform/appkit.rs`): la
// sorgente di verità è un modello in-memory (nodi con testo/genitore/
// figli) e la `gtk4::TreeView` è una vista che viene ricostruita ad
// ogni mutazione (`tree_redraw`). I riferimenti `TreeItem` sono id
// numerici (come su macOS), così `get_item_text` / la traversalia
// funzionano anche in ambiente headless senza widget GTK.

/// Codice di notifica di cambio selezione — pendant Linux di
/// `appkit::TREE_NOTIFY_SELCHANGED` (stesso valore, così la `Frame`
/// inoltra lo stesso codice al handler registrato dal `TreeCtrl`).
pub const TREE_NOTIFY_SELCHANGED: u32 = 0xFFFFFE6E;

struct GtkTreeNode {
    text: String,
    parent: Option<usize>,
    children: Vec<usize>,
    expanded: bool,
}

thread_local! {
    static TREE_NODES: RefCell<HashMap<NativeHandle, RefCell<HashMap<usize, GtkTreeNode>>>> =
        RefCell::new(HashMap::new());
    static TREE_ROOTS: RefCell<HashMap<NativeHandle, RefCell<Vec<usize>>>> =
        RefCell::new(HashMap::new());
    static TREE_NEXT_ID: RefCell<HashMap<NativeHandle, Cell<usize>>> =
        RefCell::new(HashMap::new());
    static TREE_SELECTED: RefCell<HashMap<NativeHandle, RefCell<Option<usize>>>> =
        RefCell::new(HashMap::new());
    /// Handle tree → (handle frame, id notifica) per il dispatch della
    /// selezione verso la `Frame`.
    static TREE_NOTIFY: RefCell<HashMap<NativeHandle, (NativeHandle, u16)>> =
        RefCell::new(HashMap::new());
    /// `true` durante la ricostruzione della vista: sopprime il dispatch
    /// di selezione che altrimenti scatterebbe su `store.clear()`.
    static TREE_REBUILDING: Cell<bool> = const { Cell::new(false) };
}

/// Crea un albero (GTK `TreeView` + `TreeStore` testuale). In ambiente
/// headless restituisce un handle stub ma inizializza comunque il
/// modello in-memory, così `get_item_text` / `get_count` restano validi.
pub fn create_tree_ctrl(parent: NativeHandle, command_id: u16, initial_rect: Rect) -> NativeHandle {
    let handle = alloc_widget_handle();
    TREE_NODES.with(|m| {
        m.borrow_mut().insert(handle, RefCell::new(HashMap::new()));
    });
    TREE_ROOTS.with(|m| {
        m.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    TREE_NEXT_ID.with(|m| {
        m.borrow_mut().insert(handle, Cell::new(1));
    });
    TREE_SELECTED.with(|m| {
        m.borrow_mut().insert(handle, RefCell::new(None));
    });

    if !ensure_ready() {
        return handle;
    }

    let store = gtk4::TreeStore::new(&[glib::Type::STRING, glib::Type::I64]);
    let view = gtk4::TreeView::with_model(&store);
    view.set_headers_visible(false);
    let column = gtk4::TreeViewColumn::new();
    let renderer = gtk4::CellRendererText::new();
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", 0);
    view.append_column(&column);

    let target = resolve_frame_handle(parent);
    TREE_NOTIFY.with(|m| {
        m.borrow_mut().insert(handle, (target, command_id));
    });

    let selection = view.selection();
    selection.set_mode(gtk4::SelectionMode::Single);
    let h = handle;
    selection.connect_changed(move |_| {
        if TREE_REBUILDING.with(|c| c.get()) {
            return;
        }
        let node = tree_selected_node(h);
        TREE_SELECTED.with(|m| {
            if let Some(entry) = m.borrow().get(&h) {
                *entry.borrow_mut() = node;
            }
        });
        if let Some((frame, id)) = TREE_NOTIFY.with(|m| m.borrow().get(&h).copied()) {
            dispatch_notify(frame, id, TREE_NOTIFY_SELCHANGED);
        }
    });

    attach(parent, handle, &view.upcast::<gtk4::Widget>(), initial_rect);
    crate::platform::declared::mark(handle, "tree");
    handle
}

/// Nodo selezionato letto dalla vista GTK (colonna 1 = id).
fn tree_selected_node(handle: NativeHandle) -> Option<usize> {
    let view = tree_view_of(handle)?;
    let selection = view.selection();
    let (model, iter) = selection.selected()?;
    let id: i64 = model.get_value(&iter, 1).get().ok()?;
    if id > 0 {
        Some(id as usize)
    } else {
        None
    }
}

/// Trova l'`iter` la cui colonna 1 vale `id` (ricorsivo su tutto l'albero).
fn find_iter_by_id(store: &gtk4::TreeStore, id: usize) -> Option<gtk4::TreeIter> {
    fn walk(
        store: &gtk4::TreeStore,
        parent: Option<&gtk4::TreeIter>,
        id: usize,
    ) -> Option<gtk4::TreeIter> {
        let mut iter = store.iter_children(parent)?;
        loop {
            let val: i64 = store.get_value(&iter, 1).get().ok()?;
            if val as usize == id {
                return Some(iter.clone());
            }
            if let Some(found) = walk(store, Some(&iter), id) {
                return Some(found);
            }
            if !store.iter_next(&mut iter) {
                return None;
            }
        }
    }
    walk(store, None, id)
}

/// Inserisce ricorsivamente il nodo e i suoi figli nel `TreeStore`.
fn insert_node_recursive(
    store: &gtk4::TreeStore,
    handle: NativeHandle,
    parent: Option<&gtk4::TreeIter>,
    node_id: usize,
) {
    let (text, children) = TREE_NODES
        .with(|m| {
            m.borrow().get(&handle).and_then(|nodes| {
                nodes
                    .borrow()
                    .get(&node_id)
                    .map(|n| (n.text.clone(), n.children.clone()))
            })
        })
        .unwrap_or_else(|| (String::new(), Vec::new()));
    let text_v: &dyn glib::value::ToValue = &text;
    let id_v: &dyn glib::value::ToValue = &(node_id as i64);
    let iter = store.insert_with_values(parent, None, &[(0, text_v), (1, id_v)]);
    for child in children {
        insert_node_recursive(store, handle, Some(&iter), child);
    }
}

/// Ricostruisce la `TreeView` dal modello in-memory, ripristinando
/// espansione e selezione. Sopramezza il dispatch durante la rebuild.
fn tree_redraw(handle: NativeHandle) {
    let Some(view) = tree_view_of(handle) else {
        return;
    };
    let Some(store) = view
        .model()
        .and_then(|m| m.downcast::<gtk4::TreeStore>().ok())
    else {
        return;
    };

    let selected = TREE_SELECTED.with(|m| m.borrow().get(&handle).and_then(|s| *s.borrow()));
    TREE_REBUILDING.with(|c| c.set(true));

    store.clear();
    let roots: Vec<usize> = TREE_ROOTS
        .with(|m| m.borrow().get(&handle).map(|r| r.borrow().clone()))
        .unwrap_or_default();
    for root in roots {
        insert_node_recursive(&store, handle, None, root);
    }

    // Ripristina l'espansione programmata.
    let expanded: Vec<usize> = TREE_NODES
        .with(|m| {
            m.borrow().get(&handle).map(|nodes| {
                nodes
                    .borrow()
                    .iter()
                    .filter(|(_, n)| n.expanded)
                    .map(|(k, _)| *k)
                    .collect()
            })
        })
        .unwrap_or_default();
    for id in expanded {
        if let Some(iter) = find_iter_by_id(&store, id) {
            let path = store.path(&iter);
            view.expand_row(&path, false);
        }
    }

    // Ripristina la selezione.
    if let Some(sel) = selected {
        if let Some(iter) = find_iter_by_id(&store, sel) {
            let path = store.path(&iter);
            view.selection().select_path(&path);
        }
    }
    TREE_SELECTED.with(|m| {
        if let Some(entry) = m.borrow().get(&handle) {
            *entry.borrow_mut() = selected;
        }
    });
    TREE_REBUILDING.with(|c| c.set(false));
}

/// Allocà un nuovo nodo (root o figlio) e notifica la vista.
fn tree_alloc_node(handle: NativeHandle, parent: Option<usize>, text: &str) -> isize {
    let id = TREE_NEXT_ID
        .with(|m| {
            m.borrow().get(&handle).map(|cell| {
                let id = cell.get();
                cell.set(id + 1);
                id
            })
        })
        .unwrap_or(1);

    TREE_NODES.with(|m| {
        if let Some(nodes) = m.borrow().get(&handle) {
            nodes.borrow_mut().insert(
                id,
                GtkTreeNode {
                    text: text.to_string(),
                    parent,
                    children: Vec::new(),
                    expanded: false,
                },
            );
        }
    });

    if let Some(p) = parent {
        TREE_NODES.with(|m| {
            if let Some(nodes) = m.borrow().get(&handle) {
                if let Some(node) = nodes.borrow_mut().get_mut(&p) {
                    node.children.push(id);
                }
            }
        });
    } else {
        TREE_ROOTS.with(|m| {
            if let Some(roots) = m.borrow().get(&handle) {
                roots.borrow_mut().push(id);
            }
        });
    }

    tree_redraw(handle);
    id as isize
}

/// Inserisce un nodo radice. Restituisce l'id (`TreeItem`).
pub fn tree_add_root(handle: NativeHandle, text: &str) -> isize {
    tree_alloc_node(handle, None, text)
}

/// Inserisce un nodo figlio sotto `parent`. Restituisce l'id, oppure
/// `0` se il genitore non è valido.
pub fn tree_append_item(handle: NativeHandle, parent: isize, text: &str) -> isize {
    if parent == 0 {
        return 0;
    }
    tree_alloc_node(handle, Some(parent as usize), text)
}

/// Testo del nodo, o `None` se l'id non esiste.
pub fn tree_get_item_text(handle: NativeHandle, item: isize) -> Option<String> {
    if item == 0 {
        return None;
    }
    TREE_NODES.with(|m| {
        m.borrow()
            .get(&handle)
            .and_then(|nodes| nodes.borrow().get(&(item as usize)).map(|n| n.text.clone()))
    })
}

/// Nodo selezionato (id), o `None`.
pub fn tree_get_selection(handle: NativeHandle) -> Option<isize> {
    TREE_SELECTED
        .with(|m| m.borrow().get(&handle).and_then(|s| *s.borrow()))
        .map(|id| id as isize)
}

/// Seleziona il nodo (aggiorna lo stato e la vista GTK).
pub fn tree_select_item(handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    TREE_SELECTED.with(|m| {
        if let Some(entry) = m.borrow().get(&handle) {
            *entry.borrow_mut() = Some(item as usize);
        }
    });
    if let Some(view) = tree_view_of(handle) {
        if let Some(store) = view
            .model()
            .and_then(|m| m.downcast::<gtk4::TreeStore>().ok())
        {
            if let Some(iter) = find_iter_by_id(&store, item as usize) {
                let path = store.path(&iter);
                view.selection().select_path(&path);
                view.scroll_to_cell(Some(&path), None::<&gtk4::TreeViewColumn>, false, 0.0, 0.0);
            }
        }
    }
}

/// Contrassegna il nodo come espanso (e lo espande nella vista).
pub fn tree_expand(handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    TREE_NODES.with(|m| {
        if let Some(nodes) = m.borrow().get(&handle) {
            if let Some(node) = nodes.borrow_mut().get_mut(&(item as usize)) {
                node.expanded = true;
            }
        }
    });
    if let Some(view) = tree_view_of(handle) {
        if let Some(store) = view
            .model()
            .and_then(|m| m.downcast::<gtk4::TreeStore>().ok())
        {
            if let Some(iter) = find_iter_by_id(&store, item as usize) {
                let path = store.path(&iter);
                view.expand_row(&path, false);
            }
        }
    }
}

/// Contrassegna il nodo come collassato (e lo collassa nella vista).
pub fn tree_collapse(handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    TREE_NODES.with(|m| {
        if let Some(nodes) = m.borrow().get(&handle) {
            if let Some(node) = nodes.borrow_mut().get_mut(&(item as usize)) {
                node.expanded = false;
            }
        }
    });
    if let Some(view) = tree_view_of(handle) {
        if let Some(store) = view
            .model()
            .and_then(|m| m.downcast::<gtk4::TreeStore>().ok())
        {
            if let Some(iter) = find_iter_by_id(&store, item as usize) {
                let path = store.path(&iter);
                view.collapse_row(&path);
            }
        }
    }
}

/// Primo nodo radice.
pub fn tree_get_root_item(handle: NativeHandle) -> Option<isize> {
    TREE_ROOTS
        .with(|m| {
            m.borrow()
                .get(&handle)
                .and_then(|r| r.borrow().first().copied())
        })
        .map(|id| id as isize)
}

/// Primo figlio del nodo.
pub fn tree_get_first_child(handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    TREE_NODES
        .with(|m| {
            m.borrow().get(&handle).and_then(|nodes| {
                nodes
                    .borrow()
                    .get(&(item as usize))
                    .and_then(|n| n.children.first().copied())
            })
        })
        .map(|id| id as isize)
}

/// Fratello successivo del nodo.
pub fn tree_get_next_sibling(handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    sibling_at(handle, item as usize, 1).map(|id| id as isize)
}

/// Fratello precedente del nodo.
pub fn tree_get_prev_sibling(handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    sibling_at(handle, item as usize, -1).map(|id| id as isize)
}

/// Helper: trova il fratello a offset `delta` rispetto a `item`
/// nell'ordine dei figli del suo genitore (o tra le radici).
fn sibling_at(handle: NativeHandle, item: usize, delta: isize) -> Option<usize> {
    let parent = TREE_NODES.with(|m| {
        m.borrow()
            .get(&handle)
            .and_then(|nodes| nodes.borrow().get(&item).and_then(|n| n.parent))
    });
    let (siblings, pos) = match parent {
        Some(p) => {
            let list = TREE_NODES.with(|m| {
                m.borrow()
                    .get(&handle)
                    .and_then(|nodes| nodes.borrow().get(&p).map(|n| n.children.clone()))
            })?;
            let pos = list.iter().position(|&c| c == item)?;
            (list, pos)
        }
        None => {
            let list = TREE_ROOTS.with(|m| m.borrow().get(&handle).map(|r| r.borrow().clone()))?;
            let pos = list.iter().position(|&c| c == item)?;
            (list, pos)
        }
    };
    let target = pos as isize + delta;
    if target < 0 {
        return None;
    }
    siblings.get(target as usize).copied()
}

/// Genitore del nodo (o `None` se è una radice).
pub fn tree_get_item_parent(handle: NativeHandle, item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    TREE_NODES
        .with(|m| {
            m.borrow()
                .get(&handle)
                .and_then(|nodes| nodes.borrow().get(&(item as usize)).and_then(|n| n.parent))
        })
        .map(|id| id as isize)
}

/// Numero totale di nodi nell'albero.
pub fn tree_get_count(handle: NativeHandle) -> usize {
    TREE_NODES
        .with(|m| m.borrow().get(&handle).map(|nodes| nodes.borrow().len()))
        .unwrap_or(0)
}

/// Cambia il testo di un nodo esistente.
pub fn tree_set_item_text(handle: NativeHandle, item: isize, text: &str) {
    if item == 0 {
        return;
    }
    TREE_NODES.with(|m| {
        if let Some(nodes) = m.borrow().get(&handle) {
            if let Some(node) = nodes.borrow_mut().get_mut(&(item as usize)) {
                node.text = text.to_string();
            }
        }
    });
    tree_redraw(handle);
}

fn tree_delete_subtree(handle: NativeHandle, node_id: usize) {
    let children: Vec<usize> = TREE_NODES
        .with(|m| {
            m.borrow()
                .get(&handle)
                .and_then(|nodes| nodes.borrow().get(&node_id).map(|n| n.children.clone()))
        })
        .unwrap_or_default();
    for child in children {
        tree_delete_subtree(handle, child);
    }
    TREE_NODES.with(|m| {
        if let Some(nodes) = m.borrow().get(&handle) {
            nodes.borrow_mut().remove(&node_id);
        }
    });
}

/// Elimina il nodo e tutti i suoi discendenti.
pub fn tree_delete_item(handle: NativeHandle, item: isize) {
    if item == 0 {
        return;
    }
    let node_id = item as usize;
    let parent = TREE_NODES.with(|m| {
        m.borrow()
            .get(&handle)
            .and_then(|nodes| nodes.borrow().get(&node_id).and_then(|n| n.parent))
    });
    if let Some(p) = parent {
        TREE_NODES.with(|m| {
            if let Some(nodes) = m.borrow().get(&handle) {
                if let Some(node) = nodes.borrow_mut().get_mut(&p) {
                    node.children.retain(|&c| c != node_id);
                }
            }
        });
    } else {
        TREE_ROOTS.with(|m| {
            if let Some(roots) = m.borrow().get(&handle) {
                roots.borrow_mut().retain(|&c| c != node_id);
            }
        });
    }
    TREE_SELECTED.with(|m| {
        if let Some(entry) = m.borrow().get(&handle) {
            if *entry.borrow() == Some(node_id) {
                *entry.borrow_mut() = None;
            }
        }
    });
    tree_delete_subtree(handle, node_id);
    tree_redraw(handle);
}

/// Elimina tutti i nodi dell'albero.
pub fn tree_delete_all_items(handle: NativeHandle) {
    TREE_NODES.with(|m| {
        if let Some(nodes) = m.borrow().get(&handle) {
            nodes.borrow_mut().clear();
        }
    });
    TREE_ROOTS.with(|m| {
        if let Some(roots) = m.borrow().get(&handle) {
            roots.borrow_mut().clear();
        }
    });
    TREE_SELECTED.with(|m| {
        if let Some(entry) = m.borrow().get(&handle) {
            *entry.borrow_mut() = None;
        }
    });
    TREE_NEXT_ID.with(|m| {
        if let Some(counter) = m.borrow().get(&handle) {
            counter.set(1);
        }
    });
    tree_redraw(handle);
}

// ── Widget: proprietà ───────────────────────────────────────────────

/// Sposta/ridimensiona un widget nel contenitore del genitore.
pub fn set_view_frame(parent: NativeHandle, handle: NativeHandle, rect: Rect) {
    let Some(widget) = widget_of(handle) else {
        return;
    };
    widget.set_size_request(rect.width as i32, rect.height as i32);
    if let Some(fixed) = fixed_for(parent) {
        // `move_` funziona solo se il widget è già figlio di questo
        // `Fixed`: se è stato riparentato altrove (pane host, tab)
        // staccalo e reinseriscilo qui.
        let already_inside = widget
            .parent()
            .map(|p| p.is::<Fixed>() && p == fixed.clone().upcast::<gtk4::Widget>())
            .unwrap_or(false);
        if already_inside {
            fixed.move_(&widget, rect.x as f64, rect.y as f64);
        } else {
            if widget.parent().is_some() {
                widget.unparent();
            }
            fixed.put(&widget, rect.x as f64, rect.y as f64);
        }
    }
}

/// Mostra/nasconde un widget.
pub fn set_view_visible(handle: NativeHandle, visible: bool) {
    if let Some(widget) = widget_of(handle) {
        widget.set_visible(visible);
    }
}

/// Abilita/disabilita un widget.
pub fn set_view_enabled(handle: NativeHandle, enabled: bool) {
    if let Some(widget) = widget_of(handle) {
        widget.set_sensitive(enabled);
    }
}

/// Cambia l'etichetta di un pulsante.
pub fn set_button_title(handle: NativeHandle, title: &str) {
    if let Some(widget) = widget_of(handle) {
        if let Ok(button) = widget.downcast::<Button>() {
            button.set_label(title);
        }
    }
}

/// Cambia il testo di un label statico.
pub fn set_static_text(handle: NativeHandle, text: &str) {
    if let Some(widget) = widget_of(handle) {
        if let Ok(label) = widget.downcast::<Label>() {
            label.set_text(text);
        }
    }
}

/// Il `TextView` sottostante (anche nascosto dentro una ScrolledWindow).
fn text_view_of(handle: NativeHandle) -> Option<TextView> {
    let widget = widget_of(handle)?;
    if let Ok(view) = widget.clone().downcast::<TextView>() {
        return Some(view);
    }
    if let Ok(scrolled) = widget.downcast::<ScrolledWindow>() {
        return scrolled.child()?.downcast::<TextView>().ok();
    }
    None
}

/// Legge il testo corrente di Entry / TextView / Label.
pub fn get_widget_text(handle: NativeHandle) -> Option<String> {
    let widget = widget_of(handle)?;
    if let Ok(entry) = widget.clone().downcast::<Entry>() {
        return Some(entry.text().to_string());
    }
    if let Some(view) = text_view_of(handle) {
        let buffer = view.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        return Some(text.to_string());
    }
    if let Ok(label) = widget.downcast::<Label>() {
        return Some(label.text().to_string());
    }
    None
}

/// Sostituisce il testo di Entry / TextView.
pub fn set_widget_text(handle: NativeHandle, text: &str) {
    if let Some(widget) = widget_of(handle) {
        if let Ok(entry) = widget.downcast::<Entry>() {
            entry.set_text(text);
            return;
        }
    }
    if let Some(view) = text_view_of(handle) {
        view.buffer().set_text(text);
    }
}

/// Rende il controllo di testo sola lettura.
pub fn set_widget_readonly(handle: NativeHandle, readonly: bool) {
    if let Some(widget) = widget_of(handle) {
        if let Ok(entry) = widget.downcast::<Entry>() {
            entry.set_editable(!readonly);
            return;
        }
    }
    if let Some(view) = text_view_of(handle) {
        view.set_editable(!readonly);
    }
}

/// Lunghezza massima di un campo singolo (no-op per i multilinea).
pub fn set_widget_max_length(handle: NativeHandle, max_length: i32) {
    if let Some(widget) = widget_of(handle) {
        if let Ok(entry) = widget.downcast::<Entry>() {
            entry.set_max_length(max_length);
        }
    }
}

// ── Bounds in coordinate schermo ────────────────────────────────────

/// FFI minimale su libX11: serve solo a leggere la posizione della
/// finestra top-level (`gdk4-sys` non espone `gdk_toplevel_get_position`).
///
/// Duplicata (non unificata) rispetto a `inspector_gtk::x11` perché
/// quel modulo è gated debug/feature mentre questo è sempre attivo.
#[allow(clashing_extern_declarations)]
mod screen_x11 {
    use std::os::raw::{c_char, c_int, c_uint, c_ulong};

    #[repr(C)]
    pub struct Display {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct XErrorEvent {
        _private: [u8; 0],
    }

    pub type ErrorHandler = Option<extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>;

    #[link(name = "X11")]
    extern "C" {
        pub fn XOpenDisplay(name: *const c_char) -> *mut Display;
        pub fn XCloseDisplay(dpy: *mut Display) -> c_int;
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
        pub fn XSync(dpy: *mut Display, discard: c_int) -> c_int;
        pub fn XSetErrorHandler(handler: ErrorHandler) -> ErrorHandler;
    }
}

/// Error handler che ignora ogni errore di protocollo X (solo per la
/// nostra connessione dedicata, ripristinato subito dopo).
extern "C" fn ignore_screen_x_error(
    _d: *mut screen_x11::Display,
    _e: *mut screen_x11::XErrorEvent,
) -> std::os::raw::c_int {
    0
}

/// Posizione (x, y) della finestra in coordinate schermo, letta con
/// `XGetGeometry` su una connessione dedicata: per una top-level il
/// parent è la root window, quindi `x`/`y` sono già coordinate schermo.
/// Fallback `(0, 0)` senza server X o con backend non-X11 (Wayland).
fn window_screen_origin(window: &Window) -> (i32, i32) {
    use std::os::raw::{c_int, c_uint, c_ulong};

    let Some(surface) = window.surface() else {
        return (0, 0);
    };
    let Ok(x11_surface) = surface.downcast::<X11Surface>() else {
        return (0, 0);
    };
    unsafe {
        let dpy = screen_x11::XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return (0, 0);
        }
        let prev = screen_x11::XSetErrorHandler(Some(ignore_screen_x_error));
        let mut root: c_ulong = 0;
        let mut x: c_int = 0;
        let mut y: c_int = 0;
        let mut w: c_uint = 0;
        let mut h: c_uint = 0;
        let mut border: c_uint = 0;
        let mut depth: c_uint = 0;
        let ok = screen_x11::XGetGeometry(
            dpy,
            x11_surface.xid(),
            &mut root,
            &mut x,
            &mut y,
            &mut w,
            &mut h,
            &mut border,
            &mut depth,
        );
        screen_x11::XSync(dpy, 0);
        screen_x11::XSetErrorHandler(prev);
        screen_x11::XCloseDisplay(dpy);
        if ok == 0 {
            (0, 0)
        } else {
            (x, y)
        }
    }
}

/// Screen-space rect (origine in alto a sinistra) di un widget: origine
/// schermo della finestra top-level (X11) più bounds relativi al widget
/// finestra (`compute_bounds`).
///
/// Per l'handle di una frame (assente da `VIEWS`) si misura la finestra
/// stessa. Ritorna un rect tutto a zero per handle senza widget, finestra
/// non ancora mappata o ambienti senza server X.
pub fn widget_screen_rect(handle: NativeHandle) -> Rect {
    let frame = resolve_frame_handle(handle);
    let Some(window) = frame_window_of(frame) else {
        return Rect::new(0, 0, 0, 0);
    };
    let target: gtk4::Widget = match widget_of(handle) {
        Some(widget) => widget,
        None if handle == frame => window.clone().upcast::<gtk4::Widget>(),
        None => return Rect::new(0, 0, 0, 0),
    };
    let (origin_x, origin_y) = window_screen_origin(&window);
    match target.compute_bounds(&window) {
        Some(bounds) => Rect::new(
            origin_x + bounds.x() as i32,
            origin_y + bounds.y() as i32,
            bounds.width().max(0.0) as u32,
            bounds.height().max(0.0) as u32,
        ),
        None => Rect::new(0, 0, 0, 0),
    }
}

// ── Menu bar ────────────────────────────────────────────────────────

/// Pulisce la label di menu: rimuove il mnemonic `&` e la scorciatoia
/// Win32 dopo `\t` (GTK mostra le scorciatoie in modo separato).
fn gtk_menu_label(label: &str) -> String {
    let text = label.split('\t').next().unwrap_or(label);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                chars.next();
                out.push('&');
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Installa (o reinstalla) la menu bar della frame.
pub fn install_menu_bar(frame_handle: NativeHandle, menus: &[(String, Vec<MenuInstallRow>)]) {
    if !ensure_ready() {
        return;
    }
    let owned: Vec<(String, Vec<MenuInstallRow>)> = menus.to_vec();
    FRAME_MENUS.with(|map| map.borrow_mut().insert(frame_handle, owned.clone()));
    build_menu_bar(frame_handle, &owned);
}

fn build_menu_bar(frame_handle: NativeHandle, menus: &[(String, Vec<MenuInstallRow>)]) {
    let Some(root) = FRAME_ROOT.with(|map| map.borrow().get(&frame_handle).cloned()) else {
        return;
    };
    let Some(window) = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned()) else {
        return;
    };

    // Rimuovi l'eventuale barra precedente.
    if let Some(old) = FRAME_MENUBARS.with(|map| map.borrow_mut().remove(&frame_handle)) {
        root.remove(&old);
    }

    let group = gio::SimpleActionGroup::new();
    let model = gio::Menu::new();

    for (title, items) in menus {
        let submenu = gio::Menu::new();
        let mut section = gio::Menu::new();

        for (id, label, enabled, kind, checked, shortcut) in items {
            if *kind == MenuItemKind::Separator {
                submenu.append_section(None, &section);
                section = gio::Menu::new();
                continue;
            }

            let mut text = gtk_menu_label(label);
            if let Some(accelerator) = shortcut {
                text.push_str("  ");
                text.push_str(&accelerator.to_string());
            }

            let action_name = format!("cmd{}", id);
            let detailed = format!("wx.{}", action_name);
            let is_toggle = *kind == MenuItemKind::Check || *kind == MenuItemKind::Radio;

            let action = if is_toggle {
                gio::SimpleAction::new_stateful(&action_name, None, &checked.to_variant())
            } else {
                gio::SimpleAction::new(&action_name, None)
            };
            action.set_enabled(*enabled);

            let target = resolve_frame_handle(frame_handle);
            let command_id = *id;
            let state = Rc::new(Cell::new(*checked));
            action.connect_activate(move |action, _| {
                if is_toggle {
                    let new_state = !state.get();
                    state.set(new_state);
                    action.set_state(&new_state.to_variant());
                }
                dispatch_command(target, command_id);
            });
            group.add_action(&action);

            section.append(Some(&text), Some(&detailed));
        }

        submenu.append_section(None, &section);
        // Anche i titoli di primo livello passano da `gtk_menu_label`:
        // altrimenti il mnemonic Win32 `&` comparrebbe letterale.
        model.append_submenu(Some(&gtk_menu_label(title)), &submenu);
    }

    window.insert_action_group("wx", Some(&group));

    let bar = PopoverMenuBar::from_model(Some(&model));
    bar.set_hexpand(true);
    root.prepend(&bar);

    FRAME_MENUBARS.with(|map| map.borrow_mut().insert(frame_handle, bar));
    FRAME_ACTION_GROUPS.with(|map| map.borrow_mut().insert(frame_handle, group));
}

/// Aggiorna lo stato checked di una voce di menu.
pub fn menu_set_item_checked(frame_handle: NativeHandle, id: u16, checked: bool) {
    let group = FRAME_ACTION_GROUPS.with(|map| map.borrow().get(&frame_handle).cloned());
    if let Some(group) = group {
        if let Some(action) = group.lookup_action(&format!("cmd{}", id)) {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                action.set_state(&checked.to_variant());
            }
        }
    }
    FRAME_MENUS.with(|map| {
        if let Some(menus) = map.borrow_mut().get_mut(&frame_handle) {
            for (_, items) in menus.iter_mut() {
                for item in items.iter_mut() {
                    if item.0 == id {
                        item.4 = checked;
                    }
                }
            }
        }
    });
}

/// Aggiorna la scorciatoia di una voce di menu (ricostruisce la barra).
pub fn menu_set_item_shortcut(frame_handle: NativeHandle, id: u16, shortcut: Option<Accelerator>) {
    let menus = FRAME_MENUS.with(|map| {
        let mut map = map.borrow_mut();
        let menus = map.get_mut(&frame_handle)?;
        for (_, items) in menus.iter_mut() {
            for item in items.iter_mut() {
                if item.0 == id {
                    item.5 = shortcut;
                }
            }
        }
        Some(menus.clone())
    });
    if let Some(menus) = menus {
        build_menu_bar(frame_handle, &menus);
    }
}

// ── MessageBox modale ───────────────────────────────────────────────

/// Dialogo modale equivalente della `MessageBoxW` / `NSAlert`.
///
/// In assenza di GTK restituisce `MessageBoxResult::Cancel`, come il
/// precedente stub headless.
pub fn show_message_box(
    parent: NativeHandle,
    message: &str,
    title: &str,
    style: MessageBoxStyle,
    icon: MessageBoxIcon,
) -> MessageBoxResult {
    if !ensure_ready() {
        return MessageBoxResult::Cancel;
    }
    let frame_handle = resolve_frame_handle(parent);
    let Some(window) = FRAME_WINDOWS.with(|map| map.borrow().get(&frame_handle).cloned()) else {
        return MessageBoxResult::Cancel;
    };

    let dialog = Dialog::new();
    dialog.set_title(Some(title));
    dialog.set_transient_for(Some(&window));
    dialog.set_modal(true);
    let _ = icon;

    let label = Label::new(Some(message));
    label.set_wrap(true);
    label.set_margin_top(12);
    label.set_margin_bottom(12);
    label.set_margin_start(12);
    label.set_margin_end(12);
    dialog.content_area().append(&label);

    match style {
        MessageBoxStyle::Ok => {
            dialog.add_button("OK", ResponseType::Ok);
        }
        MessageBoxStyle::OkCancel => {
            dialog.add_button("OK", ResponseType::Ok);
            dialog.add_button("Cancel", ResponseType::Cancel);
        }
        MessageBoxStyle::YesNo => {
            dialog.add_button("Yes", ResponseType::Yes);
            dialog.add_button("No", ResponseType::No);
        }
        MessageBoxStyle::YesNoCancel => {
            dialog.add_button("Yes", ResponseType::Yes);
            dialog.add_button("No", ResponseType::No);
            dialog.add_button("Cancel", ResponseType::Cancel);
        }
    }

    let result = Rc::new(Cell::new(None::<MessageBoxResult>));
    {
        let result = Rc::clone(&result);
        dialog.connect_response(move |dialog, response| {
            // Convenzione macOS: First → Ok/Yes, Second → Cancel/No.
            let value = match style {
                MessageBoxStyle::Ok => MessageBoxResult::Ok,
                MessageBoxStyle::OkCancel => match response {
                    ResponseType::Ok => MessageBoxResult::Ok,
                    _ => MessageBoxResult::Cancel,
                },
                MessageBoxStyle::YesNo => match response {
                    ResponseType::Yes => MessageBoxResult::Yes,
                    _ => MessageBoxResult::No,
                },
                MessageBoxStyle::YesNoCancel => match response {
                    ResponseType::Yes => MessageBoxResult::Yes,
                    ResponseType::No => MessageBoxResult::No,
                    _ => MessageBoxResult::Cancel,
                },
            };
            result.set(Some(value));
            dialog.destroy();
        });
    }

    dialog.present();

    let context = glib::MainContext::default();
    while result.get().is_none() {
        context.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    result.get().unwrap_or(MessageBoxResult::Cancel)
}

// ── Timer (GLib timeout sul main loop) ──────────────────────────────

thread_local! {
    /// Callback registrati per timer id: condivisi per `Rc` con la
    /// sorgente attiva, così un `timer_set_fire` successivo aggiorna
    /// anche il timer già in esecuzione.
    static TIMER_CALLBACKS: RefCell<HashMap<u32, Rc<RefCell<Option<Box<dyn FnMut()>>>>>> =
        RefCell::new(HashMap::new());
    /// Sorgente GLib attiva per timer id, con la "generazione" che ha
    /// creato la voce: serve a non rimuovere una sorgente appena
    /// sostituita da un `timer_start` dentro il callback one-shot.
    static TIMER_SOURCES: RefCell<HashMap<u32, (u64, glib::SourceId)>> =
        RefCell::new(HashMap::new());
    /// Contatore delle generazioni dei timer (numerazione degli start).
    static TIMER_GENERATION: Cell<u64> = Cell::new(0);
}

/// Registra il callback che il timer chiamerà a ogni scatto.
pub fn timer_set_fire(timer_id: u32, callback: Box<dyn FnMut()>) {
    TIMER_CALLBACKS.with(|map| {
        let mut map = map.borrow_mut();
        match map.get(&timer_id) {
            // Stessa cella: la sorgente già attiva vede il nuovo cb.
            Some(slot) => *slot.borrow_mut() = Some(callback),
            None => {
                map.insert(timer_id, Rc::new(RefCell::new(Some(callback))));
            }
        }
    });
}

/// Avvia (o riavvia da zero) il timer. `repeat == false` = one-shot.
pub fn timer_start(timer_id: u32, ms: u32, repeat: bool) {
    timer_stop(timer_id);
    let Some(callback) = TIMER_CALLBACKS.with(|map| map.borrow().get(&timer_id).cloned()) else {
        return;
    };
    let generation = TIMER_GENERATION.with(|gen| {
        let next = gen.get().wrapping_add(1);
        gen.set(next);
        next
    });
    let source = glib::timeout_add_local(std::time::Duration::from_millis(ms as u64), move || {
        // Esegue il callback fuori dal prestito: così il tick può
        // legalmente rifare `start`/`stop` sullo stesso timer.
        let tick = callback.borrow_mut().take();
        if let Some(mut tick) = tick {
            tick();
            let mut slot = callback.borrow_mut();
            if slot.is_none() {
                *slot = Some(tick);
            }
        }
        if repeat {
            return glib::ControlFlow::Continue;
        }
        // One-shot: la sorgente muore da sola, ci togliamo dal
        // registro solo se non è già stata sostituita (riavvio
        // programmato dentro il callback).
        TIMER_SOURCES.with(|map| {
            let mut map = map.borrow_mut();
            if map
                .get(&timer_id)
                .is_some_and(|(gen, _)| *gen == generation)
            {
                map.remove(&timer_id);
            }
        });
        glib::ControlFlow::Break
    });
    TIMER_SOURCES.with(|map| map.borrow_mut().insert(timer_id, (generation, source)));
}

/// Ferma il timer se sta girando (nessun effetto se è già fermo).
pub fn timer_stop(timer_id: u32) {
    if let Some((_, source)) = TIMER_SOURCES.with(|map| map.borrow_mut().remove(&timer_id)) {
        source.remove();
    }
}

/// Rimuove sorgente e callback: va chiamato alla drop del `Timer`.
pub fn timer_unregister(timer_id: u32) {
    timer_stop(timer_id);
    TIMER_CALLBACKS.with(|map| map.borrow_mut().remove(&timer_id));
}

// ── Tooltip (set_tooltip_text nativo GTK) ────────────────────────────

/// Imposta (`Some`) o rimuove (`None`) il tooltip del widget collegato
/// all'handle. GTK mostra la bolla al passaggio del mouse senza altre
/// registrazioni.
pub fn view_set_tooltip(handle: NativeHandle, text: Option<&str>) {
    if let Some(widget) = widget_of(handle) {
        widget.set_tooltip_text(text);
    }
}

// ── Splitter (GtkPaned) ──────────────────────────────────────────────

thread_local! {
    /// Handle nativo → splitter GTK reale.
    static SPLITTERS: RefCell<HashMap<NativeHandle, gtk4::Paned>> =
        RefCell::new(HashMap::new());
    /// Handle nativo → handler invocato a ogni variazione della sash.
    static SASH_HANDLERS: RefCell<HashMap<NativeHandle, Rc<RefCell<dyn FnMut(i32)>>>> =
        RefCell::new(HashMap::new());
    /// Handle nativo → `true` quando il segnale `position-notify`
    /// è già collegato all'handler.
    static SASH_CONNECTED: RefCell<HashMap<NativeHandle, bool>> =
        RefCell::new(HashMap::new());
}

/// Orientamento dello splitter (asse della sash), specchio
/// dell'omonimo enum del modulo `containers::splitter_window`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitterOrientation {
    /// Sash verticale: primo pannello a sinistra, secondo a destra.
    Vertical,
    /// Sash orizzontale: primo pannello in alto, secondo in basso.
    Horizontal,
}

/// Mappa l'orientamento della sash sull'orientamento del `Paned`:
/// un `Paned` orizzontale affianca i figli, quindi la sash è verticale.
fn gtk_orientation(orientation: SplitterOrientation) -> gtk4::Orientation {
    match orientation {
        SplitterOrientation::Vertical => gtk4::Orientation::Horizontal,
        SplitterOrientation::Horizontal => gtk4::Orientation::Vertical,
    }
}

fn splitter_of(handle: NativeHandle) -> Option<gtk4::Paned> {
    SPLITTERS.with(|map| map.borrow().get(&handle).cloned())
}

/// Collega (una sola volta per splitter) il segnale
/// `position-notify` all'handler registrato con
/// [`splitter_set_sash_handler`]. Nessun effetto se l'handler
/// non esiste ancora.
fn connect_sash_notify(splitter: NativeHandle) {
    let connected = SASH_CONNECTED.with(|map| map.borrow().contains_key(&splitter));
    if connected {
        return;
    }
    let handler = SASH_HANDLERS.with(|map| map.borrow().get(&splitter).cloned());
    let paned = splitter_of(splitter);
    let (Some(handler), Some(paned)) = (handler, paned) else {
        return;
    };
    // Il segnale scatta anche per gli spostamenti programmatici:
    // è il modo più fedele a "ogni variazione di sash".
    paned.connect_position_notify(move |paned| {
        handler.borrow_mut()(paned.position());
    });
    SASH_CONNECTED.with(|map| map.borrow_mut().insert(splitter, true));
}

/// Registra (e sostituisce l'eventuale precedente) il callback
/// invocato con la nuova posizione della sash ogni volta che
/// l'utente la trascina o il programma la sposta.
pub fn splitter_set_sash_handler<F: FnMut(i32) + 'static>(splitter: NativeHandle, callback: F) {
    SASH_HANDLERS.with(|map| {
        map.borrow_mut()
            .insert(splitter, Rc::new(RefCell::new(callback)))
    });
    connect_sash_notify(splitter);
}

/// Crea uno splitter (due pannelli separati da una sash
/// trascinabile) come figlio di `parent`.
///
/// `vertical` indica una sash verticale iniziale (`true`), cioè
/// pannelli affiancati — stessa convenzione della controparte
/// appkit.
pub fn create_splitter(parent: NativeHandle, initial_rect: Rect, vertical: bool) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let orientation = if vertical {
        gtk_orientation(SplitterOrientation::Vertical)
    } else {
        gtk_orientation(SplitterOrientation::Horizontal)
    };
    let paned = gtk4::Paned::new(orientation);
    // `upcast_ref` tiene `paned` vivo: serve per il registro.
    attach(
        parent,
        handle,
        paned.upcast_ref::<gtk4::Widget>(),
        initial_rect,
    );
    SPLITTERS.with(|map| map.borrow_mut().insert(handle, paned));
    handle
}

/// Sostituisce i due pannelli dello splitter e ne imposta
/// l'orientamento della sash.
///
/// I widget vengono prima staccati dal genitore attuale (nella
/// pratica i pannelli sono creati come figli del frame) e poi
/// riparentati dentro il `Paned`, che si occupa da solo di
/// ridistribuirli al resize e al trascinamento della sash.
pub fn splitter_set_panes(
    splitter: NativeHandle,
    pane1: NativeHandle,
    pane2: NativeHandle,
    orientation: SplitterOrientation,
) {
    let Some(paned) = splitter_of(splitter) else {
        return;
    };
    paned.set_orientation(gtk_orientation(orientation));
    for handle in [pane1, pane2] {
        if let Some(widget) = widget_of(handle) {
            if let Some(parent_widget) = widget.parent() {
                // Stacca solo se il genitore è un altro widget:
                // se è già dentro questo splitter non farebbe che
                // rimuoverlo per riaggiungerlo.
                let already_inside =
                    (parent_widget.as_ptr() as *const ()) == (paned.as_ptr() as *const ());
                if !already_inside {
                    match parent_widget.downcast_ref::<Fixed>() {
                        Some(fixed) => fixed.remove(&widget),
                        None => widget.unparent(),
                    }
                }
            }
        }
    }
    let first = widget_of(pane1);
    let second = widget_of(pane2);
    paned.set_start_child(first.as_ref());
    paned.set_end_child(second.as_ref());
    connect_sash_notify(splitter);
}

/// Imposta la posizione della sash in pixel dall'inizio
/// dell'area client dello splitter.
pub fn splitter_set_position(splitter: NativeHandle, position: i32) {
    if let Some(paned) = splitter_of(splitter) {
        paned.set_position(position);
    }
}

/// Ridimensiona lo splitter: il `Paned` ridistribuisce i due
/// pannelli mantenendo la sash relativa.
pub fn splitter_resize(splitter: NativeHandle, width: u32, height: u32) {
    if let Some(paned) = splitter_of(splitter) {
        paned.set_size_request(width as i32, height as i32);
    }
}

/// Cambia l'asse della sash mantenendo gli stessi due pannelli.
pub fn splitter_set_orientation(splitter: NativeHandle, orientation: SplitterOrientation) {
    if let Some(paned) = splitter_of(splitter) {
        paned.set_orientation(gtk_orientation(orientation));
    }
}

/// Mostra (`true`) o nasconde (`false`) lo splitter con i due
/// pannelli.
pub fn splitter_set_visible(splitter: NativeHandle, visible: bool) {
    set_view_visible(splitter, visible);
}

/// Abilita/disabilita lo splitter: i due pannelli ereditano la
/// sensitività del `Paned`.
pub fn splitter_set_enabled(splitter: NativeHandle, enabled: bool) {
    set_view_enabled(splitter, enabled);
}

// ── Toolbar (GtkBox + pulsanti) ──────────────────────────────────────

/// Voce in attesa di `toolbar_realize`.
///
/// GTK4 non ha `GtkToolbar` (rimosso da GTK4): la striscia è un
/// `GtkBox` orizzontale e i pulsanti vengono costruiti quando la
/// toolbar viene realizzata, come fa la controparte appkit.
#[derive(Clone)]
enum ToolbarPendingItem {
    Separator,
    Tool {
        /// Id comando inviato alla frame al click.
        id: u16,
        /// Etichetta del pulsante (usata anche come tooltip).
        label: String,
        /// `true` per i pulsanti a due stati (`add_check_tool`).
        check: bool,
    },
}

/// Lato di dock della striscia AUI (valori identici ad AppKit).
pub const TOOLBAR_DOCK_TOP: u8 = 0;
pub const TOOLBAR_DOCK_BOTTOM: u8 = 1;
pub const TOOLBAR_DOCK_LEFT: u8 = 2;
pub const TOOLBAR_DOCK_RIGHT: u8 = 3;

/// Stato della striscia della toolbar.
struct ToolbarStorage {
    /// Contenitore orizzontale che ospita i pulsanti.
    strip: GtkBox,
    /// Frame a cui vengono instradati i click.
    frame_handle: NativeHandle,
    /// Voci registrate in attesa di realizzazione.
    pending: Vec<ToolbarPendingItem>,
    /// `true` per le strisce AuiToolBar (gripper "≡" in testa).
    with_gripper: bool,
    /// Id comando del gripper: il click alterna float/dock.
    gripper_id: u16,
    /// Altezza corrente della striscia (px).
    height: i32,
    /// Lato di dock corrente (una costante `TOOLBAR_DOCK_*`).
    dock_side: u8,
    /// Finestra fluttuante aperta dall'AuiToolBar (`Some` = staccata).
    floating: Option<Window>,
}

thread_local! {
    /// Handle nativo → striscia della toolbar.
    static TOOLBARS: RefCell<HashMap<NativeHandle, ToolbarStorage>> =
        RefCell::new(HashMap::new());
}

/// Crea la striscia della toolbar come figlio di `parent`.
///
/// Stessa signature della controparte appkit: `with_gripper` aggiunge
/// il grip "≡" in testa (invia `gripper_id` alla frame), `height` è
/// l'altezza della striscia.
pub fn create_toolbar_strip(
    parent: NativeHandle,
    frame_handle: NativeHandle,
    with_gripper: bool,
    gripper_id: u16,
    height: i32,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let strip = GtkBox::new(gtk4::Orientation::Horizontal, 4);
    let height = height.max(24);
    attach(
        parent,
        handle,
        strip.upcast_ref::<gtk4::Widget>(),
        Rect::new(0, 0, 0, height as u32),
    );
    TOOLBARS.with(|map| {
        map.borrow_mut().insert(
            handle,
            ToolbarStorage {
                strip,
                frame_handle,
                pending: Vec::new(),
                with_gripper,
                gripper_id,
                height,
                dock_side: TOOLBAR_DOCK_TOP,
                floating: None,
            },
        );
    });
    handle
}

/// Registra un pulsante (o pulsante a due stati) in attesa di realize.
pub fn toolbar_add_tool(toolbar_handle: NativeHandle, id: u16, label: &str, check: bool) {
    TOOLBARS.with(|map| {
        if let Some(storage) = map.borrow_mut().get_mut(&toolbar_handle) {
            storage.pending.push(ToolbarPendingItem::Tool {
                id,
                label: label.to_string(),
                check,
            });
        }
    });
}

/// Registra un separatore verticale in attesa di realize.
pub fn toolbar_add_separator(toolbar_handle: NativeHandle) {
    TOOLBARS.with(|map| {
        if let Some(storage) = map.borrow_mut().get_mut(&toolbar_handle) {
            storage.pending.push(ToolbarPendingItem::Separator);
        }
    });
}

/// Costruisce (o ricostruisce) i pulsanti della toolbar.
///
/// Ogni pulsante, al click, invia il proprio id comando alla frame
/// (`dispatch_command` → `Frame::register_command_handler`), stesso
/// percorso dei pulsanti standard. L'etichetta diventa anche il
/// tooltip: su Linux l'`ImageList` è uno stub, quindi le icone non
/// sono disponibili e i bottoni mostrano solo il testo.
pub fn toolbar_realize(toolbar_handle: NativeHandle) {
    TOOLBARS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(storage) = map.get_mut(&toolbar_handle) else {
            return;
        };
        // Rimuove la realizzazione precedente: `realize` è idempotente.
        while let Some(child) = storage.strip.first_child() {
            storage.strip.remove(&child);
        }
        let target = storage.frame_handle;
        // Grip "≡" in testa (strisce AuiToolBar): il click alterna
        // float/dock inoltrando `gripper_id` alla frame.
        if storage.with_gripper {
            let grip = Button::with_label("≡");
            grip.add_css_class("flat");
            grip.set_valign(gtk4::Align::Center);
            grip.set_tooltip_text(Some("Sposta/ancora la toolbar"));
            let frame = target;
            let gripper_id = storage.gripper_id;
            grip.connect_clicked(move |_| dispatch_command(frame, gripper_id));
            storage.strip.append(&grip);
        }
        for item in storage.pending.clone() {
            match item {
                ToolbarPendingItem::Separator => {
                    let sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
                    storage.strip.append(&sep);
                }
                ToolbarPendingItem::Tool { id, label, check } => {
                    let button: gtk4::Button = if check {
                        gtk4::ToggleButton::with_label(&label).upcast::<gtk4::Button>()
                    } else {
                        Button::with_label(&label)
                    };
                    // Stile piatto da toolbar, label centrata sull'altezza.
                    button.add_css_class("flat");
                    button.set_valign(gtk4::Align::Center);
                    button.set_tooltip_text(Some(&label));
                    let frame = target;
                    button.connect_clicked(move |_| dispatch_command(frame, id));
                    storage.strip.append(&button);
                }
            }
        }
    });
}

/// Dimensione corrente dell'area client della frame (`Fixed` content).
///
/// Restituisce `(larghezza, altezza)` in pixel — la controparte
/// appkit restituisce `(u32, u32)`, quindi i chiamanti cross-platform
/// devono adattare i tipi.
pub fn frame_client_size(frame_handle: NativeHandle) -> (i32, i32) {
    let allocated = FRAME_CONTENT.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .map(|content| (content.allocated_width(), content.allocated_height()))
            .unwrap_or((0, 0))
    });
    if allocated.0 > 0 && allocated.1 > 0 {
        return allocated;
    }
    // Prima del primo layout GTK lo stub della frame ha la size nota.
    frame_data(frame_handle)
        .map(|data| {
            let rect = data.borrow().stub.rect;
            (rect.width as i32, rect.height as i32)
        })
        .unwrap_or((0, 0))
}

/// Applica la geometria della striscia dockata sul lato corrente.
///
/// Se la striscia proviene da una finestra fluttuante (genitore non
/// `Fixed`) viene prima staccata e reinserita nel contenitore della
/// frame. Nessun effetto se la striscia è fluttuante: il popup ha
/// già la sua geometria.
fn toolbar_layout_docked(toolbar_handle: NativeHandle) {
    let plan = TOOLBARS.with(|map| {
        let map = map.borrow();
        let storage = map.get(&toolbar_handle)?;
        if storage.floating.is_some() {
            return None;
        }
        Some((
            storage.strip.clone(),
            storage.frame_handle,
            storage.height,
            storage.dock_side,
        ))
    });
    let Some((strip, frame, height, side)) = plan else {
        return;
    };
    let (frame_w, frame_h) = frame_client_size(frame);
    if frame_w <= 0 || frame_h <= 0 {
        return;
    }
    let rect = match side {
        TOOLBAR_DOCK_BOTTOM => {
            Rect::new(0, (frame_h - height).max(0), frame_w as u32, height as u32)
        }
        TOOLBAR_DOCK_LEFT => Rect::new(0, 0, height as u32, frame_h as u32),
        TOOLBAR_DOCK_RIGHT => {
            Rect::new((frame_w - height).max(0), 0, height as u32, frame_h as u32)
        }
        _ => Rect::new(0, 0, frame_w as u32, height as u32),
    };
    // Ai lati la striscia gira in verticale (colonna di pulsanti).
    let orientation = match side {
        TOOLBAR_DOCK_LEFT | TOOLBAR_DOCK_RIGHT => gtk4::Orientation::Vertical,
        _ => gtk4::Orientation::Horizontal,
    };
    strip.set_orientation(orientation);
    let widget: gtk4::Widget = strip.upcast();
    // Genitore non-`Fixed` = proveniente da un popup: da reinserire.
    let needs_put = match widget.parent() {
        None => true,
        Some(parent) => !parent.is::<Fixed>(),
    };
    if needs_put {
        // Stacca il widget (non il contenitore) dal vecchio genitore.
        if widget.parent().is_some() {
            widget.unparent();
        }
        if let Some(fixed) = fixed_for(frame) {
            fixed.put(&widget, rect.x as f64, rect.y as f64);
        }
    }
    set_view_frame(frame, toolbar_handle, rect);
}

/// Stacca la striscia da una eventuale finestra fluttuante.
/// Restituisce la finestra da chiudere (se c'era).
fn toolbar_undock_popup(toolbar_handle: NativeHandle) -> Option<Window> {
    TOOLBARS.with(|map| {
        let mut map = map.borrow_mut();
        let storage = map.get_mut(&toolbar_handle)?;
        storage.floating.take()
    })
}

/// Cambia l'altezza della striscia e ne riapplica la geometria.
pub fn toolbar_set_height(toolbar_handle: NativeHandle, height: i32) {
    TOOLBARS.with(|map| {
        if let Some(storage) = map.borrow_mut().get_mut(&toolbar_handle) {
            storage.height = height.max(24);
        }
    });
    toolbar_layout_docked(toolbar_handle);
}

/// Riapplica la geometria dockata (no-op se fluttuante).
pub fn toolbar_relayout_docked(toolbar_handle: NativeHandle) {
    toolbar_layout_docked(toolbar_handle);
}

/// Porta la striscia in primo piano (no-op: le strisce dockate non
/// hanno z-order proprio; il popup viene `present`ato).
pub fn toolbar_bring_to_front(toolbar_handle: NativeHandle) {
    TOOLBARS.with(|map| {
        if let Some(storage) = map.borrow().get(&toolbar_handle) {
            let floating = storage.floating.clone();
            if let Some(window) = floating {
                window.present();
            }
        }
    });
}

/// `true` se la striscia è staccata in una finestra fluttuante.
pub fn toolbar_is_floating(toolbar_handle: NativeHandle) -> bool {
    TOOLBARS.with(|map| {
        map.borrow()
            .get(&toolbar_handle)
            .map(|storage| storage.floating.is_some())
            .unwrap_or(false)
    })
}

/// Ancora la striscia al lato `side` (costanti `TOOLBAR_DOCK_*`).
pub fn toolbar_dock(toolbar_handle: NativeHandle, side: u8) {
    TOOLBARS.with(|map| {
        if let Some(storage) = map.borrow_mut().get_mut(&toolbar_handle) {
            storage.dock_side = side;
        }
    });
    // Chiude il popup (se aperto) e riporta la striscia nella frame.
    if let Some(popup) = toolbar_undock_popup(toolbar_handle) {
        toolbar_layout_docked(toolbar_handle);
        popup.close();
        return;
    }
    toolbar_layout_docked(toolbar_handle);
}

/// Alterna float/dock. Restituisce il nuovo stato (`true` = fluttuante).
pub fn toolbar_toggle_float(toolbar_handle: NativeHandle, x: i32, y: i32) -> bool {
    // Fase 1 — stacca: chiudi il popup e ri-docka.
    if let Some(popup) = toolbar_undock_popup(toolbar_handle) {
        toolbar_layout_docked(toolbar_handle);
        popup.close();
        return false;
    }
    // Fase 2 — fluttua: sposta la striscia in una finestra popup.
    let plan = TOOLBARS.with(|map| {
        let map = map.borrow();
        let storage = map.get(&toolbar_handle)?;
        Some((storage.strip.clone(), storage.frame_handle, storage.height))
    });
    let Some((strip, frame, height)) = plan else {
        return false;
    };
    let popup = Window::new();
    popup.set_title(Some("Toolbar"));
    let width = strip.allocated_width().max(240);
    popup.set_default_size(width, height);
    if let Some(frame_window) = frame_window_of(frame) {
        popup.set_transient_for(Some(&frame_window));
    }
    // Sgancia la striscia dal `Fixed` della frame prima di ospitarla.
    if strip.parent().is_some() {
        strip.unparent();
    }
    let child: gtk4::Widget = strip.upcast();
    popup.set_child(Some(&child));
    let handle = toolbar_handle;
    popup.connect_close_request(move |_| {
        // Chiusura manuale della finestra: riporta la striscia nella frame.
        if toolbar_undock_popup(handle).is_some() {
            toolbar_layout_docked(handle);
        }
        Propagation::Proceed
    });
    TOOLBARS.with(|map| {
        if let Some(storage) = map.borrow_mut().get_mut(&toolbar_handle) {
            storage.floating = Some(popup.clone());
        }
    });
    // GTK4 non permette di posizionare in modo assoluto le finestre:
    // il popup viene presentato transitorio alla frame (`x`/`y` ignorati).
    let _ = (x, y);
    popup.present();
    true
}

/// Stacca la striscia e la apre a `(x, y)` (no-op se già fluttuante).
pub fn toolbar_float_at(toolbar_handle: NativeHandle, x: i32, y: i32) {
    if !toolbar_is_floating(toolbar_handle) {
        toolbar_toggle_float(toolbar_handle, x, y);
    }
}

/// Le strisce GTK non hanno una controparte "nativa": sempre `false`.
pub fn toolbar_is_native(_toolbar_handle: NativeHandle) -> bool {
    false
}

/// Preferenza "nativa" senza effetto su GTK.
pub fn toolbar_set_prefer_native(_toolbar_handle: NativeHandle, _prefer: bool) {}

// ── HeaderCtrl (GtkBox di etichette) ────────────────────────────────

/// Larghezza del separatore verticale tra due colonne.
const HEADER_DIVIDER_W: i32 = 5;

/// Una colonna della striscia header.
struct HeaderColumnState {
    /// Testo della colonna (senza indicatore di ordinamento).
    title: String,
    /// Larghezza richiesta in pixel.
    width: u32,
}

/// Stato della striscia dell'header (`wxHeaderCtrl`).
///
/// GTK4 non ha un widget header equivalente a `SysHeader32`: la
/// striscia è un `GtkBox` orizzontale con un pulsante piatto per
/// colonna e un separatore verticale, analogamente alla toolbar.
struct HeaderStripState {
    /// Contenitore orizzontale che ospita le colonne.
    strip: GtkBox,
    /// Id base dell'header: i click usano `header_id + indice + 1`.
    header_id: u16,
    /// Frame a cui vengono instradati i click.
    frame_handle: NativeHandle,
    /// Colonne attualmente visualizzate.
    columns: Vec<HeaderColumnState>,
    /// Colonna ordinata (mostra la freccia `▾`).
    sort_column: Option<usize>,
}

thread_local! {
    /// Handle nativo → striscia dell'header.
    static HEADER_STRIPS: RefCell<HashMap<NativeHandle, HeaderStripState>> =
        RefCell::new(HashMap::new());
}

/// Crea la striscia dell'header come figlio di `parent`.
///
/// Stessa signature della controparte appkit: `header_id` è l'id base
/// usato per costruire i comandi dei click di colonna.
pub fn create_header_ctrl(
    parent: NativeHandle,
    header_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let strip = GtkBox::new(gtk4::Orientation::Horizontal, 0);
    attach(
        parent,
        handle,
        strip.upcast_ref::<gtk4::Widget>(),
        initial_rect,
    );
    let frame_handle = resolve_frame_handle(parent);
    HEADER_STRIPS.with(|map| {
        map.borrow_mut().insert(
            handle,
            HeaderStripState {
                strip,
                header_id,
                frame_handle,
                columns: Vec::new(),
                sort_column: None,
            },
        );
    });
    handle
}

/// Ricostruisce i pulsanti delle colonne dalla lista `columns`.
///
/// La ricostruzione è idempotente (come `toolbar_realize`) e mantiene
/// i connettori già creati coerenti con indici e larghezze correnti.
fn rebuild_header_strip(state: &mut HeaderStripState) {
    while let Some(child) = state.strip.first_child() {
        state.strip.remove(&child);
    }
    for (index, column) in state.columns.iter().enumerate() {
        let mut label = column.title.clone();
        if state.sort_column == Some(index) && !label.ends_with('▾') {
            label.push('▾');
        }
        let button = Button::with_label(&label);
        button.add_css_class("flat");
        button.set_halign(gtk4::Align::Start);
        button.set_size_request((column.width as i32 - HEADER_DIVIDER_W).max(24), -1);
        button.set_tooltip_text(Some(&column.title));
        // Click → comando `header_id + indice + 1`, come su macOS.
        let cmd = state.header_id.wrapping_add(index as u16 + 1);
        let frame = state.frame_handle;
        button.connect_clicked(move |_| dispatch_command(frame, cmd));
        state.strip.append(&button);
        let divider = gtk4::Separator::new(gtk4::Orientation::Vertical);
        state.strip.append(&divider);
    }
}

/// Inserisce una colonna in posizione `index`.
pub fn header_insert_column(handle: NativeHandle, index: usize, title: &str, width: u32) {
    HEADER_STRIPS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        let entry = HeaderColumnState {
            title: title.to_string(),
            width,
        };
        let position = index.min(state.columns.len());
        state.columns.insert(position, entry);
        rebuild_header_strip(state);
    });
}

/// Aggiorna la larghezza di una colonna.
pub fn header_set_column_width(handle: NativeHandle, column: usize, width: u32) {
    HEADER_STRIPS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        if let Some(entry) = state.columns.get_mut(column) {
            entry.width = width;
        }
        rebuild_header_strip(state);
    });
}

/// Imposta la colonna ordinata (e la relativa freccia `▾`).
pub fn header_set_sort_column(handle: NativeHandle, column: Option<usize>) {
    HEADER_STRIPS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        state.sort_column = column;
        rebuild_header_strip(state);
    });
}

/// Sposta/ridimensiona la striscia e ricostruisce le colonne.
pub fn header_sync_frame(handle: NativeHandle, parent: NativeHandle, rect: Rect) {
    set_view_frame(parent, handle, rect);
    HEADER_STRIPS.with(|map| {
        if let Some(state) = map.borrow_mut().get_mut(&handle) {
            rebuild_header_strip(state);
        }
    });
}

// ── AuiNotebook (tab) / PaneHost / Sash — GTK ──────────────────────

use crate::chrome::aui_manager::{AuiDockArt, AuiManager, AuiPaneButton, AuiSashSpec};
use crate::chrome::aui_tool_bar::AuiDockSide;

/// Larghezza minima della striscia laterale (sinistra/destra).
pub const SIDE_CAPTION_WIDTH: u32 = 72;

/// Larghezza della caption per i pannelli dockati a sinistra/destra.
pub fn aui_side_caption_width(caption_height: u32) -> u32 {
    caption_height.max(SIDE_CAPTION_WIDTH)
}

/// Estrae `(r, g, b)` da un colore `0xRRGGBB`.
fn rgb_of(colour: u32) -> (u8, u8, u8) {
    (
        ((colour >> 16) & 0xFF) as u8,
        ((colour >> 8) & 0xFF) as u8,
        (colour & 0xFF) as u8,
    )
}

thread_local! {
    /// Provider CSS per nome widget: uno solo per nome, così il
    /// contenuto viene sostituito invece di accumulare provider.
    static CSS_PROVIDERS: RefCell<HashMap<String, CssProvider>> =
        RefCell::new(HashMap::new());
}

/// Applica un blocco CSS a un widget usando un nome (`#id`) univoco.
fn apply_widget_css(widget: &gtk4::Widget, name: &str, css: &str) {
    widget.set_widget_name(name);
    CSS_PROVIDERS.with(|map| {
        let mut map = map.borrow_mut();
        if let Some(provider) = map.get(name) {
            provider.load_from_data(css);
            return;
        }
        let provider = CssProvider::new();
        provider.load_from_data(css);
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        map.insert(name.to_string(), provider);
    });
}

// ── Tab (AuiNotebook) ─────────────────────────────────────────────

thread_local! {
    static TABS: RefCell<HashMap<NativeHandle, gtk4::Notebook>> =
        RefCell::new(HashMap::new());
    static TAB_CHILDREN: RefCell<HashMap<NativeHandle, Vec<NativeHandle>>> =
        RefCell::new(HashMap::new());
    static TAB_HANDLERS: RefCell<HashMap<NativeHandle, Box<dyn FnMut(usize)>>> =
        RefCell::new(HashMap::new());
    /// Tab per cui il segnale `switch_page` è già collegato.
    static TAB_WIRED: RefCell<Vec<NativeHandle>> = RefCell::new(Vec::new());
    /// Evita loop quando la selezione è impostata programmaticamente.
    static TAB_SYNCING: Cell<bool> = const { Cell::new(false) };
}

/// Crea un `Notebook` (controllo a tab) dentro il contenitore `parent`.
pub fn create_tab(parent: NativeHandle, rect: Rect) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let handle = alloc_widget_handle();
    let notebook = gtk4::Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_show_border(true);
    attach(parent, handle, notebook.upcast_ref::<gtk4::Widget>(), rect);
    TABS.with(|map| map.borrow_mut().insert(handle, notebook));
    TAB_CHILDREN.with(|map| map.borrow_mut().insert(handle, Vec::new()));
    handle
}

fn notebook_of(tab_handle: NativeHandle) -> Option<gtk4::Notebook> {
    TABS.with(|map| map.borrow().get(&tab_handle).cloned())
}

/// Aggiunge una pagina al tab. Restituisce l'indice (`-1` se il tab
/// non esiste o il figlio non è registrato).
pub fn tab_add_page(tab_handle: NativeHandle, title: &str, child_handle: NativeHandle) -> i32 {
    let Some(notebook) = notebook_of(tab_handle) else {
        return -1;
    };
    let child = match widget_of(child_handle) {
        Some(child) => child,
        None => return -1,
    };
    // Stacca il figlio dal vecchio contenitore prima di ospitarlo.
    if child.parent().is_some() {
        child.unparent();
    }
    let label = Label::new(Some(title));
    let index = notebook.append_page(&child, Some(&label)) as i32;
    child.set_size_request(-1, -1);
    TAB_CHILDREN.with(|map| {
        if let Some(children) = map.borrow_mut().get_mut(&tab_handle) {
            children.push(child_handle);
        }
    });
    index
}

/// Rimuove la pagina `index`. Restituisce `false` se fuori range.
pub fn tab_remove_page(tab_handle: NativeHandle, index: usize) -> bool {
    let Some(notebook) = notebook_of(tab_handle) else {
        return false;
    };
    if index as u32 >= notebook.n_pages() {
        return false;
    }
    // Sgancia il figlio così GTK non lo distrugge con la pagina.
    let child_handle = TAB_CHILDREN.with(|map| {
        map.borrow()
            .get(&tab_handle)
            .and_then(|c| c.get(index).copied())
    });
    if let Some(handle) = child_handle {
        if let Some(child) = widget_of(handle) {
            if child.parent().is_some() {
                child.unparent();
            }
        }
    }
    notebook.remove_page(Some(index as u32));
    TAB_CHILDREN.with(|map| {
        if let Some(children) = map.borrow_mut().get_mut(&tab_handle) {
            if index < children.len() {
                children.remove(index);
            }
        }
    });
    true
}

fn set_tab_current(tab_handle: NativeHandle, index: usize) {
    let Some(notebook) = notebook_of(tab_handle) else {
        return;
    };
    if index as u32 >= notebook.n_pages() {
        return;
    }
    if notebook.current_page() == Some(index as u32) {
        return;
    }
    TAB_SYNCING.with(|flag| flag.set(true));
    notebook.set_current_page(Some(index as u32));
    TAB_SYNCING.with(|flag| flag.set(false));
}

/// Seleziona la pagina `index` (usato per allineare lo stato).
pub fn tab_select_index(tab_handle: NativeHandle, index: usize) {
    set_tab_current(tab_handle, index);
}

/// Seleziona la pagina `index` (usato da `AuiNotebook::set_selection`).
pub fn tab_set_selection(tab_handle: NativeHandle, index: usize) {
    set_tab_current(tab_handle, index);
}

/// Il layout delle pagine è gestito dal `Notebook`: nessun intervento.
pub fn tab_layout_page(_tab_handle: NativeHandle, _child_handle: NativeHandle) {}

/// Collega il callback notificato al cambio di selezione.
pub fn set_tab_selection_handler(tab_handle: NativeHandle, handler: Box<dyn FnMut(usize)>) {
    TAB_HANDLERS.with(|map| map.borrow_mut().insert(tab_handle, handler));
    if TAB_WIRED.with(|v| v.borrow().contains(&tab_handle)) {
        return;
    }
    TAB_WIRED.with(|v| v.borrow_mut().push(tab_handle));
    let Some(notebook) = notebook_of(tab_handle) else {
        return;
    };
    notebook.connect_switch_page(move |_notebook, _page, page_num| {
        if TAB_SYNCING.with(|flag| flag.get()) {
            return;
        }
        TAB_HANDLERS.with(|map| {
            if let Some(handler) = map.borrow_mut().get_mut(&tab_handle) {
                handler(page_num as usize);
            }
        });
    });
}

/// `ImageList` è uno stub su Linux: la chiamata è compatibile ma
/// non aggiunge icone alle tab.
pub fn tab_set_image_list(_tab_handle: NativeHandle, _image_list_handle: NativeHandle) {}

/// Icona di pagina non supportata (stub `ImageList`).
pub fn tab_set_page_image(_tab_handle: NativeHandle, _page_index: usize, _image_index: i32) {}

/// Cambia il testo della tab `index`. Restituisce `false` se fuori range.
pub fn tab_set_page_text(tab_handle: NativeHandle, index: usize, title: &str) -> bool {
    let Some(notebook) = notebook_of(tab_handle) else {
        return false;
    };
    let Some(child) = notebook.nth_page(Some(index as u32)) else {
        return false;
    };
    notebook.set_tab_label_text(&child, title);
    true
}

// ── Pane host (caption + contenuto) ───────────────────────────────

/// Stato di un pannello dockato gestito dall'AUI manager.
struct PaneHostState {
    host_handle: NativeHandle,
    /// Contenitore verticale: caption in alto, contenuto sotto.
    host: GtkBox,
    caption: GtkBox,
    title: Label,
    content: Fixed,
    /// Bottoni della caption: `(command id, widget, tipo)`.
    buttons: Vec<(u16, Button, AuiPaneButton)>,
    child_handle: NativeHandle,
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    dock_side: AuiDockSide,
    floatable: bool,
    pinned: bool,
}

thread_local! {
    static PANE_HOSTS: RefCell<HashMap<(NativeHandle, String), PaneHostState>> =
        RefCell::new(HashMap::new());
    static PANE_FLOATS: RefCell<HashMap<(NativeHandle, String), Window>> =
        RefCell::new(HashMap::new());
    static PANE_BTN_COMMANDS: RefCell<HashMap<u16, (NativeHandle, String, AuiPaneButton)>> =
        RefCell::new(HashMap::new());
    /// Contatore dei command id assegnati ai bottoni delle caption.
    static PANE_BTN_NEXT: Cell<u16> = const { Cell::new(9200) };
    static PANE_ACTION: RefCell<HashMap<NativeHandle, (*mut AuiManager, Frame)>> =
        RefCell::new(HashMap::new());
    /// Striscie toolbar sotto il controllo dell'AUI.
    static AUI_MANAGED_TOOLBARS: RefCell<Vec<NativeHandle>> = RefCell::new(Vec::new());
}

/// Glifo del bottone della caption (stessi unicode della controparte).
fn button_glyph(kind: &AuiPaneButton, pinned: bool) -> &'static str {
    match kind {
        AuiPaneButton::Close => "✕",
        AuiPaneButton::Maximize => "□",
        AuiPaneButton::Minimize => "–",
        AuiPaneButton::Pin => {
            if pinned {
                "◉"
            } else {
                "◎"
            }
        }
        AuiPaneButton::Custom(_) => "•",
    }
}

/// Applica i colori dell'`AuiDockArt` alla striscia della caption.
fn apply_pane_caption_art(state: &PaneHostState, art: &AuiDockArt) {
    let (r, g, b) = rgb_of(art.active_caption());
    // Testo scuro su fondo chiaro, chiaro su fondo scuro.
    let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    let text = if luminance > 140.0 {
        "#000000"
    } else {
        "#ffffff"
    };
    let name = format!("ruwx-aui-caption-{}", state.host_handle);
    let css = format!(
        "#{} {{ background-color: rgb({}, {}, {}); color: {}; padding: 0 4px; }}",
        name, r, g, b, text
    );
    apply_widget_css(state.caption.upcast_ref::<gtk4::Widget>(), &name, &css);
    state.title.add_css_class("heading");
}

/// Ricostruisce i bottoni della caption (command id assegnati in ordine).
fn rebuild_caption_buttons(
    state: &mut PaneHostState,
    frame_handle: NativeHandle,
    pane_name: &str,
    buttons: &[AuiPaneButton],
    pinned: bool,
) {
    for (_, button, _) in state.buttons.drain(..) {
        state.caption.remove(&button);
    }
    // I comandi delle versioni precedenti non servono più.
    PANE_BTN_COMMANDS.with(|map| {
        map.borrow_mut()
            .retain(|_, (frame, name, _)| !(*frame == frame_handle && name == pane_name));
    });
    for kind in buttons {
        let command = PANE_BTN_NEXT.with(|counter| {
            let value = counter.get();
            counter.set(value.wrapping_add(1));
            value
        });
        PANE_BTN_COMMANDS.with(|map| {
            map.borrow_mut()
                .insert(command, (frame_handle, pane_name.to_string(), kind.clone()));
        });
        let button = Button::with_label(button_glyph(kind, pinned));
        button.add_css_class("flat");
        button.set_focus_on_click(false);
        button.set_valign(gtk4::Align::Center);
        let frame = frame_handle;
        button.connect_clicked(move |_| dispatch_command(frame, command));
        state.caption.append(&button);
        state.buttons.push((command, button, kind.clone()));
    }
}

/// Crea (o aggiorna) la "host" di un pannello: striscia con titolo e
/// bottoni + area contenuto con il figlio riparentato dentro.
pub fn ensure_aui_pane_host(
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    pane_name: &str,
    child_handle: NativeHandle,
    dock_side: AuiDockSide,
    caption: &str,
    buttons: &[AuiPaneButton],
    art: &AuiDockArt,
    floatable: bool,
    pinned: bool,
) -> NativeHandle {
    if !ensure_ready() {
        return alloc_widget_handle();
    }
    let key = (frame_handle, pane_name.to_string());
    if PANE_HOSTS.with(|map| map.borrow().contains_key(&key)) {
        PANE_HOSTS.with(|map| {
            let mut map = map.borrow_mut();
            let state = map.get_mut(&key).expect("pane host GTK");
            state.dock_side = dock_side;
            state.floatable = floatable;
            state.pinned = pinned;
            state.parent_handle = parent_handle;
            state.frame_handle = frame_handle;
            state.title.set_text(caption);
            apply_pane_caption_art(state, art);
            let same_buttons = state.buttons.len() == buttons.len()
                && state
                    .buttons
                    .iter()
                    .zip(buttons.iter())
                    .all(|((_, _, current), wanted)| current == wanted);
            if same_buttons {
                for (_, button, kind) in &state.buttons {
                    if matches!(kind, AuiPaneButton::Pin) {
                        button.set_label(button_glyph(kind, pinned));
                    }
                }
            } else {
                rebuild_caption_buttons(state, frame_handle, pane_name, buttons, pinned);
            }
        });
        return PANE_HOSTS.with(|map| map.borrow()[&key].host_handle);
    }

    let host_handle = alloc_widget_handle();
    let host = GtkBox::new(gtk4::Orientation::Vertical, 0);
    host.set_hexpand(true);
    host.set_vexpand(true);

    let caption_box = GtkBox::new(gtk4::Orientation::Horizontal, 4);
    caption_box.set_size_request(-1, art.caption_height() as i32);
    caption_box.set_margin_start(4);
    caption_box.set_margin_end(2);

    let title = Label::new(Some(caption));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    caption_box.append(&title);

    let content = Fixed::new();
    content.set_hexpand(true);
    content.set_vexpand(true);

    host.append(&caption_box);
    host.append(&content);

    attach(
        parent_handle,
        host_handle,
        host.upcast_ref::<gtk4::Widget>(),
        Rect::new(0, 0, 120, 120),
    );

    if let Some(child) = widget_of(child_handle) {
        // Stacca il figlio (non il contenitore) dal vecchio genitore.
        if child.parent().is_some() {
            child.unparent();
        }
        child.set_size_request(96, 64);
        content.put(&child, 0.0, 0.0);
        record_parent(child_handle, host_handle);
    }

    // Doppio click sulla caption → alterna float/dock (come il drag
    // della controparte macOS).
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(gdk::BUTTON_PRIMARY);
    let click_frame = frame_handle;
    let click_name = pane_name.to_string();
    gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
        if n_press != 2 {
            return;
        }
        let floating = PANE_FLOATS.with(|map| {
            map.borrow()
                .contains_key(&(click_frame, click_name.clone()))
        });
        if floating {
            request_pane_dock(click_frame, &click_name);
        } else {
            request_pane_float(click_frame, &click_name);
        }
    });
    caption_box.add_controller(gesture);

    let mut state = PaneHostState {
        host_handle,
        host,
        caption: caption_box,
        title,
        content,
        buttons: Vec::new(),
        child_handle,
        frame_handle,
        parent_handle,
        dock_side,
        floatable,
        pinned,
    };
    apply_pane_caption_art(&state, art);
    rebuild_caption_buttons(&mut state, frame_handle, pane_name, buttons, pinned);
    PANE_HOSTS.with(|map| map.borrow_mut().insert(key, state));
    host_handle
}

/// Posiziona la host nel contenitore e ridimensiona caption + figlio.
pub fn position_aui_pane_host(
    parent: NativeHandle,
    host_handle: NativeHandle,
    rect: Rect,
    caption_height: u32,
) {
    set_view_frame(parent, host_handle, rect);
    let width = rect.width as i32;
    let height = (rect.height as i32 - caption_height as i32).max(0);
    PANE_HOSTS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(state) = map.values_mut().find(|s| s.host_handle == host_handle) else {
            return;
        };
        state.caption.set_size_request(-1, caption_height as i32);
        if let Some(child) = widget_of(state.child_handle) {
            child.set_size_request(width.max(0), height);
            state.content.move_(&child, 0.0, 0.0);
        }
    });
}

/// Riallinea gli interni della host (caption + figlio).
pub fn layout_pane_host(host_handle: NativeHandle, caption_height: u32) {
    let plan = PANE_HOSTS.with(|map| {
        let map = map.borrow();
        let state = map.values().find(|s| s.host_handle == host_handle)?;
        Some((
            state.caption.clone(),
            state.child_handle,
            state.content.clone(),
        ))
    });
    let Some((caption, child_handle, content)) = plan else {
        return;
    };
    caption.set_size_request(-1, caption_height as i32);
    if let Some(child) = widget_of(child_handle) {
        child.set_size_request(content.allocated_width(), content.allocated_height());
        content.move_(&child, 0.0, 0.0);
    }
}

/// Handle della host di un pannello (se creato).
pub fn aui_pane_host_handle(frame_handle: NativeHandle, pane_name: &str) -> Option<NativeHandle> {
    PANE_HOSTS.with(|map| {
        map.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .map(|state| state.host_handle)
    })
}

/// Sincronizza lo stato "pinnato" e il relativo glifo.
pub fn sync_aui_pane_host_pin(frame_handle: NativeHandle, pane_name: &str, pinned: bool) {
    PANE_HOSTS.with(|map| {
        let mut map = map.borrow_mut();
        let Some(state) = map.get_mut(&(frame_handle, pane_name.to_string())) else {
            return;
        };
        state.pinned = pinned;
        for (_, button, kind) in &state.buttons {
            if matches!(kind, AuiPaneButton::Pin) {
                button.set_label(button_glyph(kind, pinned));
            }
        }
    });
}

/// L'auto-hide GTK si limita alla geometria gestita dal manager
/// (`AUTO_HIDE_STRIP`): nessuna rappresentazione nativa aggiuntiva.
pub fn sync_aui_pane_auto_hide(
    _frame_handle: NativeHandle,
    _pane_name: &str,
    _auto_hide: bool,
    _auto_hidden: bool,
) {
}

/// Comandi dei bottoni caption registrati per la frame indicata.
pub fn pane_button_commands_for_frame(
    frame_handle: NativeHandle,
) -> Vec<(u16, String, AuiPaneButton)> {
    PANE_BTN_COMMANDS.with(|map| {
        map.borrow()
            .iter()
            .filter(|(_, (frame, _, _))| *frame == frame_handle)
            .map(|(command, (_, name, kind))| (*command, name.clone(), kind.clone()))
            .collect()
    })
}

/// Registra il puntatore all'`AuiManager` per la frame (le azioni dei
/// bottoni caption e del doppio click lo richiamano).
pub fn register_aui_pane_action_target(
    frame_handle: NativeHandle,
    manager: *mut AuiManager,
    frame: Frame,
) {
    PANE_ACTION.with(|map| {
        map.borrow_mut().insert(frame_handle, (manager, frame));
    });
}

/// Rimuove il target delle azioni per la frame.
pub fn unregister_aui_pane_action_target(frame_handle: NativeHandle) {
    PANE_ACTION.with(|map| {
        map.borrow_mut().remove(&frame_handle);
    });
}

fn pane_action_for(frame_handle: NativeHandle) -> Option<(*mut AuiManager, Frame)> {
    PANE_ACTION.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .map(|(manager, frame)| (*manager, frame.clone()))
    })
}

/// Stacca il pannello e lo apre in una finestra transitoria.
pub fn float_aui_pane_host(frame_handle: NativeHandle, pane_name: &str, rect: &Rect, title: &str) {
    if !ensure_ready() {
        return;
    }
    let key = (frame_handle, pane_name.to_string());
    let plan = PANE_HOSTS.with(|map| {
        let map = map.borrow();
        let state = map.get(&key)?;
        Some(state.host.clone())
    });
    let Some(host) = plan else {
        return;
    };
    if PANE_FLOATS.with(|map| map.borrow().contains_key(&key)) {
        return;
    }

    let popup = Window::new();
    popup.set_title(Some(title));
    popup.set_default_size(rect.width.max(160) as i32, rect.height.max(120) as i32);
    if let Some(frame_window) = frame_window_of(frame_handle) {
        popup.set_transient_for(Some(&frame_window));
    }
    // Sgancia la host dal contenitore della frame prima di ospitarla.
    if host.parent().is_some() {
        host.unparent();
    }
    let host_widget: gtk4::Widget = host.upcast();
    popup.set_child(Some(&host_widget));
    PANE_FLOATS.with(|map| map.borrow_mut().insert(key, popup.clone()));

    let frame = frame_handle;
    let name = pane_name.to_string();
    popup.connect_close_request(move |_| {
        // La chiusura manuale ri-aggancia il pannello alla frame.
        request_pane_dock(frame, &name);
        Propagation::Proceed
    });
    // GTK4 non permette di posizionare in modo assoluto le finestre:
    // `rect.x`/`rect.y` vengono ignorati dal window manager.
    popup.present();
}

/// Riporta il pannello fluttuante dentro la frame.
pub fn dock_aui_pane_host(frame_handle: NativeHandle, pane_name: &str) {
    let key = (frame_handle, pane_name.to_string());
    if let Some(popup) = PANE_FLOATS.with(|map| map.borrow_mut().remove(&key)) {
        popup.close();
    }
    let plan = PANE_HOSTS.with(|map| {
        let map = map.borrow();
        let state = map.get(&key)?;
        Some((state.host.clone(), state.host_handle, state.parent_handle))
    });
    let Some((host, host_handle, parent)) = plan else {
        return;
    };
    if host.parent().is_some() {
        host.unparent();
    }
    if let Some(fixed) = fixed_for(parent) {
        fixed.put(&host, 0.0, 0.0);
    }
    record_parent(host_handle, parent);
}

/// Chiede al manager di staccare il pannello.
pub fn request_pane_float(frame_handle: NativeHandle, pane_name: &str) {
    let Some((manager, frame)) = pane_action_for(frame_handle) else {
        return;
    };
    // GTK4 non espone facilmente il punto del cursore a livello
    // globale: la finestra appare con la posizione scelta dal WM.
    unsafe {
        if let Some(manager) = manager.as_mut() {
            manager.float_pane(&frame, pane_name, 64, 64);
        }
    }
}

/// Chiede al manager di ri-agganciare il pannello.
pub fn request_pane_dock(frame_handle: NativeHandle, pane_name: &str) {
    let Some((manager, frame)) = pane_action_for(frame_handle) else {
        return;
    };
    unsafe {
        if let Some(manager) = manager.as_mut() {
            manager.dock_pane(&frame, pane_name);
        }
    }
}

/// Chiede al manager di comprimere/espandere il pannello auto-hide.
pub fn request_pane_toggle_auto_hide(frame_handle: NativeHandle, pane_name: &str) {
    let Some((manager, frame)) = pane_action_for(frame_handle) else {
        return;
    };
    unsafe {
        if let Some(manager) = manager.as_mut() {
            manager.toggle_auto_hide_pane(&frame, pane_name);
        }
    }
}

/// Chiede al manager di ridimensionare il pannello dopo un drag del sash.
pub fn request_pane_sash_resize(
    frame_handle: NativeHandle,
    pane_name: &str,
    delta: i32,
    vertical: bool,
) {
    if delta == 0 {
        return;
    }
    let Some((manager, frame)) = pane_action_for(frame_handle) else {
        return;
    };
    unsafe {
        if let Some(manager) = manager.as_mut() {
            manager.apply_sash_resize(&frame, pane_name, delta, vertical);
        }
    }
}

/// Comandi bottoni caption: nessuna differenza rispetto ad altre piattaforme.
pub fn pane_button_commands_available() -> bool {
    true
}

// ── Sash (handle di ridimensionamento) ────────────────────────────

/// Sash GTK: una striscia colorata trascinabile.
#[derive(Clone)]
struct SashView {
    handle: NativeHandle,
    widget: gtk4::Widget,
}

thread_local! {
    static SASHES: RefCell<HashMap<(NativeHandle, String), SashView>> =
        RefCell::new(HashMap::new());
}

/// Allinea i sash agli spigoli dei pannelli (crea/rimuove/posiziona).
pub fn sync_aui_sashes(parent: NativeHandle, frame_handle: NativeHandle, specs: &[AuiSashSpec]) {
    if !ensure_ready() {
        return;
    }
    // Rimuove gli sash che non corrispondono più ad alcuna specifica.
    SASHES.with(|map| {
        let mut map = map.borrow_mut();
        let stale: Vec<_> = map
            .keys()
            .filter(|(frame, name)| {
                *frame == frame_handle && !specs.iter().any(|spec| &spec.pane_name == name)
            })
            .cloned()
            .collect();
        for key in &stale {
            if let Some(view) = map.remove(key) {
                if view.widget.parent().is_some() {
                    view.widget.unparent();
                }
            }
        }
    });

    for spec in specs {
        let key = (frame_handle, spec.pane_name.clone());
        let (r, g, b) = rgb_of(spec.sash_color);
        let name = format!("ruwx-aui-sash-{}-{}", frame_handle, spec.pane_name);
        let css = format!(
            "#{} {{ background-color: rgb({}, {}, {}); }}",
            name, r, g, b
        );
        let vertical = spec.vertical;
        let rect = spec.rect;

        // Il controller di trascinamento viene aggiunto solo alla creazione,
        // così ogni sash ne possiede esattamente uno.
        let (view, created) = match SASHES.with(|map| map.borrow().get(&key).cloned()) {
            Some(view) => {
                apply_widget_css(&view.widget, &name, &css);
                set_view_frame(parent, view.handle, rect);
                (view, false)
            }
            None => {
                let handle = alloc_widget_handle();
                let strip = GtkBox::new(
                    if vertical {
                        gtk4::Orientation::Vertical
                    } else {
                        gtk4::Orientation::Horizontal
                    },
                    0,
                );
                let widget: gtk4::Widget = strip.upcast();
                apply_widget_css(&widget, &name, &css);
                widget.set_cursor_from_name(Some(if vertical { "ew-resize" } else { "ns-resize" }));
                attach(parent, handle, &widget, rect);
                let view = SashView { handle, widget };
                SASHES.with(|map| map.borrow_mut().insert(key.clone(), view.clone()));
                (view, true)
            }
        };

        if created {
            // Trascinamento → delta rispetto al precedente aggiornamento.
            let drag = gtk4::GestureDrag::new();
            let applied = Rc::new(Cell::new(0.0f64));
            let frame = frame_handle;
            let pane_name = spec.pane_name.clone();
            drag.connect_drag_update(move |_gesture, offset_x, offset_y| {
                let current = if vertical { offset_x } else { offset_y };
                let previous = applied.get();
                applied.set(current);
                let delta = (current - previous).round() as i32;
                if delta != 0 {
                    request_pane_sash_resize(frame, &pane_name, delta, vertical);
                }
            });
            view.widget.add_controller(drag);
        }
    }
}

// ── Toolbar sotto controllo AUI ───────────────────────────────────

/// Segna la striscia come gestita dall'AUI manager.
pub fn toolbar_set_managed_by_aui(toolbar_handle: NativeHandle, managed: bool) {
    AUI_MANAGED_TOOLBARS.with(|list| {
        let mut list = list.borrow_mut();
        if managed {
            if !list.contains(&toolbar_handle) {
                list.push(toolbar_handle);
            }
        } else {
            list.retain(|handle| *handle != toolbar_handle);
        }
    });
}

/// Riallinea la striscia dopo il layout dell'AUI.
pub fn toolbar_sync_host_layout(toolbar_handle: NativeHandle) {
    toolbar_relayout_docked(toolbar_handle);
}
