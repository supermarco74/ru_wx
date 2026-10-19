//! macOS drag-out via `NSPasteboard` + `NSDraggingSource`.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSDragOperation, NSDraggingContext, NSDraggingSource, NSImage, NSPasteboard,
    NSPasteboardWriting, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSPoint, NSSize, NSString, NSURL,
};

use crate::dnd::ole_dnd::{OleDragData, OleDragError, OleDragSourceCallbacks, OleDropEffect};

use super::{menu_icon_from_colour, NativeHandle, FRAME_CONTENT, VIEWS};

thread_local! {
    static DRAG_SOURCES: RefCell<Vec<Retained<DragSourceDelegate>>> = RefCell::new(Vec::new());
    static DRAG_CALLBACKS: RefCell<Option<OleDragSourceCallbacks>> = RefCell::new(None);
}

struct DragSourceDelegateIvars {
    allowed_mask: Cell<NSDragOperation>,
    result: Cell<OleDropEffect>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = DragSourceDelegateIvars]
    #[name = "RuWxDragSourceDelegate"]
    struct DragSourceDelegate;

    unsafe impl NSObjectProtocol for DragSourceDelegate {}

    unsafe impl NSDraggingSource for DragSourceDelegate {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn dragging_session_source_operation_mask(
            &self,
            _session: &objc2_app_kit::NSDraggingSession,
            _context: NSDraggingContext,
        ) -> NSDragOperation {
            self.ivars().allowed_mask.get()
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn dragging_session_ended_at_point_operation(
            &self,
            _session: &objc2_app_kit::NSDraggingSession,
            _screen_point: NSPoint,
            operation: NSDragOperation,
        ) {
            let mut effect = ns_operation_to_ole(operation);
            DRAG_CALLBACKS.with(|slot| {
                if let Some(callbacks) = slot.borrow_mut().as_mut() {
                    if let Some(ref mut feedback) = callbacks.on_give_feedback {
                        effect = feedback(effect);
                    }
                }
            });
            self.ivars().result.set(effect);
        }
    }
);

impl DragSourceDelegate {
    fn new(mtm: MainThreadMarker, allowed: OleDropEffect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DragSourceDelegateIvars {
            allowed_mask: Cell::new(ole_allowed_to_ns_mask(allowed)),
            result: Cell::new(OleDropEffect::NONE),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn ole_allowed_to_ns_mask(allowed: OleDropEffect) -> NSDragOperation {
    let mut mask = NSDragOperation::None;
    if allowed.contains(OleDropEffect::COPY) {
        mask |= NSDragOperation::Copy;
    }
    if allowed.contains(OleDropEffect::MOVE) {
        mask |= NSDragOperation::Move;
    }
    if allowed.contains(OleDropEffect::LINK) {
        mask |= NSDragOperation::Link;
    }
    if mask == NSDragOperation::None {
        mask = NSDragOperation::Copy;
    }
    mask
}

fn ns_operation_to_ole(operation: NSDragOperation) -> OleDropEffect {
    let mut effect = OleDropEffect::NONE;
    if operation.contains(NSDragOperation::Copy) {
        effect |= OleDropEffect::COPY;
    }
    if operation.contains(NSDragOperation::Move) {
        effect |= OleDropEffect::MOVE;
    }
    if operation.contains(NSDragOperation::Link) {
        effect |= OleDropEffect::LINK;
    }
    effect
}

fn view_for_handle(handle: NativeHandle) -> Option<Retained<NSView>> {
    VIEWS
        .with(|map| map.borrow().get(&handle).cloned())
        .or_else(|| FRAME_CONTENT.with(|map| map.borrow().get(&handle).cloned()))
}

fn fill_drag_pasteboard(pboard: &NSPasteboard, data: &OleDragData) -> bool {
    pboard.clearContents();
    match data {
        OleDragData::Files(paths) if paths.is_empty() => false,
        OleDragData::Files(paths) => write_files_to_pasteboard(pboard, paths),
        OleDragData::Text(text) => {
            let plain = NSString::from_str("public.plain-text");
            let legacy = NSString::from_str("NSStringPboardType");
            let types = NSArray::from_slice(&[plain.as_ref(), legacy.as_ref()]);
            unsafe {
                pboard.declareTypes_owner(&types, None);
            }
            pboard.setString_forType(&NSString::from_str(text), &plain)
                || pboard.setString_forType(&NSString::from_str(text), &legacy)
        }
    }
}

fn write_files_to_pasteboard(pboard: &NSPasteboard, paths: &[PathBuf]) -> bool {
    let urls: Vec<Retained<NSURL>> = paths
        .iter()
        .map(|path| {
            NSURL::fileURLWithPath_isDirectory(&NSString::from_str(&path.to_string_lossy()), false)
        })
        .collect();
    if urls.is_empty() {
        return false;
    }
    let writers: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = urls
        .iter()
        .map(|url| ProtocolObject::from_retained(url.clone()))
        .collect();
    let array = NSArray::from_retained_slice(&writers);
    pboard.writeObjects(&array)
}

fn drag_image(mtm: MainThreadMarker) -> Retained<NSImage> {
    menu_icon_from_colour(96, 128, 208, 255, 32)
}

/// Begin a synchronous AppKit drag from the view identified by `view_handle`.
pub fn mac_do_drag_drop(
    view_handle: NativeHandle,
    data: &OleDragData,
    allowed: OleDropEffect,
    callbacks: &mut OleDragSourceCallbacks,
) -> Result<OleDropEffect, OleDragError> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let Some(view) = view_for_handle(view_handle) else {
        return Err(OleDragError::DoDragDropFailed(-2147467263)); // E_NOTIMPL
    };
    let Some(event) = NSApplication::sharedApplication(mtm).currentEvent() else {
        return Err(OleDragError::DoDragDropFailed(-2147467263));
    };

    let pboard = NSPasteboard::pasteboardWithName(unsafe { objc2_app_kit::NSPasteboardNameDrag });
    if !fill_drag_pasteboard(&pboard, data) {
        return Err(OleDragError::DoDragDropFailed(-2147024809)); // E_INVALIDARG
    }

    let delegate = DragSourceDelegate::new(mtm, allowed);
    DRAG_SOURCES.with(|vec| vec.borrow_mut().push(delegate.clone()));
    DRAG_CALLBACKS.with(|slot| {
        *slot.borrow_mut() = Some(std::mem::take(callbacks));
    });

    let bounds = view.bounds();
    let location = NSPoint::new(bounds.size.width * 0.5, bounds.size.height * 0.5);
    let image = drag_image(mtm);
    unsafe {
        let source: &ProtocolObject<dyn NSDraggingSource> = ProtocolObject::from_ref(&*delegate);
        view.dragImage_at_offset_event_pasteboard_source_slideBack(
            &image,
            location,
            NSSize::new(0.0, 0.0),
            &event,
            &pboard,
            source.as_ref(),
            true,
        );
    }

    let effect = delegate.ivars().result.get();
    DRAG_CALLBACKS.with(|slot| {
        if let Some(stored) = slot.borrow_mut().take() {
            *callbacks = stored;
        }
    });
    Ok(effect)
}
