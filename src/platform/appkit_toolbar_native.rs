//! macOS toolbar backed by native [`NSToolbar`] (title-bar area, top-dock only).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
// `NSToolbarSeparatorItemIdentifier` is deprecated by Apple ("no longer recommended
// and will be ignored on 10.7 and later"); it is still the canonical identifier that
// AppKit maps back to `NSToolbarSeparatorItem`, so it is kept on purpose.
#[allow(deprecated)]
use objc2_app_kit::{
    NSBezelStyle, NSButton, NSCellImagePosition, NSImageScaling, NSToolbar, NSToolbarDelegate,
    NSToolbarDisplayMode, NSToolbarItem, NSToolbarSeparatorItemIdentifier,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSString};

use super::{
    button_target_new, image_for_display, image_list_image_at, ns_window_for_frame_handle,
    toolbar_icon_side, NativeHandle, ToolbarPendingItem, TOOLBARS, TOOLBAR_DOCK_TOP,
};

thread_local! {
    static NATIVE_TOOLBARS: RefCell<HashMap<NativeHandle, Retained<NSToolbar>>> =
        RefCell::new(HashMap::new());
    static NATIVE_TOOLBAR_DELEGATES: RefCell<HashMap<NativeHandle, Retained<ToolbarNativeDelegate>>> =
        RefCell::new(HashMap::new());
    /// At most one native toolbar per frame (AppKit window).
    static NATIVE_TOOLBAR_BY_FRAME: RefCell<HashMap<NativeHandle, NativeHandle>> =
        RefCell::new(HashMap::new());
}

pub(super) fn toolbar_is_native(toolbar_handle: NativeHandle) -> bool {
    NATIVE_TOOLBARS.with(|map| map.borrow().contains_key(&toolbar_handle))
}

pub(super) fn should_use_native(toolbar_handle: NativeHandle) -> bool {
    if toolbar_is_floating(toolbar_handle) {
        return false;
    }
    TOOLBARS.with(|map| {
        let map_b = map.borrow();
        let Some(tb) = map_b.get(&toolbar_handle) else {
            return false;
        };
        let s = tb.borrow();
        s.prefer_native
            && s.dock_side.get() == TOOLBAR_DOCK_TOP
            && !frame_has_other_native_toolbar(s.frame_handle, toolbar_handle)
    })
}

fn frame_has_other_native_toolbar(
    frame_handle: NativeHandle,
    toolbar_handle: NativeHandle,
) -> bool {
    NATIVE_TOOLBAR_BY_FRAME.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .is_some_and(|h| *h != toolbar_handle)
    })
}

fn toolbar_is_floating(toolbar_handle: NativeHandle) -> bool {
    TOOLBARS.with(|map| {
        map.borrow()
            .get(&toolbar_handle)
            .map(|tb| tb.borrow().floating_window.get() != 0)
            .unwrap_or(false)
    })
}

fn build_item_identifiers(
    with_gripper: bool,
    pending: &[ToolbarPendingItem],
) -> Vec<Retained<NSString>> {
    let mut ids = Vec::new();
    let mut sep_idx = 0usize;
    if with_gripper {
        ids.push(NSString::from_str("ruwx.gripper"));
    }
    for spec in pending {
        match spec {
            ToolbarPendingItem::Separator => {
                ids.push(NSString::from_str(&format!("ruwx.sep.{sep_idx}")));
                sep_idx += 1;
            }
            ToolbarPendingItem::Tool { id, .. } => {
                ids.push(NSString::from_str(&format!("ruwx.tool.{id}")));
            }
        }
    }
    ids
}

fn make_gripper_item(
    mtm: MainThreadMarker,
    frame_handle: NativeHandle,
    gripper_id: u16,
) -> Retained<NSToolbarItem> {
    let ident = NSString::from_str("ruwx.gripper");
    let item = NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), &ident);
    let target = button_target_new(mtm, frame_handle, gripper_id);
    super::MENU_TARGETS.with(|m| m.borrow_mut().push(target.clone()));
    let btn = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("≡"),
            Some(&target),
            Some(sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    btn.setBezelStyle(NSBezelStyle::SmallSquare);
    item.setView(Some(&btn));
    item.setLabel(&NSString::from_str("Gripper"));
    item
}

