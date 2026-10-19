//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Generazione SQL parametrica da uno [`TableSchema`]: `INSERT` e
//! `UPDATE ... WHERE pk = ?` pronti per [`DataProvider::execute`].
//!
//! Le funzioni sono **pure** (nessun I/O, nessun database): costruiscono
//! la coppia `(sql, parametri)` e sono quindi testabili senza backend.
//! Il quoting degli identificatori è delegato al provider
//! ([`DataProvider::quote_ident`]), passato come closure.
//!
//! Regole comuni:
//! - le colonne `auto_increment` sono sempre escluse (le genera il DB);
//! - i parametri seguono l'ordine delle colonne dello schema;
//! - l'`UPDATE` richiede una chiave primaria singola e usa il valore
//!   **originale** della riga (non quello editato nel form).

use crate::data::error::DataError;
use crate::data::schema::TableSchema;
use crate::data::value::DataValue;

/// Costruisce `INSERT INTO <tabella> (<colonne>) VALUES (?, ...)`.
///
/// `row` è la riga di valori nell'ordine delle colonne dello schema
/// (p.es. da [`crate::containers::auto_form::AutoForm::get_row`]).
/// Le colonne auto-incremento sono escluse da colonne e parametri.
pub fn build_insert(
    schema: &TableSchema,
    row: &[DataValue],
    quote: &dyn Fn(&str) -> String,
) -> Result<(String, Vec<DataValue>), DataError> {
    let included: Vec<usize> = schema
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.auto_increment)
        .map(|(i, _)| i)
        .collect();
    if included.is_empty() {
        return Err(DataError::Schema(format!(
            "tabella {}: nessuna colonna inseribile",
            schema.name
        )));
    }
    let cols = included
        .iter()
        .map(|&i| quote(&schema.columns[i].name))
        .collect::<Vec<_>>()
        .join(", ");
    let marks = vec!["?"; included.len()].join(", ");
    let sql = format!(
        "INSERT INTO {} ({cols}) VALUES ({marks})",
        quote(&schema.name)
    );
    let params = included
        .iter()
        .map(|&i| row.get(i).cloned().unwrap_or(DataValue::Null))
        .collect();
    Ok((sql, params))
}

/// Costruisce `UPDATE <tabella> SET <col> = ?, ... WHERE <pk> = ?`.
///
/// * `row` — valori correnti (p.es. dal form), nell'ordine dello schema;
/// * `pk_original` — valore **originale** della chiave primaria (dalla
///   tabella), usato nella `WHERE`: la colonna PK non viene mai
///   riscritta.
///
/// Fallisce se lo schema non dichiara una `primary_key` o se la PK è
/// auto-incremento ma non è presente tra le colonne.
pub fn build_update(
    schema: &TableSchema,
    row: &[DataValue],
    pk_original: &DataValue,
    quote: &dyn Fn(&str) -> String,
) -> Result<(String, Vec<DataValue>), DataError> {
    let pk_name = schema.primary_key.as_ref().ok_or_else(|| {
        DataError::Schema(format!("tabella {}: nessuna chiave primaria", schema.name))
    })?;
    if schema.column_index(pk_name).is_none() {
        return Err(DataError::Schema(format!(
            "tabella {}: chiave primaria {pk_name} assente dalle colonne",
            schema.name
        )));
    }
    let pk_idx = schema.column_index(pk_name).expect("pk appena verificata");
    let included: Vec<usize> = schema
        .columns
        .iter()
        .enumerate()
        .filter(|(i, c)| !c.auto_increment && *i != pk_idx)
        .map(|(i, _)| i)
        .collect();
    if included.is_empty() {
        return Err(DataError::Schema(format!(
            "tabella {}: nessuna colonna aggiornabile",
            schema.name
        )));
    }
    let sets = included
        .iter()
        .map(|&i| format!("{} = ?", quote(&schema.columns[i].name)))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE {} SET {sets} WHERE {} = ?",
        quote(&schema.name),
        quote(pk_name)
    );
    let mut params: Vec<DataValue> = included
        .iter()
        .map(|&i| row.get(i).cloned().unwrap_or(DataValue::Null))
        .collect();
    params.push(pk_original.clone());
    Ok((sql, params))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::column::{DataColumn, DataFieldType};

    /// Quoting alla SQLite (`"nome"`), sufficiente per i test.
    fn quote(ident: &str) -> String {
        format!("\"{ident}\"")
    }

    fn schema() -> TableSchema {
        let mut s = TableSchema::new("prodotti");
        s.columns = vec![
            DataColumn::new("id", DataFieldType::Integer)
                .with_primary_key()
                .with_auto_increment(),
            DataColumn::new("nome", DataFieldType::Text).with_not_null(),
            DataColumn::new("prezzo", DataFieldType::Real),
        ];
        s.primary_key = Some("id".to_string());
        s
    }

    fn riga() -> Vec<DataValue> {
        vec![
            DataValue::Int(7),
            DataValue::Text("Mela".to_string()),
            DataValue::Real(1.5),
        ]
    }

    #[test]
    fn insert_esclude_l_auto_increment() {
        let (sql, params) = build_insert(&schema(), &riga(), &quote).expect("insert");
        assert_eq!(
            sql,
            "INSERT INTO \"prodotti\" (\"nome\", \"prezzo\") VALUES (?, ?)"
        );
        assert_eq!(
            params,
            vec![DataValue::Text("Mela".to_string()), DataValue::Real(1.5)]
        );
    }

    #[test]
    fn update_usa_la_pk_originale_nella_where() {
        let (sql, params) =
            build_update(&schema(), &riga(), &DataValue::Int(7), &quote).expect("update");
        assert_eq!(
            sql,
            "UPDATE \"prodotti\" SET \"nome\" = ?, \"prezzo\" = ? WHERE \"id\" = ?"
        );
        // La PK va in coda ai parametri e non fra i SET.
        assert_eq!(
            params,
            vec![
                DataValue::Text("Mela".to_string()),
                DataValue::Real(1.5),
                DataValue::Int(7),
            ]
        );
    }

    #[test]
    fn update_senza_pk_fallisce() {
        let mut s = TableSchema::new("log");
        s.columns = vec![DataColumn::new("msg", DataFieldType::Text)];
        let err = build_update(
            &s,
            &[DataValue::Text("x".to_string())],
            &DataValue::Null,
            &quote,
        );
        assert!(err.is_err());
    }

    #[test]
    fn riga_corta_riempie_con_null() {
        let (sql, params) = build_insert(&schema(), &[DataValue::Int(1)], &quote).expect("insert");
        assert!(sql.contains("VALUES (?, ?)"));
        assert_eq!(params, vec![DataValue::Null, DataValue::Null]);
    }
}
