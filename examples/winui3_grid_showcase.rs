//! WinUI3 Grid showcase — demonstrates the full Grid API surface
//! with Cell types, value providers, styling, sorting, and selection.
//!
//! Run with:
//! ```bash
//! cargo run --release --example winui3_grid_showcase --features winui3
//! ```

#![windows_subsystem = "windows"]

use ru_wx::platform::winui3::{
    set_frame_icon, svg_to_hicon, App, BoxSizer, ColumnAlign, Frame, Grid, Menu, MenuBar, Splitter,
    StatusBar, TreeCtrl,
};
use ru_wx::{Cell, Colour, GridCellStyle};

/// Product data for the grid.
struct Product {
    id: u32,
    name: &'static str,
    category: &'static str,
    price: f64,
    stock: u32,
    rating: u8,
}

const PRODUCTS: &[Product] = &[
    Product {
        id: 1,
        name: "ru_wx crate",
        category: "Rust",
        price: 0.00,
        stock: 999,
        rating: 5,
    },
    Product {
        id: 2,
        name: "windows-sys",
        category: "Rust",
        price: 0.00,
        stock: 999,
        rating: 4,
    },
    Product {
        id: 3,
        name: "resvg",
        category: "Rust",
        price: 0.00,
        stock: 999,
        rating: 5,
    },
    Product {
        id: 4,
        name: "image",
        category: "Rust",
        price: 0.00,
        stock: 999,
        rating: 4,
    },
    Product {
        id: 5,
        name: "serde",
        category: "Rust",
        price: 0.00,
        stock: 999,
        rating: 5,
    },
    Product {
        id: 6,
        name: "human-panic",
        category: "Rust",
        price: 0.00,
        stock: 999,
        rating: 3,
    },
    Product {
        id: 7,
        name: "obj2-app-kit",
        category: "macOS",
        price: 0.00,
        stock: 999,
        rating: 4,
    },
    Product {
        id: 8,
        name: "objc2-web-kit",
        category: "macOS",
        price: 0.00,
        stock: 999,
        rating: 4,
    },
    Product {
        id: 9,
        name: "gtk4-rs",
        category: "Linux",
        price: 0.00,
        stock: 999,
        rating: 4,
    },
    Product {
        id: 10,
        name: "winit",
        category: "Cross",
        price: 0.00,
        stock: 999,
        rating: 5,
    },
];

fn main() {
    let app = App::new();
    let mut frame = Frame::new("WinUI3 Grid Showcase", 1100, 650);
    frame.center();
    frame.show();

    // Icon.
    let svg: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><rect x='4' y='4' width='24' height='24' rx='4' fill='%234f46e5'/><text x='16' y='22' text-anchor='middle' fill='white' font-size='14'>G</text></svg>";
    if let Some(h) = svg_to_hicon(svg, 32) {
        set_frame_icon(frame.hwnd(), h);
    }

    // Menu.
    let mut mb = MenuBar::new();
    let mut file_menu = Menu::new();
    file_menu.append_text(1001, "Sort by Name");
    file_menu.append_text(1002, "Sort by Price");
    file_menu.append_text(1003, "Sort by Stock");
    file_menu.append_separator();
    file_menu.append_text(1004, "Refresh");
    mb.append("&Grid", file_menu);
    frame.set_menu_bar(mb);

    // Tree on the left.
    let mut tree = TreeCtrl::new(&frame);
    let root = tree.add_root("Products");
    let rust = tree.append_item(root, "Rust crates");
    tree.append_item(rust, "ru_wx");
    tree.append_item(rust, "windows-sys");
    tree.append_item(rust, "resvg");
    let mac = tree.append_item(root, "macOS crates");
    tree.append_item(mac, "objc2-app-kit");
    tree.select_item(root);

    // Grid in the center.
    let grid = Grid::new(&frame);
    grid.set_checkboxes(true);

    // Add columns.
    grid.append_column_with_align("ID", 50, ColumnAlign::Right);
    grid.append_column_with_align("Name", 200, ColumnAlign::Left);
    grid.append_column_with_align("Category", 100, ColumnAlign::Left);
    grid.append_column_with_align("Price", 100, ColumnAlign::Right);
    grid.append_column_with_align("Stock", 80, ColumnAlign::Right);
    grid.append_column_with_align("Rating", 80, ColumnAlign::Center);

    // Set up value provider using Cell types.
    grid.set_row_count(PRODUCTS.len());
    grid.set_value_provider(move |row, col| {
        let p = &PRODUCTS[row.min(PRODUCTS.len() - 1)];
        match col {
            0 => Cell::Text(format!("{}", p.id)),
            1 => Cell::Text(p.name.to_string()),
            2 => Cell::Text(p.category.to_string()),
            3 => Cell::Text(format!("€{:.2}", p.price)),
            4 => Cell::Text(format!("{} units", p.stock)),
            5 => Cell::Text(format!("{}/5", p.rating)),
            _ => Cell::Empty,
        }
    });

    // Styling.
    grid.set_grid_line_color(Colour::new(230, 230, 230, 255));
    grid.set_header_bg_color(Colour::new(245, 245, 250, 255));
    grid.set_header_fg_color(Colour::new(30, 30, 60, 255));

    // Alternating row colors (API takes even/odd Colour, no enable flag).
    grid.set_alternating_row_colors(
        Colour::new(255, 255, 255, 255),
        Colour::new(245, 245, 250, 255),
    );

    // Highlight stock < 10 in red.
    grid.set_cell_style(
        4,
        4,
        GridCellStyle {
            foreground: Some(Colour::new(220, 50, 50, 255)),
            background: None,
        },
    );

    // Apply Win11 theme.
    grid.apply_win11_theme(&frame);

    // Interactive features.
    grid.enable_interactive_features(&frame);
    grid.enable_column_context_menu_with_frame(&frame);

    // Selection handler.
    grid.on_selection_changed(&frame, |sel| {
        if let Some(row) = sel {
            println!("Selected row: {}", row);
        }
    });

    // Splitter between tree and grid.
    let splitter = Splitter::new(&frame);

    // Status bar.
    let mut status = StatusBar::new(&frame);
    status.set_part_count(3);
    status.set_part_text(0, &format!("{} products", PRODUCTS.len()));
    status.set_part_text(1, "No selection");
    status.set_part_text(2, "WinUI3 Grid");

    // Layout: [tree | splitter | grid].
    let mut sizer = BoxSizer::horizontal();
    sizer.add(tree.hwnd());
    sizer.add(splitter.hwnd());
    sizer.add(grid.hwnd());
    sizer.layout(frame.hwnd());

    // Menu handlers.
    frame.on_command(1001, move |_| {
        println!("Sort by Name clicked");
    });
    frame.on_command(1002, move |_| {
        println!("Sort by Price clicked");
    });
    frame.on_command(1003, move |_| {
        println!("Sort by Stock clicked");
    });
    frame.on_command(1004, move |_| {
        println!("Refresh clicked");
    });

    frame.set_on_close(|| println!("Closing..."));

    println!("Grid showcase ready. {} products loaded.", PRODUCTS.len());
    app.run(&frame, || {});
}
