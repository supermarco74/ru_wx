//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Control value-change events (`wxSpinEvent`, …).

/// Spin control delta (`wxSpinEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpinEvent {
    pub value: i32,
    pub delta: i32,
}

impl SpinEvent {
    pub const fn new(value: i32, delta: i32) -> Self {
        Self { value, delta }
    }
}

/// Slider thumb moved (`wxScrollEvent` / slider notification).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderEvent {
    pub value: i32,
    pub dragging: bool,
}

impl SliderEvent {
    pub const fn new(value: i32, dragging: bool) -> Self {
        Self { value, dragging }
    }
}

/// Gauge range hit (`wxGaugeEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GaugeEvent {
    pub value: i32,
}

impl GaugeEvent {
    pub const fn new(value: i32) -> Self {
        Self { value }
    }
}

/// Collapsible pane toggled (`wxCollapsiblePaneEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapsiblePaneEvent {
    pub expanded: bool,
}

impl CollapsiblePaneEvent {
    pub const fn new(expanded: bool) -> Self {
        Self { expanded }
    }
}
