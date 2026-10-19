//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Alternative notebook controls: [`Listbook`], [`Choicebook`],
//! [`Treebook`], [`Toolbook`].
//!
//! All four are siblings of [`crate::containers::tab::Tab`] — they are
//! "notebooks" of pages, each backed by a [`Panel`], but instead of
//! a `SysTabControl32` tab strip they use a different widget to
//! show the page labels:
//!
//! * [`Listbook`] — pages are entries in a [`ListBox`] (vertical
//!   strip on the left, content area on the right).
//! * [`Choicebook`] — pages are entries in a [`Choice`] drop-down
//!   (combobox on top, content area below).
//! * [`Treebook`] — pages are nodes in a [`TreeCtrl`] (hierarchical
//!   outline on the left, content area on the right).
//! * [`Toolbook`] — pages are buttons in a [`ToolBar`] (icon strip
//!   on the top, content area below).
//!
//! # Design
//!
//! Il book possiede l'elenco delle pagine, la pagina corrente e la
//! logica show/hide; la striscia ("strip") è un widget separato,
//! creato e posizionato dall'utente. Il metodo `bind` di ciascun book
//! (es. [`Listbook::bind`]) collega i due mondi in un colpo solo:
//!
//! * la selezione della strip cambia la pagina (`strip → book.select`);
//! * il book si registra all'inspector con il ruolo §6 di
//!   `PIANO_PORTABILITA_COMPONENTI.md` (`tabs` per Listbook/Choicebook/
//!   Toolbook, `tree` per Treebook), con `read_state.selection` e il
//!   selettore di `inject_event: select`;
//! * la strip viene allineata alla pagina già corrente.
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! use ru_wx::containers::book::Listbook;
//! let frame = Frame::builder().with_title("book").build();
//! let list = ListBox::new(&frame);
//! list.append("Page 1");
//! let book = Listbook::new();
//! book.add_page("Page 1", Panel::new(&frame));
//! // Wiring strip ↔ book + ruolo inspector (§6) in un colpo solo:
//! book.bind(&frame, &list);
//! ```
//!
//! Il wiring resta possibile anche a mano, senza `bind`, chiamando
//! `strip.on_selection_change(...)` → `book.select(idx)`: `bind` non
//! cambia la semantica, la incapsula (F3 del piano).
//!
//! ## Note d'uso
//!
//! * Chiama `bind` dopo aver aggiunto pagine al book e item alla strip:
//!   il book allinea la strip alla pagina corrente solo in quel momento.
//! * `bind` registra il *suo* handler sulla strip: una
//!   `on_selection_change`/`on_tool_clicked` registrata sulla stessa
//!   strip *dopo* `bind` sostituisce il cablaggio. Per ricevere
//!   l'evento di cambio pagina usa `book.on_selection_change`.
//! * La callback di `on_selection_change` viene invocata con il
//!   prestito mutabile dello stato attivo: dentro di essa non
//!   leggere/scrivere lo stato del book (panico `RefCell`), usa
//!   l'indice che ti passa.
//!
//! # Cross-platform behaviour
//!
//! Su Windows il wiring sfrutta le notifiche native (`WM_COMMAND` /
//! `WM_NOTIFY`); su macOS la stessa API passa dai dispatcher del
//! `Frame` (gli handler sono registrati sul frame in entrambi i casi),
//! quindi il comportamento è omogeneo. Sui sistemi non supportati le
//! chiamate sono no-op e `current_selection` resta lo stato puro Rust.

use std::cell::RefCell;
use std::rc::Rc;

use crate::chrome::tool_bar::ToolBar;
use crate::controls::choice::Choice;
use crate::controls::list_box::ListBox;
use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};
use crate::platform::declared;
use crate::platform::NativeHandle;
use crate::window::frame::Frame;
use crate::window::panel::Panel;

// ─── BookCore (the shared state machine) ───────────────────────────────

/// Page bookkeeping shared by all book variants.
struct BookCore {
    /// Each entry is `(label, panel)`. Panels are stored in the
    /// order they are added. The book shows only the panel at
    /// `selected` (when `Some`) and hides the rest.
    pages: Vec<(String, Panel)>,
    /// Currently selected page (0-based). `None` means "no page is
    /// selected" (e.g. a freshly-built book with no pages).
    selected: Option<usize>,
    /// User callback for selection change.
    on_selection_change: Option<Box<dyn FnMut(usize)>>,
    /// Handle nativo della strip, registrato nel registro `declared`
    /// da `bind`: `select` aggiorna la `selection` che `read_state`
    /// espone all'inspector (§6 del piano).
    strip_handle: Option<NativeHandle>,
}

