//! XamlBoxSizer — BoxSizer layout (vertical or horizontal stack).

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Orientation for the sizer.
pub use super::super::element_bridge::XamlOrientation;

/// BoxSizer layout container.
///
/// Maps to reactor's `vstack` (vertical) or `hstack` (horizontal).
pub struct XamlBoxSizer {
    id: WidgetId,
    frame: XamlFrame,
    orientation: XamlOrientation,
    spacing: f64,
}

impl XamlBoxSizer {
    /// Create a vertical BoxSizer.
    pub fn vertical(frame: &XamlFrame) -> Self {
        Self::new(frame, XamlOrientation::Vertical)
    }

    /// Create a horizontal BoxSizer.
    pub fn horizontal(frame: &XamlFrame) -> Self {
        Self::new(frame, XamlOrientation::Horizontal)
    }

    fn new(frame: &XamlFrame, orientation: XamlOrientation) -> Self {
        let id = next_id();
        let desc = match orientation {
            XamlOrientation::Vertical => WidgetDesc::VStack {
                id,
                children: Vec::new(),
                spacing: 0.0,
                group: true,
            },
            XamlOrientation::Horizontal => WidgetDesc::HStack {
                id,
                children: Vec::new(),
                spacing: 0.0,
                group: true,
            },
        };
        frame.desc.borrow_mut().children.push(desc);
        Self {
            id,
            frame: frame.clone(),
            orientation,
            spacing: 0.0,
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Set spacing between children.
    pub fn set_spacing(&mut self, spacing: f64) {
        self.spacing = spacing;
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            match child {
                WidgetDesc::VStack { id, spacing: s, .. } if *id == self.id => {
                    *s = spacing;
                    break;
                }
                WidgetDesc::HStack { id, spacing: s, .. } if *id == self.id => {
                    *s = spacing;
                    break;
                }
                _ => {}
            }
        }
    }

    /// Add a child widget descriptor.
    pub fn add(&self, child: WidgetDesc) {
        let mut desc = self.frame.desc.borrow_mut();
        for c in &mut desc.children {
            match c {
                WidgetDesc::VStack { id, children, .. } if *id == self.id => {
                    children.push(child);
                    break;
                }
                WidgetDesc::HStack { id, children, .. } if *id == self.id => {
                    children.push(child);
                    break;
                }
                _ => {}
            }
        }
    }

    /// Rimuove il descrittore di questo sizer dal frame e lo restituisce,
    /// così può essere aggiunto come figlio di un altro sizer
    /// (es. `main.add(footer.take_desc())`).
    pub fn take_desc(&self) -> Option<WidgetDesc> {
        let mut desc = self.frame.desc.borrow_mut();
        let idx = desc.children.iter().position(|c| match c {
            WidgetDesc::VStack { id, .. } | WidgetDesc::HStack { id, .. } => *id == self.id,
            _ => false,
        })?;
        Some(desc.children.remove(idx))
    }
}
