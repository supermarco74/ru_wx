//! Minitest: `PropertySheetDialog` — tabbed settings dialog
//! (`wxPropertySheetDialog`).
//!
//! Demonstrates:
//! - Building a `PropertySheetDialog` parented to a launcher frame.
//! - Adding multiple tabbed pages via `add_page(label, panel)`.
//! - Wiring each page with a `StaticText` label and a per-page button.
//! - Registering an `on_apply` callback (fires inline, dialog stays
//!   open — Apply is **not** terminal).
//! - Inspecting the modal result via `PropertySheetDialogResult`.
//!
//! The dialog is created on demand by clicking a button on a small
//! "launcher" frame; once the user clicks OK or Cancel, the launcher
//! shows the outcome in its status bar.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_property_sheet_dialog
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Button, Frame, Panel, PropertySheetDialog, PropertySheetDialogResult, StaticText,
    StatusBar,
};

fn main() {
    let app = App::new();

    // Launcher frame — owns the dialog as a modal child.
    let launcher = Frame::builder()
        .with_title("Minitest — PropertySheetDialog")
        .with_size(440, 220)
        .build();
    let status = StatusBar::new(&launcher, 1);
    status.set_status_text("Click 'Open settings' to start the demo.", 0);

    StaticText::new(
        &launcher,
        "Press the button to open a tabbed settings dialog\n(General / Advanced / About).",
    );

    // ── Open settings dialog ───────────────────────────────────────────
    let status_for_open = status.clone();
    let launcher_for_open = launcher.clone();
    let btn_open = Button::new(&launcher, "Open settings");
    btn_open.on_click(&launcher, move || {
        // The dialog is a top-level window parented to the launcher.
        // The launcher is automatically disabled while the dialog is
        // modal and re-enabled when `show_modal` returns.
        let mut dlg = PropertySheetDialog::new(&launcher_for_open, "Settings", 480, 380);

        // Tab 1 — General
        let p1 = Panel::new(&dlg.frame());
        StaticText::new(&p1, "General settings go here.");
        dlg.add_page("General", p1);

        // Tab 2 — Advanced
        let p2 = Panel::new(&dlg.frame());
        StaticText::new(&p2, "Advanced settings go here.");
        dlg.add_page("Advanced", p2);

        // Tab 3 — About
        let p3 = Panel::new(&dlg.frame());
        StaticText::new(&p3, "About this app — version 0.5.7.");
        dlg.add_page("About", p3);

        // Apply does **not** close the dialog — it just runs this
        // callback inline. The user keeps editing and can dismiss the
        // dialog with OK or Cancel at their leisure.
        dlg.on_apply(|| {
            println!("[psd] on_apply callback fired (dialog stays open)");
        });

        // Run the modal flow. The launcher is disabled until the
        // user clicks OK, Cancel, or closes the window. `on_apply`
        // fires inline whenever the user clicks Apply.
        let result = dlg.show_modal();
        let msg = match result {
            PropertySheetDialogResult::Ok => "Settings: OK clicked.",
            PropertySheetDialogResult::Cancelled => "Settings: cancelled.",
        };
        println!("[psd] show_modal() returned: {msg}");
        status_for_open.set_status_text(msg, 0);
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_open.as_widget_ref());
    launcher.set_sizer(sizer);

    app.run(launcher);
}
