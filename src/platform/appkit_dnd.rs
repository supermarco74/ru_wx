//! macOS drag-and-drop onto frame content views (NSPasteboard).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, ProtocolObject};
use objc2::ClassType;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSDragOperation, NSDraggingDestination, NSDraggingInfo, NSView,
    NSWindowOrderingMode,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObjectProtocol, NSRect, NSString, NSURL};

use crate::dnd::drop_target::DroppedFiles;
use crate::OleDropPosition;
use crate::OleDroppedData;

use super::{NativeHandle, FRAME_CONTENT, FRAME_DATA_WEAK};

thread_local! {
    static FRAME_DROP_VIEWS: RefCell<HashMap<NativeHandle, Retained<FileDropView>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct FileDropViewIvars {
    frame_handle: Cell<NativeHandle>,
    ole_enabled: Cell<bool>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FileDropViewIvars]
    struct FileDropView;

    unsafe impl NSObjectProtocol for FileDropView {}

    unsafe impl NSDraggingDestination for FileDropView {
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            drag_operation_for(
                info,
                self.ivars().frame_handle.get(),
                self.ivars().ole_enabled.get(),
            )
        }

        #[unsafe(method(draggingUpdated:))]
        fn dragging_updated(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            drag_operation_for(
                info,
                self.ivars().frame_handle.get(),
                self.ivars().ole_enabled.get(),
            )
        }

        #[unsafe(method(draggingExited:))]
        fn dragging_exited(&self, _info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {}

        #[unsafe(method(prepareForDragOperation:))]
        fn prepare_for_drag_operation(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            drop_payload(
                info,
                self.ivars().frame_handle.get(),
                self.ivars().ole_enabled.get(),
            )
            .is_some()
        }

        #[unsafe(method(performDragOperation:))]
        fn perform_drag_operation(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> Bool {
            let frame_handle = self.ivars().frame_handle.get();
            let ole = self.ivars().ole_enabled.get();
            let Some((data, pos)) = drop_payload(info, frame_handle, ole) else {
                return Bool::NO;
            };
            dispatch_drop(frame_handle, data, pos, ole);
            Bool::YES
        }

        #[unsafe(method(concludeDragOperation:))]
        fn conclude_drag_operation(&self, _info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {}
    }
);

impl FileDropView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        frame_handle: NativeHandle,
        ole_enabled: bool,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FileDropViewIvars {
            frame_handle: Cell::new(frame_handle),
            ole_enabled: Cell::new(ole_enabled),
        });
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        view.registerForDraggedTypes(&drag_types());
        view
    }
}

fn drag_operation_for(
    info: &ProtocolObject<dyn NSDraggingInfo>,
    frame_handle: NativeHandle,
    ole_enabled: bool,
) -> NSDragOperation {
    if drop_payload(info, frame_handle, ole_enabled).is_some() {
        NSDragOperation::Copy
    } else {
        NSDragOperation::None
    }
}

fn drag_types() -> Retained<NSArray<NSString>> {
    NSArray::from_retained_slice(&[
        NSString::from_str("public.file-url"),
        NSString::from_str("NSFilenamesPboardType"),
        NSString::from_str("public.url"),
        NSString::from_str("public.plain-text"),
        NSString::from_str("NSStringPboardType"),
    ])
}

fn url_to_path(url: &NSURL) -> Option<PathBuf> {
    url.path().map(|s| PathBuf::from(s.to_string()))
}

fn paths_from_url_array(objects: &NSArray) -> Vec<PathBuf> {
    (0..objects.count())
        .filter_map(|i| {
            objects
                .objectAtIndex(i)
                .downcast_ref::<NSURL>()
                .and_then(url_to_path)
        })
        .collect()
}

fn paths_from_string_array(objects: &NSArray) -> Vec<PathBuf> {
    (0..objects.count())
        .filter_map(|i| {
            objects
                .objectAtIndex(i)
                .downcast_ref::<NSString>()
                .map(|s| PathBuf::from(s.to_string()))
        })
        .collect()
}

fn paths_from_property_list(obj: &AnyObject) -> Vec<PathBuf> {
    if let Some(urls) = obj.downcast_ref::<NSArray>() {
        let as_urls = paths_from_url_array(urls);
        if !as_urls.is_empty() {
            return as_urls;
        }
        return paths_from_string_array(urls);
    }
    if let Some(url) = obj.downcast_ref::<NSURL>() {
        return url_to_path(url).into_iter().collect();
    }
    if let Some(path) = obj.downcast_ref::<NSString>() {
        return vec![PathBuf::from(path.to_string())];
    }
    Vec::new()
}

