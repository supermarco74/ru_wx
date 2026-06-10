//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `HyperlinkCtrl` — SysLink controls with SVG icons.
//!
//! Demonstrates:
//! 1. Several links, each with a coloured inline-SVG icon next to
//!    it (`StaticBitmap` + `SVGBitmap`).
//! 2. `on_click` callbacks: clicked URL in status field 0, total
//!    click counter in field 1.
//! 3. `on_link` with the typed [`ru_wx::HyperlinkEvent`] payload.
//! 4. Retargeting a link at runtime with `set_label` / `set_url`.
//!
//! Note: the underlying Win32 SysLink always renders with the
//! system link colour — there is no custom-colour API on
//! `HyperlinkCtrl`, so the colour accents come from the icons.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_hyperlink
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{
    App, BoxSizer, Button, Frame, HyperlinkCtrl, SVGBitmap, StaticBitmap, StaticText, StatusBar,
};

// Coloured glyphs (double-hash raw literals: the SVGs contain `#RRGGBB`).
const SVG_GITHUB: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#24292F"><circle cx="12" cy="12" r="10"/><circle cx="9" cy="11" r="1.6" fill="#FFFFFF"/><circle cx="15" cy="11" r="1.6" fill="#FFFFFF"/><path d="M9 16c2 1.4 4 1.4 6 0" stroke="#FFFFFF" stroke-width="1.4" fill="none"/></svg>"##;
const SVG_DOCS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20V4H6.5A2.5 2.5 0 0 0 4 6.5z"/><path d="M8 8h8M8 12h6"/></svg>"##;
const SVG_CRATE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#E8B339" stroke="#9C7218" stroke-width="1.5"><path d="M12 2 3 7v10l9 5 9-5V7z"/><path d="M3 7l9 5 9-5M12 12v10" fill="none"/></svg>"##;
const SVG_HOME: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#4FA464" stroke-width="2"><path d="M3 11 12 3l9 8"/><path d="M5 10v10h5v-6h4v6h5V10"/></svg>"##;

/// One row: 16-px SVG icon + SysLink.
fn link_row(frame: &Frame, svg: &[u8], label: &str, url: &str) -> (HyperlinkCtrl, BoxSizer, SVGBitmap) {
    let mut icon_svg = SVGBitmap::new(16, 16);
    let mut row = BoxSizer::horizontal();
    if icon_svg.load_from_bytes(svg) {
        if let Some(bmp) = icon_svg.bitmap() {
            let icon = StaticBitmap::with_bitmap(frame, bmp.handle(), 16, 16);
            row.add(icon.as_widget_ref());
            row.add_spacer(6);
        }
    }
    let link = HyperlinkCtrl::new(frame, label, url);
    row.add(link.as_widget_ref());
    (link, row, icon_svg)
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — HyperlinkCtrl")
        .with_size(520, 320)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Click a link…", 0);
    status.set_status_text("clicks: 0", 1);

    let hint = StaticText::new(&frame, "SysLink controls with SVG icons (click to test):");

    let clicks = Rc::new(Cell::new(0u32));
    let bump = {
        let status = status.clone();
        let clicks = clicks.clone();
        move |what: &str| {
            clicks.set(clicks.get() + 1);
            status.set_status_text(what, 0);
            status.set_status_text(&format!("clicks: {}", clicks.get()), 1);
        }
    };

    // ── Links ─────────────────────────────────────────────────────
    let (gh, row_gh, keep_gh) =
        link_row(&frame, SVG_GITHUB, "ru_wx on GitHub", "https://github.com/");
    let cb = bump.clone();
    gh.on_click(&frame, move || cb("GitHub link clicked"));

    let (docs, row_docs, keep_docs) =
        link_row(&frame, SVG_DOCS, "API docs on docs.rs", "https://docs.rs/");
    // `on_link` delivers a typed HyperlinkEvent carrying the URL.
    let cb = bump.clone();
    docs.on_link(&frame, move |ev: &ru_wx::HyperlinkEvent| {
        cb(&format!("HyperlinkEvent → {}", ev.url));
    });

    let (krate, row_crate, keep_crate) =
        link_row(&frame, SVG_CRATE, "ru_wx on crates.io", "https://crates.io/");
    let cb = bump.clone();
    krate.on_click(&frame, move || cb("crates.io link clicked"));

    let (home, row_home, keep_home) = link_row(
        &frame,
        SVG_HOME,
        "EasyTaskFlow home",
        "https://www.easytaskflow.app",
    );
    let home_for_click = home.clone();
    let cb = bump.clone();
    home.on_click(&frame, move || {
        cb(&format!("Home link → {}", home_for_click.url()));
    });

    // ── Retarget the last link at runtime ─────────────────────────
    let retarget_btn = Button::new(&frame, "Retarget last link");
    let home_for_btn = home.clone();
    let status_for_btn = status.clone();
    let swapped = Rc::new(Cell::new(false));
    retarget_btn.on_click(&frame, move || {
        let to_blog = !swapped.get();
        swapped.set(to_blog);
        if to_blog {
            home_for_btn.set_label("EasyTaskFlow blog");
            home_for_btn.set_url("https://www.easytaskflow.app/blog");
        } else {
            home_for_btn.set_label("EasyTaskFlow home");
            home_for_btn.set_url("https://www.easytaskflow.app");
        }
        status_for_btn.set_status_text(
            &format!("Last link now points to {}", home_for_btn.url()),
            0,
        );
    });

    // ── Layout ────────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add_spacer(4);
    sizer.add_sizer(row_gh);
    sizer.add_sizer(row_docs);
    sizer.add_sizer(row_crate);
    sizer.add_sizer(row_home);
    sizer.add_spacer(8);
    sizer.add(retarget_btn.as_widget_ref());
    frame.set_sizer(sizer);

    // Keep the SVG rasterisations alive for the message loop.
    let _keep = (keep_gh, keep_docs, keep_crate, keep_home);

    app.run(frame);
}
