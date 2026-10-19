//! Event dispatch: reactor callback → ru_wx closure.
//!
//! When reactor fires a `TypedEventHandler`, it calls into our registered
//! closure. This module provides the lookup-and-dispatch logic that
//! bridges from the reactor callback (no payload, just the widget ID)
//! to the user's typed closure (`FnMut(String)`, `FnMut(usize)`, …).

use super::element_bridge::{SharedRegistry, WidgetId};

/// Dispatch a simple click event.
pub(crate) fn dispatch_click(
    registry: &SharedRegistry,
    id: WidgetId,
) -> impl FnMut() + Send + 'static {
    let registry = registry.clone();
    move || {
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.click_handlers.get_mut(&id) {
                handler();
            }
        }
    }
}

/// Dispatch a text-changed event (e.g. `TextBox` → `on_text_changed`).
pub(crate) fn dispatch_text_changed(
    registry: &SharedRegistry,
    id: WidgetId,
) -> impl FnMut(String) + Send + 'static {
    let registry = registry.clone();
    move |text: String| {
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.text_changed_handlers.get_mut(&id) {
                handler(text);
            }
        }
    }
}

/// Dispatch a selection-changed event (e.g. `ComboBox` → `on_selection_changed`).
pub(crate) fn dispatch_selection_changed(
    registry: &SharedRegistry,
    id: WidgetId,
) -> impl FnMut(usize) + Send + 'static {
    let registry = registry.clone();
    move |index: usize| {
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.selection_changed_handlers.get_mut(&id) {
                handler(index);
            }
        }
    }
}

/// Dispatch a toggled event (e.g. `CheckBox` → `on_toggled`).
pub(crate) fn dispatch_toggled(
    registry: &SharedRegistry,
    id: WidgetId,
) -> impl FnMut(bool) + Send + 'static {
    let registry = registry.clone();
    move |checked: bool| {
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.toggled_handlers.get_mut(&id) {
                handler(checked);
            }
        }
    }
}

/// Dispatch a window-close event.
pub(crate) fn dispatch_close(
    registry: &SharedRegistry,
    id: WidgetId,
) -> impl FnMut() + Send + 'static {
    let registry = registry.clone();
    move || {
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.close_handlers.get_mut(&id) {
                handler();
            }
        }
    }
}

/// Dispatch a window-resize event.
pub(crate) fn dispatch_resize(
    registry: &SharedRegistry,
    id: WidgetId,
) -> impl FnMut(i32, i32) + Send + 'static {
    let registry = registry.clone();
    move |width: i32, height: i32| {
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.resize_handlers.get_mut(&id) {
                handler(width, height);
            }
        }
    }
}
