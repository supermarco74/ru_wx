//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Demo: a minimal Windows-File-Explorer-style window built from
//! pure `ru_wx` widgets.
//!
//! Layout (vertical `BoxSizer`):
//!   ┌───────────────────────────────────────────────────────────┐
//!   │ [ ⬆ ] [ Copy ] [ Paste ] [ Delete ] [ Refresh ]           │  Toolbar
//!   ├──────────────────┬────────────────────────────────────────┤
//!   │  TreeCtrl        │  ListCtrl (Report)                     │
//!   │  (locations)     │  Name │ Type │ Size │ Modified          │
//!   │                  │  native multi-select                    │
//!   ├──────────────────┴────────────────────────────────────────┤
//!   │  StatusBar                                              │  Status
//!   └───────────────────────────────────────────────────────────┘
//!
//! Features shown:
//! - `TreeCtrl` populated with a known folder hierarchy, with a
//!   selection-change handler that refreshes the list pane.
//! - `ListCtrl::Report` with `LVS_EX_FULLROWSELECT` (auto-enabled
//!   by the library) → native Ctrl/Shift-click multi-select.
//! - `SplitterWindow::split_vertically` dividing the two panes
//!   (sash position is draggable).
//! - Toolbar with action buttons dispatched on `u16` tool ids.
//! - Right-click context menu on the frame with Open / Copy / Delete.
//! - File operations: copy-to-clipboard (paths serialised as
//!   newline-separated text — the library's `Clipboard` is text-only),
//!   paste-from-clipboard (parse + `fs::copy`), and multi-delete.
//! - Drag-and-drop of files from Windows Explorer via
//!   `frame.on_drop_files` (Shell-level WM_DROPFILES).

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use image::{ImageBuffer, Rgba};
use ru_wx::{
    message_box, App, Clipboard, ContextMenuEvent, Dialog, DropFilesEvent, Frame, ImageList,
    ListCtrl, ListCtrlStyle, MessageBoxIcon, MessageBoxStyle, PopupMenu, SpinCtrl,
    SplitterOrientation, SplitterWindow, StaticText, StatusBar, ToolBar, TreeCtrl, TreeItem,
};

// ─── Toolbar tool IDs ────────────────────────────────────────────────────────
const ID_TB_UP: u16 = 2001;
const ID_TB_COPY: u16 = 2002;
const ID_TB_PASTE: u16 = 2003;
const ID_TB_DELETE: u16 = 2004;
const ID_TB_REFRESH: u16 = 2005;
const ID_TB_SETTINGS: u16 = 2006;
const ID_TB_DUPLICATE: u16 = 2007;

// ─── Toolbar icons (PNG generated at runtime, 16×16) ─────────────────────────
//
// We don't embed SVG strings: small icons rasterised via resvg can look
// muddy at 16 px. Instead we draw pixel-art PNGs with the `image` crate
// at startup, encode to PNG bytes, and feed them to
// `ImageList::add_image_bytes`. The icons stay readable at toolbar size
// and avoid any resvg quirks.
//
// Each `icon_*` function returns the encoded PNG bytes for a single
// glyph. The drawing is hand-rolled pixel-by-pixel to keep the
// example dependency-free of extra crates (`imageproc` etc.).

/// Icon canvas size in pixels.
const ICON_W: u32 = 16;
const ICON_H: u32 = 16;

type RgbaBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;

/// Encode an `RgbaBuf` to PNG bytes using the `image` crate's PngEncoder.
fn encode_png(img: &RgbaBuf) -> Vec<u8> {
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    use image::ImageEncoder;
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .expect("PNG encode");
    out
}

fn blank_canvas() -> RgbaBuf {
    ImageBuffer::from_fn(ICON_W, ICON_H, |_, _| Rgba([0, 0, 0, 0]))
}

/// Plot a single pixel, ignoring out-of-bounds coordinates.
fn put(buf: &mut RgbaBuf, x: i32, y: i32, rgba: Rgba<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < buf.width() && (y as u32) < buf.height() {
        buf.put_pixel(x as u32, y as u32, rgba);
    }
}

/// Stroke a 1-pixel line from (x0,y0) to (x1,y1) using Bresenham.
fn line(buf: &mut RgbaBuf, x0: i32, y0: i32, x1: i32, y1: i32, rgba: Rgba<u8>) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        put(buf, x, y, rgba);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Stroke a rectangle outline.
fn rect_outline(buf: &mut RgbaBuf, x0: i32, y0: i32, x1: i32, y1: i32, rgba: Rgba<u8>) {
    line(buf, x0, y0, x1, y0, rgba);
    line(buf, x0, y1, x1, y1, rgba);
    line(buf, x0, y0, x0, y1, rgba);
    line(buf, x1, y0, x1, y1, rgba);
}

fn fill_rect(buf: &mut RgbaBuf, x0: i32, y0: i32, x1: i32, y1: i32, rgba: Rgba<u8>) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            put(buf, x, y, rgba);
        }
    }
}

/// Up-arrow: filled triangle + stem.
fn icon_up() -> Vec<u8> {
    let mut b = blank_canvas();
    let fg = Rgba([30, 30, 30, 255]);
    // Triangle: (8,2) → (2,8) → (14,8)
    for y in 2..=8 {
        let span = (8 - (8 - y) * 2).max(0);
        for x in (8 - span)..=(8 + span) {
            put(&mut b, x, y, fg);
        }
    }
    // Stem
    fill_rect(&mut b, 7, 9, 8, 14, fg);
    encode_png(&b)
}

/// Copy: two overlapping outlined rectangles.
fn icon_copy() -> Vec<u8> {
    let mut b = blank_canvas();
    let fg = Rgba([30, 30, 30, 255]);
    // Back doc (top-right)
    rect_outline(&mut b, 6, 2, 14, 9, fg);
    // Front doc (bottom-left, filled lighter)
    fill_rect(&mut b, 2, 6, 11, 14, Rgba([240, 240, 240, 255]));
    rect_outline(&mut b, 2, 6, 11, 14, fg);
    encode_png(&b)
}

/// Paste: clipboard rectangle + clip at top.
fn icon_paste() -> Vec<u8> {
    let mut b = blank_canvas();
    let fg = Rgba([30, 30, 30, 255]);
    // Clip on top
    fill_rect(&mut b, 6, 1, 10, 3, fg);
    // Body
    rect_outline(&mut b, 3, 3, 13, 14, fg);
    // Two lines of "text"
    line(&mut b, 5, 7, 11, 7, fg);
    line(&mut b, 5, 10, 11, 10, fg);
    line(&mut b, 5, 12, 9, 12, fg);
    encode_png(&b)
}

/// Delete: red X over a square.
fn icon_delete() -> Vec<u8> {
    let mut b = blank_canvas();
    let red = Rgba([204, 51, 51, 255]);
    rect_outline(&mut b, 2, 2, 13, 13, red);
    line(&mut b, 4, 4, 11, 11, red);
    line(&mut b, 11, 4, 4, 11, red);
    encode_png(&b)
}

/// Refresh: open circle with arrow head.
fn icon_refresh() -> Vec<u8> {
    let mut b = blank_canvas();
    let blue = Rgba([34, 102, 204, 255]);
    // Three-quarter circle (top, right, bottom) approximated with
    // horizontal/vertical line segments around (8, 8).
    let r = 5;
    let cx = 8;
    let cy = 8;
    // Top-left arc segment
    for deg in 200..=340 {
        let rad = (deg as f64).to_radians();
        let x = cx + (r as f64 * rad.cos()) as i32;
        let y = cy + (r as f64 * rad.sin()) as i32;
        put(&mut b, x, y, blue);
    }
    // Arrow head at top of circle pointing right
    fill_rect(&mut b, 8, 2, 10, 4, blue);
    line(&mut b, 8, 2, 12, 5, blue);
    line(&mut b, 12, 5, 8, 6, blue);
    encode_png(&b)
}

