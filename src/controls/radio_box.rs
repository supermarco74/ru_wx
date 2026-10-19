//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxRadioBox — a group of mutually-exclusive radio buttons in a box.
//!
//! On Windows we create a static `BUTTON` control with the
//! `BS_GROUPBOX` style as the visual frame, then create N child
//! `BS_AUTORADIOBUTTON` controls inside it. The first radio carries
//! `WS_GROUP` so the radio buttons are mutually exclusive.
//!
//! On macOS we delegate to `appkit::create_radio_box`, which draws an
//! `NSBox` as the visual frame plus N `NSButton` radio controls. The
//! radios share the same superview and action selector, so AppKit
//! links them into a mutually-exclusive group automatically.
//!
//! Use [`RadioBox::new`] with a slice of labels. The currently-selected
//! index is reported by [`RadioBox::get_selection`] and can be set with
//! [`RadioBox::set_selection`]. Subscribe to [`RadioBox::on_select`] for
//! change notifications.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use crate::platform::declared;
use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 constants ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
const BM_GETCHECK: u32 = 0x00F0;
#[cfg(target_os = "windows")]
const BM_SETCHECK: u32 = 0x00F1;
#[cfg(target_os = "windows")]
const BST_CHECKED: usize = 1;
#[cfg(target_os = "windows")]
const BST_UNCHECKED: usize = 0;
#[cfg(target_os = "windows")]
const BST_CHECKED_VALUE: isize = 1;

#[cfg(target_os = "windows")]
const BS_GROUPBOX: u32 = 0x0007;
#[cfg(target_os = "windows")]
const BS_AUTORADIOBUTTON: u32 = 0x0009;
#[cfg(target_os = "windows")]
const WS_GROUP: u32 = 0x0002_0000;

// ── Costanti di layout (ramo macOS) ───────────────────────────────────
//
// Devono restare in sincronia con `appkit::create_radio_box`
// (`appkit_controls.rs`): la stessa griglia verticale di righe.

#[cfg(target_os = "macos")]
const ROW_HEIGHT: i32 = 22;
#[cfg(target_os = "macos")]
const PADDING_TOP: i32 = 18;
#[cfg(target_os = "macos")]
const PADDING_X: i32 = 10;

// ── Inner type ─────────────────────────────────────────────────────────

struct RadioBoxInner {
    #[cfg(target_os = "windows")]
    box_hwnd: HWND,
    #[cfg(target_os = "windows")]
    radio_hwnds: Vec<HWND>,
    /// Handle AppKit dell'`NSBox` (la cornice visiva del gruppo).
    #[cfg(target_os = "macos")]
    box_handle: crate::platform::NativeHandle,
    /// Handle AppKit dei singoli `NSButton` radio.
    #[cfg(target_os = "macos")]
    radio_handles: Vec<crate::platform::NativeHandle>,
    /// Id di comando assegnati ai singoli radio (per `on_select`).
    #[cfg(target_os = "macos")]
    radio_ids: Vec<u16>,
    /// Handle del genitore (content view) su cui piazzati box e radio.
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    rect: Rect,
    /// Whether each radio is enabled (mirrors the visual state).
    enabled: bool,
    visible: bool,
}

// ── Public type ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RadioBox {
    inner: Rc<RefCell<RadioBoxInner>>,
}

impl RadioBox {
    /// Create a new radio-box with the given `labels`, as a child of
    /// `parent`. The first label is initially selected unless
    /// `initial_selection` is provided.
    pub fn new<W: Window>(parent_in: &W, title: &str, labels: &[&str]) -> Self {
        Self::with_selection(parent_in, title, labels, 0)
    }

