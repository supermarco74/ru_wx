//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `ScrollBar` — standalone horizontal and vertical scroll
//! bars (child `SCROLLBAR` controls) made fully interactive.
//!
//! Demonstrates:
//! - `new_full` with custom range + page size for both orientations
//! - Live handling of **all nine** [`ScrollBarEvent`] variants: the
//!   callback computes the new thumb position (arrows, paging,
//!   Ctrl+Home / Ctrl+End, thumb drag) and pushes it back with
//!   `set_position`, so the bars actually move
//! - A percentage `StaticText` per bar, refreshed on every event
//! - The raw event name shown in the StatusBar as it happens
//! - Buttons that jump both bars to Home / Centre / End
//! - A `SpinCtrl` that changes the horizontal bar's page size at
//!   runtime (`set_page_size` / `get_page_size`)
//! - Layout entirely with nested `BoxSizer`s (`add_sizer`)
//!
//! Run with:
//! ```bash
//! cargo run --example mt_scroll_bar
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, Frame, ScrollBar, ScrollBarEvent, ScrollBarOrientation, SpinCtrl,
    StaticText, StatusBar, ToolTip, Widget,
};

/// Compute the new thumb position for a scroll event, given the
/// current position, range and page size of the bar.
fn next_position(ev: ScrollBarEvent, pos: i32, min: i32, max: i32, page: i32) -> i32 {
    let target = match ev {
        ScrollBarEvent::LineUp => pos - 1,
        ScrollBarEvent::LineDown => pos + 1,
        ScrollBarEvent::PageUp => pos - page,
        ScrollBarEvent::PageDown => pos + page,
        ScrollBarEvent::ThumbRelease { position } => position,
        ScrollBarEvent::ThumbTrack { position } => position,
        ScrollBarEvent::Top => min,
        ScrollBarEvent::Bottom => max,
        ScrollBarEvent::EndScroll => pos,
    };
    target.clamp(min, max)
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ScrollBar (interactive)")
        .with_size(640, 460)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click arrows, page areas or drag a thumb…", 0);

    let hint = StaticText::new(
        &frame,
        "Both bars react to every ScrollBarEvent — watch the status bar.",
    );

    // ── Horizontal bar: 0..1000, page 25 ────────────────────────────────
    let hbar = ScrollBar::new_full(&frame, ScrollBarOrientation::Horizontal, 0, 1000, 25);
    Widget::set_size(&mut *hbar.as_widget_ref().borrow_mut(), 400, 16);
    hbar.set_position(500);
    ToolTip::new("Horizontal: 0..1000").attach(&hbar.as_widget_ref());
    let lbl_h = StaticText::new(&frame, "");

    // ── Vertical bar: -100..100, page 10 ────────────────────────────────
    let vbar = ScrollBar::new_full(&frame, ScrollBarOrientation::Vertical, -100, 100, 10);
    Widget::set_size(&mut *vbar.as_widget_ref().borrow_mut(), 16, 200);
    vbar.set_position(0);
    ToolTip::new("Vertical: -100..100").attach(&vbar.as_widget_ref());
    let lbl_v = StaticText::new(&frame, "");

    // Shared label refresher: reads the live values back from both bars.
    let refresh: Rc<dyn Fn()> = {
        let hbar = hbar.clone();
        let vbar = vbar.clone();
        let lbl_h = lbl_h.clone();
        let lbl_v = lbl_v.clone();
        Rc::new(move || {
            let (hmin, hmax) = hbar.get_range();
            let hpos = hbar.get_position();
            let hpct = if hmax > hmin { (hpos - hmin) * 100 / (hmax - hmin) } else { 0 };
            lbl_h.set_label(&format!(
                "Horizontal: {hpos} in {hmin}..{hmax} ({hpct}%), page {}",
                hbar.get_page_size()
            ));
            let (vmin, vmax) = vbar.get_range();
            let vpos = vbar.get_position();
            let vpct = if vmax > vmin { (vpos - vmin) * 100 / (vmax - vmin) } else { 0 };
            lbl_v.set_label(&format!(
                "Vertical: {vpos} in {vmin}..{vmax} ({vpct}%), page {}",
                vbar.get_page_size()
            ));
        })
    };
    refresh();

    // ── Event wiring: pattern-match all nine variants, live ─────────────
    let hbar_cb = hbar.clone();
    let s = status.clone();
    let r = refresh.clone();
    hbar.on_scroll(&frame, move |ev: ScrollBarEvent| {
        let (min, max) = hbar_cb.get_range();
        let pos = hbar_cb.get_position();
        let page = hbar_cb.get_page_size();
        hbar_cb.set_position(next_position(ev, pos, min, max, page));
        s.set_status_text(&format!("Horizontal event: {ev:?}"), 0);
        s.set_status_text(&format!("H = {}", hbar_cb.get_position()), 1);
        r();
    });

    let vbar_cb = vbar.clone();
    let s = status.clone();
    let r = refresh.clone();
    vbar.on_scroll(&frame, move |ev: ScrollBarEvent| {
        let (min, max) = vbar_cb.get_range();
        let pos = vbar_cb.get_position();
        let page = vbar_cb.get_page_size();
        vbar_cb.set_position(next_position(ev, pos, min, max, page));
        s.set_status_text(&format!("Vertical event: {ev:?}"), 0);
        s.set_status_text(&format!("V = {}", vbar_cb.get_position()), 1);
        r();
    });

    // ── Jump buttons: Home / Centre / End on both bars ──────────────────
    let make_jump = |label: &str, frac: i32| -> Button {
        let btn = Button::new(&frame, label);
        let hbar = hbar.clone();
        let vbar = vbar.clone();
        let r = refresh.clone();
        let s = status.clone();
        let label = label.to_string();
        btn.on_click(&frame, move || {
            let (hmin, hmax) = hbar.get_range();
            hbar.set_position(hmin + (hmax - hmin) * frac / 100);
            let (vmin, vmax) = vbar.get_range();
            vbar.set_position(vmin + (vmax - vmin) * frac / 100);
            s.set_status_text(&format!("Jumped both bars to {label} ({frac}%)"), 0);
            r();
        });
        btn
    };
    let btn_home = make_jump("Home", 0);
    let btn_centre = make_jump("Centre", 50);
    let btn_end = make_jump("End", 100);

    // ── SpinCtrl: live page size for the horizontal bar ─────────────────
    let lbl_page = StaticText::new(&frame, "H page size:");
    Widget::set_size(&mut *lbl_page.as_widget_ref().borrow_mut(), 90, 24);
    let spin_page = SpinCtrl::new(&frame, 1, 250, 25);
    let hbar_for_spin = hbar.clone();
    let spin_for_cb = spin_page.clone();
    let s = status.clone();
    let r = refresh.clone();
    spin_page.on_value_change(&frame, move || {
        let page = spin_for_cb.get_value();
        hbar_for_spin.set_page_size(page);
        s.set_status_text(
            &format!("Horizontal page size = {}", hbar_for_spin.get_page_size()),
            0,
        );
        r();
    });

    // ── Layout ───────────────────────────────────────────────────────────
    let mut labels_col = BoxSizer::vertical();
    labels_col.add(lbl_v.as_widget_ref());
    labels_col.add(lbl_h.as_widget_ref());
    labels_col.add_stretch(1);

    let mut middle_row = BoxSizer::horizontal();
    middle_row.add(vbar.as_widget_ref());
    middle_row.add_spacer(12);
    middle_row.add_sizer_with_proportion(labels_col, 1);

    let mut buttons_row = BoxSizer::horizontal();
    buttons_row.add(btn_home.as_widget_ref());
    buttons_row.add(btn_centre.as_widget_ref());
    buttons_row.add(btn_end.as_widget_ref());

    let mut page_row = BoxSizer::horizontal();
    page_row.add(lbl_page.as_widget_ref());
    page_row.add(spin_page.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(6);
    sizer.add(hint.as_widget_ref());
    sizer.add(hbar.as_widget_ref());
    sizer.add_sizer_with_proportion(middle_row, 1);
    sizer.add_sizer(buttons_row);
    sizer.add_sizer(page_row);
    frame.set_sizer(sizer);

    app.run(frame);
}
