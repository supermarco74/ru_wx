//! macOS font realisation via [`NSFont`].

use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSFontManager};
use objc2_foundation::MainThreadMarker;

use crate::core::font::FontDesc;

pub fn mac_font_from_desc(desc: &FontDesc) -> Retained<NSFont> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let size = (desc.point_size as f64).max(1.0);
    let face = if desc.face_name.is_empty() {
        "Helvetica"
    } else {
        desc.face_name.as_str()
    };
    let mut font = NSFont::fontWithName_size(&objc2_foundation::NSString::from_str(face), size)
        .unwrap_or_else(|| NSFont::systemFontOfSize(size));
    let manager = NSFontManager::sharedFontManager(mtm);
    if desc.bold {
        font = manager.convertFont_toHaveTrait(&font, objc2_app_kit::NSFontTraitMask::BoldFontMask);
    }
    if desc.italic {
        font =
            manager.convertFont_toHaveTrait(&font, objc2_app_kit::NSFontTraitMask::ItalicFontMask);
    }
    if desc.underline {
        // Underline is an attribute on attributed strings; keep the base font.
    }
    font
}

pub fn mac_font_handle(font: &Retained<NSFont>) -> isize {
    Retained::as_ptr(font) as isize
}