/// Folder (closed): rectangle with a top "tab".
fn icon_folder_closed() -> Vec<u8> {
    let mut b = blank_canvas();
    let body = Rgba([255, 196, 64, 255]); // yellow-amber
    let tab = Rgba([216, 158, 38, 255]);
    let edge = Rgba([120, 86, 22, 255]);
    // Folder tab (top-left)
    fill_rect(&mut b, 2, 3, 7, 5, tab);
    // Folder body
    fill_rect(&mut b, 2, 5, 13, 13, body);
    // Edges
    rect_outline(&mut b, 2, 5, 13, 13, edge);
    line(&mut b, 2, 5, 7, 5, edge);
    line(&mut b, 2, 5, 2, 13, edge);
    line(&mut b, 7, 5, 7, 3, edge);
    encode_png(&b)
}

/// Folder (open): rectangle with a slightly raised back edge.
fn icon_folder_open() -> Vec<u8> {
    let mut b = blank_canvas();
    let body = Rgba([255, 210, 96, 255]);
    let back = Rgba([230, 170, 50, 255]);
    let edge = Rgba([120, 86, 22, 255]);
    // Back panel (tilted)
    fill_rect(&mut b, 3, 4, 13, 12, back);
    // Front body
    fill_rect(&mut b, 2, 6, 12, 14, body);
    // Edges
    rect_outline(&mut b, 3, 4, 13, 12, edge);
    rect_outline(&mut b, 2, 6, 12, 14, edge);
    line(&mut b, 2, 6, 3, 4, edge);
    line(&mut b, 12, 6, 13, 4, edge);
    encode_png(&b)
}

/// Settings (gear/cog): 8-toothed gear with a centre hole.
fn icon_settings() -> Vec<u8> {
    let mut b = blank_canvas();
    let grey = Rgba([80, 80, 80, 255]);
    let cx = 8;
    let cy = 8;
    // Draw 8 teeth as small filled squares around the centre.
    for tooth in 0..8 {
        let deg = tooth * 45;
        let rad = (deg as f64).to_radians();
        let tx = cx + (5.5 * rad.cos()).round() as i32;
        let ty = cy + (5.5 * rad.sin()).round() as i32;
        fill_rect(&mut b, tx - 1, ty - 1, tx + 1, ty + 1, grey);
    }
    // Gear body (annulus): outer circle 4 px, inner hole 2 px
    for deg in 0..360 {
        let rad = (deg as f64).to_radians();
        let r_outer = 4.5;
        let r_inner = 2.0;
        let x = cx as f64 + r_outer * rad.cos();
        let y = cy as f64 + r_outer * rad.sin();
        put(&mut b, x.round() as i32, y.round() as i32, grey);
        let x = cx as f64 + r_inner * rad.cos();
        let y = cy as f64 + r_inner * rad.sin();
        // clear the inner hole (draw transparent)
        if (x.round() as i32) >= 0 && (y.round() as i32) >= 0 {
            let xi = x.round() as i32;
            let yi = y.round() as i32;
            if xi < ICON_W as i32 && yi < ICON_H as i32 {
                b.put_pixel(xi as u32, yi as u32, Rgba([0, 0, 0, 0]));
            }
        }
    }
    // Centre hole: explicit 2x2 transparent square
    fill_rect(&mut b, cx - 1, cy - 1, cx + 1, cy + 1, Rgba([0, 0, 0, 0]));
    encode_png(&b)
}

/// Hard-disk drive: rectangle with a small status bar and LED.
fn icon_drive_fixed() -> Vec<u8> {
    let mut b = blank_canvas();
    let body = Rgba([60, 90, 140, 255]); // blue-steel
    let edge = Rgba([20, 40, 80, 255]);
    let led = Rgba([80, 220, 120, 255]); // green LED
                                         // Outer case
    fill_rect(&mut b, 2, 3, 14, 12, body);
    rect_outline(&mut b, 2, 3, 14, 12, edge);
    // Activity LED
    fill_rect(&mut b, 12, 5, 13, 6, led);
    // Disk activity bar
    fill_rect(&mut b, 3, 8, 11, 11, Rgba([40, 60, 100, 255]));
    rect_outline(&mut b, 3, 8, 11, 11, edge);
    // Brand stripe
    line(&mut b, 3, 6, 11, 6, edge);
    encode_png(&b)
}

/// Removable drive (USB stick): thin rectangle with a USB tip.
fn icon_drive_removable() -> Vec<u8> {
    let mut b = blank_canvas();
    let body = Rgba([80, 80, 90, 255]); // dark grey
    let edge = Rgba([30, 30, 35, 255]);
    let metal = Rgba([180, 180, 190, 255]); // silver USB tip
    fill_rect(&mut b, 2, 5, 13, 11, body);
    rect_outline(&mut b, 2, 5, 13, 11, edge);
    fill_rect(&mut b, 13, 7, 14, 9, metal);
    rect_outline(&mut b, 13, 7, 14, 9, edge);
    // Activity LED
    put(&mut b, 4, 7, Rgba([255, 200, 50, 255]));
    encode_png(&b)
}

/// Network drive: cloud-ish shape (3 overlapping circles + base).
fn icon_drive_network() -> Vec<u8> {
    let mut b = blank_canvas();
    let body = Rgba([100, 160, 220, 255]);
    let edge = Rgba([40, 80, 130, 255]);
    // Three cloud puffs
    fill_rect(&mut b, 4, 6, 12, 12, body);
    fill_rect(&mut b, 2, 8, 6, 10, body);
    fill_rect(&mut b, 10, 8, 14, 10, body);
    fill_rect(&mut b, 5, 4, 11, 8, body);
    rect_outline(&mut b, 4, 6, 12, 12, edge);
    line(&mut b, 2, 8, 4, 8, edge);
    line(&mut b, 12, 8, 14, 8, edge);
    encode_png(&b)
}

/// CDROM / DVD drive: rectangle with a centre hole (disc).
fn icon_drive_cdrom() -> Vec<u8> {
    let mut b = blank_canvas();
    let body = Rgba([200, 200, 210, 255]);
    let edge = Rgba([80, 80, 90, 255]);
    let disc = Rgba([230, 230, 240, 255]);
    fill_rect(&mut b, 2, 4, 14, 11, body);
    rect_outline(&mut b, 2, 4, 14, 11, edge);
    // Disc slot
    fill_rect(&mut b, 4, 6, 12, 9, disc);
    rect_outline(&mut b, 4, 6, 12, 9, edge);
    // Centre hole
    fill_rect(&mut b, 7, 8, 9, 9, edge);
    encode_png(&b)
}

/// Duplicate: two overlapping rectangles (like Copy) with a small
/// "+" badge in the top-right corner.
fn icon_duplicate() -> Vec<u8> {
    let mut b = blank_canvas();
    let fg = Rgba([30, 30, 30, 255]);
    let accent = Rgba([220, 80, 60, 255]); // reddish + badge
                                           // Back rectangle (offset top-right)
    rect_outline(&mut b, 7, 2, 14, 9, fg);
    // Front rectangle (filled light, outlined)
    fill_rect(&mut b, 2, 6, 10, 14, Rgba([240, 240, 240, 255]));
    rect_outline(&mut b, 2, 6, 10, 14, fg);
    // "+" badge in the top-right corner (red)
    fill_rect(&mut b, 11, 4, 14, 5, accent);
    fill_rect(&mut b, 12, 3, 13, 6, accent);
    encode_png(&b)
}

// ─── Menu icons (16×16, more detailed than toolbar 16 px) ───────────────────
//
// These PNGs are attached to context-menu items via the new
// `append_with_png_icon` helper. They use bolder strokes and richer
// colour so they stay readable at menu size on standard DPI.

