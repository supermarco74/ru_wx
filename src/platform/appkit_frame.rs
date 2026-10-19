//! macOS top-level window chrome — minimize, zoom, full screen, icon, DPI helpers.

use objc2::rc::Retained;
use objc2_app_kit::{NSAppearanceCustomization, NSImage, NSScreen, NSWindow, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSPoint, NSSize, NSString};

use crate::core::dpi::{Dpi, SYSTEM_DPI};
use crate::core::geometry::Rect;

use super::{mac_bitmap_image_at, ns_window_for_frame_handle, NativeHandle};

fn dpi_from_scale(scale: f64) -> Dpi {
    Dpi::new((SYSTEM_DPI as f64 * scale).round().max(1.0) as u32)
}

pub fn mac_screen_dpi() -> Dpi {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let screen = NSScreen::mainScreen(mtm);
    dpi_from_scale(screen.map(|s| s.backingScaleFactor()).unwrap_or(1.0))
}

pub fn mac_dpi_for_frame(handle: NativeHandle) -> Dpi {
    ns_window_for_frame_handle(handle)
        .and_then(|w| w.screen())
        .map(|s| dpi_from_scale(s.backingScaleFactor()))
        .unwrap_or_else(mac_screen_dpi)
}

fn rect_contains(rect: objc2_core_foundation::CGRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.y >= rect.origin.y
        && point.x < rect.origin.x + rect.size.width
        && point.y < rect.origin.y + rect.size.height
}

pub fn mac_dpi_for_point(x: i32, y: i32) -> Dpi {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let point = NSPoint::new(x as f64, y as f64);
    for screen in NSScreen::screens(mtm) {
        if rect_contains(screen.frame(), point) {
            return dpi_from_scale(screen.backingScaleFactor());
        }
    }
    mac_screen_dpi()
}

pub fn mac_enumerate_displays() -> Vec<crate::core::display::Display> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let screens = NSScreen::screens(mtm);
    let count = screens.count();
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        let screen = screens.objectAtIndex(i);
        let frame = screen.frame();
        let visible = screen.visibleFrame();
        let rect = Rect::new(
            frame.origin.x.round() as i32,
            frame.origin.y.round() as i32,
            frame.size.width.round() as u32,
            frame.size.height.round() as u32,
        );
        let work = Rect::new(
            visible.origin.x.round() as i32,
            visible.origin.y.round() as i32,
            visible.size.width.round() as u32,
            visible.size.height.round() as u32,
        );
        let primary = i == 0;
        out.push(crate::core::display::Display::new(
            i as u32, rect, work, primary,
        ));
    }
    if out.is_empty() {
        out.push(crate::core::display::Display::new(
            0,
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            true,
        ));
    }
    out
}

pub fn mac_frame_get_title(handle: NativeHandle) -> String {
    ns_window_for_frame_handle(handle)
        .map(|w| w.title().to_string())
        .unwrap_or_default()
}

pub fn mac_frame_set_icon(handle: NativeHandle, image: Option<&NSImage>) {
    let Some(window) = ns_window_for_frame_handle(handle) else {
        return;
    };
    // Set the dock tile icon.
    if let Some(img) = image {
        let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        unsafe {
            app.setApplicationIconImage(Some(img));
        }
    }
}

pub fn mac_frame_set_icon_from_bitmap(handle: NativeHandle, bitmap_handle: NativeHandle) {
    if bitmap_handle == 0 {
        mac_frame_set_icon(handle, None);
        return;
    }
    if let Some(image) = mac_bitmap_image_at(bitmap_handle) {
        mac_frame_set_icon(handle, Some(&image));
    }
}

pub fn mac_frame_iconize(handle: NativeHandle) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        window.miniaturize(None);
    }
}

pub fn mac_frame_deminiaturize(handle: NativeHandle) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        window.deminiaturize(None);
    }
}

pub fn mac_frame_is_iconized(handle: NativeHandle) -> bool {
    ns_window_for_frame_handle(handle)
        .map(|w| w.isMiniaturized())
        .unwrap_or(false)
}

pub fn mac_frame_maximize(handle: NativeHandle) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        window.zoom(None);
    }
}

