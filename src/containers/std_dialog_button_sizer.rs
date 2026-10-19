//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Standard dialog button sizer (`wxStdDialogButtonSizer`).
//!
//! Puro Rust: raggruppa i pulsanti standard di un dialogo e ne
//! determina l'ordine di visualizzazione in base all'OS in cui si
//! esegue l'applicazione. L'unica differenza reale tra le
//! piattaforme è l'ordine dei pulsanti:
//!
//! * **Windows** — gli affirmativi a sinistra: `[OK] [Cancel]`
//! * **macOS** — i negativi a sinistra, l'affirmativo a destra
//!   (Apple HIG): `[Cancel] [OK]`
//!
//! [`StdDialogButtonSizer::realize`] restituisce una
//! [`BoxSizer`] orizzontale già nell'ordine corretto, da annidare
//! con [`BoxSizer::add_sizer`] o passare a
//! [`crate::Frame::set_sizer`].

use crate::containers::sizer::BoxSizer;
use crate::controls::button::Button;
use crate::core::widget::WidgetRef;

/// Ruolo standard di un pulsante da dialogo (`wxStdDialogButtonSizer`
/// accetta solo questi, perché sono gli unici per cui esiste una
/// convenzione di posizione).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdDialogButton {
    Ok,
    Yes,
    No,
    Save,
    Apply,
    Cancel,
    Close,
    Help,
    Retry,
}

/// Ordine per-OS dei gruppi di pulsanti. `Windows` e la variante
/// stub (nessuno dei due OS supportato) mettono gli affirmativi a
/// sinistra; `Macos` i negativi, come da HIG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OsOrder {
    Windows,
    Macos,
}

/// L'ordine dei pulsanti valido per l'OS in cui compiliamo.
const fn os_order() -> OsOrder {
    if cfg!(target_os = "macos") {
        OsOrder::Macos
    } else {
        OsOrder::Windows
    }
}

/// Indice di gruppo (e quindi di ordinamento) di un ruolo.
///
/// `0` e `1` sono i due gruppi "normali" (l'ordine dipende dall'OS),
/// `2` è [`StdDialogButton::Help`], sempre in coda.
fn group_index(role: StdDialogButton, os: OsOrder) -> u8 {
    use StdDialogButton::*;
    let affirmative = matches!(role, Ok | Yes | Save | Apply | Retry);
    match (os, affirmative) {
        // Windows: `[OK] [Cancel] … [Help]`
        (OsOrder::Windows, true) => 0,
        (OsOrder::Windows, false) if role == Help => 2,
        (OsOrder::Windows, false) => 1,
        // macOS: `[Cancel] [OK] … [Help]`
        (OsOrder::Macos, false) if role == Help => 2,
        (OsOrder::Macos, false) => 0,
        (OsOrder::Macos, true) => 1,
    }
}

/// Sizer che dispone i pulsanti standard di un dialogo secondo le
/// convenzioni dell'OS corrente.
pub struct StdDialogButtonSizer {
    entries: Vec<(WidgetRef, StdDialogButton)>,
}

impl StdDialogButtonSizer {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Registra un pulsante con il suo ruolo standard. L'ordine di
    /// chiamata è irrilevante: è [`Self::realize`] a stabilire
    /// l'ordine di visualizzazione.
    pub fn add_button(&mut self, btn: &Button, which: StdDialogButton) {
        self.entries.push((btn.as_widget_ref(), which));
    }

    /// Costruisce la [`BoxSizer`] orizzontale con i pulsanti
    /// nell'ordine previsto dall'OS. Gli elementi mantengono
    /// l'ordine di inserimento all'interno del proprio gruppo.
    pub fn realize(&self) -> BoxSizer {
        let os = os_order();
        // `sort_by_key` è stabile: elementi dello stesso gruppo
        // restano nell'ordine in cui sono stati aggiunti.
        let mut ordered: Vec<&(WidgetRef, StdDialogButton)> = self.entries.iter().collect();
        ordered.sort_by_key(|(_, role)| group_index(*role, os));
        let mut sizer = BoxSizer::horizontal();
        for (widget, _) in ordered {
            sizer.add(widget.clone());
        }
        sizer
    }

