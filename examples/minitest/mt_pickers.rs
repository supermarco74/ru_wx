//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: pickers — colour / date / font / file / dir.
//!
//! Demonstrates:
//! 1. [`ColourPickerCtrl`] with a live colour swatch (a procedural
//!    [`Bitmap`] painted via [`MemoryDC`] and shown in a
//!    `StaticBitmap`) plus a hex label, updated by `on_change`.
//! 2. [`DatePickerCtrl`] (calendar drop-down) with `on_date_change`
//!    and a spin-style sibling (`new_spin`).
//! 3. [`FontPickerCtrl`] with a live face/size label.
//! 4. [`FilePickerCtrl`] / [`DirPickerCtrl`] rows with both the
//!    path field and the browse button laid out.
//! 5. A `StatusBar` summary kept in sync from `on_idle`.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_pickers
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{
    App, Bitmap, BoxSizer, Brush, Colour, ColourPickerCtrl, Date, DatePickerCtrl, Dc,
    DirPickerCtrl, FilePickerCtrl, FontPickerCtrl, Frame, MemoryDC, Pen, RawBitmap, StaticBitmap,
    StaticText, StatusBar,
};

const SWATCH_W: i32 = 64;
const SWATCH_H: i32 = 24;

/// Paint a bordered colour swatch into a fresh bitmap. The
/// `StaticBitmap` clones the pixels, so the returned bitmap only
/// needs to outlive the `set_bitmap` call.
fn build_swatch(colour: Colour) -> Bitmap {
    let bmp = Bitmap::new(SWATCH_W as u32, SWATCH_H as u32);
    let border_pen = Pen::solid(Colour::new(60, 64, 72, 255));
    let fill_brush = Brush::solid(colour);
    {
        let mut mdc = MemoryDC::new();
        mdc.select_bitmap(&bmp);
        mdc.set_pen(Some(&border_pen));
        mdc.set_brush(Some(&fill_brush));
        mdc.draw_rect(0, 0, SWATCH_W, SWATCH_H);
    }
    bmp
}

