//! macOS menu bar status item (`NSStatusItem`) — system tray icon.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSImage, NSMenu, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSString};

use super::{mac_bitmap_image_at, NativeHandle};

thread_local! {
    static STATUS_ITEMS: RefCell<HashMap<NativeHandle, Retained<NSStatusItem>>> =
        RefCell::new(HashMap::new());
    static STATUS_LEFT_CLICK: RefCell<HashMap<NativeHandle, Box<dyn FnMut()>>> =
        RefCell::new(HashMap::new());
    static STATUS_ITEM_TARGETS: RefCell<HashMap<NativeHandle, Retained<StatusItemTarget>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct StatusItemTargetIvars {
    status_handle: Cell<NativeHandle>,
}

objc2::define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = StatusItemTargetIvars]
    struct StatusItemTarget;

    unsafe impl NSObjectProtocol for StatusItemTarget {}

    impl StatusItemTarget {
        #[unsafe(method(ruWxStatusItemClicked:))]
        fn status_clicked(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            let handle = self.ivars().status_handle.get();
            STATUS_LEFT_CLICK.with(|m| {
                if let Some(mut cb) = m.borrow_mut().remove(&handle) {
                    cb();
                    m.borrow_mut().insert(handle, cb);
                }
            });
        }
    }
);

impl StatusItemTarget {
    fn new(mtm: MainThreadMarker, status_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(StatusItemTargetIvars {
            status_handle: Cell::new(status_handle),
        });
        unsafe { objc2::msg_send![super(this), init] }
    }
}

pub fn status_item_create(tooltip: &str, bitmap_handle: NativeHandle) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = super::alloc_widget_handle();
    let bar = NSStatusBar::systemStatusBar();
    let item = bar.statusItemWithLength(NSVariableStatusItemLength);
    item.setToolTip(Some(&NSString::from_str(tooltip)));
    if let Some(image) = mac_bitmap_image_at(bitmap_handle) {
        image.setTemplate(true);
        item.setImage(Some(&image));
    }
    STATUS_ITEMS.with(|m| m.borrow_mut().insert(handle, item));
    handle
}

pub fn status_item_set_menu(handle: NativeHandle, menu: &NSMenu) {
    STATUS_ITEMS.with(|m| {
        if let Some(item) = m.borrow().get(&handle) {
            item.setMenu(Some(menu));
        }
    });
}

pub fn status_item_set_image(handle: NativeHandle, bitmap_handle: NativeHandle) {
    STATUS_ITEMS.with(|m| {
        if let Some(item) = m.borrow().get(&handle) {
            if let Some(image) = mac_bitmap_image_at(bitmap_handle) {
                image.setTemplate(true);
                item.setImage(Some(&image));
            }
        }
    });
}

pub fn status_item_set_tooltip(handle: NativeHandle, tooltip: &str) {
    STATUS_ITEMS.with(|m| {
        if let Some(item) = m.borrow().get(&handle) {
            item.setToolTip(Some(&NSString::from_str(tooltip)));
        }
    });
}

pub fn status_item_set_left_click<F: FnMut() + 'static>(handle: NativeHandle, callback: F) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    STATUS_LEFT_CLICK.with(|m| m.borrow_mut().insert(handle, Box::new(callback)));
    let target = StatusItemTarget::new(mtm, handle);
    STATUS_ITEM_TARGETS.with(|m| m.borrow_mut().insert(handle, target.clone()));
    STATUS_ITEMS.with(|m| {
        if let Some(item) = m.borrow().get(&handle) {
            if let Some(button) = item.button(mtm) {
                unsafe {
                    button.setTarget(Some(&target));
                    button.setAction(Some(sel!(ruWxStatusItemClicked:)));
                }
            }
        }
    });
}

pub fn status_item_remove(handle: NativeHandle) {
    STATUS_LEFT_CLICK.with(|m| m.borrow_mut().remove(&handle));
    STATUS_ITEM_TARGETS.with(|m| m.borrow_mut().remove(&handle));
    STATUS_ITEMS.with(|m| {
        if let Some(item) = m.borrow_mut().remove(&handle) {
            NSStatusBar::systemStatusBar().removeStatusItem(&item);
        }
    });
}
