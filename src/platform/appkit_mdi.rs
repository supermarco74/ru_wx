//! macOS MDI — parent window with child `NSWindow` documents and z-order.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::{
    alloc_widget_handle, init_application, install_frame_content_host, ns_window_for_frame_handle,
    record_parent, store_container, NativeHandle,
};

thread_local! {
    static MDI_PARENTS: RefCell<HashMap<NativeHandle, MdiParentState>> =
        RefCell::new(HashMap::new());
    static MDI_CHILDREN: RefCell<HashMap<NativeHandle, MdiChildState>> =
        RefCell::new(HashMap::new());
}

struct MdiParentState {
    window: Retained<NSWindow>,
    client: Retained<MdiClientView>,
    client_handle: NativeHandle,
    children: RefCell<Vec<NativeHandle>>,
    visible: Cell<bool>,
    delegate: Retained<MdiParentDelegate>,
}

struct MdiChildState {
    window: Retained<NSWindow>,
    parent_handle: NativeHandle,
    title: String,
    normal_frame: Cell<Option<NSRect>>,
    maximized: Cell<bool>,
}

#[derive(Default)]
struct MdiClientIvars {}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = MdiClientIvars]
    struct MdiClientView;

    unsafe impl NSObjectProtocol for MdiClientView {}

    impl MdiClientView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            NSColor::windowBackgroundColor().setFill();
            objc2_app_kit::NSBezierPath::fillRect(self.bounds());
        }
    }
);

impl MdiClientView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MdiClientIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

#[derive(Default)]
struct MdiParentDelegateIvars {
    parent_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "RuWxMdiParentDelegate"]
    #[ivars = MdiParentDelegateIvars]
    struct MdiParentDelegate;

    unsafe impl NSObjectProtocol for MdiParentDelegate {}

    unsafe impl NSWindowDelegate for MdiParentDelegate {}
);

impl MdiParentDelegate {
    fn new(mtm: MainThreadMarker, parent_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MdiParentDelegateIvars {
            parent_handle: Cell::new(parent_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn client_rect_for_parent(parent_handle: NativeHandle) -> NSRect {
    MDI_PARENTS.with(|m| {
        m.borrow()
            .get(&parent_handle)
            .map(|s| s.client.frame())
            .unwrap_or_else(|| NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0)))
    })
}

fn screen_rect_for_client(parent_handle: NativeHandle, x: i32, y: i32, w: i32, h: i32) -> NSRect {
    let client = client_rect_for_parent(parent_handle);
    let parent_win = ns_window_for_frame_handle(parent_handle);
    let origin = parent_win
        .as_ref()
        .and_then(|w| {
            w.contentView()
                .map(|v| v.convertPoint_toView(NSPoint::new(0.0, 0.0), None))
        })
        .unwrap_or(NSPoint::new(0.0, 0.0));
    let frame_origin = parent_win
        .map(|w| w.frame().origin)
        .unwrap_or(NSPoint::new(0.0, 0.0));
    NSRect::new(
        NSPoint::new(
            frame_origin.x + origin.x + x as f64,
            frame_origin.y + origin.y + (client.size.height - y as f64 - h as f64),
        ),
        NSSize::new(w.max(120) as f64, h.max(80) as f64),
    )
}

pub fn mdi_parent_create(title: &str, width: u32, height: u32) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    init_application();
    let handle = alloc_widget_handle();
    let client_handle = alloc_widget_handle();

    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;

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
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str(title));

    let content_host = install_frame_content_host(mtm, &window, handle);
    let bounds = content_host.bounds();
    let client = MdiClientView::new(mtm, bounds);
    client.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    content_host.addSubview(&client);
    store_container(client_handle, client.clone().into_super());

    let delegate = MdiParentDelegate::new(mtm, handle);
    window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    super::FRAME_WINDOWS.with(|map| {
        map.borrow_mut().insert(handle, window.clone());
    });

    MDI_PARENTS.with(|m| {
        m.borrow_mut().insert(
            handle,
            MdiParentState {
                window,
                client: client.clone(),
                client_handle,
                children: RefCell::new(Vec::new()),
                visible: Cell::new(false),
                delegate,
            },
        );
    });

    handle
}

pub fn mdi_parent_show(parent_handle: NativeHandle) {
    if let Some(win) = ns_window_for_frame_handle(parent_handle) {
        let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        win.makeKeyAndOrderFront(None);
        app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Regular);
        MDI_PARENTS.with(|m| {
            if let Some(s) = m.borrow().get(&parent_handle) {
                s.visible.set(true);
            }
        });
    }
}

pub fn mdi_parent_destroy(parent_handle: NativeHandle) {
    mdi_close_all_children(parent_handle);
    if let Some(win) = ns_window_for_frame_handle(parent_handle) {
        win.close();
    }
    MDI_PARENTS.with(|m| m.borrow_mut().remove(&parent_handle));
    super::FRAME_WINDOWS.with(|m| m.borrow_mut().remove(&parent_handle));
}

