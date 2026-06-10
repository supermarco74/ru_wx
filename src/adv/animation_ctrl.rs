//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Animation control (`wxAnimationCtrl`).
//!
//! A custom WndProc-based widget that displays a
//! [`crate::adv::animation::Animation`] and advances its frames at the
//! per-frame rate declared by the GIF (or, for static images, at
//! 100 ms per call to `play`).
//!
//! # Win32 implementation
//!
//! The control is a child window of class `RuWxAnimationCtrl`
//! (registered at first construction). On each `WM_TIMER` tick the
//! frame index is advanced; on each `WM_PAINT` the current frame is
//! converted to a `Bitmap` and blitted to the client area. The
//! timer is re-armed with the new frame's delay (clamped to a
//! 10 ms minimum).
//!
//! # Methods
//!
//! * [`AnimationCtrl::set_animation`] / [`AnimationCtrl::animation`]
//!   — the data source.
//! * [`AnimationCtrl::play`] / [`AnimationCtrl::stop`] /
//!   [`AnimationCtrl::is_playing`] — playback control.
//! * [`AnimationCtrl::current_frame`] — index of the frame
//!   currently drawn (or `0` when stopped).
//!
//! # Cross-platform stub
//!
//! On non-Windows targets the type is still constructible (as a
//! width/height record with no real window) and the getters /
//! setters are no-ops, so code can compile cross-platform.

use std::cell::RefCell;
use std::rc::Rc;

use crate::adv::animation::{Animation, AnimationFrame};
use crate::dc::bitmap::Bitmap;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// `wParam` value for our `SetTimer`. Picked at random; we just need
// a value that doesn't collide with the other timers the user may
// have installed.
#[cfg(target_os = "windows")]
const ANIMATION_TIMER_ID: usize = 0xC0_1D;

/// Default frame delay (in milliseconds) used when the loaded
/// animation has no per-frame delay declared (e.g. a static
/// image). 100 ms = 10 fps, which is enough to look animated while
/// keeping CPU low.
const DEFAULT_FRAME_DELAY_MS: u32 = 100;

/// Minimum allowed per-frame delay. 10 ms is a safe floor: any
/// tighter interval would saturate the WM_TIMER queue on most
/// machines without giving the user any visible difference.
const MIN_FRAME_DELAY_MS: u32 = 10;

struct AnimationCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    animation: Option<Animation>,
    current_frame: usize,
    playing: bool,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct AnimationCtrl {
    inner: Rc<RefCell<AnimationCtrlInner>>,
}