/// Open: a folder with an arrow flying out of it.
fn icon_menu_open() -> Vec<u8> {
    let mut b = blank_canvas();
    let folder = Rgba([255, 196, 64, 255]);
    let tab = Rgba([216, 158, 38, 255]);
    let edge = Rgba([80, 60, 20, 255]);
    let arrow = Rgba([30, 144, 255, 255]);
    // Folder tab + body
    fill_rect(&mut b, 1, 5, 6, 7, tab);
    fill_rect(&mut b, 1, 7, 11, 14, folder);
    rect_outline(&mut b, 1, 5, 6, 7, edge);
    rect_outline(&mut b, 1, 7, 11, 14, edge);
    // Arrow exiting the folder, going up-right
    fill_rect(&mut b, 9, 2, 10, 3, arrow);
    fill_rect(&mut b, 13, 4, 14, 5, arrow);
    fill_rect(&mut b, 9, 2, 14, 3, arrow);
    fill_rect(&mut b, 13, 2, 14, 6, arrow);
    encode_png(&b)
}

/// Copy: two stacked document pages (the canonical "two pages" icon).
fn icon_menu_copy() -> Vec<u8> {
    let mut b = blank_canvas();
    let edge = Rgba([80, 80, 90, 255]);
    let paper = Rgba([250, 250, 252, 255]);
    // Back page (top-right)
    fill_rect(&mut b, 5, 1, 13, 11, paper);
    rect_outline(&mut b, 5, 1, 13, 11, edge);
    // Front page (bottom-left, full)
    fill_rect(&mut b, 1, 4, 9, 14, paper);
    rect_outline(&mut b, 1, 4, 9, 14, edge);
    // Subtle text lines on the front page
    line(&mut b, 3, 7, 7, 7, Rgba([180, 180, 190, 255]));
    line(&mut b, 3, 9, 7, 9, Rgba([180, 180, 190, 255]));
    line(&mut b, 3, 11, 6, 11, Rgba([180, 180, 190, 255]));
    encode_png(&b)
}

/// Paste: clipboard with a clip and an upward arrow.
fn icon_menu_paste() -> Vec<u8> {
    let mut b = blank_canvas();
    let board = Rgba([255, 200, 96, 255]);
    let paper = Rgba([250, 250, 252, 255]);
    let edge = Rgba([100, 70, 20, 255]);
    let arrow = Rgba([30, 144, 255, 255]);
    // Clipboard body
    fill_rect(&mut b, 3, 5, 13, 14, board);
    rect_outline(&mut b, 3, 5, 13, 14, edge);
    // Clip at top
    fill_rect(&mut b, 6, 2, 10, 5, edge);
    fill_rect(&mut b, 7, 3, 9, 4, Rgba([180, 180, 190, 255]));
    // Inner paper
    fill_rect(&mut b, 5, 7, 11, 11, paper);
    rect_outline(&mut b, 5, 7, 11, 11, edge);
    // Upward arrow on the paper (paste = data flowing in)
    fill_rect(&mut b, 7, 8, 9, 9, arrow);
    fill_rect(&mut b, 6, 9, 10, 10, arrow);
    put(&mut b, 5, 10, arrow);
    put(&mut b, 11, 10, arrow);
    encode_png(&b)
}

/// Delete (menu variant, bolder than toolbar): red cross on white square.
fn icon_menu_delete() -> Vec<u8> {
    let mut b = blank_canvas();
    let bg = Rgba([255, 255, 255, 255]);
    let red = Rgba([220, 50, 50, 255]);
    let edge = Rgba([140, 30, 30, 255]);
    fill_rect(&mut b, 2, 2, 14, 14, bg);
    rect_outline(&mut b, 2, 2, 14, 14, edge);
    // Thick red X
    for i in 0..3 {
        let d = i;
        line(&mut b, 4 + d, 4 + d, 12 + d, 12 + d, red);
        line(&mut b, 12 - d, 4 + d, 4 - d, 12 + d, red);
    }
    encode_png(&b)
}

/// Duplicate (menu variant): two rectangles + red "+".
fn icon_menu_duplicate() -> Vec<u8> {
    let mut b = blank_canvas();
    let fg = Rgba([30, 30, 30, 255]);
    let accent = Rgba([220, 60, 60, 255]);
    fill_rect(&mut b, 7, 1, 14, 9, Rgba([250, 250, 250, 255]));
    rect_outline(&mut b, 7, 1, 14, 9, fg);
    fill_rect(&mut b, 1, 5, 9, 14, Rgba([255, 255, 255, 255]));
    rect_outline(&mut b, 1, 5, 9, 14, fg);
    // Big red + in the bottom-right
    fill_rect(&mut b, 9, 10, 14, 12, accent);
    fill_rect(&mut b, 11, 8, 12, 14, accent);
    encode_png(&b)
}

// ─── Application state (held in Rc<RefCell<>> so handlers can share) ────────
struct AppState {
    current_dir: PathBuf,
    /// Paths staged with Ctrl+C — kept in addition to the OS clipboard so
    /// in-app paste works even when the user copies text afterwards.
    clipboard: Vec<PathBuf>,
}

/// Mirror `TreeItem → PathBuf` (the widget doesn't expose user data).
#[derive(Default)]
struct TreePayload {
    map: Vec<(TreeItem, PathBuf)>,
}

impl TreePayload {
    fn add(&mut self, item: TreeItem, path: PathBuf) {
        self.map.push((item, path));
    }
    fn lookup(&self, item: TreeItem) -> Option<PathBuf> {
        self.map
            .iter()
            .find(|(it, _)| *it == item)
            .map(|(_, p)| p.clone())
    }
}

/// Resolve the user's known folders (Windows: USERPROFILE, others: HOME).
fn known_locations() -> Vec<(&'static str, PathBuf)> {
    let mut out = Vec::new();
    let profile = std::env::var("USERPROFILE").ok().map(PathBuf::from);
    let home = profile
        .clone()
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from));
    if let Some(p) = profile.as_ref() {
        out.push(("Desktop", p.join("Desktop")));
        out.push(("Documents", p.join("Documents")));
        out.push(("Downloads", p.join("Downloads")));
        out.push(("Pictures", p.join("Pictures")));
        out.push(("Music", p.join("Music")));
        out.push(("Videos", p.join("Videos")));
    }
    if let Some(p) = home {
        out.push(("Home", p));
    }
    out
}

