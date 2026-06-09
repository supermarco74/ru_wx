//! Demo: a comprehensive showcase of input controls in `ru_wx`,
//! organised into a tabbed notebook.
//!
//! This example demonstrates every input-style widget exposed by the library,
//! laid out across four `Tab` pages, each backed by its own `Panel`.
//!
//! Demonstrates:
//! - `TextCtrl` in three modes: single-line, password, multi-line
//! - `CheckBox` controls with `on_toggle` callbacks
//! - `RadioButton` group (using `is_group_start = true` on the first button)
//! - `ComboBox` (editable dropdown) with `on_selection_change`
//! - `ListBox` in single-select and multi-select modes
//! - `ListCtrl` in `Report` view with multiple columns
//! - The `Tab` / `Panel` split: each page hosts its own sizer and the
//!   page contents are forwarded to the frame's `WM_COMMAND` dispatcher
//!   via the panel's WndProc
//! - Aggregating the state of every control and showing it via a `message_box`
//! - Resetting all controls to their default values
//!
//! Run with:
//! ```bash
//! cargo run --example input_controls_demo
//! ```

// #![windows_subsystem = "windows"] // disabled for debugging

use std::io::Write;

use ru_wx::{
    message_box, App, BoxSizer, Button, CheckBox, ComboBox, Frame, ListBox, ListCtrl,
    ListCtrlStyle, MessageBoxIcon, MessageBoxStyle, Panel, RadioButton, StaticText, Tab, TextCtrl,
};

macro_rules! step {
    ($($arg:tt)*) => {{
        eprintln!("[demo] {}", format_args!($($arg)*));
        let _ = std::io::stderr().flush();
    }};
}

