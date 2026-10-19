//! Native list header strip (`wxHeaderCtrl`) for macOS.

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObjectProtocol};
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBezelStyle, NSButton, NSEvent, NSTextField, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

use super::{
    alloc_widget_handle, button_target_new, child_view_for_handle, parent_view_for, record_parent,
    set_view_frame, store_container, NativeHandle, Rect,
};

const DIVIDER_W: f64 = 5.0;

thread_local! {
    static HEADER_STRIPS: RefCell<std::collections::HashMap<NativeHandle, HeaderStripState>> =
        RefCell::new(std::collections::HashMap::new());
}

struct HeaderStripState {
    host: Retained<HeaderHostView>,
    header_id: u16,
    frame_handle: NativeHandle,
    columns: Vec<(Retained<NSTextField>, u32)>,
    dividers: Vec<Retained<HeaderDividerView>>,
    sort_column: Option<usize>,
}

#[derive(Default)]
struct HeaderHostIvars {}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = HeaderHostIvars]
    struct HeaderHostView;

    unsafe impl NSObjectProtocol for HeaderHostView {}

    impl HeaderHostView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }
    }
);

impl HeaderHostView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HeaderHostIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

#[derive(Default)]
struct HeaderDividerIvars {
    column_index: Cell<usize>,
    header_handle: Cell<NativeHandle>,
    drag_start_x: Cell<f64>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = HeaderDividerIvars]
    struct HeaderDividerView;

    unsafe impl NSObjectProtocol for HeaderDividerView {}

    impl HeaderDividerView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let loc = self.convertPoint_fromView(event.locationInWindow(), None);
            self.ivars().drag_start_x.set(loc.x);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let loc = self.convertPoint_fromView(event.locationInWindow(), None);
            let delta = (loc.x - self.ivars().drag_start_x.get()).round() as i32;
            if delta == 0 {
                return;
            }
            self.ivars().drag_start_x.set(loc.x);
            resize_header_column(
                self.ivars().header_handle.get(),
                self.ivars().column_index.get(),
                delta,
            );
        }
    }
);

impl HeaderDividerView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        header_handle: NativeHandle,
        column: usize,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HeaderDividerIvars {
            column_index: Cell::new(column),
            header_handle: Cell::new(header_handle),
            drag_start_x: Cell::new(0.0),
        });
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

fn resize_header_column(handle: NativeHandle, column: usize, delta: i32) {
    HEADER_STRIPS.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        if let Some((_, w)) = state.columns.get_mut(column) {
            *w = (*w as i32 + delta).max(24) as u32;
        }
        relayout_header_strip(state);
    });
}

fn relayout_header_strip(state: &mut HeaderStripState) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let height = state.host.bounds().size.height;
    let mut x = 0.0;
    for (idx, (field, width)) in state.columns.iter().enumerate() {
        let w = *width as f64;
        field.setFrame(NSRect::new(
            NSPoint::new(x + 2.0, 0.0),
            NSSize::new(w - DIVIDER_W - 4.0, height),
        ));
        let mut title = field.stringValue().to_string();
        if state.sort_column == Some(idx) && !title.ends_with('▾') {
            title.push('▾');
            field.setStringValue(&NSString::from_str(&title));
        }
        if let Some(div) = state.dividers.get(idx) {
            div.setFrame(NSRect::new(
                NSPoint::new(x + w - DIVIDER_W, 0.0),
                NSSize::new(DIVIDER_W, height),
            ));
        }
        x += w;
        let _ = mtm;
    }
}

pub fn create_header_ctrl(
    parent: NativeHandle,
    header_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let frame_handle = super::resolve_frame_handle(parent);
    let ns_rect = NSRect::new(
        NSPoint::new(initial_rect.x as f64, initial_rect.y as f64),
        NSSize::new(initial_rect.width as f64, initial_rect.height as f64),
    );
    let host = HeaderHostView::new(mtm, ns_rect);
    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&host);
    }
    store_container(handle, host.clone().into_super());
    record_parent(handle, parent);
    HEADER_STRIPS.with(|m| {
        m.borrow_mut().insert(
            handle,
            HeaderStripState {
                host,
                header_id,
                frame_handle,
                columns: Vec::new(),
                dividers: Vec::new(),
                sort_column: None,
            },
        );
    });
    handle
}

pub fn header_insert_column(handle: NativeHandle, index: usize, title: &str, width: u32) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    HEADER_STRIPS.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        let cmd = state.header_id.wrapping_add(index as u16 + 1);
        let target = button_target_new(mtm, state.frame_handle, cmd);
        let field = unsafe {
            NSTextField::initWithFrame(
                NSTextField::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width as f64, 20.0)),
            )
        };
        field.setEditable(false);
        field.setBezeled(false);
        field.setDrawsBackground(false);
        field.setStringValue(&NSString::from_str(title));
        unsafe {
            field.setTarget(Some(&target));
            field.setAction(Some(objc2::sel!(performClick:)));
        }
        state.host.addSubview(&field);

        let divider = HeaderDividerView::new(
            mtm,
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(DIVIDER_W, 20.0)),
            handle,
            index,
        );
        state.host.addSubview(&divider);

        if index <= state.columns.len() {
            state.columns.insert(index, (field, width));
            state.dividers.insert(index, divider);
        }
        relayout_header_strip(state);
    });
}

pub fn header_set_column_width(handle: NativeHandle, column: usize, width: u32) {
    HEADER_STRIPS.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        if let Some((_, w)) = state.columns.get_mut(column) {
            *w = width;
        }
        relayout_header_strip(state);
    });
}

pub fn header_set_sort_column(handle: NativeHandle, column: Option<usize>) {
    HEADER_STRIPS.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.get_mut(&handle) else {
            return;
        };
        state.sort_column = column;
        relayout_header_strip(state);
    });
}

pub fn header_sync_frame(_handle: NativeHandle, parent: NativeHandle, rect: Rect) {
    if let Some(view) = child_view_for_handle(_handle) {
        set_view_frame(parent, _handle, rect);
        HEADER_STRIPS.with(|m| {
            if let Some(state) = m.borrow_mut().get_mut(&_handle) {
                relayout_header_strip(state);
            }
        });
        let _ = view;
    }
}
