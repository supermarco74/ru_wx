//! macOS [`ColourPickerCtrl`] backed by native [`NSColorWell`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSColor, NSColorWell, NSColorWellStyle};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};

use super::{
    alloc_widget_handle, dispatch_command, ns_color_from_rgba, parent_view_for, record_parent,
    resolve_frame_handle, ru_rect_to_ns, store_ns_view, Colour, NativeHandle, Rect, Rgba,
};

thread_local! {
    static COLOR_WELLS: RefCell<HashMap<NativeHandle, Retained<NSColorWell>>> =
        RefCell::new(HashMap::new());
    static COLOR_WELL_TARGETS: RefCell<HashMap<NativeHandle, Retained<ColorWellTarget>>> =
        RefCell::new(HashMap::new());
    static COLOR_PICKER_COLOURS: RefCell<HashMap<NativeHandle, Colour>> =
        RefCell::new(HashMap::new());
}

fn ns_color_to_colour(color: &NSColor) -> Colour {
    Colour::new(
        (color.redComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
        (color.greenComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
        (color.blueComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
        (color.alphaComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn color_well_for(handle: NativeHandle) -> Option<Retained<NSColorWell>> {
    COLOR_WELLS.with(|map| map.borrow().get(&handle).cloned())
}

#[derive(Default)]
struct ColorWellTargetIvars {
    picker_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ColorWellTargetIvars]
    struct ColorWellTarget;

    unsafe impl NSObjectProtocol for ColorWellTarget {}

    impl ColorWellTarget {
        #[unsafe(method(ruWxColorWellChanged:))]
        fn color_well_changed(&self, sender: Option<&AnyObject>) {
            let Some(well) = sender.and_then(|s| s.downcast_ref::<NSColorWell>()) else {
                return;
            };
            let handle = self.ivars().picker_handle.get();
            let colour = ns_color_to_colour(&well.color());
            COLOR_PICKER_COLOURS.with(|map| {
                map.borrow_mut().insert(handle, colour);
            });
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }
);

impl ColorWellTarget {
    fn new(
        mtm: MainThreadMarker,
        picker_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ColorWellTargetIvars {
            picker_handle: Cell::new(picker_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn create_colour_picker_native(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    colour: Colour,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let frame_handle = resolve_frame_handle(parent);

    COLOR_PICKER_COLOURS.with(|map| {
        map.borrow_mut().insert(handle, colour);
    });

    let content_h = super::parent_height_for(parent);
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let well = NSColorWell::colorWellWithStyle(NSColorWellStyle::Expanded, mtm);
    well.setFrame(ns_rect);
    well.setSupportsAlpha(false);
    well.setColor(&ns_color_from_rgba(colour_to_rgba(colour)));

    let target = ColorWellTarget::new(mtm, handle, frame_handle, command_id);
    COLOR_WELL_TARGETS.with(|map| map.borrow_mut().insert(handle, target.clone()));
    unsafe {
        well.setTarget(Some(&target));
        well.setAction(Some(sel!(ruWxColorWellChanged:)));
    }

    if let Some(parent_view) = parent_view_for(parent) {
        parent_view.addSubview(&well);
    }

    COLOR_WELLS.with(|map| map.borrow_mut().insert(handle, well.clone()));
    store_ns_view(handle, well.into_super().into_super());
    record_parent(handle, parent);
    handle
}

fn colour_to_rgba(colour: Colour) -> Rgba {
    (colour.r, colour.g, colour.b, colour.a)
}

pub fn colour_picker_set_colour(handle: NativeHandle, colour: Colour) {
    COLOR_PICKER_COLOURS.with(|map| {
        map.borrow_mut().insert(handle, colour);
    });
    if let Some(well) = color_well_for(handle) {
        well.setColor(&ns_color_from_rgba(colour_to_rgba(colour)));
    }
}

pub fn colour_picker_get_colour(handle: NativeHandle) -> Colour {
    if let Some(well) = color_well_for(handle) {
        let colour = ns_color_to_colour(&well.color());
        COLOR_PICKER_COLOURS.with(|map| {
            map.borrow_mut().insert(handle, colour);
        });
        return colour;
    }
    COLOR_PICKER_COLOURS.with(|map| map.borrow().get(&handle).copied().unwrap_or(Colour::BLACK))
}

pub fn colour_picker_activate(handle: NativeHandle) {
    if let Some(well) = color_well_for(handle) {
        well.activate(true);
    }
}
