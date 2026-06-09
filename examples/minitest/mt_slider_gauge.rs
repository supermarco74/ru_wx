//! Minitest: `Slider`, `Gauge` and `SpinCtrl` — numeric controls.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_slider_gauge
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, Gauge, Slider, SpinCtrl, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Slider / Gauge / SpinCtrl")
        .with_size(460, 360)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Move the slider — the gauge follows.", 0);

    // Slider 0..100
    let lbl_slider = StaticText::new(&frame, "Slider (0..100):");
    let slider = Slider::new(&frame, 0, 100, 30);
    slider.set_tick_freq(10);

    // Gauge 0..100 — driven by the slider
    let lbl_gauge = StaticText::new(&frame, "Gauge (driven by slider):");
    let gauge = Gauge::new(&frame, 100);
    gauge.set_value(30);

    // SpinCtrl 0..1000 — independent
    let lbl_spin = StaticText::new(&frame, "SpinCtrl (0..1000):");
    let spin = SpinCtrl::new(&frame, 0, 1000, 250);

    // Button: copy the spin value into the gauge (clamped 0..100)
    let spin_for_btn = spin.clone();
    let gauge_for_btn = gauge.clone();
    let s = status.clone();
    let btn = Button::new(&frame, "Copy spin → gauge (mod 100)");
    btn.on_click(&frame, move || {
        let v = spin_for_btn.get_value();
        let mapped = v.rem_euclid(101);
        gauge_for_btn.set_value(mapped);
        s.set_status_text(&format!("Spin = {v} → gauge = {mapped}"), 0);
    });

    // Wire the slider to the gauge
    let slider_for_cb = slider.clone();
    let gauge_for_cb = gauge.clone();
    let s = status.clone();
    slider.on_value_change(&frame, move || {
        let v = slider_for_cb.get_value();
        gauge_for_cb.set_value(v);
        s.set_status_text(&format!("Slider = {v}"), 0);
    });

    // SpinCtrl status
    let spin_for_cb = spin.clone();
    let s = status.clone();
    spin.on_value_change(&frame, move || {
        s.set_status_text(&format!("Spin = {}", spin_for_cb.get_value()), 0);
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl_slider.as_widget_ref());
    sizer.add(slider.as_widget_ref());
    sizer.add(lbl_gauge.as_widget_ref());
    sizer.add(gauge.as_widget_ref());
    sizer.add(lbl_spin.as_widget_ref());
    sizer.add(spin.as_widget_ref());
    sizer.add(btn.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
