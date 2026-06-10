//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `StaticBitmap` — image display gallery.
//!
//! Demonstrates:
//! 1. A multi-resolution [`BitmapBundle`] from an SVG, shown at
//!    16 / 32 / 64 px side by side (best-fit selection).
//! 2. Coloured inline-SVG glyphs rasterised via [`SVGBitmap`].
//! 3. A **procedurally drawn** [`Bitmap`] (MemoryDC: stripes,
//!    ellipse, text) displayed in a `StaticBitmap`.
//! 4. An `HICON` produced from inline SVG bytes.
//! 5. Runtime image swapping with `set_bitmap` and `clear`.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_static_bitmap
//! ```

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::rc::Rc;

use ru_wx::{
    svg_bytes_to_hicon, App, Bitmap, BitmapBundle, BoxSizer, Brush, Button, Colour, Dc, Frame,
    MemoryDC, Pen, RawBitmap, SVGBitmap, StaticBitmap, StaticText, StatusBar,
};

const STAR_SVG: &[u8] = include_bytes!("../../assets/icons/star.svg");

// Coloured glyphs (note the `br##"…"##` form: the SVG contains `#RRGGBB`).
const SVG_HEART: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#E2484F" stroke="#A41F2B" stroke-width="1.5"><path d="M12 21s-7.5-4.9-10-9.5C.5 7.5 2.5 4 6 4c2.2 0 3.6 1.2 6 3.6C14.4 5.2 15.8 4 18 4c3.5 0 5.5 3.5 4 7.5C19.5 16.1 12 21 12 21z"/></svg>"##;
const SVG_LEAF: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#4FA464" stroke="#2E7041" stroke-width="1.5"><path d="M5 21c0-9 4-16 14-17 1 10-3 16-12 16-1 0-2 .4-2 1z"/><path d="M5 21C8 14 12 10 17 7" fill="none"/></svg>"##;
const SVG_BOLT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#F2B705" stroke="#A37C03" stroke-width="1.5" d="M13 2 4 14h6l-1 8 9-12h-6z"/></svg>"##;
const SVG_INFO: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#3A86C8" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>"##;

/// Procedurally paint a little "sunset" card: sky bands, a sun and
/// a caption — all drawn with [`MemoryDC`] primitives.
fn build_sunset(width: i32, height: i32) -> Bitmap {
    let bmp = Bitmap::new(width as u32, height as u32);
    // Pens / brushes are declared BEFORE the DC so the DC drops
    // first (restoring stock objects) and the GDI handles can be
    // deleted cleanly afterwards.
    let sun_brush = Brush::solid(Colour::new(255, 200, 60, 255));
    let sun_pen = Pen::solid(Colour::new(214, 140, 20, 255));
    {
        let mut mdc = MemoryDC::new();
        mdc.select_bitmap(&bmp);
        // Sky: four horizontal colour bands.
        let bands = [
            Colour::new(38, 70, 130, 255),
            Colour::new(196, 90, 70, 255),
            Colour::new(236, 140, 70, 255),
            Colour::new(250, 200, 120, 255),
        ];
        let band_h = height / bands.len() as i32;
        for (i, c) in bands.iter().enumerate() {
            mdc.fill_rect(0, i as i32 * band_h, width, band_h + 1, *c);
        }
        // The sun: filled circle straddling the horizon.
        mdc.set_pen(Some(&sun_pen));
        mdc.set_brush(Some(&sun_brush));
        mdc.draw_ellipse(width / 2 - 14, height / 2 - 6, 28, 28);
        // Sea: bottom strip.
        mdc.fill_rect(0, height - band_h / 2, width, band_h / 2, Colour::new(30, 60, 110, 255));
    }
    bmp
}

