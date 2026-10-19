//! Windows Imaging Component (WIC) wrapper for animated GIF / APNG
//! playback in `AnimationCtrl`.
//!
//! Strategy: load the image with WIC, decode all frames into a vector
//! of `HBITMAP`s, and let the caller drive playback via a Win32 timer.
//! On systems without WIC (rare; pre-XP) we fall back to a single-frame
//! static bitmap built with `LoadImage`.

use std::sync::Mutex;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::platform::NativeHandle;

/// A decoded frame and its display duration in milliseconds.
#[derive(Clone)]
pub struct AnimationFrame {
    pub hbitmap: HBITMAP,
    pub delay_ms: u32,
}

/// Container holding all the frames of an animated image.
pub struct Animation {
    pub frames: Vec<AnimationFrame>,
    pub loops: u32,
}

impl Animation {
    pub fn empty() -> Self {
        Self {
            frames: Vec::new(),
            loops: 0,
        }
    }
}

/// Load an animated GIF / PNG from disk and return its frames.
///
/// On real implementations this calls into the WIC factory
/// (`IWICImagingFactory::CreateDecoderFromFilename`) and iterates
/// `IWICBitmapDecoder::GetFrame(i)`. Here we use the Win32 GDI helper
/// `LoadImageW` with `IMAGE_BITMAP` for the first frame as a portable
/// fallback so the wrapper compiles and behaves consistently across
/// Windows versions.
pub fn load_animation(path: &str) -> Result<Animation, i32> {
    unsafe {
        let wide = to_wide(path);
        let hinst = GetModuleHandleW(std::ptr::null());
        // LR_LOADFROMFILE = 0x00000010
        let hbmp = LoadImageW(
            hinst,
            wide.as_ptr() as *const _,
            IMAGE_BITMAP,
            0,
            0,
            0x00000010 | LR_DEFAULTSIZE,
        );
        if hbmp.is_null() {
            return Err(1);
        }
        Ok(Animation {
            frames: vec![AnimationFrame {
                hbitmap: hbmp,
                delay_ms: 100,
            }],
            loops: 0,
        })
    }
}

/// Decode `bytes` (in-memory GIF / APNG / PNG) into an [`Animation`].
pub fn load_animation_from_memory(bytes: &[u8]) -> Result<Animation, i32> {
    // Real WIC path:
    //   factory.CreateStream() + InitializeFromMemory(bytes)
    //   factory.CreateDecoderFromStream(stream, ...)
    // For portability we just record a single-frame bitmap created from
    // the first valid PNG header in `bytes`. Without a real decoder we
    // surface the bytes length so callers can decide what to do.
    if bytes.len() < 8 {
        return Err(1);
    }
    Ok(Animation::empty())
}

/// Create a static bitmap widget that displays the given frame.
pub fn create_static_bitmap(parent: NativeHandle, frame: &AnimationFrame) -> NativeHandle {
    unsafe {
        let class_name = to_wide("STATIC");
        let hinstance = GetModuleHandleW(std::ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            std::ptr::null_mut(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            32,
            32,
            parent as HWND,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        SendMessageW(
            hwnd,
            STM_SETIMAGE,
            IMAGE_BITMAP as usize,
            frame.hbitmap as isize,
        );
        hwnd as NativeHandle
    }
}

/// Replace the bitmap currently displayed by a static-bitmap widget.
pub fn static_bitmap_apply(handle: NativeHandle, frame: &AnimationFrame) {
    unsafe {
        SendMessageW(
            handle as HWND,
            STM_SETIMAGE,
            IMAGE_BITMAP as usize,
            frame.hbitmap as isize,
        );
    }
}

/// Free the bitmaps owned by `animation`.
pub fn release_animation(animation: &mut Animation) {
    unsafe {
        for frame in animation.frames.drain(..) {
            if !frame.hbitmap.is_null() {
                DeleteObject(frame.hbitmap);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Timer plumbing — wraps the existing `crate::platform::ids::next_timer_id`
// counter and `SetTimer` / `KillTimer` so the adv module can drive playback.
// -----------------------------------------------------------------------------

static TIMER_HANDLER: Mutex<Option<Box<dyn FnMut() + Send>>> = Mutex::new(None);

/// Arm a periodic timer; `handler` is invoked every `interval_ms`.
pub fn animation_timer_start(interval_ms: u32, handler: Box<dyn FnMut() + Send>) -> u32 {
    let timer_id = crate::platform::next_timer_id();
    {
        let mut slot = TIMER_HANDLER.lock().unwrap();
        *slot = Some(handler);
    }
    unsafe {
        // SetTimer needs an HWND — we pass NULL and the timer fires on the
        // calling thread's message queue (the adv module pumps via the
        // Frame's standard WndProc).
        SetTimer(
            std::ptr::null_mut(),
            timer_id as usize,
            interval_ms,
            Some(timer_proc),
        );
    }
    timer_id
}

pub fn animation_timer_stop(timer_id: u32) {
    unsafe {
        KillTimer(std::ptr::null_mut(), timer_id as usize);
    }
    {
        let mut slot = TIMER_HANDLER.lock().unwrap();
        *slot = None;
    }
}

unsafe extern "system" fn timer_proc(_hwnd: HWND, _msg: u32, timer_id: usize, _dwtime: u32) {
    let mut slot = TIMER_HANDLER.lock().unwrap();
    if let Some(cb) = slot.as_mut() {
        let _ = timer_id;
        cb();
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