/// Register the `RuWxAnimationCtrl` window class (idempotent).
#[cfg(target_os = "windows")]
fn register_animation_ctrl_class() {
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("RuWxAnimationCtrl");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(animation_ctrl_wnd_proc),
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

impl AnimationCtrl {
    /// Default pixel size used when the caller does not pass one.
    pub const DEFAULT_W: u32 = 32;
    pub const DEFAULT_H: u32 = 32;

    /// Create a new `AnimationCtrl` with a sensible default size.
    pub fn new<W: Window>(parent: &W) -> Self {
        Self::with_size(parent, Self::DEFAULT_W, Self::DEFAULT_H)
    }

    /// Create a new `AnimationCtrl` of the given pixel size.
    pub fn with_size<W: Window>(parent: &W, width: u32, height: u32) -> Self {
        #[cfg(target_os = "windows")]
        register_animation_ctrl_class();

        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent_hwnd = parent.hwnd();
            let wide_class = to_wide("RuWxAnimationCtrl");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
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

        let ctrl = AnimationCtrl {
            inner: Rc::new(RefCell::new(AnimationCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                animation: None,
                current_frame: 0,
                playing: false,
                rect: Rect::new(0, 0, width, height),
                visible: true,
                enabled: true,
            })),
        };

        #[cfg(target_os = "windows")]
        {
            let inner_clone = ctrl.inner.clone();
            let raw = Rc::into_raw(inner_clone);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
            }
        }

        ctrl
    }

    /// Install a new [`Animation`] to display. Resets the
    /// currently-shown frame to `0`. If a previous animation was
    /// playing it is stopped.
    pub fn set_animation(&self, anim: Animation) {
        let mut inner = self.inner.borrow_mut();
        inner.animation = Some(anim);
        inner.current_frame = 0;
        // The new animation may have a different natural size, so
        // we trigger a full repaint to lay it out.
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            InvalidateRect(inner.hwnd, std::ptr::null(), 1);
        }
    }

    /// Drop the current animation. The control paints an empty
    /// (background-coloured) client area until a new animation is
    /// installed.
    pub fn clear_animation(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.animation = None;
        inner.current_frame = 0;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            InvalidateRect(inner.hwnd, std::ptr::null(), 1);
        }
    }

    /// Start playing the loaded animation. No-op if the animation
    /// is empty or already playing.
    pub fn play(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd;
            let initial_delay;
            {
                let mut inner = self.inner.borrow_mut();
                // Compute the initial delay *before* any mutable
                // borrow of `inner` is in scope. The previous
                // implementation used the equivalent of
                // `inner.animation.as_ref().unwrap().frame(0)`,
                // which the compiler accepted only because the
                // temporary `&inner.animation` died at the end
                // of the statement. The pre-v0.5.8 code is
                // correct but the `unwrap()` reads as if it
                // could panic; we replace it with a `match` so
                // the no-animation branch is explicit at the
                // source level.
                initial_delay = match inner.animation.as_ref() {
                    Some(anim) if anim.is_loaded() => anim
                        .frame(0)
                        .map(|f| f.delay_ms)
                        .unwrap_or(DEFAULT_FRAME_DELAY_MS),
                    _ => return,
                };
                if inner.playing {
                    return;
                }
                inner.playing = true;
                inner.current_frame = 0;
                hwnd = inner.hwnd;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetTimer(hwnd, ANIMATION_TIMER_ID, initial_delay.max(MIN_FRAME_DELAY_MS), None);
                InvalidateRect(hwnd, std::ptr::null(), 1);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self.inner.borrow_mut().playing = true;
        }
    }

    /// Stop playing. The control keeps showing the first frame of
    /// the animation.
    pub fn stop(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd;
            {
                let mut inner = self.inner.borrow_mut();
                if !inner.playing {
                    return;
                }
                inner.playing = false;
                inner.current_frame = 0;
                hwnd = inner.hwnd;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                KillTimer(hwnd, ANIMATION_TIMER_ID);
                InvalidateRect(hwnd, std::ptr::null(), 1);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self.inner.borrow_mut().playing = false;
        }
    }

    /// `true` while the control is ticking frames.
    pub fn is_playing(&self) -> bool {
        self.inner.borrow().playing
    }

    /// Index of the frame currently being drawn. Returns `0` when
    /// the control is stopped or has no animation.
    pub fn current_frame(&self) -> usize {
        self.inner.borrow().current_frame
    }

    /// A clone of the currently-installed animation, or `None` if
    /// the control is empty.
    pub fn animation(&self) -> Option<Animation> {
        self.inner.borrow().animation.clone()
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

impl Drop for AnimationCtrlInner {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            if self.playing && !self.hwnd.is_null() {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    KillTimer(self.hwnd, ANIMATION_TIMER_ID);
                }
            }
        }
    }
}