// `NSBezelStyleShadowlessSquare` (raw value 6) has no non-deprecated equivalent in
// objc2-app-kit 0.3; it is required for the borderless toolbar item look.
#[allow(deprecated)]
fn make_tool_item(
    mtm: MainThreadMarker,
    frame_handle: NativeHandle,
    command_id: u16,
    image_index: i32,
    list_h: NativeHandle,
    icon_side: f64,
    tooltip: &str,
) -> Retained<NSToolbarItem> {
    let ident = NSString::from_str(&format!("ruwx.tool.{command_id}"));
    let item = NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), &ident);
    let target = button_target_new(mtm, frame_handle, command_id);
    super::MENU_TARGETS.with(|m| m.borrow_mut().push(target.clone()));
    let btn = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(""),
            Some(&target),
            Some(sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    btn.setBezelStyle(NSBezelStyle::ShadowlessSquare);
    btn.setBordered(false);
    if !tooltip.is_empty() {
        btn.setToolTip(Some(&NSString::from_str(tooltip)));
    }
    if image_index >= 0 {
        if let Some(img) = image_list_image_at(list_h, image_index) {
            let sized = image_for_display(&img, icon_side);
            btn.setImage(Some(&sized));
            btn.setImagePosition(NSCellImagePosition::ImageOnly);
            btn.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        }
    }
    item.setView(Some(&btn));
    item.setLabel(&ident);
    item
}

#[derive(Default)]
struct ToolbarNativeDelegateIvars {
    toolbar_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ToolbarNativeDelegateIvars]
    struct ToolbarNativeDelegate;

    unsafe impl NSObjectProtocol for ToolbarNativeDelegate {}

    unsafe impl NSToolbarDelegate for ToolbarNativeDelegate {
        // These are Objective-C `NSToolbarDelegate` methods: the names mirror
        // the selectors they implement, so snake_case must not be enforced.
        #[allow(non_snake_case)]
        #[unsafe(method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:))]
        fn toolbar_itemForItemIdentifier_willBeInsertedIntoToolbar(
            &self,
            _toolbar: &NSToolbar,
            item_identifier: &NSString,
            _flag: bool,
        ) -> Option<Retained<NSToolbarItem>> {
            self.item_for_identifier(item_identifier)
        }

        #[allow(non_snake_case)]
        #[unsafe(method_id(toolbarDefaultItemIdentifiers:))]
        fn toolbarDefaultItemIdentifiers(
            &self,
            _toolbar: &NSToolbar,
        ) -> Retained<NSArray<NSString>> {
            self.default_item_identifiers()
        }

        #[allow(non_snake_case)]
        #[unsafe(method_id(toolbarAllowedItemIdentifiers:))]
        fn toolbarAllowedItemIdentifiers(
            &self,
            _toolbar: &NSToolbar,
        ) -> Retained<NSArray<NSString>> {
            self.default_item_identifiers()
        }
    }
);

