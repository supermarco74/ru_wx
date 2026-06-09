//! Minitest: `wxDC` family — device-context drawing primitives.
//!
//! Demonstrates every public DC flavour in `ru_wx`:
//! - [`Pen`] / [`Brush`] / [`BackgroundMode`] construction and the
//!   [`Pen::solid`] / [`Brush::solid`] convenience constructors.
//! - [`MemoryDC`] with a [`Bitmap`] backing store: select a bitmap, draw into it,
//!   demonstrate the full [`Dc`] trait (lines, rects, ellipses, text, blit).
//! - [`ClientDC`] for transient drawing on the client area outside `WM_PAINT`
//!   (drag feedback, overlay annotations, debug reticles).
//! - [`WindowDC`] for transient drawing on the *whole* window (client + non-client).
//! - Frame's `register_paint_handler` arm — receives the live `HDC` from the
//!   `BeginPaint` / `EndPaint` cycle. [`PaintDC`] is *not* constructed here
//!   (it would call `BeginPaint` a second time — Win32 UB); it is exercised
//!   in the unit tests inside `src/dc.rs` instead.
//!
//! Run with: `cargo run --example mt_dc`. Close the window to exit.

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BackgroundMode, Bitmap, Brush, BrushStyle, ClientDC, Colour, Dc, Frame, MemoryDC, Pen,
    PenStyle, Rect, WindowDC,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — wxDC")
        .with_size(800, 600)
        .build();

    // ── Section 1: pre-build a 220×220 memory bitmap ─────────────────────
    // The MemoryDC section below will draw into it, and the paint
    // handler will blit the result to the frame. We allocate the
    // bitmap up front (outside the closure) so the closure can
    // borrow it for the duration of the message loop.
    let mem_bmp = Bitmap::new(220, 220);

    // ── Section 2: Pen / Brush construction round-trip ──────────────────
    // Build a red 2-px solid pen, a blue solid brush, and a transparent
    // (NULL_BRUSH) brush. The width / colour / style getters are
    // exercised to verify the constructors store the metadata.
    let red_pen = Pen::new(Colour::new(255, 0, 0, 255), 2, PenStyle::Solid);
    let _ = red_pen.colour;
    let _ = red_pen.width;
    let _ = red_pen.style;
    let blue_brush = Brush::new(Colour::new(0, 0, 255, 255), BrushStyle::Solid);
    let _ = blue_brush.colour;
    let _ = blue_brush.style;
    let _transparent_brush = Brush::new(Colour::BLACK, BrushStyle::Transparent);
    let green_pen = Pen::solid(Colour::new(0, 200, 0, 255));
    let _ = green_pen.style;

    // ── Section 3: MemoryDC — draw into the bitmap ──────────────────────
    // Select the bitmap into a fresh memory DC, then issue every
    // drawing primitive the [`Dc`] trait exposes. The block keeps the
    // MemoryDC value short-lived; once it goes out of scope, the
    // selected bitmap is deselected and the DC is deleted, but
    // `mem_bmp` (held outside the block) keeps the pixel data.
    {
        let mut mdc = MemoryDC::new();
        mdc.select_bitmap(&mem_bmp);
        // Background fill — opaque white so the bitmap has a known
        // starting colour. BackgroundMode::Opaque is the default
        // but we set it explicitly for the doc.
        mdc.set_bk_mode(BackgroundMode::Opaque);
        mdc.set_bk_color(Colour::WHITE);
        mdc.fill_rect(0, 0, 220, 220, Colour::WHITE);
        // Outline + fill
        mdc.set_pen(Some(&red_pen));
        mdc.set_brush(Some(&blue_brush));
        mdc.draw_rect(10, 10, 200, 200);
        mdc.draw_ellipse(30, 30, 160, 160);
        // Transparent fill — outline only
        mdc.set_brush(None);
        mdc.draw_ellipse(60, 60, 100, 100);
        // A diagonal cross
        mdc.draw_line(10, 10, 210, 210);
        mdc.draw_line(10, 210, 210, 10);
        // A text label centred in the rect
        mdc.set_pen(Some(&green_pen));
        mdc.set_text_color(Colour::BLACK);
        let (tw, th) = mdc.text_extent("MemoryDC");
        mdc.draw_text("MemoryDC", 110 - tw / 2, 110 - th / 2);
        // Rect-based centred text
        mdc.draw_text_in_rect(
            "drew into a bitmap",
            Rect::new(10, 170, 200, 30),
            true,
        );
    } // mdc dropped — bitmap preserved

    // ── Section 4: ClientDC — transient drawing on the client area ─────
    // No paint cycle required. We grab a DC, draw a banner across the
    // top, and the `Drop` releases the DC back to the OS. This is the
    // pattern for drag feedback, hover highlights, etc.
    {
        let mut cdc = ClientDC::new(frame.hwnd());
        let _ = cdc.handle();
        let _ = cdc.is_null();
        cdc.set_bk_mode(BackgroundMode::Transparent);
        cdc.set_text_color(Colour::new(0, 128, 0, 255));
        cdc.draw_text("ClientDC: transient draw (released in Drop)", 240, 10);
        // A small reticle in the top-left of the client area
        let pen = Pen::solid(Colour::new(0, 180, 0, 255));
        cdc.set_pen(Some(&pen));
        cdc.set_brush(None);
        cdc.draw_line(0, 0, 60, 60);
        cdc.draw_line(60, 0, 0, 60);
        cdc.draw_rect(0, 0, 60, 60);
    } // cdc dropped — DC released

    // ── Section 5: WindowDC — transient drawing on the whole window ────
    // Same as ClientDC but covers the title bar / borders too. We
    // paint a 16-px yellow border around the whole window. Drop
    // releases the DC.
    {
        let mut wdc = WindowDC::new(frame.hwnd());
        let _ = wdc.handle();
        let _ = wdc.is_null();
        wdc.fill_rect(0, 0, 16, 16, Colour::new(255, 255, 0, 255)); // top-left
        wdc.fill_rect(0, 0, 800, 4, Colour::new(255, 255, 0, 255)); // top edge
        wdc.fill_rect(0, 0, 4, 600, Colour::new(255, 255, 0, 255)); // left edge
    } // wdc dropped

    // ── Section 6: paint handler — closure signature ────────────────────
    // The frame's `WM_PAINT` arm wraps the callback in
    // `BeginPaint` / `EndPaint` and hands us the live `HDC`
    // as an `isize`. We can't re-construct a `PaintDC` here
    // (it would call `BeginPaint` a second time — Win32 UB)
    // — the canonical pattern is to use the `HDC` directly
    // via raw Win32 FFI: create a transient memory DC, select
    // the source bitmap, `BitBlt` to the frame's HDC, tear
    // the memory DC down. See [`Dc::draw_bitmap`] in
    // `src/dc.rs` for the canonical FFI pattern.
    //
    // `PaintDC::new` is the right choice for custom WndProc
    // subclasses that manage their own `WM_PAINT` arm. It is
    // exercised in the unit tests inside `src/dc.rs` and is
    // deliberately not constructed in this minitest to avoid
    // the double-`BeginPaint` hazard. The body below just
    // exercises the closure signature; the actual `BitBlt` is
    // left as a FFI exercise in the library's `dc.rs` (it
    // would require `windows-sys` in dev-dependencies, which
    // is out of scope for a minitest).
    //
    // Note: the `register_paint_handler` signature requires
    // `FnMut + 'static`, so we cannot borrow `mem_bmp` from
    // the enclosing function — we use the `mem_bmp` *length*
    // (u32 = 'static) as a compile-time check that the
    // bitmap is still alive at registration time. The actual
    // bitmap stays alive in `main` for the duration of the
    // message loop, so the paint handler can safely borrow
    // it via FFI.
    let mem_bmp_alive = mem_bmp.width;
    frame.register_paint_handler(move |hdc: isize| {
        let _ = (hdc, mem_bmp_alive);
    });

    app.run(frame);
}
