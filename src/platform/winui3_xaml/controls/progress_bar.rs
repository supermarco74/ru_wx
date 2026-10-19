//! XamlProgressBar — progress bar widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Progress bar widget.
pub struct XamlProgressBar {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlProgressBar {
    pub fn new(frame: &XamlFrame, max: f64) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::ProgressBar {
                id,
                max,
                value: 0.0,
                indeterminate: false,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn indeterminate(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::ProgressBar {
                id,
                max: 100.0,
                value: 0.0,
                indeterminate: true,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn set_value(&self, value: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ProgressBar { id, value: v, .. } = child {
                if *id == self.id {
                    *v = value;
                    break;
                }
            }
        }
    }
}
