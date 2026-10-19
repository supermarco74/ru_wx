//! macOS OpenGL canvas via [`NSOpenGLView`].
//!
//! Apple deprecates OpenGL in favour of Metal; this backend keeps portable
//! `GLCanvas` + `gl11` demos working on macOS. For new native-only renderers
//! prefer Metal/MetalKit via a dedicated canvas type.

use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{NSOpenGLContext, NSOpenGLView};
use objc2_foundation::MainThreadMarker;

use super::{
    alloc_widget_handle, parent_view_for, record_parent, ru_rect_to_ns, store_ns_view,
    NativeHandle, Rect,
};

thread_local! {
    static GL_VIEWS: RefCell<HashMap<NativeHandle, Retained<NSOpenGLView>>> =
        RefCell::new(HashMap::new());
}

pub fn create_gl_canvas(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let content_h = super::parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);

    let view = NSOpenGLView::initWithFrame(NSOpenGLView::alloc(mtm), ns_rect);
    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&view);
    }
    GL_VIEWS.with(|m| m.borrow_mut().insert(handle, view.clone()));
    store_ns_view(handle, view.into_super());
    record_parent(handle, parent);
    handle
}

pub fn gl_canvas_set_current(handle: NativeHandle) -> bool {
    let view = GL_VIEWS.with(|m| m.borrow().get(&handle).cloned());
    let Some(view) = view else {
        return false;
    };
    if let Some(ctx) = view.openGLContext() {
        ctx.makeCurrentContext();
        true
    } else {
        false
    }
}

pub fn gl_canvas_make_inactive(_handle: NativeHandle) -> bool {
    if NSOpenGLContext::currentContext().is_some() {
        NSOpenGLContext::clearCurrentContext();
        true
    } else {
        false
    }
}

pub fn gl_canvas_swap_buffers(handle: NativeHandle) -> bool {
    let view = GL_VIEWS.with(|m| m.borrow().get(&handle).cloned());
    let Some(view) = view else {
        return false;
    };
    if let Some(ctx) = view.openGLContext() {
        ctx.flushBuffer();
        true
    } else {
        false
    }
}

pub fn gl_canvas_is_current(handle: NativeHandle) -> bool {
    let view = GL_VIEWS.with(|m| m.borrow().get(&handle).cloned());
    let Some(view) = view else {
        return false;
    };
    let Some(current) = NSOpenGLContext::currentContext() else {
        return false;
    };
    view.openGLContext()
        .is_some_and(|ctx| Retained::as_ptr(&ctx) == Retained::as_ptr(&current))
}

pub fn gl_canvas_has_context(handle: NativeHandle) -> bool {
    GL_VIEWS.with(|m| {
        m.borrow()
            .get(&handle)
            .and_then(|v| v.openGLContext())
            .is_some()
    })
}

pub fn gl_canvas_destroy(handle: NativeHandle) {
    if handle == 0 {
        return;
    }
    let view = GL_VIEWS.with(|m| m.borrow_mut().remove(&handle));
    if let Some(view) = view {
        view.removeFromSuperview();
    }
    super::remove_stored_view(handle);
}
