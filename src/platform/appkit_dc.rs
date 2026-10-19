//! macOS Core Graphics device context — BGRA bitmap backing for [`MemoryDC`] / [`PaintDC`].

use std::cell::RefCell;
use std::rc::Rc;

use objc2_core_foundation::{CGFloat, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpaceCreateDeviceRGB, CGContext, CGContextAddLineToPoint,
    CGContextDrawImage, CGContextFillEllipseInRect, CGContextFillRect, CGContextMoveToPoint,
    CGContextScaleCTM, CGContextSetLineWidth, CGContextSetRGBFillColor, CGContextSetRGBStrokeColor,
    CGContextSetShouldAntialias, CGContextStrokeEllipseInRect, CGContextStrokePath,
    CGContextStrokeRect, CGContextTranslateCTM, CGImage,
};
use objc2_foundation::NSString;

use crate::core::geometry::{Colour, Rect};
use crate::dc::bitmap::Bitmap;
use crate::dc::brush::{Brush, BrushStyle};
use crate::dc::pen::{Pen, PenStyle};

use super::appkit_bitmap::{mac_bitmap_attach_bgra, mac_bitmap_store_bgra, mac_bitmap_sync_image};
use super::NativeHandle;

const BGRA_BITMAP_INFO: u32 = 1 | (1 << 12); // kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little

/// Core Graphics drawing context backed by a premultiplied BGRA pixel buffer.
pub struct MacCgDc {
    ctx: objc2_core_foundation::CFRetained<CGContext>,
    bgra: Rc<RefCell<Vec<u8>>>,
    width: u32,
    height: u32,
    bitmap_handle: NativeHandle,
    stroke_colour: Colour,
    fill_colour: Colour,
    text_colour: Colour,
    stroke_width: f64,
    stroke_enabled: bool,
    fill_enabled: bool,
    flush_view: Option<NativeHandle>,
}

fn colour_components(c: Colour) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    (
        c.r as CGFloat / 255.0,
        c.g as CGFloat / 255.0,
        c.b as CGFloat / 255.0,
        c.a as CGFloat / 255.0,
    )
}

fn create_context(
    bgra: Rc<RefCell<Vec<u8>>>,
    width: u32,
    height: u32,
) -> Option<objc2_core_foundation::CFRetained<CGContext>> {
    if width == 0 || height == 0 {
        return None;
    }
    let bytes_per_row = width as usize * 4;
    let needed = bytes_per_row * height as usize;
    {
        let mut buf = bgra.borrow_mut();
        if buf.len() < needed {
            buf.resize(needed, 0);
        }
    }
    let space = CGColorSpaceCreateDeviceRGB()?;
    let ptr = bgra.borrow_mut().as_mut_ptr() as *mut core::ffi::c_void;
    // SAFETY: `bgra` outlives the context; buffer is 32-bpp premultiplied BGRA.
    let ctx = unsafe {
        CGBitmapContextCreate(
            ptr,
            width as usize,
            height as usize,
            8,
            bytes_per_row,
            Some(&space),
            BGRA_BITMAP_INFO,
        )
    }?;
    // Match Win32 top-left origin.
    unsafe {
        CGContextTranslateCTM(Some(&ctx), 0.0, height as CGFloat);
        CGContextScaleCTM(Some(&ctx), 1.0, -1.0);
        CGContextSetShouldAntialias(Some(&ctx), true);
    }
    Some(ctx)
}

pub fn mac_cg_dc_new(width: u32, height: u32) -> MacCgDc {
    let w = width.max(1);
    let h = height.max(1);
    let bgra = Rc::new(RefCell::new(vec![0u8; (w * h * 4) as usize]));
    let ctx = create_context(bgra.clone(), w, h).expect("CGBitmapContextCreate");
    MacCgDc {
        ctx,
        bgra,
        width: w,
        height: h,
        bitmap_handle: 0,
        stroke_colour: Colour::new(0, 0, 0, 255),
        fill_colour: Colour::new(255, 255, 255, 255),
        text_colour: Colour::new(0, 0, 0, 255),
        stroke_width: 1.0,
        stroke_enabled: true,
        fill_enabled: true,
        flush_view: None,
    }
}

pub fn mac_cg_dc_for_bitmap(handle: NativeHandle) -> Option<MacCgDc> {
    let (bgra, w, h) = mac_bitmap_attach_bgra(handle)?;
    let ctx = create_context(bgra.clone(), w, h)?;
    Some(MacCgDc {
        ctx,
        bgra,
        width: w,
        height: h,
        bitmap_handle: handle,
        stroke_colour: Colour::new(0, 0, 0, 255),
        fill_colour: Colour::new(255, 255, 255, 255),
        text_colour: Colour::new(0, 0, 0, 255),
        stroke_width: 1.0,
        stroke_enabled: true,
        fill_enabled: true,
        flush_view: None,
    })
}

impl MacCgDc {
    pub fn attach_flush_view(&mut self, view: NativeHandle) {
        self.flush_view = Some(view);
    }

    pub fn ctx_ptr(&self) -> isize {
        (&*self.ctx as *const CGContext) as isize
    }

    pub fn set_pen(&mut self, pen: Option<&Pen>) {
        match pen {
            Some(p) => {
                self.stroke_colour = p.colour;
                self.stroke_width = p.width.max(1) as f64;
                self.stroke_enabled = p.style != PenStyle::Transparent;
            }
            None => self.stroke_enabled = false,
        }
    }

    pub fn set_brush(&mut self, brush: Option<&Brush>) {
        match brush {
            Some(b) => {
                self.fill_colour = b.colour;
                self.fill_enabled = b.style != BrushStyle::Transparent;
            }
            None => self.fill_enabled = false,
        }
    }

