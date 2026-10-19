//! Office-style ribbon panel — [`NSBox`] with [`NSTextField`] label.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use objc2::rc::Retained;
#[allow(unused_imports)]
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, MainThreadOnly};
use objc2_app_kit::{
    NSBezelStyle, NSBox, NSBoxType, NSButton, NSCellImagePosition, NSColor, NSEvent, NSImage,
    NSImageScaling, NSPopUpButton, NSTextAlignment, NSTextField, NSTitlePosition, NSTrackingArea,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

use super::{
    alloc_widget_handle, parent_height_for, parent_view_for, record_parent, ru_rect_to_ns,
    NativeHandle, Rect,
};

// ── NSTrackingArea helper ────────────────────────────────────────────────────

const TRACK_ENTER_EXIT: usize = 0x01;
const TRACK_ACTIVE_ALWAYS: usize = 0x80;

fn add_tracking_area(view: &NSButton, rect: NSRect) {
    unsafe {
        let cls = class!(NSTrackingArea);
        let alloc_area: objc2::rc::Allocated<NSTrackingArea> = msg_send![cls, alloc];
        let options = TRACK_ENTER_EXIT | TRACK_ACTIVE_ALWAYS;
        let ta: Retained<NSTrackingArea> = msg_send![alloc_area, initWithRect: rect,
            options: options,
            owner: view,
            userInfo: None::<&objc2_foundation::NSDictionary>];
        view.addTrackingArea(&ta);
    }
}

// ── Ribbon Panel ─────────────────────────────────────────────────────────────

const PANEL_BG_R: f64 = 249.0 / 255.0;
const PANEL_BG_G: f64 = 249.0 / 255.0;
const PANEL_BG_B: f64 = 249.0 / 255.0;
const LABEL_R: f64 = 64.0 / 255.0;
const LABEL_G: f64 = 64.0 / 255.0;
const LABEL_B: f64 = 64.0 / 255.0;

thread_local! {
    static RIBBON_PANELS: RefCell<HashMap<NativeHandle, (Retained<NSBox>, Retained<NSTextField>)>> = RefCell::new(HashMap::new());
}

pub fn create_ribbon_panel(parent: NativeHandle, label: &str, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);

    let box_view = NSBox::initWithFrame(NSBox::alloc(mtm), ns_rect);
    box_view.setBoxType(NSBoxType::Custom);
    box_view.setBorderWidth(0.0);
    box_view.setBorderColor(&NSColor::clearColor());
    box_view.setFillColor(&NSColor::colorWithCalibratedRed_green_blue_alpha(
        PANEL_BG_R, PANEL_BG_G, PANEL_BG_B, 1.0,
    ));
    box_view.setTitle(&NSString::from_str(""));
    box_view.setTitlePosition(NSTitlePosition::NoTitle);

    let label_h = 18.0;
    let label_y = ns_rect.size.height - label_h - 2.0;
    let label_rect = NSRect::new(
        NSPoint::new(0.0, label_y),
        NSSize::new(ns_rect.size.width, label_h),
    );
    let label_field = NSTextField::initWithFrame(NSTextField::alloc(mtm), label_rect);
    label_field.setStringValue(&NSString::from_str(label));
    label_field.setEditable(false);
    label_field.setSelectable(false);
    label_field.setBezeled(false);
    label_field.setDrawsBackground(false);
    label_field.setFont(Some(&objc2_app_kit::NSFont::boldSystemFontOfSize(11.0)));
    label_field.setTextColor(Some(&NSColor::colorWithCalibratedRed_green_blue_alpha(
        LABEL_R, LABEL_G, LABEL_B, 1.0,
    )));
    label_field.setAlignment(NSTextAlignment::Center);
    label_field.setHidden(true);

    RIBBON_PANELS.with(|m| {
        m.borrow_mut()
            .insert(handle, (box_view.clone(), label_field.clone()));
    });
    box_view.addSubview(&label_field);
    content.addSubview(&box_view);
    record_parent(handle, parent);
    handle
}

pub fn ribbon_panel_set_label(handle: NativeHandle, label: &str) {
    RIBBON_PANELS.with(|m| {
        if let Some((_, lbl)) = m.borrow().get(&handle) {
            lbl.setStringValue(&NSString::from_str(label));
            lbl.setHidden(false);
        }
    });
}

