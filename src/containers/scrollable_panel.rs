//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Scrollable panel — a [`Panel`] with vertical scroll bars and
//! automatic content panning (wxScrolledWindow-style).
//!
//! Use [`ScrollablePanel::install`] on an outer panel (for example a
//! tab page), parent controls to [`ScrollablePanel::content`], then
//! call [`ScrollablePanel::set_content_sizer`]. The scroll area fills
//! the outer panel; when the content is taller than the view, a
//! vertical scroll bar appears and the inner content panel pans as
//! the user scrolls.

use std::cell::RefCell;
use std::rc::Rc;

use crate::containers::scrolled_window::{ScrollEvent, ScrolledWindow};
use crate::containers::sizer::BoxSizer;
use crate::window::panel::Panel;

const LINE_STEP: i32 = 16;
const CONTENT_PADDING: i32 = 12;

struct ScrollablePanelInner {
    scroll: ScrolledWindow,
    content: Panel,
    content_sizer: Option<BoxSizer>,
    /// Guards against re-entrant `relayout` (e.g. `on_resize` during `set_size`).
    in_relayout: bool,
    /// Minimum virtual content height even when the sizer measures smaller.
    min_content_height: i32,
}

/// A panel region with vertical scroll bars and automatic panning.
#[derive(Clone)]
pub struct ScrollablePanel {
    inner: Rc<RefCell<ScrollablePanelInner>>,
}

impl ScrollablePanel {
    /// Install a scrollable region inside `outer`. The outer panel's
    /// sizer is replaced with one that gives the scroll area all
    /// available space.
    pub fn install(outer: &Panel) -> Self {
        let scroll = ScrolledWindow::new(outer);
        let content = Panel::new_child(&scroll);

        let panel = ScrollablePanel {
            inner: Rc::new(RefCell::new(ScrollablePanelInner {
                scroll,
                content,
                content_sizer: None,
                in_relayout: false,
                min_content_height: 0,
            })),
        };

        let mut outer_sizer = BoxSizer::vertical();
        outer_sizer.add_with_proportion(panel.scroll_ref().as_widget_ref(), 1);
        outer.set_sizer(outer_sizer);

        panel.wire_scroll();
        panel.wire_resize();
        panel
    }

    /// Ensure the scrollable area is at least `h` pixels tall so a
    /// vertical scroll bar appears when the view is shorter.
    pub fn set_min_content_height(&self, h: i32) {
        self.inner.borrow_mut().min_content_height = h.max(0);
        self.relayout();
    }

    /// Inner panel where controls should be parented.
    pub fn content(&self) -> Panel {
        self.inner.borrow().content.clone()
    }

    /// Install the content sizer and compute the virtual scroll height
    /// from the sizer's minimum size.
    pub fn set_content_sizer(&self, sizer: BoxSizer) {
        self.inner.borrow_mut().content_sizer = Some(sizer);
        self.relayout();
    }

    /// Recompute virtual size and scroll-bar range (call after the
    /// parent panel / tab page has been resized).
    pub fn refresh(&self) {
        self.relayout();
    }

    fn scroll_ref(&self) -> ScrolledWindow {
        self.inner.borrow().scroll.clone()
    }

    fn relayout(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.in_relayout {
                return;
            }
            inner.in_relayout = true;
        }

        self.relayout_inner();

        self.inner.borrow_mut().in_relayout = false;
    }

    fn relayout_inner(&self) {
        let (view_w, view_h, y) = {
            let inner = self.inner.borrow();
            let rect = inner
                .scroll
                .as_widget_ref()
                .try_borrow()
                .map(|w| w.rect())
                .unwrap_or_else(|_| crate::core::geometry::Rect::new(0, 0, 200, 200));
            let y = inner.scroll.get_view_position().1;
            (rect.width.max(100), rect.height, y)
        };

        let virtual_h = {
            let min_h = self.inner.borrow().min_content_height;
            let mut inner = self.inner.borrow_mut();
            let Some(ref mut sizer) = inner.content_sizer else {
                return;
            };
            // Lay out at the view width with generous height to measure content.
            sizer.layout(0, 0, view_w, 2000);
            let (_, measured_h) = sizer.min_size();
            let virtual_h = min_h.max(measured_h + CONTENT_PADDING);
            sizer.layout(0, 0, view_w, virtual_h as u32);
            virtual_h
        };

        let inner = self.inner.borrow();
        inner.content.set_size(view_w, virtual_h as u32);
        inner.scroll.set_virtual_size(view_w as i32, virtual_h);
        let max_y = (virtual_h - view_h as i32).max(0);
        let y = y.clamp(0, max_y);
        inner.scroll.set_view_position(0, y);
        if let Ok(mut content) = inner.content.as_widget_ref().try_borrow_mut() {
            content.set_position(0, -y);
        }
    }

    fn pan_to(&self, y: i32) {
        let inner = self.inner.borrow();
        let view_h = inner.scroll.as_widget_ref().borrow().rect().height as i32;
        let (_, vh) = inner.scroll.get_virtual_size();
        let max_y = (vh - view_h).max(0);
        let y = y.clamp(0, max_y);
        drop(inner);

        self.inner.borrow().scroll.set_view_position(0, y);
        self.inner
            .borrow()
            .content
            .as_widget_ref()
            .borrow_mut()
            .set_position(0, -y);
    }

    fn wire_scroll(&self) {
        let panel = self.clone();
        let scroll = self.scroll_ref();
        scroll.on_scroll(move |ev: ScrollEvent| {
            let (_, y) = panel.inner.borrow().scroll.get_view_position();
            let view_h = panel
                .inner
                .borrow()
                .scroll
                .as_widget_ref()
                .borrow()
                .rect()
                .height as i32;
            let (_, vh) = panel.inner.borrow().scroll.get_virtual_size();
            let max_y = (vh - view_h).max(0);
            let target = match ev {
                ScrollEvent::LineUp => y - LINE_STEP,
                ScrollEvent::LineDown => y + LINE_STEP,
                ScrollEvent::PageUp => y - view_h,
                ScrollEvent::PageDown => y + view_h,
                ScrollEvent::ThumbRelease { position } => position,
                ScrollEvent::ThumbTrack { position } => position,
                ScrollEvent::Top => 0,
                ScrollEvent::Bottom => max_y,
                ScrollEvent::EndScroll => y,
            };
            panel.pan_to(target);
        });
    }

    fn wire_resize(&self) {
        let panel = self.clone();
        self.scroll_ref()
            .on_resize(move || panel.relayout());
    }
}
