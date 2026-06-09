//! Minitest: `ScrolledWindow` — scrollable container with virtual size.
//!
//! Demonstrates the full `ScrolledWindow` API surface:
//! - `new` (default 200×200, virtual size `(0, 0)`)
//! - `set_virtual_size` / `get_virtual_size` round-trip — the
//!   first call expands the virtual area past the visible size
//!   so the scroll bars appear, the second shrinks the virtual
//!   area so the scroll bars disappear
//! - `set_view_position` / `get_view_position` round-trip
//! - `on_scroll` callback registration with a closure that
//!   pattern-matches the nine [`ScrolledWindowScrollEvent`]
//!   variants
//!
//! The contents of the scrolled window are a single large
//! [`StaticText`] — the scroll bars let the user pan over its
//! full width / height.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_scrolled
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, Frame, ScrolledWindow, ScrolledWindowScrollEvent, StaticText, Widget};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ScrolledWindow")
        .with_size(700, 500)
        .build();

    let scrolled = ScrolledWindow::new(&frame);
    Widget::set_size(&mut *scrolled.as_widget_ref().borrow_mut(), 660, 440);

    // ── Virtual size round-trip ─────────────────────────────────────────
    // Default virtual size is (0, 0) — no scroll bars shown. The
    // first call expands the virtual area to 4× the visible size,
    // so both scroll bars appear. The second call shrinks it back
    // to 0×0, so the scroll bars disappear.
    let _ = scrolled.get_virtual_size(); // expect (0, 0)
    scrolled.set_virtual_size(4 * 660, 4 * 440);
    let _ = scrolled.get_virtual_size(); // expect (2640, 1760)
    scrolled.set_virtual_size(0, 0);
    let _ = scrolled.get_virtual_size(); // expect (0, 0) again
    scrolled.set_virtual_size(2000, 1200);
    let _ = scrolled.get_virtual_size(); // expect (2000, 1200)

    // ── View position round-trip ────────────────────────────────────────
    // The view position is the (x, y) offset into the virtual
    // content. Scrolling the thumb moves the view position; the
    // setter updates the OS thumb and the cached value.
    let _ = scrolled.get_view_position(); // expect (0, 0)
    scrolled.set_view_position(50, 30);
    let _ = scrolled.get_view_position(); // expect (50, 30)
    scrolled.set_view_position(0, 0);
    let _ = scrolled.get_view_position(); // expect (0, 0)

    // ── Content ────────────────────────────────────────────────────────
    // A long StaticText label that overflows the scrolled window's
    // visible area; the scroll bars let the user pan over it.
    let _label = StaticText::new(
        &scrolled,
        "Scroll me! This label is much wider than the visible area, \
         and the virtual height is taller than the visible area, so \
         both the horizontal and vertical scroll bars should be active.",
    );

    // ── Scroll event callback ───────────────────────────────────────────
    // Register a callback that pattern-matches all nine scroll
    // event variants. The callback never fires from this
    // synchronous test (it would fire on real scroll interaction),
    // but the closure must compile and accept every variant.
    scrolled.on_scroll(|ev: ScrolledWindowScrollEvent| match ev {
        ScrolledWindowScrollEvent::LineUp => {}
        ScrolledWindowScrollEvent::LineDown => {}
        ScrolledWindowScrollEvent::PageUp => {}
        ScrolledWindowScrollEvent::PageDown => {}
        ScrolledWindowScrollEvent::ThumbRelease { position } => {
            let _ = position;
        }
        ScrolledWindowScrollEvent::ThumbTrack { position } => {
            let _ = position;
        }
        ScrolledWindowScrollEvent::Top => {}
        ScrolledWindowScrollEvent::Bottom => {}
        ScrolledWindowScrollEvent::EndScroll => {}
    });

    app.run(frame);
}
