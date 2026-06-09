//! Minitest: `ListBox` and `CheckListBox` — list-style selection controls.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_list_box
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, CheckListBox, Frame, ListBox, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ListBox / CheckListBox")
        .with_size(460, 420)
        .build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click items / toggle checks.", 0);

    // ListBox
    let lbl1 = StaticText::new(&frame, "ListBox (single-click + double-click):");
    let list = ListBox::new(&frame);
    for item in ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"] {
        list.append(item);
    }
    let s = status.clone();
    list.on_selection_change(&frame, move || {
        s.set_status_text("ListBox: selection changed", 0);
    });
    let s = status.clone();
    list.on_double_click(&frame, move || {
        s.set_status_text("ListBox: item double-clicked", 0);
    });

    // CheckListBox
    let lbl2 = StaticText::new(&frame, "CheckListBox (per-item checkbox):");
    let clist = CheckListBox::new(&frame);
    for task in ["Read docs", "Write code", "Run tests", "Ship release"] {
        clist.append(task);
    }
    clist.check(0, true);
    clist.check(2, true);
    let clist_for_cb = clist.clone();
    let s = status.clone();
    clist.on_check_toggle(&frame, move |idx, checked| {
        let name = clist_for_cb.get_string(idx).unwrap_or_default();
        s.set_status_text(&format!("CheckListBox: {name} = {checked}"), 0);
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl1.as_widget_ref());
    sizer.add(list.as_widget_ref());
    sizer.add(lbl2.as_widget_ref());
    sizer.add(clist.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
