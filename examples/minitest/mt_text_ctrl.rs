//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `TextCtrl` — single-line, multiline and password fields.
//!
//! Demonstrates:
//! - Live character counter via `on_change` in the `StatusBar`
//! - `set_max_length` on the single-line field
//! - Multiline editor with a read-only toggle (`set_readonly` / `is_readonly`)
//! - Copy / Paste buttons backed by the system `Clipboard`
//! - Undo button (`can_undo` / `undo`)
//! - Append-text and clear operations
//!
//! Run with:
//! ```bash
//! cargo run --example mt_text_ctrl
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{App, BoxSizer, Button, Clipboard, Frame, StaticText, StatusBar, TextCtrl, ToolTip};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TextCtrl")
        .with_size(580, 520)
        .with_modern_style().build();

    // Field 0 = messages, field 1 = live character counter.
    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Type into the fields.", 0);

    // 1. Single-line with live char counter and a 40-char limit.
    let lbl1 = StaticText::new(&frame, "Single-line (max 40 chars, live counter):");
    let single = TextCtrl::new(&frame, "Hello world");
    single.set_max_length(40);
    ToolTip::new("This field accepts at most 40 characters").attach(&single.as_widget_ref());
    {
        let single = single.clone();
        let s = status.clone();
        single.clone().on_change(&frame, move || {
            let v = single.get_value();
            s.set_status_text(&format!("{} / {} chars", v.chars().count(), single.max_length()), 1);
        });
    }
    status.set_status_text(
        &format!("{} / {} chars", single.get_value().chars().count(), single.max_length()),
        1,
    );

    // 2. Multiline editor.
    let lbl2 = StaticText::new(&frame, "Multiline editor:");
    let multi = TextCtrl::multiline(&frame, "First line\nSecond line\nThird line — type freely.");

    // 3. Password.
    let lbl3 = StaticText::new(&frame, "Password:");
    let pwd = TextCtrl::password(&frame, "");
    {
        let pwd_for_btn = pwd.clone();
        let s = status.clone();
        pwd.clone().on_change(&frame, move || {
            s.set_status_text(
                &format!("Password length: {}", pwd_for_btn.get_value().chars().count()),
                0,
            );
        });
    }

    // ── Row 1: editing operations on the multiline control ──────────
    let counter = Rc::new(Cell::new(0u32));
    let btn_append = Button::new(&frame, "Append line");
    {
        let multi = multi.clone();
        let counter = counter.clone();
        btn_append.on_click(&frame, move || {
            counter.set(counter.get() + 1);
            multi.append_text(&format!("\nappended #{}", counter.get()));
        });
    }

    let btn_clear = Button::new(&frame, "Clear");
    {
        let multi = multi.clone();
        let s = status.clone();
        btn_clear.on_click(&frame, move || {
            multi.clear();
            s.set_status_text("Multiline cleared", 0);
        });
    }

    let btn_undo = Button::new(&frame, "Undo");
    ToolTip::new("Undoes the last edit in the multiline control").attach(&btn_undo.as_widget_ref());
    {
        let multi = multi.clone();
        let s = status.clone();
        btn_undo.on_click(&frame, move || {
            if multi.can_undo() {
                multi.undo();
                s.set_status_text("Undone last edit", 0);
            } else {
                s.set_status_text("Nothing to undo", 0);
            }
        });
    }

    // Read-only toggle: flips the multiline editor and its own label.
    let btn_readonly = Button::new(&frame, "Lock (read-only)");
    {
        let multi = multi.clone();
        let btn = btn_readonly.clone();
        let s = status.clone();
        btn_readonly.on_click(&frame, move || {
            let lock = !multi.is_readonly();
            multi.set_readonly(lock);
            btn.set_label(if lock { "Unlock (editable)" } else { "Lock (read-only)" });
            s.set_status_text(
                if lock { "Multiline is now read-only" } else { "Multiline is editable again" },
                0,
            );
        });
    }

    // ── Row 2: clipboard interop ─────────────────────────────────────
    let btn_copy = Button::new(&frame, "Copy multiline");
    ToolTip::new("Copies the whole multiline text to the system clipboard")
        .attach(&btn_copy.as_widget_ref());
    {
        let multi = multi.clone();
        let s = status.clone();
        btn_copy.on_click(&frame, move || {
            let text = multi.get_value();
            if Clipboard::set_text(&text) {
                s.set_status_text(&format!("Copied {} chars to clipboard", text.chars().count()), 0);
            } else {
                s.set_status_text("Clipboard copy failed", 0);
            }
        });
    }

    let btn_paste = Button::new(&frame, "Paste into single-line");
    {
        let single = single.clone();
        let s = status.clone();
        btn_paste.on_click(&frame, move || match Clipboard::get_text() {
            Some(text) => {
                single.set_value(&text);
                s.set_status_text("Clipboard pasted into the single-line field", 0);
            }
            None => s.set_status_text("Clipboard has no text", 0),
        });
    }

    // Layout: two nested horizontal button rows under the editors.
    let mut row_edit = BoxSizer::horizontal();
    row_edit.add(btn_append.as_widget_ref());
    row_edit.add(btn_clear.as_widget_ref());
    row_edit.add(btn_undo.as_widget_ref());
    row_edit.add(btn_readonly.as_widget_ref());

    let mut row_clip = BoxSizer::horizontal();
    row_clip.add(btn_copy.as_widget_ref());
    row_clip.add(btn_paste.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl1.as_widget_ref());
    sizer.add(single.as_widget_ref());
    sizer.add(lbl2.as_widget_ref());
    sizer.add_with_proportion(multi.as_widget_ref(), 1);
    sizer.add_sizer(row_edit);
    sizer.add_sizer(row_clip);
    sizer.add(lbl3.as_widget_ref());
    sizer.add(pwd.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
