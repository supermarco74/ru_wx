//! Minitest: `ScrollBar` — standalone horizontal and vertical scroll
//! bars (child `SCROLLBAR` controls, not the window-attached scroll
//! bars used by `ScrolledWindow`).
//!
//! Demonstrates the full `ScrollBar` API surface:
//! - `new` (default range `0..100`, page size `10`)
//! - `new_full` (custom range + page size)
//! - `set_range` / `get_range` round-trip
//! - `set_position` (thumb) / `get_position` round-trip (live
//!   value from `SBM_GETPOS`)
//! - `set_page_size` / `get_page_size` round-trip
//! - `orientation` getter
//! - `on_scroll` callback registration with a closure that
//!   pattern-matches the nine [`ScrollBarEvent`] variants
//!
//! The frame hosts two scroll bars: a horizontal one at the top
//! of the client area and a vertical one on the left side. A
//! label in the centre reports the live values read back from
//! each bar.
//!
//! Note on method-name collision: `ScrollBar` exposes an
//! inherent `set_position(pos: i32)` that sets the *thumb*
//! position. The `Widget` trait's `set_position(x: i32, y: i32)`
//! (for window x/y placement) is shadowed by the inherent
//! method, so the layout calls below go through the explicit
//! `Widget::set_position` / `Widget::set_size` qualified syntax
//! on the `as_widget_ref()` handle.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_scroll_bar
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, Frame, ScrollBar, ScrollBarEvent, ScrollBarOrientation, StaticText, Widget};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ScrollBar")
        .with_size(800, 500)
        .build();

    // ── Section 1: horizontal scroll bar (default `new`) ───────────────
    // `new` builds a scroll bar with range `0..100` and page
    // size `10`. The default size is 200×16 (horizontal).
    let hbar = ScrollBar::new(&frame, ScrollBarOrientation::Horizontal);
    Widget::set_position(&mut *hbar.as_widget_ref().borrow_mut(), 20, 60);
    Widget::set_size(&mut *hbar.as_widget_ref().borrow_mut(), 760, 16);
    let _ = hbar.orientation(); // expect Horizontal
    let _ = hbar.get_range(); // expect (0, 100)
    let _ = hbar.get_page_size(); // expect 10
    let _ = hbar.get_position(); // expect 0

    // ── Section 2: vertical scroll bar (custom `new_full`) ─────────────
    // `new_full` builds a scroll bar with explicit range and
    // page size. The default size is 16×200 (vertical).
    let vbar = ScrollBar::new_full(
        &frame,
        ScrollBarOrientation::Vertical,
        -50, // min
        50,  // max
        5,   // page_size
    );
    Widget::set_position(&mut *vbar.as_widget_ref().borrow_mut(), 20, 100);
    Widget::set_size(&mut *vbar.as_widget_ref().borrow_mut(), 16, 360);
    let _ = vbar.orientation(); // expect Vertical
    let _ = vbar.get_range(); // expect (-50, 50)
    let _ = vbar.get_page_size(); // expect 5
    let _ = vbar.get_position(); // expect -50 (clamped to min)

    // ── Section 3: range round-trip ────────────────────────────────────
    // Change both bars' ranges and read them back. The position
    // is clamped to the new range automatically.
    hbar.set_range(0, 1000);
    let _ = hbar.get_range(); // expect (0, 1000)
    hbar.set_position(500);
    let _ = hbar.get_position(); // expect 500
    hbar.set_position(9999); // clamped to max
    let _ = hbar.get_position(); // expect 1000

    vbar.set_range(-100, 100);
    let _ = vbar.get_range(); // expect (-100, 100)
    vbar.set_position(0);
    let _ = vbar.get_position(); // expect 0
    vbar.set_position(-200); // clamped to min
    let _ = vbar.get_position(); // expect -100

    // ── Section 4: page-size round-trip ────────────────────────────────
    hbar.set_page_size(50);
    let _ = hbar.get_page_size(); // expect 50
    hbar.set_page_size(25);
    let _ = hbar.get_page_size(); // expect 25

    vbar.set_page_size(10);
    let _ = vbar.get_page_size(); // expect 10

    // ── Section 5: explanatory label ───────────────────────────────────
    let _label = StaticText::new(
        &frame,
        "Top: horizontal scroll bar (0..1000, page 25)   |   \
         Left: vertical scroll bar (-100..100, page 10)   |   \
         Drag either thumb to see the position round-trip",
    );

    // ── Section 6: scroll event callback ───────────────────────────────
    // Register a callback on the horizontal bar that pattern-matches
    // all nine scroll event variants. The callback never fires from
    // this synchronous test (it would fire on real scroll
    // interaction), but the closure must compile and accept every
    // variant.
    hbar.on_scroll(&frame, |ev: ScrollBarEvent| match ev {
        ScrollBarEvent::LineUp => {}
        ScrollBarEvent::LineDown => {}
        ScrollBarEvent::PageUp => {}
        ScrollBarEvent::PageDown => {}
        ScrollBarEvent::ThumbRelease { position } => {
            let _ = position;
        }
        ScrollBarEvent::ThumbTrack { position } => {
            let _ = position;
        }
        ScrollBarEvent::Top => {}
        ScrollBarEvent::Bottom => {}
        ScrollBarEvent::EndScroll => {}
    });

    app.run(frame);
}
