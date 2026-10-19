//! XamlTreeCtrl — hierarchical tree view.

use super::super::element_bridge::{next_id, TreeNodeDesc, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Tree control (hierarchical list).
pub struct XamlTreeCtrl {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlTreeCtrl {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::TreeView {
            id,
            nodes: Vec::new(),
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a root node.
    pub fn add_root(&self, text: &str) -> TreeNodeBuilder {
        TreeNodeBuilder {
            desc: TreeNodeDesc {
                text: text.to_string(),
                expanded: false,
                children: Vec::new(),
            },
        }
    }

    /// Add a built node tree to the control.
    pub fn add_node(&self, node: TreeNodeBuilder) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TreeView { id, nodes } = child {
                if *id == self.id {
                    nodes.push(node.desc);
                    break;
                }
            }
        }
    }
}

/// Builder for tree nodes.
pub struct TreeNodeBuilder {
    desc: TreeNodeDesc,
}

impl TreeNodeBuilder {
    /// Mark this node as expanded by default.
    pub fn expanded(mut self) -> Self {
        self.desc.expanded = true;
        self
    }

    /// Add a child node.
    pub fn child(mut self, text: &str) -> Self {
        self.desc.children.push(TreeNodeDesc {
            text: text.to_string(),
            expanded: false,
            children: Vec::new(),
        });
        self
    }

    /// Add a child node with its own children.
    pub fn child_node(mut self, node: TreeNodeBuilder) -> Self {
        self.desc.children.push(node.desc);
        self
    }
}
