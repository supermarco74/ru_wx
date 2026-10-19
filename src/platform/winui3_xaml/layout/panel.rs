//! XamlPanel — simple container panel.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Simple container panel. Children are stacked vertically.
pub struct XamlPanel {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlPanel {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::Panel {
            id,
            children: Vec::new(),
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a child widget descriptor to this panel.
    pub fn add_child(&self, child: WidgetDesc) {
        let mut desc = self.frame.desc.borrow_mut();
        for c in &mut desc.children {
            if let WidgetDesc::Panel { id, children } = c {
                if *id == self.id {
                    children.push(child);
                    break;
                }
            }
        }
    }

    /// Rimuove il descrittore di questo panel dal frame e lo restituisce,
    /// così può essere aggiunto come figlio di un altro container
    /// (es. `outer.add_child(inner.take_desc())`).
    pub fn take_desc(&self) -> Option<WidgetDesc> {
        let mut desc = self.frame.desc.borrow_mut();
        let idx = desc
            .children
            .iter()
            .position(|c| matches!(c, WidgetDesc::Panel { id, .. } if *id == self.id))?;
        Some(desc.children.remove(idx))
    }
}
