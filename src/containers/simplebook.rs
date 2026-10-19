//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `Simplebook` — contenitore di pagine senza tab (`wxSimplebook`).
//!
//! Puro Rust, nessun FFI: le pagine sono widget generici ([`WidgetRef`])
//! di cui il contenitore comanda la visibilità con `set_visible`, come i
//! book di [`crate::containers::book`] (architettura ereditata). A
//! differenza di quei book — che hanno un controllo nativo proprio — qui
//! il contenitore espone all'inspector il ruolo dichiarato `tabs` (§6 di
//! `PIANO_PORTABILITA_COMPONENTI.md`): il ruolo, la selezione corrente e
//! il selettore di `inject_event` vivono nel registro
//! `crate::platform::declared` agganciato al guscio [`Panel`].
//!
//! Uso consueto (il guscio va creato *prima* delle pagine, perché ne
//! occupa l'area):
//!
//! ```no_run
//! use ru_wx::{App, Frame, Panel, Simplebook};
//!
//! let app = App::new();
//! let frame = Frame::builder()
//!     .with_title("Demo")
//!     .with_size(400, 300)
//!     .build();
//! let book = Simplebook::new(&frame);
//! let page = Panel::new_child(&frame);
//! book.add_page("Details", page.as_widget_ref());
//! ```
//!
//! Cross-platform behaviour: identico su Windows e macOS — solo la
//! pagina corrente è visibile, `selection()` parte da `0` e torna a `0`
//! finché non viene aggiunta la prima pagina.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::widget::{WidgetRef, Window};
use crate::platform::declared;
use crate::platform::NativeHandle;
use crate::window::panel::Panel;

/// Stato puro Rust del contenitore: pagine e selezione corrente.
///
/// È separato da [`Simplebook`] così la logica di selezione — l'unica
/// degna di test unitari — gira anche senza creare widget nativi.
struct SimplebookCore {
    pages: Vec<(String, WidgetRef)>,
    selected: Option<usize>,
}

impl SimplebookCore {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            selected: None,
        }
    }

    /// Aggiunge una pagina in coda.
    ///
    /// La *prima* pagina diventa subito la corrente (viene mostrata);
    /// le successive vengono nascoste, perché solo la pagina corrente
    /// resta visibile. Ritorna la selezione risultante se è cambiata.
    fn add_page(&mut self, name: &str, page: WidgetRef) -> Option<usize> {
        self.pages.push((name.to_string(), page));
        if self.selected.is_none() {
            self.select(self.pages.len() - 1);
            return self.selected;
        }
        if let Some((_, page)) = self.pages.last() {
            page.borrow_mut().set_visible(false);
        }
        None
    }

    /// Seleziona la pagina `index`: nasconde la precedente e mostra la
    /// nuova. Ritorna `true` se la selezione è cambiata.
    fn select(&mut self, index: usize) -> bool {
        if index >= self.pages.len() || Some(index) == self.selected {
            return false;
        }
        if let Some(prev) = self.selected {
            if let Some((_, page)) = self.pages.get(prev) {
                page.borrow_mut().set_visible(false);
            }
        }
        self.selected = Some(index);
        if let Some((_, page)) = self.pages.get(index) {
            page.borrow_mut().set_visible(true);
        }
        true
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.pages.iter().position(|(label, _)| label == name)
    }
}

/// Contenitore di pagine senza tab: solo la pagina corrente è visibile.
pub struct Simplebook {
    core: Rc<RefCell<SimplebookCore>>,
    /// Guscio (`RuWxPanel`) che fa da nodo con ruolo dichiarato `tabs`
    /// nell'albero dell'inspector e ne rappresenta l'area.
    shell: Panel,
    handle: NativeHandle,
}

impl Simplebook {
    /// Crea il contenitore come figlio di `parent`.
    ///
    /// Il guscio occupa l'area della prima pagina aggiunta; il pannello
    /// `RuWxPanel` non disegna sfondo, quindi non copre le pagine.
    pub fn new<W: Window>(parent: &W) -> Self {
        let shell = Panel::new_child(parent);
        let handle = shell.hwnd();
        // Ruolo dichiarato: senza classe nativa propria è l'unico modo
        // perché `/find?role=tabs` trovi il contenitore (§6 del piano).
        declared::mark(handle, "tabs");
        let core = Rc::new(RefCell::new(SimplebookCore::new()));
        declared::set_selector(handle, {
            let core = Rc::clone(&core);
            Rc::new(move |index| apply_selection(&core, handle, index))
        });
        Self {
            core,
            shell,
            handle,
        }
    }

