//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `BitmapComboBox` — a drop-down where each row carries a small icon.
//!
//! Run with:
//! ```bash
//! cargo run --example mt_bitmap_combo
//! ```
//!
//! The control is a thin Win32 `WC_COMBOBOXEX` (the `ComboBoxEx32` window
//! class) with an attached image list. Each row in the list is drawn as
//! `[icon | text]`. This is the closest Win32 equivalent of
//! `wxBitmapComboBox`.

#![windows_subsystem = "windows"]

use ru_wx::{
    App, BitmapComboBox, BoxSizer, Frame, ImageList, StaticText, StatusBar,
};
use ru_wx::dc::icon;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — BitmapComboBox")
        .with_size(460, 320)
        .with_modern_style().build();

    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Pick an action.", 0);

    // Build a 16x16 image list. 16 is the conventional size for
    // menu / toolbar icons on Windows at the default 96 DPI.
    let icons = ImageList::new(16, 16);

    // Load four SVG icons from the assets directory. `load_svg_as_hbitmap`
    // returns an `HBITMAP` that can be handed straight to the image list.
    let icon_files = [
        "assets/icons/file-new.svg",
        "assets/icons/folder-open.svg",
        "assets/icons/info.svg",
        "assets/icons/exit.svg",
    ];
    for path in &icon_files {
        if let Some(hbmp) = icon::load_svg_as_hbitmap(std::path::Path::new(path), 16, 16) {
            icons.add_bitmap(hbmp);
        }
    }

    // --- BitmapComboBox -------------------------------------------------
    let lbl = StaticText::new(&frame, "Pick a file action:");
    let bcb = BitmapComboBox::new(&frame);
    bcb.set_image_list(&icons);

    // Populate with `[icon, text]` rows. The image indices are 0-based
    // and match the order we added them to the image list above.
    bcb.append_with_image("New file", 0);
    bcb.append_with_image("Open folder", 1);
    bcb.append_with_image("Show info", 2);
    bcb.append_with_image("Exit", 3);
    bcb.append_with_image("No icon (plain text row)", -1);
    bcb.set_selection(0);

    // Fire a callback when the user picks a different row.
    let bcb_for_cb = bcb.clone();
    let s = status.clone();
    bcb.on_selection_change(&frame, move || {
        if let Some(i) = bcb_for_cb.get_selection() {
            let label = bcb_for_cb.get_string(i);
            s.set_status_text(&format!("BitmapComboBox → {label} (idx {i})"), 0);
        }
    });

    // --- A plain ComboBox for comparison --------------------------------
    let lbl2 = StaticText::new(&frame, "Plain ComboBox (no images) for comparison:");
    let plain = ru_wx::ComboBox::new(&frame);
    for c in ["Red", "Green", "Blue"] {
        plain.append(c);
    }

    let mut sizer = BoxSizer::vertical();
    sizer.add(lbl.as_widget_ref());
    sizer.add(bcb.as_widget_ref());
    sizer.add(lbl2.as_widget_ref());
    sizer.add(plain.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