fn hex(colour: Colour) -> String {
    format!("#{:02X}{:02X}{:02X}", colour.r, colour.g, colour.b)
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — pickers (colour / date / font / file / dir)")
        .with_size(640, 420)
        .build();

    // Field 0: last picker event. Field 1: live summary (from idle).
    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Pick something…", 0);

    // ── Colour picker + live swatch ───────────────────────────────
    let initial = Colour::new(58, 134, 200, 255);
    let colour_label = StaticText::new(&frame, "Colour:");
    let colour_picker = ColourPickerCtrl::with_colour(&frame, initial);
    let swatch = {
        let bmp = build_swatch(initial);
        StaticBitmap::with_bitmap(&frame, bmp.handle(), SWATCH_W as u32, SWATCH_H as u32)
    };
    let colour_value = StaticText::new(&frame, &hex(initial));

    let swatch_for_change = swatch.clone();
    let value_for_change = colour_value.clone();
    let status_for_colour = status.clone();
    colour_picker.on_change(&frame, move |c: Colour| {
        let bmp = build_swatch(c);
        swatch_for_change.set_bitmap(RawBitmap {
            hbitmap: bmp.handle(),
            width: SWATCH_W as u32,
            height: SWATCH_H as u32,
        });
        value_for_change.set_label(&hex(c));
        status_for_colour.set_status_text(&format!("Colour picked: {}", hex(c)), 0);
    });

    let mut row_colour = BoxSizer::horizontal();
    row_colour.add(colour_label.as_widget_ref());
    row_colour.add(colour_picker.as_widget_ref());
    row_colour.add_spacer(8);
    row_colour.add(swatch.as_widget_ref());
    row_colour.add_spacer(8);
    row_colour.add(colour_value.as_widget_ref());

    // ── Date pickers: calendar drop-down + spin style ─────────────
    let date_label = StaticText::new(&frame, "Date:");
    let date_picker = DatePickerCtrl::new(&frame);
    date_picker.set_value(Some(Date::new(2026, 6, 10)));
    let date_spin = DatePickerCtrl::new_spin(&frame);
    let date_value = StaticText::new(&frame, "2026-06-10");

    let value_for_date = date_value.clone();
    let status_for_date = status.clone();
    date_picker.on_date_change(&frame, move |d: Option<Date>| {
        let text = match d {
            Some(d) => format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
            None => String::from("(no date)"),
        };
        value_for_date.set_label(&text);
        status_for_date.set_status_text(&format!("Date picked: {text}"), 0);
    });

    let mut row_date = BoxSizer::horizontal();
    row_date.add(date_label.as_widget_ref());
    row_date.add(date_picker.as_widget_ref());
    row_date.add_spacer(8);
    row_date.add(date_spin.as_widget_ref());
    row_date.add_spacer(8);
    row_date.add(date_value.as_widget_ref());

    // ── Font picker + live description ────────────────────────────
    let font_label = StaticText::new(&frame, "Font:");
    let font_picker = FontPickerCtrl::new(&frame, &frame);
    let font_value = StaticText::new(&frame, "(default font)");

    let mut row_font = BoxSizer::horizontal();
    row_font.add(font_label.as_widget_ref());
    row_font.add(font_picker.as_widget_ref());
    row_font.add_spacer(8);
    row_font.add(font_value.as_widget_ref());

    // ── File / dir pickers: path field + browse button ────────────
    let file_label = StaticText::new(&frame, "File:");
    let file_picker = FilePickerCtrl::new(&frame, &frame);
    file_picker.set_wildcard("*.rs");
    let mut row_file = BoxSizer::horizontal();
    row_file.add(file_label.as_widget_ref());
    row_file.add_with_proportion(file_picker.path_widget(), 1);
    row_file.add(file_picker.browse_widget());

    let dir_label = StaticText::new(&frame, "Dir:");
    let dir_picker = DirPickerCtrl::new(&frame, &frame);
    let mut row_dir = BoxSizer::horizontal();
    row_dir.add(dir_label.as_widget_ref());
    row_dir.add_with_proportion(dir_picker.path_widget(), 1);
    row_dir.add(dir_picker.browse_widget());

    // ── Idle: keep the font label + status summary live ───────────
    // Cache the last strings so we only repaint on real changes.
    let font_for_idle = font_picker.clone();
    let font_value_for_idle = font_value.clone();
    let file_for_idle = file_picker.clone();
    let dir_for_idle = dir_picker.clone();
    let colour_for_idle = colour_picker.clone();
    let status_for_idle = status.clone();
    let last_font = std::cell::RefCell::new(String::new());
    let last_summary = std::cell::RefCell::new(String::new());
    frame.on_idle(move |_ev: &mut ru_wx::IdleEvent| {
        let font = font_for_idle.selected_font();
        let font_text = format!("{} {}pt", font.face_name, font.point_size);
        if *last_font.borrow() != font_text {
            font_value_for_idle.set_label(&font_text);
            *last_font.borrow_mut() = font_text;
        }
        let summary = format!(
            "colour={} file={} dir={}",
            hex(colour_for_idle.get_colour()),
            if file_for_idle.path().is_empty() { "(none)".into() } else { file_for_idle.path() },
            if dir_for_idle.path().is_empty() { "(none)".into() } else { dir_for_idle.path() },
        );
        if *last_summary.borrow() != summary {
            status_for_idle.set_status_text(&summary, 1);
            *last_summary.borrow_mut() = summary;
        }
    });

    // ── Layout ────────────────────────────────────────────────────
    let header = StaticText::new(
        &frame,
        "Colour / Date / Font / File / Dir pickers with live previews:",
    );
    let mut sizer = BoxSizer::vertical();
    sizer.add(header.as_widget_ref());
    sizer.add_spacer(4);
    sizer.add_sizer(row_colour);
    sizer.add_sizer(row_date);
    sizer.add_sizer(row_font);
    sizer.add_sizer(row_file);
    sizer.add_sizer(row_dir);
    frame.set_sizer(sizer);

    app.run(frame);
}
