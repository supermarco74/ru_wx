//! macOS [`TipWindow`], [`SplashScreen`], and [`MiniFrame`] helpers.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSImageScaling, NSImageView, NSTextField, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer,
};

use super::{
    alloc_widget_handle, init_application, mac_bitmap_image_at, ns_window_for_frame_handle,
    remove_container, store_container, NativeHandle, Rect,
};

thread_local! {
    static TIP_WINDOWS: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
    static TIP_LABELS: RefCell<HashMap<NativeHandle, Retained<NSTextField>>> =
        RefCell::new(HashMap::new());
    static SPLASH_WINDOWS: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
    static SPLASH_TIMERS: RefCell<HashMap<NativeHandle, Retained<NSTimer>>> =
        RefCell::new(HashMap::new());
    static MINI_FRAMES: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct SplashTimerTargetIvars {
    handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "RuWxSplashTimerTarget"]
    #[ivars = SplashTimerTargetIvars]
    struct SplashTimerTarget;

    unsafe impl NSObjectProtocol for SplashTimerTarget {}

    impl SplashTimerTarget {
        #[unsafe(method(splashTimerFired:))]
        fn splash_timer_fired(&self, _timer: &NSTimer) {
            let handle = self.ivars().handle.get();
            splash_close(handle);
        }
    }
);

impl SplashTimerTarget {
    fn new(mtm: MainThreadMarker, handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SplashTimerTargetIvars {
            handle: Cell::new(handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    static SPLASH_TIMER_TARGETS: RefCell<HashMap<NativeHandle, Retained<SplashTimerTarget>>> =
        RefCell::new(HashMap::new());
}

fn win32_screen_rect_to_ns(mtm: MainThreadMarker, rect: &Rect) -> NSRect {
    let screen_h = objc2_app_kit::NSScreen::mainScreen(mtm)
        .map(|s| s.frame().size.height)
        .unwrap_or(900.0);
    NSRect::new(
        NSPoint::new(rect.x as f64, screen_h - rect.y as f64 - rect.height as f64),
        NSSize::new(rect.width.max(1) as f64, rect.height.max(1) as f64),
    )
}

fn borderless_popup_at(
    mtm: MainThreadMarker,
    rect: Rect,
) -> (NativeHandle, Retained<NSWindow>, Retained<NSView>) {
    let handle = alloc_widget_handle();
    let ns_rect = win32_screen_rect_to_ns(mtm, &rect);
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            ns_rect,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setLevel(objc2_app_kit::NSFloatingWindowLevel);
    window.setHidesOnDeactivate(false);
    let content = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    content.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    window.setContentView(Some(&content));
    store_container(handle, content.clone());
    (handle, window, content)
}

pub fn tip_create(rect: Rect, text: &str) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    init_application();
    let (handle, window, content) = borderless_popup_at(mtm, rect);
    window.setLevel(objc2_app_kit::NSPopUpMenuWindowLevel);
    window.setIgnoresMouseEvents(true);
    let bounds = content.bounds();
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFrame(bounds);
    label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
    label.setBackgroundColor(Some(&NSColor::controlBackgroundColor()));
    label.setDrawsBackground(true);
    label.setBezeled(true);
    content.addSubview(&label);
    TIP_WINDOWS.with(|m| m.borrow_mut().insert(handle, window));
    TIP_LABELS.with(|m| m.borrow_mut().insert(handle, label));
    handle
}

pub fn tip_show(handle: NativeHandle) {
    TIP_WINDOWS.with(|m| {
        if let Some(win) = m.borrow().get(&handle) {
            win.orderFrontRegardless();
        }
    });
}

pub fn tip_set_text(handle: NativeHandle, text: &str) {
    TIP_LABELS.with(|m| {
        if let Some(label) = m.borrow().get(&handle) {
            label.setStringValue(&NSString::from_str(text));
        }
    });
}

pub fn tip_close(handle: NativeHandle) {
    TIP_LABELS.with(|m| m.borrow_mut().remove(&handle));
    if let Some(win) = TIP_WINDOWS.with(|m| m.borrow_mut().remove(&handle)) {
        win.close();
    }
    remove_container(handle);
}

pub fn splash_create(bitmap_handle: NativeHandle, x: i32, y: i32, w: u32, h: u32) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    init_application();
    let rect = Rect::new(x, y, w.max(1), h.max(1));
    let (handle, window, content) = borderless_popup_at(mtm, rect);
    if let Some(image) = mac_bitmap_image_at(bitmap_handle) {
        let icon = NSImageView::initWithFrame(NSImageView::alloc(mtm), content.bounds());
        icon.setImage(Some(&image));
        icon.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        icon.setAutoresizingMask(
            objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        content.addSubview(&icon);
    }
    SPLASH_WINDOWS.with(|m| m.borrow_mut().insert(handle, window));
    handle
}

pub fn splash_show(handle: NativeHandle) {
    SPLASH_WINDOWS.with(|m| {
        if let Some(win) = m.borrow().get(&handle) {
            win.makeKeyAndOrderFront(None);
        }
    });
}

pub fn splash_set_auto_close(handle: NativeHandle, milliseconds: u32) {
    if milliseconds == 0 {
        return;
    }
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let target = SplashTimerTarget::new(mtm, handle);
    let interval = milliseconds as f64 / 1000.0;
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            interval,
            &target,
            objc2::sel!(splashTimerFired:),
            None,
            false,
        )
    };
    SPLASH_TIMER_TARGETS.with(|m| m.borrow_mut().insert(handle, target));
    SPLASH_TIMERS.with(|m| m.borrow_mut().insert(handle, timer));
}

pub fn splash_close(handle: NativeHandle) {
    SPLASH_TIMERS.with(|m| {
        if let Some(timer) = m.borrow_mut().remove(&handle) {
            timer.invalidate();
        }
    });
    SPLASH_TIMER_TARGETS.with(|m| m.borrow_mut().remove(&handle));
    if let Some(win) = SPLASH_WINDOWS.with(|m| m.borrow_mut().remove(&handle)) {
        win.close();
    }
    remove_container(handle);
}

pub fn mini_frame_create(title: &str, width: u32, height: u32) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    init_application();
    let handle = alloc_widget_handle();
    let style =
        NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable;
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            NSRect::new(
                NSPoint::new(200.0, 200.0),
                NSSize::new(width as f64, height as f64),
            ),
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str(title));
    window.setLevel(objc2_app_kit::NSFloatingWindowLevel);
    let content = window
        .contentView()
        .expect("NSWindow must have a content view");
    content.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    super::FRAME_WINDOWS.with(|m| m.borrow_mut().insert(handle, window.clone()));
    store_container(handle, content.clone());
    MINI_FRAMES.with(|m| m.borrow_mut().insert(handle, window));
    handle
}

pub fn mini_frame_show(handle: NativeHandle, visible: bool) {
    if let Some(win) = ns_window_for_frame_handle(handle) {
        if visible {
            win.makeKeyAndOrderFront(None);
        } else {
            win.orderOut(None);
        }
    }
}

pub fn mini_frame_set_title(handle: NativeHandle, title: &str) {
    if let Some(win) = ns_window_for_frame_handle(handle) {
        win.setTitle(&NSString::from_str(title));
    }
}

pub fn mini_frame_destroy(handle: NativeHandle) {
    MINI_FRAMES.with(|m| {
        if let Some(win) = m.borrow_mut().remove(&handle) {
            win.close();
        }
    });
    super::FRAME_WINDOWS.with(|m| m.borrow_mut().remove(&handle));
    remove_container(handle);
}