    /// Create a new radio-box with the given `labels` and an initial
    /// selection. `initial_selection` is clamped into the valid range.
    pub fn with_selection<W: Window>(
        parent_in: &W,
        title: &str,
        labels: &[&str],
        initial_selection: usize,
    ) -> Self {
        let id = next_control_id();
        let row_height = 22;
        let box_padding_top = 18;
        #[cfg(target_os = "windows")]
        let box_padding_x = 10;
        let box_padding_bottom = 8;
        let box_width = 200;
        let box_height = box_padding_top + (labels.len() as i32) * row_height + box_padding_bottom;

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let (box_hwnd, radio_hwnds) = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_class = to_wide("BUTTON");
            let wide_title = to_wide(title);

            let bx = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_title.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_GROUPBOX,
                0,
                0,
                box_width,
                box_height,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            let mut radios = Vec::with_capacity(labels.len());
            for (i, label) in labels.iter().enumerate() {
                let wide_label = to_wide(label);
                let style = if i == 0 {
                    WS_CHILD | WS_VISIBLE | BS_AUTORADIOBUTTON | WS_GROUP
                } else {
                    WS_CHILD | WS_VISIBLE | BS_AUTORADIOBUTTON
                };
                let radio_id = next_control_id();
                let rh = CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    wide_label.as_ptr(),
                    style,
                    box_padding_x,
                    box_padding_top + (i as i32) * row_height,
                    box_width - 2 * box_padding_x,
                    row_height,
                    parent,
                    radio_id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                radios.push(rh);
            }
            (bx, radios)
        };

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let (box_handle, radio_handles, radio_ids) = crate::platform::appkit::create_radio_box(
            parent_handle,
            title,
            labels,
            Rect::new(0, 0, box_width as u32, box_height as u32),
        );

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent_in, title, labels);

        let radio_box = RadioBox {
            inner: Rc::new(RefCell::new(RadioBoxInner {
                #[cfg(target_os = "windows")]
                box_hwnd,
                #[cfg(target_os = "windows")]
                radio_hwnds,
                #[cfg(target_os = "macos")]
                box_handle,
                #[cfg(target_os = "macos")]
                radio_handles,
                #[cfg(target_os = "macos")]
                radio_ids,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
                rect: Rect::new(0, 0, box_width as u32, box_height as u32),
                enabled: true,
                visible: true,
            })),
        };

        // Registro dell'inspector (§6 del piano): il guscio
        // (groupbox/NSBox) è il nodo `radio` dell'albero e il selettore
        // rende pilotabile `POST /event` con azione `select`. Va fatto
        // prima della selezione iniziale: `set_selection` /
        // `set_selector` sono no-op su un handle non marcato.
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let shell = radio_box.inner.borrow().native_handle();
            declared::mark(shell, "radio");
            declared::set_selector(shell, {
                let radio_box = radio_box.clone();
                Rc::new(move |index| radio_box.set_selection(index))
            });
        }

        // Apply initial selection. Clamp the requested index.
        let initial = initial_selection.min(labels.len().saturating_sub(1));
        radio_box.set_selection(initial);

        radio_box
    }

    /// Return the index of the currently selected radio button.
    /// Returns `None` if no radio is selected.
    pub fn get_selection(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            let hwnds = self.inner.borrow().radio_hwnds.clone();
            for (i, hwnd) in hwnds.iter().enumerate() {
                // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
                let result = unsafe { SendMessageW(*hwnd, BM_GETCHECK, 0, 0) };
                if result == BST_CHECKED_VALUE {
                    return Some(i);
                }
            }
        }

        // macOS: leggiamo lo stato di checked di ogni NSButton radio.
        #[cfg(target_os = "macos")]
        {
            let handles = self.inner.borrow().radio_handles.clone();
            for (i, handle) in handles.iter().enumerate() {
                if crate::platform::appkit::radio_is_selected(*handle) {
                    return Some(i);
                }
            }
        }

        None
    }

    /// Set the currently selected radio button. `index` is the
    /// zero-based position of the radio to select; out-of-range
    /// indices leave all radios unchecked.
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        {
            let hwnds = self.inner.borrow().radio_hwnds.clone();
            for (i, hwnd) in hwnds.iter().enumerate() {
                let flag = if i == index {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                };
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SendMessageW(*hwnd, BM_SETCHECK, flag, 0);
                }
            }
        }

        // macOS: selezioniamo l'indice richiesto e deselezioniamo gli
        // altri (equivalente di BM_SETCHECK per ogni radio del gruppo).
        #[cfg(target_os = "macos")]
        {
            let handles = self.inner.borrow().radio_handles.clone();
            for (i, handle) in handles.iter().enumerate() {
                crate::platform::appkit::radio_set_selected(*handle, i == index);
            }
        }

        // Stato letto da `read_state` come `checked_index` (§6): lo
        // aggiorniamo qui, così resta coerente anche senza `on_select`.
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        declared::set_selection(self.inner.borrow().native_handle(), index);
    }

    /// Return the number of radio buttons in the group.
    pub fn len(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow().radio_hwnds.len()
        }
        #[cfg(target_os = "macos")]
        {
            self.inner.borrow().radio_handles.len()
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        0
    }

    /// Return `true` if the radio-box has no radio buttons.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Register a callback that fires when the user picks a different
    /// radio button. The callback receives the index of the newly
    /// selected radio.
    ///
    /// Because all radio buttons share a single `WM_COMMAND` dispatch
    /// path (by control id), this installs a handler on the **first**
    /// radio button's id. If you have other command handlers installed
    /// on that id, only the last one registered will be active.
    pub fn on_select<F: FnMut(usize) + 'static>(&self, frame: &Frame, callback: F) {
        #[cfg(target_os = "windows")]
        {
            let hwnds = self.inner.borrow().radio_hwnds.clone();
            if hwnds.is_empty() {
                return;
            }
            // We register the handler against the first radio's id; the
            // WndProc dispatches WM_COMMAND by id, so any radio click
            // within this group will fire it. We then look up the
            // currently selected index and call the user callback.
            // Note: this is best-effort — a more robust implementation
            // would intercept WM_COMMAND at the parent WndProc and
            // match on the id with a per-id handler map, but that
            // requires changes to frame.rs. For now this works for
            // the common case where a RadioBox owns its radio ids.
            //
            // Strategy: install the same handler on every radio id so
            // we get notified for any of them.
            //
            // `F: FnMut` is not `Clone`, so we wrap the callback in a
            // `Rc<RefCell<>>` to share a single instance across all
            // per-id closures.
            let callback = std::rc::Rc::new(std::cell::RefCell::new(callback));
            let shell = self.inner.borrow().native_handle();
            for (i, _hwnd) in hwnds.iter().enumerate() {
                let id = self.get_radio_id(i);
                let hwnds_clone = hwnds.clone();
                let cb_holder = callback.clone();
                let cb = move || {
                    let mut cb = cb_holder.borrow_mut();
                    for (j, h) in hwnds_clone.iter().enumerate() {
                        // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
                        let r = unsafe { SendMessageW(*h, BM_GETCHECK, 0, 0) };
                        if r == BST_CHECKED_VALUE {
                            // Anche il click dell'utente aggiorna
                            // `checked_index` (§6) sul guscio.
                            declared::set_selection(shell, j);
                            cb(j);
                            return;
                        }
                    }
                };
                frame.register_command_handler(id, Box::new(cb));
            }
        }

        // macOS: stesso identico schema del ramo Windows — registriamo
        // lo stesso handler su ogni radio id e rileggiamo lo stato
        // selezionato. `appkit::dispatch_command` legge la mappa
        // `command_handlers` del Frame, quindi il dispatch è già
        // predisposto.
        #[cfg(target_os = "macos")]
        {
            let handles = self.inner.borrow().radio_handles.clone();
            let ids = self.inner.borrow().radio_ids.clone();
            if ids.is_empty() {
                return;
            }
            let callback = Rc::new(RefCell::new(callback));
            let shell = self.inner.borrow().native_handle();
            for id in ids {
                let handles_clone = handles.clone();
                let cb_holder = callback.clone();
                let cb = move || {
                    let mut cb = cb_holder.borrow_mut();
                    for (j, h) in handles_clone.iter().enumerate() {
                        if crate::platform::appkit::radio_is_selected(*h) {
                            // Anche il click dell'utente aggiorna
                            // `checked_index` (§6) sul guscio.
                            declared::set_selection(shell, j);
                            cb(j);
                            return;
                        }
                    }
                };
                frame.register_command_handler(id, Box::new(cb));
            }
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = (frame, callback);
        }
    }

    /// Selection with [`RadioBoxEvent`](crate::RadioBoxEvent) payload (`wxRadioBoxEvent`).
    pub fn on_radio_event<F: FnMut(&crate::RadioBoxEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        self.on_select(frame, move |sel| f(&crate::RadioBoxEvent::new(sel)));
    }

    /// Get the control id of the Nth radio button. This is the id
    /// stored in the underlying `HMENU` of the radio control.
    #[cfg(target_os = "windows")]
    fn get_radio_id(&self, index: usize) -> u16 {
        // We allocated ids in sequence; the first radio's id is the
        // one returned from next_control_id() when the box was created,
        // then the rest follow. Because next_control_id() is global,
        // we can't recompute them after the fact — but we stored
        // them implicitly via CreateWindowExW. To recover them, we use
        // GetDlgCtrlID.
        let hwnds = self.inner.borrow().radio_hwnds.clone();
        if let Some(&hwnd) = hwnds.get(index) {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe { GetDlgCtrlID(hwnd) as u16 }
        } else {
            0
        }
    }

    /// Get the id of the outer group-box (the visual frame).
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ───────────────────────────────────────────────────────
//
// `RadioBox` is a composite: it owns the groupbox frame plus all the
// radio children. Sizers position the whole composite as a single
// widget using the groupbox hwnd.

