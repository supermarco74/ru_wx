//! macOS [`StaticText`] wrapping via [`NSTextField`] / [`NSCell`].

use std::cell::RefCell;
use std::collections::HashMap;

use objc2_app_kit::{NSLineBreakMode, NSTextField};

use super::{NativeHandle, VIEWS};

thread_local! {
    static STATIC_TEXT_MAX_LINES: RefCell<HashMap<NativeHandle, i32>> =
        RefCell::new(HashMap::new());
}

/// Configure word wrapping (`max_lines`: 0 = unlimited, 1 = single line).
pub fn static_text_set_max_lines(view_handle: NativeHandle, max_lines: i32) {
    STATIC_TEXT_MAX_LINES.with(|map| {
        map.borrow_mut().insert(view_handle, max_lines);
    });
    apply_wrap(view_handle);
}

/// Re-apply wrap width after the label is resized.
pub fn static_text_refresh_wrap(view_handle: NativeHandle) {
    if STATIC_TEXT_MAX_LINES.with(|map| map.borrow().contains_key(&view_handle)) {
        apply_wrap(view_handle);
    }
}

fn apply_wrap(view_handle: NativeHandle) {
    let max_lines =
        STATIC_TEXT_MAX_LINES.with(|map| map.borrow().get(&view_handle).copied().unwrap_or(1));

    VIEWS.with(|map| {
        let views = map.borrow();
        let Some(view) = views.get(&view_handle) else {
            return;
        };
        let Some(field) = view.downcast_ref::<NSTextField>() else {
            return;
        };

        if max_lines <= 1 {
            // Default label behaviour: single line with tail truncation.
            // Avoid `setMaximumNumberOfLines(1)` — it can raise AppKit exceptions
            // on `labelWithString` fields on some macOS versions.
            if let Some(cell) = field.cell() {
                cell.setWraps(false);
                cell.setUsesSingleLineMode(true);
                cell.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
                cell.setTruncatesLastVisibleLine(false);
            }
            return;
        }

        let width = field.frame().size.width;
        if width > 0.0 {
            field.setPreferredMaxLayoutWidth(width);
        }
        field.setMaximumNumberOfLines(max_lines as isize);

        if let Some(cell) = field.cell() {
            cell.setWraps(true);
            cell.setUsesSingleLineMode(false);
            cell.setLineBreakMode(NSLineBreakMode::ByWordWrapping);
            cell.setTruncatesLastVisibleLine(true);
        }
    });
}
