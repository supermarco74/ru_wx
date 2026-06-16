//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Data-view controls (`wxDataViewCtrl` family) — model/view with sort/filter.

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::list_ctrl::{ListCtrl, ListCtrlStyle};
use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// Column descriptor (`wxDataViewColumn`).
#[derive(Debug, Clone)]
pub struct DataViewColumn {
    pub title: String,
    pub width: i32,
    pub align_right: bool,
}

/// Cell renderer trait (`wxDataViewRenderer`).
pub trait DataViewRenderer: Send {
    fn render_text(&self, value: &str) -> String;
}

#[derive(Default)]
pub struct TextRenderer;

impl DataViewRenderer for TextRenderer {
    fn render_text(&self, value: &str) -> String {
        value.to_string()
    }
}

/// Row-oriented data source for [`DataViewCtrl`].
pub trait DataViewModel: Send {
    fn row_count(&self) -> usize;
    fn value(&self, row: usize, col: usize) -> String;
    fn push_row(&mut self, values: Vec<String>) {
        let _ = values;
    }
}

/// In-memory tabular model (`wxDataViewIndexListModel`-like).
#[derive(Debug, Clone, Default)]
pub struct InMemoryDataViewModel {
    pub rows: Vec<Vec<String>>,
}

impl DataViewModel for InMemoryDataViewModel {
    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn value(&self, row: usize, col: usize) -> String {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .cloned()
            .unwrap_or_default()
    }

    fn push_row(&mut self, values: Vec<String>) {
        self.rows.push(values);
    }
}

struct DataViewState {
    model: Option<Rc<RefCell<dyn DataViewModel>>>,
    sort_column: Option<usize>,
    sort_ascending: bool,
    filter: String,
    cached_rows: Vec<usize>,
}

/// Generic data-view host (`wxDataViewCtrl`).
#[derive(Clone)]
pub struct DataViewCtrl {
    list: ListCtrl,
    columns: Rc<RefCell<Vec<DataViewColumn>>>,
    state: Rc<RefCell<DataViewState>>,
}

impl DataViewCtrl {
    pub fn new<W: crate::core::widget::Window>(parent: &W) -> Self {
        Self {
            list: ListCtrl::new(parent, ListCtrlStyle::Report),
            columns: Rc::new(RefCell::new(Vec::new())),
            state: Rc::new(RefCell::new(DataViewState {
                model: None,
                sort_column: None,
                sort_ascending: true,
                filter: String::new(),
                cached_rows: Vec::new(),
            })),
        }
    }

    pub fn set_model(&self, model: Rc<RefCell<dyn DataViewModel>>) {
        self.state.borrow_mut().model = Some(model);
        self.refresh();
    }

    pub fn set_sort_column(&self, column: Option<usize>, ascending: bool) {
        let mut st = self.state.borrow_mut();
        st.sort_column = column;
        st.sort_ascending = ascending;
        drop(st);
        self.refresh();
    }

    pub fn set_filter(&self, text: &str) {
        self.state.borrow_mut().filter = text.to_string();
        self.refresh();
    }

    pub fn refresh(&self) {
        let (rows, col_count) = {
            let st = self.state.borrow();
            let Some(model) = st.model.as_ref() else {
                return;
            };
            let model = model.borrow();
            let col_count = self.columns.borrow().len().max(1);
            let filter = st.filter.to_lowercase();
            let mut rows: Vec<usize> = (0..model.row_count())
                .filter(|&row| {
                    if filter.is_empty() {
                        return true;
                    }
                    (0..col_count).any(|col| {
                        model
                            .value(row, col)
                            .to_lowercase()
                            .contains(&filter)
                    })
                })
                .collect();
            if let Some(sort_col) = st.sort_column {
                let asc = st.sort_ascending;
                rows.sort_by(|&a, &b| {
                    let va = model.value(a, sort_col);
                    let vb = model.value(b, sort_col);
                    if asc {
                        va.cmp(&vb)
                    } else {
                        vb.cmp(&va)
                    }
                });
            }
            (rows, col_count)
        };

        while self.list.get_item_count() > 0 {
            self.list.delete_item(0);
        }
        let st = self.state.borrow();
        let Some(model) = st.model.as_ref() else {
            return;
        };
        let model = model.borrow();
        for (display_idx, &row) in rows.iter().enumerate() {
            let first = model.value(row, 0);
            let idx = self.list.insert_item(display_idx, &first);
            for col in 1..col_count {
                self.list.set_item_text(idx, col, &model.value(row, col));
            }
        }
        drop(model);
        drop(st);
        self.state.borrow_mut().cached_rows = rows;
    }

    /// Source row indices after the last sort/filter pass.
    pub fn visible_row_indices(&self) -> Vec<usize> {
        self.state.borrow().cached_rows.clone()
    }

    pub fn append_column(&self, title: &str, width: i32) {
        self.columns.borrow_mut().push(DataViewColumn {
            title: title.to_string(),
            width,
            align_right: false,
        });
        let idx = self.columns.borrow().len() as u32 - 1;
        self.list.insert_column(idx, title, width);
    }

    /// Append a row directly (creates an internal model when needed).
    pub fn append_item(&self, values: &[&str]) {
        if self.state.borrow().model.is_none() {
            self.set_model(Rc::new(RefCell::new(InMemoryDataViewModel::default())));
        }
        if let Some(model) = self.state.borrow().model.as_ref() {
            model
                .borrow_mut()
                .push_row(values.iter().map(|s| (*s).to_string()).collect());
        }
        self.refresh();
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }
}

/// List-shaped data view (`wxDataViewListCtrl`).
pub type DataViewListCtrl = DataViewCtrl;

/// Tree-shaped data view (`wxDataViewTreeCtrl`).
#[derive(Clone)]
pub struct DataViewTreeCtrl {
    tree: TreeCtrl,
}

impl DataViewTreeCtrl {
    pub fn new(frame: &Frame) -> Self {
        Self {
            tree: TreeCtrl::new(frame),
        }
    }

    pub fn append_root(&self, label: &str) -> TreeItem {
        self.tree.add_root(label)
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.tree.as_widget_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_model_sort_and_filter() {
        let model = Rc::new(RefCell::new(InMemoryDataViewModel {
            rows: vec![
                vec!["b".into(), "2".into()],
                vec!["a".into(), "1".into()],
            ],
        }));
        let dv = DataViewCtrl::new(&crate::window::frame::Frame::for_testing());
        dv.append_column("Name", 80);
        dv.set_model(model);
        dv.set_sort_column(Some(0), true);
        dv.set_filter("a");
        assert_eq!(dv.visible_row_indices(), vec![1]);
    }
}
