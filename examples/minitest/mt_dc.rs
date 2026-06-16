//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `wxDC` family — a full painted scene.
//!
//! Demonstrates:
//! - [`MemoryDC`] + [`Bitmap`]: a bullseye "badge" pre-rendered into
//!   an off-screen bitmap, blitted on every repaint with
//!   [`Dc::draw_bitmap`].
//! - The frame's `register_paint_handler` arm used to paint a real
//!   scene on every `WM_PAINT`: title banner, grid, a coloured bar
//!   chart with value labels, axis lines and a legend.
//! - [`Pen`] / [`Brush`] / [`BackgroundMode`] state handling and the
//!   `Pen::solid` / `Brush::solid` convenience constructors.
//! - `text_extent` / `draw_text` / `draw_text_in_rect` for measured
//!   and rect-centred text.
//!
//! Run with: `cargo run --example mt_dc`. Resize the window — the
//! chart re-lays itself out on every repaint. Close it to exit.

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BackgroundMode, Bitmap, Brush, ClientDC, Colour, Dc, Frame, MemoryDC, Pen, PenStyle,
    Rect,
};

/// Pre-render a bullseye badge into an off-screen bitmap with a
/// [`MemoryDC`]. The bitmap is later blitted to the frame on every
/// repaint via [`Dc::draw_bitmap`].
fn build_badge(size: i32) -> Bitmap {
    let bmp = Bitmap::new(size as u32, size as u32);
    // Pens / brushes are created BEFORE the DC so the DC drops first
    // (restoring stock objects) and the GDI handles delete cleanly.
    let rim_pen = Pen::new(Colour::new(40, 40, 48, 255), 2, PenStyle::Solid);
    let red_brush = Brush::solid(Colour::new(214, 64, 56, 255));
    let white_brush = Brush::solid(Colour::WHITE);
    let gold_brush = Brush::solid(Colour::new(240, 184, 48, 255));
    {
        let mut mdc = MemoryDC::new();
        mdc.select_bitmap(&bmp);
        mdc.fill_rect(0, 0, size, size, Colour::new(245, 246, 250, 255));
        mdc.set_pen(Some(&rim_pen));
        // Three concentric filled rings.
        mdc.set_brush(Some(&red_brush));
        mdc.draw_ellipse(4, 4, size - 8, size - 8);
        mdc.set_brush(Some(&white_brush));
        mdc.draw_ellipse(size / 2 - 38, size / 2 - 38, 76, 76);
        mdc.set_brush(Some(&gold_brush));
        mdc.draw_ellipse(size / 2 - 18, size / 2 - 18, 36, 36);
        // Caption centred near the bottom edge.
        mdc.set_bk_mode(BackgroundMode::Transparent);
        mdc.set_text_color(Colour::new(40, 40, 48, 255));
        mdc.draw_text_in_rect("MemoryDC", Rect::new(0, size - 24, size as u32, 20), true);
    }
    bmp
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — wxDC painted scene")
        .with_size(820, 560)
        .build();

    let badge = build_badge(140);
    let hwnd = frame.hwnd();

    // The frame hands us the live WM_PAINT HDC as an `isize`; the
    // safe, library-level way to draw is a fresh ClientDC over the
    // same window (BeginPaint already validated the update region).
    frame.register_paint_handler(move |_hdc: isize| {
        // Chart data: (label, value, colour).
        let data: [(&str, i32, Colour); 6] = [
            ("Jan", 34, Colour::new(214, 64, 56, 255)),
            ("Feb", 58, Colour::new(240, 144, 40, 255)),
            ("Mar", 72, Colour::new(240, 184, 48, 255)),
            ("Apr", 49, Colour::new(80, 164, 100, 255)),
            ("May", 91, Colour::new(58, 134, 200, 255)),
            ("Jun", 65, Colour::new(130, 90, 190, 255)),
        ];
        let max_value = 100;

        let grid_pen = Pen::solid(Colour::new(214, 218, 228, 255));
        let axis_pen = Pen::new(Colour::new(60, 64, 72, 255), 2, PenStyle::Solid);

        let mut dc = ClientDC::new(hwnd);
        let cw = dc.client_width();
        let ch = dc.client_height();

        // ── Title banner ──────────────────────────────────────────
        dc.fill_rect(0, 0, cw, 56, Colour::new(28, 40, 70, 255));
        dc.set_bk_mode(BackgroundMode::Transparent);
        dc.set_text_color(Colour::WHITE);
        let title = "GDI scene: bar chart + grid + MemoryDC badge";
        let (tw, th) = dc.text_extent(title);
        dc.draw_text(title, (cw - tw) / 2, (56 - th) / 2);

        // ── Chart geometry ────────────────────────────────────────
        let chart_left = 50;
        let chart_top = 90;
        let chart_bottom = ch - 70;
        let chart_right = cw - 200;
        let chart_h = (chart_bottom - chart_top).max(40);
        let chart_w = (chart_right - chart_left).max(120);

        // Horizontal grid lines every 20 units, with scale labels.
        dc.set_text_color(Colour::new(110, 114, 124, 255));
        dc.set_pen(Some(&grid_pen));
        let mut level = 0;
        while level <= max_value {
            let y = chart_bottom - level * chart_h / max_value;
            dc.draw_line(chart_left, y, chart_left + chart_w, y);
            let label = format!("{level}");
            let (lw, lh) = dc.text_extent(&label);
            dc.draw_text(&label, chart_left - lw - 6, y - lh / 2);
            level += 20;
        }

        // ── Bars + value labels + month labels ────────────────────
        let slot = chart_w / data.len() as i32;
        let bar_w = slot * 6 / 10;
        for (i, (label, value, colour)) in data.iter().enumerate() {
            let x = chart_left + i as i32 * slot + (slot - bar_w) / 2;
            let bar_h = value * chart_h / max_value;
            let y = chart_bottom - bar_h;
            dc.fill_rect(x, y, bar_w, bar_h, *colour);
            // Value above the bar.
            dc.set_text_color(Colour::new(40, 44, 52, 255));
            let v = format!("{value}");
            let (vw, vh) = dc.text_extent(&v);
            dc.draw_text(&v, x + (bar_w - vw) / 2, y - vh - 2);
            // Month centred below the baseline.
            dc.draw_text_in_rect(
                label,
                Rect::new(x, chart_bottom + 4, bar_w as u32, 20),
                true,
            );
        }

        // ── Axes drawn on top of the grid ─────────────────────────
        dc.set_pen(Some(&axis_pen));
        dc.draw_line(chart_left, chart_top - 10, chart_left, chart_bottom);
        dc.draw_line(chart_left, chart_bottom, chart_left + chart_w, chart_bottom);

        // ── Legend: colour swatches + labels ──────────────────────
        let legend_x = cw - 180;
        let mut legend_y = 90;
        dc.set_text_color(Colour::new(40, 44, 52, 255));
        dc.draw_text("Legend", legend_x, legend_y);
        legend_y += 22;
        for (label, _value, colour) in data.iter() {
            dc.fill_rect(legend_x, legend_y, 14, 14, *colour);
            dc.set_pen(Some(&grid_pen));
            dc.set_brush(None);
            dc.draw_rect(legend_x, legend_y, 14, 14);
            dc.draw_text(label, legend_x + 20, legend_y);
            legend_y += 20;
        }

        // ── Blit the pre-rendered MemoryDC badge ──────────────────
        dc.draw_bitmap(&badge, cw - 180, legend_y + 10);
    });

    app.run(frame);
}