// ─── Main entry point ────────────────────────────────────────────────────────
fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx · File Explorer Demo")
        .with_size(960, 600)
        .with_modern_tabbed_style() // Mica Alt + rounded corners + dark title bar
        .build();

    // Start in the user's home directory.
    let initial_dir = known_locations()
        .into_iter()
        .find(|(name, _)| *name == "Home")
        .map(|(_, p)| p)
        .unwrap_or_else(|| PathBuf::from("."));

    let state = Rc::new(RefCell::new(AppState {
        current_dir: initial_dir.clone(),
        clipboard: Vec::new(),
    }));
    let tree_payload = Rc::new(RefCell::new(TreePayload::default()));

    // ── Status bar (auto-positions at the bottom) ───────────────────────
    let status = StatusBar::new(&frame, 1);
    set_status(&status, &initial_dir.display().to_string());

    // ── Toolbar (basic `ToolBar`) with icon image list ─────────────────
    // Build the image list first so the toolbar can reference icons
    // by index. We use 16×16 to match the toolbar's default button size.
    let images = ImageList::new(16, 16);
    let img_up = images.add_image_bytes(&icon_up());
    let img_copy = images.add_image_bytes(&icon_copy());
    let img_paste = images.add_image_bytes(&icon_paste());
    let img_delete = images.add_image_bytes(&icon_delete());
    let img_refresh = images.add_image_bytes(&icon_refresh());
    let img_settings = images.add_image_bytes(&icon_settings());
    let img_duplicate = images.add_image_bytes(&icon_duplicate());

    let toolbar = ToolBar::new(&frame);
    toolbar.set_image_list(&images);
    toolbar.add_tool(ID_TB_UP, "Up to parent folder", img_up.unwrap_or(-1));
    toolbar.add_separator();
    toolbar.add_tool(ID_TB_COPY, "Copy selection", img_copy.unwrap_or(-1));
    toolbar.add_tool(ID_TB_PASTE, "Paste from clipboard", img_paste.unwrap_or(-1));
    toolbar.add_tool(ID_TB_DELETE, "Delete selection", img_delete.unwrap_or(-1));
    toolbar.add_separator();
    toolbar.add_tool(ID_TB_REFRESH, "Refresh view", img_refresh.unwrap_or(-1));
    toolbar.add_separator();
    toolbar.add_tool(
        ID_TB_DUPLICATE,
        "Duplicate selection",
        img_duplicate.unwrap_or(-1),
    );
    toolbar.add_separator();
    toolbar.add_tool(ID_TB_SETTINGS, "Settings…", img_settings.unwrap_or(-1));
    toolbar.realize();
    // Pin the toolbar across the top of the frame (28 px tall).
    toolbar.as_widget_ref().borrow_mut().set_position(0, 0);
    toolbar.as_widget_ref().borrow_mut().set_size(960, 28);

    // ── Splitter + Tree + List ──────────────────────────────────────────
    // Layout: toolbar at top (0..28), splitter at (0, 28) sized to
    // (960, 550). The status bar auto-reserves the bottom 22 px.
    //
    // IMPORTANT: `SplitterWindow::layout_panes` calls `MoveWindow(pane, 0, 0, …)`
    // for both panes, which positions them relative to **whatever
    // window the panes are parented to**. Since `TreeCtrl::new` and
    // `ListCtrl::new` take `&Frame` (not `&SplitterWindow`), the panes
    // would end up at (0, 0) of the frame and overlap the toolbar.
    //
    // Fix: re-parent both panes to the splitter via `SetParent` BEFORE
    // calling `split_vertically`. After that, `layout_panes` positions
    // them at (0, 0) of the splitter — i.e. (0, 28) of the frame.
    let splitter = SplitterWindow::new(&frame);
    splitter.set_orientation(SplitterOrientation::Vertical);
    splitter.set_sash_position(240);
    splitter.as_widget_ref().borrow_mut().set_position(0, 28);
    splitter.as_widget_ref().borrow_mut().set_size(960, 550);

    let tree = TreeCtrl::new(&frame);
    tree.as_widget_ref().borrow_mut().set_size(240, 550);
    // Build an ImageList for the tree with a closed-folder + open-folder icon.
    let tree_imgs = ImageList::new(16, 16);
    let img_folder_closed = tree_imgs.add_image_bytes(&icon_folder_closed());
    let img_folder_open = tree_imgs.add_image_bytes(&icon_folder_open());
    let img_drive_fixed = tree_imgs.add_image_bytes(&icon_drive_fixed());
    let img_drive_remove = tree_imgs.add_image_bytes(&icon_drive_removable());
    let img_drive_cdrom = tree_imgs.add_image_bytes(&icon_drive_cdrom());
    let img_drive_network = tree_imgs.add_image_bytes(&icon_drive_network());
    tree.set_image_list(&tree_imgs);

    let list = ListCtrl::new(&frame, ListCtrlStyle::Report);
    list.as_widget_ref().borrow_mut().set_size(720, 550);
    list.insert_column(0, "Name", 220);
    list.insert_column(1, "Type", 100);
    list.insert_column(2, "Size", 100);
    list.insert_column(3, "Modified", 180);
    // Build an ImageList for the list, wrapped in Rc<RefCell<>> so the
    // closures below can add icons lazily while sharing the same list.
    let list_imgs: Rc<RefCell<ImageList>> = Rc::new(RefCell::new(ImageList::new(16, 16)));
    let img_folder_idx = list_imgs.borrow().add_image_bytes(&icon_folder_closed());
    list.set_image_list(&list_imgs.borrow());
    // Cache mapping extension → ImageList index (built lazily as we
    // encounter each extension while populating).
    let icon_cache: Rc<RefCell<HashMap<String, i32>>> = Rc::new(RefCell::new(HashMap::new()));

    // Re-parent the panes to the splitter on Windows so `layout_panes`
    // coordinates are interpreted relative to the splitter, not the frame.
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::SetParent;
        let tree_hwnd = tree.as_widget_ref().borrow().native_handle() as HWND;
        let list_hwnd = list.as_widget_ref().borrow().native_handle() as HWND;
        let splitter_hwnd = splitter.as_widget_ref().borrow().native_handle() as HWND;
        // SAFETY: all three HWNDs are live child windows owned by
        // the same thread. SetParent updates the parent and re-emits
        // WM_SHOWWINDOW for the new child.
        unsafe {
            SetParent(tree_hwnd, splitter_hwnd);
            SetParent(list_hwnd, splitter_hwnd);
        }
        splitter.split_vertically(tree_hwnd, list_hwnd);
    }
    #[cfg(not(target_os = "windows"))]
    {
        splitter.split_vertically(
            tree.as_widget_ref().borrow().native_handle(),
            list.as_widget_ref().borrow().native_handle(),
        );
    }

    // ── Populate the tree ──────────────────────────────────────────────
    populate_tree(
        &tree,
        &tree_payload,
        img_folder_closed.unwrap_or(-1),
        img_folder_open.unwrap_or(-1),
        img_drive_fixed.unwrap_or(-1),
        img_drive_remove.unwrap_or(-1),
        img_drive_cdrom.unwrap_or(-1),
        img_drive_network.unwrap_or(-1),
    );
    if let Some(root) = tree.get_root_item() {
        tree.expand(root);
    }

    // ── Initial list fill ──────────────────────────────────────────────
    populate_list(
        &list,
        &initial_dir,
        img_folder_idx.unwrap_or(-1),
        &icon_cache,
        &list_imgs.borrow(),
    );

    // ── Tree → List selection bridge ───────────────────────────────────
    let state_t = Rc::clone(&state);
    let payload_t = Rc::clone(&tree_payload);
    let list_for_tree = list.clone();
    let status_for_tree = status.clone();
    let img_folder_idx_c: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(img_folder_idx));
    let icon_cache_c: Rc<RefCell<HashMap<String, i32>>> = Rc::new(RefCell::new(HashMap::new()));
    let list_imgs_c: Rc<RefCell<ImageList>> = Rc::clone(&list_imgs);
    tree.on_selection_change(&frame, move |maybe_item| {
        let Some(item) = maybe_item else { return };
        if let Some(path) = payload_t.borrow().lookup(item) {
            if path.is_dir() {
                state_t.borrow_mut().current_dir = path.clone();
                populate_list(
                    &list_for_tree,
                    &path,
                    img_folder_idx_c.borrow().unwrap_or(-1),
                    &icon_cache_c,
                    &list_imgs_c.borrow(),
                );
                // Build the status text: if the path is a drive root
                // like "C:\" or "D:\", append free-space info.
                let path_str = path.display().to_string();
                let status_msg = if path_str.len() == 3
                    && path_str.ends_with(":\\")
                    && path_str.chars().nth(1) == Some(':')
                {
                    // Looks like a drive root — query free space.
                    let letter = path_str.chars().next().unwrap_or('?');
                    match drive_free_space(letter) {
                        Some((free, total)) => {
                            format!("{} — {}", path_str, format_disk_space(free, total))
                        }
                        None => path_str,
                    }
                } else {
                    path_str
                };
                set_status(&status_for_tree, &status_msg);
            }
        }
    });

    // ── Toolbar dispatch ───────────────────────────────────────────────
    let state_tb = Rc::clone(&state);
    let list_for_tb = list.clone();
    let status_for_tb = status.clone();
    let frame_for_tb = frame.clone();
    let img_folder_idx_tb: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(img_folder_idx));
    let icon_cache_tb: Rc<RefCell<HashMap<String, i32>>> = Rc::new(RefCell::new(HashMap::new()));
    let list_imgs_tb: Rc<RefCell<ImageList>> = Rc::clone(&list_imgs);
    toolbar.on_tool_clicked(&frame, move |id| match id {
        ID_TB_UP => {
            let parent = state_tb
                .borrow()
                .current_dir
                .parent()
                .map(|p| p.to_path_buf());
            if let Some(p) = parent {
                state_tb.borrow_mut().current_dir = p.clone();
                populate_list(
                    &list_for_tb,
                    &p,
                    img_folder_idx_tb.borrow().unwrap_or(-1),
                    &icon_cache_tb,
                    &list_imgs_tb.borrow(),
                );
                set_status(&status_for_tb, &p.display().to_string());
            }
        }
        ID_TB_COPY => {
            let paths = collect_selected_paths(&list_for_tb);
            if !paths.is_empty() {
                let _ = Clipboard::set_text(&serialise_paths(&paths));
                state_tb.borrow_mut().clipboard = paths.clone();
                set_status(&status_for_tb, &format!("Copied {} item(s).", paths.len()));
            }
        }
        ID_TB_PASTE => {
            let text = Clipboard::get_text().unwrap_or_default();
            let src = parse_paths(&text);
            let dst = state_tb.borrow().current_dir.clone();
            let count = paste_into(&src, &dst);
            populate_list(
                &list_for_tb,
                &dst,
                img_folder_idx_tb.borrow().unwrap_or(-1),
                &icon_cache_tb,
                &list_imgs_tb.borrow(),
            );
            set_status(
                &status_for_tb,
                &format!("Pasted {} item(s) into {}.", count, dst.display()),
            );
        }
        ID_TB_DELETE => {
            let paths = collect_selected_paths(&list_for_tb);
            if paths.is_empty() {
                return;
            }
            let msg = format!("Delete {} item(s)? This cannot be undone.", paths.len());
            if confirm(&frame_for_tb, "Confirm delete", &msg) {
                let n = delete_paths(&paths);
                populate_list(
                    &list_for_tb,
                    &state_tb.borrow().current_dir,
                    img_folder_idx_tb.borrow().unwrap_or(-1),
                    &icon_cache_tb,
                    &list_imgs_tb.borrow(),
                );
                set_status(&status_for_tb, &format!("Deleted {} item(s).", n));
            }
        }
        ID_TB_REFRESH => {
            let cur = state_tb.borrow().current_dir.clone();
            populate_list(
                &list_for_tb,
                &cur,
                img_folder_idx_tb.borrow().unwrap_or(-1),
                &icon_cache_tb,
                &list_imgs_tb.borrow(),
            );
            set_status(&status_for_tb, &cur.display().to_string());
        }
        ID_TB_DUPLICATE => {
            let paths = collect_selected_paths(&list_for_tb);
            if paths.is_empty() {
                return;
            }
            let pairs = duplicate_paths(&paths);
            let cur = state_tb.borrow().current_dir.clone();
            populate_list(
                &list_for_tb,
                &cur,
                img_folder_idx_tb.borrow().unwrap_or(-1),
                &icon_cache_tb,
                &list_imgs_tb.borrow(),
            );
            // Status message: list the new names for confirmation.
            let summary = pairs
                .iter()
                .map(|(_, dst)| {
                    dst.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>()
                .join(", ");
            set_status(
                &status_for_tb,
                &format!("Duplicated {} item(s): {summary}", pairs.len()),
            );
        }
        ID_TB_SETTINGS => {
            show_settings_dialog(&frame_for_tb, &list_for_tb, &status_for_tb);
        }
        _ => {}
    });

    // ── Right-click context menu on the frame ──────────────────────────
    let state_ctx = Rc::clone(&state);
    let img_folder_idx_c_ctx: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(img_folder_idx));
    let icon_cache_c_ctx: Rc<RefCell<HashMap<String, i32>>> = Rc::new(RefCell::new(HashMap::new()));
    let list_imgs_c_ctx: Rc<RefCell<ImageList>> = Rc::clone(&list_imgs);
    let frame_for_ctx = frame.clone();
    let list_for_ctx = list.clone();
    let status_for_ctx = status.clone();
    frame.on_context_menu(move |ev: &ContextMenuEvent| {
        let p = ev.position;
        let mut popup = PopupMenu::new();
        // The menu callbacks must be `FnMut() + 'static`, so we clone
        // every captured widget into each callback to drop the
        // outer-closure borrow.
        // Menu icons (16×16 PNG bytes). Generated once per popup and
        // re-used by every callback below.
        let png_open = icon_menu_open();
        let png_copy = icon_menu_copy();
        let _png_paste = icon_menu_paste();
        let png_duplicate = icon_menu_duplicate();
        let png_delete = icon_menu_delete();

        let list_a = list_for_ctx.clone();
        popup.append_with_png_icon("Open", &png_open, 16, &frame_for_ctx, move || {
            if let Some(idx) = list_a.get_selected_item() {
                if let Some(p) = list_path_at(&list_a, idx) {
                    let _ = open_in_explorer(&p);
                }
            }
        });
        let list_b = list_for_ctx.clone();
        let status_b = status_for_ctx.clone();
        let state_b = Rc::clone(&state_ctx);
        popup.append_with_png_icon("Copy", &png_copy, 16, &frame_for_ctx, move || {
            let paths = collect_selected_paths(&list_b);
            if !paths.is_empty() {
                let _ = Clipboard::set_text(&serialise_paths(&paths));
                state_b.borrow_mut().clipboard = paths;
                set_status(&status_b, "Copied via context menu.");
            }
        });
        // ── Duplicate ───────────────────────────────────────────────
        let list_d = list_for_ctx.clone();
        let status_d = status_for_ctx.clone();
        let state_d = Rc::clone(&state_ctx);
        let icon_idx_d_ctx = Rc::clone(&img_folder_idx_c_ctx);
        let cache_d = Rc::clone(&icon_cache_c_ctx);
        let imgs_d = Rc::clone(&list_imgs_c_ctx);
        popup.append_with_png_icon("Duplicate", &png_duplicate, 16, &frame_for_ctx, move || {
            let paths = collect_selected_paths(&list_d);
            if paths.is_empty() {
                return;
            }
            let pairs = duplicate_paths(&paths);
            let cur = state_d.borrow().current_dir.clone();
            populate_list(
                &list_d,
                &cur,
                icon_idx_d_ctx.borrow().unwrap_or(-1),
                &cache_d,
                &imgs_d.borrow(),
            );
            let summary = pairs
                .iter()
                .map(|(_, dst)| {
                    dst.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>()
                .join(", ");
            set_status(
                &status_d,
                &format!("Duplicated {} item(s): {summary}", pairs.len()),
            );
        });
        let list_c = list_for_ctx.clone();
        let status_c = status_for_ctx.clone();
        let state_c = Rc::clone(&state_ctx);
        let frame_c = frame_for_ctx.clone();
        let icon_idx_c = Rc::clone(&img_folder_idx_c_ctx);
        let cache_c = Rc::clone(&icon_cache_c_ctx);
        let imgs_c = Rc::clone(&list_imgs_c_ctx);
        popup.append_with_png_icon("Delete", &png_delete, 16, &frame_for_ctx, move || {
            let paths = collect_selected_paths(&list_c);
            if paths.is_empty() {
                return;
            }
            let count = paths.len();
            let msg = format!("Delete {count} item(s)?");
            if confirm(&frame_c, "Confirm delete", &msg) {
                let n = delete_paths(&paths);
                populate_list(
                    &list_c,
                    &state_c.borrow().current_dir,
                    icon_idx_c.borrow().unwrap_or(-1),
                    &cache_c,
                    &imgs_c.borrow(),
                );
                set_status(&status_c, &format!("Deleted {n} item(s)."));
            }
        });
        popup.popup_at(&frame_for_ctx, p.x, p.y);
    });

    // ── Drag-and-drop from Explorer (Shell protocol, Windows-only) ─────
    let state_drop = Rc::clone(&state);
    let img_folder_idx_d: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(img_folder_idx));
    let icon_cache_d: Rc<RefCell<HashMap<String, i32>>> = Rc::new(RefCell::new(HashMap::new()));
    let list_imgs_d: Rc<RefCell<ImageList>> = Rc::clone(&list_imgs);
    let list_for_drop = list.clone();
    let status_for_drop = status.clone();
    frame.on_drop_files(move |ev: &DropFilesEvent| {
        let dst = state_drop.borrow().current_dir.clone();
        let count = paste_into(ev.paths(), &dst);
        populate_list(
            &list_for_drop,
            &dst,
            img_folder_idx_d.borrow().unwrap_or(-1),
            &icon_cache_d,
            &list_imgs_d.borrow(),
        );
        set_status(
            &status_for_drop,
            &format!("Dropped {} file(s) into {}.", count, dst.display()),
        );
    });

    app.run(frame);
}

// ─── Tree + list population ──────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)] // drive icon indices are passed individually for clarity
fn populate_tree(
    tree: &TreeCtrl,
    payload: &Rc<RefCell<TreePayload>>,
    img_folder_closed: i32,
    img_folder_open: i32,
    img_drive_fixed: i32,
    img_drive_remove: i32,
    img_drive_cdrom: i32,
    img_drive_network: i32,
) {
    use drive_type::*;
    // Drives first (roots). Each drive gets its own icon based on
    // its kind so the user can tell HDD / USB / CD / Net at a glance.
    for drive in enumerate_drives() {
        let icon = match drive.kind {
            DRIVE_FIXED => img_drive_fixed,
            DRIVE_REMOVABLE => img_drive_remove,
            DRIVE_CDROM => img_drive_cdrom,
            DRIVE_REMOTE => img_drive_network,
            _ => img_folder_closed,
        };
        let r = tree.add_root_with_image(&drive.label, icon);
        payload.borrow_mut().add(r, drive.root);
    }
    // Then the user-folder shortcuts (Documents, Downloads, …) as
    // children of the first fixed drive (typically C:).
    let fixed_root: Option<TreeItem> = enumerate_drives()
        .into_iter()
        .find(|d| d.kind == DRIVE_FIXED)
        .and_then(|d| {
            // Find the item we just added that maps to the same path.
            payload
                .borrow()
                .map
                .iter()
                .find(|(_, p)| **p == d.root)
                .map(|(it, _)| *it)
        });
    for (label, path) in known_locations() {
        let item = match fixed_root {
            Some(root) => tree.append_item_with_image(root, label, img_folder_closed),
            None => tree.add_root_with_image(label, img_folder_open),
        };
        payload.borrow_mut().add(item, path);
    }
}

fn populate_list(
    list: &ListCtrl,
    dir: &Path,
    img_folder_idx: i32,
    icon_cache: &Rc<RefCell<HashMap<String, i32>>>,
    list_imgs: &ImageList,
) {
    list.delete_all_items();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            list.insert_item(0, &format!("<cannot read: {err}>"));
            return;
        }
    };
    for (idx, entry) in entries.flatten().enumerate() {
        let path = entry.path();
        let is_dir = path.is_dir();
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = if is_dir {
            "Folder".to_string()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().into_owned())
                .map(|e| format!("{e} file"))
                .unwrap_or_else(|| "File".to_string())
        };
        let size = entry
            .metadata()
            .ok()
            .map(|m| {
                if m.is_dir() {
                    String::new()
                } else {
                    human_size(m.len())
                }
            })
            .unwrap_or_default();
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(format_system_time)
            .unwrap_or_default();

        // Resolve the icon index: folders always use the default PNG
        // folder; files look up the extension in the cache, or extract
        // the system icon on cache miss and add it to the list's
        // ImageList.
        let icon_idx = if is_dir {
            img_folder_idx
        } else {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_else(|| "<no_ext>".to_string());
            // Look up the cache, dropping the borrow before calling
            // add_image_bytes (which itself may borrow the cache).
            let cached = icon_cache.borrow().get(&ext).copied();
            if let Some(c) = cached {
                c
            } else {
                let idx = extract_shell_icon_hicon(&path, false)
                    .and_then(hicon_to_png_bytes)
                    .and_then(|png| list_imgs.add_image_bytes(&png))
                    .unwrap_or(-1);
                if idx >= 0 {
                    icon_cache.borrow_mut().insert(ext.clone(), idx);
                }
                idx
            }
        };

        let new_idx = if icon_idx >= 0 {
            list.insert_item_with_image(idx, &name, icon_idx)
        } else {
            list.insert_item(idx, &name)
        };
        list.set_item_text(new_idx, 1, &file_type);
        list.set_item_text(new_idx, 2, &size);
        list.set_item_text(new_idx, 3, &modified);
    }
}

