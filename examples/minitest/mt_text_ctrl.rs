//! Minitest: `TextCtrl` — single-line, multiline and password fields.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_text_ctrl
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, StaticText, StatusBar, TextCtrl};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TextCtrl")
        .with_size(560, 460)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Type into the fields.", 0);

    // 1. Single-line
    let lbl1 = StaticText::new(&frame, "Single-line:");
    let single = TextCtrl::new(&frame, "Hello world");
    let single_clone = single.clone();
    let s = status.clone();
    single.on_change(&frame, move || {
        s.set_status_text(&format!("Single: {:?}", single_clone.get_value()), 0);
    });

    // 2. Multiline
    let lbl2 = StaticText::new(&frame, "Multiline (3+ lines):");
    let multi = TextCtrl::multiline(&frame, "First line\nSecond line\nThird line — type freely.");

    // 3. Password
    let lbl3 = StaticText::new(&frame, "Password:");
    let pwd = TextCtrl::password(&frame, "");

    // 4. Read-only
    let lbl4 = StaticText::new(&frame, "Read-only:");
    let ro = TextCtrl::new(&frame, "you cannot edit me");
    ro.set_readonly(true);

    // 5. Buttons: append a line / show password value
    let multi_for_btn = multi.clone();
    let btn_append = Button::new(&frame, "Append line to multiline");
    let counter = std::rc::Rc::new(std::cell::Cell::new(0u32));
    btn_append.on_click(&frame, move || {
        counter.set(counter.get() + 1);
        multi_for_btn.append_text(&format!("\nappended #{}", counter.get()));
    });

    let pwd_for_btn = pwd.clone();
    let s = status.clone();
    let btn_pwd = Button::new(&frame, "Show password value");
    btn_pwd.on_click(&frame, move || {
        s.set_status_text(&format!("Password = {:?}", pwd_for_btn.get_value()), 0);
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl1.as_widget_ref());
    sizer.add(single.as_widget_ref());
    sizer.add(lbl2.as_widget_ref());
    sizer.add(multi.as_widget_ref());
    sizer.add(lbl3.as_widget_ref());
    sizer.add(pwd.as_widget_ref());
    sizer.add(lbl4.as_widget_ref());
    sizer.add(ro.as_widget_ref());
    sizer.add(btn_append.as_widget_ref());
    sizer.add(btn_pwd.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
