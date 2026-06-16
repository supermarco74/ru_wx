//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `SearchCtrl` — live filtering of a `ListBox`.
//!
//! Demonstrates:
//! - `SearchCtrl` with cue banner and `on_search` callback
//! - Live case-insensitive filtering of a `ListBox` catalogue
//! - Match counter in a two-field `StatusBar`
//! - Clear-search button and selection feedback from the list
//!
//! Run with:
//! ```bash
//! cargo run --example mt_search
//! ```

#![windows_subsystem = "windows"]

use ru_wx::{App, BoxSizer, Button, Frame, ListBox, SearchCtrl, StaticText, StatusBar, ToolTip};

const PRODUCTS: [&str; 12] = [
    "Mechanical keyboard",
    "Wireless mouse",
    "USB-C hub",
    "27\" monitor",
    "Laptop stand",
    "Webcam 1080p",
    "Noise-cancelling headset",
    "External SSD 1TB",
    "Ergonomic chair",
    "Desk lamp",
    "Microphone arm",
    "Graphics tablet",
];

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — SearchCtrl + ListBox filter")
        .with_size(500, 460)
        .build();

    // Field 0 = messages / selection, field 1 = match counter.
    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Type to filter the product list.", 0);

    let hint = StaticText::new(&frame, "Search filters the catalogue below (case-insensitive):");

    let search = SearchCtrl::new(&frame, "Search products…");
    ToolTip::new("Live filter — every keystroke updates the list").attach(&search.as_widget_ref());

    let list = ListBox::new(&frame);

    // Repopulate the listbox with the products matching `query`.
    let refilter = {
        let list = list.clone();
        let status = status.clone();
        move |query: &str| {
            let q = query.to_lowercase();
            list.clear();
            let mut matches = 0usize;
            for p in PRODUCTS {
                if q.is_empty() || p.to_lowercase().contains(&q) {
                    list.append(p);
                    matches += 1;
                }
            }
            status.set_status_text(
                &format!("{matches} / {} match(es)", PRODUCTS.len()),
                1,
            );
        }
    };
    refilter(""); // show the whole catalogue at start

    {
        let refilter = refilter.clone();
        let s = status.clone();
        search.on_search(&frame, move |text| {
            refilter(&text);
            if text.is_empty() {
                s.set_status_text("Filter cleared — full catalogue shown.", 0);
            } else {
                s.set_status_text(&format!("Filtering by '{text}'"), 0);
            }
        });
    }

    // Selecting a filtered item reports it in the status bar.
    {
        let list_for_sel = list.clone();
        let s = status.clone();
        list.on_selection_change(&frame, move || {
            if let Some(i) = list_for_sel.get_selection() {
                if let Some(item) = list_for_sel.get_string(i) {
                    s.set_status_text(&format!("Selected: {item}"), 0);
                }
            }
        });
    }

    // Clear button: resets the search field and the filter.
    let btn_clear = Button::new(&frame, "Clear search");
    {
        let search = search.clone();
        let refilter = refilter.clone();
        let s = status.clone();
        btn_clear.on_click(&frame, move || {
            search.clear();
            refilter("");
            s.set_status_text("Search cleared.", 0);
        });
    }

    // Layout: search field and clear button share a horizontal row,
    // the list takes the remaining vertical space.
    let mut row = BoxSizer::horizontal();
    row.add_with_proportion(search.as_widget_ref(), 1);
    row.add(btn_clear.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add_sizer(row);
    sizer.add_with_proportion(list.as_widget_ref(), 1);
    frame.set_sizer(sizer);

    app.run(frame);
}
