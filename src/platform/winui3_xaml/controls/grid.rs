//! XamlGrid — data grid widget.
//!
//! The most complex widget in the bridge: columns, rows, sorting,
//! selection, alternating colors, and multiple cell types.

use super::super::element_bridge::{
    next_id, GridCellDesc, GridColumnAlign, GridColumnDesc, WidgetDesc, WidgetId,
};
use super::super::window::XamlFrame;

/// Data grid (full-featured table).
pub struct XamlGrid {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlGrid {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::Grid {
            id,
            columns: Vec::new(),
            rows: Vec::new(),
            sort_column: None,
            sort_ascending: true,
            selected_row: None,
            alternating_colors: false,
            even_color: (255, 255, 255),
            odd_color: (245, 245, 250),
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a column.
    pub fn add_column(&self, title: &str, width: f64) {
        self.add_column_aligned(title, width, GridColumnAlign::Left);
    }

    /// Add a column with alignment.
    pub fn add_column_aligned(&self, title: &str, width: f64, align: GridColumnAlign) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid { id, columns, .. } = child {
                if *id == self.id {
                    columns.push(GridColumnDesc {
                        title: title.to_string(),
                        width,
                        align,
                    });
                    break;
                }
            }
        }
    }

    /// Set the row count (pre-allocate).
    pub fn set_row_count(&self, count: usize) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid {
                id, rows, columns, ..
            } = child
            {
                if *id == self.id {
                    let col_count = columns.len();
                    rows.resize_with(count, || vec![GridCellDesc::Empty; col_count]);
                    break;
                }
            }
        }
    }

    /// Set a cell value.
    pub fn set_cell(&self, row: usize, col: usize, cell: GridCellDesc) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid { id, rows, .. } = child {
                if *id == self.id {
                    if let Some(r) = rows.get_mut(row) {
                        if let Some(c) = r.get_mut(col) {
                            *c = cell;
                        }
                    }
                    break;
                }
            }
        }
    }

    /// Set a text cell.
    pub fn set_cell_text(&self, row: usize, col: usize, text: &str) {
        self.set_cell(row, col, GridCellDesc::Text(text.to_string()));
    }

    /// Enable alternating row colors.
    pub fn set_alternating_row_colors(&self, even: (u8, u8, u8), odd: (u8, u8, u8)) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid {
                id,
                alternating_colors,
                even_color,
                odd_color,
                ..
            } = child
            {
                if *id == self.id {
                    *alternating_colors = true;
                    *even_color = even;
                    *odd_color = odd;
                    break;
                }
            }
        }
    }

    /// Sort by a column.
    ///
    /// Riordina fisicamente le righe (chiave = testo renderizzato della
    /// cella; `Progress`/`Bool` hanno chiave numerica/0-1) e imposta
    /// l'indicatore ↑/↓ nell'header. L'ordine di inserimento non è
    /// memorizzato: `clear_sort` rimuove solo l'indicatore.
    pub fn sort_by_column(&self, col: usize, ascending: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid {
                id,
                rows,
                sort_column,
                sort_ascending,
                ..
            } = child
            {
                if *id == self.id {
                    *sort_column = Some(col);
                    *sort_ascending = ascending;
                    rows.sort_by(|a, b| {
                        let ord = cell_sort_key(a.get(col)).cmp(&cell_sort_key(b.get(col)));
                        if ascending {
                            ord
                        } else {
                            ord.reverse()
                        }
                    });
                    break;
                }
            }
        }
    }

    /// Clear the sort.
    pub fn clear_sort(&self) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid {
                id, sort_column, ..
            } = child
            {
                if *id == self.id {
                    *sort_column = None;
                    break;
                }
            }
        }
    }

    /// Set the selected row.
    pub fn set_selected_row(&self, row: Option<usize>) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Grid {
                id, selected_row, ..
            } = child
            {
                if *id == self.id {
                    *selected_row = row;
                    break;
                }
            }
        }
    }
}

/// Chiave di ordinamento di una cella (testo renderizzato; i valori
/// numerici usano chiave a zeri pieni perché l'ordinamento sia lessicografico
/// coerente con il resto della colonna).
fn cell_sort_key(cell: Option<&GridCellDesc>) -> String {
    match cell {
        Some(GridCellDesc::Text(t)) => t.clone(),
        Some(GridCellDesc::Badge { text, .. }) => text.clone(),
        Some(GridCellDesc::Bool(b)) => if *b { "1" } else { "0" }.to_string(),
        Some(GridCellDesc::Progress { value, .. }) => format!("{:010}", value),
        _ => String::new(),
    }
}
