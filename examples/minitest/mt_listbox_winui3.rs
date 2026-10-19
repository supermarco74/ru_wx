//! Minitest: ListBox variants comparison - WinUI3
//!
//! Demonstrates three ListBox implementations, laid out side-by-side
//! via `BoxSizer::horizontal`:
//!
//! 1. ListBox   - classic Win32 `LISTBOX`
//! 2. ListView  - `SysListView32` in report mode (columns)
//! 3. ListBoxUWP - UWP-style stub (backed by `LISTBOX`)
//!
//! Run with:
//! ```bash
//! cargo run --example mt_listbox_winui3 --features winui3
//! ```

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{App, BoxSizer, Frame, ListBox, ListBoxUWP, ListView};

fn main() {
    println!("=== ListBox WinUI3 Demo ===");

    let app = App::new();
    let frame = Frame::new("ListBox Variants - WinUI3", 980, 480);
    frame.center();

    // --- 1. ListBox ---
    let mut listbox = ListBox::new(&frame);
    for item in [
        "Apple",
        "Banana",
        "Cherry",
        "Date",
        "Elderberry",
        "Fig",
        "Grape",
    ] {
        listbox.add(item);
    }

    // --- 2. ListView in report mode ---
    let mut listview = ListView::new(&frame);
    listview.add_column("Name", 160);
    listview.add_column("Kind", 140);
    listview.add_column("Size", 100);
    for (n, k, s) in [
        ("main.rs", "Rust", "12 KB"),
        ("lib.rs", "Rust", "48 KB"),
        ("README.md", "Doc", " 4 KB"),
        ("Cargo.toml", "Config", " 2 KB"),
        ("advanced_ui_demo.rs", "Example", "18 KB"),
    ] {
        listview.add_item(n);
        listview.add_subitem(k, 1);
        listview.add_subitem(s, 2);
    }

    // --- 3. ListBoxUWP (stub) ---
    let mut listbox_uwp = ListBoxUWP::new(&frame);
    for item in ["One", "Two", "Three", "Four", "Five"] {
        listbox_uwp.add(item);
    }

    // Lay them out side-by-side via a horizontal BoxSizer.
    let mut sizer = BoxSizer::horizontal();
    sizer.add(listbox.hwnd());
    sizer.add(listview.hwnd());
    sizer.add(listbox_uwp.hwnd());
    sizer.layout(frame.hwnd());

    frame.show();

    println!("All three ListBox variants created with data and a sizer.");
    println!("Close window to exit.");

    app.run(&frame, || {});
}
