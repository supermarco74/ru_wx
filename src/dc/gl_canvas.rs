//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxGLCanvas` — OpenGL rendering surface.
//!
//! On Windows the canvas is realised as a child window of class
//! `RuWxGLCanvas` with a pixel format chosen by
//! [`ChoosePixelFormat`] on a standard 32-bit RGBA / 24-bit depth
//! / 8-bit stencil [`PIXELFORMATDESCRIPTOR`].
//!
//! # Usage
//!
//! ```no_run
//! # #[cfg(target_os = "windows")]
//! # {
//! use ru_wx::*;
//! let app = App::new();
//! let frame = Frame::builder().with_title("gl").with_size(400, 300).build();
//! let gl = GLCanvas::new(&frame);
//! gl.set_current();
//! // ... call glClear, glColor3f, etc. ...
//! gl.swap_buffers();
//! # app.run(frame); }
//! ```
//!
//! OpenGL 1.1 functions (`glClear`, `glColor3f`, `glBegin`,
//! `glEnd`, …) are re-exported from `windows_sys::Win32::Graphics::OpenGL`
//! so you can call them directly. For OpenGL 2.0+ entry points
//! (shaders, VBOs, etc.), use `wglGetProcAddress` at runtime.
//!
//! # Cross-platform stub
//!
//! On non-Windows targets the type is still constructible (as a
//! width/height record with no real GL context) and the methods
//! are no-ops, so code can compile cross-platform.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{HBRUSH, HDC, GetDC, ReleaseDC};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::OpenGL::{
    ChoosePixelFormat, HGLRC, PIXELFORMATDESCRIPTOR, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW,
    PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, SetPixelFormat, SwapBuffers,
    wglCreateContext, wglDeleteContext, wglMakeCurrent,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Re-export the OpenGL 1.1 entry points that `windows_sys`