    /// Aggiunge una pagina al contenitore.
    pub fn add_page(&self, name: &str, page: WidgetRef) {
        // Il guscio segue l'area della prima pagina, così i bounds
        // dell'inspector coincidono con quelli del book.
        if self.core.borrow().pages.is_empty() {
            let rect = page.borrow().rect();
            self.shell.set_position(rect.x, rect.y);
            self.shell.set_size(rect.width, rect.height);
        }
        let selection = self.core.borrow_mut().add_page(name, page);
        if let Some(selection) = selection {
            declared::set_selection(self.handle, selection);
        }
    }

    /// Mostra la pagina il cui nome è `name` (nessun effetto se
    /// il nome non esiste).
    pub fn show_page(&self, name: &str) {
        let index = self.core.borrow().index_of(name);
        if let Some(index) = index {
            self.set_selection(index);
        }
    }

    /// Seleziona la pagina `index` (indice ignoto/fuori range: nessun
    /// effetto, come nei book di [`crate::containers::book`]).
    pub fn set_selection(&self, index: usize) {
        apply_selection(&self.core, self.handle, index);
    }

    /// Indice della pagina corrente; `0` se non ci sono pagine.
    pub fn selection(&self) -> usize {
        self.core.borrow().selected.unwrap_or(0)
    }

    /// Numero di pagine aggiunte.
    pub fn page_count(&self) -> usize {
        self.core.borrow().pages.len()
    }
}

/// Seleziona una pagina e, se la selezione è cambiata, la propaga al
/// registro dell'inspector (è `read_state` a leggerla come `selection`).
fn apply_selection(core: &Rc<RefCell<SimplebookCore>>, handle: NativeHandle, index: usize) {
    if core.borrow_mut().select(index) {
        if let Some(selection) = core.borrow().selected {
            declared::set_selection(handle, selection);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::Rect;
    use crate::core::widget::Widget;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Widget minimo per i test (stesso pattern di `sizer.rs`).
    struct MockWidget {
        rect: Rect,
        visible: bool,
        enabled: bool,
    }

    impl MockWidget {
        #[allow(clippy::new_ret_no_self)]
        fn new(w: u32, h: u32) -> Rc<RefCell<dyn Widget>> {
            Rc::new(RefCell::new(MockWidget {
                rect: Rect::new(0, 0, w, h),
                visible: true,
                enabled: true,
            }))
        }
    }

    impl Widget for MockWidget {
        fn native_handle(&self) -> isize {
            0
        }
        fn set_position(&mut self, x: i32, y: i32) {
            self.rect.x = x;
            self.rect.y = y;
        }
        fn set_size(&mut self, w: u32, h: u32) {
            self.rect.width = w;
            self.rect.height = h;
        }
        fn rect(&self) -> Rect {
            self.rect
        }
        fn is_visible(&self) -> bool {
            self.visible
        }
        fn set_visible(&mut self, visible: bool) {
            self.visible = visible;
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }
    }

    fn visible(page: &Rc<RefCell<dyn Widget>>) -> bool {
        page.borrow().is_visible()
    }

    #[test]
    fn prima_pagina_aggiunta_e_selezionata() {
        let mut core = SimplebookCore::new();
        let page = MockWidget::new(100, 60);
        let selection = core.add_page("One", page.clone());
        assert_eq!(selection, Some(0));
        assert_eq!(core.selected, Some(0));
        assert!(visible(&page));
    }

    #[test]
    fn pagine_successive_restate_nascoste_fino_alla_selezione() {
        let mut core = SimplebookCore::new();
        let first = MockWidget::new(100, 60);
        let second = MockWidget::new(100, 60);
        core.add_page("One", first.clone());
        // La seconda pagina non deve diventare visibile: solo la
        // corrente resta visibile.
        let selection = core.add_page("Two", second.clone());
        assert_eq!(selection, None);
        assert!(!visible(&second));

        assert!(core.select(1));
        assert!(!visible(&first));
        assert!(visible(&second));
        assert_eq!(core.selected, Some(1));
    }

    #[test]
    fn selezione_fuori_range_o_invariata_non_cambia_niente() {
        let mut core = SimplebookCore::new();
        let page = MockWidget::new(100, 60);
        core.add_page("One", page.clone());
        assert!(!core.select(5), "fuori range: nessun effetto");
        assert!(!core.select(0), "stessa pagina: nessun effetto");
        assert_eq!(core.selected, Some(0));
        assert!(visible(&page));
    }

    #[test]
    fn index_of_cerca_per_nome() {
        let mut core = SimplebookCore::new();
        core.add_page("One", MockWidget::new(10, 10));
        core.add_page("Two", MockWidget::new(10, 10));
        assert_eq!(core.index_of("Two"), Some(1));
        assert_eq!(core.index_of("Missing"), None);
        assert_eq!(core.index_of("One"), Some(0));
    }
}