/// Resolve all currently-selected rows to their absolute paths by
/// joining each row name with the current directory. (We don't
/// use `LVITEM::lParam` because the library's `insert_item` API
/// doesn't expose a tag-setter; the demo keeps the join trivial.)
fn collect_selected_paths(list: &ListCtrl) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let sel = list.get_selected_items();
    for idx in sel {
        let name = list.get_item_text(idx, 0);
        if !name.is_empty() {
            paths.push(PathBuf::from(name));
        }
    }
    paths
}

fn list_path_at(list: &ListCtrl, idx: usize) -> Option<PathBuf> {
    let name = list.get_item_text(idx, 0);
    if name.is_empty() {
        None
    } else {
        Some(PathBuf::from(name))
    }
}

// ─── Formatting ──────────────────────────────────────────────────────────────

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

fn format_system_time(t: SystemTime) -> String {
    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let (year, month, day, hour, minute, second) = epoch_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

/// Crude epoch -> calendar conversion (Gregorian). No external chrono dep.
fn epoch_to_ymdhms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let second = (secs % 60) as u32;
    let minutes_total = secs / 60;
    let minute = (minutes_total % 60) as u32;
    let hours_total = minutes_total / 60;
    let hour = (hours_total % 24) as u32;
    let mut days = hours_total / 24;
    let mut year: u32 = 1970;
    loop {
        let leap = is_leap(year);
        let yd = if leap { 366 } else { 365 };
        if days >= yd {
            days -= yd;
            year += 1;
        } else {
            break;
        }
    }
    let mdays = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &dm in &mdays {
        if days >= dm as u64 {
            days -= dm as u64;
            month += 1;
        } else {
            break;
        }
    }
    let day = (days as u32) + 1;
    (year, month, day, hour, minute, second)
}

