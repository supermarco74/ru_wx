//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Demo: showcase all 20 controls ported from `MIGRATION_STATUS.md`,
//! the per-widget `wxToolTip` port, plus the v0.4.0 HiDPI and
//! v0.4.1 accelerator APIs.
//!
//! Demonstrates:
//! - 1.  `wxSlider`             — continuous value input
//! - 2.  `wxGauge`              — determinate + indeterminate progress
//! - 3.  `wxSpinCtrl`           — numeric stepper
//! - 4.  `wxChoice`             — simple drop-down (no edit)
//! - 5.  `wxCheckListBox`       — list of items with per-item checkboxes
//! - 6.  `wxDatePickerCtrl`     — calendar popup date chooser
//! - 7.  `wxColourPickerCtrl`   — colour chooser button
//! - 8.  `wxRadioBox`           — group of radio buttons in a box
//! - 9.  `wxStatusBar`          — 1..N field status bar at the bottom
//! - 10. `wxToolBar`            — icon toolbar with separators
//! - 11. `wxNotebook`           — tab control that uses `wxImageList` icons
//! - 12. `wxTimer`              — repeating / one-shot timer with `on_tick`
//! - 13. `wxFont`               — custom font (face, size, weight)
//! - 14. `wxMessageDialog`      — modal message dialog (About box, etc.)
//! - 15. `wxBitmapBundle`       — multi-resolution bitmap (HiDPI toolbar icons)
//! - 16. `wxArtProvider`        — system-icon provider (`ArtId::New`, `ArtId::Cut`, ...)
//! - 17. `wxPopupMenu`          — on-demand popup (different from `Menu`)
//! - 18. `wxMenuItem` check/radio — checkable / radio menu items
//! - 19. `wxTopLevelWindow` base — a more complete window base than `Frame`
//! - 20. `wxToolTip`            — per-widget hover tooltips + global enable
//! - 21. HiDPI (`Frame::dpi` + `Frame::scale_factor`) — live read-out in the status bar; the app's manifest declares `PerMonitorV2` awareness so the value follows the monitor when the window is dragged.
//! - 22. Keyboard accelerators (`Accelerator` + `Menu::append_with_shortcut`) — the **File** menu items carry `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Q` shortcuts; the `Ctrl+P` "Print preview" item is dimmed to demonstrate `append_disabled_with_shortcut`.
//!
//! Run with:
//! ```bash
//! cargo run --example showcase_all
//! ```

#![windows_subsystem = "windows"]

use std::time::Duration;

use ru_wx::{
    Accelerator, AnyButton, App, ArtClient, ArtId, ArtProvider, AuiToolBar, BitmapBundle,
    BoxSizer, Button, ButtonVariants, CentreDirection, CheckListBox, Choice, Colour,
    ColourPickerCtrl, DatePickerCtrl, Font, FontDesc, Frame, Gauge, ImageList, ListCtrl,
    ListCtrlStyle, MessageBoxIcon, MessageDialog, MessageDialogStyle, Orientation, Panel,
    PopupMenu, RadioBox, ScrollablePanel, Slider, SpinCtrl, StaticText, StatusBar, Tab, Timer,
    ToolTip, TopLevelWindow, UserAttentionFlags,
};

// Colorful inline SVG icons (24×24 viewBox) with filled backgrounds so
// they stand out on the toolbar surface.
const ICON_NEW: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#4F46E5"/><path d="M14 6H8a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V10z" fill="none" stroke="white" stroke-width="1.6"/><path d="M14 6v4h4 M11 16h2 M12 13v5" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg>"##;
const ICON_OPEN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#10B981"/><path d="M3 10a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#F59E0B"/><path d="M6 4h10l3 3v13H5z M9 4v5h6V4 M8 13h8v7H8z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_CUT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#EF4444"/><circle cx="7" cy="8" r="2.2" fill="white"/><circle cx="7" cy="16" r="2.2" fill="white"/><path d="M9 9.5L20 20 M9 14.5L20 4" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
const ICON_COPY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#8B5CF6"/><rect x="9" y="9" width="11" height="11" rx="1.5" fill="none" stroke="white" stroke-width="1.6"/><path d="M15 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h3" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;