// `NSToolbarSeparatorItemIdentifier` is deprecated by Apple but still the identifier
// AppKit maps back to `NSToolbarSeparatorItem`; kept on purpose (see the import).
#[allow(deprecated)]
impl ToolbarNativeDelegate {
    fn item_for_identifier(&self, item_identifier: &NSString) -> Option<Retained<NSToolbarItem>> {
        let mtm = MainThreadMarker::new()?;
        let handle = self.ivars().toolbar_handle.get();
        let id_str = item_identifier.to_string();

        if id_str == "ruwx.gripper" {
            let (frame_handle, gripper_id) = TOOLBARS.with(|map| {
                let map_b = map.borrow();
                let tb = map_b.get(&handle)?;
                let s = tb.borrow();
                Some((s.frame_handle, s.gripper_id))
            })?;
            return Some(make_gripper_item(mtm, frame_handle, gripper_id));
        }

        if id_str.starts_with("ruwx.tool.") {
            let cmd_id: u16 = id_str.strip_prefix("ruwx.tool.")?.parse().ok()?;
            let (frame_handle, list_h, img_w, img_h, bar_h, image_index, tooltip) =
                TOOLBARS.with(|map| {
                    let map_b = map.borrow();
                    let tb = map_b.get(&handle)?;
                    let s = tb.borrow();
                    let found = s.pending.iter().find_map(|spec| match spec {
                        ToolbarPendingItem::Tool {
                            id,
                            image_index,
                            tooltip,
                        } if *id == cmd_id => Some((*image_index, tooltip.clone())),
                        _ => None,
                    })?;
                    let (image_index, tooltip) = found;
                    Some((
                        s.frame_handle,
                        s.image_list_handle,
                        s.image_w,
                        s.image_h,
                        s.height,
                        image_index,
                        tooltip,
                    ))
                })?;
            let icon_side = toolbar_icon_side(bar_h, img_w as f64, img_h as f64);
            return Some(make_tool_item(
                mtm,
                frame_handle,
                cmd_id,
                image_index,
                list_h,
                icon_side,
                &tooltip,
            ));
        }

        if id_str.starts_with("ruwx.sep.") {
            let item = NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), unsafe {
                NSToolbarSeparatorItemIdentifier
            });
            return Some(item);
        }

        None
    }

    fn default_item_identifiers(&self) -> Retained<NSArray<NSString>> {
        let handle = self.ivars().toolbar_handle.get();
        let ids = TOOLBARS.with(|map| {
            let map_b = map.borrow();
            map_b.get(&handle).map(|tb| {
                let s = tb.borrow();
                build_item_identifiers(s.with_gripper, &s.pending)
            })
        });
        if let Some(ids) = ids {
            NSArray::from_retained_slice(&ids)
        } else {
            NSArray::from_retained_slice(&[])
        }
    }

    fn new(mtm: MainThreadMarker, toolbar_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ToolbarNativeDelegateIvars {
            toolbar_handle: Cell::new(toolbar_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) fn toolbar_detach_native(toolbar_handle: NativeHandle) {
    let toolbar = NATIVE_TOOLBARS.with(|map| map.borrow_mut().remove(&toolbar_handle));
    let Some(toolbar) = toolbar else {
        return;
    };
    TOOLBARS.with(|map| {
        if let Some(tb) = map.borrow().get(&toolbar_handle) {
            let frame_handle = tb.borrow().frame_handle;
            NATIVE_TOOLBAR_BY_FRAME.with(|frames| {
                frames.borrow_mut().remove(&frame_handle);
            });
            if let Some(window) = ns_window_for_frame_handle(frame_handle) {
                window.setToolbar(None);
            }
        }
    });
    NATIVE_TOOLBAR_DELEGATES.with(|d| d.borrow_mut().remove(&toolbar_handle));
    let _ = toolbar;
}

pub(super) fn toolbar_realize_native(toolbar_handle: NativeHandle) -> bool {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return false,
    };

    if !should_use_native(toolbar_handle) {
        return false;
    }

    let spec = TOOLBARS.with(|map| {
        let map_b = map.borrow();
        let tb = map_b.get(&toolbar_handle)?;
        let s = tb.borrow();
        Some((
            s.frame_handle,
            s.host.clone(),
            s.with_gripper,
            s.pending.clone(),
        ))
    });
    let Some((frame_handle, host, _with_gripper, _pending)) = spec else {
        return false;
    };

    let Some(window) = ns_window_for_frame_handle(frame_handle) else {
        return false;
    };

    toolbar_detach_native(toolbar_handle);

    let toolbar_id = NSString::from_str(&format!("ru-wx-toolbar-{toolbar_handle}"));
    let toolbar = NSToolbar::initWithIdentifier(NSToolbar::alloc(mtm), &toolbar_id);
    toolbar.setAllowsUserCustomization(false);
    toolbar.setDisplayMode(NSToolbarDisplayMode::IconOnly);

    let delegate = ToolbarNativeDelegate::new(mtm, toolbar_handle);
    NATIVE_TOOLBAR_DELEGATES.with(|d| d.borrow_mut().insert(toolbar_handle, delegate.clone()));
    let delegate_proto: Retained<ProtocolObject<dyn NSToolbarDelegate>> =
        ProtocolObject::from_retained(delegate);
    toolbar.setDelegate(Some(&delegate_proto));

    window.setToolbar(Some(&toolbar));

    host.removeFromSuperview();

    NATIVE_TOOLBARS.with(|map| {
        map.borrow_mut().insert(toolbar_handle, toolbar);
    });
    NATIVE_TOOLBAR_BY_FRAME.with(|map| {
        map.borrow_mut().insert(frame_handle, toolbar_handle);
    });

    true
}
