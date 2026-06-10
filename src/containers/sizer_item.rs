//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Sizer child descriptor (`wxSizerItem`).

use crate::core::widget::WidgetRef;

/// One entry inside a sizer (`wxSizerItem`).
#[derive(Clone)]
pub enum SizerItem {
    Widget {
        widget: WidgetRef,
        proportion: u32,
    },
    Stretch {
        proportion: u32,
    },
    FixedSpace {
        size: i32,
    },
    /// A nested [`crate::BoxSizer`] laid out inside the slot the
    /// parent assigns to it (`wxSizer::Add(sizer, …)`).
    Nested {
        sizer: Box<crate::containers::sizer::BoxSizer>,
        proportion: u32,
    },
}

impl std::fmt::Debug for SizerItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Widget { proportion, .. } => f.debug_struct("Widget").field("proportion", proportion).finish(),
            Self::Stretch { proportion } => f.debug_struct("Stretch").field("proportion", proportion).finish(),
            Self::FixedSpace { size } => f.debug_struct("FixedSpace").field("size", size).finish(),
            Self::Nested { proportion, .. } => f.debug_struct("Nested").field("proportion", proportion).finish(),
        }
    }
}

impl SizerItem {
    pub fn widget(widget: WidgetRef, proportion: u32) -> Self {
        Self::Widget { widget, proportion }
    }

    pub fn stretch(proportion: u32) -> Self {
        Self::Stretch { proportion }
    }

    pub fn fixed_space(size: i32) -> Self {
        Self::FixedSpace { size: size.max(0) }
    }

    pub fn proportion(&self) -> u32 {
        match self {
            Self::Widget { proportion, .. }
            | Self::Stretch { proportion }
            | Self::Nested { proportion, .. } => *proportion,
            Self::FixedSpace { .. } => 0,
        }
    }
}
