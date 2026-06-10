//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! IP address entry (`wxIPAddressCtrl`).

use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};

/// IPv4 address field (`wxIPAddressCtrl`).
#[derive(Clone)]
pub struct IPAddressCtrl {
    text: TextCtrl,
}

impl IPAddressCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        let text = TextCtrl::new(parent, "0.0.0.0");
        Self { text }
    }

    pub fn set_address(&self, address: &str) {
        self.text.set_value(address);
    }

    pub fn address(&self) -> String {
        self.text.get_value()
    }

    pub fn is_valid_ipv4(&self) -> bool {
        let address = self.address();
        let parts: Vec<_> = address.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u8>().is_ok())
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.text.as_widget_ref()
    }
}