    pub fn set_text_color(&mut self, colour: Colour) {
        self.text_colour = colour;
    }

    pub fn set_bk_color(&mut self, colour: Colour) {
        self.fill_colour = colour;
    }

    pub fn set_bk_mode(&mut self, mode: crate::dc::dc::BackgroundMode) {
        let _ = mode;
    }

    fn apply_stroke_colour(&self) {
        let (r, g, b, a) = colour_components(self.stroke_colour);
        unsafe {
            CGContextSetRGBStrokeColor(Some(&self.ctx), r, g, b, a);
            CGContextSetLineWidth(Some(&self.ctx), self.stroke_width as CGFloat);
        }
    }

    fn apply_fill_colour(&self) {
        let (r, g, b, a) = colour_components(self.fill_colour);
        unsafe {
            CGContextSetRGBFillColor(Some(&self.ctx), r, g, b, a);
        }
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        if !self.stroke_enabled {
            return;
        }
        self.apply_stroke_colour();
        unsafe {
            CGContextMoveToPoint(Some(&self.ctx), x1 as CGFloat, y1 as CGFloat);
            CGContextAddLineToPoint(Some(&self.ctx), x2 as CGFloat, y2 as CGFloat);
            CGContextStrokePath(Some(&self.ctx));
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let rect = CGRect::new(
            CGPoint::new(x as CGFloat, y as CGFloat),
            CGSize::new(w as CGFloat, h as CGFloat),
        );
        if self.fill_enabled {
            self.apply_fill_colour();
            unsafe {
                CGContextFillRect(Some(&self.ctx), rect);
            }
        }
        if self.stroke_enabled {
            self.apply_stroke_colour();
            unsafe {
                CGContextStrokeRect(Some(&self.ctx), rect);
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour) {
        let (r, g, b, a) = colour_components(colour);
        let rect = CGRect::new(
            CGPoint::new(x as CGFloat, y as CGFloat),
            CGSize::new(w as CGFloat, h as CGFloat),
        );
        unsafe {
            CGContextSetRGBFillColor(Some(&self.ctx), r, g, b, a);
            CGContextFillRect(Some(&self.ctx), rect);
        }
    }

    pub fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let rect = CGRect::new(
            CGPoint::new(x as CGFloat, y as CGFloat),
            CGSize::new(w as CGFloat, h as CGFloat),
        );
        if self.fill_enabled {
            self.apply_fill_colour();
            unsafe {
                CGContextFillEllipseInRect(Some(&self.ctx), rect);
            }
        }
        if self.stroke_enabled {
            self.apply_stroke_colour();
            unsafe {
                CGContextStrokeEllipseInRect(Some(&self.ctx), rect);
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        use objc2_app_kit::NSStringDrawing;
        let ns = NSString::from_str(text);
        let size = unsafe { ns.sizeWithAttributes(None) };
        // CGContext has Y-flip: (0,0) is top-left, so draw at (x, y) in flipped coords.
        // NSString.drawAtPoint uses bottom-left, so we flip Y.
        let flipped_y = self.height as CGFloat - y as CGFloat - size.height;
        unsafe {
            let _: () = objc2::msg_send![&ns, drawAtPoint: CGPoint::new(x as CGFloat, flipped_y), withAttributes: std::ptr::null::<objc2::runtime::AnyObject>()];
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool) {
        let (tw, th) = self.text_extent(text);
        let mut x = rect.x;
        let mut y = rect.y;
        if center {
            x += ((rect.width as i32 - tw) / 2).max(0);
            y += ((rect.height as i32 - th) / 2).max(0);
        }
        self.draw_text(text, x, y);
    }

    pub fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32) {
        if bmp.is_null() {
            return;
        }
        let Some(image) = super::mac_bitmap_image_at(bmp.handle()) else {
            return;
        };
        let cg = match unsafe {
            image.CGImageForProposedRect_context_hints(std::ptr::null_mut(), None, None)
        } {
            Some(cg) => cg,
            None => return,
        };
        let rect = CGRect::new(
            CGPoint::new(x as CGFloat, y as CGFloat),
            CGSize::new(bmp.width as CGFloat, bmp.height as CGFloat),
        );
        unsafe {
            CGContextDrawImage(Some(&self.ctx), rect, Some(&cg));
        }
    }

    pub fn text_extent(&self, text: &str) -> (i32, i32) {
        use objc2_app_kit::NSStringDrawing;
        let ns = NSString::from_str(text);
        // SAFETY: `sizeWithAttributes` with nil attributes is valid.
        let size = unsafe { ns.sizeWithAttributes(None) };
        (size.width.ceil() as i32, size.height.ceil() as i32)
    }

    pub fn flush(&mut self) {
        if self.bitmap_handle != 0 {
            mac_bitmap_sync_image(self.bitmap_handle, &self.bgra, self.width, self.height);
        }
        if let Some(view) = self.flush_view.take() {
            super::mac_view_redraw(view);
            self.flush_view = Some(view);
        }
    }

    /// Blit this buffer onto another [`Dc`] as a bitmap.
    pub fn blit_to_dc(&self, target: &mut dyn crate::dc::dc::Dc, dest_x: i32, dest_y: i32) {
        let bgra = self.bgra.borrow().clone();
        if bgra.is_empty() {
            return;
        }
        let handle = mac_bitmap_store_bgra(&bgra, self.width, self.height);
        if handle == 0 {
            return;
        }
        let bmp = Bitmap::from_cache_handle(handle, self.width, self.height);
        target.draw_bitmap(&bmp, dest_x, dest_y);
    }

    pub fn client_size(view: NativeHandle) -> (u32, u32) {
        super::mac_view_client_size(view)
    }
}

impl Drop for MacCgDc {
    fn drop(&mut self) {
        self.flush();
    }
}
