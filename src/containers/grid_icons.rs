//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Named icon sets for [`crate::Grid`] — Lucide / Bootstrap Icons SVGs
//! and raster images (PNG / JPEG) in a single [`ImageList`].
//!
//! Icons are looked up by name so value providers and menus can write
//! `icons.cell("cart", "Espresso")` instead of hard-coded indices.

use std::collections::HashMap;

use crate::containers::grid::Cell;
use crate::dc::image_list::ImageList;

/// Bootstrap Icons (MIT) — https://icons.getbootstrap.com/
macro_rules! grid_icon_bytes {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../assets/icons/",
            $name,
            ".svg"
        ))
    };
}

/// Lucide Icons (ISC) — https://lucide.dev/
macro_rules! lucide_icon_bytes {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../assets/icons/lucide/",
            $name,
            ".svg"
        ))
    };
}

const BOOTSTRAP_MODERN: &[(&str, &[u8])] = &[
    ("star", grid_icon_bytes!("star")),
    ("info", grid_icon_bytes!("info")),
    ("file-new", grid_icon_bytes!("file-new")),
    ("folder", grid_icon_bytes!("folder-open")),
    ("archived", grid_icon_bytes!("exit")),
    ("cart", grid_icon_bytes!("cart-fill")),
    ("tag", grid_icon_bytes!("tag-fill")),
    ("box", grid_icon_bytes!("box-seam-fill")),
    ("cloud", grid_icon_bytes!("cloud-fill")),
    ("lightning", grid_icon_bytes!("lightning-charge-fill")),
    ("featured", grid_icon_bytes!("bookmark-star-fill")),
    ("verified", grid_icon_bytes!("patch-check-fill")),
    ("sold-out", grid_icon_bytes!("x-circle-fill")),
    ("favorite", grid_icon_bytes!("heart-fill")),
    ("trophy", grid_icon_bytes!("trophy-fill")),
    ("premium", grid_icon_bytes!("gem")),
    ("shop", grid_icon_bytes!("shop")),
    ("software", grid_icon_bytes!("cpu-fill")),
    ("media", grid_icon_bytes!("music-note-beamed")),
    ("book", grid_icon_bytes!("book-fill")),
];

/// Lucide stroke icons (24 glyphs, ISC licence) — same logical names
/// as [`BOOTSTRAP_MODERN`] so [`Self::cell_for_product`] works unchanged.
const LUCIDE_MODERN: &[(&str, &[u8])] = &[
    ("star", lucide_icon_bytes!("star")),
    ("info", lucide_icon_bytes!("info")),
    ("file-new", lucide_icon_bytes!("file-plus")),
    ("folder", lucide_icon_bytes!("folder-open")),
    ("archived", lucide_icon_bytes!("archive")),
    ("cart", lucide_icon_bytes!("shopping-cart")),
    ("tag", lucide_icon_bytes!("tag")),
    ("box", lucide_icon_bytes!("package")),
    ("cloud", lucide_icon_bytes!("cloud")),
    ("lightning", lucide_icon_bytes!("zap")),
    ("featured", lucide_icon_bytes!("bookmark")),
    ("verified", lucide_icon_bytes!("circle-check-big")),
    ("sold-out", lucide_icon_bytes!("circle-x")),
    ("favorite", lucide_icon_bytes!("heart")),
    ("trophy", lucide_icon_bytes!("trophy")),
    ("premium", lucide_icon_bytes!("gem")),
    ("shop", lucide_icon_bytes!("store")),
    ("software", lucide_icon_bytes!("cpu")),
    ("media", lucide_icon_bytes!("music")),
    ("book", lucide_icon_bytes!("book-open")),
    ("sparkles", lucide_icon_bytes!("sparkles")),
    ("trending", lucide_icon_bytes!("trending-up")),
    ("grid", lucide_icon_bytes!("layout-grid")),
    ("percent", lucide_icon_bytes!("badge-percent")),
];

/// A named set of cell icons backed by one shared [`ImageList`].
pub struct GridIcons {
    list: ImageList,
    names: HashMap<String, i32>,
    assets: Vec<(String, Vec<u8>)>,
    size: i32,
}

impl GridIcons {
    /// Empty icon set; add images with [`Self::add_svg`] / [`Self::add_image`].
    pub fn new(size: i32) -> Self {
        Self {
            list: ImageList::new(size, size),
            names: HashMap::new(),
            assets: Vec::new(),
            size,
        }
    }

