//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Chiavi esterne: `ForeignKey`, il vincolo verso un'altra tabella.

/// Vincolo di chiave esterna di una tabella.
///
/// È il pezzo fondamentale per il futuro generatore di form: indica
/// quali colonna è un "lookup" verso un'altra tabella e quale colonna
/// usare come etichetta leggibile nei combo/liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKey {
    /// Colonna della tabella corrente che contiene la chiave.
    pub column: String,
    /// Tabella referenziata.
    pub ref_table: String,
    /// Colonna referenziata nella tabella remota (tipicamente la PK).
    pub ref_column: String,
    /// Colonna testuale della tabella remota da usare come etichetta
    /// (es. `nome`, `descrizione`); `None` se ancora da scoprire.
    pub display_column: Option<String>,
}

impl ForeignKey {
    /// Crea una chiave esterna minima (colonna → tabella.colonna).
    pub fn new(
        column: impl Into<String>,
        ref_table: impl Into<String>,
        ref_column: impl Into<String>,
    ) -> Self {
        ForeignKey {
            column: column.into(),
            ref_table: ref_table.into(),
            ref_column: ref_column.into(),
            display_column: None,
        }
    }

    /// Imposta la colonna di visualizzazione della tabella referenziata.
    pub fn with_display_column(mut self, col: impl Into<String>) -> Self {
        self.display_column = Some(col.into());
        self
    }
}
