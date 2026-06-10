//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `Wizard` — multi-page navigation dialog (`wxWizard`).
//!
//! Demonstrates:
//! - Building a `Wizard` with three pages (Welcome / Details / Finish).
//! - Wiring each page with a `StaticText` label and a per-page button.
//! - Reacting to navigation events via `on_page_changed`.
//! - Reacting to terminal events via `on_finish` / `on_cancel`.
//! - Inspecting the modal result via `WizardResult`.
//!
//! The wizard is created on demand by clicking a button on a small
//! "launcher" frame; once the user finishes or cancels the wizard,
//! the launcher shows the outcome in its status bar.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_wizard
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Button, Frame, Panel, StaticText, StatusBar, Wizard, WizardResult,
};

fn main() {
    let app = App::new();

    // Launcher frame — hosts the button that opens the wizard and
    // reports the outcome. The wizard owns its own top-level window
    // so it doesn't need a parent here.
    let launcher = Frame::builder()
        .with_title("Minitest — Wizard")
        .with_size(440, 220)
        .with_modern_style().build();
    let status = StatusBar::new(&launcher, 1);
    status.set_status_text("Click 'Open wizard' to start the demo.", 0);

    StaticText::new(
        &launcher,
        "Press the button to open a 3-page wizard (Welcome → Details → Finish).",
    );

    // ── Open wizard ────────────────────────────────────────────────────
    let status_for_open = status.clone();
    let btn_open = Button::new(&launcher, "Open wizard");
    btn_open.on_click(&launcher, move || {
        // Build a 3-page wizard. The wizard owns its own frame; the
        // caller gets to lay out each page with normal widgets before
        // handing the panel off via `add_page`.
        let mut wiz = Wizard::new("Setup Wizard", 480, 340);

        // Page 1 — Welcome
        let p1 = Panel::new(&wiz.frame());
        StaticText::new(
            &p1,
            "Welcome!\n\nThis is the first page of the wizard demo.\nClick 'Next >' to continue.",
        );
        wiz.add_page("Welcome", p1);

        // Page 2 — Details
        let p2 = Panel::new(&wiz.frame());
        StaticText::new(
            &p2,
            "Page 2 — Details\n\nPretend there are some fields here.\nClick 'Next >' to continue.",
        );
        wiz.add_page("Details", p2);

        // Page 3 — Finish
        let p3 = Panel::new(&wiz.frame());
        StaticText::new(
            &p3,
            "Page 3 — Finish\n\nThis is the last page. The 'Next >' button\nis now replaced by 'Finish'.",
        );
        wiz.add_page("Finish", p3);

        // React to every page transition: write the new page index
        // out to a side channel (println!) so the smoke check can
        // see the navigation. In a real app this would update
        // page-local state.
        wiz.on_page_changed(|idx| {
            println!("[wizard] navigated to page {idx}");
        });

        // React to terminal events. The `on_finish` / `on_cancel`
        // callbacks are fired by `run()` itself, after the modal
        // loop exits but before the result is returned to the caller.
        wiz.on_finish(|| {
            println!("[wizard] on_finish callback fired");
        });
        wiz.on_cancel(|| {
            println!("[wizard] on_cancel callback fired");
        });

        // Run the modal flow. The wizard opens its own window and
        // pumps a local message loop until the user clicks Finish,
        // Cancel, or closes the window. `on_finish` / `on_cancel`
        // fire before `run` returns.
        let result = wiz.run();
        let msg = match result {
            WizardResult::Finished => "Wizard completed (Finish clicked).",
            WizardResult::Cancelled => "Wizard cancelled (Cancel clicked or window closed).",
        };
        println!("[wizard] run() returned: {msg}");
        status_for_open.set_status_text(msg, 0);
    });

    // Layout
    let mut sizer = BoxSizer::vertical();
    sizer.add(btn_open.as_widget_ref());
    launcher.set_sizer(sizer);

    app.run(launcher);
}