/// Supporto di layout del ramo macOS: ridisegna il frame dell'`NSBox`
/// e di ogni radio secondo `self.rect`.
#[cfg(target_os = "macos")]
impl RadioBoxInner {
    fn reposition_mac_radios(&self) {
        crate::platform::appkit::set_view_frame(self.parent_handle, self.box_handle, self.rect);
        let radio_w = (self.rect.width as i32 - 2 * PADDING_X).max(1) as u32;
        for (i, handle) in self.radio_handles.iter().enumerate() {
            let radio_rect = Rect::new(
                PADDING_X + self.rect.x,
                PADDING_TOP + (i as i32) * ROW_HEIGHT + self.rect.y,
                radio_w,
                ROW_HEIGHT as u32,
            );
            crate::platform::appkit::set_view_frame(self.parent_handle, *handle, radio_rect);
        }
    }
}

impl Widget for RadioBoxInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.box_hwnd as isize
        }
        #[cfg(target_os = "macos")]
        {
            self.box_handle
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        0
    }

    fn set_position(&mut self, x: i32, y: i32) {
        let old_x = self.rect.x;
        let old_y = self.rect.y;
        let dx = x - old_x;
        let dy = y - old_y;
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.box_hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
            // Move each radio by the same delta.
            for hwnd in &self.radio_hwnds {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                GetWindowRect(*hwnd, &mut rect);
                // Convert screen coords to parent coords
                let parent = GetParent(*hwnd);
                let mut pt = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                ScreenToClient(parent, &mut pt);
                MoveWindow(
                    *hwnd,
                    pt.x + dx,
                    pt.y + dy,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    1,
                );
            }
        }

        // macOS: i radio sono figli della content view (stesso
        // superview del box), quindi vanno spostati della stessa
        // delta, esattamente come nel ramo Windows.
        #[cfg(target_os = "macos")]
        {
            let _ = (dx, dy);
            self.reposition_mac_radios();
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.box_hwnd,
                self.rect.x,
                self.rect.y,
                w as i32,
                h as i32,
                1,
            );
            // The radios are stacked vertically (one per row, 10 px
            // side padding — see `with_selection`), so each radio is
            // stretched to span the new width; rows keep their
            // current positions and heights (best-effort).
            let radio_w = ((w as i32) - 20).max(1);
            for hwnd in &self.radio_hwnds {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                GetWindowRect(*hwnd, &mut rect);
                let parent = GetParent(*hwnd);
                let mut pt = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                ScreenToClient(parent, &mut pt);
                MoveWindow(*hwnd, pt.x, pt.y, radio_w, rect.bottom - rect.top, 1);
            }
        }

        // macOS: box ridimensionato + radio riadattati alla nuova
        // larghezza (stessa griglia di righe).
        #[cfg(target_os = "macos")]
        {
            self.reposition_mac_radios();
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.box_hwnd, if visible { SW_SHOW } else { SW_HIDE });
            for hwnd in &self.radio_hwnds {
                ShowWindow(*hwnd, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_visible(self.box_handle, visible);
            for handle in &self.radio_handles {
                crate::platform::appkit::set_view_visible(*handle, visible);
            }
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.box_hwnd, if enabled { 1 } else { 0 });
            for hwnd in &self.radio_hwnds {
                EnableWindow(*hwnd, if enabled { 1 } else { 0 });
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_enabled(self.box_handle, enabled);
            for handle in &self.radio_handles {
                crate::platform::appkit::set_view_enabled(*handle, enabled);
            }
        }
    }
}