/// Procedurally paint an "ocean" card: vertical stripes + rings.
fn build_ocean(width: i32, height: i32) -> Bitmap {
    let bmp = Bitmap::new(width as u32, height as u32);
    let ring_pen = Pen::new(Colour::WHITE, 2, ru_wx::PenStyle::Solid);
    {
        let mut mdc = MemoryDC::new();
        mdc.select_bitmap(&bmp);
        let stripes = [
            Colour::new(20, 60, 110, 255),
            Colour::new(30, 90, 150, 255),
            Colour::new(50, 130, 190, 255),
            Colour::new(90, 180, 220, 255),
        ];
        let stripe_w = width / stripes.len() as i32;
        for (i, c) in stripes.iter().enumerate() {
            mdc.fill_rect(i as i32 * stripe_w, 0, stripe_w + 1, height, *c);
        }
        // Concentric outline rings (transparent fill).
        mdc.set_pen(Some(&ring_pen));
        mdc.set_brush(None);
        mdc.draw_ellipse(width / 2 - 22, height / 2 - 22, 44, 44);
        mdc.draw_ellipse(width / 2 - 12, height / 2 - 12, 24, 24);
    }
    bmp
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StaticBitmap gallery")
        .with_size(560, 520)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("SVG bundle, SVGBitmap, MemoryDC art, HICON, set_bitmap/clear", 0);

    // ── (1) One SVG bundle, three sizes ──────────────────────────────
    let header_sizes = StaticText::new(&frame, "1. BitmapBundle from SVG at 16 / 32 / 64 px:");
    let bundle = BitmapBundle::from_svg_bytes(STAR_SVG, &[(16, 16), (32, 32), (64, 64)]);
    let star16 = StaticBitmap::new(&frame, &bundle, (16, 16));
    let star32 = StaticBitmap::new(&frame, &bundle, (32, 32));
    let star64 = StaticBitmap::new(&frame, &bundle, (64, 64));

    let mut row_sizes = BoxSizer::horizontal();
    row_sizes.add(star16.as_widget_ref());
    row_sizes.add_spacer(12);
    row_sizes.add(star32.as_widget_ref());
    row_sizes.add_spacer(12);
    row_sizes.add(star64.as_widget_ref());

    // ── (2) Coloured inline SVGs rasterised via SVGBitmap ────────────
    let header_svg = StaticText::new(&frame, "2. Coloured inline SVG glyphs (SVGBitmap, 32 px):");
    let mut row_svg = BoxSizer::horizontal();
    let mut svg_keep: Vec<SVGBitmap> = Vec::new();
    for bytes in [SVG_HEART, SVG_LEAF, SVG_BOLT] {
        let mut svg = SVGBitmap::new(32, 32);
        if svg.load_from_bytes(bytes) {
            if let Some(bmp) = svg.bitmap() {
                let ctrl = StaticBitmap::with_bitmap(&frame, bmp.handle(), 32, 32);
                row_svg.add(ctrl.as_widget_ref());
                row_svg.add_spacer(12);
            }
        }
        svg_keep.push(svg);
    }

    // ── (3) Procedural MemoryDC art + runtime swap ───────────────────
    let header_art = StaticText::new(&frame, "3. Procedural Bitmap (MemoryDC) — swap / clear at runtime:");
    let sunset = build_sunset(96, 96);
    let ocean = build_ocean(96, 96);
    let art = StaticBitmap::with_bitmap(&frame, sunset.handle(), 96, 96);

    let swap_btn = Button::new(&frame, "Swap image");
    let clear_btn = Button::new(&frame, "Clear");

    let showing_sunset = Rc::new(Cell::new(true));
    let art_for_swap = art.clone();
    let status_for_swap = status.clone();
    let flag = showing_sunset.clone();
    swap_btn.on_click(&frame, move || {
        let next_is_sunset = !flag.get();
        flag.set(next_is_sunset);
        let src = if next_is_sunset { &sunset } else { &ocean };
        art_for_swap.set_bitmap(RawBitmap {
            hbitmap: src.handle(),
            width: 96,
            height: 96,
        });
        status_for_swap.set_status_text(
            if next_is_sunset { "Showing: sunset (procedural)" } else { "Showing: ocean (procedural)" },
            0,
        );
    });

    let art_for_clear = art.clone();
    let status_for_clear = status.clone();
    clear_btn.on_click(&frame, move || {
        art_for_clear.clear();
        status_for_clear.set_status_text("Cleared — press Swap to restore", 0);
    });

    let mut row_art = BoxSizer::horizontal();
    row_art.add(art.as_widget_ref());
    row_art.add_spacer(12);
    row_art.add(swap_btn.as_widget_ref());
    row_art.add(clear_btn.as_widget_ref());

    // ── (4) HICON from inline SVG bytes ──────────────────────────────
    let header_icon = StaticText::new(&frame, "4. HICON from inline SVG (svg_bytes_to_hicon, 32 px):");
    let hicon = svg_bytes_to_hicon(SVG_INFO, 32).unwrap_or(std::ptr::null_mut());
    let from_icon = StaticBitmap::with_icon(&frame, hicon, (32, 32));

    // ── Layout ────────────────────────────────────────────────────────
    let mut sizer = BoxSizer::vertical();
    sizer.add(header_sizes.as_widget_ref());
    sizer.add_sizer(row_sizes);
    sizer.add_spacer(8);
    sizer.add(header_svg.as_widget_ref());
    sizer.add_sizer(row_svg);
    sizer.add_spacer(8);
    sizer.add(header_art.as_widget_ref());
    sizer.add_sizer(row_art);
    sizer.add_spacer(8);
    sizer.add(header_icon.as_widget_ref());
    sizer.add(from_icon.as_widget_ref());
    frame.set_sizer(sizer);

    // Keep the SVG rasterisations alive for the message loop.
    let _keep = (bundle, svg_keep);

    app.run(frame);
}