pub fn mac_frame_is_maximized(handle: NativeHandle) -> bool {
    ns_window_for_frame_handle(handle)
        .map(|w| w.isZoomed())
        .unwrap_or(false)
}

pub fn mac_frame_restore(handle: NativeHandle) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        if window.isMiniaturized() {
            window.deminiaturize(None);
        } else if window.isZoomed() {
            window.zoom(None);
        }
    }
}

pub fn mac_frame_show_full_screen(handle: NativeHandle, full_screen: bool) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        let mask = window.styleMask();
        let is_fs = mask.contains(NSWindowStyleMask::FullScreen);
        if full_screen && !is_fs {
            window.toggleFullScreen(None);
        } else if !full_screen && is_fs {
            window.toggleFullScreen(None);
        }
    }
}

pub fn mac_frame_is_full_screen(handle: NativeHandle) -> bool {
    ns_window_for_frame_handle(handle)
        .map(|w| w.styleMask().contains(NSWindowStyleMask::FullScreen))
        .unwrap_or(false)
}

pub fn mac_frame_hide(handle: NativeHandle) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        window.orderOut(None);
    }
}

pub fn mac_frame_center(handle: NativeHandle, parent: Option<NativeHandle>) {
    let Some(window) = ns_window_for_frame_handle(handle) else {
        return;
    };
    if let Some(parent_handle) = parent {
        if let Some(parent_win) = ns_window_for_frame_handle(parent_handle) {
            let pf = parent_win.frame();
            let wf = window.frame();
            let x = pf.origin.x + (pf.size.width - wf.size.width) / 2.0;
            let y = pf.origin.y + (pf.size.height - wf.size.height) / 2.0;
            window.setFrameOrigin(NSPoint::new(x, y));
            return;
        }
    }
    window.center();
}

pub fn mac_frame_request_attention(handle: NativeHandle) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if ns_window_for_frame_handle(handle).is_some() {
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        let _ = app
            .requestUserAttention(objc2_app_kit::NSRequestUserAttentionType::InformationalRequest);
    }
}

pub fn mac_frame_set_min_size(handle: NativeHandle, w: u32, h: u32) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        window.setMinSize(NSSize::new(w as f64, h as f64));
    }
}

pub fn mac_frame_set_max_size(handle: NativeHandle, w: u32, h: u32) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        let ww = if w == 0 { f64::MAX } else { w as f64 };
        let hh = if h == 0 { f64::MAX } else { h as f64 };
        window.setMaxSize(NSSize::new(ww, hh));
    }
}

pub fn mac_frame_set_size(handle: NativeHandle, w: u32, h: u32) {
    if let Some(window) = ns_window_for_frame_handle(handle) {
        let frame = window.frame();
        window.setFrame_display(
            objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(frame.origin.x, frame.origin.y),
                objc2_foundation::NSSize::new(w as f64, h as f64),
            ),
            true,
        );
    }
}

/// Map Win32 dark title bar to `NSAppearance` (dark aqua vs aqua).
pub fn mac_set_dark_title_bar(handle: NativeHandle, dark: bool) -> bool {
    let Some(window) = ns_window_for_frame_handle(handle) else {
        return false;
    };
    unsafe {
        let name = if dark {
            &objc2_app_kit::NSAppearanceNameDarkAqua
        } else {
            &objc2_app_kit::NSAppearanceNameAqua
        };
        if let Some(appearance) = objc2_app_kit::NSAppearance::appearanceNamed(name) {
            window.setAppearance(Some(&appearance));
            return true;
        }
    }
    false
}

pub fn mac_dark_title_bar(handle: NativeHandle) -> Option<bool> {
    ns_window_for_frame_handle(handle)
        .map(|w| w.effectiveAppearance().name().to_string().contains("Dark"))
}

/// Apply system-resolved dark title bar on macOS (no Mica/backdrop equivalent).
pub fn mac_apply_modern_style(handle: NativeHandle) -> bool {
    let dark = crate::core::appearance::Appearance::System.resolve();
    mac_set_dark_title_bar(handle, dark)
}
