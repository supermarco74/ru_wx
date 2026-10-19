//! macOS standalone [`ScrollBar`] backed by [`NSScroller`] with full event wiring.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSScroller, NSScrollerPart, NSScrollerStyle};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};

use super::{
    alloc_widget_handle, dispatch_scroll, parent_view_for, record_parent, resolve_frame_handle,
    ru_rect_to_ns, store_ns_view, NativeHandle, Rect,
};

/// Win32 `SB_*` request codes (shared with [`crate::containers::scroll_bar`]).
const SB_LINEUP: u16 = 0;
const SB_LINEDOWN: u16 = 1;
const SB_PAGEUP: u16 = 2;
const SB_PAGEDOWN: u16 = 3;
const SB_THUMBPOSITION: u16 = 4;
const SB_THUMBTRACK: u16 = 5;
const SB_TOP: u16 = 6;
const SB_BOTTOM: u16 = 7;
const SB_ENDSCROLL: u16 = 8;

#[derive(Clone, Copy)]
struct ScrollBarState {
    min: i32,
    max: i32,
    page: i32,
    value: i32,
    vertical: bool,
    frame_handle: NativeHandle,
}

thread_local! {
    static SCROLLBAR_STATE: RefCell<HashMap<NativeHandle, RefCell<ScrollBarState>>> =
        RefCell::new(HashMap::new());
    static SCROLLBAR_TARGETS: RefCell<HashMap<NativeHandle, Retained<ScrollBarTarget>>> =
        RefCell::new(HashMap::new());
}

fn scroll_span(min: i32, max: i32) -> i32 {
    (max - min).max(1)
}

fn value_to_float(min: i32, max: i32, value: i32) -> f32 {
    let span = scroll_span(min, max) as f32;
    ((value - min) as f32 / span).clamp(0.0, 1.0)
}

fn float_to_value(min: i32, max: i32, float: f32) -> i32 {
    let span = scroll_span(min, max);
    min + (float * span as f32).round() as i32
}

fn knob_proportion(min: i32, max: i32, page: i32) -> f32 {
    let total = (max - min + page.max(1)).max(1) as f32;
    (page.max(1) as f32 / total).clamp(0.05, 1.0)
}

fn clamp_value(min: i32, max: i32, value: i32) -> i32 {
    value.max(min).min(max)
}

fn apply_scroller_values(scroller: &NSScroller, state: ScrollBarState) {
    let value = clamp_value(state.min, state.max, state.value);
    let float_val = value_to_float(state.min, state.max, value);
    let proportion = knob_proportion(state.min, state.max, state.page);
    scroller.setFloatValue(float_val);
    scroller.setKnobProportion(proportion as f64);
}

fn scroller_for(handle: NativeHandle) -> Option<Retained<NSScroller>> {
    super::VIEWS.with(|map| {
        let view = map.borrow().get(&handle)?.clone();
        view.downcast::<NSScroller>().ok()
    })
}