impl BookCore {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            selected: None,
            on_selection_change: None,
            strip_handle: None,
        }
    }

    fn add_page(&mut self, label: &str, panel: Panel) {
        let index = self.pages.len();
        self.pages.push((label.to_string(), panel));
        if self.selected.is_none() {
            self.select(index);
        }
    }

    fn select(&mut self, index: usize) {
        if index >= self.pages.len() {
            return;
        }
        if Some(index) == self.selected {
            return;
        }
        // Hide the previously-selected panel, show the new one.
        if let Some(prev) = self.selected {
            if let Some((_, p)) = self.pages.get(prev) {
                p.hide();
            }
        }
        self.selected = Some(index);
        if let Some((_, p)) = self.pages.get(index) {
            p.show();
        }
        // Stato dell'inspector aggiornato prima della callback, così
        // `read_state.selection` è già coerente quando l'evento arriva.
        if let Some(handle) = self.strip_handle {
            declared::set_selection(handle, index);
        }
        if let Some(cb) = self.on_selection_change.as_mut() {
            cb(index);
        }
    }

    fn current_selection(&self) -> Option<usize> {
        self.selected
    }

    fn set_on_selection_change<F: FnMut(usize) + 'static>(&mut self, f: F) {
        self.on_selection_change = Some(Box::new(f));
    }

    fn page_count(&self) -> usize {
        self.pages.len()
    }
}

/// Registra la strip nel registro `declared` per
/// l'inspector (§6 del piano): ruolo (`tabs`/`tree`), selettore di
/// `inject_event: select` e — via `strip_handle` sul core — la
/// `selection` letta da `read_state` a ogni cambio pagina.
fn register_strip(
    core: &Rc<RefCell<BookCore>>,
    handle: NativeHandle,
    role: &'static str,
    selector: Rc<dyn Fn(usize)>,
) {
    declared::mark(handle, role);
    declared::set_selector(handle, selector);
    let mut book = core.borrow_mut();
    book.strip_handle = Some(handle);
    if let Some(selection) = book.selected {
        declared::set_selection(handle, selection);
    }
}

// ─── Listbook ──────────────────────────────────────────────────────────

/// A notebook backed by a [`ListBox`]: call [`Listbook::bind`] to
/// wire the listbox strip to the book (selection → page, plus the
/// inspector role/state of §6).
///
/// Sibling of `wxListbook` from wxWidgets.
pub struct Listbook {
    core: Rc<RefCell<BookCore>>,
}

impl Listbook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    /// Callback di cambio pagina (indice 0-based). Una sola alla
    /// volta: la nuova sostituisce la precedente. Vedi le note del
    /// modulo sul prestito `RefCell`.
    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }

    /// Cablaggio strip ↔ book (C4/F3): la selezione della `ListBox`
    /// seleziona la pagina; il book si registra all'inspector come
    /// `tabs` con `read_state.selection` e `inject_event: select`.
    ///
    /// Sostituisce le callback `on_selection_change` già registrate
    /// sulla strip: per l'evento pagina usa [`Listbook::on_selection_change`].
    pub fn bind(&self, frame: &Frame, strip: &ListBox) {
        // strip → book: la selezione della lista cambia la pagina.
        {
            let core = Rc::clone(&self.core);
            let strip_for_cb = strip.clone();
            strip.on_selection_change(frame, move || {
                if let Some(index) = strip_for_cb.get_selection() {
                    core.borrow_mut().select(index);
                }
            });
        }
        // book → strip: selettore per `inject_event: select`.
        let handle = strip.as_widget_ref().borrow().native_handle();
        let strip_for_selector = strip.clone();
        let core_for_selector = Rc::clone(&self.core);
        register_strip(&self.core, handle, "tabs", {
            Rc::new(move |index| {
                strip_for_selector.set_selection(index);
                core_for_selector.borrow_mut().select(index);
            })
        });
        // Allinea la strip alla pagina già corrente (selezionata da
        // `add_page` prima del `bind`). Il valore va estratto prima del
        // blocco `if let`: GTK emette `selected-rows-changed` in modo
        // sincrono in `set_selection` e il `borrow()` dell'if-let
        // resterebbe vivo durante il rientro nell'handler.
        let current = self.core.borrow().current_selection();
        if let Some(index) = current {
            strip.set_selection(index);
        }
    }
}

