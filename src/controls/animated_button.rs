//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Animated bitmap button — cycles through a sequence of bitmap
//! frames on a [`Timer`], mirroring a `wxButton` with an embedded
//! animation (or a toolbar glyph that pulses while idle).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::adv::animation::Animation;
use crate::controls::bitmap_button::BitmapButton;
use crate::dc::bitmap::Bitmap;
use crate::dc::image::Image;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;
use crate::Timer;

const TICK_MS: u64 = 120;

/// Four built-in SVG frames (different fill colours) used by
/// [`AnimatedButton::demo`].
const DEMO_FRAME_0: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="4" fill="#4F46E5"/><circle cx="12" cy="12" r="5" fill="white" opacity="0.9"/></svg>"##;
const DEMO_FRAME_1: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="4" fill="#10B981"/><circle cx="12" cy="12" r="5" fill="white" opacity="0.9"/></svg>"##;
const DEMO_FRAME_2: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="4" fill="#F59E0B"/><circle cx="12" cy="12" r="5" fill="white" opacity="0.9"/></svg>"##;
const DEMO_FRAME_3: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="4" fill="#EF4444"/><circle cx="12" cy="12" r="5" fill="white" opacity="0.9"/></svg>"##;

struct AnimatedButtonInner {
    button: BitmapButton,
    frames: Vec<Bitmap>,
    frame_index: usize,
    timer: Option<Timer>,
    playing: bool,
}

/// Bitmap button that cycles through animation frames automatically.
#[derive(Clone)]
pub struct AnimatedButton {
    inner: Rc<RefCell<AnimatedButtonInner>>,
}

impl AnimatedButton {
    /// Build an animated button from an [`Animation`] and start playback.
    pub fn from_animation<W: Window>(
        parent: &W,
        frame: &Frame,
        animation: Animation,
        width: i32,
        height: i32,
    ) -> Self {
        let frames = animation
            .frames()
            .iter()
            .map(|f| f.image.to_bitmap())
            .collect::<Vec<_>>();
        let btn = Self::from_bitmaps(parent, frames, width, height);
        btn.start(frame);
        btn
    }

    /// Build from a sequence of embedded SVG byte strings (each rasterised
    /// to `icon_size × icon_size`) and start playback.
    pub fn from_svg_cycle<W: Window>(
        parent: &W,
        frame: &Frame,
        svgs: &[&[u8]],
        icon_size: u32,
        width: i32,
        height: i32,
    ) -> Self {
        let frames = svgs
            .iter()
            .filter_map(|svg| svg_bytes_to_bitmap(svg, icon_size))
            .collect();
        let btn = Self::from_bitmaps(parent, frames, width, height);
        btn.start(frame);
        btn
    }

    /// Ready-made pulsing demo button (four coloured frames).
    pub fn demo<W: Window>(parent: &W, frame: &Frame) -> Self {
        Self::from_svg_cycle(
            parent,
            frame,
            &[DEMO_FRAME_0, DEMO_FRAME_1, DEMO_FRAME_2, DEMO_FRAME_3],
            32,
            40,
            40,
        )
    }

    fn from_bitmaps<W: Window>(parent: &W, frames: Vec<Bitmap>, width: i32, height: i32) -> Self {
        let first = frames
            .first()
            .cloned()
            .unwrap_or_else(|| Bitmap::new(width as u32, height as u32));
        let button = BitmapButton::new(parent, &first, width, height);
        AnimatedButton {
            inner: Rc::new(RefCell::new(AnimatedButtonInner {
                button,
                frames,
                frame_index: 0,
                timer: None,
                playing: false,
            })),
        }
    }

    /// Start (or resume) frame cycling.
    pub fn start(&self, frame: &Frame) {
        let mut inner = self.inner.borrow_mut();
        if inner.frames.len() < 2 {
            return;
        }
        if inner.playing {
            return;
        }
        inner.playing = true;
        let me = self.clone();
        let timer = Timer::new(frame);
        timer.on_tick(move || me.advance_frame());
        timer.start(Duration::from_millis(TICK_MS));
        inner.timer = Some(timer);
    }

    /// Stop frame cycling (the current frame stays visible).
    pub fn stop(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.playing = false;
        inner.timer = None;
    }

    pub fn is_playing(&self) -> bool {
        self.inner.borrow().playing
    }

    pub fn id(&self) -> u16 {
        self.inner.borrow().button.id()
    }

    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        self.inner.borrow().button.on_click(frame, callback);
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.borrow().button.as_widget_ref()
    }

    fn advance_frame(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.frames.is_empty() {
            return;
        }
        inner.frame_index = (inner.frame_index + 1) % inner.frames.len();
        if let Some(bmp) = inner.frames.get(inner.frame_index) {
            inner.button.set_bitmap_label(bmp);
        }
    }
}

#[cfg(target_os = "windows")]
fn svg_bytes_to_bitmap(svg: &[u8], size: u32) -> Option<Bitmap> {
    let hb = crate::dc::icon::svg_bytes_to_hbitmap(svg, size, size)?;
    let img = hbitmap_to_image(hb, size, size)?;
    // SAFETY: we took ownership of the temporary HBITMAP pixels.
    unsafe {
        windows_sys::Win32::Graphics::Gdi::DeleteObject(hb);
    }
    Some(img.to_bitmap())
}

#[cfg(not(target_os = "windows"))]
fn svg_bytes_to_bitmap(_svg: &[u8], size: u32) -> Option<Bitmap> {
    Some(Bitmap::new(size, size))
}

#[cfg(target_os = "windows")]
fn hbitmap_to_image(hbmp: windows_sys::Win32::Graphics::Gdi::HBITMAP, w: u32, h: u32) -> Option<Image> {
    use windows_sys::Win32::Graphics::Gdi::{
        GetDC, GetDIBits, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };

    // SAFETY: Win32 GDI calls with valid HBITMAP and buffers.
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        if hdc.is_null() {
            return None;
        }
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [std::mem::zeroed(); 1],
        };
        let byte_len = (w as usize) * (h as usize) * 4;
        let mut bgra = vec![0u8; byte_len];
        let lines = GetDIBits(
            hdc,
            hbmp,
            0,
            h,
            bgra.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(std::ptr::null_mut(), hdc);
        if lines == 0 {
            return None;
        }
        let mut rgba = Vec::with_capacity(byte_len);
        for chunk in bgra.chunks_exact(4) {
            rgba.push(chunk[2]);
            rgba.push(chunk[1]);
            rgba.push(chunk[0]);
            rgba.push(chunk[3]);
        }
        Some(Image::from_rgba8(w, h, rgba))
    }
}
