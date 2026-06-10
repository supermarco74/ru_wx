//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Directory tree (`wxGenericDirCtrl`).

use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// Tree of filesystem directories (`wxGenericDirCtrl`).
#[derive(Clone)]
pub struct GenericDirCtrl {
    tree: TreeCtrl,
    root: TreeItem,
}

impl GenericDirCtrl {
    pub fn new(frame: &Frame) -> Self {
        let tree = TreeCtrl::new(frame);
        let root = tree.add_root("Computer");
        Self { tree, root }
    }

    pub fn add_directory(&self, parent: TreeItem, path: &str) -> TreeItem {
        self.tree.append_item(parent, path)
    }

    pub fn root(&self) -> TreeItem {
        self.root
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.tree.as_widget_ref()
    }
}