impl Default for Listbook {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Choicebook ────────────────────────────────────────────────────────

/// A notebook backed by a [`Choice`] drop-down: call
/// [`Choicebook::bind`] to wire the combobox strip to the book.
///
/// Sibling of `wxChoicebook` from wxWidgets.
pub struct Choicebook {
    core: Rc<RefCell<BookCore>>,
}

impl Choicebook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    /// Callback di cambio pagina (indice 0-based). Una sola alla
    /// volta: la nuova sostituisce la precedente. Vedi le note del
    /// modulo sul prestito `RefCell`.
    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }

    /// Cablaggio strip ↔ book (C4/F3): la selezione della [`Choice`]
    /// seleziona la pagina; il book si registra all'inspector come
    /// `tabs` con `read_state.selection` e `inject_event: select`.
    ///
    /// Sostituisce le callback `on_selection_change` già registrate
    /// sulla strip: per l'evento pagina usa
    /// [`Choicebook::on_selection_change`].
    pub fn bind(&self, frame: &Frame, strip: &Choice) {
        // strip → book: la selezione della combo cambia la pagina.
        {
            let core = Rc::clone(&self.core);
            let strip_for_cb = strip.clone();
            strip.on_selection_change(frame, move || {
                if let Some(index) = strip_for_cb.get_selection() {
                    core.borrow_mut().select(index);
                }
            });
        }
        // book → strip: selettore per `inject_event: select`.
        let handle = strip.as_widget_ref().borrow().native_handle();
        let strip_for_selector = strip.clone();
        let core_for_selector = Rc::clone(&self.core);
        register_strip(&self.core, handle, "tabs", {
            Rc::new(move |index| {
                strip_for_selector.set_selection(index);
                core_for_selector.borrow_mut().select(index);
            })
        });
        if let Some(index) = self.core.borrow().current_selection() {
            strip.set_selection(index);
        }
    }
}

impl Default for Choicebook {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Treebook ──────────────────────────────────────────────────────────

/// A notebook backed by a [`TreeCtrl`]: call [`Treebook::bind`] with
/// the node↔page mapping, so tree selections become page selections
/// (and vice-versa for `inject_event: select`).
///
/// Sibling of `wxTreebook` from wxWidgets.
pub struct Treebook {
    core: Rc<RefCell<BookCore>>,
}

impl Treebook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    /// Callback di cambio pagina (indice 0-based). Una sola alla
    /// volta: la nuova sostituisce la precedente. Vedi le note del
    /// modulo sul prestito `RefCell`.
    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }

    /// Cablaggio strip ↔ book (C4/F3): il nodo selezionato nella
    /// [`TreeCtrl`] seleziona la pagina; il book si registra
    /// all'inspector come `tree` con `read_state.selection` e
    /// `inject_event: select`.
    ///
    /// * `page_of` traduce un nodo in un indice pagina (dà `None` per
    ///   i nodi che non sono pagine, es. i nodi foglia decorativi);
    /// * `item_of` fa l'inverso, per `inject_event: select` e per
    ///   allineare la strip.
    ///
    /// Sostituisce la callback `on_selection_change` registrata sulla
    /// strip: per l'evento pagina usa
    /// [`Treebook::on_selection_change`].
    pub fn bind(
        &self,
        frame: &Frame,
        strip: &TreeCtrl,
        page_of: impl Fn(&TreeItem) -> Option<usize> + 'static,
        item_of: impl Fn(usize) -> Option<TreeItem> + 'static,
    ) {
        // strip → book: il nodo selezionato cambia la pagina.
        {
            let core = Rc::clone(&self.core);
            let strip = strip.clone();
            strip.on_selection_change(frame, move |item| {
                if let Some(item) = item {
                    if let Some(index) = page_of(&item) {
                        core.borrow_mut().select(index);
                    }
                }
            });
        }
        // Allinea la strip alla pagina già corrente.
        let initial = self.core.borrow().current_selection();
        if let Some(index) = initial {
            if let Some(item) = item_of(index) {
                strip.select_item(item);
            }
        }
        // book → strip: selettore per `inject_event: select`.
        let handle = strip.as_widget_ref().borrow().native_handle();
        let strip_for_selector = strip.clone();
        let core_for_selector = Rc::clone(&self.core);
        register_strip(&self.core, handle, "tree", {
            Rc::new(move |index| {
                if let Some(item) = item_of(index) {
                    strip_for_selector.select_item(item);
                }
                core_for_selector.borrow_mut().select(index);
            })
        });
    }
}

