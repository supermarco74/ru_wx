//! Office-style ribbon tab strip — gradient header + title + segmented tabs.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSColor, NSControl, NSFont, NSSegmentStyle, NSSegmentSwitchTracking, NSSegmentedControl,
    NSTextAlignment, NSTextField, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::{
    alloc_widget_handle, parent_height_for, parent_view_for, record_parent, ru_rect_to_ns,
    NativeHandle, Rect,
};

thread_local! {
    /// Maps strip handle → (wrapper_view, gradient_views, segmented_control, title_label)
    static RIBBON_STRIPS: RefCell<HashMap<NativeHandle, (
        Retained<NSView>,
        Vec<Retained<NSView>>,
        Retained<NSSegmentedControl>,
        Retained<NSTextField>,
    )>> = RefCell::new(HashMap::new());

    static RIBBON_STRIP_LABELS: RefCell<HashMap<NativeHandle, RefCell<Vec<String>>>> =
        RefCell::new(HashMap::new());

    static RIBBON_STRIP_HANDLERS: RefCell<HashMap<NativeHandle, Box<dyn FnMut(usize)>>> =
        RefCell::new(HashMap::new());
}

// Windows 11 / Office 365 style - subtle gray/white gradient (top → bottom)
const GRADIENT_STOPS: &[(&str, f64, f64, f64, f64)] = &[
    // (label, R, G, B, alpha)
    ("top", 252.0 / 255.0, 252.0 / 255.0, 252.0 / 255.0, 1.0), // #FCFCFC (near white)
    ("upper", 248.0 / 255.0, 248.0 / 255.0, 248.0 / 255.0, 1.0), // #F8F8F8
    ("midup", 244.0 / 255.0, 244.0 / 255.0, 244.0 / 255.0, 1.0), // #F4F4F4
    ("center", 240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0, 1.0), // #F0F0F0
    ("low", 236.0 / 255.0, 236.0 / 255.0, 236.0 / 255.0, 1.0), // #ECECEC
    ("bottom", 232.0 / 255.0, 232.0 / 255.0, 232.0 / 255.0, 1.0), // #E8E8E8
];

#[derive(Default)]
struct RibbonStripTargetIvars {
    strip_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "RuWxRibbonStripTarget"]
    #[ivars = RibbonStripTargetIvars]
    struct RibbonStripTarget;

    unsafe impl NSObjectProtocol for RibbonStripTarget {}

    impl RibbonStripTarget {
        #[unsafe(method(segmentClicked:))]
        fn segment_clicked(&self, control: &NSControl) {
            let handle = self.ivars().strip_handle.get();
            let Some(segmented) = control.downcast_ref::<NSSegmentedControl>() else {
                return;
            };
            let index = segmented.selectedSegment() as usize;
            RIBBON_STRIP_HANDLERS.with(|map| {
                if let Some(handler) = map.borrow_mut().get_mut(&handle) {
                    handler(index);
                }
            });
        }
    }
);

