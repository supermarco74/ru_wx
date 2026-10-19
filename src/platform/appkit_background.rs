//! Panel / frame background fills via [`CALayer`] and [`NSVisualEffectView`].

use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use objc2_foundation::MainThreadMarker;

use super::{ns_color_from_rgba, Colour, NativeHandle, Rgba, CONTAINERS, FRAME_CONTENT};

thread_local! {
    /// Frosted-glass backdrops for translucent colours (`alpha` < 255).
    static EFFECT_BACKDROPS: RefCell<HashMap<NativeHandle, Retained<NSVisualEffectView>>> =
        RefCell::new(HashMap::new());
}

fn remove_effect_backdrop(handle: NativeHandle) {
    EFFECT_BACKDROPS.with(|map| {
        if let Some(effect) = map.borrow_mut().remove(&handle) {
            effect.removeFromSuperview();
        }
    });
}

fn apply_solid_layer_background(view: &NSView, colour: Colour) {
    view.setWantsLayer(true);
    let ns = ns_color_from_rgba(colour_to_rgba(colour));
    if let Some(layer) = view.layer() {
        layer.setBackgroundColor(Some(&ns.CGColor()));
    }
}

fn apply_translucent_effect_background(
    mtm: MainThreadMarker,
    view: &NSView,
    handle: NativeHandle,
    colour: Colour,
) {
    view.setWantsLayer(false);
    let bounds = view.bounds();
    let effect = EFFECT_BACKDROPS.with(|map| {
        if let Some(existing) = map.borrow().get(&handle) {
            existing.clone()
        } else {
            let effect = NSVisualEffectView::new(mtm);
            effect.setFrame(bounds);
            effect.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            effect.setMaterial(NSVisualEffectMaterial::ContentBackground);
            effect.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
            effect.setState(NSVisualEffectState::Active);
            view.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, None);
            map.borrow_mut().insert(handle, effect.clone());
            effect
        }
    });
    effect.setFrame(bounds);
    effect.setWantsLayer(true);
    let ns = ns_color_from_rgba(colour_to_rgba(colour));
    if let Some(layer) = effect.layer() {
        layer.setBackgroundColor(Some(&ns.CGColor()));
    }
}

fn apply_background_to_view(view: &NSView, handle: NativeHandle, colour: Colour) {
    if colour.a == u8::MAX {
        remove_effect_backdrop(handle);
        apply_solid_layer_background(view, colour);
    } else if let Some(mtm) = MainThreadMarker::new() {
        apply_translucent_effect_background(mtm, view, handle, colour);
    }
}

fn colour_to_rgba(colour: Colour) -> Rgba {
    (colour.r, colour.g, colour.b, colour.a)
}

pub(super) fn set_container_background(container: NativeHandle, colour: Colour) {
    CONTAINERS.with(|map| {
        if let Some(view) = map.borrow().get(&container) {
            apply_background_to_view(view, container, colour);
        }
    });
}

pub(super) fn frame_set_content_background(frame_handle: NativeHandle, colour: Colour) {
    FRAME_CONTENT.with(|map| {
        if let Some(view) = map.borrow().get(&frame_handle) {
            apply_background_to_view(view, frame_handle, colour);
        }
    });
}