    /// Il numero di pulsanti registrati finora.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` se nessun pulsante è stato registrato.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for StdDialogButtonSizer {
    fn default() -> Self {
        Self::new()
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

    /// Posizioni X (dopo layout) dei mock registrati in `sizer`.
    fn laid_out_x(sizer: BoxSizer, widgets: &[Rc<RefCell<dyn Widget>>]) -> Vec<i32> {
        let mut sizer = sizer;
        sizer.layout(0, 0, 400, 30);
        widgets.iter().map(|w| w.borrow().rect().x).collect()
    }

    #[test]
    fn windows_puts_affirmative_first() {
        // Anche aggiungendo il negativo per primo, `[OK] [Cancel]`.
        let cancel = MockWidget::new(70, 24);
        let ok = MockWidget::new(70, 24);
        let mut s = StdDialogButtonSizer::new();
        s.entries.push((cancel.clone(), StdDialogButton::Cancel));
        s.entries.push((ok.clone(), StdDialogButton::Ok));

        if os_order() == OsOrder::Windows {
            let xs = laid_out_x(s.realize(), &[ok, cancel]);
            assert_eq!(xs[0], 0, "OK a sinistra su Windows");
            assert!(xs[1] > xs[0], "Cancel dopo OK");
        }
    }

    #[test]
    fn macos_puts_negative_first() {
        let cancel = MockWidget::new(70, 24);
        let ok = MockWidget::new(70, 24);
        let mut s = StdDialogButtonSizer::new();
        s.entries.push((ok.clone(), StdDialogButton::Ok));
        s.entries.push((cancel.clone(), StdDialogButton::Cancel));

        if os_order() == OsOrder::Macos {
            let xs = laid_out_x(s.realize(), &[cancel, ok]);
            assert_eq!(xs[0], 0, "Cancel a sinistra su macOS");
            assert!(xs[1] > xs[0], "OK dopo Cancel");
        }
    }

    #[test]
    fn group_index_matches_expected_order() {
        use StdDialogButton::*;
        // Windows: affirmativi 0, negativi 1, help 2.
        assert_eq!(group_index(Ok, OsOrder::Windows), 0);
        assert_eq!(group_index(Retry, OsOrder::Windows), 0);
        assert_eq!(group_index(Cancel, OsOrder::Windows), 1);
        assert_eq!(group_index(Close, OsOrder::Windows), 1);
        assert_eq!(group_index(Help, OsOrder::Windows), 2);
        // macOS: negativi 0, affirmativi 1, help 2.
        assert_eq!(group_index(Cancel, OsOrder::Macos), 0);
        assert_eq!(group_index(No, OsOrder::Macos), 0);
        assert_eq!(group_index(Ok, OsOrder::Macos), 1);
        assert_eq!(group_index(Yes, OsOrder::Macos), 1);
        assert_eq!(group_index(Help, OsOrder::Macos), 2);
    }

    #[test]
    fn help_is_always_last() {
        // Su entrambi gli OS Help ha il gruppo 2: deve chiudere la fila.
        for os in [OsOrder::Windows, OsOrder::Macos] {
            assert_eq!(group_index(StdDialogButton::Help, os), 2);
            assert!(group_index(StdDialogButton::Ok, os) < 2);
            assert!(group_index(StdDialogButton::Cancel, os) < 2);
        }
    }

    #[test]
    fn realize_keeps_insertion_order_within_group() {
        // Due affirmativi: l'ordine di aggiunta è preservato.
        let save = MockWidget::new(70, 24);
        let ok = MockWidget::new(70, 24);
        let mut s = StdDialogButtonSizer::new();
        s.entries.push((save.clone(), StdDialogButton::Save));
        s.entries.push((ok.clone(), StdDialogButton::Ok));
        let xs = laid_out_x(s.realize(), &[save, ok]);
        assert!(xs[0] < xs[1], "Save resta prima di OK (stesso gruppo)");
    }

    #[test]
    fn empty_sizer_is_empty_and_realizes_to_empty_row() {
        let s = StdDialogButtonSizer::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        // Un layout con zero elementi non deve andare in panic.
        s.realize().layout(0, 0, 100, 30);
    }
}
