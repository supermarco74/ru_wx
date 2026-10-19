//! XamlSlider — slider widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Slider (trackbar) widget.
pub struct XamlSlider {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlSlider {
    pub fn new(frame: &XamlFrame, min: f64, max: f64, value: f64) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::Slider {
            id,
            min,
            max,
            value,
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
            if let WidgetDesc::Slider { id, value: v, .. } = child {
                if *id == self.id {
                    *v = value;
                    break;
                }
            }
        }
    }
}
