//! macOS [`NSSplitView`] wrapper for [`SplitterWindow`].

use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{NSSplitView, NSView};
use objc2_foundation::{MainThreadMarker, NSRect, NSSize, NSString};

use super::{
    alloc_widget_handle, parent_view_for, record_parent, ru_rect_to_ns, store_container,
    NativeHandle, Rect,
};

thread_local! {
    static SPLITVIEWS: RefCell<HashMap<NativeHandle, Retained<NSSplitView>>> =
        RefCell::new(HashMap::new());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitterOrientation {
    Vertical,
    Horizontal,
}

pub fn create_splitter(parent: NativeHandle, initial_rect: Rect, vertical: bool) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let content_h = super::parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let split = NSSplitView::initWithFrame(NSSplitView::alloc(mtm), ns_rect);
    split.setVertical(vertical);
    split.setDividerStyle(objc2_app_kit::NSSplitViewDividerStyle::Thin);
    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&split);
    }
    SPLITVIEWS.with(|m| m.borrow_mut().insert(handle, split.clone()));
    store_container(handle, split.into_super());
    record_parent(handle, parent);
    handle
}

pub fn splitter_set_panes(
    splitter: NativeHandle,
    pane1: NativeHandle,
    pane2: NativeHandle,
    orientation: SplitterOrientation,
) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let Some(split) = SPLITVIEWS.with(|m| m.borrow().get(&splitter).cloned()) else {
        return;
    };
    split.setVertical(orientation == SplitterOrientation::Vertical);
    for sub in split.subviews().iter().collect::<Vec<_>>() {
        sub.removeFromSuperview();
    }
    // `child_view_for_handle` consulta la mappa `VIEWS`, dove sono
    // registrate le NSScrollView di TreeCtrl/ListCtrl; `parent_view_for`
    // cerca solo in SCROLLED_DOCUMENTS/FRAME_CONTENT/CONTAINERS e non
    // troverebbe le due pane, lasciando lo splitter vuoto.
    if let Some(v1) = super::child_view_for_handle(pane1) {
        split.addSubview(&v1);
    }
    if let Some(v2) = super::child_view_for_handle(pane2) {
        split.addSubview(&v2);
    }
    let _ = mtm;
    split.adjustSubviews();
}

pub fn splitter_set_position(splitter: NativeHandle, position: f64) {
    let Some(split) = SPLITVIEWS.with(|m| m.borrow().get(&splitter).cloned()) else {
        return;
    };
    let subs = split.subviews();
    if subs.count() >= 2 {
        split.setPosition_ofDividerAtIndex(position, 0);
    }
}

pub fn splitter_resize(splitter: NativeHandle, width: u32, height: u32) {
    let Some(split) = SPLITVIEWS.with(|m| m.borrow().get(&splitter).cloned()) else {
        return;
    };
    split.setFrameSize(NSSize::new(width as f64, height as f64));
    split.adjustSubviews();
}

pub fn splitter_set_orientation(splitter: NativeHandle, orientation: SplitterOrientation) {
    let Some(split) = SPLITVIEWS.with(|m| m.borrow().get(&splitter).cloned()) else {
        return;
    };
    split.setVertical(orientation == SplitterOrientation::Vertical);
}

pub fn splitter_set_visible(splitter: NativeHandle, visible: bool) {
    let Some(split) = SPLITVIEWS.with(|m| m.borrow().get(&splitter).cloned()) else {
        return;
    };
    split.setHidden(!visible);
}

pub fn splitter_set_enabled(splitter: NativeHandle, enabled: bool) {
    let Some(split) = SPLITVIEWS.with(|m| m.borrow().get(&splitter).cloned()) else {
        return;
    };
    // NSSplitView doesn't have setEnabled. Disable by setting alpha.
    split.setAlphaValue(if enabled { 1.0 } else { 0.5 });
}