// Toolbar user-chosen identifiers (we dispatch on these in the click
// handler below).
const ID_TOOL_NEW: u16 = 1001;
const ID_TOOL_OPEN: u16 = 1002;
const ID_TOOL_SAVE: u16 = 1003;
const ID_TOOL_CUT: u16 = 1004;
const ID_TOOL_COPY: u16 = 1005;

fn main() {
    let app = App::new();

    // 19. TopLevelWindow — a richer window base than the bare Frame.
    let window = TopLevelWindow::new("ru_wx showcase — AuiToolBar + Buttons + scroll", 900, 700);

    // Centre the window on the screen before showing it.
    window.centre(CentreDirection::Screen);

    // ---- Status bar (9) — 3 fields at the bottom of the frame ----
    // Field 1 is wired to a live read-out of the frame's DPI / scale
    // factor (21. HiDPI). PerMonitorV2 awareness means the value
    // changes automatically when the window is dragged to a
    // different monitor.
    let status = StatusBar::new(window.frame(), 3);
    status.set_status_text(
        "Nuovo: AuiToolBar 40px | tab Buttons | pagine scrollabili — clicca ≡ per staccare la barra",
        0,
    );
    {
        let dpi = window.frame().dpi();
        let scale = dpi.scale_factor();
        status.set_status_text(&format!("DPI: {} ({:.2}x)", dpi, scale), 1);
    }
    status.set_status_text("Field 3", 2);

    // ---- AuiToolBar (10) — dockable / floating bar with large
    // colourful icons. 15. BitmapBundle rasterises the SVGs at 32,
    // 40 and 48 px for HiDPI.
    let icon_sizes: [(u32, u32); 3] = [(32, 32), (40, 40), (48, 48)];

    let bundle_new = BitmapBundle::from_svg_bytes(ICON_NEW, &icon_sizes);
    let bundle_open = BitmapBundle::from_svg_bytes(ICON_OPEN, &icon_sizes);
    let bundle_save = BitmapBundle::from_svg_bytes(ICON_SAVE, &icon_sizes);
    let bundle_cut = BitmapBundle::from_svg_bytes(ICON_CUT, &icon_sizes);
    let bundle_copy = BitmapBundle::from_svg_bytes(ICON_COPY, &icon_sizes);

    let toolbar_images = ImageList::new(40, 40);
    if let Some(bmp) = bundle_new.best_for_size((40, 40)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_open.best_for_size((40, 40)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_save.best_for_size((40, 40)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_cut.best_for_size((40, 40)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_copy.best_for_size((40, 40)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }

    let aui = AuiToolBar::new(window.frame());
    aui.set_toolbar_height(52);
    aui.set_image_list(&toolbar_images);
    aui.add_tool(ID_TOOL_NEW, "New document", 0);
    aui.add_tool(ID_TOOL_OPEN, "Open file…", 1);
    aui.add_tool(ID_TOOL_SAVE, "Save document", 2);
    aui.add_separator();
    aui.add_tool(ID_TOOL_CUT, "Cut selection", 3);
    aui.add_tool(ID_TOOL_COPY, "Copy selection", 4);
    aui.realize();
    let toolbar_reserved = aui.reserved_height();

    // Wire up the toolbar's click events. The single callback fires
    // for every tool with the tool's id. We clone the window so the
    // callback owns its captured value (the callback must be
    // `'static`).
    let status_for_tools = status.clone();
    let window_for_tools = window.clone();
    aui.on_tool_clicked(window.frame(), move |id| {
        let label = match id {
            ID_TOOL_NEW => "New",
            ID_TOOL_OPEN => "Open",
            ID_TOOL_SAVE => "Save",
            ID_TOOL_CUT => "Cut",
            ID_TOOL_COPY => "Copy",
            _ => "?",
        };
        status_for_tools.set_status_text(&format!("Tool: {label}"), 0);
        if id == ID_TOOL_NEW {
            // Demonstrate one of the TopLevelWindow-only methods: flash
            // the taskbar button to request the user's attention.
            window_for_tools.request_user_attention(UserAttentionFlags::Default);
        }
    });

    // 16. ArtProvider — register some overrides so other code can ask
    // for an icon by id and get a nice fallback. We just construct one
    // here; the wrappers expose `get_bitmap(ArtId::New, ArtClient::Menu)`
    // etc. for callers that need it.
    let _art = ArtProvider::new();
    // Reference some symbols from the public API so they are not
    // reported as unused imports in the example.
    let _ = (ArtClient::Menu, ArtId::New);

    // 11. Notebook / Tab control with image list ----
    let notebook = Tab::new(window.frame());
    notebook.set_image_list(&toolbar_images);

    // ===== Page 1: lists & selections (scrollable) =====
    let page1 = Panel::new(window.frame());
    let scroll1 = ScrollablePanel::install(&page1);
    let content1 = scroll1.content();
    let lbl1 = StaticText::new(&content1, "Lists and selection controls:");

    // 4. Choice — simple drop-down with no edit
    let choice = Choice::new(&content1);
    choice.append("Apples");
    choice.append("Oranges");
    choice.append("Bananas");
    choice.append("Grapes");
    choice.append("Pineapples");
    choice.set_selection(0);
    // `choice` is moved into the closure, so keep a clone for the
    // sizer below.
    let choice_for_cb = choice.clone();
    let status_for_choice = status.clone();
    {
        let cb = choice_for_cb.clone();
        let s = status_for_choice.clone();
        choice_for_cb.on_selection_change(window.frame(), move || {
            if let Some(i) = cb.get_selection() {
                if let Some(s_str) = cb.get_string(i) {
                    s.set_status_text(&format!("Choice: {s_str}"), 1);
                }
            }
        });
    }

    // 5. CheckListBox — list with per-item checkboxes
    let checklist = CheckListBox::new(&content1);
    checklist.append("Read documentation");
    checklist.append("Write example");
    checklist.append("Run tests");
    checklist.append("Build release");
    checklist.check(0, true);
    checklist.check(2, true);
    let checklist_for_cb = checklist.clone();
    let status_for_clb = status.clone();
    {
        let cb = checklist_for_cb.clone();
        let s = status_for_clb.clone();
        checklist_for_cb.on_check_toggle(window.frame(), move |idx, checked| {
            if let Some(name) = cb.get_string(idx) {
                s.set_status_text(&format!("CheckListBox: {name} = {checked}"), 2);
            }
        });
    }

    // 8. RadioBox — a labelled group of radio buttons
    let radio = RadioBox::new(&content1, "Priority", &["Low", "Normal", "High", "Urgent"]);
    radio.set_selection(1);
    let status_for_radio = status.clone();
    radio.on_select(window.frame(), move |idx| {
        let label = ["Low", "Normal", "High", "Urgent"]
            .get(idx)
            .copied()
            .unwrap_or("?");
        status_for_radio.set_status_text(&format!("Priority: {label}"), 0);
    });

    lbl1.as_widget_ref().borrow_mut().set_size(440, 22);
    choice.as_widget_ref().borrow_mut().set_size(440, 28);
    checklist
        .as_widget_ref()
        .borrow_mut()
        .set_size(440, 140);
    radio.as_widget_ref().borrow_mut().set_size(440, 120);

    let mut sizer1 = BoxSizer::vertical();
    sizer1.set_padding(8);
    sizer1.add(lbl1.as_widget_ref());
    sizer1.add_spacer(4);
    sizer1.add(choice.as_widget_ref());
    sizer1.add_spacer(8);
    sizer1.add(checklist.as_widget_ref());
    sizer1.add_spacer(8);
    sizer1.add(radio.as_widget_ref());
    scroll1.set_content_sizer(sizer1);
    scroll1.set_min_content_height(380);

    // ===== Page 2: numeric inputs + progress (scrollable) =====
    let page2 = Panel::new(window.frame());
    let scroll2 = ScrollablePanel::install(&page2);
    let content2 = scroll2.content();
    let lbl2 = StaticText::new(&content2, "Sliders, spinners, gauges:");

    // 1. Slider — continuous value input
    let slider = Slider::new(&content2, 0, 100, 40);
    slider.set_tick_freq(10);
    let slider_for_cb = slider.clone();
    let status_for_slider = status.clone();
    {
        let cb = slider_for_cb.clone();
        let s = status_for_slider.clone();
        slider_for_cb.on_value_change(window.frame(), move || {
            s.set_status_text(&format!("Slider: {}", cb.get_value()), 0);
        });
    }

    // 3. SpinCtrl — numeric stepper
    let spin = SpinCtrl::new(&content2, 0, 1000, 250);
    let spin_for_cb = spin.clone();
    let status_for_spin = status.clone();
    {
        let cb = spin_for_cb.clone();
        let s = status_for_spin.clone();
        spin_for_cb.on_value_change(window.frame(), move || {
            s.set_status_text(&format!("Spin: {}", cb.get_value()), 0);
        });
    }

    // 2. Gauge — determinate progress
    let gauge = Gauge::new(&content2, 100);
    gauge.set_value(40);

    // 12. Timer — drives the gauge and a small status update every
    // 50ms. The tick closure is `FnMut` so we can use a counter
    // captured by `move`.
    let gauge_for_timer = gauge.clone();
    let status_for_timer = status.clone();
    let timer = Timer::new(window.frame());
    timer.on_tick(move || {
        let v = gauge_for_timer.get_value();
        gauge_for_timer.set_value((v + 1) % 101);
        if v % 10 == 0 {
            status_for_timer.set_status_text(&format!("Timer tick (gauge={v})"), 0);
        }
    });
    timer.start(Duration::from_millis(50));

    lbl2.as_widget_ref().borrow_mut().set_size(440, 22);
    slider.as_widget_ref().borrow_mut().set_size(440, 32);
    spin.as_widget_ref().borrow_mut().set_size(440, 28);
    gauge.as_widget_ref().borrow_mut().set_size(440, 28);

    let mut sizer2 = BoxSizer::vertical();
    sizer2.set_padding(8);
    sizer2.add(lbl2.as_widget_ref());
    sizer2.add_spacer(6);
    sizer2.add(slider.as_widget_ref());
    sizer2.add_spacer(6);
    sizer2.add(spin.as_widget_ref());
    sizer2.add_spacer(6);
    sizer2.add(gauge.as_widget_ref());
    scroll2.set_content_sizer(sizer2);
    scroll2.set_min_content_height(420);

    // ===== Page 3: pickers & custom font & popup trigger (scrollable) =====
    let page3 = Panel::new(window.frame());
    let scroll3 = ScrollablePanel::install(&page3);
    let content3 = scroll3.content();
    let lbl3 = StaticText::new(&content3, "Pickers and typography:");

    // 6. DatePickerCtrl — calendar popup
    let date_label = StaticText::new(&content3, "(no date chosen)");
    let date = DatePickerCtrl::new(&content3);
    let date_label_clone = date_label.clone();
    let status_for_date = status.clone();
    date.on_date_change(window.frame(), move |d| {
        if let Some(d) = d {
            date_label_clone.set_label(&format!("Date: {:04}-{:02}-{:02}", d.year, d.month, d.day));
            status_for_date.set_status_text(
                &format!("Date: {:04}-{:02}-{:02}", d.year, d.month, d.day),
                0,
            );
        } else {
            date_label_clone.set_label("(no date chosen)");
        }
    });

    // 7. ColourPickerCtrl — colour chooser
    let colour_label = StaticText::new(&content3, "Current colour: #000000");
    let colour = ColourPickerCtrl::new(&content3);
    let colour_label_clone = colour_label.clone();
    let status_for_colour = status.clone();
    colour.on_change(window.frame(), move |c: Colour| {
        let hex = format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b);
        colour_label_clone.set_label(&format!("Current colour: {hex}"));
        status_for_colour.set_status_text(&format!("Colour: {hex}"), 0);
    });

    // 13. Font — custom font for a label. We build a `FontDesc` with
    // the builder API (`bold()`) and turn it into a real `Font`. The
    // label then receives the font via `StaticText::set_font` (which
    // under the hood sends `WM_SETFONT` to the control).
    let custom_font = Font::new(FontDesc::new("Segoe UI", 16).bold());
    let fancy_label = StaticText::new(&content3, "Hello in a custom font!");
    fancy_label.set_font(&custom_font);

    // 17. PopupMenu — we don't get a right-click hook on the frame,
    // so the popup is shown by clicking this button. The popup mixes
    // plain items, a separator, and a checkable item, so it covers
    // the 18. `wxMenuItem` check/radio controls too.
    //
    // The `on_click` callback is `'static + FnMut`, so it can't
    // capture a local `&Frame`. We clone the owned `TopLevelWindow`
    // (which contains the Frame) and borrow a fresh `&Frame` from
    // inside the closure body. The inner popup-item callbacks are
    // also `'static`, so we use `move` on each and clone the values
    // they capture — that way they own their state and don't need
    // to borrow from the outer closure body.
    let status_for_popup = status.clone();
    let window_for_popup = window.clone();
    let popup_button = Button::new(&content3, "Show popup menu");
    popup_button.on_click(window.frame(), move || {
        let popup_frame: &Frame = window_for_popup.frame();
        let mut popup = PopupMenu::new();

        // Cut
        {
            let s = status_for_popup.clone();
            popup.append("Cut", popup_frame, move || {
                s.set_status_text("Popup: Cut", 0);
            });
        }
        // Copy
        {
            let s = status_for_popup.clone();
            popup.append("Copy", popup_frame, move || {
                s.set_status_text("Popup: Copy", 0);
            });
        }
        popup.append_separator();
        // Pin to top (checkable)
        {
            let s = status_for_popup.clone();
            popup.append_check_item("Pin to top", popup_frame, move || {
                s.set_status_text("Popup: Pin toggled", 0);
            });
        }
        // About… — opens a MessageDialog (14. `wxMessageDialog`).
        {
            let s = status_for_popup.clone();
            let w = window_for_popup.clone();
            popup.append("About…", popup_frame, move || {
                let _ = s;
                let dlg = MessageDialog::new(
                    w.frame(),
                    "About",
                    "Right-click popup fired!",
                    MessageDialogStyle::Ok,
                    MessageBoxIcon::Information,
                );
                dlg.show_modal();
            });
        }
        popup.popup(popup_frame);
    });

    lbl3.as_widget_ref().borrow_mut().set_size(440, 22);
    date.as_widget_ref().borrow_mut().set_size(200, 28);
    date_label.as_widget_ref().borrow_mut().set_size(440, 20);
    colour.as_widget_ref().borrow_mut().set_size(200, 32);
    colour_label.as_widget_ref().borrow_mut().set_size(440, 20);
    fancy_label.as_widget_ref().borrow_mut().set_size(440, 28);
    popup_button.as_widget_ref().borrow_mut().set_size(200, 32);

    let mut sizer3 = BoxSizer::vertical();
    sizer3.set_padding(8);
    sizer3.add(lbl3.as_widget_ref());
    sizer3.add_spacer(6);
    sizer3.add(date.as_widget_ref());
    sizer3.add(date_label.as_widget_ref());
    sizer3.add_spacer(6);
    sizer3.add(colour.as_widget_ref());
    sizer3.add(colour_label.as_widget_ref());
    sizer3.add_spacer(6);
    sizer3.add(fancy_label.as_widget_ref());
    sizer3.add_spacer(6);
    sizer3.add(popup_button.as_widget_ref());
    scroll3.set_content_sizer(sizer3);
    scroll3.set_min_content_height(480);

    // ===== Page 4: ListCtrl report view with per-row icons (scrollable) =====
    // Shows the `ListCtrl::set_image_list` + `insert_item_with_image`
    // API: each row carries one of the toolbar SVG glyphs, and the
    // selection callback drives the status bar.
    let page4 = Panel::new(window.frame());
    let scroll4 = ScrollablePanel::install(&page4);
    let content4 = scroll4.content();
    let lbl4 = StaticText::new(&content4, "ListCtrl (report) with per-row icons:");

    let report = ListCtrl::new(&content4, ListCtrlStyle::Report);
    report.insert_column(0, "Document", 200);
    report.insert_column(1, "Action", 120);
    report.insert_column(2, "When", 110);
    report.set_image_list(&toolbar_images);

    let history: [(i32, &str, &str, &str); 5] = [
        (0, "report_q3.docx", "created", "09:12"),
        (1, "budget.xlsx", "opened", "09:30"),
        (2, "notes.md", "saved", "10:02"),
        (3, "draft_old.txt", "cut", "10:15"),
        (4, "summary.pdf", "copied", "10:40"),
    ];
    for (i, (icon, doc, action, when)) in history.iter().enumerate() {
        report.insert_item_with_image(i, doc, *icon);
        report.set_item_text(i, 1, action);
        report.set_item_text(i, 2, when);
    }

    let status_for_report = status.clone();
    let report_for_cb = report.clone();
    report.on_item_selected(window.frame(), move |sel| {
        if let Some(idx) = sel {
            status_for_report.set_status_text(&format!("History row {idx} selected"), 0);
            let _ = &report_for_cb;
        }
    });

    lbl4.as_widget_ref().borrow_mut().set_size(500, 22);
    report.as_widget_ref().borrow_mut().set_size(500, 220);

    let mut sizer4 = BoxSizer::vertical();
    sizer4.set_padding(8);
    sizer4.add(lbl4.as_widget_ref());
    sizer4.add_spacer(6);
    sizer4.add(report.as_widget_ref());
    scroll4.set_content_sizer(sizer4);
    scroll4.set_min_content_height(400);

    // ===== Page 5: wxWidgets button variants (library factories) =====
    let page5 = Panel::new(window.frame());
    let scroll5 = ScrollablePanel::install(&page5);
    let content5 = scroll5.content();
    let lbl5 = StaticText::new(&content5, "Button variants (wxButton family):");

    let btn_standard = ButtonVariants::standard(&content5, "Standard");
    let btn_flat = ButtonVariants::flat(&content5, "Flat / liscio");
    let btn_bitmap = ButtonVariants::bitmap_only_svg(&content5, ICON_SAVE, 40, 40);
    let btn_img_left =
        ButtonVariants::text_with_image_left(&content5, "Save left", ICON_SAVE, 24);
    let btn_img_right =
        ButtonVariants::text_with_image_right(&content5, "Open right", ICON_OPEN, 24);
    let btn_cmd = ButtonVariants::command_link(
        &content5,
        "Install feature pack",
        "Adds optional components to the showcase",
    );
    let btn_toggle = ButtonVariants::toggle(&content5, "Pin toolbar");
    let btn_bmp_toggle = ButtonVariants::bitmap_toggle_svg(&content5, ICON_COPY, 32);
    let btn_menu = ButtonVariants::menu_drop_down(&content5, "Actions ▾");
    {
        let mut menu = btn_menu.menu_mut();
        let s = status.clone();
        menu.append("Refresh", window.frame(), move || {
            s.set_status_text("Menu: Refresh", 0);
        });
        let s = status.clone();
        menu.append("Export…", window.frame(), move || {
            s.set_status_text("Menu: Export…", 0);
        });
    }
    btn_menu.bind_menu_popup(window.frame());
    let btn_anim = ButtonVariants::animated_demo(&content5, window.frame());

    let buttons: [(&AnyButton, &str); 10] = [
        (&btn_standard, "Standard push button"),
        (&btn_flat, "Flat push button"),
        (&btn_bitmap, "Bitmap-only button"),
        (&btn_img_left, "Text + image left"),
        (&btn_img_right, "Text + image right"),
        (&btn_cmd, "Command link"),
        (&btn_toggle, "Toggle button"),
        (&btn_bmp_toggle, "Bitmap toggle"),
        (&btn_menu, "Menu drop-down"),
        (&btn_anim, "Animated button"),
    ];
    for (btn, msg) in buttons {
        btn.on_click_status(window.frame(), &status, msg);
    }

    lbl5.as_widget_ref().borrow_mut().set_size(520, 22);

    let mut left_col = BoxSizer::vertical();
    left_col.set_padding(4);
    let mut right_col = BoxSizer::vertical();
    right_col.set_padding(4);

    for (i, (btn, _)) in buttons.iter().enumerate() {
        let col = if i < 5 { &mut left_col } else { &mut right_col };
        let hint = StaticText::new(&content5, btn.kind_label());
        hint.as_widget_ref().borrow_mut().set_size(250, 18);
        let h = if btn.kind() == ru_wx::ButtonKind::CommandLink {
            52
        } else if btn.kind() == ru_wx::ButtonKind::Animated || btn.kind() == ru_wx::ButtonKind::BitmapOnly
        {
            48
        } else {
            40
        };
        btn.as_widget_ref().borrow_mut().set_size(250, h);
        col.add(hint.as_widget_ref());
        col.add_spacer(2);
        col.add(btn.as_widget_ref());
        col.add_spacer(10);
    }

    let mut cols = BoxSizer::horizontal();
    cols.set_padding(8);
    cols.add_sizer_with_proportion(left_col, 1);
    cols.add_sizer_with_proportion(right_col, 1);

    let mut sizer5 = BoxSizer::vertical();
    sizer5.set_padding(8);
    sizer5.add(lbl5.as_widget_ref());
    sizer5.add_spacer(6);
    sizer5.add_sizer(cols);
    scroll5.set_content_sizer(sizer5);
    scroll5.set_min_content_height(560);

    let scroll_panels = [
        scroll1.clone(),
        scroll2.clone(),
        scroll3.clone(),
        scroll4.clone(),
        scroll5.clone(),
    ];

    // Add the five pages to the notebook. The page-with-image
    // variant picks the tab-strip icon out of the toolbar's image
    // list.
    notebook.add_page_with_image("Lists", &page1, 0);
    notebook.add_page_with_image("Numeric", &page2, 3);
    notebook.add_page_with_image("Pickers", &page3, 4);
    notebook.add_page_with_image("Data", &page4, 2);
    notebook.add_page_with_image("Buttons", &page5, 1);

    let on_change_status = status.clone();
    let scrolls_for_tab = scroll_panels.clone();
    notebook.on_selection_change(window.frame(), move |idx| {
        let names = ["Lists", "Numeric", "Pickers", "Data", "Buttons"];
        let name = names.get(idx).copied().unwrap_or("?");
        on_change_status.set_status_text(&format!("Tab: {name} ({idx})"), 0);
        if let Some(scroll) = scrolls_for_tab.get(idx) {
            scroll.refresh();
        }
    });

    // ---- Menu bar with checkable / radio items (18) + accelerators (22) ----
    // The library does not support nested submenus, so the View menu
    // is intentionally flat: status-bar / tool-bar / full-screen
    // toggles, the zoom radio group, a separator, and a "flash" item
    // to demo a TopLevelWindow method. The File menu is built with
    // `append_with_shortcut` so each entry shows its `Ctrl+…` binding
    // on the right-hand side of the menu and is also registered with
    // the frame's accelerator table — the shortcut fires even if the
    // menu bar is hidden (e.g. with the Alt key alone). The dimmed
    // "Print preview" entry exercises `append_disabled_with_shortcut`.

    // ---- 22. File menu — `Accelerator` + `append_with_shortcut` ----
    let mut file_menu = ru_wx::Menu::new("&File");

    let status_for_new = status.clone();
    let _new_id = file_menu.append_with_shortcut(
        "&New",
        Accelerator::parse("Ctrl+N").unwrap(),
        window.frame(),
        move || {
            status_for_new.set_status_text("File > New (Ctrl+N)", 0);
        },
    );

    let status_for_open = status.clone();
    let _open_id = file_menu.append_with_shortcut(
        "&Open…",
        Accelerator::parse("Ctrl+O").unwrap(),
        window.frame(),
        move || {
            status_for_open.set_status_text("File > Open… (Ctrl+O)", 0);
        },
    );

    let status_for_save = status.clone();
    let _save_id = file_menu.append_with_shortcut(
        "&Save",
        Accelerator::parse("Ctrl+S").unwrap(),
        window.frame(),
        move || {
            status_for_save.set_status_text("File > Save (Ctrl+S)", 0);
        },
    );

    file_menu.append_separator();

    // Disabled item: still owns an accelerator, so the binding is
    // parsed / displayed, but the item itself is greyed out and
    // does not fire when clicked or pressed.
    let _print_id = file_menu.append_disabled_with_shortcut(
        "&Print preview (disabled)",
        Accelerator::parse("Ctrl+P").unwrap(),
        window.frame(),
    );

    file_menu.append_separator();

    let status_for_quit = status.clone();
    let _quit_id = file_menu.append_with_shortcut(
        "&Quit",
        Accelerator::parse("Ctrl+Q").unwrap(),
        window.frame(),
        move || {
            status_for_quit.set_status_text("File > Quit (Ctrl+Q) — would close the app", 0);
        },
    );

    let mut view_menu = ru_wx::Menu::new("&View");
    let sb_id = view_menu.append_check_item("Show &status bar", window.frame(), || {});
    let aui_for_view = aui.clone();
    let tb_id = view_menu.append_check_item("Show &tool bar", window.frame(), move || {
        let visible = aui_for_view.as_widget_ref().borrow().is_visible();
        aui_for_view
            .as_widget_ref()
            .borrow_mut()
            .set_visible(!visible);
    });
    let fs_id = view_menu.append_check_item("&Full screen", window.frame(), || {});

    // Check the items by default so the user sees the checkmark.
    view_menu.check_item(sb_id, true);
    view_menu.check_item(tb_id, true);
    view_menu.check_item(fs_id, false);

    view_menu.append_separator();
    let z100 = view_menu.append_radio_item("&100%", window.frame(), || {});
    let _z125 = view_menu.append_radio_item("&125%", window.frame(), || {});
    let _z150 = view_menu.append_radio_item("&150%", window.frame(), || {});
    view_menu.check_item(z100, true);

    view_menu.append_separator();
    let window_for_flash = window.clone();
    view_menu.append("&Flash taskbar", window.frame(), move || {
        window_for_flash.request_user_attention(UserAttentionFlags::Default);
    });

    let mut help_menu = ru_wx::Menu::new("&Help");
    {
        let window_for_about = window.clone();
        help_menu.append("&About ru_wx…", window.frame(), move || {
            // 14. MessageDialog — modal "About" box.
            let dlg = MessageDialog::new(
                window_for_about.frame(),
                "About ru_wx",
                "ru_wx — native Win32 GUI library for Rust.\n\n\
                 This window showcases all 20 controls ported from\n\
                 MIGRATION_STATUS.md (lines 126-144), plus the\n\
                 v0.4.0 HiDPI and v0.4.1 accelerator APIs.",
                MessageDialogStyle::Ok,
                MessageBoxIcon::Information,
            );
            dlg.show_modal();
        });
    }

    let mut menubar = ru_wx::MenuBar::new();
    menubar.append(file_menu);
    menubar.append(view_menu);
    menubar.append(help_menu);
    window.frame().set_menu_bar(menubar);

    // ---- 20. ToolTip — per-widget hover tooltips ----
    // Bind a small description to a handful of widgets. The tooltip
    // is implemented as a single native `tooltips_class32` child of
    // the top-level window, so all of these tooltips share one OS
    // control. The library walks `GetAncestor(target, GA_ROOT)` to
    // find the top-level and then locates (or creates) that one
    // child.
    ToolTip::new("Create a new document (Ctrl+N)").attach(&popup_button.as_widget_ref());
    ToolTip::new("Type a custom name and a font size above").attach(&fancy_label.as_widget_ref());
    ToolTip::new("Drag the handle or click to set a value").attach(&slider.as_widget_ref());
    let spin_tip = ToolTip::new("Increments shown in the status bar");
    spin_tip.attach(&spin.as_widget_ref());
    // Mutating a tooltip after attach: the new text takes effect on
    // the next hover. (Won't fire visibly because the cursor isn't
    // over the control, but the registration is updated.)
    spin_tip.set_text("Numeric stepper (0..1000)");
    // Globally disable / re-enable all tooltips this library owns.
    // Comment out the next line to see the tooltips appear on hover.
    ToolTip::enable(false);
    // Show that we can still read the current text (e.g. for an
    // inspection / debug overlay).
    let _ = spin_tip.text();

    // Banner sotto la AuiToolBar — rende evidente che la demo è aggiornata.
    let banner = StaticText::new(
        window.frame(),
        "≡ AuiToolBar colorata 40×40  |  5 tab (apri «Buttons»)  |  scrollbar verticali nelle pagine",
    );

    // Use the Frame's sizer: spacer per la toolbar, banner, poi notebook.
    let mut main_sizer = BoxSizer::new(Orientation::Vertical);
    main_sizer.set_padding(4);
    main_sizer.add_spacer(toolbar_reserved);
    main_sizer.add(banner.as_widget_ref());
    main_sizer.add_with_proportion(notebook.as_widget_ref(), 1);
    window.frame().set_sizer(main_sizer);

    // Dopo il layout iniziale: toolbar sopra il notebook, scroll aggiornati.
    aui.bring_to_front();
    for scroll in &scroll_panels {
        scroll.refresh();
    }

    // Apri la pagina Lists (Choice corretto); usa le tab per le altre sezioni.
    notebook.set_selection(0);

    let scrolls_on_resize = scroll_panels.clone();
    let aui_on_resize = aui.clone();
    window.frame().on_resize(move |_w, _h| {
        aui_on_resize.bring_to_front();
        for scroll in &scrolls_on_resize {
            scroll.refresh();
        }
    });

    // ---- Run the application ----
    app.run(window.into_frame());
}
