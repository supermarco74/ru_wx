//! Custom table grid — complete wxPython `CustTableGrid` + mixin port.
//!
//! **Inventory tab** — full-featured grid:
//! - SVG icons (`GridIcons`, dedicated Type column, Lucide ↔ Bootstrap)
//! - 3 label renderers (`GridWithLabelRenderersMixin`)
//! - Advanced sorting (every column ↑/↓, clear, data-level sort)
//! - Column context menu (right-click header / body → add/remove columns)
//! - Row context menu (toggle check, duplicate, delete, highlight, tier)
//! - Checkboxes, themes, filters, highlight styles, export checked
//!
//! **String table** + **Function table** tabs for `GridStringTable` / `FunctionGridTable`.
//!
//! ```bash
//! cargo run --example cust_table_grid
//! ```

#![windows_subsystem = "windows"]

use std::cell::{Cell as StdCell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

use ru_wx::{
    App, Appearance, BadgeKind, BarStyle, BoxSizer, Button, Cell, Colour, ColumnAlign,
    FontDesc, Frame, FunctionGridTable, Grid, GridAppearance, GridCellAttr, GridCellBoolRenderer,
    GridCellStringRenderer, GridCellStyle, GridDateFormat, GridIcons, GridStringTable, GridTable,
    Menu, MenuBar, MessageBoxIcon, MessageDialog, MessageDialogStyle, NumberFormat, Panel,
    PopupMenu, PriorityKind, SearchCtrl, SortOrder, StaticText, StatusBar, Tab,
};

/// Grid column indices (display order).
mod grid_col {
    pub const ROW: usize = 0;
    pub const TYPE: usize = 1;
    pub const PRODUCT: usize = 2;
    pub const SKU: usize = 3;
    pub const CATEGORY: usize = 4;
    pub const PRICE: usize = 5;
    pub const STOCK: usize = 6;
    pub const STATUS: usize = 7;
    pub const DISC: usize = 8;
    pub const SALES: usize = 9;
    pub const LISTED: usize = 10;
    pub const URL: usize = 11;
    pub const RATE: usize = 12;
    pub const PRIO: usize = 13;
    pub const WEIGHT: usize = 14;
    pub const COUNT: usize = 15;
}

// ═══════════════════════════════════════════════════════════════════════
// Data model
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowTier {
    Standard,
    Featured,
    Vip,
    Archived,
}

#[derive(Clone, Debug)]
struct InventoryRow {
    sku: String,
    name: String,
    category: String,
    qty: u32,
    max_qty: u32,
    icon: String,
    tier: RowTier,
    discount_pct: u32,
    price: f64,
    sales: f64,
    listed_date: String,
    url: String,
    rating: u32,
    max_rating: u32,
    priority: PriorityKind,
    weight: String,
    description: String,
    is_popular: bool,
}

const DATA_COLS: usize = 13;

/// Map on-screen column → index inside [`GridTable`] (None = synthetic cell).
fn data_col_for_grid_col(col: usize) -> Option<usize> {
    use grid_col::*;
    match col {
        SKU => Some(0),
        PRODUCT => Some(1),
        CATEGORY => Some(2),
        PRICE => Some(3),
        STOCK => Some(4),
        STATUS => Some(5),
        DISC => Some(6),
        SALES => Some(7),
        LISTED => Some(8),
        URL => Some(9),
        RATE => Some(10),
        PRIO => Some(11),
        WEIGHT => Some(12),
        _ => None,
    }
}

fn row(args: RowArgs) -> InventoryRow {
    InventoryRow {
        sku: args.sku.into(),
        name: args.name.into(),
        category: args.category.into(),
        qty: args.qty,
        max_qty: args.max_qty,
        icon: args.icon.into(),
        tier: args.tier,
        discount_pct: args.discount_pct,
        price: args.price,
        sales: args.sales,
        listed_date: args.listed_date.into(),
        url: args.url.into(),
        rating: args.rating,
        max_rating: args.max_rating,
        priority: args.priority,
        weight: args.weight.into(),
        description: args.description.into(),
        is_popular: args.is_popular,
    }
}

struct RowArgs {
    sku: &'static str,
    name: &'static str,
    category: &'static str,
    qty: u32,
    max_qty: u32,
    icon: &'static str,
    tier: RowTier,
    discount_pct: u32,
    price: f64,
    sales: f64,
    listed_date: &'static str,
    url: &'static str,
    rating: u32,
    max_rating: u32,
    priority: PriorityKind,
    weight: &'static str,
    description: &'static str,
    is_popular: bool,
}

macro_rules! inv {
    ($sku:expr, $name:expr, $cat:expr, $qty:expr, $max:expr, $icon:expr, $tier:expr, $disc:expr, $price:expr, $sales:expr, $date:expr, $url:expr, $rate:expr, $maxrate:expr, $prio:expr, $wt:expr, $desc:expr, $pop:expr) => {
        row(RowArgs {
            sku: $sku,
            name: $name,
            category: $cat,
            qty: $qty,
            max_qty: $max,
            icon: $icon,
            tier: $tier,
            discount_pct: $disc,
            price: $price,
            sales: $sales,
            listed_date: $date,
            url: $url,
            rating: $rate,
            max_rating: $maxrate,
            priority: $prio,
            weight: $wt,
            description: $desc,
            is_popular: $pop,
        })
    };
}

#[derive(Debug, Default)]
struct CustTableGridTable {
    rows: Vec<InventoryRow>,
    /// Visible row indices after tier + search filters.
    display_rows: Vec<usize>,
    tier_filter: Option<RowTier>,
    search_query: String,
}

impl CustTableGridTable {
    fn rebuild_display_rows(&mut self) {
        let q = self.search_query.trim().to_lowercase();
        self.display_rows = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                self.tier_filter.is_none_or(|t| r.tier == t)
                    && (q.is_empty()
                        || r.sku.to_lowercase().contains(&q)
                        || r.name.to_lowercase().contains(&q)
                        || r.category.to_lowercase().contains(&q)
                        || r.description.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect();
    }

    fn set_filter(&mut self, tier: Option<RowTier>) {
        self.tier_filter = tier;
        self.rebuild_display_rows();
    }

    fn set_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.rebuild_display_rows();
    }

    fn total_rows(&self) -> usize {
        self.rows.len()
    }

    fn filter_summary(&self) -> String {
        let tier = match self.tier_filter {
            None => "all tiers",
            Some(RowTier::Vip) => "VIP",
            Some(RowTier::Featured) => "Featured",
            Some(RowTier::Standard) => "Standard",
            Some(RowTier::Archived) => "Archived",
        };
        if self.search_query.trim().is_empty() {
            format!("Filter: {tier}")
        } else {
            format!("Filter: {tier} · search \"{}\"", self.search_query.trim())
        }
    }
    fn sample_inventory() -> Self {
        let rows = vec![
            inv!("SKU-1001", "Espresso Machine", "Kitchen", 12, 20, "cart", RowTier::Vip, 15, 599.99, 4823.0, "2024-03-15", "https://shop.example.com/espresso", 5, 5, PriorityKind::High, "12 kg", "Premium\nbean-to-cup\nespresso", true),
            inv!("SKU-2044", "Project Notebook", "Stationery", 250, 300, "book", RowTier::Standard, 0, 14.50, 18120.0, "2023-09-01", "https://shop.example.com/notebook", 4, 5, PriorityKind::Low, "350 g", "Hardcover\ndotted\n192 pages", false),
            inv!("SKU-3300", "Code Repository Pro", "Software", 999, 999, "software", RowTier::Featured, 20, 49.00, 23987.0, "2022-11-20", "https://code.example.com/pro", 5, 5, PriorityKind::Critical, "—", "Git hosting\nwith CI/CD", true),
            inv!("SKU-4102", "Mechanical Keyboard", "Peripherals", 18, 30, "box", RowTier::Featured, 5, 129.00, 764.0, "2024-05-22", "https://shop.example.com/kbd", 4, 5, PriorityKind::Medium, "980 g", "Hot-swap\nRGB switches", true),
            inv!("SKU-5099", "Cloud Backup Plan", "Service", 0, 0, "cloud", RowTier::Standard, 0, 9.99, 5412.0, "2024-08-14", "https://cloud.example.com/plan", 3, 5, PriorityKind::Medium, "1 TB", "1 TB\nencrypted\nauto-sync", false),
            inv!("SKU-7781", "Discontinued Mug", "Archive", 0, 0, "archived", RowTier::Archived, 100, 29.99, 0.0, "2019-04-10", "https://archive.example.com/old", 1, 5, PriorityKind::None, "n/a", "Last-chance\nclearance", false),
            inv!("SKU-8800", "Design System Kit", "Software", 43, 100, "sparkles", RowTier::Vip, 10, 89.00, 1209.0, "2024-01-12", "https://figma.example.com/dsk", 5, 5, PriorityKind::High, "1.2 GB", "500+\ncomponents\nFigma", true),
            inv!("SKU-9012", "Conference Ticket", "Events", 80, 100, "trophy", RowTier::Featured, 0, 349.00, 150.0, "2025-09-30", "https://events.example.com/2025", 5, 5, PriorityKind::Critical, "1 ea", "3-day\nfull access", true),
            inv!("SKU-1100", "API Documentation", "Digital", 0, 100, "info", RowTier::Standard, 0, 0.0, 98421.0, "2021-06-08", "https://docs.example.com", 4, 5, PriorityKind::Medium, "0 B", "Free\nopen-source\ndocs", false),
            inv!("SKU-2200", "Limited Edition Mug", "Kitchen", 60, 100, "cart", RowTier::Vip, 25, 24.00, 312.0, "2025-02-28", "https://shop.example.com/mug", 5, 5, PriorityKind::Low, "350 ml", "Ceramic\n350ml", true),
            inv!("SKU-3301", "Media Player Pro", "Software", 120, 200, "media", RowTier::Standard, 8, 39.99, 890.0, "2024-11-01", "https://media.example.com/pro", 4, 5, PriorityKind::Medium, "45 MB", "MP3/WAV/FLAC\nplayback", false),
            inv!("SKU-4400", "Trending Widget", "Digital", 55, 80, "trending", RowTier::Featured, 12, 19.99, 4200.0, "2025-01-20", "https://shop.example.com/widget", 3, 5, PriorityKind::High, "120 g", "Viral\nproduct", true),
        ];
        let display_rows: Vec<usize> = (0..rows.len()).collect();
        let mut table = Self {
            rows,
            display_rows,
            tier_filter: None,
            search_query: String::new(),
        };
        table.rebuild_display_rows();
        table
    }

    fn row_count(&self) -> usize {
        self.display_rows.len()
    }

    fn logical_row(&self, display: usize) -> Option<usize> {
        self.display_rows.get(display).copied()
    }

    fn row(&self, display: usize) -> Option<&InventoryRow> {
        self.logical_row(display).and_then(|i| self.rows.get(i))
    }

    fn row_mut(&mut self, display: usize) -> Option<&mut InventoryRow> {
        let i = self.logical_row(display)?;
        self.rows.get_mut(i)
    }

    fn push_row(&mut self, item: InventoryRow) {
        self.rows.push(item);
        self.rebuild_display_rows();
    }

    fn remove_display_row(&mut self, display: usize) -> bool {
        if display >= self.display_rows.len() {
            return false;
        }
        self.display_rows.remove(display);
        true
    }

    fn duplicate_display_row(&mut self, display: usize) -> bool {
        let Some(src) = self.row(display).cloned() else {
            return false;
        };
        let mut copy = src;
        copy.sku = format!("{}-copy", copy.sku);
        copy.name = format!("{} (copy)", copy.name);
        self.push_row(copy);
        true
    }

    fn total_inventory_value(&self) -> f64 {
        self.display_rows
            .iter()
            .filter_map(|&i| self.rows.get(i))
            .map(|r| r.price * r.qty as f64)
            .sum()
    }

    /// Sort visible rows in the backing store (clears grid permutation).
    fn sort_display_by<F>(&mut self, mut cmp: F)
    where
        F: FnMut(&InventoryRow, &InventoryRow) -> Ordering,
    {
        self.display_rows.sort_by(|&a, &b| {
            cmp(
                self.rows.get(a).expect("valid index"),
                self.rows.get(b).expect("valid index"),
            )
        });
    }

    fn mark_tier(&mut self, display: usize, tier: RowTier) -> bool {
        if let Some(i) = self.logical_row(display) {
            if let Some(row) = self.rows.get_mut(i) {
                row.tier = tier;
                return true;
            }
        }
        false
    }
}

impl GridTable for CustTableGridTable {
    fn row_count(&self) -> usize {
        CustTableGridTable::row_count(self)
    }

    fn col_count(&self) -> usize {
        DATA_COLS
    }

    fn value(&self, display: usize, col: usize) -> Cell {
        let Some(item) = self.row(display) else {
            return Cell::Empty;
        };
        let string_renderer = GridCellStringRenderer::new();

        match col {
            0 => Cell::Text(string_renderer.render_string(&item.sku)),
            1 => Cell::Text(item.name.clone()),
            2 => Cell::Badge {
                text: format!("{} · {}", item.category, item.weight),
                kind: BadgeKind::Neutral,
            },
            3 => Cell::Number {
                value: item.price,
                format: if display.is_multiple_of(3) {
                    NumberFormat::CurrencyEuro
                } else if display % 3 == 1 {
                    NumberFormat::Fixed2
                } else {
                    NumberFormat::Plain
                },
            },
            4 => stock_cell(item.qty, item.max_qty, display),
            5 => status_cell(item),
            6 => Cell::Bar {
                value: item.discount_pct.min(100),
                max: 100,
                width: 8,
                style: match display % 4 {
                    0 => BarStyle::Solid,
                    1 => BarStyle::Rounded,
                    2 => BarStyle::Square,
                    _ => BarStyle::Dots,
                },
                label: Some(format!("-{}%", item.discount_pct)),
            },
            7 => Cell::Number {
                value: item.sales,
                format: NumberFormat::WithThousands,
            },
            8 => Cell::DateTime {
                iso: item.listed_date.clone(),
                format: match display % 3 {
                    0 => GridDateFormat::Eu,
                    1 => GridDateFormat::Us,
                    _ => GridDateFormat::Long,
                },
            },
            9 => Cell::Link {
                url: item.url.clone(),
                text: short_url(&item.url),
            },
            10 => Cell::Stars {
                value: item.rating,
                max: item.max_rating,
            },
            11 => Cell::Priority {
                kind: item.priority,
            },
            12 => Cell::Text(item.weight.clone()),
            _ => Cell::Empty,
        }
    }

    fn set_value(&mut self, display: usize, col: usize, value: Cell) -> bool {
        let Some(item) = self.row_mut(display) else {
            return false;
        };
        match col {
            0 => {
                if let Cell::Text(s) = value {
                    item.sku = s;
                    true
                } else {
                    false
                }
            }
            1 => {
                if let Cell::Text(s) = value {
                    item.name = s;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

fn stock_cell(qty: u32, max: u32, display: usize) -> Cell {
    if max == 0 {
        return Cell::Text("—".into());
    }
    Cell::Bar {
        value: qty.min(max),
        max,
        width: 10,
        style: match display % 4 {
            0 => BarStyle::Solid,
            1 => BarStyle::Rounded,
            2 => BarStyle::Square,
            _ => BarStyle::Dots,
        },
        label: Some(format!("{qty}u")),
    }
}

fn status_cell(item: &InventoryRow) -> Cell {
    let (kind, text) = if item.max_qty == 0 || item.qty == 0 {
        (BadgeKind::Bad, "Sold out")
    } else if item.qty < item.max_qty / 4 {
        (BadgeKind::Warn, "Low")
    } else if item.is_popular {
        (BadgeKind::Hot, "Popular")
    } else {
        (BadgeKind::Ok, "OK")
    };
    Cell::Badge {
        text: text.into(),
        kind,
    }
}

fn short_url(url: &str) -> String {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = stripped.split('/').next().unwrap_or(stripped);
    if host.len() > 20 {
        format!("{}…", &host[..18])
    } else {
        host.to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Label renderers (`GridWithLabelRenderersMixin`)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelRendererKind {
    Inventory,
    Compact,
    TierCode,
}

trait GridWithLabelRenderers {
    fn draw_row_label(&self, row: usize, table: &CustTableGridTable) -> Cell;
    fn row_label_style(&self, row: usize, table: &CustTableGridTable) -> GridCellStyle;
    fn column_title(&self, col: usize) -> &str;
    fn column_header_attr(&self, col: usize) -> GridCellAttr;
}

#[derive(Clone, Copy)]
struct InventoryLabelRenderer;

impl GridWithLabelRenderers for InventoryLabelRenderer {
    fn draw_row_label(&self, row: usize, table: &CustTableGridTable) -> Cell {
        let tier = table.row(row).map(|r| r.tier).unwrap_or(RowTier::Standard);
        let glyph = match tier {
            RowTier::Vip => "★",
            RowTier::Featured => "◆",
            RowTier::Archived => "▪",
            RowTier::Standard => "○",
        };
        Cell::Text(format!("{glyph} {:02}", row + 1))
    }

    fn row_label_style(&self, row: usize, table: &CustTableGridTable) -> GridCellStyle {
        tier_style(table.row(row).map(|r| r.tier))
    }

    fn column_title(&self, col: usize) -> &str {
        COL_TITLES[col]
    }

    fn column_header_attr(&self, col: usize) -> GridCellAttr {
        header_attr(col)
    }
}

#[derive(Clone, Copy)]
struct CompactLabelRenderer;

impl GridWithLabelRenderers for CompactLabelRenderer {
    fn draw_row_label(&self, row: usize, _table: &CustTableGridTable) -> Cell {
        Cell::Text(format!("{:02}", row + 1))
    }

    fn row_label_style(&self, _row: usize, _table: &CustTableGridTable) -> GridCellStyle {
        GridCellStyle::default()
    }

    fn column_title(&self, col: usize) -> &str {
        COL_TITLES[col]
    }

    fn column_header_attr(&self, col: usize) -> GridCellAttr {
        header_attr(col)
    }
}

#[derive(Clone, Copy)]
struct TierCodeLabelRenderer;

impl GridWithLabelRenderers for TierCodeLabelRenderer {
    fn draw_row_label(&self, row: usize, table: &CustTableGridTable) -> Cell {
        let code = table
            .row(row)
            .map(|r| match r.tier {
                RowTier::Vip => "VIP",
                RowTier::Featured => "FTR",
                RowTier::Archived => "ARC",
                RowTier::Standard => "STD",
            })
            .unwrap_or("???");
        Cell::Text(code.to_string())
    }

    fn row_label_style(&self, row: usize, table: &CustTableGridTable) -> GridCellStyle {
        tier_style(table.row(row).map(|r| r.tier))
    }

    fn column_title(&self, col: usize) -> &str {
        COL_TITLES[col]
    }

    fn column_header_attr(&self, col: usize) -> GridCellAttr {
        header_attr(col)
    }
}

fn tier_style(tier: Option<RowTier>) -> GridCellStyle {
    match tier.unwrap_or(RowTier::Standard) {
        RowTier::Vip => GridCellStyle {
            foreground: Some(Colour::new(120, 53, 15, 255)),
            background: Some(Colour::new(254, 243, 199, 255)),
        },
        RowTier::Featured => GridCellStyle {
            foreground: Some(Colour::new(30, 64, 175, 255)),
            background: Some(Colour::new(219, 234, 254, 255)),
        },
        RowTier::Archived => GridCellStyle {
            foreground: Some(Colour::new(107, 114, 128, 255)),
            background: Some(Colour::new(243, 244, 246, 255)),
        },
        RowTier::Standard => GridCellStyle::default(),
    }
}

fn header_attr(col: usize) -> GridCellAttr {
    let accent = match col {
        0 => Colour::new(79, 70, 229, 255),
        3 => Colour::new(180, 83, 9, 255),
        5 => Colour::new(22, 163, 74, 255),
        10 => Colour::new(147, 51, 234, 255),
        _ => Colour::new(51, 65, 85, 255),
    };
    GridCellAttr::new()
        .with_foreground(Colour::new(255, 255, 255, 255))
        .with_background(accent)
        .read_only()
}

const COL_TITLES: [&str; 15] = [
    "Row",
    "Type",
    "▣ Product",
    "SKU",
    "Category",
    "Price €",
    "Stock",
    "Status",
    "Disc%",
    "Sales",
    "Listed",
    "URL",
    "★ Rate",
    "Prio",
    "Wt",
];

const COL_SPECS: [(i32, ColumnAlign); 15] = [
    (52, ColumnAlign::Center),
    (52, ColumnAlign::Center),
    (130, ColumnAlign::Left),
    (82, ColumnAlign::Left),
    (90, ColumnAlign::Center),
    (72, ColumnAlign::Right),
    (88, ColumnAlign::Center),
    (68, ColumnAlign::Center),
    (78, ColumnAlign::Center),
    (62, ColumnAlign::Right),
    (78, ColumnAlign::Center),
    (96, ColumnAlign::Left),
    (52, ColumnAlign::Center),
    (62, ColumnAlign::Left),
    (52, ColumnAlign::Right),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum IconSetKind {
    Lucide,
    Bootstrap,
}

// ═══════════════════════════════════════════════════════════════════════
// CustTableGrid — composed widget
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct CustTableGrid {
    grid: Grid,
    table: Rc<RefCell<CustTableGridTable>>,
    icons: Rc<RefCell<GridIcons>>,
    label_kind: Rc<StdCell<LabelRendererKind>>,
    icon_kind: Rc<StdCell<IconSetKind>>,
    sort_label: Rc<RefCell<String>>,
}

const HIGHLIGHT_STYLE: GridCellStyle = GridCellStyle {
    foreground: Some(Colour::new(30, 64, 175, 255)),
    background: Some(Colour::new(219, 234, 254, 255)),
};

const VIP_STYLE: GridCellStyle = GridCellStyle {
    foreground: Some(Colour::new(120, 53, 15, 255)),
    background: Some(Colour::new(255, 251, 235, 255)),
};

fn sync_accent_selection(frame: &Frame, grid: &Grid) {
    if let Some(accent) = frame.border_color() {
        let mut appearance = grid.appearance();
        appearance.selection_background = accent;
        grid.set_appearance(appearance, Some(frame));
    }
}

fn sort_label_for(col: Option<usize>, order: Option<SortOrder>) -> String {
    match (col, order) {
        (Some(c), Some(SortOrder::Ascending)) => {
            format!("Sort: {} ↑", InventoryLabelRenderer.column_title(c))
        }
        (Some(c), Some(SortOrder::Descending)) => {
            format!("Sort: {} ↓", InventoryLabelRenderer.column_title(c))
        }
        _ => "Sort: original".to_string(),
    }
}

impl CustTableGrid {
    fn new<W: ru_wx::Window>(parent: &W, frame: &Frame) -> Self {
        let table = Rc::new(RefCell::new(CustTableGridTable::sample_inventory()));
        let icons = Rc::new(RefCell::new(GridIcons::lucide_modern(20)));
        let label_kind = Rc::new(StdCell::new(LabelRendererKind::Inventory));
        let icon_kind = Rc::new(StdCell::new(IconSetKind::Lucide));
        let sort_label = Rc::new(RefCell::new("Sort: original".to_string()));

        let grid = Grid::new(parent);
        grid.set_checkboxes(true);
        grid.set_font_desc(FontDesc::new("Segoe UI", 9), true);
        grid.enable_interactive_features(frame);

        let mut appearance = GridAppearance::modern();
        appearance.header_background = Colour::new(51, 65, 85, 255);
        appearance.header_foreground = Colour::new(255, 255, 255, 255);
        appearance.selection_background = Colour::new(79, 70, 229, 255);
        grid.set_appearance(appearance, Some(frame));
        grid.set_alternating_row_colors(
            Colour::new(255, 255, 255, 255),
            Colour::new(248, 250, 252, 255),
        );

        for (col, (width, align)) in COL_SPECS.iter().enumerate() {
            grid.append_column_with_align(
                InventoryLabelRenderer.column_title(col),
                *width,
                *align,
            );
            let _ = InventoryLabelRenderer.column_header_attr(col);
        }
        grid.attach_icons_with_column_width(&icons.borrow(), grid_col::TYPE, 52);

        let slf = Self {
            grid,
            table,
            icons,
            label_kind,
            icon_kind,
            sort_label,
        };
        slf.rebind_provider();
        slf.apply_styles();

        let deferred = slf.clone();
        slf.grid.call_after_message_loop(frame, move || {
            deferred.deferred_init();
        });
        slf
    }

    fn deferred_init(&self) {
        let rows = self.table.borrow().row_count();
        for r in [0usize, 2, 6, 9] {
            if r < rows {
                self.grid.set_checked(r, true);
            }
        }
        self.apply_styles();
        self.grid.request_repaint();
    }

    fn draw_row_label(kind: LabelRendererKind, row: usize, table: &CustTableGridTable) -> Cell {
        match kind {
            LabelRendererKind::Inventory => InventoryLabelRenderer.draw_row_label(row, table),
            LabelRendererKind::Compact => CompactLabelRenderer.draw_row_label(row, table),
            LabelRendererKind::TierCode => TierCodeLabelRenderer.draw_row_label(row, table),
        }
    }

    fn row_label_style(kind: LabelRendererKind, row: usize, table: &CustTableGridTable) -> GridCellStyle {
        match kind {
            LabelRendererKind::Inventory => InventoryLabelRenderer.row_label_style(row, table),
            LabelRendererKind::Compact => CompactLabelRenderer.row_label_style(row, table),
            LabelRendererKind::TierCode => TierCodeLabelRenderer.row_label_style(row, table),
        }
    }

    fn rebind_provider(&self) {
        let table = Rc::clone(&self.table);
        let icons = Rc::clone(&self.icons);
        let label_kind = Rc::clone(&self.label_kind);

        self.grid.set_value_provider(move |row, col| {
            use grid_col::*;
            if col == ROW {
                let kind = label_kind.get();
                return CustTableGrid::draw_row_label(kind, row, &table.borrow());
            }
            if col == TYPE {
                if let Some(item) = table.borrow().row(row) {
                    let ic = icons.borrow();
                    if ic.index(&item.icon).is_some() {
                        return ic.cell(&item.icon, &item.name);
                    }
                    return ic.cell_for_product(
                        &item.category,
                        item.qty,
                        item.max_qty,
                        item.is_popular,
                        &item.name,
                    );
                }
                return Cell::Empty;
            }
            if col == PRODUCT {
                if let Some(item) = table.borrow().row(row) {
                    if item.tier == RowTier::Archived {
                        return Cell::MultiLine(item.description.clone());
                    }
                    return Cell::Text(item.name.clone());
                }
                return Cell::Empty;
            }
            if let Some(dc) = data_col_for_grid_col(col) {
                let cell = table.borrow().value(row, dc);
                // Dynamic columns added via header context menu.
                if col >= COUNT {
                    return Cell::Text(format!("Col {col} · r{row}"));
                }
                return cell;
            }
            Cell::Empty
        });
        self.grid.set_row_count(self.table.borrow().row_count());
    }

    fn apply_styles(&self) {
        let rows = self.table.borrow().row_count();
        let kind = self.label_kind.get();
        for row in 0..rows {
            self.grid.set_cell_style(
                row,
                0,
                Self::row_label_style(kind, row, &self.table.borrow()),
            );
            if let Some(item) = self.table.borrow().row(row) {
                if item.tier == RowTier::Archived {
                    self.grid.set_row_style(
                        row,
                        GridCellStyle {
                            foreground: Some(Colour::new(156, 163, 175, 255)),
                            background: None,
                        },
                    );
                }
                if item.tier == RowTier::Vip {
                    self.grid.set_row_style(row, VIP_STYLE);
                }
                if item.is_popular {
                    self.grid.set_cell_style(
                        row,
                        grid_col::STATUS,
                        GridCellStyle {
                            foreground: Some(Colour::new(180, 83, 9, 255)),
                            background: Some(Colour::new(255, 251, 235, 255)),
                        },
                    );
                }
            }
        }
        for r in [0usize, 2, 6, 9] {
            if r < rows {
                let _ = r; // checked in deferred_init (after message loop)
            }
        }
    }

    fn sort_by(&self, col: usize, order: SortOrder, title: &str) {
        let _ = title;
        self.grid.sort_by_column(col, order);
    }

    fn clear_sort(&self) {
        self.grid.clear_sort();
        self.refresh();
    }

    fn sort_data_by_price(&self) {
        self.table.borrow_mut().sort_display_by(|a, b| {
            b.price
                .partial_cmp(&a.price)
                .unwrap_or(Ordering::Equal)
        });
        self.sort_label
            .borrow_mut()
            .clone_from(&"Sort: data price ↓".to_string());
        self.grid.clear_sort();
        self.refresh();
    }

    fn highlight_selected(&self) {
        self.grid.highlight_selected_row(HIGHLIGHT_STYLE);
    }

    fn toggle_check_selected(&self) -> bool {
        let Some(r) = self.grid.get_selected_row() else {
            return false;
        };
        self.grid.set_checked(r, !self.grid.is_checked(r));
        true
    }

    fn export_checked_lines(&self) -> String {
        let table = self.table.borrow();
        let n = table.row_count();
        let mut lines = Vec::new();
        for r in 0..n {
            if self.grid.is_checked(r) {
                if let Some(item) = table.row(r) {
                    lines.push(format!(
                        "{} | {} | €{:.2} | {} | ★{}/{}",
                        item.sku, item.name, item.price, item.category, item.rating, item.max_rating
                    ));
                }
            }
        }
        lines.join("\n")
    }

    fn copy_selected_row(&self) -> bool {
        self.grid
            .get_selected_row()
            .is_some_and(|r| self.grid.copy_row_to_clipboard(r))
    }

    fn footer_summary(&self) -> String {
        let table = self.table.borrow();
        let order = self.grid.column_order();
        let (sort_col, sort_order) = self.grid.sort_state();
        let sort = match (sort_col, sort_order) {
            (Some(c), Some(o)) => format!(
                "sort col {c} {}",
                if o == SortOrder::Ascending { "↑" } else { "↓" }
            ),
            _ => "no sort".into(),
        };
        format!(
            "{} · {} rows ({} total) · {} checked · cols {} · {}",
            table.filter_summary(),
            table.row_count(),
            table.total_rows(),
            count_checked(self),
            order
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(","),
            sort,
        )
    }

    fn popup_row_menu(&self, frame: &Frame) {
        let Some(row) = self.grid.get_selected_row() else {
            return;
        };
        let mut menu = PopupMenu::new();
        let cust = self.clone();
        let frame_a = frame.clone();
        menu.append("Toggle checkbox", &frame_a, move || {
            let _ = cust.toggle_check_selected();
        });
        let cust = self.clone();
        let frame_b = frame.clone();
        menu.append("Duplicate row", &frame_b, move || {
            cust.table.borrow_mut().duplicate_display_row(row);
            cust.refresh();
        });
        let cust = self.clone();
        let frame_c = frame.clone();
        menu.append("Delete row", &frame_c, move || {
            cust.table.borrow_mut().remove_display_row(row);
            cust.refresh();
        });
        let cust = self.clone();
        let frame_d = frame.clone();
        menu.append("Highlight row", &frame_d, move || {
            cust.grid.set_row_style(row, HIGHLIGHT_STYLE);
            cust.grid.request_repaint();
        });
        let cust = self.clone();
        let frame_e = frame.clone();
        menu.append("Mark as VIP", &frame_e, move || {
            cust.table.borrow_mut().mark_tier(row, RowTier::Vip);
            cust.refresh();
        });
        let cust = self.clone();
        let frame_f = frame.clone();
        menu.append("Archive row", &frame_f, move || {
            cust.table.borrow_mut().mark_tier(row, RowTier::Archived);
            cust.refresh();
        });
        let cust = self.clone();
        let frame_g = frame.clone();
        let frame_h = frame.clone();
        menu.append("Export checked…", &frame_g, move || {
            let text = cust.export_checked_lines();
            MessageDialog::new(
                &frame_h,
                if text.is_empty() {
                    "No rows checked."
                } else {
                    &text
                },
                "Checked rows",
                MessageDialogStyle::Ok,
                MessageBoxIcon::Information,
            )
            .show_modal();
        });
        menu.popup(frame);
    }

    fn sort_label_text(&self) -> String {
        self.sort_label.borrow().clone()
    }

    fn refresh(&self) {
        self.grid.set_row_count(self.table.borrow().row_count());
        self.grid.refresh();
        self.apply_styles();
    }

    fn set_label_renderer(&self, kind: LabelRendererKind) {
        self.label_kind.set(kind);
        self.rebind_provider();
        self.apply_styles();
    }

    fn toggle_icon_set(&self) {
        let next = match self.icon_kind.get() {
            IconSetKind::Lucide => {
                self.icons.replace(GridIcons::bootstrap_modern(20));
                IconSetKind::Bootstrap
            }
            IconSetKind::Bootstrap => {
                self.icons.replace(GridIcons::lucide_modern(20));
                IconSetKind::Lucide
            }
        };
        self.icon_kind.set(next);
        self.grid.attach_icons_with_column_width(&self.icons.borrow(), grid_col::TYPE, 52);
        self.refresh();
    }

    fn inner(&self) -> &Grid {
        &self.grid
    }

    fn table(&self) -> Rc<RefCell<CustTableGridTable>> {
        Rc::clone(&self.table)
    }

    fn set_theme(&self, theme: GridTheme, frame: &Frame) {
        let appearance = match theme {
            GridTheme::Win11 => {
                self.grid.apply_win11_theme(frame);
                return;
            }
            GridTheme::Modern => {
                let mut a = GridAppearance::modern();
                a.header_background = Colour::new(51, 65, 85, 255);
                a
            }
            GridTheme::Warm => GridAppearance::warm(),
            GridTheme::Dark => GridAppearance::dark(),
            GridTheme::Classic => GridAppearance::classic(),
        };
        self.grid.set_appearance(appearance, Some(frame));
        self.refresh();
    }
}

#[derive(Clone, Copy)]
enum GridTheme {
    Win11,
    Modern,
    Warm,
    Dark,
    Classic,
}

// ═══════════════════════════════════════════════════════════════════════
// Tab 2 — GridStringTable demo
// ═══════════════════════════════════════════════════════════════════════

fn build_string_table_tab(panel: &Panel, frame: &Frame) -> Grid {
    let hint = StaticText::new(
        panel,
        "GridStringTable — editable wxGridStringTable equivalent (double-click cells):",
    );
    let grid = Grid::new(panel);
    grid.enable_interactive_features(frame);
    grid.apply_win11_theme(frame);
    grid.set_font_desc(FontDesc::new("Consolas", 9), true);

    let mut table = GridStringTable::new();
    table.resize(6, 4);
    let seed = [
        ("A1", "Name", "Qty", "Notes"),
        ("B2", "Widget", "42", "OK"),
        ("C3", "Gadget", "7", "Low"),
        ("D4", "Tool", "128", "Restock"),
        ("E5", "Part", "0", "Sold out"),
        ("F6", "Kit", "15", "New"),
    ];
    for (r, row) in seed.iter().enumerate() {
        table.set_value(r, 0, row.0);
        table.set_value(r, 1, row.1);
        table.set_value(r, 2, row.2);
        table.set_value(r, 3, row.3);
    }

    let table = Rc::new(RefCell::new(table));
    let table_for_set = Rc::clone(&table);

    grid.append_column("ID", 48);
    grid.append_column("Product", 120);
    grid.append_column("Count", 64);
    grid.append_column("Memo", 160);
    grid.append_column_with_align("Extra", 72, ColumnAlign::Center);

    grid.set_value_provider({
        let table = Rc::clone(&table);
        move |row, col| {
            if col < 4 {
                table.borrow().value(row, col)
            } else {
                let qty: u32 = table
                    .borrow()
                    .get_value(row, 2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if qty == 0 {
                    Cell::Badge {
                        text: "Empty".into(),
                        kind: BadgeKind::Bad,
                    }
                } else if qty < 10 {
                    Cell::Badge {
                        text: "Low".into(),
                        kind: BadgeKind::Warn,
                    }
                } else {
                    Cell::Badge {
                        text: "OK".into(),
                        kind: BadgeKind::Ok,
                    }
                }
            }
        }
    });
    grid.set_row_count(6);

    let edit_btn = Button::new(panel, "Append row");
    edit_btn.on_click(frame, {
        let grid = grid.clone();
        let table = Rc::clone(&table_for_set);
        move || {
            let r = table.borrow().row_count();
            table.borrow_mut().resize(r + 1, 5);
            table
                .borrow_mut()
                .set_value(r, 0, &format!("X{}", r + 1));
            table.borrow_mut().set_value(r, 1, "New item");
            table.borrow_mut().set_value(r, 2, "1");
            table.borrow_mut().set_value(r, 3, "Added");
            grid.set_row_count(r + 1);
            grid.refresh();
        }
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(edit_btn.as_widget_ref());
    sizer.add_with_proportion(grid.as_widget_ref(), 1);
    panel.set_sizer(sizer);
    grid
}

// ═══════════════════════════════════════════════════════════════════════
// Tab 3 — FunctionGridTable demo
// ═══════════════════════════════════════════════════════════════════════

fn build_function_table_tab(panel: &Panel, frame: &Frame) -> Grid {
    let hint = StaticText::new(
        panel,
        "FunctionGridTable — closure model f(row,col)→Cell (live sin/cos heat-map):",
    );
    let grid = Grid::new(panel);
    grid.apply_win11_theme(frame);

    let phase = Rc::new(StdCell::new(0.0f64));
    let table = FunctionGridTable::new(12, 8, {
        let phase = Rc::clone(&phase);
        move |row, col| {
            let x = col as f64 * 0.55 + phase.get();
            let y = row as f64 * 0.40;
            let v = (x.sin() * y.cos() + 1.0) / 2.0;
            let pct = (v * 100.0) as u32;
            Cell::Bar {
                value: pct,
                max: 100,
                width: 10,
                style: BarStyle::Solid,
                label: Some(format!("{pct}%")),
            }
        }
    });

    let _ = table; // model documented; grid uses same closure
    grid.append_column("A", 72);
    grid.append_column("B", 72);
    grid.append_column("C", 72);
    grid.append_column("D", 72);
    grid.append_column("E", 72);
    grid.append_column("F", 72);
    grid.append_column("G", 72);
    grid.append_column("H", 72);

    let phase_provider = Rc::clone(&phase);
    grid.set_value_provider(move |row, col| {
        let x = col as f64 * 0.55 + phase_provider.get();
        let y = row as f64 * 0.40;
        let v = (x.sin() * y.cos() + 1.0) / 2.0;
        let pct = (v * 100.0) as u32;
        Cell::Bar {
            value: pct,
            max: 100,
            width: 10,
            style: BarStyle::Rounded,
            label: None,
        }
    });
    grid.set_row_count(12);

    let bool_demo = StaticText::new(
        panel,
        &format!(
            "GridCellBoolRenderer: {}",
            GridCellBoolRenderer::new().render_bool(true)
        ),
    );

    let animate_btn = Button::new(panel, "Animate (+0.25 phase)");
    animate_btn.on_click(frame, {
        let grid = grid.clone();
        let phase = Rc::clone(&phase);
        move || {
            phase.set(phase.get() + 0.25);
            grid.refresh();
        }
    });

    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(bool_demo.as_widget_ref());
    sizer.add(animate_btn.as_widget_ref());
    sizer.add_with_proportion(grid.as_widget_ref(), 1);
    panel.set_sizer(sizer);
    grid
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx — CustTableGrid (sort · context · images)")
        .with_size(1280, 760)
        .build();

    let status = StatusBar::new(&frame, 3);
    status.set_status_text("CustTableGrid + GridStringTable + FunctionGridTable", 0);
    status.set_status_text("Header sort · row/column context · tooltips · Mica Alt", 1);
    status.set_status_text("F5 Refresh", 2);

    let notebook = Tab::new(&frame);

    // ── Tab 1: CustTableGrid ─────────────────────────────────────────
    let inv_panel = Panel::new(&frame);
    let inv_hint = StaticText::new(
        &inv_panel,
        "Advanced Grid showcase — all Cell types, Win32 extras (header sort, row/column menus, \
         tooltips, clipboard, search). Double-click row for details. F5 refresh.",
    );
    let search = SearchCtrl::new(&inv_panel, "Search SKU, name, category…");
    let cust = Rc::new(CustTableGrid::new(&inv_panel, &frame));
    sync_accent_selection(&frame, cust.inner());

    cust.inner().on_row_context_menu({
        let cust = Rc::clone(&cust);
        move |f, _row, _col| {
            cust.popup_row_menu(f);
        }
    });

    cust.inner().on_row_activated(&frame, {
        let cust = Rc::clone(&cust);
        let frame = frame.clone();
        move |row, col| {
            if let Some(item) = cust.table().borrow().row(row) {
                MessageDialog::new(
                    &frame,
                    &format!(
                        "{}\n\nSKU: {}\nCategory: {}\nPrice: €{:.2}\nStock: {}/{}\n\
                         Tier: {:?}\nColumn: {}\n\n{}",
                        item.name,
                        item.sku,
                        item.category,
                        item.price,
                        item.qty,
                        item.max_qty,
                        item.tier,
                        col,
                        item.description.replace('\n', " · "),
                    ),
                    "Row details (double-click)",
                    MessageDialogStyle::Ok,
                    MessageBoxIcon::Information,
                )
                .show_modal();
            }
        }
    });

    search.on_search(&frame, {
        let cust = Rc::clone(&cust);
        move |text| {
            cust.table().borrow_mut().set_search(&text);
            cust.refresh();
        }
    });

    cust.inner().set_cell_tooltip_provider(&frame, {
        let table = cust.table();
        move |row, col| {
            use grid_col::*;
            let borrowed = table.borrow();
            let item = borrowed.row(row)?;
            match col {
                URL => Some(item.url.clone()),
                PRODUCT if item.tier == RowTier::Archived => Some(item.description.clone()),
                SKU => Some(format!("{} — {}", item.sku, item.description)),
                _ => None,
            }
        }
    });

    let selection_label = StaticText::new(&inv_panel, "Selection: (none)");
    let sort_label = StaticText::new(&inv_panel, &cust.sort_label_text());
    let stats_label = StaticText::new(
        &inv_panel,
        &format!(
            "Rows: {} · inventory value: € {:.0}",
            cust.table().borrow().row_count(),
            cust.table().borrow().total_inventory_value()
        ),
    );
    let footer_label = StaticText::new(&inv_panel, &cust.footer_summary());
    cust.inner().on_sort_changed({
        let sort_l = sort_label.clone();
        let footer = footer_label.clone();
        let cust = Rc::clone(&cust);
        move |col, order| {
            let text = sort_label_for(col, order);
            cust.sort_label.borrow_mut().clone_from(&text);
            sort_l.set_label(&text);
            footer.set_label(&cust.footer_summary());
        }
    });

    let mut btn_row = BoxSizer::horizontal();
    let check_all = Button::new(&inv_panel, "Check all");
    let uncheck_all = Button::new(&inv_panel, "Uncheck all");
    let copy_row = Button::new(&inv_panel, "Copy row");
    let copy_checked = Button::new(&inv_panel, "Copy checked");
    let sort_price = Button::new(&inv_panel, "Price ↓");
    let sort_name = Button::new(&inv_panel, "Product ↑");
    let clear_sort_btn = Button::new(&inv_panel, "Clear sort");
    let highlight_btn = Button::new(&inv_panel, "Highlight");
    let row_menu_btn = Button::new(&inv_panel, "Row menu…");
    let add_row = Button::new(&inv_panel, "Add row");
    let refresh_btn = Button::new(&inv_panel, "Refresh");
    btn_row.add(check_all.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(uncheck_all.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(copy_row.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(copy_checked.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(sort_price.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(sort_name.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(clear_sort_btn.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(highlight_btn.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(row_menu_btn.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(add_row.as_widget_ref());
    btn_row.add_spacer(6);
    btn_row.add(refresh_btn.as_widget_ref());

    let mut inv_sizer = BoxSizer::vertical();
    inv_sizer.add(inv_hint.as_widget_ref());
    inv_sizer.add(search.as_widget_ref());
    inv_sizer.add_sizer(btn_row);
    inv_sizer.add_with_proportion(cust.inner().as_widget_ref(), 1);
    inv_sizer.add(sort_label.as_widget_ref());
    inv_sizer.add(selection_label.as_widget_ref());
    inv_sizer.add(stats_label.as_widget_ref());
    inv_sizer.add(footer_label.as_widget_ref());
    inv_panel.set_sizer(inv_sizer);
    notebook.add_page("Inventory", &inv_panel);

    // ── Tab 2 & 3 ────────────────────────────────────────────────────
    let string_panel = Panel::new(&frame);
    let _string_grid = build_string_table_tab(&string_panel, &frame);
    notebook.add_page("String table", &string_panel);

    let fn_panel = Panel::new(&frame);
    let _fn_grid = build_function_table_tab(&fn_panel, &frame);
    notebook.add_page("Function table", &fn_panel);

    cust.inner().on_selection_changed(&frame, {
        let label = selection_label.clone();
        let stats = stats_label.clone();
        let sort_l = sort_label.clone();
        let footer = footer_label.clone();
        let table = cust.table();
        let cust_rc = Rc::clone(&cust);
        move |row| {
            if let Some(row) = row {
                if let Some(item) = table.borrow().row(row) {
                    let chk = if cust_rc.inner().is_checked(row) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    label.set_label(&format!(
                        "{chk} {} — {} · {} · {} · ★{}/{} · {:?}",
                        item.sku,
                        item.name,
                        item.category,
                        item.weight,
                        item.rating,
                        item.max_rating,
                        item.priority
                    ));
                }
            } else {
                label.set_label("Selection: (none)");
            }
            sort_l.set_label(&cust_rc.sort_label_text());
            stats.set_label(&format!(
                "Rows: {} · value: € {:.0} · checked: {} · cols: {}",
                table.borrow().row_count(),
                table.borrow().total_inventory_value(),
                count_checked(&cust_rc),
                cust_rc.inner().col_count()
            ));
            footer.set_label(&cust_rc.footer_summary());
        }
    });

    check_all.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let footer = footer_label.clone();
        let stats = stats_label.clone();
        let status = status.clone();
        move || {
            cust.inner().set_all_checked(true);
            stats.set_label(&format!(
                "Rows: {} · value: € {:.0} · checked: {} · cols: {}",
                cust.table().borrow().row_count(),
                cust.table().borrow().total_inventory_value(),
                count_checked(&cust),
                cust.inner().col_count()
            ));
            footer.set_label(&cust.footer_summary());
            status.set_status_text("All rows checked", 0);
        }
    });
    uncheck_all.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let footer = footer_label.clone();
        let stats = stats_label.clone();
        let status = status.clone();
        move || {
            cust.inner().set_all_checked(false);
            stats.set_label(&format!(
                "Rows: {} · value: € {:.0} · checked: {} · cols: {}",
                cust.table().borrow().row_count(),
                cust.table().borrow().total_inventory_value(),
                count_checked(&cust),
                cust.inner().col_count()
            ));
            footer.set_label(&cust.footer_summary());
            status.set_status_text("All rows unchecked", 0);
        }
    });
    copy_row.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            if cust.copy_selected_row() {
                status.set_status_text("Row copied (TSV) to clipboard", 0);
            } else {
                status.set_status_text("Select a row to copy", 0);
            }
        }
    });
    copy_checked.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            if cust.inner().copy_checked_rows_to_clipboard() {
                status.set_status_text("Checked rows copied (TSV) to clipboard", 0);
            } else {
                status.set_status_text("No checked rows to copy", 0);
            }
        }
    });

    sort_price.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        let sort_l = sort_label.clone();
        move || {
            cust.sort_by(grid_col::PRICE, SortOrder::Descending, "Price");
            sort_l.set_label(&cust.sort_label_text());
            status.set_status_text("Sorted by price ↓", 0);
        }
    });
    sort_name.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        let sort_l = sort_label.clone();
        move || {
            cust.sort_by(grid_col::PRODUCT, SortOrder::Ascending, "Product");
            sort_l.set_label(&cust.sort_label_text());
            status.set_status_text("Sorted by product ↑", 0);
        }
    });
    clear_sort_btn.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        let sort_l = sort_label.clone();
        move || {
            cust.clear_sort();
            sort_l.set_label(&cust.sort_label_text());
            status.set_status_text("Sort cleared", 0);
        }
    });
    highlight_btn.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.highlight_selected();
            status.set_status_text("Row highlighted", 0);
        }
    });
    row_menu_btn.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let frame = frame.clone();
        let status = status.clone();
        move || {
            cust.popup_row_menu(&frame);
            status.set_status_text("Row context menu", 0);
        }
    });
    refresh_btn.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let footer = footer_label.clone();
        let status = status.clone();
        move || {
            cust.refresh();
            footer.set_label(&cust.footer_summary());
            status.set_status_text("Grid refreshed", 0);
        }
    });
    add_row.on_click(&frame, {
        let cust = Rc::clone(&cust);
        let stats = stats_label.clone();
        let status = status.clone();
        move || {
            cust.table().borrow_mut().push_row(inv!(
                "SKU-NEW", "New Product", "Digital", 10, 50, "file-new",
                RowTier::Standard, 0, 9.99, 0.0, "2026-06-10", "https://example.com/new",
                3, 5, PriorityKind::Low, "—", "Added\nvia button", false
            ));
            cust.refresh();
            stats.set_label(&format!(
                "Rows: {} · value: € {:.0}",
                cust.table().borrow().row_count(),
                cust.table().borrow().total_inventory_value()
            ));
            status.set_status_text("Row appended", 0);
        }
    });

    // ── Sort menu (every column ↑/↓) ───────────────────────────────
    let mut sort_menu = Menu::new("&Sort");
    let sort_pairs: [(usize, &str); 10] = [
        (grid_col::PRODUCT, "Product"),
        (grid_col::SKU, "SKU"),
        (grid_col::CATEGORY, "Category"),
        (grid_col::PRICE, "Price"),
        (grid_col::STOCK, "Stock"),
        (grid_col::SALES, "Sales"),
        (grid_col::LISTED, "Listed"),
        (grid_col::RATE, "Rating"),
        (grid_col::PRIO, "Priority"),
        (grid_col::WEIGHT, "Weight"),
    ];
    for (col, title) in sort_pairs {
        sort_menu.append(&format!("{title} ↑"), &frame, {
            let cust = Rc::clone(&cust);
            let status = status.clone();
            let sort_l = sort_label.clone();
            move || {
                cust.sort_by(col, SortOrder::Ascending, title);
                sort_l.set_label(&cust.sort_label_text());
                status.set_status_text(&format!("Sort {title} ↑"), 0);
            }
        });
        sort_menu.append(&format!("{title} ↓"), &frame, {
            let cust = Rc::clone(&cust);
            let status = status.clone();
            let sort_l = sort_label.clone();
            move || {
                cust.sort_by(col, SortOrder::Descending, title);
                sort_l.set_label(&cust.sort_label_text());
                status.set_status_text(&format!("Sort {title} ↓"), 0);
            }
        });
    }
    sort_menu.append_separator();
    sort_menu.append("Data sort by price (model)", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        let sort_l = sort_label.clone();
        move || {
            cust.sort_data_by_price();
            sort_l.set_label(&cust.sort_label_text());
            status.set_status_text("Data sorted by price", 0);
        }
    });
    sort_menu.append("Clear sort / restore order", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        let sort_l = sort_label.clone();
        move || {
            cust.clear_sort();
            sort_l.set_label(&cust.sort_label_text());
            status.set_status_text("Sort cleared", 0);
        }
    });

    // ── Row context menu (selection-based) ─────────────────────────
    let mut row_menu = Menu::new("&Row");
    row_menu.append("Popup row menu…", &frame, {
        let cust = Rc::clone(&cust);
        let frame = frame.clone();
        move || {
            cust.popup_row_menu(&frame);
        }
    });
    row_menu.append("Toggle checkbox", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            let _ = cust.toggle_check_selected();
        }
    });
    row_menu.append("Highlight selected", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            cust.highlight_selected();
        }
    });
    row_menu.append("Clear row highlight", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            if let Some(r) = cust.inner().get_selected_row() {
                cust.inner().clear_row_style(r);
            }
        }
    });
    row_menu.append("Clear all highlights", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            cust.inner().clear_all_row_styles();
        }
    });
    row_menu.append_separator();
    row_menu.append("Export checked rows…", &frame, {
        let cust = Rc::clone(&cust);
        let frame = frame.clone();
        move || {
            let text = cust.export_checked_lines();
            MessageDialog::new(
                &frame,
                if text.is_empty() { "No rows checked." } else { &text },
                "Export",
                MessageDialogStyle::Ok,
                MessageBoxIcon::Information,
            )
            .show_modal();
        }
    });

    // ── Grid menu (column context + grid ops) ──────────────────────
    let mut grid_menu = Menu::new("&Grid");
    grid_menu.append("Popup column menu…", &frame, {
        let grid = cust.inner().clone();
        let frame = frame.clone();
        move || {
            grid.popup_column_context_menu(&frame);
        }
    });
    grid_menu.append("Delete selected row", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            cust.inner().delete_selected_row();
            cust.refresh();
        }
    });
    grid_menu.append("Append empty row", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            cust.inner().append_row();
            cust.refresh();
        }
    });
    grid_menu.append("Copy selected row (TSV)", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            if cust.copy_selected_row() {
                status.set_status_text("Row copied to clipboard", 0);
            }
        }
    });
    grid_menu.append("Copy checked rows (TSV)", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            if cust.inner().copy_checked_rows_to_clipboard() {
                status.set_status_text("Checked rows copied", 0);
            }
        }
    });
    grid_menu.append("Check all / uncheck all", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            let any_unchecked = (0..cust.inner().row_count()).any(|r| !cust.inner().is_checked(r));
            cust.inner().set_all_checked(any_unchecked);
            status.set_status_text("Toggled all checkboxes", 0);
        }
    });
    grid_menu.append_separator();
    grid_menu.append("Auto-fit all columns", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            let n = cust.inner().col_count();
            for c in 0..n {
                cust.inner().autosize_column(c);
            }
            status.set_status_text("Columns auto-fitted (header + content)", 0);
        }
    });
    grid_menu.append_separator();
    grid_menu.append("Refresh", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            cust.refresh();
        }
    });
    grid_menu.append("Force refresh", &frame, {
        let cust = Rc::clone(&cust);
        move || {
            cust.inner().force_refresh();
        }
    });

    // ── Menus (data / labels / view / theme) ───────────────────────
    let mut data_menu = Menu::new("&Data");
    data_menu.append("Add row", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.table().borrow_mut().push_row(inv!(
                "SKU-MENU", "Menu Item", "Service", 5, 20, "cloud",
                RowTier::Featured, 5, 29.99, 100.0, "2026-06-10", "https://example.com/menu",
                4, 5, PriorityKind::Medium, "—", "From menu", true
            ));
            cust.refresh();
            status.set_status_text("Data → Add row", 0);
        }
    });
    data_menu.append("Duplicate selected", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            if let Some(r) = cust.inner().get_selected_row() {
                cust.table().borrow_mut().duplicate_display_row(r);
                cust.refresh();
                status.set_status_text(&format!("Duplicated row {r}"), 0);
            }
        }
    });
    data_menu.append("Remove selected", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            if let Some(r) = cust.inner().get_selected_row() {
                cust.table().borrow_mut().remove_display_row(r);
                cust.refresh();
                status.set_status_text(&format!("Removed row {r}"), 0);
            }
        }
    });
    data_menu.append_separator();
    data_menu.append("Show all tiers", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.table().borrow_mut().set_filter(None);
            cust.refresh();
            status.set_status_text("Filter: all tiers", 0);
        }
    });
    data_menu.append("VIP only", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.table().borrow_mut().set_filter(Some(RowTier::Vip));
            cust.refresh();
            status.set_status_text("Filter: VIP", 0);
        }
    });
    data_menu.append("Featured only", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.table().borrow_mut().set_filter(Some(RowTier::Featured));
            cust.refresh();
            status.set_status_text("Filter: Featured", 0);
        }
    });

    let mut label_menu = Menu::new("&Labels");
    label_menu.append("Inventory glyphs (★◆○)", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.set_label_renderer(LabelRendererKind::Inventory);
            status.set_status_text("Labels: Inventory", 0);
        }
    });
    label_menu.append("Compact numbers", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.set_label_renderer(LabelRendererKind::Compact);
            status.set_status_text("Labels: Compact", 0);
        }
    });
    label_menu.append("Tier codes (VIP/FTR/STD)", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.set_label_renderer(LabelRendererKind::TierCode);
            status.set_status_text("Labels: TierCode", 0);
        }
    });

    let mut view_menu = Menu::new("&View");
    view_menu.append("Toggle icon set (Lucide ↔ Bootstrap)", &frame, {
        let cust = Rc::clone(&cust);
        let status = status.clone();
        move || {
            cust.toggle_icon_set();
            status.set_status_text("Icon set toggled", 0);
        }
    });
    view_menu.append("Toggle checkboxes", &frame, {
        let grid = cust.inner().clone();
        let status = status.clone();
        move || {
            let on = grid.is_checked(0);
            grid.set_checkboxes(!on);
            status.set_status_text("Checkboxes toggled", 0);
        }
    });
    view_menu.append("Pick grid font…", &frame, {
        let grid = cust.inner().clone();
        let frame = frame.clone();
        move || {
            let _ = grid.pick_font(&frame);
        }
    });

    let mut theme_menu = Menu::new("&Theme");
    for (name, theme) in [
        ("Win11", GridTheme::Win11),
        ("Modern", GridTheme::Modern),
        ("Warm", GridTheme::Warm),
        ("Dark", GridTheme::Dark),
        ("Classic", GridTheme::Classic),
    ] {
        theme_menu.append(name, &frame, {
            let cust = Rc::clone(&cust);
            let frame = frame.clone();
            let status = status.clone();
            move || {
                cust.set_theme(theme, &frame);
                status.set_status_text(&format!("Theme: {name}"), 0);
            }
        });
    }

    let mut menu_bar = MenuBar::new();
    menu_bar.append(data_menu);
    menu_bar.append(sort_menu);
    menu_bar.append(row_menu);
    menu_bar.append(grid_menu);
    menu_bar.append(label_menu);
    menu_bar.append(view_menu);
    menu_bar.append(theme_menu);
    frame.set_menu_bar(menu_bar);

    status.set_status_text(
        "Header click sort · row right-click · double-click divider auto-fit",
        1,
    );

    frame.on_key_down({
        let cust = Rc::clone(&cust);
        let footer = footer_label.clone();
        let status = status.clone();
        move |ev| {
            if ev.key_code == 0x74 {
                cust.refresh();
                footer.set_label(&cust.footer_summary());
                status.set_status_text("F5 — grid refreshed", 0);
            }
        }
    });

    frame.on_sys_colour_changed({
        let cust = Rc::clone(&cust);
        let frame = frame.clone();
        move |_| {
            frame.set_dark_title_bar(Appearance::System.resolve());
            cust.inner().apply_win11_theme(&frame);
            sync_accent_selection(&frame, cust.inner());
            cust.refresh();
        }
    });

    let mut root = BoxSizer::vertical();
    root.add_with_proportion(notebook.as_widget_ref(), 1);
    frame.set_sizer(root);

    app.run(frame);
}

fn count_checked(cust: &CustTableGrid) -> usize {
    let n = cust.table().borrow().row_count();
    (0..n).filter(|&r| cust.inner().is_checked(r)).count()
}
