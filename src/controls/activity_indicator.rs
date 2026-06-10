//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Indeterminate spinner (`wxActivityIndicator`).

use crate::controls::gauge::{Gauge, GaugeStyle};
use crate::core::widget::{WidgetRef, Window};

#[derive(Clone)]
pub struct ActivityIndicator {
    gauge: Gauge,
}

impl ActivityIndicator {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            gauge: Gauge::new_with_style(parent, 100, GaugeStyle::Horizontal, true),
        }
    }

    pub fn start(&self) {
        self.gauge.pulse();
    }

    pub fn stop(&self) {
        self.gauge.stop_pulse();
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.gauge.as_widget_ref()
    }
}
