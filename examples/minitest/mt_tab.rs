//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Tab` — notebook with icons and real page content.
//!
//! Demonstrates:
//! - `ImageList` + `set_image_list` + `add_page_with_image` (SVG icons in tab strip)
//! - Pages with real, interactive content (ListBox, CheckBox, Slider, Gauge, TextCtrl)
//! - Nested sizers inside pages (`add_sizer` / `add_sizer_with_proportion`)
//! - `on_selection_change` → StatusBar (with `get_page_text`)
//! - Adding a page at runtime (`add_page_with_image` + `set_selection`)
//! - Renaming the current tab (`set_page_text`) and `get_page_count`
//!
//! Run with:
//! ```bash
//! cargo run --example mt_tab
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, CheckBox, Frame, Gauge, ImageList, ListBox, Panel, Slider, StaticText,
    StatusBar, Tab, TextCtrl,
};

// 16×16 SVG glyphs (rasterised by resvg), one colour per page.
const SVG_TASKS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#4FA464" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="m8 12 3 3 5-6"/></svg>"##;
const SVG_GEAR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9 17 7M7 17l-2.1 2.1"/></svg>"##;
const SVG_NOTE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#E8B339" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M9 13h6M9 17h4"/></svg>"##;
const SVG_STAR: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#9B59B6" stroke="#9B59B6" stroke-width="1"><path d="m12 2 3 7h7l-5.5 4.5L18.5 21 12 16.8 5.5 21l2-7.5L2 9h7z"/></svg>"##;

