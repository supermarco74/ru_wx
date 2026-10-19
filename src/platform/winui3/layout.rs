//! WinUI3 Layout System
//!
//! This module provides a BoxSizer layout system similar to wxBoxSizer.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Direction for BoxSizer layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Orientation::Horizontal)
    }
}

/// Flags for sizer item behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizerFlag {
    Expand,
    Shaped,
    FixedMinSize,
    AlignCenter,
    AlignLeft,
    AlignRight,
    AlignTop,
    AlignBottom,
}

impl SizerFlag {
    pub fn to_bits(&self) -> u32 {
        match self {
            SizerFlag::Expand => 0x0001,
            SizerFlag::Shaped => 0x0002,
            SizerFlag::FixedMinSize => 0x0004,
            SizerFlag::AlignCenter => 0x0008,
            SizerFlag::AlignLeft => 0x0010,
            SizerFlag::AlignRight => 0x0020,
            SizerFlag::AlignTop => 0x0040,
            SizerFlag::AlignBottom => 0x0080,
        }
    }
}

/// Spacing between sizer items
pub const SIZER_DEFAULT_SPACING: i32 = 5;
pub const SIZER_DEFAULT_BORDER: i32 = 5;

/// BoxSizer - arranges controls in a horizontal or vertical line
pub struct BoxSizer {
    orientation: Orientation,
    items: Vec<SizerItem>,
    spacing: i32,
}

struct SizerItem {
    hwnd: HWND,
    proportion: i32,
    flags: u32,
    min_width: i32,
    min_height: i32,
}

impl BoxSizer {
    /// Create a new BoxSizer with the given orientation
    pub fn new(orientation: Orientation) -> Self {
        Self {
            orientation,
            items: Vec::new(),
            spacing: SIZER_DEFAULT_SPACING,
        }
    }

    /// Create a new horizontal BoxSizer
    pub fn horizontal() -> Self {
        Self::new(Orientation::Horizontal)
    }

    /// Create a new vertical BoxSizer
    pub fn vertical() -> Self {
        Self::new(Orientation::Vertical)
    }

    /// Add a window to the sizer with default settings
    pub fn add(&mut self, hwnd: HWND) {
        self.add_with_proportion(hwnd, 0);
    }

    /// Add a window with a proportion value
    pub fn add_with_proportion(&mut self, hwnd: HWND, proportion: i32) {
        self.add_with_flags_and_proportion(hwnd, proportion, 0);
    }

    /// Add a window with flags and proportion
    pub fn add_with_flags_and_proportion(&mut self, hwnd: HWND, proportion: i32, flags: u32) {
        self.add_sized_with_flags_and_proportion(hwnd, 0, 0, proportion, flags);
    }

    /// Add a window with specific size, flags, and proportion
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
    pub fn add_sized_with_flags_and_proportion(
        &mut self,
        hwnd: HWND,
        width: i32,
        height: i32,
        proportion: i32,
        flags: u32,
    ) {
        // Get current window size if not specified
        let (min_width, min_height) = if width == 0 || height == 0 {
            unsafe {
                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
                let w = if width == 0 {
                    rect.right - rect.left
                } else {
                    width
                };
                let h = if height == 0 {
                    rect.bottom - rect.top
                } else {
                    height
                };
                (w, h)
            }
        } else {
            (width, height)
        };

        self.items.push(SizerItem {
            hwnd,
            proportion,
            flags,
            min_width,
            min_height,
        });
    }

    /// Add spacing to the sizer
    pub fn add_spacer(&mut self, size: i32) {
        // Spacer is represented by a null HWND item
        self.items.push(SizerItem {
            hwnd: std::ptr::null_mut(),
            proportion: 0,
            flags: 0,
            min_width: if self.orientation.is_horizontal() {
                size
            } else {
                0
            },
            min_height: if self.orientation.is_horizontal() {
                0
            } else {
                size
            },
        });
    }

    /// Add a stretch spacer
    pub fn add_stretch_spacer(&mut self, proportion: i32) {
        self.items.push(SizerItem {
            hwnd: std::ptr::null_mut(),
            proportion,
            flags: SizerFlag::Expand.to_bits(),
            min_width: 0,
            min_height: 0,
        });
    }

    /// Set spacing between items
    pub fn set_spacing(&mut self, spacing: i32) {
        self.spacing = spacing;
    }

