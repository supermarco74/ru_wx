//! macOS modal dialog windows and progress utility windows.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSButton, NSProgressIndicator, NSProgressIndicatorStyle,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use super::{alloc_widget_handle, ns_window_for_frame_handle, store_container, NativeHandle};

/// Parts of the progress sheet: message field, bar and optional cancel button.
type ProgressParts = (
    Retained<objc2_app_kit::NSTextField>,
    Retained<NSProgressIndicator>,
    Option<Retained<NSButton>>,
);

thread_local! {
    static DIALOG_WINDOWS: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
    static PROGRESS_WINDOWS: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
    static PROGRESS_PARTS: RefCell<HashMap<NativeHandle, ProgressParts>> =
        RefCell::new(HashMap::new());
    static PROGRESS_CANCELLED: RefCell<HashMap<NativeHandle, Cell<bool>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct ProgressCancelTargetIvars {
    progress_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ProgressCancelTargetIvars]
    struct ProgressCancelTarget;

    unsafe impl NSObjectProtocol for ProgressCancelTarget {}

    impl ProgressCancelTarget {
        #[unsafe(method(ruWxProgressCancel:))]
        fn progress_cancel(&self, _sender: Option<&AnyObject>) {
            let handle = self.ivars().progress_handle.get();
            PROGRESS_CANCELLED.with(|m| {
                if let Some(flag) = m.borrow().get(&handle) {
                    flag.set(true);
                }
            });
        }
    }
);

impl ProgressCancelTarget {
    fn new(mtm: MainThreadMarker, progress_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ProgressCancelTargetIvars {
            progress_handle: Cell::new(progress_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn create_dialog_window(
    parent: NativeHandle,
    title: &str,
    width: u32,
    height: u32,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let style =
        NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            NSRect::new(
                NSPoint::new(100.0, 100.0),
                NSSize::new(width as f64, height as f64),
            ),
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str(title));
    if let Some(parent_win) = ns_window_for_frame_handle(parent) {
        window.setFrameOrigin(NSPoint::new(
            parent_win.frame().origin.x + 40.0,
            parent_win.frame().origin.y + 40.0,
        ));
    }
    if let Some(content) = window.contentView() {
        store_container(handle, content);
    }
    DIALOG_WINDOWS.with(|m| m.borrow_mut().insert(handle, window));
    handle
}

pub fn dialog_content_view(handle: NativeHandle) -> Option<Retained<objc2_app_kit::NSView>> {
    DIALOG_WINDOWS.with(|m| m.borrow().get(&handle).and_then(|w| w.contentView()))
}

pub fn dialog_show_modal(handle: NativeHandle) -> i32 {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let Some(window) = DIALOG_WINDOWS.with(|m| m.borrow().get(&handle).cloned()) else {
        return 0;
    };
    window.makeKeyAndOrderFront(None);
    let response = app.runModalForWindow(&window);
    window.orderOut(None);
    if response == objc2_app_kit::NSModalResponseOK {
        1
    } else {
        0
    }
}

pub fn dialog_present(handle: NativeHandle) {
    if let Some(window) = DIALOG_WINDOWS.with(|m| m.borrow().get(&handle).cloned()) {
        window.makeKeyAndOrderFront(None);
    }
}

pub fn dialog_run_until<F: FnMut() -> bool>(handle: NativeHandle, mut keep_running: F) {
    dialog_present(handle);
    while keep_running() {
        super::wait_for_next_event();
    }
}

pub fn dialog_close(handle: NativeHandle) {
    if let Some(window) = DIALOG_WINDOWS.with(|m| m.borrow_mut().remove(&handle)) {
        window.close();
    }
}

pub fn create_progress_window(
    title: &str,
    message: &str,
    range: i32,
    has_cancel: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let window_h = if has_cancel { 150.0 } else { 120.0 };
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(420.0, window_h)),
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str(title));
    let content = window.contentView().expect("content view");
    let label = objc2_app_kit::NSTextField::labelWithString(&NSString::from_str(message), mtm);
    label.setFrame(NSRect::new(
        NSPoint::new(16.0, window_h - 48.0),
        NSSize::new(388.0, 20.0),
    ));
    content.addSubview(&label);
    let gauge = NSProgressIndicator::initWithFrame(
        NSProgressIndicator::alloc(mtm),
        NSRect::new(
            NSPoint::new(16.0, window_h - 84.0),
            NSSize::new(388.0, 20.0),
        ),
    );
    gauge.setStyle(NSProgressIndicatorStyle::Bar);
    gauge.setIndeterminate(false);
    gauge.setMinValue(0.0);
    gauge.setMaxValue(range.max(1) as f64);
    gauge.setDoubleValue(0.0);
    content.addSubview(&gauge);

    let cancel_btn = if has_cancel {
        let btn = NSButton::initWithFrame(
            NSButton::alloc(mtm),
            NSRect::new(NSPoint::new(314.0, 12.0), NSSize::new(90.0, 28.0)),
        );
        btn.setTitle(&NSString::from_str("Cancel"));
        btn.setBezelStyle(objc2_app_kit::NSBezelStyle::Push);
        let target = ProgressCancelTarget::new(mtm, handle);
        unsafe {
            btn.setTarget(Some(&target));
            btn.setAction(Some(sel!(ruWxProgressCancel:)));
        }
        content.addSubview(&btn);
        Some(btn)
    } else {
        None
    };

    PROGRESS_CANCELLED.with(|m| m.borrow_mut().insert(handle, Cell::new(false)));
    PROGRESS_WINDOWS.with(|m| m.borrow_mut().insert(handle, window));
    PROGRESS_PARTS.with(|m| {
        m.borrow_mut().insert(handle, (label, gauge, cancel_btn));
    });
    handle
}

pub fn progress_show(handle: NativeHandle) {
    if let Some(window) = PROGRESS_WINDOWS.with(|m| m.borrow().get(&handle).cloned()) {
        window.makeKeyAndOrderFront(None);
    }
}

pub fn progress_update(handle: NativeHandle, value: i32, message: &str) {
    if let Some((label, gauge, _)) = PROGRESS_PARTS.with(|m| m.borrow().get(&handle).cloned()) {
        label.setStringValue(&NSString::from_str(message));
        gauge.setDoubleValue(value as f64);
    }
    super::pump_pending_events();
}

pub fn progress_is_cancelled(handle: NativeHandle) -> bool {
    PROGRESS_CANCELLED.with(|m| m.borrow().get(&handle).map(|c| c.get()).unwrap_or(false))
}

pub fn progress_close(handle: NativeHandle) {
    PROGRESS_PARTS.with(|m| m.borrow_mut().remove(&handle));
    PROGRESS_CANCELLED.with(|m| m.borrow_mut().remove(&handle));
    if let Some(window) = PROGRESS_WINDOWS.with(|m| m.borrow_mut().remove(&handle)) {
        window.close();
    }
}