#[cfg(all(test, any(target_os = "windows", target_os = "macos")))]
mod tests {
    /// Il guscio va marcato `radio` (§6): è il nodo che `/find?role=radio`
    /// trova come gruppo e da cui `read_state` legge `checked_index`.
    /// Su macOS richiede il main thread AppKit (widget nativi reali),
    /// non disponibile sotto `cargo test`.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn radio_box_registra_ruolo_selezione_e_selettore() {
        use super::*;
        use crate::window::panel::Panel;

        let frame = Frame::builder()
            .with_title("radio-box-test")
            .with_size(320, 240)
            .build();
        let panel = Panel::new(&frame);
        let radio_box = RadioBox::with_selection(&panel, "Scelta", &["Uno", "Due", "Tre"], 2);
        let shell = radio_box.inner.borrow().native_handle();

        // Ruolo e selezione iniziale (letti da `role_for_class`/`read_state`).
        assert_eq!(declared::role(shell), Some("radio"));
        assert_eq!(declared::selection(shell), Some(2));

        // `inject_event: select` dell'inspector invoca il selettore,
        // che applica la selezione ai radio nativi.
        assert!(declared::select(shell, 1));
        assert_eq!(radio_box.get_selection(), Some(1));
        assert_eq!(declared::selection(shell), Some(1));

        // Anche una `set_selection` diretta tiene in sincronia
        // `checked_index`.
        radio_box.set_selection(0);
        assert_eq!(declared::selection(shell), Some(0));
    }
}