impl Widget for AnimationCtrlInner {
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
impl Window for AnimationCtrl {
    fn hwnd(&self) -> HWND {
        self.hwnd()
    }
}

/// Draw the current frame into the supplied `HDC`. Used both by
/// `WM_PAINT` and by the parent (e.g. for printing / screenshots).
#[cfg(target_os = "windows")]
fn draw_current_frame(hwnd: HWND, hdc: HDC) {
    // SAFETY: All GDI calls below use handles that are valid for
    // the lifetime of this function. `hwnd` is a live AnimationCtrl
    // window.
    unsafe {
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);

        // Fill background with the system window colour so
        // transparent areas of the animation don't show through.
        let bg = (COLOR_WINDOW + 1) as HBRUSH;
        FillRect(hdc, &client, bg);

        // Fetch the current frame without holding the RefCell
        // borrow while doing GDI work.
        let frame_opt: Option<AnimationFrame> = {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr == 0 {
                return;
            }
            // Bump the strong count before `Rc::from_raw`;
            // see `frame.rs` WM_NOTIFY for the full rationale
            // (the build's `clone + into_raw` leaves the count
            // at 2, so without this bump the second dispatch
            // would drop the count to 0 and free the backing
            // storage).
            // SAFETY: `ptr` was stored via `Rc::into_raw` and the
            // outer dispatch path bumped the refcount by one before
            // this drop (see the long comment in `frame.rs`
            // WM_NOTIFY for the full rationale). The outer `unsafe`
            // block above covers this call.
            Rc::increment_strong_count(ptr as *const RefCell<AnimationCtrlInner>);
            // SAFETY: same as above; the outer `unsafe` block covers
            // this call.
            let rc = Rc::from_raw(ptr as *const RefCell<AnimationCtrlInner>);
            let out = {
                let inner = rc.borrow();
                if let Some(anim) = &inner.animation {
                    anim.frame(inner.current_frame).cloned()
                } else {
                    None
                }
            };
            drop(rc);
            out
        };

        let frame = match frame_opt {
            Some(f) => f,
            None => return,
        };
        if frame.image.is_null() {
            return;
        }

        // Convert the current frame's RGBA8 buffer into a 32-bit
        // DIB section. The conversion (RGBA → BGRA) happens inside
        // `Image::to_bitmap`, so we just blit.
        let bmp: Bitmap = frame.image.to_bitmap();
        if bmp.is_null() {
            return;
        }
        let hbmp = bmp.handle();
        if hbmp.is_null() {
            return;
        }

        let mem_dc = CreateCompatibleDC(hdc);
        if mem_dc.is_null() {
            return;
        }
        let old = SelectObject(mem_dc, hbmp as _);

        // We blit at the natural frame size, top-left aligned. If
        // the control is bigger than the frame the rest of the
        // client area is left as the background colour.
        let frame_w = frame.image.width as i32;
        let frame_h = frame.image.height as i32;
        let dest_w = client.right - client.left;
        let dest_h = client.bottom - client.top;
        if frame_w == dest_w && frame_h == dest_h {
            BitBlt(hdc, 0, 0, frame_w, frame_h, mem_dc, 0, 0, SRCCOPY);
        } else {
            // Aspect-preserving stretch.
            StretchBlt(
                hdc, 0, 0, dest_w, dest_h, mem_dc, 0, 0, frame_w, frame_h, SRCCOPY,
            );
        }

        SelectObject(mem_dc, old);
        DeleteDC(mem_dc);
        // `bmp` is dropped at the end of this scope, releasing
        // the HBITMAP we just blitted. That is intentional: each
        // repaint produces a fresh DIB from the current frame's
        // pixels, and the previous one is no longer needed.
    }
}

/// Win32 WndProc for the `RuWxAnimationCtrl` class.
#[cfg(target_os = "windows")]
unsafe extern "system" fn animation_ctrl_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hdc = BeginPaint(hwnd, &mut ps);
            draw_current_frame(hwnd, hdc);
            EndPaint(hwnd, &ps);
            0
        }
        WM_TIMER => {
            if wparam != ANIMATION_TIMER_ID {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            // Pull what we need out of state, then drop the borrow
            // before re-arming the timer / repainting.
            let (next_delay, has_next) = {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr == 0 {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                // Bump the strong count before `Rc::from_raw`;
                // see `frame.rs` WM_NOTIFY for the full rationale.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<AnimationCtrlInner>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<AnimationCtrlInner>) };
                let out = {
                    let mut inner = rc.borrow_mut();
                    if !inner.playing {
                        (0, false)
                    } else if let Some(anim) = &inner.animation {
                        if !anim.is_loaded() {
                            inner.playing = false;
                            (0, false)
                        } else {
                            let frame_count = anim.frame_count();
                            if frame_count == 0 {
                                inner.playing = false;
                                (0, false)
                            } else {
                                let next = (inner.current_frame + 1) % frame_count;
                                // Compute the next delay BEFORE the
                                // mutable borrow assignment, so we
                                // don't hold an immutable reference
                                // into `anim` while still mutating
                                // `inner`.
                                let delay = anim
                                    .frame(next)
                                    .map(|f| f.delay_ms)
                                    .filter(|d| *d > 0)
                                    .unwrap_or(DEFAULT_FRAME_DELAY_MS);
                                inner.current_frame = next;
                                (delay, true)
                            }
                        }
                    } else {
                        (0, false)
                    }
                };
                drop(rc);
                out
            };
            if has_next {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    KillTimer(hwnd, ANIMATION_TIMER_ID);
                    SetTimer(
                        hwnd,
                        ANIMATION_TIMER_ID,
                        next_delay.max(MIN_FRAME_DELAY_MS),
                        None,
                    );
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                }
            } else {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    KillTimer(hwnd, ANIMATION_TIMER_ID);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                }
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<AnimationCtrlInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_size_is_positive() {
        const { assert!(AnimationCtrl::DEFAULT_W > 0) };
        const { assert!(AnimationCtrl::DEFAULT_H > 0) };
    }

    #[test]
    fn new_control_is_not_playing() {
        let a = AnimationCtrlInner {
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            animation: None,
            current_frame: 0,
            playing: false,
            rect: Rect::new(0, 0, 16, 16),
            visible: true,
            enabled: true,
        };
        assert!(!a.playing);
        assert_eq!(a.current_frame, 0);
    }
}