    /// Get the number of items in the sizer
    pub fn get_item_count(&self) -> usize {
        self.items.len()
    }

    /// Clear all items from the sizer
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Layout the sizer and position all windows
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `parent_hwnd` must be live
    pub fn layout(&self, parent_hwnd: HWND) {
        unsafe {
            let mut parent_rect: RECT = std::mem::zeroed();
            GetClientRect(parent_hwnd, &mut parent_rect);

            self.layout_into(parent_hwnd, parent_rect);
        }
    }

    /// Layout the sizer into a specific sub-rectangle of `parent_hwnd`'s
    /// client area. Useful when one wants to position multiple
    /// sizers manually or carve the client area into regions.
    pub fn layout_into(&self, _parent_hwnd: HWND, area: RECT) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            BeginDeferWindowPos, DeferWindowPos, EndDeferWindowPos,
        };
        unsafe {
            let area_x = area.left;
            let area_y = area.top;
            let total_width = area.right - area.left;
            let total_height = area.bottom - area.top;

            if self.items.is_empty() {
                return;
            }

            // Count visible items (non-null hwnd) for BeginDeferWindowPos.
            let n_visible = self.items.iter().filter(|it| !it.hwnd.is_null()).count();
            if n_visible == 0 {
                return;
            }

            // Begin batch window positioning — eliminates flickering
            // caused by individual SetWindowPos calls during resize.
            let mut hdwp = BeginDeferWindowPos(n_visible as i32);
            if hdwp.is_null() {
                // Fallback: Win32 returned NULL, degrade gracefully.
                self.layout_into_fallback(area);
                return;
            }

            let total_proportion: i32 = self.items.iter().map(|item| item.proportion).sum();

            let mut fixed_width = 0;
            let mut fixed_height = 0;

            for item in &self.items {
                if item.hwnd.is_null() || item.proportion == 0 {
                    if self.orientation.is_horizontal() {
                        fixed_width += item.min_width;
                    } else {
                        fixed_height += item.min_height;
                    }
                }
            }

            let spacing_count = if self.items.len() > 1 {
                (self.items.len() - 1) as i32
            } else {
                0
            };
            let total_spacing = spacing_count * self.spacing;

            let available = if self.orientation.is_horizontal() {
                total_width - fixed_width - total_spacing
            } else {
                total_height - fixed_height - total_spacing
            };

            let mut x = 0;
            let mut y = 0;

            for (i, item) in self.items.iter().enumerate() {
                if item.hwnd.is_null() {
                    if self.orientation.is_horizontal() {
                        x += item.min_width
                            + if i < self.items.len() - 1 {
                                self.spacing
                            } else {
                                0
                            };
                    } else {
                        y += item.min_height
                            + if i < self.items.len() - 1 {
                                self.spacing
                            } else {
                                0
                            };
                    }
                    continue;
                }

                let (item_width, item_height) = if item.proportion == 0 || total_proportion <= 0 {
                    (item.min_width, item.min_height)
                } else {
                    let ratio = item.proportion as f32 / total_proportion as f32;
                    let size = (available as f32 * ratio).round() as i32;
                    if self.orientation.is_horizontal() {
                        (size.max(item.min_width), item.min_height)
                    } else {
                        (item.min_width, size.max(item.min_height))
                    }
                };

                let flags = item.flags;
                let mut pos_x = x;
                let mut pos_y = y;

                if !self.orientation.is_horizontal()
                    && (flags & SizerFlag::AlignLeft.to_bits()) != 0
                {
                    pos_x = 0;
                }
                if !self.orientation.is_horizontal()
                    && (flags & SizerFlag::AlignRight.to_bits()) != 0
                {
                    pos_x = total_width - item_width;
                }
                if self.orientation.is_horizontal() && (flags & SizerFlag::AlignTop.to_bits()) != 0
                {
                    pos_y = 0;
                }
                if self.orientation.is_horizontal()
                    && (flags & SizerFlag::AlignBottom.to_bits()) != 0
                {
                    pos_y = total_height - item_height;
                }

                hdwp = DeferWindowPos(
                    hdwp,
                    item.hwnd,
                    std::ptr::null_mut(),
                    area_x + pos_x,
                    area_y + pos_y,
                    item_width,
                    item_height,
                    SWP_NOZORDER,
                );

                if self.orientation.is_horizontal() {
                    x += item_width
                        + if i < self.items.len() - 1 {
                            self.spacing
                        } else {
                            0
                        };
                } else {
                    y += item_height
                        + if i < self.items.len() - 1 {
                            self.spacing
                        } else {
                            0
                        };
                }
            }

            // Commit all deferred positions in a single, atomic window-manager pass.
            let _ = EndDeferWindowPos(hdwp);
        }
    }

    /// Fallback path when `BeginDeferWindowPos` returns NULL.
    /// Identical to the original per-item `SetWindowPos` loop.
    fn layout_into_fallback(&self, area: RECT) {
        // Minimal fallback — uses the old per-item path.
        let area_x = area.left;
        let area_y = area.top;
        let total_width = area.right - area.left;
        let total_height = area.bottom - area.top;
        if self.items.is_empty() {
            return;
        }
        let total_proportion: i32 = self.items.iter().map(|item| item.proportion).sum();
        let mut fixed_width = 0;
        let mut fixed_height = 0;
        for item in &self.items {
            if item.hwnd.is_null() || item.proportion == 0 {
                if self.orientation.is_horizontal() {
                    fixed_width += item.min_width;
                } else {
                    fixed_height += item.min_height;
                }
            }
        }
        let spacing_count = if self.items.len() > 1 {
            (self.items.len() - 1) as i32
        } else {
            0
        };
        let total_spacing = spacing_count * self.spacing;
        let available = if self.orientation.is_horizontal() {
            total_width - fixed_width - total_spacing
        } else {
            total_height - fixed_height - total_spacing
        };
        let mut x = 0;
        let mut y = 0;
        for (i, item) in self.items.iter().enumerate() {
            if item.hwnd.is_null() {
                if self.orientation.is_horizontal() {
                    x += item.min_width
                        + if i < self.items.len() - 1 {
                            self.spacing
                        } else {
                            0
                        };
                } else {
                    y += item.min_height
                        + if i < self.items.len() - 1 {
                            self.spacing
                        } else {
                            0
                        };
                }
                continue;
            }
            let (item_width, item_height) = if item.proportion == 0 || total_proportion <= 0 {
                (item.min_width, item.min_height)
            } else {
                let ratio = item.proportion as f32 / total_proportion as f32;
                let size = (available as f32 * ratio).round() as i32;
                if self.orientation.is_horizontal() {
                    (size.max(item.min_width), item.min_height)
                } else {
                    (item.min_width, size.max(item.min_height))
                }
            };
            unsafe {
                SetWindowPos(
                    item.hwnd,
                    std::ptr::null_mut(),
                    area_x + x,
                    area_y + y,
                    item_width,
                    item_height,
                    SWP_NOZORDER,
                );
            }
            if self.orientation.is_horizontal() {
                x += item_width
                    + if i < self.items.len() - 1 {
                        self.spacing
                    } else {
                        0
                    };
            } else {
                y += item_height
                    + if i < self.items.len() - 1 {
                        self.spacing
                    } else {
                        0
                    };
            }
        }
    }

    /// Fit the sizer within the given window size
    pub fn fit(&self, parent_hwnd: HWND) {
        self.layout(parent_hwnd);
    }

    /// Calculate the minimum size needed by the sizer
    pub fn calc_min(&self) -> (i32, i32) {
        let mut min_width = 0;
        let mut min_height = 0;

        for (i, item) in self.items.iter().enumerate() {
            if item.hwnd.is_null() {
                if self.orientation.is_horizontal() {
                    min_width += item.min_width;
                } else {
                    min_height += item.min_height;
                }
            } else {
                if self.orientation.is_horizontal() {
                    min_width += item.min_width;
                    min_height = min_height.max(item.min_height);
                } else {
                    min_height += item.min_height;
                    min_width = min_width.max(item.min_width);
                }
            }

            // Add spacing
            if i < self.items.len() - 1 {
                if self.orientation.is_horizontal() {
                    min_width += self.spacing;
                } else {
                    min_height += self.spacing;
                }
            }
        }

        (min_width, min_height)
    }

    /// Set the size of the sizer to fill the parent
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `parent_hwnd` must be live
    pub fn set_size(&self, parent_hwnd: HWND, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                parent_hwnd,
                std::ptr::null_mut(),
                0,
                0,
                width,
                height,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
        self.layout(parent_hwnd);
    }
}

impl Default for BoxSizer {
    fn default() -> Self {
        Self::horizontal()
    }
}
