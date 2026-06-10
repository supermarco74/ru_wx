//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Office ribbon (`wxRibbonBar`) — toolbar strip placeholder.

use std::cell::RefCell;
use std::rc::Rc;

use crate::chrome::ribbon_page::RibbonPage;
use crate::chrome::tool_bar::ToolBar;
use crate::platform::win32::next_control_id;
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct RibbonBar {
    bar: ToolBar,
    pages: Rc<RefCell<Vec<RibbonPage>>>,
}

impl RibbonBar {
    pub fn new(frame: &Frame) -> Self {
        Self {
            bar: ToolBar::new(frame),
            pages: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn add_page(&self, page: RibbonPage) -> usize {
        let mut pages = self.pages.borrow_mut();
        let idx = pages.len();
        for bar in page.panels().iter().flat_map(|p| p.bars()) {
            for &(id, ref label) in bar.buttons() {
                self.bar.add_tool(id, label, 0);
            }
        }
        pages.push(page);
        idx
    }

    pub fn page_count(&self) -> usize {
        self.pages.borrow().len()
    }

    pub fn add_tool(&self, label: &str) -> u16 {
        let id = next_control_id();
        self.bar.add_tool(id, label, 0);
        id
    }

    pub fn realize(&self) {
        self.bar.realize();
    }

    pub fn on_tool<F: FnMut(u16) + 'static>(&self, frame: &Frame, f: F) {
        self.bar.on_tool_clicked(frame, f);
    }

    /// Tool click with [`RibbonBarEvent`] payload (`wxRibbonBarEvent`).
    pub fn on_ribbon_event<F: FnMut(&crate::chrome::ribbon_bar_event::RibbonBarEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        self.on_tool(frame, move |id| {
            f(&crate::chrome::ribbon_bar_event::RibbonBarEvent::new(
                crate::chrome::ribbon_bar_event::RibbonBarEventKind::ToolClick,
                id,
            ));
        });
    }
}
