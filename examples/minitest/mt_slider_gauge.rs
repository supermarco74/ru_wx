//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Slider`, `Gauge`, `SpinCtrl`, `SpinCtrlDouble` and `Timer`
//! — a small "numeric controls playground".
//!
//! Demonstrates:
//! - Horizontal slider driving two gauges (segmented + smooth with a
//!   custom bar colour) and a live percentage label
//! - Vertical slider (`Slider::new_vertical`) driving a vertical gauge
//! - Indeterminate / marquee gauge (`Gauge::new_with_style` +
//!   `pulse` / `stop_pulse`) toggled by a button
//! - `Timer` animating a stepped gauge (`set_step` / `step`), with
//!   start / stop buttons and a `SpinCtrl` for the tick interval
//! - `SpinCtrlDouble` controlling the animation step size
//! - StatusBar feedback, ToolTips and nested sizers (`add_sizer`)
//!
//! Run with:
//! ```bash
//! cargo run --example mt_slider_gauge
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use ru_wx::controls::gauge::GaugeStyle;
use ru_wx::{
    App, BoxSizer, Button, Colour, Font, FontDesc, Frame, Gauge, Slider, SpinCtrl, SpinCtrlDouble,
    StaticText, StatusBar, Timer, ToolTip, Widget,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Slider / Gauge / Spin / Timer")
        .with_size(560, 640)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Move a slider, spin a value, start the timer…", 0);
    status.set_status_text("timer: stopped", 1);

    // ── Title with a custom font ────────────────────────────────────────
    let title = StaticText::new(&frame, "Numeric controls playground");
    let title_font = Font::new(FontDesc::new("Segoe UI", 14).bold());
    title.set_font(&title_font);
    Widget::set_size(&mut *title.as_widget_ref().borrow_mut(), 400, 28);

    // ── Section 1: horizontal slider → two gauges + percent label ──────
    let lbl_slider = StaticText::new(&frame, "Slider (0..100) drives both gauges:");
    let slider = Slider::new(&frame, 0, 100, 40);
    slider.set_tick_freq(10);
    slider.set_line_size(1);
    slider.set_page_size(10);
    ToolTip::new("Drag me — arrow keys move by 1, PgUp/PgDn by 10").attach(&slider.as_widget_ref());

    let gauge_seg = Gauge::new(&frame, 100);
    gauge_seg.set_value(40);
    let gauge_smooth = Gauge::new_smooth(&frame, 100);
    gauge_smooth.set_value(40);
    gauge_smooth.set_bar_colour(Colour::new(206, 106, 40, 255));
    let lbl_percent = StaticText::new(&frame, "Value: 40 (40%)");

    let slider_for_cb = slider.clone();
    let g1 = gauge_seg.clone();
    let g2 = gauge_smooth.clone();
    let lbl_p = lbl_percent.clone();
    let s = status.clone();
    slider.on_value_change(&frame, move || {
        let v = slider_for_cb.get_value();
        g1.set_value(v);
        g2.set_value(v);
        let (min, max) = slider_for_cb.get_range();
        let pct = if max > min { (v - min) * 100 / (max - min) } else { 0 };
        lbl_p.set_label(&format!("Value: {v} ({pct}%)"));
        s.set_status_text(&format!("Horizontal slider = {v}"), 0);
    });

    // ── Section 2: vertical slider → vertical gauge (nested row) ───────
    let lbl_vertical = StaticText::new(&frame, "Vertical slider → vertical gauge:");
    let slider_v = Slider::new_vertical(&frame, 0, 100, 60);
    slider_v.set_tick_freq(20);
    Widget::set_size(&mut *slider_v.as_widget_ref().borrow_mut(), 50, 150);
    let gauge_v = Gauge::new_vertical(&frame, 100);
    Widget::set_size(&mut *gauge_v.as_widget_ref().borrow_mut(), 40, 150);
    gauge_v.set_value(60);
    let lbl_v_value = StaticText::new(&frame, "60 / 100");
    Widget::set_size(&mut *lbl_v_value.as_widget_ref().borrow_mut(), 120, 20);

    let slider_v_cb = slider_v.clone();
    let gv = gauge_v.clone();
    let lbl_vv = lbl_v_value.clone();
    let s = status.clone();
    slider_v.on_value_change(&frame, move || {
        let v = slider_v_cb.get_value();
        gv.set_value(v);
        lbl_vv.set_label(&format!("{v} / 100"));
        s.set_status_text(&format!("Vertical slider = {v}"), 0);
    });

    // ── Section 3: marquee (indeterminate) gauge ────────────────────────
    let lbl_marquee = StaticText::new(&frame, "Marquee gauge (indeterminate):");
    let gauge_marquee = Gauge::new_with_style(&frame, 100, GaugeStyle::SmoothHorizontal, true);
    let marquee_running = Rc::new(Cell::new(true));
    let btn_marquee = Button::new(&frame, "Stop marquee");
    let gm = gauge_marquee.clone();
    let mr = marquee_running.clone();
    let btn_m = btn_marquee.clone();
    let s = status.clone();
    btn_marquee.on_click(&frame, move || {
        if mr.get() {
            gm.stop_pulse();
            mr.set(false);
            btn_m.set_label("Start marquee");
            s.set_status_text("Marquee stopped", 0);
        } else {
            gm.pulse();
            mr.set(true);
            btn_m.set_label("Stop marquee");
            s.set_status_text("Marquee running", 0);
        }
    });

    // ── Section 4: timer-animated gauge ─────────────────────────────────
    let lbl_timer = StaticText::new(&frame, "Timer-animated gauge (step + wrap):");
    let gauge_anim = Gauge::new(&frame, 100);
    gauge_anim.set_step(2);

    let timer = Rc::new(Timer::new(&frame));
    let ga = gauge_anim.clone();
    timer.on_tick(move || {
        if ga.get_value() >= ga.get_range() {
            ga.set_value(0);
        } else {
            ga.step();
        }
    });

    // SpinCtrl: tick interval in milliseconds (applied on Start).
    let lbl_interval = StaticText::new(&frame, "Interval (ms):");
    Widget::set_size(&mut *lbl_interval.as_widget_ref().borrow_mut(), 90, 24);
    let spin_interval = SpinCtrl::new(&frame, 20, 1000, 100);
    ToolTip::new("Tick interval used when the timer starts").attach(&spin_interval.as_widget_ref());

    // SpinCtrlDouble: step multiplier for the animated gauge.
    let lbl_step = StaticText::new(&frame, "Step size:");
    Widget::set_size(&mut *lbl_step.as_widget_ref().borrow_mut(), 90, 24);
    let spin_step = SpinCtrlDouble::new(&frame, 2.0, 0.5, 10.0, 0.5, 1);
    let ga = gauge_anim.clone();
    let s = status.clone();
    spin_step.on_value_change(&frame, move |v| {
        let step = v.round().max(1.0) as i32;
        ga.set_step(step);
        s.set_status_text(&format!("Gauge step set to {step} (spin = {v:.1})"), 0);
    });

    let btn_start = Button::new(&frame, "Start timer");
    let t = timer.clone();
    let spin_i = spin_interval.clone();
    let s = status.clone();
    btn_start.on_click(&frame, move || {
        let ms = spin_i.get_value().max(20) as u64;
        t.start(Duration::from_millis(ms));
        s.set_status_text(&format!("timer: running every {ms} ms"), 1);
    });

    let btn_stop = Button::new(&frame, "Stop timer");
    let t = timer.clone();
    let s = status.clone();
    btn_stop.on_click(&frame, move || {
        t.stop();
        s.set_status_text("timer: stopped", 1);
    });

    // ── Layout ───────────────────────────────────────────────────────────
    let mut vertical_row = BoxSizer::horizontal();
    vertical_row.add(slider_v.as_widget_ref());
    vertical_row.add(gauge_v.as_widget_ref());
    vertical_row.add(lbl_v_value.as_widget_ref());

    let mut interval_row = BoxSizer::horizontal();
    interval_row.add(lbl_interval.as_widget_ref());
    interval_row.add(spin_interval.as_widget_ref());

    let mut step_row = BoxSizer::horizontal();
    step_row.add(lbl_step.as_widget_ref());
    step_row.add(spin_step.as_widget_ref());

    let mut buttons_row = BoxSizer::horizontal();
    buttons_row.add(btn_start.as_widget_ref());
    buttons_row.add(btn_stop.as_widget_ref());
    buttons_row.add(btn_marquee.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(4);
    sizer.add(title.as_widget_ref());
    sizer.add(lbl_slider.as_widget_ref());
    sizer.add(slider.as_widget_ref());
    sizer.add(gauge_seg.as_widget_ref());
    sizer.add(gauge_smooth.as_widget_ref());
    sizer.add(lbl_percent.as_widget_ref());
    sizer.add(lbl_vertical.as_widget_ref());
    sizer.add_sizer(vertical_row);
    sizer.add(lbl_marquee.as_widget_ref());
    sizer.add(gauge_marquee.as_widget_ref());
    sizer.add(lbl_timer.as_widget_ref());
    sizer.add(gauge_anim.as_widget_ref());
    sizer.add_sizer(interval_row);
    sizer.add_sizer(step_row);
    sizer.add_sizer(buttons_row);
    frame.set_sizer(sizer);

    app.run(frame);
}
