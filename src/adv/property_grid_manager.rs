//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Multi-page property grid (`wxPropertyGridManager`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::adv::property_grid::PropertyGrid;
use crate::window::frame::Frame;

/// Hosts several [`PropertyGrid`] pages (`wxPropertyGridManager`).
#[derive(Clone)]
pub struct PropertyGridManager {
    pages: Rc<RefCell<Vec<(String, PropertyGrid)>>>,
    current: Rc<RefCell<usize>>,
}

impl PropertyGridManager {
    pub fn new(_frame: &Frame) -> Self {
        Self {
            pages: Rc::new(RefCell::new(Vec::new())),
            current: Rc::new(RefCell::new(0)),
        }
    }

    pub fn add_page(&self, label: &str, grid: PropertyGrid) -> usize {
        let mut pages = self.pages.borrow_mut();
        let idx = pages.len();
        pages.push((label.to_string(), grid));
        idx
    }

    pub fn page_count(&self) -> usize {
        self.pages.borrow().len()
    }

    pub fn page_label(&self, index: usize) -> Option<String> {
        self.pages.borrow().get(index).map(|(l, _)| l.clone())
    }

    pub fn current_page(&self) -> usize {
        *self.current.borrow()
    }

    pub fn set_current_page(&self, index: usize) {
        if index < self.page_count() {
            *self.current.borrow_mut() = index;
        }
    }

    pub fn current_grid(&self) -> Option<PropertyGrid> {
        let idx = *self.current.borrow();
        self.pages.borrow().get(idx).map(|(_, g)| g.clone())
    }
}