const IDX_TASKS: i32 = 0;
const IDX_GEAR: i32 = 1;
const IDX_NOTE: i32 = 2;
const IDX_STAR: i32 = 3;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — Tab + icons")
        .with_size(620, 460)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Switch tabs, then interact with each page.", 0);

    let notebook = Tab::new(&frame);

    // ── Image list with one SVG icon per page ────────────────────────
    let icons = ImageList::new(16, 16);
    icons.add_svg_bytes(SVG_TASKS);
    icons.add_svg_bytes(SVG_GEAR);
    icons.add_svg_bytes(SVG_NOTE);
    icons.add_svg_bytes(SVG_STAR);
    notebook.set_image_list(&icons);

    // ── Page 1 — Tasks (ListBox + add/remove row) ────────────────────
    let page1 = Panel::new(&frame);
    let lbl1 = StaticText::new(&page1, "Task list — add and remove items:");
    let tasks = ListBox::new(&page1);
    for t in ["Write the report", "Review the PR", "Ship v0.6.4"] {
        tasks.append(t);
    }
    let input = TextCtrl::new(&page1, "New task");
    let btn_add = Button::new(&page1, "Add");
    let btn_del = Button::new(&page1, "Remove selected");

    let tasks_add = tasks.clone();
    let input_add = input.clone();
    let s_add = status.clone();
    btn_add.on_click(&frame, move || {
        let text = input_add.get_value();
        if text.trim().is_empty() {
            s_add.set_status_text("Nothing to add — the input is empty.", 0);
            return;
        }
        tasks_add.append(text.trim());
        input_add.set_value("");
        s_add.set_status_text(&format!("Added task ({} total)", tasks_add.get_count()), 0);
    });
    let tasks_del = tasks.clone();
    let s_del = status.clone();
    btn_del.on_click(&frame, move || match tasks_del.get_selection() {
        Some(i) => {
            tasks_del.remove(i);
            s_del.set_status_text(&format!("Removed task ({} left)", tasks_del.get_count()), 0);
        }
        None => s_del.set_status_text("Select a task first.", 0),
    });

    // Nested horizontal sizer: input grows, buttons keep their size.
    let mut row1 = BoxSizer::horizontal();
    row1.add_with_proportion(input.as_widget_ref(), 1);
    row1.add(btn_add.as_widget_ref());
    row1.add(btn_del.as_widget_ref());
    let mut sz1 = BoxSizer::vertical();
    sz1.add(lbl1.as_widget_ref());
    sz1.add_with_proportion(tasks.as_widget_ref(), 1);
    sz1.add_sizer(row1);
    page1.set_sizer(sz1);

    // ── Page 2 — Options (CheckBoxes + Slider → Gauge) ───────────────
    let page2 = Panel::new(&frame);
    let lbl2 = StaticText::new(&page2, "Options — toggles and a linked slider/gauge:");
    let chk_autosave = CheckBox::new(&page2, "Enable autosave");
    let chk_dark = CheckBox::new(&page2, "Dark mode");
    chk_autosave.set_checked(true);
    let slider = Slider::new(&page2, 0, 100, 40);
    let gauge = Gauge::new(&page2, 100);
    gauge.set_value(40);

    let chk_a = chk_autosave.clone();
    let s_chk = status.clone();
    chk_autosave.on_toggle(&frame, move || {
        s_chk.set_status_text(
            &format!("Autosave: {}", if chk_a.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let chk_d = chk_dark.clone();
    let s_chk2 = status.clone();
    chk_dark.on_toggle(&frame, move || {
        s_chk2.set_status_text(
            &format!("Dark mode: {}", if chk_d.is_checked() { "ON" } else { "OFF" }),
            0,
        );
    });
    let slider_cb = slider.clone();
    let gauge_cb = gauge.clone();
    let s_sld = status.clone();
    slider.on_value_change(&frame, move || {
        let v = slider_cb.get_value();
        gauge_cb.set_value(v);
        s_sld.set_status_text(&format!("Quality: {v}%"), 0);
    });

    let mut sz2 = BoxSizer::vertical();
    sz2.add(lbl2.as_widget_ref());
    sz2.add(chk_autosave.as_widget_ref());
    sz2.add(chk_dark.as_widget_ref());
    sz2.add_spacer(8);
    sz2.add(slider.as_widget_ref());
    sz2.add(gauge.as_widget_ref());
    sz2.add_stretch(1);
    page2.set_sizer(sz2);

    // ── Page 3 — Notes (multiline TextCtrl + actions) ────────────────
    let page3 = Panel::new(&frame);
    let lbl3 = StaticText::new(&page3, "Notes — free-form multiline text:");
    let notes = TextCtrl::multiline(&page3, "Meeting notes:\r\n- icons everywhere\r\n");
    let btn_line = Button::new(&page3, "Append line");
    let btn_count = Button::new(&page3, "Word count");
    let btn_clear = Button::new(&page3, "Clear");

    let notes_app = notes.clone();
    btn_line.on_click(&frame, move || {
        notes_app.append_text("- another bullet point\r\n");
    });
    let notes_cnt = notes.clone();
    let s_cnt = status.clone();
    btn_count.on_click(&frame, move || {
        let words = notes_cnt.get_value().split_whitespace().count();
        s_cnt.set_status_text(&format!("Notes contain {words} word(s)"), 0);
    });
    let notes_clr = notes.clone();
    btn_clear.on_click(&frame, move || notes_clr.clear());

    let mut row3 = BoxSizer::horizontal();
    row3.add(btn_line.as_widget_ref());
    row3.add(btn_count.as_widget_ref());
    row3.add(btn_clear.as_widget_ref());
    let mut sz3 = BoxSizer::vertical();
    sz3.add(lbl3.as_widget_ref());
    sz3.add_with_proportion(notes.as_widget_ref(), 1);
    sz3.add_sizer(row3);
    page3.set_sizer(sz3);

    // ── Add the pages, each with its icon ────────────────────────────
    notebook.add_page_with_image("Tasks", &page1, IDX_TASKS);
    notebook.add_page_with_image("Options", &page2, IDX_GEAR);
    notebook.add_page_with_image("Notes", &page3, IDX_NOTE);
    status.set_status_text(&format!("{} pages", notebook.get_page_count()), 1);

    // ── Selection change → status bar with the page title ────────────
    let tab_sel = notebook.clone();
    let s_sel = status.clone();
    notebook.on_selection_change(&frame, move |idx| {
        let title = tab_sel.get_page_text(idx).unwrap_or_default();
        s_sel.set_status_text(&format!("Selected tab {idx}: {title}"), 0);
    });

    // ── Bottom bar: runtime page-add + rename current tab ────────────
    let btn_new_page = Button::new(&frame, "Add page at runtime");
    let btn_rename = Button::new(&frame, "Rename current tab (*)");

    let frame_c = frame.clone();
    let tab_c = notebook.clone();
    let status_c = status.clone();
    let counter = Rc::new(Cell::new(0u32));
    btn_new_page.on_click(&frame, move || {
        let n = counter.get() + 1;
        counter.set(n);
        let page = Panel::new(&frame_c);
        let lbl = StaticText::new(&page, &format!("Dynamic page #{n} — created at runtime"));
        let btn = Button::new(&page, "Click me");
        let s_dyn = status_c.clone();
        btn.on_click(&frame_c, move || {
            s_dyn.set_status_text(&format!("Button on dynamic page #{n} clicked"), 0)
        });
        let mut sz = BoxSizer::vertical();
        sz.add(lbl.as_widget_ref());
        sz.add(btn.as_widget_ref());
        sz.add_stretch(1);
        page.set_sizer(sz);
        let idx = tab_c.add_page_with_image(&format!("Extra {n}"), &page, IDX_STAR);
        if idx >= 0 {
            tab_c.set_selection(idx as usize);
        }
        status_c.set_status_text(&format!("{} pages", tab_c.get_page_count()), 1);
        status_c.set_status_text(&format!("Added page \"Extra {n}\""), 0);
    });

    let tab_r = notebook.clone();
    let s_ren = status.clone();
    btn_rename.on_click(&frame, move || {
        if let Some(idx) = tab_r.get_selection() {
            let old = tab_r.get_page_text(idx).unwrap_or_default();
            tab_r.set_page_text(idx, &format!("{old}*"));
            s_ren.set_status_text(&format!("Renamed page {idx} to \"{old}*\""), 0);
        }
    });

    // ── Frame layout: notebook on top, action row below ──────────────
    let mut actions = BoxSizer::horizontal();
    actions.add(btn_new_page.as_widget_ref());
    actions.add(btn_rename.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add_with_proportion(notebook.as_widget_ref(), 1);
    sizer.add_sizer(actions);
    frame.set_sizer(sizer);

    app.run(frame);
}
