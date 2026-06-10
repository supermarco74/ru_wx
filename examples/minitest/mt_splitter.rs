//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `SplitterWindow` — master / detail browser with a live
//! draggable sash.
//!
//! Demonstrates:
//! - A vertical splitter whose two panes are real [`Panel`]s with
//!   their own sizers and content: a `ListBox` of topics on the left,
//!   a title (custom `Font`) + description on the right
//! - Selecting a topic in the list updates the right pane and the
//!   StatusBar (master → detail interaction)
//! - `on_sash_drag` reporting `DragStart` / `DragMove` / `DragEnd` in
//!   the StatusBar, with the panes reflowed at the end of the drag
//! - Buttons that place the sash at 25% / 50% / 75% of the splitter
//!   width via `set_sash_position` (+ `get_sash_position` round-trip)
//!
//! Run with:
//! ```bash
//! cargo run --example mt_splitter
//! ```

#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, Font, FontDesc, Frame, ListBox, Panel, SashEvent, SplitterWindow,
    StaticText, StatusBar,
};

const TOPICS: [(&str, &str); 5] = [
    ("Sizers", "BoxSizer stacks widgets vertically or horizontally; nested rows are added with add_sizer / add_sizer_with_proportion."),
    ("Splitter", "SplitterWindow owns two pane HWNDs and a draggable sash; on_sash_drag reports DragStart, DragMove and DragEnd."),
    ("Panels", "Panel forwards WM_COMMAND to its parent frame, so controls hosted on a pane keep their callbacks working."),
    ("Timers", "Timer::new(&frame) + on_tick + start(Duration) drives periodic UI updates without blocking the event loop."),
    ("Status bars", "StatusBar::new(&frame, n) creates n fields; set_status_text(text, i) updates one field at a time."),
];

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — SplitterWindow (master / detail)")
        .with_size(820, 520)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Pick a topic on the left, or drag the sash.", 0);
    status.set_status_text("sash: 280", 1);

    let splitter = SplitterWindow::new(&frame);

    // ── Left pane: topic list ────────────────────────────────────────────
    let left = Panel::new(&frame);
    left.set_size(280, 420);
    let lbl_topics = StaticText::new(&left, "Topics:");
    let list = ListBox::new(&left);
    for (title, _) in TOPICS.iter() {
        list.append(title);
    }
    list.set_selection(0);
    let mut left_sizer = BoxSizer::vertical();
    left_sizer.set_padding(4);
    left_sizer.add(lbl_topics.as_widget_ref());
    left_sizer.add_with_proportion(list.as_widget_ref(), 1);
    left.set_sizer(left_sizer);

    // ── Right pane: detail view ──────────────────────────────────────────
    let right = Panel::new(&frame);
    right.set_size(520, 420);
    let detail_title = StaticText::new(&right, TOPICS[0].0);
    let title_font = Font::new(FontDesc::new("Segoe UI", 15).bold());
    detail_title.set_font(&title_font);
    let detail_body = StaticText::new(&right, TOPICS[0].1);
    let mut right_sizer = BoxSizer::vertical();
    right_sizer.set_padding(6);
    right_sizer.add(detail_title.as_widget_ref());
    right_sizer.add_with_proportion(detail_body.as_widget_ref(), 1);
    right.set_sizer(right_sizer);

    // ── Master → detail wiring ───────────────────────────────────────────
    let list_cb = list.clone();
    let dt = detail_title.clone();
    let db = detail_body.clone();
    let s = status.clone();
    list.on_selection_change(&frame, move || {
        if let Some(i) = list_cb.get_selection() {
            if let Some((title, body)) = TOPICS.get(i) {
                dt.set_label(title);
                db.set_label(body);
                s.set_status_text(&format!("Topic {} of {}: {title}", i + 1, TOPICS.len()), 0);
            }
        }
    });

    // ── Split + sash position ────────────────────────────────────────────
    #[cfg(target_os = "windows")]
    {
        splitter.split_vertically(left.hwnd(), right.hwnd());
        splitter.set_sash_position(280);
    }

    // Shared routine: move the sash to `pct` percent of the splitter
    // width (or pass the current position with pct < 0) and reflow the
    // two pane panels so their sizers track the new pane sizes.
    let place: Rc<dyn Fn(i32)> = {
        let splitter = splitter.clone();
        let left = left.clone();
        let right = right.clone();
        let status = status.clone();
        Rc::new(move |pct: i32| {
            let rect = splitter.as_widget_ref().borrow().rect();
            let w = rect.width as i32;
            let h = rect.height;
            if pct >= 0 {
                #[cfg(target_os = "windows")]
                splitter.set_sash_position(w * pct / 100);
            }
            let pos = splitter.get_sash_position();
            left.set_position(rect.x, rect.y);
            left.set_size(pos.max(0) as u32, h);
            right.set_position(rect.x + pos + 1, rect.y);
            right.set_size((w - pos - 1).max(0) as u32, h);
            status.set_status_text(&format!("sash: {pos}"), 1);
        })
    };

    // ── Sash drag feedback ───────────────────────────────────────────────
    #[cfg(target_os = "windows")]
    {
        let s = status.clone();
        let place_cb = place.clone();
        splitter.on_sash_drag(move |ev: SashEvent| match ev {
            SashEvent::DragStart => s.set_status_text("Sash drag started…", 0),
            SashEvent::DragMove { position } => {
                s.set_status_text(&format!("Dragging sash at x = {position}"), 0);
                s.set_status_text(&format!("sash: {position}"), 1);
            }
            SashEvent::DragEnd { position } => {
                s.set_status_text(&format!("Sash released at x = {position}"), 0);
                place_cb(-1); // reflow panes at the final position
            }
        });
    }

    // ── Sash placement buttons ───────────────────────────────────────────
    let make_place_btn = |label: &str, pct: i32| -> Button {
        let btn = Button::new(&frame, label);
        let place = place.clone();
        let s = status.clone();
        let label = label.to_string();
        btn.on_click(&frame, move || {
            place(pct);
            s.set_status_text(&format!("Sash moved to {label}"), 0);
        });
        btn
    };
    let btn_25 = make_place_btn("Sash 25%", 25);
    let btn_50 = make_place_btn("Sash 50%", 50);
    let btn_75 = make_place_btn("Sash 75%", 75);

    // ── Layout ───────────────────────────────────────────────────────────
    let mut buttons_row = BoxSizer::horizontal();
    buttons_row.add(btn_25.as_widget_ref());
    buttons_row.add(btn_50.as_widget_ref());
    buttons_row.add(btn_75.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add_with_proportion(splitter.as_widget_ref(), 1);
    sizer.add_sizer(buttons_row);
    frame.set_sizer(sizer);

    // Initial reflow at the configured sash position.
    place(-1);

    app.run(frame);
}