impl RibbonStripTarget {
    fn new(mtm: MainThreadMarker, strip_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RibbonStripTargetIvars {
            strip_handle: Cell::new(strip_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    static RIBBON_STRIP_TARGETS: RefCell<HashMap<NativeHandle, Retained<RibbonStripTarget>>> =
        RefCell::new(HashMap::new());
}

fn sync_segment_labels(handle: NativeHandle) {
    let labels = RIBBON_STRIP_LABELS.with(|m| {
        m.borrow()
            .get(&handle)
            .map(|v| v.borrow().clone())
            .unwrap_or_default()
    });
    RIBBON_STRIPS.with(|m| {
        if let Some((_, _, control, _)) = m.borrow().get(&handle) {
            let ctrl = control.clone();
            ctrl.setSegmentCount(labels.len() as isize);
            for (i, label) in labels.iter().enumerate() {
                ctrl.setLabel_forSegment(&NSString::from_str(label), i as isize);
            }
            if !labels.is_empty() && ctrl.selectedSegment() < 0 {
                ctrl.setSelectedSegment(0);
            }
        }
    });
}

pub fn create_ribbon_tab_strip(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let frame = ru_rect_to_ns(content_h, &initial_rect);
    let frame_w = frame.size.width;
    let frame_h = frame.size.height;

    // ── Wrapper view (base layer) ───────────────────────────────────────────
    let wrapper = NSView::initWithFrame(NSView::alloc(mtm), frame);
    wrapper.setWantsLayer(true);
    // Base: darkest Office blue (#17375E)
    let base_color = NSColor::colorWithCalibratedRed_green_blue_alpha(
        GRADIENT_STOPS[0].1,
        GRADIENT_STOPS[0].2,
        GRADIENT_STOPS[0].3,
        GRADIENT_STOPS[0].4,
    );
    if let Some(layer) = wrapper.layer() {
        layer.setBackgroundColor(Some(&base_color.CGColor()));
    }

    // ── Gradient overlay layers ─────────────────────────────────────────────
    // Each layer adds a lighter blue at the bottom, creating gradient illusion
    let mut gradient_views: Vec<Retained<NSView>> = Vec::new();
    let num_stops = GRADIENT_STOPS.len() as f64;

    for (i, stop) in GRADIENT_STOPS.iter().enumerate() {
        if i == 0 {
            continue;
        } // skip top (base layer)

        let frac = (i as f64 + 1.0) / num_stops; // fraction from top
        let overlay_h = frame_h * frac; // how tall this overlay is
        let y_pos = 0.0; // all anchored at bottom

        let overlay_rect = NSRect::new(NSPoint::new(0.0, y_pos), NSSize::new(frame_w, overlay_h));
        let overlay = NSView::initWithFrame(NSView::alloc(mtm), overlay_rect);
        overlay.setWantsLayer(true);

        let color =
            NSColor::colorWithCalibratedRed_green_blue_alpha(stop.1, stop.2, stop.3, stop.4);
        if let Some(layer) = overlay.layer() {
            layer.setBackgroundColor(Some(&color.CGColor()));
        }
        // Place above base but below controls
        wrapper.addSubview(&overlay);
        gradient_views.push(overlay);
    }

    // ── NSSegmentedControl — Separated style on top of gradient ─────────────
    let tab_x = 200.0_f64; // space for the title on the left
    let tab_w = (frame_w - tab_x - 16.0).max(200.0);
    let tab_rect = NSRect::new(NSPoint::new(tab_x, 0.0), NSSize::new(tab_w, frame_h));
    let control = NSSegmentedControl::initWithFrame(NSSegmentedControl::alloc(mtm), tab_rect);
    control.setSegmentCount(0);
    control.setSegmentStyle(NSSegmentStyle::Separated);
    control.setTrackingMode(NSSegmentSwitchTracking::SelectOne);

    let target = RibbonStripTarget::new(mtm, handle);
    unsafe {
        control.setTarget(Some(&target));
        control.setAction(Some(sel!(segmentClicked:)));
    }

    // ── Title label (left side) ────────────────────────────────────────────
    let title_label_w = tab_x - 12.0;
    let title_rect = NSRect::new(NSPoint::new(8.0, 4.0), NSSize::new(title_label_w, 20.0));
    let title_field = NSTextField::initWithFrame(NSTextField::alloc(mtm), title_rect);
    title_field.setStringValue(&NSString::from_str("Nome modello scrittore: Composer"));
    title_field.setEditable(false);
    title_field.setSelectable(false);
    title_field.setBezeled(false);
    title_field.setDrawsBackground(false);
    title_field.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    title_field.setTextColor(Some(&NSColor::whiteColor()));
    title_field.setAlignment(NSTextAlignment::Center);

    // ── Assemble ───────────────────────────────────────────────────────────
    wrapper.addSubview(&title_field);
    wrapper.addSubview(&control);

    content.addSubview(&wrapper);
    RIBBON_STRIPS.with(|m| {
        m.borrow_mut()
            .insert(handle, (wrapper, gradient_views, control, title_field));
    });
    RIBBON_STRIP_LABELS.with(|m| m.borrow_mut().insert(handle, RefCell::new(Vec::new())));
    RIBBON_STRIP_TARGETS.with(|m| m.borrow_mut().insert(handle, target));
    record_parent(handle, parent);
    handle
}

pub fn ribbon_strip_add_page(strip_handle: NativeHandle, title: &str) -> i32 {
    RIBBON_STRIP_LABELS.with(|m| {
        if let Some(labels) = m.borrow().get(&strip_handle) {
            labels.borrow_mut().push(title.to_string());
        }
    });
    sync_segment_labels(strip_handle);
    RIBBON_STRIP_LABELS.with(|m| {
        m.borrow()
            .get(&strip_handle)
            .map(|v| v.borrow().len() as i32 - 1)
            .unwrap_or(-1)
    })
}

pub fn ribbon_strip_set_selection_handler(
    strip_handle: NativeHandle,
    handler: Box<dyn FnMut(usize)>,
) {
    RIBBON_STRIP_HANDLERS.with(|m| {
        m.borrow_mut().insert(strip_handle, handler);
    });
}

pub fn ribbon_strip_set_selection(strip_handle: NativeHandle, index: usize) {
    if let Some((_, _, control, _)) = RIBBON_STRIPS.with(|m| m.borrow().get(&strip_handle).cloned())
    {
        control.setSelectedSegment(index as isize);
    }
}

pub fn ribbon_strip_set_frame(_parent: NativeHandle, strip_handle: NativeHandle, rect: Rect) {
    let content_h = parent_height_for(_parent);
    let ns_rect = ru_rect_to_ns(content_h, &rect);
    if let Some((wrapper, gradient_views, control, title_field)) =
        RIBBON_STRIPS.with(|m| m.borrow().get(&strip_handle).cloned())
    {
        wrapper.setFrame(ns_rect);
        let frame_w = ns_rect.size.width;
        let frame_h = ns_rect.size.height;

        // Update base layer background
        let base_color = NSColor::colorWithCalibratedRed_green_blue_alpha(
            GRADIENT_STOPS[0].1,
            GRADIENT_STOPS[0].2,
            GRADIENT_STOPS[0].3,
            GRADIENT_STOPS[0].4,
        );
        if let Some(layer) = wrapper.layer() {
            layer.setBackgroundColor(Some(&base_color.CGColor()));
        }

        // Re-layout gradient overlays
        let num_stops = GRADIENT_STOPS.len() as f64;
        for (i, overlay) in gradient_views.iter().enumerate() {
            let stop_idx = i + 1; // skip top (base)
            if stop_idx < GRADIENT_STOPS.len() {
                let stop = GRADIENT_STOPS[stop_idx];
                let frac = (stop_idx as f64 + 1.0) / num_stops;
                let overlay_h = frame_h * frac;
                let overlay_rect =
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(frame_w, overlay_h));
                overlay.setFrame(overlay_rect);
                let color = NSColor::colorWithCalibratedRed_green_blue_alpha(
                    stop.1, stop.2, stop.3, stop.4,
                );
                if let Some(layer) = overlay.layer() {
                    layer.setBackgroundColor(Some(&color.CGColor()));
                }
            }
        }

        // Re-layout tab control and title
        let tab_x = 200.0_f64;
        let tab_w = (frame_w - tab_x - 16.0).max(200.0);
        let tab_rect = NSRect::new(NSPoint::new(tab_x, 0.0), NSSize::new(tab_w, frame_h));
        control.setFrame(tab_rect);

        let title_w = tab_x - 12.0;
        let title_rect = NSRect::new(NSPoint::new(8.0, 4.0), NSSize::new(title_w, 20.0));
        title_field.setFrame(title_rect);
    }
}

pub fn ribbon_strip_page_count(strip_handle: NativeHandle) -> usize {
    RIBBON_STRIP_LABELS.with(|m| {
        m.borrow()
            .get(&strip_handle)
            .map(|v| v.borrow().len())
            .unwrap_or(0)
    })
}
