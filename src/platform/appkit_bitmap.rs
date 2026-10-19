//! macOS raster bitmap cache — BGRA / SVG → [`NSImage`] handles for [`RawBitmap`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_app_kit::NSImage;
use objc2_foundation::MainThreadMarker;

use super::{alloc_widget_handle, nsimage_from_bgra, static_bitmap_set_image, NativeHandle};

struct MacBitmapEntry {
    image: Retained<NSImage>,
    bgra: Rc<RefCell<Vec<u8>>>,
    width: u32,
    height: u32,
}

thread_local! {
    static MAC_BITMAPS: RefCell<HashMap<NativeHandle, MacBitmapEntry>> =
        RefCell::new(HashMap::new());
}

fn store_entry(bgra: Vec<u8>, width: u32, height: u32, image: Retained<NSImage>) -> NativeHandle {
    let handle = alloc_widget_handle();
    MAC_BITMAPS.with(|m| {
        m.borrow_mut().insert(
            handle,
            MacBitmapEntry {
                image,
                bgra: Rc::new(RefCell::new(bgra)),
                width,
                height,
            },
        );
    });
    handle
}

/// Store premultiplied BGRA pixels and return a cache handle for [`RawBitmap::hbitmap`].
pub fn mac_bitmap_store_bgra(bgra: &[u8], width: u32, height: u32) -> NativeHandle {
    let Some(mtm) = MainThreadMarker::new() else {
        return 0;
    };
    if width == 0 || height == 0 {
        return 0;
    }
    let image = nsimage_from_bgra(mtm, bgra, width as i32, height as i32);
    store_entry(bgra.to_vec(), width, height, image)
}

/// Store an existing [`NSImage`] and return a cache handle.
pub fn mac_bitmap_store_image(image: Retained<NSImage>) -> NativeHandle {
    let size = image.size();
    let width = size.width.max(1.0) as u32;
    let height = size.height.max(1.0) as u32;
    let bgra = vec![0u8; (width * height * 4) as usize];

    // Note: Extracting pixel data from NSImage requires rendering to a CGContext.
    // This is complex and the BGRA buffer starts as zeros.
    // The actual pixel data will be populated when drawing into the bitmap.

    store_entry(bgra, width, height, image)
}

/// Return a cached [`NSImage`] for a bitmap handle, if present.
pub fn mac_bitmap_image_at(handle: NativeHandle) -> Option<Retained<NSImage>> {
    MAC_BITMAPS.with(|m| m.borrow().get(&handle).map(|e| e.image.clone()))
}

/// Borrow the writable BGRA buffer backing a cached bitmap.
pub fn mac_bitmap_attach_bgra(handle: NativeHandle) -> Option<(Rc<RefCell<Vec<u8>>>, u32, u32)> {
    MAC_BITMAPS.with(|m| {
        m.borrow()
            .get(&handle)
            .map(|e| (e.bgra.clone(), e.width, e.height))
    })
}

pub fn mac_bitmap_bgra_dims(handle: NativeHandle) -> Option<(u32, u32)> {
    MAC_BITMAPS.with(|m| m.borrow().get(&handle).map(|e| (e.width, e.height)))
}

/// Rebuild the cached [`NSImage`] from the current BGRA buffer.
pub fn mac_bitmap_sync_image(
    handle: NativeHandle,
    bgra: &Rc<RefCell<Vec<u8>>>,
    width: u32,
    height: u32,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    MAC_BITMAPS.with(|m| {
        if let Some(entry) = m.borrow_mut().get_mut(&handle) {
            let slice = bgra.borrow();
            entry.image = nsimage_from_bgra(mtm, &slice, width as i32, height as i32);
            entry.width = width;
            entry.height = height;
        }
    });
}

/// Release a cached bitmap (no-op if unknown).
pub fn mac_bitmap_release(handle: NativeHandle) {
    if handle == 0 {
        return;
    }
    MAC_BITMAPS.with(|m| m.borrow_mut().remove(&handle));
}

/// Apply a cached bitmap handle to a static-bitmap view.
pub fn static_bitmap_apply_cached(view_handle: NativeHandle, bitmap_handle: NativeHandle) {
    if bitmap_handle == 0 {
        return;
    }
    MAC_BITMAPS.with(|m| {
        if let Some(entry) = m.borrow().get(&bitmap_handle) {
            static_bitmap_set_image(view_handle, &entry.image);
        }
    });
}

/// Render SVG bytes at the given size and return a cache handle.
pub fn mac_bitmap_from_svg(svg: &[u8], width: u32, height: u32) -> NativeHandle {
    let Some(bgra) = crate::dc::icon::svg_bytes_to_bgra(svg, width, height) else {
        return 0;
    };
    mac_bitmap_store_bgra(&bgra, width, height)
}
