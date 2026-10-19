//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! NumberBox (casella numerica con spin) — la variante WinUI 3 del
//! controllo numerico: `EDIT` + `msctls_updown32` con range, incremento,
//! decimali e notifica al cambio valore.
//!
//! Sul backend Windows è un sottotipo di [`SpinCtrlDouble`] (che gestisce
//! già la coppia EDIT/up-down e la scala `10^decimali`): qui si espone
//! soltanto l'API tipica dei `NumberBox` di WinUI 3 (`decimals` al posto
//! di `digits`, `new` con quattro argomenti).
//!
//! # Esempio
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let nb = NumberBox::new(&frame, 0.5, 0.0, 10.0);
//! nb.set_decimals(2);
//! assert!((nb.get_value() - 0.5).abs() < 1e-9);
//! ```

use crate::controls::spin_ctrl_double::SpinCtrlDouble;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

/// Casella numerica con spin dedicato (`wxSpinCtrlDouble` con API
/// WinUI 3 `NumberBox`).
pub struct NumberBox {
    spin: SpinCtrlDouble,
}

impl NumberBox {
    /// Crea la casella numerica.
    ///
    /// * `value` — valore iniziale, clamped in `[min, max]`.
    /// * `min`, `max` — range in unità utente.
    ///
    /// Default di fabbrica: incremento `1.0`, 2 decimali (entrambi
    /// modificabili con [`NumberBox::set_increment`] /
    /// [`NumberBox::set_decimals`]).
    pub fn new<W: Window>(parent: &W, value: f64, min: f64, max: f64) -> Self {
        NumberBox {
            spin: SpinCtrlDouble::new(parent, value, min, max, 1.0, 2),
        }
    }

    /// Valore corrente.
    pub fn get_value(&self) -> f64 {
        self.spin.get_value()
    }

    /// Imposta il valore (clamp nel range corrente, nessuna notifica).
    pub fn set_value(&self, value: f64) {
        self.spin.set_value(value);
    }

    /// Imposta il range; il valore corrente viene clamped.
    pub fn set_range(&self, min: f64, max: f64) {
        self.spin.set_range(min, max);
    }

    /// Imposta l'incremento dei pulsanti su/giù (deve essere > 0).
    pub fn set_increment(&self, increment: f64) {
        self.spin.set_increment(increment);
    }

    /// Numero di decimali mostrati nella casella.
    pub fn set_decimals(&self, decimals: u32) {
        self.spin.set_digits(decimals);
    }

    /// Numero di decimali corrente.
    pub fn get_decimals(&self) -> u32 {
        self.spin.get_digits()
    }

    /// Registra la callback invocata quando il valore effettivo cambia
    /// (fine modifica del testo, click sulle frecce, tasti su/giù).
    pub fn on_value_change<F: FnMut(f64) + 'static>(&self, frame: &Frame, callback: F) {
        self.spin.on_value_change(frame, callback);
    }

    /// Dimensione ideale: larghezza sufficiente a "0.00" + freccette,
    /// altezza standard da casella di testo.
    pub fn default_size() -> (u32, u32) {
        (120, 24)
    }

    /// Widget sottostante (l'`EDIT` della coppia up-down).
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.spin.as_widget_ref()
    }
}
