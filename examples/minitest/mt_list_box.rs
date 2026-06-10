//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `ListBox` and `CheckListBox` — list-style selection
//! controls, runtime mutation and list-to-list transfer.
//!
//! Demonstrates:
//! - `ListBox` selection / double-click callbacks with item text
//! - Double-click moves an item from the ListBox into the CheckListBox
//! - `CheckListBox` check toggles + "Check all" / "Uncheck all"
//! - Add / remove items at runtime, live counters in the status bar
//! - Nested horizontal button rows (`BoxSizer::add_sizer`)
//!
//! Run with:
//! ```bash
//! cargo run --example mt_list_box
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, CheckListBox, Frame, ListBox, StaticText, StatusBar};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ListBox / CheckListBox")
        .with_size(520, 560)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click items / toggle checks.", 0);

    // ── ListBox: backlog ─────────────────────────────────────────────
    let lbl1 = StaticText::new(&frame, "Backlog (double-click to move into the sprint):");
    let list = ListBox::new(&frame);
    for item in [
        "Fix login bug",
        "Polish settings UI",
        "Add dark theme",
        "Optimise startup",
        "Write user guide",
    ] {
        list.append(item);
    }

    // ── CheckListBox: sprint ─────────────────────────────────────────
    let lbl2 = StaticText::new(&frame, "Sprint (check = done):");
    let clist = CheckListBox::new(&frame);
    for task in ["Read docs", "Write code"] {
        clist.append(task);
    }
    clist.check(0, true);

    let update_counts = {
        let list = list.clone();
        let clist = clist.clone();
        let status = status.clone();
        move || {
            status.set_status_text(
                &format!(
                    "Backlog: {} — Sprint: {}",
                    list.get_count(),
                    clist.get_count()
                ),
                1,
            );
        }
    };
    update_counts();

    // Selection shows the item text, not just the index.
    let s = status.clone();
    let list_for_sel = list.clone();
    list.on_selection_change(&frame, move || {
        if let Some(idx) = list_for_sel.get_selection() {
            let text = list_for_sel.get_string(idx).unwrap_or_default();
            s.set_status_text(&format!("Backlog: {text}"), 0);
        }
    });

    // Double-click = move the item into the sprint list.
    let s = status.clone();
    let list_for_dbl = list.clone();
    let clist_for_dbl = clist.clone();
    let counts_for_dbl = update_counts.clone();
    list.on_double_click(&frame, move || {
        if let Some(idx) = list_for_dbl.get_selection() {
            if let Some(text) = list_for_dbl.get_string(idx) {
                list_for_dbl.remove(idx);
                clist_for_dbl.append(&text);
                s.set_status_text(&format!("Moved to sprint: {text}"), 0);
                counts_for_dbl();
            }
        }
    });

    // Check toggles report the task name and state.
    let clist_for_cb = clist.clone();
    let s = status.clone();
    clist.on_check_toggle(&frame, move |idx, checked| {
        let name = clist_for_cb.get_string(idx).unwrap_or_default();
        s.set_status_text(&format!("Sprint: {name} = {checked}"), 0);
    });

    // ── Backlog buttons ──────────────────────────────────────────────
    let btn_add = Button::new(&frame, "Add task");
    let list_for_add = list.clone();
    let counts_for_add = update_counts.clone();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0u32));
    btn_add.on_click(&frame, move || {
        counter.set(counter.get() + 1);
        list_for_add.append(&format!("New task #{}", counter.get()));
        counts_for_add();
    });

    let btn_remove = Button::new(&frame, "Remove selected");
    let list_for_rm = list.clone();
    let counts_for_rm = update_counts.clone();
    btn_remove.on_click(&frame, move || {
        if let Some(idx) = list_for_rm.get_selection() {
            list_for_rm.remove(idx);
            counts_for_rm();
        }
    });

    // ── Sprint buttons ───────────────────────────────────────────────
    let btn_all = Button::new(&frame, "Check all");
    let clist_for_all = clist.clone();
    btn_all.on_click(&frame, move || {
        for i in 0..clist_for_all.get_count() {
            clist_for_all.check(i, true);
        }
    });

    let btn_none = Button::new(&frame, "Uncheck all");
    let clist_for_none = clist.clone();
    btn_none.on_click(&frame, move || {
        for i in 0..clist_for_none.get_count() {
            clist_for_none.check(i, false);
        }
    });

    // ── Layout ───────────────────────────────────────────────────────
    let mut backlog_buttons = BoxSizer::horizontal();
    backlog_buttons.add(btn_add.as_widget_ref());
    backlog_buttons.add(btn_remove.as_widget_ref());

    let mut sprint_buttons = BoxSizer::horizontal();
    sprint_buttons.add(btn_all.as_widget_ref());
    sprint_buttons.add(btn_none.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl1.as_widget_ref());
    sizer.add_with_proportion(list.as_widget_ref(), 1);
    sizer.add_sizer(backlog_buttons);
    sizer.add(lbl2.as_widget_ref());
    sizer.add_with_proportion(clist.as_widget_ref(), 1);
    sizer.add_sizer(sprint_buttons);
    frame.set_sizer(sizer);

    app.run(frame);
}