fn main() {
    step!("start");
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx — Input Controls Demo (Tabbed)")
        .with_size(680, 720)
        .build();
    step!("frame created, hwnd={:?}", frame.hwnd());

    // ── Build the tab notebook ───────────────────────────────────────────
    // Each page is a `Panel` that owns its own vertical sizer. The page's
    // widgets are parented to the page's panel (works thanks to the
    // generic `W: Window` constructors). When the frame is resized, the
    // frame's sizer resizes the tab, and the tab re-layouts every page
    // to fit the tab control's content area.
    let tab = Tab::new(&frame);
    step!("tab created");

    // ── Page 1: "Text" ──────────────────────────────────────────────────
    let page_text = Panel::new(&frame);
    step!("page_text created");
    let name_input = TextCtrl::new(&page_text, "Your name");
    step!("name_input created");
    let pwd_input = TextCtrl::password(&page_text, "");
    step!("pwd_input created");
    let notes_input = TextCtrl::multiline(&page_text, "Multi-line notes…");
    step!("notes_input created");
    // Pin the multi-line input's height to a small fixed value so it
    // doesn't take over the whole page.
    notes_input.as_widget_ref().borrow_mut().set_size(0, 35);
    step!("notes_input sized");

    let lbl_text = StaticText::new(&page_text, "── Text inputs ──");
    step!("lbl_text created");
    let mut s_text = BoxSizer::vertical();
    s_text.set_padding(4);
    s_text.add(lbl_text.as_widget_ref());
    s_text.add(name_input.as_widget_ref());
    s_text.add(pwd_input.as_widget_ref());
    s_text.add(notes_input.as_widget_ref());
    page_text.set_sizer(s_text);
    step!("page_text sizer set");
    tab.add_page("Text", &page_text);
    step!("page_text added to tab");

    // ── Page 2: "Choices" ───────────────────────────────────────────────
    let page_choices = Panel::new(&frame);
    step!("page_choices created");
    let cb_newsletter = CheckBox::new(&page_choices, "Subscribe to newsletter");
    step!("cb_newsletter created");
    let cb_terms = CheckBox::new(&page_choices, "I accept the terms");
    let cb_marketing = CheckBox::new(&page_choices, "Send me marketing emails");
    let cb_dark_mode = CheckBox::new(&page_choices, "Enable dark mode");
    cb_dark_mode.set_checked(true);

    let rb_free = RadioButton::new(&page_choices, "Free", true);
    let rb_pro = RadioButton::new(&page_choices, "Pro", false);
    let rb_enterprise = RadioButton::new(&page_choices, "Enterprise", false);
    rb_pro.set_selected(true);

    let combo = ComboBox::new(&page_choices);
    combo.append("Italy");
    combo.append("France");
    combo.append("Germany");
    combo.append("Spain");
    combo.append("United Kingdom");
    combo.append("United States");
    combo.append("Japan");
    combo.set_selection(0);
    step!("page_choices widgets created");

    let lbl_cb = StaticText::new(&page_choices, "── CheckBoxes ──");
    let lbl_rb = StaticText::new(&page_choices, "── RadioButtons (pick a plan) ──");
    let lbl_combo = StaticText::new(&page_choices, "── ComboBox (country) ──");
    let mut s_choices = BoxSizer::vertical();
    s_choices.set_padding(4);
    s_choices.add(lbl_cb.as_widget_ref());
    s_choices.add(cb_newsletter.as_widget_ref());
    s_choices.add(cb_terms.as_widget_ref());
    s_choices.add(cb_marketing.as_widget_ref());
    s_choices.add(cb_dark_mode.as_widget_ref());
    s_choices.add(lbl_rb.as_widget_ref());
    s_choices.add(rb_free.as_widget_ref());
    s_choices.add(rb_pro.as_widget_ref());
    s_choices.add(rb_enterprise.as_widget_ref());
    s_choices.add(lbl_combo.as_widget_ref());
    s_choices.add(combo.as_widget_ref());
    page_choices.set_sizer(s_choices);
    step!("page_choices sizer set");
    tab.add_page("Choices", &page_choices);
    step!("page_choices added to tab");

    // ── Page 3: "Lists" ─────────────────────────────────────────────────
    let page_lists = Panel::new(&frame);
    step!("page_lists created");
    let listbox = ListBox::new(&page_lists);
    listbox.append("Rome");
    listbox.append("Milan");
    listbox.append("Naples");
    listbox.append("Turin");
    listbox.append("Florence");
    listbox.set_selection(0);
    listbox.as_widget_ref().borrow_mut().set_size(0, 45);

    let listbox_multi = ListBox::multi_select(&page_lists);
    listbox_multi.append("Reading");
    listbox_multi.append("Gaming");
    listbox_multi.append("Cooking");
    listbox_multi.append("Hiking");
    listbox_multi.as_widget_ref().borrow_mut().set_size(0, 30);

    let listctrl = ListCtrl::new(&page_lists, ListCtrlStyle::Report);
    listctrl.insert_column(0, "ID", 40);
    listctrl.insert_column(1, "Name", 120);
    listctrl.insert_column(2, "City", 100);
    listctrl.insert_column(3, "Role", 120);

    let people: [(u32, &str, &str, &str); 3] = [
        (1, "Alice Rossi", "Rome", "Engineer"),
        (2, "Bruno Bianchi", "Milan", "Designer"),
        (3, "Carla Verdi", "Naples", "Manager"),
    ];
    for (i, (id, name, city, role)) in people.iter().enumerate() {
        let row = listctrl.insert_item(i, &id.to_string());
        listctrl.set_item_text(row, 1, name);
        listctrl.set_item_text(row, 2, city);
        listctrl.set_item_text(row, 3, role);
    }
    listctrl.as_widget_ref().borrow_mut().set_size(0, 40);
    step!("page_lists widgets created");

    let lbl_lb = StaticText::new(&page_lists, "── ListBox (single-select city) ──");
    let lbl_lbm = StaticText::new(&page_lists, "── ListBox (multi-select hobbies) ──");
    let lbl_lc = StaticText::new(&page_lists, "── ListCtrl (Report view) ──");
    let mut s_lists = BoxSizer::vertical();
    s_lists.set_padding(4);
    s_lists.add(lbl_lb.as_widget_ref());
    s_lists.add(listbox.as_widget_ref());
    s_lists.add(lbl_lbm.as_widget_ref());
    s_lists.add(listbox_multi.as_widget_ref());
    s_lists.add(lbl_lc.as_widget_ref());
    s_lists.add(listctrl.as_widget_ref());
    page_lists.set_sizer(s_lists);
    step!("page_lists sizer set");
    tab.add_page("Lists", &page_lists);
    step!("page_lists added to tab");

    // ── Page 4: "Actions" ───────────────────────────────────────────────
    let page_actions = Panel::new(&frame);
    step!("page_actions created");
    let summary_btn = Button::new(&page_actions, "Show Summary");
    let clear_btn = Button::new(&page_actions, "Clear All");
    let mut s_actions = BoxSizer::vertical();
    s_actions.set_padding(8);
    s_actions.add(summary_btn.as_widget_ref());
    s_actions.add(clear_btn.as_widget_ref());
    page_actions.set_sizer(s_actions);
    step!("page_actions sizer set");
    tab.add_page("Actions", &page_actions);
    step!("page_actions added to tab");

    // ── Status label (frame-level, always visible) ──────────────────────
    let status_label = StaticText::new(&frame, "Status: ready.");
    step!("status_label created");

    // ── Build the frame sizer: tab (proportion 1) on top, status at the bottom
    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(4);
    sizer.add_with_proportion(tab.as_widget_ref(), 1);
    sizer.add(status_label.as_widget_ref());

    frame.set_sizer(sizer);
    step!("frame.set_sizer done");

    // ── Callbacks (all registered on the frame — child events are
    //    forwarded up from the page panels) ─────────────────────────────

    // Checkbox toggles — update the status label
    let s_for_nl = status_label.clone();
    let nl_for_cb = cb_newsletter.clone();
    cb_newsletter.on_toggle(&frame, move || {
        s_for_nl.set_label(&format!(
            "Newsletter: {}",
            if nl_for_cb.is_checked() { "ON" } else { "OFF" }
        ));
    });

    let s_for_terms = status_label.clone();
    let terms_for_cb = cb_terms.clone();
    cb_terms.on_toggle(&frame, move || {
        s_for_terms.set_label(&format!(
            "Terms accepted: {}",
            if terms_for_cb.is_checked() {
                "yes"
            } else {
                "no"
            }
        ));
    });

    let s_for_mkt = status_label.clone();
    let mkt_for_cb = cb_marketing.clone();
    cb_marketing.on_toggle(&frame, move || {
        s_for_mkt.set_label(&format!(
            "Marketing: {}",
            if mkt_for_cb.is_checked() { "ON" } else { "OFF" }
        ));
    });

    let s_for_dark = status_label.clone();
    let dark_for_cb = cb_dark_mode.clone();
    cb_dark_mode.on_toggle(&frame, move || {
        s_for_dark.set_label(&format!(
            "Dark mode: {}",
            if dark_for_cb.is_checked() {
                "ON"
            } else {
                "OFF"
            }
        ));
    });

    // Radio button group — the currently selected one updates the status label
    let s_for_free = status_label.clone();
    let free_for_rb = rb_free.clone();
    rb_free.on_select(&frame, move || {
        if free_for_rb.is_selected() {
            s_for_free.set_label("Plan: Free");
        }
    });

    let s_for_pro = status_label.clone();
    let pro_for_rb = rb_pro.clone();
    rb_pro.on_select(&frame, move || {
        if pro_for_rb.is_selected() {
            s_for_pro.set_label("Plan: Pro");
        }
    });

    let s_for_ent = status_label.clone();
    let ent_for_rb = rb_enterprise.clone();
    rb_enterprise.on_select(&frame, move || {
        if ent_for_rb.is_selected() {
            s_for_ent.set_label("Plan: Enterprise");
        }
    });

    // ComboBox selection change
    let s_for_combo = status_label.clone();
    let combo_for_cb = combo.clone();
    combo.on_selection_change(&frame, move || {
        let text = combo_for_cb.get_value();
        s_for_combo.set_label(&format!("Country: \"{text}\""));
    });

    // ListBox single-select
    let s_for_lb = status_label.clone();
    let lb_for_cb = listbox.clone();
    listbox.on_selection_change(&frame, move || {
        if let Some(idx) = lb_for_cb.get_selection() {
            if let Some(text) = lb_for_cb.get_string(idx) {
                s_for_lb.set_label(&format!("City: {text} (index {idx})"));
            }
        }
    });

    // ListBox multi-select
    let s_for_lbm = status_label.clone();
    let lbm_for_cb = listbox_multi.clone();
    listbox_multi.on_selection_change(&frame, move || {
        let sel = lbm_for_cb.get_selections();
        s_for_lbm.set_label(&format!("Hobbies selected: {} item(s)", sel.len()));
    });

    // Live-update the status label as the user types the name
    let s_for_name = status_label.clone();
    let name_for_status = name_input.clone();
    name_input.on_change(&frame, move || {
        s_for_name.set_label(&format!("Name: \"{}\"", name_for_status.get_value()));
    });

    // ── "Show Summary" button — read every control and show a MessageBox ──
    let frame_for_summary = frame.clone();
    let name_for_sum = name_input.clone();
    let pwd_for_sum = pwd_input.clone();
    let notes_for_sum = notes_input.clone();
    let nl_for_sum = cb_newsletter.clone();
    let terms_for_sum = cb_terms.clone();
    let mkt_for_sum = cb_marketing.clone();
    let dark_for_sum = cb_dark_mode.clone();
    let free_for_sum = rb_free.clone();
    let pro_for_sum = rb_pro.clone();
    let ent_for_sum = rb_enterprise.clone();
    let combo_for_sum = combo.clone();
    let lb_for_sum = listbox.clone();
    let lbm_for_sum = listbox_multi.clone();
    let lc_for_sum = listctrl.clone();
    let s_for_summary = status_label.clone();
    summary_btn.on_click(&frame, move || {
        // Build the summary text
        let name = name_for_sum.get_value();
        let pwd_len = pwd_for_sum.get_value().len();
        let notes = notes_for_sum.get_value();
        let notes_summary = if notes.len() > 60 {
            format!("{}…", &notes[..60])
        } else {
            notes.clone()
        };
        let notes_first_line = notes.lines().next().unwrap_or("").to_string();

        let plan = if free_for_sum.is_selected() {
            "Free"
        } else if pro_for_sum.is_selected() {
            "Pro"
        } else if ent_for_sum.is_selected() {
            "Enterprise"
        } else {
            "(none)"
        };

        let country = combo_for_sum.get_value();

        let city = lb_for_sum
            .get_selection()
            .and_then(|i| lb_for_sum.get_string(i))
            .unwrap_or_else(|| "(none)".to_string());

        let hobbies = lbm_for_sum.get_selections();
        let hobby_names: Vec<String> = hobbies
            .iter()
            .filter_map(|&i| lbm_for_sum.get_string(i))
            .collect();
        let hobby_display = if hobby_names.is_empty() {
            "(none)".to_string()
        } else {
            hobby_names.join(", ")
        };

        let lc_selection = lc_for_sum.get_selected_item();
        let lc_text = if let Some(row) = lc_selection {
            let id = lc_for_sum.get_item_text(row, 0);
            let name = lc_for_sum.get_item_text(row, 1);
            let city = lc_for_sum.get_item_text(row, 2);
            let role = lc_for_sum.get_item_text(row, 3);
            format!("Row {row}: #{id} — {name} ({city}, {role})")
        } else {
            "(no row selected)".to_string()
        };

        let summary = format!(
            "Input Controls Summary\n\
             \n\
             Name:       {name}\n\
             Password:   {pwd_len} char(s)\n\
             Notes:      {notes_summary}\n\
             First line: {notes_first_line}\n\
             \n\
             CheckBoxes:\n\
             • Newsletter: {}\n\
             • Terms:      {}\n\
             • Marketing:  {}\n\
             • Dark mode:  {}\n\
             \n\
             Plan:        {plan}\n\
             Country:     {country}\n\
             City:        {city}\n\
             Hobbies:     {hobby_display}\n\
             \n\
             ListCtrl:    {lc_text}",
            if nl_for_sum.is_checked() {
                "✓"
            } else {
                "✗"
            },
            if terms_for_sum.is_checked() {
                "✓"
            } else {
                "✗"
            },
            if mkt_for_sum.is_checked() {
                "✓"
            } else {
                "✗"
            },
            if dark_for_sum.is_checked() {
                "✓"
            } else {
                "✗"
            },
        );

        s_for_summary.set_label("Status: summary shown.");
        message_box(
            &frame_for_summary,
            &summary,
            "ru_wx — Input Summary",
            MessageBoxStyle::Ok,
            MessageBoxIcon::Information,
        );
    });

    // ── "Clear All" button — reset every control ─────────────────────────
    let name_for_clear = name_input.clone();
    let pwd_for_clear = pwd_input.clone();
    let notes_for_clear = notes_input.clone();
    let nl_for_clear = cb_newsletter.clone();
    let terms_for_clear = cb_terms.clone();
    let mkt_for_clear = cb_marketing.clone();
    let dark_for_clear = cb_dark_mode.clone();
    let free_for_clear = rb_free.clone();
    let pro_for_clear = rb_pro.clone();
    let ent_for_clear = rb_enterprise.clone();
    let combo_for_clear = combo.clone();
    let lb_for_clear = listbox.clone();
    // (Multi-select ListBox has no clear-selection API in the current build.)
    let lc_for_clear = listctrl.clone();
    let s_for_clear = status_label.clone();
    clear_btn.on_click(&frame, move || {
        // Text controls
        name_for_clear.set_value("");
        pwd_for_clear.set_value("");
        notes_for_clear.set_value("");

        // Checkboxes
        nl_for_clear.set_checked(false);
        terms_for_clear.set_checked(false);
        mkt_for_clear.set_checked(false);
        dark_for_clear.set_checked(false);

        // Radio buttons: select "Free"
        free_for_clear.set_selected(true);
        pro_for_clear.set_selected(false);
        ent_for_clear.set_selected(false);

        // ComboBox
        combo_for_clear.set_selection(0);

        // ListBox single
        lb_for_clear.set_selection(0);

        // ListBox multi — clear selections by re-selecting index 0 (no API
        // exists to clear all selections directly).
        // (No-op for the demo — multi-select state is left as-is.)

        // ListCtrl
        lc_for_clear.delete_all_items();

        s_for_clear.set_label("Status: cleared.");
    });
    step!("all callbacks registered");

    // ── Run the event loop ───────────────────────────────────────────────
    step!("about to run event loop");
    app.run(frame);
    step!("event loop returned");
}
