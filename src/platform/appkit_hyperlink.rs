//! macOS [`HyperlinkCtrl`] — native link-styled [`NSButton`] with URL storage.

use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{
    NSBezelStyle, NSButton, NSColor, NSForegroundColorAttributeName, NSUnderlineStyle,
    NSUnderlineStyleAttributeName,
};
use objc2_foundation::{MainThreadMarker, NSMutableAttributedString, NSNumber, NSRange, NSString};

use super::{
    alloc_widget_handle, button_target_new, parent_view_for, record_parent, resolve_frame_handle,
    ru_rect_to_ns, store_ns_view, NativeHandle, Rect, MENU_TARGETS, VIEWS,
};

thread_local! {
    static HYPERLINK_URLS: RefCell<HashMap<NativeHandle, String>> = RefCell::new(HashMap::new());
}

fn link_attributed_title(label: &str) -> Retained<NSMutableAttributedString> {
    let text = NSString::from_str(label);
    let range = NSRange::new(0, text.len());
    let attr = NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &text);
    let color = NSColor::linkColor();
    let underline = NSNumber::numberWithInteger(NSUnderlineStyle::Single.0 as isize);
    unsafe {
        attr.addAttribute_value_range(&NSForegroundColorAttributeName, &color, range);
        attr.addAttribute_value_range(&NSUnderlineStyleAttributeName, &underline, range);
    }
    attr
}

fn button_for(handle: NativeHandle) -> Option<Retained<NSButton>> {
    VIEWS.with(|map| {
        let view = map.borrow().get(&handle)?.clone();
        view.downcast::<NSButton>().ok()
    })
}

fn apply_link_appearance(button: &NSButton, label: &str) {
    let title = link_attributed_title(label);
    button.setAttributedTitle(&title);
    button.setBezelStyle(NSBezelStyle::Inline);
    button.setBordered(false);
    button.setContentTintColor(Some(&NSColor::linkColor()));
}

pub fn create_hyperlink_native(
    parent: NativeHandle,
    label: &str,
    url: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    HYPERLINK_URLS.with(|map| {
        map.borrow_mut().insert(handle, url.to_string());
    });

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let frame_handle = resolve_frame_handle(parent);
    let target = button_target_new(mtm, frame_handle, command_id);
    MENU_TARGETS.with(|map| map.borrow_mut().push(target.clone()));

    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(label),
            Some(&target),
            Some(objc2::sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    apply_link_appearance(&button, label);
    button.setFrame(ru_rect_to_ns(content_h, &initial_rect));
    content.addSubview(&button);
    store_ns_view(handle, button.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn hyperlink_update(handle: NativeHandle, label: &str, url: &str) {
    HYPERLINK_URLS.with(|map| {
        map.borrow_mut().insert(handle, url.to_string());
    });
    if let Some(button) = button_for(handle) {
        apply_link_appearance(&button, label);
    }
}

pub fn hyperlink_get_url(handle: NativeHandle) -> Option<String> {
    HYPERLINK_URLS.with(|map| map.borrow().get(&handle).cloned())
}