fn is_leap(y: u32) -> bool {
    y.is_multiple_of(4) && !y.is_multiple_of(100) || y.is_multiple_of(400)
}

// ─── Clipboard & file operations ────────────────────────────────────────────

fn serialise_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_paths(text: &str) -> Vec<PathBuf> {
    text.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn paste_into(src: &[PathBuf], dst: &Path) -> usize {
    let mut count = 0;
    for s in src {
        let Some(name) = s.file_name() else { continue };
        let target = dst.join(name);
        let result = if s.is_dir() {
            copy_dir_recursive(s, &target)
        } else {
            fs::copy(s, &target).map(|_| ())
        };
        if result.is_ok() {
            count += 1;
        }
    }
    count
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn delete_paths(paths: &[PathBuf]) -> usize {
    let mut n = 0;
    for p in paths {
        let result = if p.is_dir() {
            fs::remove_dir_all(p)
        } else {
            fs::remove_file(p)
        };
        if result.is_ok() {
            n += 1;
        }
    }
    n
}

// ─── UI helpers ──────────────────────────────────────────────────────────────

fn confirm(frame: &Frame, title: &str, msg: &str) -> bool {
    let res = message_box(
        frame,
        msg,
        title,
        MessageBoxStyle::YesNo,
        MessageBoxIcon::Warning,
    );
    matches!(res, ru_wx::MessageBoxResult::Yes)
}

fn open_in_explorer(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(path)
            .spawn()?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
        Ok(())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
        Ok(())
    }
}

fn set_status(status: &StatusBar, msg: &str) {
    status.set_status_text(&format!(" {msg}"), 0);
}

// ─── System icon extraction (SHGetFileInfoW) ─────────────────────────────────
//
// On Windows the Shell exposes per-extension icons via `SHGetFileInfoW`.
// We extract a 16×16 HICON for each file (or attribute) and feed it to
// the list's `ImageList` via the library's `add_image_bytes` helper.
// To avoid re-extracting the same icon many times during a single
// session, the extracted-HBITMAP-to-ImageList-index mapping is kept in
// a HashMap keyed by extension (or `"<folder>"` for directories).

#[cfg(target_os = "windows")]
fn extract_shell_icon_hicon(path: &Path, is_dir: bool) -> Option<isize> {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGFI_USEFILEATTRIBUTES,
    };
    let wide = ru_wx::platform::to_wide(&path.to_string_lossy());
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let attrs = if is_dir {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        FILE_ATTRIBUTE_NORMAL
    };
    // SAFETY: SHGetFileInfoW writes into `info` (size-checked) and the
    // SHGFI_USEFILEATTRIBUTES flag means it never touches the disk.
    unsafe {
        let r = SHGetFileInfoW(
            wide.as_ptr(),
            attrs,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON | SHGFI_USEFILEATTRIBUTES,
        );
        if r == 0 || info.hIcon.is_null() {
            return None;
        }
        // Caller is responsible for DestroyIcon.
        Some(info.hIcon as isize)
    }
}

