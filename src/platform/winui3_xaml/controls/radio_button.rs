//! XamlRadioButton — radio button widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Radio button with label and group name.
pub struct XamlRadioButton {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlRadioButton {
    pub fn new(frame: &XamlFrame, label: &str, group: &str, checked: bool) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::RadioButton {
                id,
                label: label.to_string(),
                group: group.to_string(),
                checked,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn set_checked(&self, checked: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::RadioButton { id, checked: c, .. } = child {
                if *id == self.id {
                    *c = checked;
                    break;
                }
            }
        }
    }
}
