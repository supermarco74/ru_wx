//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `StaticLine` — separators inside a realistic little form.
//!
//! Demonstrates:
//! - Horizontal lines (`new` + `new_horizontal`) separating the form
//!   title, the input fields, the live summary and the button row
//! - A vertical line (`new_vertical`) splitting the summary area into
//!   two columns inside a nested horizontal sizer
//! - `orientation()` round-trip for all three constructors
//! - A title with a custom bold `Font`, `TextCtrl` fields with live
//!   `on_change` character counts, ToolTips, and a nested button row
//!   added with `add_sizer`
//!
//! Run with:
//! ```bash
//! cargo run --example mt_static_line
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Button, Font, FontDesc, Frame, StaticLine, StaticLineOrientation, StaticText,
    StatusBar, TextCtrl, ToolTip, Widget,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StaticLine (form separators)")
        .with_size(520, 420)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Fill in the form and press Submit.", 0);
    status.set_status_text("name: 0 chars, email: 0 chars", 1);

    // ── Title with a custom font ────────────────────────────────────────
    let title = StaticText::new(&frame, "Create account");
    let title_font = Font::new(FontDesc::new("Segoe UI", 16).bold());
    title.set_font(&title_font);
    Widget::set_size(&mut *title.as_widget_ref().borrow_mut(), 300, 32);

    // ── Separators: all three constructors, orientation round-trip ─────
    let line_top = StaticLine::new(&frame, StaticLineOrientation::Horizontal);
    assert_eq!(line_top.orientation(), StaticLineOrientation::Horizontal);
    let line_mid = StaticLine::new_horizontal(&frame);
    assert_eq!(line_mid.orientation(), StaticLineOrientation::Horizontal);
    let line_bottom = StaticLine::new_horizontal(&frame);
    let line_v = StaticLine::new_vertical(&frame);
    assert_eq!(line_v.orientation(), StaticLineOrientation::Vertical);

    // ── Input fields ─────────────────────────────────────────────────────
    let lbl_name = StaticText::new(&frame, "Name:");
    Widget::set_size(&mut *lbl_name.as_widget_ref().borrow_mut(), 70, 24);
    let txt_name = TextCtrl::new(&frame, "");
    ToolTip::new("Your display name").attach(&txt_name.as_widget_ref());

    let lbl_email = StaticText::new(&frame, "Email:");
    Widget::set_size(&mut *lbl_email.as_widget_ref().borrow_mut(), 70, 24);
    let txt_email = TextCtrl::new(&frame, "");
    ToolTip::new("Where the confirmation goes").attach(&txt_email.as_widget_ref());

    // Live character counts in the second StatusBar field.
    let count_status = {
        let txt_name = txt_name.clone();
        let txt_email = txt_email.clone();
        let status = status.clone();
        move || {
            status.set_status_text(
                &format!(
                    "name: {} chars, email: {} chars",
                    txt_name.get_value().chars().count(),
                    txt_email.get_value().chars().count()
                ),
                1,
            );
        }
    };
    let cs = count_status.clone();
    txt_name.on_change(&frame, cs);
    let cs = count_status.clone();
    txt_email.on_change(&frame, cs);

    // ── Summary: two columns split by the vertical line ─────────────────
    let sum_left = StaticText::new(&frame, "Name preview:\n(empty)");
    let sum_right = StaticText::new(&frame, "Email preview:\n(empty)");
    Widget::set_size(&mut *sum_left.as_widget_ref().borrow_mut(), 200, 60);
    Widget::set_size(&mut *sum_right.as_widget_ref().borrow_mut(), 200, 60);

    // ── Buttons (nested row added with add_sizer) ────────────────────────
    let btn_submit = Button::new(&frame, "Submit");
    let tn = txt_name.clone();
    let te = txt_email.clone();
    let sl = sum_left.clone();
    let sr = sum_right.clone();
    let s = status.clone();
    btn_submit.on_click(&frame, move || {
        let name = tn.get_value();
        let email = te.get_value();
        sl.set_label(&format!(
            "Name preview:\n{}",
            if name.is_empty() { "(empty)" } else { name.as_str() }
        ));
        sr.set_label(&format!(
            "Email preview:\n{}",
            if email.is_empty() { "(empty)" } else { email.as_str() }
        ));
        s.set_status_text(&format!("Submitted: '{name}' <{email}>"), 0);
    });

    let btn_reset = Button::new(&frame, "Reset");
    let tn = txt_name.clone();
    let te = txt_email.clone();
    let sl = sum_left.clone();
    let sr = sum_right.clone();
    let s = status.clone();
    btn_reset.on_click(&frame, move || {
        tn.clear();
        te.clear();
        sl.set_label("Name preview:\n(empty)");
        sr.set_label("Email preview:\n(empty)");
        s.set_status_text("Form reset.", 0);
    });

    // ── Layout ───────────────────────────────────────────────────────────
    let mut name_row = BoxSizer::horizontal();
    name_row.add(lbl_name.as_widget_ref());
    name_row.add_with_proportion(txt_name.as_widget_ref(), 1);

    let mut email_row = BoxSizer::horizontal();
    email_row.add(lbl_email.as_widget_ref());
    email_row.add_with_proportion(txt_email.as_widget_ref(), 1);

    let mut summary_row = BoxSizer::horizontal();
    summary_row.add_with_proportion(sum_left.as_widget_ref(), 1);
    summary_row.add(line_v.as_widget_ref());
    summary_row.add_with_proportion(sum_right.as_widget_ref(), 1);

    let mut buttons_row = BoxSizer::horizontal();
    buttons_row.add(btn_submit.as_widget_ref());
    buttons_row.add(btn_reset.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(6);
    sizer.add(title.as_widget_ref());
    sizer.add(line_top.as_widget_ref());
    sizer.add_sizer(name_row);
    sizer.add_sizer(email_row);
    sizer.add(line_mid.as_widget_ref());
    sizer.add_sizer(summary_row);
    sizer.add(line_bottom.as_widget_ref());
    sizer.add_sizer(buttons_row);
    frame.set_sizer(sizer);

    app.run(frame);
}
