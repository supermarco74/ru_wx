//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `ScrolledWindow` — scrollable container with real,
//! pannable content.
//!
//! Demonstrates:
//! - `set_virtual_size` (vertical-only overflow: the virtual height is
//!   much taller than the view, the virtual width fits)
//! - 30 `StaticText` rows parented *inside* the scrolled window and
//!   repositioned live as the view scrolls — genuine panning
//! - Live handling of all nine [`ScrolledWindowScrollEvent`] variants:
//!   line / page / thumb-track / thumb-release / top / bottom events
//!   move the view via `set_view_position`
//! - The current view position reported in a footer label and the raw
//!   event name in the StatusBar
//! - Buttons that jump the view to Top / Middle / Bottom
//!
//! Run with:
//! ```bash
//! cargo run --example mt_scrolled
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, Frame, ScrolledWindow, ScrolledWindowScrollEvent, StaticText,
    StatusBar, Widget,
};

const ROWS: i32 = 30;
const ROW_HEIGHT: i32 = 34;
const LINE_STEP: i32 = 17;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ScrolledWindow (live panning)")
        .with_size(700, 520)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Use the vertical scroll bar — the rows really move.", 0);

    let hint = StaticText::new(&frame, "30 rows inside the window; scroll to pan over them.");

    let scrolled = ScrolledWindow::new(&frame);
    Widget::set_size(&mut *scrolled.as_widget_ref().borrow_mut(), 660, 380);

    // ── Content: 30 labelled rows parented to the scrolled window ───────
    // Each row remembers its base y; panning subtracts the current
    // view offset from it.
    let mut rows = Vec::with_capacity(ROWS as usize);
    for i in 0..ROWS {
        let text = format!(
            "Row {:02} — virtual y = {}  {}",
            i + 1,
            10 + i * ROW_HEIGHT,
            if i % 5 == 0 { "◄ section marker" } else { "" }
        );
        let label = StaticText::new(&scrolled, &text);
        let base_y = 10 + i * ROW_HEIGHT;
        Widget::set_position(&mut *label.as_widget_ref().borrow_mut(), 10, base_y);
        Widget::set_size(&mut *label.as_widget_ref().borrow_mut(), 600, 22);
        rows.push((label.as_widget_ref(), 10, base_y));
    }
    let rows = Rc::new(rows);

    // Virtual area: same width as the view (no horizontal bar), much
    // taller than the view (vertical bar active).
    let virtual_h = 20 + ROWS * ROW_HEIGHT;
    scrolled.set_virtual_size(200, virtual_h);

    let footer = StaticText::new(&frame, "View position: (0, 0)");

    // ── Shared "scroll to y" routine: clamps, moves the thumb and pans
    //    the content rows. ──────────────────────────────────────────────
    let apply: Rc<dyn Fn(i32)> = {
        let scrolled = scrolled.clone();
        let rows = rows.clone();
        let footer = footer.clone();
        let status = status.clone();
        Rc::new(move |y: i32| {
            let view_h = scrolled.as_widget_ref().borrow().rect().height as i32;
            let (_, vh) = scrolled.get_virtual_size();
            let max_y = (vh - view_h).max(0);
            let y = y.clamp(0, max_y);
            scrolled.set_view_position(0, y);
            for (widget, base_x, base_y) in rows.iter() {
                if let Ok(mut w) = widget.try_borrow_mut() {
                    w.set_position(*base_x, *base_y - y);
                }
            }
            footer.set_label(&format!("View position: (0, {y}) of 0..{max_y}"));
            status.set_status_text(&format!("y = {y} / {max_y}"), 1);
        })
    };

    // ── Scroll events: all nine variants drive the view live ────────────
    let scrolled_cb = scrolled.clone();
    let apply_cb = apply.clone();
    let s = status.clone();
    scrolled.on_scroll(move |ev: ScrolledWindowScrollEvent| {
        let (_, y) = scrolled_cb.get_view_position();
        let view_h = scrolled_cb.as_widget_ref().borrow().rect().height as i32;
        let (_, vh) = scrolled_cb.get_virtual_size();
        let target = match ev {
            ScrolledWindowScrollEvent::LineUp => y - LINE_STEP,
            ScrolledWindowScrollEvent::LineDown => y + LINE_STEP,
            ScrolledWindowScrollEvent::PageUp => y - view_h,
            ScrolledWindowScrollEvent::PageDown => y + view_h,
            ScrolledWindowScrollEvent::ThumbRelease { position } => position,
            ScrolledWindowScrollEvent::ThumbTrack { position } => position,
            ScrolledWindowScrollEvent::Top => 0,
            ScrolledWindowScrollEvent::Bottom => vh,
            ScrolledWindowScrollEvent::EndScroll => y,
        };
        apply_cb(target);
        s.set_status_text(&format!("Scroll event: {ev:?}"), 0);
    });

    // ── Jump buttons ─────────────────────────────────────────────────────
    let btn_top = Button::new(&frame, "Top");
    let a = apply.clone();
    btn_top.on_click(&frame, move || a(0));

    let btn_middle = Button::new(&frame, "Middle");
    let a = apply.clone();
    btn_middle.on_click(&frame, move || a(virtual_h / 2));

    let btn_bottom = Button::new(&frame, "Bottom");
    let a = apply.clone();
    btn_bottom.on_click(&frame, move || a(virtual_h));

    // ── Layout ───────────────────────────────────────────────────────────
    let mut buttons_row = BoxSizer::horizontal();
    buttons_row.add(btn_top.as_widget_ref());
    buttons_row.add(btn_middle.as_widget_ref());
    buttons_row.add(btn_bottom.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(4);
    sizer.add(hint.as_widget_ref());
    sizer.add_with_proportion(scrolled.as_widget_ref(), 1);
    sizer.add(footer.as_widget_ref());
    sizer.add_sizer(buttons_row);
    frame.set_sizer(sizer);

    app.run(frame);
}