impl Default for Treebook {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Toolbook ──────────────────────────────────────────────────────────

/// A notebook backed by a [`ToolBar`]: call [`Toolbook::bind`] with
/// the tool-id↔page mapping, so clicking a tool selects the page.
///
/// Sibling of `wxToolbook` from wxWidgets.
pub struct Toolbook {
    core: Rc<RefCell<BookCore>>,
}

impl Toolbook {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(BookCore::new())),
        }
    }

    pub fn add_page(&self, label: &str, panel: Panel) {
        self.core.borrow_mut().add_page(label, panel);
    }

    pub fn select(&self, index: usize) {
        self.core.borrow_mut().select(index);
    }

    pub fn current_selection(&self) -> Option<usize> {
        self.core.borrow().current_selection()
    }

    /// Callback di cambio pagina (indice 0-based). Una sola alla
    /// volta: la nuova sostituisce la precedente. Vedi le note del
    /// modulo sul prestito `RefCell`.
    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, f: F) {
        self.core.borrow_mut().set_on_selection_change(f);
    }

    pub fn page_count(&self) -> usize {
        self.core.borrow().page_count()
    }

    /// Cablaggio strip ↔ book (C4/F3): il tool cliccato nella
    /// [`ToolBar`] seleziona la pagina; il book si registra
    /// all'inspector come `tabs` con `read_state.selection` e
    /// `inject_event: select`.
    ///
    /// * `page_of` traduce l'id del tool in un indice pagina (dà
    ///   `None` per gli id che non sono pagine, es. separator/azioni);
    /// * la toolbar non espone una selezione programmatica dei tool:
    ///   `inject_event: select` cambia pagina ma non evidenzia il
    ///   relativo pulsante.
    ///
    /// Sostituisce le callback `on_tool_clicked` già registrate sulla
    /// strip: per l'evento pagina usa
    /// [`Toolbook::on_selection_change`].
    pub fn bind<F: Fn(u16) -> Option<usize> + 'static>(
        &self,
        frame: &Frame,
        strip: &ToolBar,
        page_of: F,
    ) {
        // strip → book: il tool cliccato cambia la pagina.
        {
            let core = Rc::clone(&self.core);
            strip.on_tool_clicked(frame, move |tool_id| {
                if let Some(index) = page_of(tool_id) {
                    core.borrow_mut().select(index);
                }
            });
        }
        // book → strip: selettore per `inject_event: select`.
        let handle = strip.as_widget_ref().borrow().native_handle();
        let core_for_selector = Rc::clone(&self.core);
        register_strip(&self.core, handle, "tabs", {
            Rc::new(move |index| {
                core_for_selector.borrow_mut().select(index);
            })
        });
    }
}

impl Default for Toolbook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Handle fittizio: `declared` è `thread_local`, quindi ogni test
    /// lavora sul proprio registro senza interferenze.
    const STRIP: NativeHandle = 0x0FF3_0001;

    #[test]
    fn register_strip_scrive_ruolo_e_selettore() {
        let core = Rc::new(RefCell::new(BookCore::new()));
        let fired: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let fired_for_selector = Rc::clone(&fired);
        register_strip(
            &core,
            STRIP,
            "tabs",
            Rc::new(move |index| {
                fired_for_selector.set(Some(index));
            }),
        );
        // Ruolo §6 letto da `role_for_class` prima dell'euristica.
        assert_eq!(declared::role(STRIP), Some("tabs"));
        // `inject_event: select` dell'inspector invoca il selettore.
        assert!(declared::select(STRIP, 3));
        assert_eq!(fired.get(), Some(3));
    }

    #[test]
    fn register_strip_allinea_la_selection_iniziale() {
        let core = Rc::new(RefCell::new(BookCore::new()));
        core.borrow_mut().selected = Some(2);
        register_strip(&core, STRIP, "tree", Rc::new(|_| {}));
        // `read_state` dell'inspector legge questa selection.
        assert_eq!(declared::role(STRIP), Some("tree"));
        assert_eq!(declared::selection(STRIP), Some(2));
    }

    /// Su macOS la creazione della `Frame` richiede il main thread
    /// AppKit, che `cargo test` non garantisce: il test gira solo
    /// sulle piattaforme senza questo vincolo.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn select_aggiorna_la_selection_e_non_rifa_la_callback() {
        // Pagine reali: il core show/hide i `Panel` nativi.
        let frame = Frame::builder()
            .with_title("book-test")
            .with_size(120, 80)
            .build();
        let core = Rc::new(RefCell::new(BookCore::new()));
        core.borrow_mut().add_page("One", Panel::new(&frame));
        core.borrow_mut().add_page("Two", Panel::new(&frame));
        // La prima `add_page` diventa la corrente.
        assert_eq!(core.borrow().current_selection(), Some(0));

        let fires: Rc<Cell<usize>> = Rc::new(Cell::new(0));
        let fires_for_cb = Rc::clone(&fires);
        core.borrow_mut()
            .set_on_selection_change(move |_| fires_for_cb.set(fires_for_cb.get() + 1));

        // `bind` registra la strip: la selection iniziale viene
        // allineata a `read_state` dell'inspector.
        register_strip(&core, STRIP, "tabs", Rc::new(|_| {}));
        assert_eq!(declared::selection(STRIP), Some(0));

        // Guard: la stessa selezione non ricambia nulla né firea
        // la callback (doppio evento nativo o chiamata ripetuta).
        core.borrow_mut().select(0);
        assert_eq!(fires.get(), 0);

        // Un cambio reale aggiorna `read_state` prima della callback.
        core.borrow_mut().select(1);
        assert_eq!(declared::selection(STRIP), Some(1));
        assert_eq!(fires.get(), 1);
    }
}
