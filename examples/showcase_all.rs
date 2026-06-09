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
    Accelerator, App, ArtClient, ArtId, ArtProvider, BitmapBundle, BoxSizer, Button,
    CentreDirection, CheckListBox, Choice, Colour, ColourPickerCtrl, DatePickerCtrl, Font,
    FontDesc, Frame, Gauge, ImageList, MessageBoxIcon, MessageDialog, MessageDialogStyle,
    Orientation, Panel, PopupMenu, RadioBox, Slider, SpinCtrl, StaticText, StatusBar, Tab, Timer,
    ToolBar, ToolTip, TopLevelWindow, UserAttentionFlags,
};

// SVG icons for the toolbar. These are tiny inline SVGs that get
// rasterised by the `image` crate at startup.
const ICON_NEW: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M12 18v-6M9 15h6"/></svg>"#;
const ICON_OPEN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>"#;
const ICON_SAVE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M5 3h11l3 3v15H5z M8 3v6h7V3 M8 14h8v7H8z"/></svg>"#;
const ICON_CUT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M8.12 8.12L20 20 M8.12 15.88L20 4"/></svg>"#;
const ICON_COPY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="8" y="8" width="13" height="13"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/></svg>"#;

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
    let window = TopLevelWindow::new("ru_wx showcase (19 controls)", 820, 620);

    // Centre the window on the screen before showing it.
    window.centre(CentreDirection::Screen);

    // ---- Status bar (9) — 3 fields at the bottom of the frame ----
    // Field 1 is wired to a live read-out of the frame's DPI / scale
    // factor (21. HiDPI). PerMonitorV2 awareness means the value
    // changes automatically when the window is dragged to a
    // different monitor.
    let status = StatusBar::new(window.frame(), 3);
    status.set_status_text("Ready", 0);
    {
        let dpi = window.frame().dpi();
        let scale = dpi.scale_factor();
        status.set_status_text(&format!("DPI: {} ({:.2}x)", dpi, scale), 1);
    }
    status.set_status_text("Field 3", 2);

    // ---- Tool bar (10) with custom SVG icons + separators ----
    // 15. BitmapBundle: rasterise the SVGs at 16, 20 and 24 px so the
    // toolbar looks crisp on HiDPI screens. 16. ArtProvider could also
    // supply these, but BitmapBundle gives us full control over the
    // asset (and we can register the result back into ArtProvider).
    let icon_sizes: [(u32, u32); 3] = [(16, 16), (20, 20), (24, 24)];

    let bundle_new = BitmapBundle::from_svg_bytes(ICON_NEW, &icon_sizes);
    let bundle_open = BitmapBundle::from_svg_bytes(ICON_OPEN, &icon_sizes);
    let bundle_save = BitmapBundle::from_svg_bytes(ICON_SAVE, &icon_sizes);
    let bundle_cut = BitmapBundle::from_svg_bytes(ICON_CUT, &icon_sizes);
    let bundle_copy = BitmapBundle::from_svg_bytes(ICON_COPY, &icon_sizes);

    // Build an ImageList from the bitmaps inside the bundles. The
    // image list will be attached to the toolbar and to the notebook
    // below.
    let toolbar_images = ImageList::new(24, 24);
    if let Some(bmp) = bundle_new.best_for_size((24, 24)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_open.best_for_size((24, 24)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_save.best_for_size((24, 24)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_cut.best_for_size((24, 24)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_copy.best_for_size((24, 24)) {
        toolbar_images.add_bitmap(bmp.hbitmap);
    }

    let toolbar = ToolBar::new(window.frame());
    toolbar.set_image_list(&toolbar_images);
    toolbar.add_tool(ID_TOOL_NEW, "New", 0);
    toolbar.add_tool(ID_TOOL_OPEN, "Open", 1);
    toolbar.add_tool(ID_TOOL_SAVE, "Save", 2);
    toolbar.add_separator();
    toolbar.add_tool(ID_TOOL_CUT, "Cut", 3);
    toolbar.add_tool(ID_TOOL_COPY, "Copy", 4);
    toolbar.realize();

    // Wire up the toolbar's click events. The single callback fires
    // for every tool with the tool's id. We clone the window so the
    // callback owns its captured value (the callback must be
    // `'static`).
    let status_for_tools = status.clone();
    let window_for_tools = window.clone();
    toolbar.on_tool_clicked(window.frame(), move |id| {
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

    // ===== Page 1: lists & selections =====
    let page1 = Panel::new(window.frame());
    let lbl1 = StaticText::new(&page1, "Lists and selection controls:");

    // 4. Choice — simple drop-down with no edit
    let choice = Choice::new(&page1);
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
    let checklist = CheckListBox::new(&page1);
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
    let radio = RadioBox::new(&page1, "Priority", &["Low", "Normal", "High", "Urgent"]);
    radio.set_selection(1);
    let status_for_radio = status.clone();
    radio.on_select(window.frame(), move |idx| {
        let label = ["Low", "Normal", "High", "Urgent"]
            .get(idx)
            .copied()
            .unwrap_or("?");
        status_for_radio.set_status_text(&format!("Priority: {label}"), 0);
    });

    let mut sizer1 = BoxSizer::vertical();
    sizer1.add(lbl1.as_widget_ref());
    sizer1.add_stretch(0);
    sizer1.add(choice.as_widget_ref());
    sizer1.add_stretch(0);
    sizer1.add(checklist.as_widget_ref());
    sizer1.add_stretch(0);
    sizer1.add(radio.as_widget_ref());
    page1.set_sizer(sizer1);

    // ===== Page 2: numeric inputs + progress =====
    let page2 = Panel::new(window.frame());
    let lbl2 = StaticText::new(&page2, "Sliders, spinners, gauges:");

    // 1. Slider — continuous value input
    let slider = Slider::new(&page2, 0, 100, 40);
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
    let spin = SpinCtrl::new(&page2, 0, 1000, 250);
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
    let gauge = Gauge::new(&page2, 100);
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

    let mut sizer2 = BoxSizer::vertical();
    sizer2.add(lbl2.as_widget_ref());
    sizer2.add_stretch(0);
    sizer2.add(slider.as_widget_ref());
    sizer2.add(spin.as_widget_ref());
    sizer2.add(gauge.as_widget_ref());
    page2.set_sizer(sizer2);

    // ===== Page 3: pickers & custom font & popup trigger =====
    let page3 = Panel::new(window.frame());
    let lbl3 = StaticText::new(&page3, "Pickers and typography:");

    // 6. DatePickerCtrl — calendar popup
    let date_label = StaticText::new(&page3, "(no date chosen)");
    let date = DatePickerCtrl::new(&page3);
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
    let colour_label = StaticText::new(&page3, "Current colour: #000000");
    let colour = ColourPickerCtrl::new(&page3);
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
    let fancy_label = StaticText::new(&page3, "Hello in a custom font!");
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
    let popup_button = Button::new(&page3, "Show popup menu");
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

    let mut sizer3 = BoxSizer::vertical();
    sizer3.add(lbl3.as_widget_ref());
    sizer3.add_stretch(0);
    sizer3.add(date.as_widget_ref());
    sizer3.add(date_label.as_widget_ref());
    sizer3.add_stretch(0);
    sizer3.add(colour.as_widget_ref());
    sizer3.add(colour_label.as_widget_ref());
    sizer3.add_stretch(0);
    sizer3.add(fancy_label.as_widget_ref());
    sizer3.add_stretch(0);
    sizer3.add(popup_button.as_widget_ref());
    page3.set_sizer(sizer3);

    // Add the three pages to the notebook. The page-with-image
    // variant picks the tab-strip icon out of the toolbar's image
    // list.
    notebook.add_page_with_image("Lists", &page1, 0);
    notebook.add_page_with_image("Numeric", &page2, 3);
    notebook.add_page_with_image("Pickers", &page3, 4);

    let on_change_status = status.clone();
    notebook.on_selection_change(window.frame(), move |idx| {
        on_change_status.set_status_text(&format!("Tab page {idx}"), 0);
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
    let tb_id = view_menu.append_check_item("Show &tool bar", window.frame(), || {});
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

    // Use the Frame's sizer to lay out the tab across the full
    // client area. `add_with_proportion(_, 1)` gives the notebook the
    // entire client rectangle (it absorbs any extra space).
    let mut main_sizer = BoxSizer::new(Orientation::Vertical);
    main_sizer.add_with_proportion(notebook.as_widget_ref(), 1);
    window.frame().set_sizer(main_sizer);

    // ---- Run the application ----
    app.run(window.into_frame());
}