pub fn ribbon_panel_set_frame(_parent: NativeHandle, handle: NativeHandle, rect: Rect) {
    let content_h = parent_height_for(_parent);
    let ns_rect = ru_rect_to_ns(content_h, &rect);
    RIBBON_PANELS.with(|m| {
        if let Some((box_view, lbl)) = m.borrow().get(&handle) {
            box_view.setFrame(ns_rect);
            let h = ns_rect.size.height;
            let ly = h - 18.0 - 2.0;
            lbl.setFrame(NSRect::new(
                NSPoint::new(0.0, ly),
                NSSize::new(ns_rect.size.width, 18.0),
            ));
        }
    });
}

// ── Ribbon Buttons ───────────────────────────────────────────────────────────

thread_local! { static RIBBON_BUTTONS: RefCell<HashMap<NativeHandle, Retained<NSButton>>> = RefCell::new(HashMap::new()); }
thread_local! { static BTN_HOVER: RefCell<HashMap<usize, (Option<Retained<NSImage>>, Option<Retained<NSImage>>)>> = RefCell::new(HashMap::new()); }
thread_local! { static BTN_HOVERING: RefCell<HashSet<usize>> = RefCell::new(HashSet::new()); }
thread_local! { static RIBBON_DROPDOWNS: RefCell<HashMap<NativeHandle, Retained<NSPopUpButton>>> = RefCell::new(HashMap::new()); }
thread_local! { static HOVER_TIMER_ID: Cell<u32> = Cell::new(0); }

#[allow(deprecated)]
pub fn create_ribbon_button(
    parent: NativeHandle,
    image_data: Option<(u32, u32, &[u8])>,
    hover_data: Option<(u32, u32, &[u8])>,
    label: Option<&str>,
    img_pos: NSCellImagePosition,
    frame: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit main thread");
    let handle = alloc_widget_handle();
    let Some(panel) = parent_view_for(parent) else {
        return handle;
    };
    let ch = panel.frame().size.height;
    let nr = ru_rect_to_ns(ch, &frame);

    let btn = NSButton::initWithFrame(NSButton::alloc(mtm), nr);
    btn.setBezelStyle(NSBezelStyle::Rounded);

    let mut normal_img = None;
    let mut hover_img = None;

    if let Some((w, h, rgba)) = image_data {
        let img = super::nsimage_from_rgba(mtm, rgba, w as i32, h as i32);
        normal_img = Some(img.clone());
        btn.setImage(Some(&img));
        btn.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        btn.setImagePosition(img_pos);
    }

    if let Some((w, h, rgba)) = hover_data {
        let img = super::nsimage_from_rgba(mtm, rgba, w as i32, h as i32);
        hover_img = Some(img);
    }

    if let Some(lbl) = label {
        btn.setTitle(&NSString::from_str(lbl));
        btn.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(11.0)));
    }

    // Store hover data
    let ptr = &*btn as *const NSButton as usize;
    BTN_HOVER.with(|m| {
        m.borrow_mut().insert(ptr, (normal_img, hover_img));
    });

    // Add tracking area
    add_tracking_area(&btn, btn.bounds());

    panel.addSubview(&btn);
    record_parent(handle, parent);
    RIBBON_BUTTONS.with(|m| {
        m.borrow_mut().insert(handle, btn);
    });
    handle
}

pub fn ribbon_button_set_frame(_parent: NativeHandle, handle: NativeHandle, rect: Rect) {
    let ch = parent_height_for(_parent);
    let nr = ru_rect_to_ns(ch, &rect);
    RIBBON_BUTTONS.with(|m| {
        if let Some(btn) = m.borrow().get(&handle) {
            btn.setFrame(nr);
        }
    });
}

pub fn set_button_handler(btn: NativeHandle, frame: NativeHandle, cmd: u16) {
    let mtm = MainThreadMarker::new().expect("main thread");
    let target = super::button_target_new(mtm, frame, cmd);
    RIBBON_BUTTONS.with(|m| {
        if let Some(b) = m.borrow().get(&btn) {
            unsafe {
                b.setTarget(Some(&target));
                b.setAction(Some(objc2::sel!(clicked:)));
            }
        }
    });
}