#[cfg(not(target_os = "windows"))]
fn extract_shell_icon_hicon(_path: &Path, _is_dir: bool) -> Option<isize> {
    None
}

/// Convert a 32 bpp ARGB HBITMAP (returned by Shell icon extraction
/// after the HICON → HBITMAP round-trip) into RGBA8 PNG bytes that the
/// library's `ImageList::add_image_bytes` can consume.
#[cfg(target_os = "windows")]
fn hicon_to_png_bytes(hicon: isize) -> Option<Vec<u8>> {
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};
    unsafe {
        // 1. Get the icon's intrinsic size via GetIconInfo (gives the
        //    colour bitmap; we ignore the mask).
        use windows_sys::Win32::Graphics::Gdi::BITMAP;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetIconInfo;
        let mut ii = std::mem::zeroed::<windows_sys::Win32::UI::WindowsAndMessaging::ICONINFO>();
        if GetIconInfo(hicon as _, &mut ii) == 0 {
            return None;
        }
        let color_hbmp = ii.hbmColor;
        // Get dimensions from BITMAP
        let mut bmp: BITMAP = std::mem::zeroed();
        let bytes_written = windows_sys::Win32::Graphics::Gdi::GetObjectW(
            color_hbmp as _,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bmp as *mut _ as *mut _,
        );
        if bytes_written == 0 {
            DeleteObject(ii.hbmMask as _);
            DeleteObject(ii.hbmColor as _);
            DestroyIcon(hicon as _);
            return None;
        }
        let w = bmp.bmWidth as u32;
        let h = bmp.bmHeight.unsigned_abs() as u32;
        DeleteObject(ii.hbmMask as _);
        DeleteObject(ii.hbmColor as _);

        // 2. Draw the icon into a 32 bpp ARGB DIB section so we can
        //    read raw pixels.
        let screen = GetDC(std::ptr::null_mut());
        let mem_dc = CreateCompatibleDC(screen);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [std::mem::zeroed()],
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(
            screen,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if dib.is_null() || bits.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen);
            DeleteDC(mem_dc);
            return None;
        }
        let old = SelectObject(mem_dc, dib as _);
        // Paint a transparent background (alpha = 0) so the icon's
        // own pixels take their colours from DrawIconEx.
        let transparent = 0u32;
        let _ = BitBlt(
            mem_dc,
            0,
            0,
            w as i32,
            h as i32,
            std::ptr::null_mut(),
            0,
            0,
            transparent,
        );
        // DrawIconEx draws the colour bitmap AND the mask, so the
        // resulting DIB has the icon correctly alpha-blended.
        let _ = DrawIconEx(
            mem_dc,
            0,
            0,
            hicon as _,
            w as i32,
            h as i32,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        );

        // 3. Read BGRA pixels (Win32 DIBs are bottom-up by default; we
        //    asked for top-down via negative biHeight so they read
        //    naturally). Convert BGRA → RGBA for `image::Rgba`.
        let stride = (w as usize) * 4;
        let total = stride * (h as usize);
        let pixels = std::slice::from_raw_parts(bits as *const u8, total);
        let rgba = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(w, h, |x, y| {
            let i = (y as usize) * stride + (x as usize) * 4;
            Rgba([pixels[i + 2], pixels[i + 1], pixels[i], pixels[i + 3]])
        });

        SelectObject(mem_dc, old);
        DeleteObject(dib as _);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen);
        DestroyIcon(hicon as _);

        Some(encode_png(&rgba))
    }
}

#[cfg(not(target_os = "windows"))]
fn hicon_to_png_bytes(_hicon: isize) -> Option<Vec<u8>> {
    None
}

// ─── Settings dialog (font size, hidden files) ───────────────────────────────
//
// The dialog is modal: the parent frame is disabled until the user clicks
// OK or Cancel. On OK, the chosen settings are applied to the live
// widgets — the font size is propagated via raw `WM_SETFONT` messages,
// because the library's widget traits don't yet expose a `set_font`
// helper that we'd prefer.

fn show_settings_dialog(parent: &Frame, list: &ListCtrl, status: &StatusBar) {
    let initial_size = 9i32;
    let dlg = Dialog::new(parent, "Settings", 360, 220);

    // Title label — register as a child of the dialog so it lays out
    // inside the modal frame.
    let title = StaticText::new(parent, "Application settings");
    title.as_widget_ref().borrow_mut().set_position(20, 15);
    dlg.add_widget(title.as_widget_ref());

    // Font size label + spin + hint
    let lbl_font = StaticText::new(parent, "Font size:");
    lbl_font.as_widget_ref().borrow_mut().set_position(20, 55);
    dlg.add_widget(lbl_font.as_widget_ref());

    let spin = SpinCtrl::new(parent, 6, 24, initial_size);
    spin.as_widget_ref().borrow_mut().set_position(120, 50);
    spin.as_widget_ref().borrow_mut().set_size(90, 24);
    dlg.add_widget(spin.as_widget_ref());

    let lbl_hint = StaticText::new(parent, "(points, 6–24)");
    lbl_hint.as_widget_ref().borrow_mut().set_position(220, 55);
    dlg.add_widget(lbl_hint.as_widget_ref());

    // OK / Cancel via the Dialog's built-in button helper.
    let (id_ok, _ok_ref) = dlg.create_button("OK");
    let (id_cancel, _cancel_ref) = dlg.create_button("Cancel");

    // OK handler — apply the chosen font size to the live widgets,
    // then close the dialog.
    let list_for_ok = list.clone();
    let status_for_ok = status.clone();
    let spin_for_ok = spin.clone();
    let dlg_for_ok = dlg.clone();
    dlg.register_command_handler(
        id_ok,
        Box::new(move || {
            let pt = spin_for_ok.get_value();
            apply_font_size(&list_for_ok, &status_for_ok, pt);
            set_status(&status_for_ok, &format!("Font size set to {pt} pt."));
            dlg_for_ok.end_modal(1);
        }),
    );
    // Cancel just closes with result 0.
    let dlg_for_cancel = dlg.clone();
    dlg.register_command_handler(
        id_cancel,
        Box::new(move || {
            dlg_for_cancel.end_modal(0);
        }),
    );

    let _ = dlg.show_modal();
}

// ─── Drive enumeration + free-space ─────────────────────────────────────────
//
// On Windows we use the standard Shell / kernel32 APIs to list logical
// drives and query their free / total space. Drives are reported with
// a type tag (`Fixed`, `Removable`, `CDROM`, `Network`) so the tree
// can pick an appropriate icon and the status bar can show free / total.