fn paths_from_drag(info: &ProtocolObject<dyn NSDraggingInfo>) -> Vec<PathBuf> {
    let pboard = info.draggingPasteboard();

    // Finder multi-select: one NSURL per file.
    let classes = NSArray::from_slice(&[NSURL::class()]);
    let urls = unsafe {
        pboard
            .readObjectsForClasses_options(&classes, None)
            .map(|objects| paths_from_url_array(&objects))
            .unwrap_or_default()
    };
    if !urls.is_empty() {
        return urls;
    }

    // Legacy multi-file drag: property list of path strings.
    let filenames = NSString::from_str("NSFilenamesPboardType");
    if let Some(plist) = pboard.propertyListForType(&filenames) {
        let paths = paths_from_property_list(&plist);
        if !paths.is_empty() {
            return paths;
        }
    }

    // Single file-url string fallback.
    if let Some(raw) = pboard.stringForType(&NSString::from_str("public.file-url")) {
        if let Some(url) = NSURL::URLWithString(&raw) {
            if let Some(path) = url_to_path(&url) {
                return vec![path];
            }
        }
    }

    Vec::new()
}

fn text_from_drag(info: &ProtocolObject<dyn NSDraggingInfo>) -> Option<String> {
    let pboard = info.draggingPasteboard();
    for ty in ["public.plain-text", "NSStringPboardType", "public.url"] {
        if let Some(raw) = pboard.stringForType(&NSString::from_str(ty)) {
            let s = raw.to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn drop_position(info: &ProtocolObject<dyn NSDraggingInfo>, view: &NSView) -> OleDropPosition {
    let point = info.draggingLocation();
    let local = view.convertPoint_fromView(point, None);
    let view_height = view.bounds().size.height;
    OleDropPosition {
        x: local.x as i32,
        y: (view_height - local.y) as i32,
    }
}

fn drop_payload(
    info: &ProtocolObject<dyn NSDraggingInfo>,
    frame_handle: NativeHandle,
    ole_enabled: bool,
) -> Option<(OleDroppedData, OleDropPosition)> {
    let view = FRAME_CONTENT.with(|m| m.borrow().get(&frame_handle).cloned())?;
    let pos = drop_position(info, &view);
    if ole_enabled {
        let paths = paths_from_drag(info);
        if !paths.is_empty() {
            return Some((OleDroppedData::Files(paths), pos));
        }
        if let Some(text) = text_from_drag(info) {
            return Some((OleDroppedData::Text(text), pos));
        }
        return Some((OleDroppedData::Other, pos));
    }
    let paths = paths_from_drag(info);
    if paths.is_empty() {
        return None;
    }
    Some((OleDroppedData::Files(paths), pos))
}

fn dispatch_drop(
    frame_handle: NativeHandle,
    data: OleDroppedData,
    pos: OleDropPosition,
    ole_enabled: bool,
) {
    let data_rc = FRAME_DATA_WEAK.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .and_then(|weak| weak.upgrade())
    });
    let Some(data_rc) = data_rc else {
        return;
    };
    let mut inner = data_rc.borrow_mut();
    if ole_enabled {
        if let Some(h) = inner.ole_drop_callback.as_mut() {
            h(data, pos);
            return;
        }
    }
    if let OleDroppedData::Files(paths) = data {
        let dropped = DroppedFiles::new(paths);
        if let Some(h) = inner.drop_files_handler.as_mut() {
            h(dropped);
        }
    }
}

/// Install a file-only drop catcher over the frame content view.
pub fn enable_frame_file_drop(frame_handle: NativeHandle) {
    install_drop_view(frame_handle, false);
}

/// Install an OLE-style drop catcher (files, text, URLs).
pub fn enable_frame_ole_drop(frame_handle: NativeHandle) {
    install_drop_view(frame_handle, true);
}

fn install_drop_view(frame_handle: NativeHandle, ole_enabled: bool) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let Some(content) = FRAME_CONTENT.with(|m| m.borrow().get(&frame_handle).cloned()) else {
        return;
    };
    if FRAME_DROP_VIEWS.with(|map| map.borrow().contains_key(&frame_handle)) {
        if let Some(existing) =
            FRAME_DROP_VIEWS.with(|map| map.borrow().get(&frame_handle).cloned())
        {
            existing.ivars().ole_enabled.set(ole_enabled);
        }
        return;
    }
    let bounds = content.bounds();
    let drop_view = FileDropView::new(mtm, bounds, frame_handle, ole_enabled);
    content.addSubview_positioned_relativeTo(&drop_view, NSWindowOrderingMode::Below, None);
    FRAME_DROP_VIEWS.with(|map| {
        map.borrow_mut().insert(frame_handle, drop_view);
    });
}
