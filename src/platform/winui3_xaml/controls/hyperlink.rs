//! XamlHyperlink — testo cliccabile con URL.
//!
//! Omologa XAML dell'`HyperlinkCtrl` di AppKit: il mapping in `app.rs`
//! la rende come `HyperlinkButton` con `NavigateUri`.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Clickable hyperlink (label + URL).
pub struct XamlHyperlink {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlHyperlink {
    pub fn new(frame: &XamlFrame, label: &str, url: &str) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::Hyperlink {
                id,
                label: label.to_string(),
                url: url.to_string(),
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Update label and URL (come `hyperlink_update` di AppKit).
    pub fn update(&self, label: &str, url: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Hyperlink {
                id,
                label: l,
                url: u,
            } = child
            {
                if *id == self.id {
                    *l = label.to_string();
                    *u = url.to_string();
                    break;
                }
            }
        }
    }
}