pub fn mdi_child_create(
    parent_handle: NativeHandle,
    title: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let _client_handle = alloc_widget_handle();
    let parent_win = ns_window_for_frame_handle(parent_handle);
    let Some(parent_win) = parent_win else {
        return handle;
    };

    let frame = screen_rect_for_client(parent_handle, x, y, w, h);
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;

    let child = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { child.setReleasedWhenClosed(false) };
    child.setTitle(&NSString::from_str(title));
    unsafe {
        child.setParentWindow(Some(&parent_win));
        parent_win.addChildWindow_ordered(&child, objc2_app_kit::NSWindowOrderingMode::Above);
    }
    child.makeKeyAndOrderFront(None);

    if let Some(content) = child.contentView() {
        store_container(handle, content.clone());
        record_parent(handle, parent_handle);
    }

    MDI_CHILDREN.with(|m| {
        m.borrow_mut().insert(
            handle,
            MdiChildState {
                window: child,
                parent_handle,
                title: title.to_string(),
                normal_frame: Cell::new(None),
                maximized: Cell::new(false),
            },
        );
    });
    MDI_PARENTS.with(|m| {
        if let Some(p) = m.borrow().get(&parent_handle) {
            p.children.borrow_mut().push(handle);
        }
    });
    mdi_child_activate(handle);
    handle
}

pub fn mdi_child_activate(child_handle: NativeHandle) {
    let info = MDI_CHILDREN.with(|m| {
        m.borrow()
            .get(&child_handle)
            .map(|s| (s.parent_handle, s.window.clone()))
    });
    let Some((parent_handle, child)) = info else {
        return;
    };
    if let Some(parent) = ns_window_for_frame_handle(parent_handle) {
        unsafe {
            parent.addChildWindow_ordered(&child, objc2_app_kit::NSWindowOrderingMode::Above);
        }
    }
    child.makeKeyAndOrderFront(None);
    MDI_PARENTS.with(|pm| {
        if let Some(p) = pm.borrow().get(&parent_handle) {
            let mut list = p.children.borrow_mut();
            list.retain(|&h| h != child_handle);
            list.push(child_handle);
        }
    });
}

pub fn mdi_child_maximize(child_handle: NativeHandle) {
    MDI_CHILDREN.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.get_mut(&child_handle) else {
            return;
        };
        if state.maximized.get() {
            return;
        }
        state.normal_frame.set(Some(state.window.frame()));
        state.maximized.set(true);
        let parent_handle = state.parent_handle;
        let frame = screen_rect_for_client(
            parent_handle,
            0,
            0,
            client_rect_for_parent(parent_handle).size.width as i32,
            client_rect_for_parent(parent_handle).size.height as i32,
        );
        state.window.setFrame_display(frame, true);
    });
}

pub fn mdi_child_restore(child_handle: NativeHandle) {
    MDI_CHILDREN.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.get_mut(&child_handle) else {
            return;
        };
        if let Some(frame) = state.normal_frame.take() {
            state.window.setFrame_display(frame, true);
        }
        state.maximized.set(false);
    });
}

pub fn mdi_child_close(child_handle: NativeHandle) {
    let parent_handle = MDI_CHILDREN.with(|m| {
        m.borrow_mut().remove(&child_handle).map(|s| {
            s.window.close();
            s.parent_handle
        })
    });
    if let Some(parent) = parent_handle {
        MDI_PARENTS.with(|m| {
            if let Some(p) = m.borrow().get(&parent) {
                p.children.borrow_mut().retain(|&h| h != child_handle);
            }
        });
    }
}

pub fn mdi_cascade(parent_handle: NativeHandle) {
    let children: Vec<NativeHandle> = MDI_PARENTS.with(|m| {
        m.borrow()
            .get(&parent_handle)
            .map(|p| p.children.borrow().clone())
            .unwrap_or_default()
    });
    let mut offset = 0i32;
    for ch in children {
        let w = 280i32;
        let h = 200i32;
        mdi_child_restore(ch);
        let frame = screen_rect_for_client(parent_handle, offset, offset, w, h);
        MDI_CHILDREN.with(|m| {
            if let Some(s) = m.borrow().get(&ch) {
                s.window.setFrame_display(frame, true);
            }
        });
        offset += 30;
    }
}

pub fn mdi_tile(parent_handle: NativeHandle, horizontal: bool) {
    let children: Vec<NativeHandle> = MDI_PARENTS.with(|m| {
        m.borrow()
            .get(&parent_handle)
            .map(|p| p.children.borrow().clone())
            .unwrap_or_default()
    });
    if children.is_empty() {
        return;
    }
    let client = client_rect_for_parent(parent_handle);
    let cw = client.size.width as i32;
    let ch = client.size.height as i32;
    let n = children.len() as i32;
    for (i, ch_handle) in children.iter().enumerate() {
        mdi_child_restore(*ch_handle);
        let (x, y, w, h) = if horizontal {
            let row_h = (ch / n).max(80);
            (0, i as i32 * row_h, cw, row_h)
        } else {
            let col_w = (cw / n).max(120);
            (i as i32 * col_w, 0, col_w, ch)
        };
        let frame = screen_rect_for_client(parent_handle, x, y, w, h);
        MDI_CHILDREN.with(|m| {
            if let Some(s) = m.borrow().get(ch_handle) {
                s.window.setFrame_display(frame, true);
            }
        });
    }
}

pub fn mdi_close_all_children(parent_handle: NativeHandle) {
    let children: Vec<NativeHandle> = MDI_PARENTS.with(|m| {
        m.borrow()
            .get(&parent_handle)
            .map(|p| p.children.borrow().clone())
            .unwrap_or_default()
    });
    for ch in children {
        mdi_child_close(ch);
    }
}

pub fn mdi_parent_client_handle(parent_handle: NativeHandle) -> NativeHandle {
    MDI_PARENTS.with(|m| {
        m.borrow()
            .get(&parent_handle)
            .map(|s| s.client_handle)
            .unwrap_or(0)
    })
}

pub fn mdi_parent_handle_for_child(child_handle: NativeHandle) -> Option<NativeHandle> {
    MDI_CHILDREN.with(|m| m.borrow().get(&child_handle).map(|s| s.parent_handle))
}