/// Check all ribbon buttons for mouse hover, swapping images or applying visual feedback.
/// Called periodically by the hover timer (every 50 ms).
pub fn update_hover_state() {
    let screen_mouse = NSEvent::mouseLocation();

    // Find which button (if any) is under the mouse via AppKit hit testing.
    let hovered_ptr: Option<usize> = RIBBON_BUTTONS.with(|btn_map| {
        let btns = btn_map.borrow();
        if btns.is_empty() {
            return None;
        }

        // Get the first button's window for coordinate conversion
        let first_btn = btns.values().next()?;
        let window = first_btn.window()?;
        let window_point = window.convertPointFromScreen(screen_mouse);
        let content_view = window.contentView()?;

        // Use AppKit hit testing — most reliable approach
        let hit_view: Option<Retained<objc2_app_kit::NSView>> =
            unsafe { msg_send![&content_view, hitTest: window_point] };

        let hit = hit_view?;
        let hit_ptr = &*hit as *const objc2_app_kit::NSView as usize;
        for (_handle, btn) in btns.iter() {
            let btn_ptr = &**btn as *const NSButton as usize;
            if btn_ptr == hit_ptr {
                return Some(btn_ptr);
            }
        }
        None
    });

    let hovered = hovered_ptr.unwrap_or(0);
    let was_hovering = if hovered != 0 {
        BTN_HOVERING.with(|h| h.borrow().contains(&hovered))
    } else {
        false
    };

    // Mouse entered a new button
    if hovered != 0 && !was_hovering {
        RIBBON_BUTTONS.with(|btn_map| {
            let btns = btn_map.borrow();
            for (_h, b) in btns.iter() {
                let p = &**b as *const NSButton as usize;
                if p == hovered {
                    BTN_HOVER.with(|m| {
                        if let Some((_normal, hover)) = m.borrow().get(&p) {
                            if let Some(ref hover_img) = hover {
                                b.setImage(Some(hover_img));
                            } else {
                                #[allow(deprecated)]
                                b.setBezelStyle(NSBezelStyle::RegularSquare);
                                b.setBordered(true);
                                b.setContentTintColor(Some(
                                    &NSColor::colorWithCalibratedRed_green_blue_alpha(
                                        0.0, 0.478, 1.0, 1.0,
                                    ),
                                ));
                            }
                        }
                    });
                    break;
                }
            }
        });
    }

    // Mouse left all buttons — restore previous hovered buttons
    BTN_HOVERING.with(|prev| {
        let mut prev_set = prev.borrow_mut();
        for &ptr in prev_set.iter() {
            if ptr != hovered {
                RIBBON_BUTTONS.with(|btn_map| {
                    let btns = btn_map.borrow();
                    for (_h, b) in btns.iter() {
                        let p = &**b as *const NSButton as usize;
                        if p == ptr {
                            BTN_HOVER.with(|m| {
                                if let Some((normal, _hover)) = m.borrow().get(&ptr) {
                                    if let Some(ref normal_img) = normal {
                                        b.setImage(Some(normal_img));
                                    } else {
                                        #[allow(deprecated)]
                                        b.setBezelStyle(NSBezelStyle::Rounded);
                                        b.setBordered(true);
                                        b.setContentTintColor(None);
                                    }
                                }
                            });
                            break;
                        }
                    }
                });
            }
        }

        prev_set.clear();
        if hovered != 0 {
            prev_set.insert(hovered);
        }
    });
}

/// Start the hover polling timer (50 ms interval). Safe to call multiple times.
pub fn start_hover_timer() {
    HOVER_TIMER_ID.with(|id| {
        let tid = id.get();
        if tid != 0 {
            // Already running — just restart
            super::timer_start(tid, 50, true);
            return;
        }
        let new_id = crate::platform::next_timer_id();
        id.set(new_id);
        super::timer_set_fire(new_id, Box::new(update_hover_state));
        super::timer_start(new_id, 50, true);
    });
}

#[allow(deprecated)]
pub fn create_ribbon_dropdown(parent: NativeHandle, title: &str, frame: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("main thread");
    let handle = alloc_widget_handle();
    let Some(panel) = parent_view_for(parent) else {
        return handle;
    };
    let ch = panel.frame().size.height;
    let nr = ru_rect_to_ns(ch, &frame);
    let popup = NSPopUpButton::initWithFrame(NSPopUpButton::alloc(mtm), nr);
    popup.removeAllItems();
    popup.setTitle(&NSString::from_str(title));
    popup.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(11.0)));
    popup.setBezelStyle(NSBezelStyle::RoundRect);
    panel.addSubview(&popup);
    record_parent(handle, parent);
    RIBBON_DROPDOWNS.with(|m| {
        m.borrow_mut().insert(handle, popup);
    });
    handle
}

pub fn ribbon_dropdown_add_item(h: NativeHandle, item: &str) {
    RIBBON_DROPDOWNS.with(|m| {
        if let Some(popup) = m.borrow().get(&h) {
            popup.addItemWithTitle(&NSString::from_str(item));
        }
    });
}
