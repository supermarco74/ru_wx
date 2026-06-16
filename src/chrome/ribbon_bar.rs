//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Office ribbon (`wxRibbonBar`) — tab strip plus per-page tool panels.

use std::cell::RefCell;
use std::rc::Rc;

use crate::chrome::ribbon_page::RibbonPage;
use crate::chrome::tool_bar::ToolBar;
use crate::containers::tab::Tab;
use crate::controls::static_text::StaticText;
use crate::core::geometry::Colour;
use crate::core::widget::Widget;
use crate::platform::next_control_id;
use crate::window::frame::Frame;
use crate::window::panel::Panel;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MoveWindow, SetWindowPos, ShowWindow, HWND_TOP, SW_HIDE, SWP_NOACTIVATE, SW_SHOW,
};

/// Height of the ribbon tab strip (page labels) in pixels.
const RIBBON_TAB_STRIP_H: u32 = 28;

#[derive(Clone)]
pub struct RibbonBar {
    frame: Frame,
    tab: Tab,
    pages: Rc<RefCell<Vec<RibbonPage>>>,
    page_toolbars: Rc<RefCell<Vec<ToolBar>>>,
    page_labels: Rc<RefCell<Vec<StaticText>>>,
    active_index: Rc<RefCell<usize>>,
    last_layout: Rc<RefCell<Option<(i32, i32, u32, u32)>>>,
}

impl RibbonBar {
    pub fn new(frame: &Frame) -> Self {
        Self {
            frame: frame.clone(),
            tab: Tab::new(frame),
            pages: Rc::new(RefCell::new(Vec::new())),
            page_toolbars: Rc::new(RefCell::new(Vec::new())),
            page_labels: Rc::new(RefCell::new(Vec::new())),
            active_index: Rc::new(RefCell::new(0)),
            last_layout: Rc::new(RefCell::new(None)),
        }
    }

    /// Tab control used as the ribbon page strip (`Home`, `Insert`, …).
    pub fn tab_widget(&self) -> crate::core::widget::WidgetRef {
        self.tab.as_widget_ref()
    }

    pub fn add_page(&self, page: RibbonPage) -> usize {
        let panel = Panel::new(&self.frame);
        panel.set_background_colour(Colour::new(245, 245, 245, 255));

        let toolbar = ToolBar::new(&self.frame);
        for bar in page.panels().iter().flat_map(|p| p.bars()) {
            for &(id, ref tool_label) in bar.buttons() {
                toolbar.add_tool(id, tool_label, 0);
            }
        }
        toolbar.realize();

        self.tab.add_page(&page.label, &panel);
        let idx = {
            let mut pages = self.pages.borrow_mut();
            pages.push(page);
            pages.len() - 1
        };
        self.page_toolbars.borrow_mut().push(toolbar);
        self.page_labels.borrow_mut().push(StaticText::new(&panel, ""));

        if idx == 0 {
            self.show_page(0);
        } else {
            self.set_toolbar_visible(idx, false);
        }
        self.tab.hide_all_pages();
        idx
    }

    fn set_toolbar_visible(&self, index: usize, visible: bool) {
        #[cfg(target_os = "windows")]
        if let Some(bar) = self.page_toolbars.borrow().get(index) {
            // SAFETY: show/hide an existing toolbar HWND.
            unsafe {
                ShowWindow(bar.hwnd(), if visible { SW_SHOW } else { SW_HIDE });
            }
        }
        if let Some(label) = self.page_labels.borrow().get(index) {
            label.as_widget_ref().borrow_mut().set_visible(false);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (index, visible);
    }

    fn show_page(&self, index: usize) {
        let count = self.page_toolbars.borrow().len();
        for i in 0..count {
            self.set_toolbar_visible(i, i == index);
        }
        *self.active_index.borrow_mut() = index;
        if let Some((x, y, w, h)) = *self.last_layout.borrow() {
            self.layout_toolbars(x, y, w, h);
        }
    }

    #[cfg(target_os = "windows")]
    fn layout_toolbars(&self, x: i32, y: i32, w: u32, h: u32) {
        let content_y = y + RIBBON_TAB_STRIP_H as i32;
        let content_h = h.saturating_sub(RIBBON_TAB_STRIP_H).max(32) as i32;
        let active = *self.active_index.borrow();
        for (i, bar) in self.page_toolbars.borrow().iter().enumerate() {
            if i == active {
                // SAFETY: toolbar HWND registered during `realize`.
                unsafe {
                    MoveWindow(bar.hwnd(), x, content_y, w as i32, content_h, 1);
                    SetWindowPos(
                        bar.hwnd(),
                        HWND_TOP,
                        x,
                        content_y,
                        w as i32,
                        content_h,
                        SWP_NOACTIVATE,
                    );
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn layout_toolbars(&self, _x: i32, _y: i32, _w: u32, _h: u32) {}

    /// Position the ribbon tab strip and the active page toolbar.
    pub fn layout(&self, x: i32, y: i32, w: u32, h: u32) {
        *self.last_layout.borrow_mut() = Some((x, y, w, h));
        self.tab.as_widget_ref().borrow_mut().set_position(x, y);
        self.tab.as_widget_ref().borrow_mut().set_size(w, h);
        self.tab.hide_all_pages();
        self.layout_toolbars(x, y, w, h);
        #[cfg(target_os = "windows")]
        {
            let tab_hwnd = self.tab.as_widget_ref().borrow().native_handle()
                as windows_sys::Win32::Foundation::HWND;
            // SAFETY: ribbon tab HWND is live during layout.
            unsafe {
                SetWindowPos(
                    tab_hwnd,
                    HWND_TOP,
                    x,
                    y,
                    w as i32,
                    h as i32,
                    SWP_NOACTIVATE,
                );
            }
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.borrow().len()
    }

    pub fn add_tool(&self, label: &str) -> u16 {
        let id = next_control_id();
        let active = *self.active_index.borrow();
        if let Some(bar) = self.page_toolbars.borrow().get(active) {
            bar.add_tool(id, label, 0);
            bar.realize();
        }
        id
    }

    pub fn realize(&self) {
        if !self.pages.borrow().is_empty() {
            self.show_page(0);
        }
        let outer = self.clone();
        self.tab.on_selection_change(&self.frame, move |index| {
            outer.show_page(index);
            outer.tab.hide_all_pages();
        });
    }

    pub fn on_tool<F: FnMut(u16) + 'static>(&self, frame: &Frame, f: F) {
        let callback = Rc::new(RefCell::new(f));
        for bar in self.page_toolbars.borrow().iter() {
            let cb = Rc::clone(&callback);
            bar.on_tool_clicked(frame, move |id| cb.borrow_mut()(id));
        }
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