/// Win32 drive types (subset of `winbase.h` DRIVE_* constants).
/// I costanti sono solo valori numerici: restano disponibili su tutte le
/// piattaforme così `populate_tree` può compilare ovunque.
mod drive_type {
    pub const DRIVE_REMOVABLE: u32 = 2;
    pub const DRIVE_FIXED: u32 = 3;
    pub const DRIVE_REMOTE: u32 = 4;
    pub const DRIVE_CDROM: u32 = 5;
    // Only the Windows path maps this one to a label.
    #[cfg(target_os = "windows")]
    pub const DRIVE_RAMDISK: u32 = 6;
}

#[cfg(target_os = "windows")]
fn drive_type_name(t: u32) -> &'static str {
    use drive_type::*;
    match t {
        DRIVE_FIXED => "Fixed",
        DRIVE_REMOVABLE => "Removable",
        DRIVE_CDROM => "CDROM",
        DRIVE_REMOTE => "Network",
        DRIVE_RAMDISK => "RAM Disk",
        _ => "Unknown",
    }
}

/// One logical drive, ready to plug into the tree.
struct DriveInfo {
    /// Pretty label, e.g. `"Local Disk (C:)"` or `"C:"`.
    label: String,
    /// `DRIVE_FIXED`, `DRIVE_REMOVABLE`, etc.
    kind: u32,
    /// Absolute root path, e.g. `C:\`.
    root: PathBuf,
}

#[cfg(target_os = "windows")]
fn enumerate_drives() -> Vec<DriveInfo> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    let bitmask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for i in 0..26u32 {
        if bitmask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let root_str = format!("{}:\\", letter);
        let wide: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();
        let dtype = unsafe { GetDriveTypeW(wide.as_ptr()) };

        // Read volume label so we can show e.g. "OS (C:)" instead
        // of just "C:". Best-effort: empty label is fine.
        let label = read_volume_label(letter).unwrap_or_else(|| format!("{}:", letter));
        let kind_label = drive_type_name(dtype);

        let display = if label == format!("{}:", letter) {
            // No volume label — just the letter + drive type.
            format!("{} ({})", letter, kind_label)
        } else {
            format!("{} ({})", label, kind_label)
        };

        out.push(DriveInfo {
            label: display,
            kind: dtype,
            root: PathBuf::from(root_str),
        });
    }
    out
}

#[cfg(not(target_os = "windows"))]
fn enumerate_drives() -> Vec<DriveInfo> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn read_volume_label(letter: char) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let root = format!("{}:\\", letter);
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut name_buf = [0u16; 261];
    let mut fs_buf = [0u16; 261];
    let mut serial = 0u32;
    let mut max_comp = 0u32;
    let mut flags = 0u32;
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            name_buf.as_mut_ptr(),
            name_buf.len() as u32,
            &mut serial,
            &mut max_comp,
            &mut flags,
            fs_buf.as_mut_ptr(),
            fs_buf.len() as u32,
        )
    };
    if ok == 0 {
        return None;
    }
    let len = name_buf
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(name_buf.len());
    if len == 0 {
        return None;
    }
    String::from_utf16(&name_buf[..len]).ok()
}

/// Free / total bytes for a drive letter. Returns `(free, total)`.
#[cfg(target_os = "windows")]
fn drive_free_space(letter: char) -> Option<(u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let root = format!("{}:\\", letter);
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut free_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_to_caller,
            &mut total,
            &mut total_free,
        )
    };
    if ok == 0 {
        return None;
    }
    Some((free_to_caller, total))
}

#[cfg(not(target_os = "windows"))]
fn drive_free_space(_letter: char) -> Option<(u64, u64)> {
    None
}

/// Format a `(free, total)` byte pair as `"234.5 GB free of 500.0 GB (47%)"`.
fn format_disk_space(free: u64, total: u64) -> String {
    let pct = if total == 0 {
        0
    } else {
        ((free as f64 / total as f64) * 100.0).round() as u32
    };
    format!(
        "{} free of {} ({}%)",
        human_size(free),
        human_size(total),
        pct
    )
}

// ─── Duplicate file/folder with auto-numbering ───────────────────────────────
//
// `find_unique_duplicate_name("foo.txt")` returns "foo - duplicate.txt"
// the first time, "foo - duplicate 2.txt" the second time, etc.
// For folders the `.ext` part is empty so we just append " - duplicate".

fn find_unique_duplicate_name(src: &Path) -> PathBuf {
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = src.extension().and_then(|s| s.to_str());

    let candidate = |suffix: &str| -> PathBuf {
        let name = match ext {
            Some(e) if !e.is_empty() => format!("{stem}{suffix}.{e}"),
            _ => format!("{stem}{suffix}"),
        };
        parent.join(name)
    };

    // First try the bare " - duplicate" form.
    let first = candidate(" - duplicate");
    if !first.exists() {
        return first;
    }
    // Then walk 2, 3, 4, … until we find a free name.
    let mut n = 2u32;
    loop {
        let path = candidate(&format!(" - duplicate {n}"));
        if !path.exists() {
            return path;
        }
        n += 1;
        if n > 10_000 {
            // Pathological case — give up with a timestamp.
            return candidate(&format!(
                " - duplicate {}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ));
        }
    }
}

/// Duplicate every path in `paths` next to its source, returning a list
/// of `(src, dst)` pairs actually created (skips entries whose target
/// already exists, so the user can retry without trashing work).
fn duplicate_paths(paths: &[PathBuf]) -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    for src in paths {
        let dst = find_unique_duplicate_name(src);
        let result = if src.is_dir() {
            copy_dir_recursive(src, &dst)
        } else {
            fs::copy(src, &dst).map(|_| ())
        };
        if result.is_ok() {
            out.push((src.clone(), dst));
        }
    }
    out
}

// ─── Apply font size to the list (and status bar) via raw WM_SETFONT ───────
fn apply_font_size(list: &ListCtrl, status: &StatusBar, point_size: i32) {
    let hfont = create_ui_font_hfont(point_size);
    if hfont == 0 {
        return;
    }
    let list_hwnd = list.as_widget_ref().borrow().native_handle();
    send_set_font(list_hwnd, hfont);
    let status_hwnd = status.as_widget_ref().borrow().native_handle();
    send_set_font(status_hwnd, hfont);
}

/// Build a HFONT for the default UI font at the given point size.
/// The handle is intentionally leaked so that WM_SETFONT receivers
/// never see a freed HFONT (the wrapper Font would `DeleteObject`
/// it on drop, which would invalidate the pointer we just handed to
/// every window).
#[cfg(target_os = "windows")]
fn create_ui_font_hfont(point_size: i32) -> isize {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateFontW, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, FW_DONTCARE, OUT_DEFAULT_PRECIS,
    };
    let face = ru_wx::platform::to_wide("Segoe UI");
    let hfont = unsafe {
        CreateFontW(
            -point_size,                // cHeight (negative → char height)
            0,                          // cWidth
            0,                          // cEscapement
            0,                          // cOrientation
            FW_DONTCARE as i32,         // cWeight
            0u32,                       // bItalic
            0u32,                       // bUnderline
            0u32,                       // bStrikeOut
            DEFAULT_CHARSET as u32,     // iCharSet
            OUT_DEFAULT_PRECIS as u32,  // iOutPrecision
            CLIP_DEFAULT_PRECIS as u32, // iClipPrecision
            5u32,                       // iQuality (CLEARTYPE_QUALITY=5)
            0u32,                       // iPitchAndFamily
            face.as_ptr(),
        )
    };
    hfont as isize
}

#[cfg(not(target_os = "windows"))]
fn create_ui_font_hfont(_point_size: i32) -> isize {
    0
}

#[cfg(target_os = "windows")]
fn send_set_font(hwnd: isize, hfont: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_SETFONT};
    if hwnd == 0 {
        return;
    }
    // SAFETY: WM_SETFONT with a valid HFONT (leaked). lParam = 1
    // means "redraw immediately".
    unsafe {
        SendMessageW(hwnd as _, WM_SETFONT, hfont as _, 1);
    }
}

#[cfg(not(target_os = "windows"))]
fn send_set_font(_hwnd: isize, _hfont: isize) {}