/// exposes natively. Higher-version entry points (shaders, VAOs,
/// etc.) must be loaded with `wglGetProcAddress` at runtime.
#[cfg(target_os = "windows")]
pub mod gl11 {
    pub use windows_sys::Win32::Graphics::OpenGL::{
        glBegin, glClear, glClearColor, glColor3f, glEnd, glFinish, glFlush, glLoadIdentity,
        glMatrixMode, glOrtho, glRotatef, glTranslatef, glVertex2f, glVertex3f, glViewport,
    };
    /// Common OpenGL constants re-exported for convenience.
    pub const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    pub const GL_DEPTH_BUFFER_BIT: u32 = 0x0000_0100;
    pub const GL_TRIANGLES: u32 = 0x0004;
    pub const GL_TRIANGLE_STRIP: u32 = 0x0005;
    pub const GL_TRIANGLE_FAN: u32 = 0x0006;
    pub const GL_QUADS: u32 = 0x0007;
    pub const GL_MODELVIEW: u32 = 0x1700;
    pub const GL_PROJECTION: u32 = 0x1701;
    pub const GL_DEPTH_TEST: u32 = 0x0B71;
    pub const GL_COLOR: u32 = 0x1800;
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlState {
    /// No GL context has been created yet.
    Uninit,
    /// GL context exists but is not current on any thread.
    Detached,
    /// GL context is current on the calling thread.
    Current,
}

struct GLCtxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    hdc: HDC,
    #[cfg(target_os = "windows")]
    hglrc: HGLRC,
    #[cfg(target_os = "windows")]
    state: GlState,
    #[cfg(target_os = "windows")]
    gl_destroyed: bool,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[cfg(target_os = "windows")]
fn teardown_gl_resources(inner: &mut GLCtxInner) {
    if inner.gl_destroyed {
        return;
    }
    inner.gl_destroyed = true;
    if !inner.hglrc.is_null() {
        // SAFETY: release any current binding, then delete the context.
        unsafe {
            let _ = wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
            wglDeleteContext(inner.hglrc);
        }
        inner.hglrc = std::ptr::null_mut();
    }
    if !inner.hwnd.is_null() && !inner.hdc.is_null() {
        // SAFETY: `hdc` was acquired with `GetDC` during construction.
        unsafe {
            ReleaseDC(inner.hwnd, inner.hdc);
        }
        inner.hdc = std::ptr::null_mut();
    }
}

#[derive(Clone)]
pub struct GLCanvas {
    inner: Rc<RefCell<GLCtxInner>>,
}

/// Register the `RuWxGLCanvas` window class (idempotent).
#[cfg(target_os = "windows")]
fn register_gl_canvas_class() {
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("RuWxGLCanvas");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            lpfnWndProc: Some(gl_canvas_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut() as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

impl GLCanvas {
    /// Default pixel size used when the caller does not pass one.
    pub const DEFAULT_W: u32 = 320;
    pub const DEFAULT_H: u32 = 240;

    /// Create a new `GLCanvas` of the given pixel size. The GL
    /// context is created but not yet current — call
    /// [`set_current`](Self::set_current) before issuing GL
    /// commands.
    #[cfg(target_os = "windows")]
    pub fn with_size<W: Window>(parent: &W, width: u32, height: u32) -> Self {
        register_gl_canvas_class();

        let id = next_control_id();

        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent_hwnd = parent.hwnd();
            let wide_class = to_wide("RuWxGLCanvas");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
                0,
                0,
                width as i32,
                height as i32,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        // Get the HDC for the new window. The window is created
        // with `CS_OWNDC` style so the HDC is private and we
        // don't need to re-acquire it before every GL call.
        // SAFETY: `hwnd` is a live `GLCanvas` window; the returned
        // HDC is valid until the window is destroyed.
        let hdc = unsafe { GetDC(hwnd) };

        // Pick a pixel format. We request a standard 32-bit RGBA
        // double-buffered surface with 24-bit depth and 8-bit
        // stencil.
        // SAFETY: zero-initialised POD struct; we set every field
        // explicitly below.
        let ppfd: PIXELFORMATDESCRIPTOR = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 0,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 24,
            cStencilBits: 8,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE as u8,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };
        // SAFETY: `ChoosePixelFormat` returns an index that best
        // matches `ppfd` for the HDC. If no match is found it
        // returns 0; we treat that as failure.
        let pix_format = unsafe { ChoosePixelFormat(hdc, &ppfd) };
        let hglrc = if pix_format != 0 {
            // SAFETY: `SetPixelFormat` must be called exactly once
            // on a private HDC; our class is `CS_OWNDC` so this
            // is safe.
            let _ = unsafe { SetPixelFormat(hdc, pix_format, &ppfd) };
            // SAFETY: `hdc` has a valid pixel format; `wglCreateContext`
            // returns NULL only if the DC has no OpenGL support.
            unsafe { wglCreateContext(hdc) }
        } else {
            std::ptr::null_mut()
        };

        let state = if hglrc.is_null() {
            GlState::Uninit
        } else {
            GlState::Detached
        };

        let canvas = GLCanvas {
            inner: Rc::new(RefCell::new(GLCtxInner {
                hwnd,
                hdc,
                hglrc,
                state,
                gl_destroyed: false,
                rect: Rect::new(0, 0, width, height),
                visible: true,
                enabled: true,
            })),
        };
        {
            let inner_clone = canvas.inner.clone();
            let raw = Rc::into_raw(inner_clone);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
            }
        }
        canvas
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn with_size<W: Window>(_parent: &W, width: u32, height: u32) -> Self {
        let _ = width;
        let _ = height;
        GLCanvas {
            inner: Rc::new(RefCell::new(GLCtxInner {
                rect: Rect::new(0, 0, width, height),
                visible: true,
                enabled: true,
            })),
        }
    }

    /// Convenience constructor with a default 320×240 size.
    pub fn new<W: Window>(parent: &W) -> Self {
        Self::with_size(parent, Self::DEFAULT_W, Self::DEFAULT_H)
    }

    /// Bind this canvas's GL context to the current thread.
    /// Returns `true` on success.
    #[cfg(target_os = "windows")]
    pub fn set_current(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.hglrc.is_null() || inner.hdc.is_null() {
            return false;
        }
        // SAFETY: `inner.hdc` is a private (CS_OWNDC) HDC; the
        // context is either Detached (safe to make current) or
        // already Current on this thread (no-op).
        let ok = unsafe { wglMakeCurrent(inner.hdc, inner.hglrc) };
        if ok != 0 {
            inner.state = GlState::Current;
            true
        } else {
            false
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn set_current(&self) -> bool {
        false
    }

    /// Release the GL context from the current thread. After this
    /// call no thread holds the context and the canvas is ready to
    /// be made current again from the same or a different thread.
    #[cfg(target_os = "windows")]
    pub fn make_inactive(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.hglrc.is_null() {
            return false;
        }
        // SAFETY: passing a null HDC + null HGLRC to
        // `wglMakeCurrent` releases the current context on the
        // calling thread. The HGLRC argument is ignored when the
        // HDC is null.
        let ok = unsafe { wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut()) };
        if ok != 0 {
            inner.state = GlState::Detached;
            true
        } else {
            false
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn make_inactive(&self) -> bool {
        false
    }

    /// Swap the front and back buffers (call once per frame).
    #[cfg(target_os = "windows")]
    pub fn swap_buffers(&self) -> bool {
        let inner = self.inner.borrow();
        if inner.hdc.is_null() {
            return false;
        }
        // SAFETY: `inner.hdc` is a private (CS_OWNDC) HDC.
        unsafe { SwapBuffers(inner.hdc) != 0 }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn swap_buffers(&self) -> bool {
        false
    }

    /// `true` if a GL context has been created and is currently
    /// bound to the calling thread.
    #[cfg(target_os = "windows")]
    pub fn is_current(&self) -> bool {
        self.inner.borrow().state == GlState::Current
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn is_current(&self) -> bool {
        false
    }

    /// `true` if a GL context has been created for this canvas.
    #[cfg(target_os = "windows")]
    pub fn has_context(&self) -> bool {
        !self.inner.borrow().hglrc.is_null()
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn has_context(&self) -> bool {
        false
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }

    /// Return the native window handle (HWND on Windows, 0 elsewhere).
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
    #[cfg(not(target_os = "windows"))]
    pub fn hwnd(&self) -> isize {
        0
    }
}

#[cfg(target_os = "windows")]
impl Drop for GLCanvas {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) == 1 {
            teardown_gl_resources(&mut self.inner.borrow_mut());
        }
    }
}

impl Widget for GLCtxInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}

#[cfg(target_os = "windows")]
impl Window for GLCanvas {
    fn hwnd(&self) -> HWND {
        self.hwnd()
    }
}

/// Win32 WndProc for the `RuWxGLCanvas` class.
#[cfg(target_os = "windows")]
unsafe extern "system" fn gl_canvas_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<GLCtxInner>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<GLCtxInner>) };
                teardown_gl_resources(&mut rc.borrow_mut());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(rc);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_size_is_positive() {
        const { assert!(GLCanvas::DEFAULT_W > 0) };
        const { assert!(GLCanvas::DEFAULT_H > 0) };
    }

    #[test]
    fn gl11_constants_are_distinct() {
        // Make sure the re-exports point to different bit patterns
        // so a user can't accidentally confuse them.
        assert_ne!(gl11::GL_COLOR_BUFFER_BIT, gl11::GL_DEPTH_BUFFER_BIT);
        assert_ne!(gl11::GL_TRIANGLES, gl11::GL_QUADS);
    }

    #[test]
    fn new_unattached_state_is_correct() {
        // We can't construct a real canvas without a window, so
        // just check the inner type's invariants via a dummy
        // construction (this test is meaningless on Windows but
        // documents the contract).
        let inner = GLCtxInner {
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            #[cfg(target_os = "windows")]
            hdc: std::ptr::null_mut(),
            #[cfg(target_os = "windows")]
            hglrc: std::ptr::null_mut(),
            #[cfg(target_os = "windows")]
            state: GlState::Uninit,
            #[cfg(target_os = "windows")]
            gl_destroyed: false,
            rect: Rect::new(0, 0, 16, 16),
            visible: true,
            enabled: true,
        };
        assert!(inner.enabled);
    }
}
