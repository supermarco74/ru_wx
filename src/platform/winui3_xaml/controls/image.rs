//! XamlImage — image widget.

use super::super::element_bridge::{next_id, ImageStretch, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Image widget (displays an image from URI).
pub struct XamlImage {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlImage {
    /// Create an image from a file path or URI.
    pub fn new(frame: &XamlFrame, uri: &str) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::Image {
            id,
            uri: uri.to_string(),
            stretch: ImageStretch::Uniform,
            width: None,
            height: None,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Set the image URI.
    pub fn set_uri(&self, uri: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Image { id, uri: u, .. } = child {
                if *id == self.id {
                    *u = uri.to_string();
                    break;
                }
            }
        }
    }

    /// Set the stretch mode.
    pub fn set_stretch(&self, stretch: ImageStretch) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Image { id, stretch: s, .. } = child {
                if *id == self.id {
                    *s = stretch;
                    break;
                }
            }
        }
    }

    /// Set an explicit display size (device-independent pixels).
    ///
    /// Senza una size l'`Image` con `Stretch::Uniform` dentro uno `StackPanel`
    /// desidera tutta la larghezza disponibile e schiaccia i widget
    /// successivi fuori dalla finestra.
    pub fn set_size(&self, width: f64, height: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Image {
                id,
                width: w,
                height: h,
                ..
            } = child
            {
                if *id == self.id {
                    *w = Some(width);
                    *h = Some(height);
                    break;
                }
            }
        }
    }
}
