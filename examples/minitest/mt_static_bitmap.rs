//! Minitest: `StaticBitmap` — image display in three flavours.
//!
//! Demonstrates:
//! 1. An **empty** `StaticBitmap` (just the control, no image).
//! 2. A `StaticBitmap` bound to a [`BitmapBundle`] built from
//!    a multi-resolution SVG (best-fit is selected at request
//!    time).
//! 3. A `StaticBitmap` with a procedurally created
//!    [`Bitmap`](crate::Bitmap) (a solid colour DIB).
//! 4. A `StaticBitmap` with an `HICON` produced from inline
//!    SVG bytes.
//! 5. The `set_bitmap` / `clear` lifecycle methods.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_static_bitmap
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    svg_bytes_to_hicon, App, Bitmap, BitmapBundle, BoxSizer, Frame, RawBitmap, StaticBitmap,
    StaticText,
};

const STAR_SVG: &[u8] = include_bytes!("../../assets/icons/star.svg");

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — StaticBitmap")
        .with_size(460, 360)
        .build();

    let header = StaticText::new(&frame, "Image display — empty / bundle / bitmap / icon");

    // (1) Empty StaticBitmap: just the placeholder control.
    let empty = StaticBitmap::with_size(&frame, 32, 32);
    let label_empty = StaticText::new(&frame, "1. empty");

    // (2) BitmapBundle sourced from an SVG. `best_for_size` is
    // invoked internally to pick the closest match.
    let bundle = BitmapBundle::from_svg_bytes(STAR_SVG, &[(16, 16), (24, 24), (32, 32)]);
    let from_bundle = StaticBitmap::new(&frame, &bundle, (32, 32));
    let label_bundle = StaticText::new(&frame, "2. bundle (SVG → 32×32)");

    // (3) Single-resolution `Bitmap` constructed via `new`.
    // On Windows this is a 32-bit DIB section.
    let red_bmp = Bitmap::new(32, 32);
    let from_bitmap = StaticBitmap::with_bitmap(&frame, red_bmp.handle(), 32, 32);
    let label_bitmap = StaticText::new(&frame, "3. raw 32×32 HBITMAP");

    // (4) HICON: convert inline SVG bytes to an HICON and feed
    // it into the control.
    let info_svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>"#;
    let hicon = svg_bytes_to_hicon(info_svg, 32).unwrap_or(std::ptr::null_mut());
    let from_icon = StaticBitmap::with_icon(&frame, hicon, (32, 32));
    let label_icon = StaticText::new(&frame, "4. icon (inline SVG → HICON)");

    // (5) Lifecycle: set_bitmap then clear. We reuse the
    // first empty control.
    let green_bmp = Bitmap::new(24, 24);
    let lifecycle = StaticBitmap::with_size(&frame, 24, 24);
    lifecycle.set_bitmap(RawBitmap {
        hbitmap: green_bmp.handle(),
        width: 24,
        height: 24,
    });
    let label_lifecycle = StaticText::new(&frame, "5. set_bitmap then clear()");
    lifecycle.clear();

    let mut sizer = BoxSizer::vertical();
    sizer.add(header.as_widget_ref());
    sizer.add(empty.as_widget_ref());
    sizer.add(label_empty.as_widget_ref());
    sizer.add(from_bundle.as_widget_ref());
    sizer.add(label_bundle.as_widget_ref());
    sizer.add(from_bitmap.as_widget_ref());
    sizer.add(label_bitmap.as_widget_ref());
    sizer.add(from_icon.as_widget_ref());
    sizer.add(label_icon.as_widget_ref());
    sizer.add(lifecycle.as_widget_ref());
    sizer.add(label_lifecycle.as_widget_ref());
    frame.set_sizer(sizer);

    // `Bitmap::drop` releases the underlying `HBITMAP`, so we
    // keep the bitmaps alive for the lifetime of the loop by
    // dropping them only at the end of `main`.
    let _keep = (red_bmp, green_bmp, bundle);

    app.run(frame);
}