    /// Bootstrap Icons catalogue (20 glyphs, MIT licence).
    pub fn bootstrap_modern(size: i32) -> Self {
        let mut icons = Self::new(size);
        for (name, bytes) in BOOTSTRAP_MODERN {
            icons.add_image(name, bytes);
        }
        icons
    }

    /// Lucide stroke icons (24 glyphs, ISC licence) — default for Win11 grids.
    pub fn lucide_modern(size: i32) -> Self {
        let mut icons = Self::new(size);
        for (name, bytes) in LUCIDE_MODERN {
            icons.add_image(name, bytes);
        }
        icons
    }

    /// Pixel size of each icon in the list.
    pub fn size(&self) -> i32 {
        self.size
    }

    /// Underlying Win32 image list (pass to [`crate::Grid::attach_icons`]).
    pub fn image_list(&self) -> &ImageList {
        &self.list
    }

    /// Number of registered icons.
    pub fn count(&self) -> usize {
        self.names.len()
    }

    /// Look up the image-list index for a registered name.
    pub fn index(&self, name: &str) -> Option<i32> {
        self.names.get(name).copied()
    }

    /// Register an SVG (or raster) asset under `name`.
    pub fn add_image(&mut self, name: &str, bytes: &[u8]) -> Option<i32> {
        let idx = self.list.add_image_bytes(bytes)?;
        self.names.insert(name.to_string(), idx);
        self.assets.push((name.to_string(), bytes.to_vec()));
        Some(idx)
    }

    /// Register embedded SVG bytes under `name`.
    pub fn add_svg(&mut self, name: &str, svg: &[u8]) -> Option<i32> {
        self.add_image(name, svg)
    }

    /// Rebuild the image list at a new pixel size (keeps names → indices).
    pub fn resize(&mut self, size: i32) {
        if size == self.size {
            return;
        }
        self.size = size;
        self.list = ImageList::new(size, size);
        self.names.clear();
        let assets = std::mem::take(&mut self.assets);
        for (name, bytes) in assets {
            self.add_image(&name, &bytes);
        }
    }

    /// Suggest an icon name for a product category string.
    pub fn icon_name_for_category(category: &str) -> &'static str {
        match category {
            "Kitchen" => "cart",
            "Stationery" => "book",
            "Software" => "software",
            "Digital" => "info",
            "Peripherals" => "box",
            "Archive" => "archived",
            "Service" => "cloud",
            "Events" => "trophy",
            _ => "tag",
        }
    }

    /// Suggest an icon for stock / popularity state.
    pub fn icon_name_for_stock(stock: u32, max: u32, popular: bool) -> &'static str {
        if max == 0 || stock == 0 {
            "sold-out"
        } else if popular {
            "featured"
        } else if stock < max / 4 {
            "lightning"
        } else {
            "verified"
        }
    }

    /// Build a [`Cell::Image`] or [`Cell::ImageOnly`] (falls back to text).
    pub fn cell(&self, name: &str, text: impl Into<String>) -> Cell {
        match self.index(name) {
            Some(idx) => Cell::Image {
                idx,
                text: text.into(),
            },
            None => Cell::Text(text.into()),
        }
    }

    /// Icon without label; empty cell if the name is unknown.
    pub fn cell_only(&self, name: &str) -> Cell {
        match self.index(name) {
            Some(idx) => Cell::ImageOnly(idx),
            None => Cell::Empty,
        }
    }

    /// Image + text for the type column using category heuristics.
    pub fn cell_for_product(
        &self,
        category: &str,
        stock: u32,
        max: u32,
        popular: bool,
        label: &str,
    ) -> Cell {
        if popular {
            return self.cell("featured", label);
        }
        if max == 0 || stock == 0 {
            return self.cell("sold-out", label);
        }
        let state = Self::icon_name_for_stock(stock, max, popular);
        if self.index(state).is_some() {
            return self.cell(state, label);
        }
        self.cell(Self::icon_name_for_category(category), label)
    }
}

impl Clone for GridIcons {
    fn clone(&self) -> Self {
        let mut copy = Self::new(self.size);
        for (name, bytes) in &self.assets {
            copy.add_image(name, bytes);
        }
        copy
    }
}
