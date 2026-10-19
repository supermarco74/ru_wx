//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Schemi: `TableSchema`, descrizione completa di una tabella per il
//! futuro generatore di form.

use crate::data::column::DataColumn;
use crate::data::foreign_key::ForeignKey;

/// Descrizione di una tabella: colonne, chiave primaria, chiavi esterne
/// e colonna-etichetta consigliata.
#[derive(Debug, Clone, PartialEq)]
pub struct TableSchema {
    /// Nome della tabella.
    pub name: String,
    /// Colonne della tabella, nell'ordine del database.
    pub columns: Vec<DataColumn>,
    /// Nome della colonna chiave primaria (sola, se la PK è composta
    /// viene restituita la prima).
    pub primary_key: Option<String>,
    /// Vincoli di chiave esterna verso altre tabelle.
    pub foreign_keys: Vec<ForeignKey>,
    /// Colonna consigliata come etichetta leggibile (per combo, liste,
    /// intestazioni di form); calcolata da `guess_label_column` se non
    /// fornita esplicitamente.
    pub label_column: Option<String>,
}

impl TableSchema {
    /// Crea uno schema vuoto per una tabella.
    pub fn new(name: impl Into<String>) -> Self {
        TableSchema {
            name: name.into(),
            columns: Vec::new(),
            primary_key: None,
            foreign_keys: Vec::new(),
            label_column: None,
        }
    }

    /// Cerca l'indice di una colonna per nome (case-insensitive).
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Ritorna la colonna per nome (case-insensitive).
    pub fn column(&self, name: &str) -> Option<&DataColumn> {
        self.column_index(name).map(|i| &self.columns[i])
    }

    /// Ritorna tutte le chiavi esterne che partono dalla colonna indicata.
    pub fn foreign_keys_for(&self, column: &str) -> Vec<&ForeignKey> {
        self.foreign_keys
            .iter()
            .filter(|fk| fk.column.eq_ignore_ascii_case(column))
            .collect()
    }

    /// Indovina la colonna-etichetta della tabella.
    ///
    /// Regole, in ordine di priorità:
    /// 1. la prima colonna testuale il cui nome contiene `desc`, `nome`,
    ///    `name`, `title` o `label` (esclusa la chiave primaria);
    /// 2. la prima colonna testuale non-PK;
    /// 3. la prima colonna non-PK;
    /// 4. la prima colonna.
    ///
    /// Il risultato è anche cachato in `label_column`.
    pub fn guess_label_column(&mut self) -> Option<String> {
        if let Some(label) = &self.label_column {
            return Some(label.clone());
        }
        let pk = self.primary_key.clone();
        let is_pk = |c: &DataColumn| -> bool {
            match &pk {
                Some(pk) => c.name.eq_ignore_ascii_case(pk),
                None => c.primary_key,
            }
        };
        let keywords = ["desc", "nome", "name", "title", "label"];
        let mut found = None;

        // 1) colonna testuale con keyword
        for c in &self.columns {
            if is_pk(c) {
                continue;
            }
            if c.data_type == crate::data::column::DataFieldType::Text {
                let lower = c.name.to_lowercase();
                if keywords.iter().any(|k| lower.contains(k)) {
                    found = Some(c.name.clone());
                    break;
                }
            }
        }
        // 2) prima colonna testuale non-PK
        if found.is_none() {
            found = self
                .columns
                .iter()
                .find(|c| !is_pk(c) && c.data_type == crate::data::column::DataFieldType::Text)
                .map(|c| c.name.clone());
        }
        // 3) prima colonna non-PK
        if found.is_none() {
            found = self
                .columns
                .iter()
                .find(|c| !is_pk(c))
                .map(|c| c.name.clone());
        }
        // 4) prima colonna
        if found.is_none() {
            found = self.columns.first().map(|c| c.name.clone());
        }

        self.label_column = found.clone();
        found
    }

    /// Assicura che `label_column` sia valorizzata (chiama `guess_label_column`).
    pub fn ensure_label(&mut self) -> Option<String> {
        self.guess_label_column()
    }
}