fn part_to_scroll_code(
    part: NSScrollerPart,
    min: i32,
    max: i32,
    new_value: i32,
) -> Option<(u16, i32)> {
    match part {
        NSScrollerPart::DecrementLine => Some((SB_LINEUP, new_value)),
        NSScrollerPart::IncrementLine => Some((SB_LINEDOWN, new_value)),
        NSScrollerPart::DecrementPage => Some((SB_PAGEUP, new_value)),
        NSScrollerPart::IncrementPage => Some((SB_PAGEDOWN, new_value)),
        NSScrollerPart::Knob => {
            let tracking = NSEvent::pressedMouseButtons() & 1 != 0;
            if tracking {
                Some((SB_THUMBTRACK, new_value))
            } else {
                Some((SB_THUMBPOSITION, new_value))
            }
        }
        NSScrollerPart::KnobSlot => {
            if new_value <= min {
                Some((SB_TOP, min))
            } else if new_value >= max {
                Some((SB_BOTTOM, max))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Default)]
struct ScrollBarTargetIvars {
    scroll_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ScrollBarTargetIvars]
    struct ScrollBarTarget;

    unsafe impl NSObjectProtocol for ScrollBarTarget {}

    impl ScrollBarTarget {
        #[unsafe(method(ruWxScrollerAction:))]
        fn scroller_action(&self, sender: Option<&AnyObject>) {
            let Some(scroller) = sender.and_then(|s| s.downcast_ref::<NSScroller>()) else {
                return;
            };
            self.handle_scroller_action(scroller);
        }
    }
);

impl ScrollBarTarget {
    fn handle_scroller_action(&self, scroller: &NSScroller) {
        let handle = self.ivars().scroll_handle.get();
        let event = SCROLLBAR_STATE.with(|map| {
            let map_b = map.borrow();
            let state = map_b.get(&handle)?.borrow();
            let float = scroller.floatValue();
            let new_value = clamp_value(
                state.min,
                state.max,
                float_to_value(state.min, state.max, float),
            );
            let part = scroller.hitPart();
            Some((state.frame_handle, part, new_value, state.min, state.max))
        });
        let Some((frame_handle, part, new_value, min, max)) = event else {
            return;
        };
        let Some((code, pos)) = part_to_scroll_code(part, min, max, new_value) else {
            return;
        };
        SCROLLBAR_STATE.with(|map| {
            if let Some(state) = map.borrow().get(&handle) {
                state.borrow_mut().value = pos;
            }
        });
        apply_scroller_values(scroller, read_state(handle));
        dispatch_scroll(frame_handle, handle, code, pos);
        if matches!(part, NSScrollerPart::Knob) && NSEvent::pressedMouseButtons() & 1 == 0 {
            dispatch_scroll(frame_handle, handle, SB_ENDSCROLL, pos);
        } else if !matches!(part, NSScrollerPart::Knob) {
            dispatch_scroll(frame_handle, handle, SB_ENDSCROLL, pos);
        }
    }

    fn new(mtm: MainThreadMarker, scroll_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ScrollBarTargetIvars {
            scroll_handle: Cell::new(scroll_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn read_state(handle: NativeHandle) -> ScrollBarState {
    SCROLLBAR_STATE.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|s| *s.borrow())
            .unwrap_or(ScrollBarState {
                min: 0,
                max: 100,
                page: 10,
                value: 0,
                vertical: true,
                frame_handle: 0,
            })
    })
}

pub fn create_scroll_bar_native(
    parent: NativeHandle,
    initial_rect: Rect,
    vertical: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let frame_handle = resolve_frame_handle(parent);

    SCROLLBAR_STATE.with(|map| {
        map.borrow_mut().insert(
            handle,
            RefCell::new(ScrollBarState {
                min: 0,
                max: 100,
                page: 10,
                value: 0,
                vertical,
                frame_handle,
            }),
        );
    });

    let content_h = super::parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let scroller = NSScroller::initWithFrame(NSScroller::alloc(mtm), ns_rect);
    scroller.setEnabled(true);
    scroller.setScrollerStyle(NSScrollerStyle::Legacy);
    if !vertical {
        scroller.setFrame(NSRect::new(
            NSPoint::new(ns_rect.origin.x, ns_rect.origin.y),
            NSSize::new(ns_rect.size.width.max(32.0), ns_rect.size.height.max(16.0)),
        ));
    }

    let target = ScrollBarTarget::new(mtm, handle);
    SCROLLBAR_TARGETS.with(|map| map.borrow_mut().insert(handle, target.clone()));
    unsafe {
        scroller.setTarget(Some(&target));
        scroller.setAction(Some(sel!(ruWxScrollerAction:)));
    }

    apply_scroller_values(&scroller, read_state(handle));

    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&scroller);
    }
    store_ns_view(handle, scroller.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn scroll_bar_set_range_native(
    handle: NativeHandle,
    min: i32,
    max: i32,
    page: i32,
    value: i32,
) {
    SCROLLBAR_STATE.with(|map| {
        if let Some(state) = map.borrow().get(&handle) {
            let mut s = state.borrow_mut();
            s.min = min;
            s.max = max;
            s.page = page.max(1);
            s.value = clamp_value(min, max, value);
        }
    });
    if let Some(scroller) = scroller_for(handle) {
        apply_scroller_values(&scroller, read_state(handle));
    }
}

pub fn scroll_bar_get_position_native(handle: NativeHandle) -> i32 {
    if let Some(scroller) = scroller_for(handle) {
        let state = read_state(handle);
        let value = float_to_value(state.min, state.max, scroller.floatValue());
        let clamped = clamp_value(state.min, state.max, value);
        SCROLLBAR_STATE.with(|map| {
            if let Some(s) = map.borrow().get(&handle) {
                s.borrow_mut().value = clamped;
            }
        });
        clamped
    } else {
        read_state(handle).value
    }
}

pub fn scroll_bar_set_enabled_native(handle: NativeHandle, enabled: bool) {
    if let Some(scroller) = scroller_for(handle) {
        scroller.setEnabled(enabled);
    }
}
