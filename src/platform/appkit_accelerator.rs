//! Map portable [`Accelerator`] values to AppKit menu key equivalents.

use std::cell::Cell;

use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSMenuItem, NSScreen, NSView};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSString};

use crate::core::accelerator::{Accelerator, Modifiers, VirtualKey};
use crate::core::context_menu_event::ContextMenuEvent;
use crate::core::geometry::Point;

use super::{dispatch_command, resolve_frame_handle, NativeHandle, FRAME_CONTENT, FRAME_DATA_WEAK};

/// Portable `Ctrl` maps to the Command key on macOS (wxWidgets convention).
pub fn modifiers_to_appkit(modifiers: Modifiers) -> NSEventModifierFlags {
    let mut flags = NSEventModifierFlags::empty();
    if modifiers.ctrl() {
        flags |= NSEventModifierFlags::Command;
    }
    if modifiers.alt() {
        flags |= NSEventModifierFlags::Option;
    }
    if modifiers.shift() {
        flags |= NSEventModifierFlags::Shift;
    }
    flags
}

fn function_key(code: u32) -> Option<String> {
    char::from_u32(code).map(|ch| ch.to_string())
}

/// Key-equivalent string for `NSMenuItem::setKeyEquivalent`.
pub fn virtual_key_to_equivalent(key: VirtualKey) -> Option<String> {
    match key {
        VirtualKey::Char(c) if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase().to_string()),
        VirtualKey::Char(c) if c.is_ascii_digit() => Some(c.to_string()),
        VirtualKey::Char(_) => None,
        VirtualKey::F1 => function_key(objc2_app_kit::NSF1FunctionKey),
        VirtualKey::F2 => function_key(objc2_app_kit::NSF2FunctionKey),
        VirtualKey::F3 => function_key(objc2_app_kit::NSF3FunctionKey),
        VirtualKey::F4 => function_key(objc2_app_kit::NSF4FunctionKey),
        VirtualKey::F5 => function_key(objc2_app_kit::NSF5FunctionKey),
        VirtualKey::F6 => function_key(objc2_app_kit::NSF6FunctionKey),
        VirtualKey::F7 => function_key(objc2_app_kit::NSF7FunctionKey),
        VirtualKey::F8 => function_key(objc2_app_kit::NSF8FunctionKey),
        VirtualKey::F9 => function_key(objc2_app_kit::NSF9FunctionKey),
        VirtualKey::F10 => function_key(objc2_app_kit::NSF10FunctionKey),
        VirtualKey::F11 => function_key(objc2_app_kit::NSF11FunctionKey),
        VirtualKey::F12 => function_key(objc2_app_kit::NSF12FunctionKey),
        VirtualKey::Escape => Some("\u{1b}".to_string()),
        VirtualKey::Tab => Some("\t".to_string()),
        VirtualKey::Enter => Some("\r".to_string()),
        VirtualKey::Space => Some(" ".to_string()),
        VirtualKey::Backspace => Some("\u{8}".to_string()),
        VirtualKey::Delete => function_key(objc2_app_kit::NSDeleteFunctionKey),
        VirtualKey::Insert => function_key(objc2_app_kit::NSInsertFunctionKey),
        VirtualKey::Home => function_key(objc2_app_kit::NSHomeFunctionKey),
        VirtualKey::End => function_key(objc2_app_kit::NSEndFunctionKey),
        VirtualKey::PageUp => function_key(objc2_app_kit::NSPageUpFunctionKey),
        VirtualKey::PageDown => function_key(objc2_app_kit::NSPageDownFunctionKey),
        VirtualKey::Left => function_key(objc2_app_kit::NSLeftArrowFunctionKey),
        VirtualKey::Right => function_key(objc2_app_kit::NSRightArrowFunctionKey),
        VirtualKey::Up => function_key(objc2_app_kit::NSUpArrowFunctionKey),
        VirtualKey::Down => function_key(objc2_app_kit::NSDownArrowFunctionKey),
    }
}

pub fn apply_accelerator_to_menu_item(item: &NSMenuItem, shortcut: Option<&Accelerator>) {
    match shortcut.and_then(|acc| virtual_key_to_equivalent(acc.key).map(|key| (key, acc))) {
        Some((key, acc)) => {
            item.setKeyEquivalent(&NSString::from_str(&key));
            item.setKeyEquivalentModifierMask(modifiers_to_appkit(acc.modifiers));
        }
        None => {
            item.setKeyEquivalent(&NSString::from_str(""));
            item.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
        }
    }
}

fn normalized_event_modifiers(event: &NSEvent) -> NSEventModifierFlags {
    event.modifierFlags() & NSEventModifierFlags::DeviceIndependentFlagsMask
}

pub fn event_matches_accelerator(event: &NSEvent, accel: &Accelerator) -> bool {
    if normalized_event_modifiers(event) != modifiers_to_appkit(accel.modifiers) {
        return false;
    }
    let Some(expected) = virtual_key_to_equivalent(accel.key) else {
        return false;
    };
    let Some(got) = event.charactersIgnoringModifiers() else {
        return false;
    };
    got.to_string() == expected
}

pub fn dispatch_frame_accelerator(frame_handle: NativeHandle, event: &NSEvent) -> bool {
    let frame_handle = resolve_frame_handle(frame_handle);
    let data_rc = FRAME_DATA_WEAK.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .and_then(|weak| weak.upgrade())
    });
    let Some(data_rc) = data_rc else {
        return false;
    };
    let accels: Vec<(Accelerator, u16)> = data_rc.borrow().accelerators.clone();
    for (accel, command_id) in accels {
        if event_matches_accelerator(event, &accel) {
            dispatch_command(frame_handle, command_id);
            return true;
        }
    }
    false
}

fn screen_point_win32_from_event(event: &NSEvent, view: &NSView) -> Point {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let window_point = event.locationInWindow();
    let screen_point = view
        .window()
        .map(|window| window.convertPointToScreen(window_point))
        .unwrap_or(window_point);
    let screen_h = NSScreen::mainScreen(mtm)
        .map(|screen| screen.frame().size.height)
        .unwrap_or(900.0);
    Point::new(
        screen_point.x.round() as i32,
        (screen_h - screen_point.y).round() as i32,
    )
}

/// Fire [`Frame::on_context_menu`](crate::window::frame::Frame::on_context_menu) handlers.
pub fn dispatch_frame_context_menu(frame_handle: NativeHandle, event: &NSEvent, view: &NSView) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let data_rc = FRAME_DATA_WEAK.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .and_then(|weak| weak.upgrade())
    });
    let Some(data_rc) = data_rc else {
        return;
    };
    let mut handlers = std::mem::take(&mut data_rc.borrow_mut().on_context_menu);
    if handlers.is_empty() {
        data_rc.borrow_mut().on_context_menu = handlers;
        return;
    }
    let ev = ContextMenuEvent::mouse(screen_point_win32_from_event(event, view));
    for handler in handlers.iter_mut() {
        handler(&ev);
    }
    data_rc.borrow_mut().on_context_menu = handlers;
}

fn is_context_menu_key(event: &NSEvent) -> bool {
    // Shift+F10 (Windows Menu key parity) and Ctrl+Shift+click alternative.
    let flags = event.modifierFlags();
    let shift = flags.contains(objc2_app_kit::NSEventModifierFlags::Shift);
    let key_code = event.keyCode();
    // F10 = 109 on many Mac keyboards; also check Unicode Menu character.
    shift && key_code == 109
}

/// Fire keyboard-initiated context menu handlers at the focused point.
pub fn dispatch_frame_context_menu_keyboard(frame_handle: NativeHandle, view: &NSView) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let data_rc = FRAME_DATA_WEAK.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .and_then(|weak| weak.upgrade())
    });
    let Some(data_rc) = data_rc else {
        return;
    };
    let mut handlers = std::mem::take(&mut data_rc.borrow_mut().on_context_menu);
    if handlers.is_empty() {
        data_rc.borrow_mut().on_context_menu = handlers;
        return;
    }
    let bounds = view.bounds();
    let pt = Point::new(
        (view.frame().origin.x + bounds.size.width / 2.0).round() as i32,
        (view.frame().origin.y + bounds.size.height / 2.0).round() as i32,
    );
    let ev = ContextMenuEvent::keyboard(pt);
    for handler in handlers.iter_mut() {
        handler(&ev);
    }
    data_rc.borrow_mut().on_context_menu = handlers;
}

#[derive(Default)]
struct FrameContentViewIvars {
    frame_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FrameContentViewIvars]
    #[name = "RuWxFrameContentView"]
    struct FrameContentView;

    unsafe impl NSObjectProtocol for FrameContentView {}

    impl FrameContentView {
        #[unsafe(method(performKeyEquivalent:))]
        fn perform_key_equivalent(&self, event: &NSEvent) -> Bool {
            if dispatch_frame_accelerator(self.ivars().frame_handle.get(), event) {
                Bool::YES
            } else {
                unsafe { msg_send![super(self), performKeyEquivalent: event] }
            }
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            dispatch_frame_context_menu(self.ivars().frame_handle.get(), event, self);
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if is_context_menu_key(event) {
                dispatch_frame_context_menu_keyboard(self.ivars().frame_handle.get(), self);
                return;
            }
            unsafe { msg_send![super(self), keyDown: event] }
        }
    }
);

impl FrameContentView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect, frame_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FrameContentViewIvars {
            frame_handle: Cell::new(frame_handle),
        });
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

/// Install a content view that dispatches frame-level keyboard accelerators.
pub fn replace_window_content_view(
    mtm: MainThreadMarker,
    window: &objc2_app_kit::NSWindow,
    frame_handle: NativeHandle,
) -> Retained<NSView> {
    use objc2_foundation::NSSize;
    let bounds = window
        .contentView()
        .map(|view| view.bounds())
        .unwrap_or_else(|| NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)));
    let view = FrameContentView::new(mtm, bounds, frame_handle);
    window.setContentView(Some(view.as_ref()));
    let stored: Retained<NSView> = view.into_super();
    FRAME_CONTENT.with(|map| {
        map.borrow_mut().insert(frame_handle, stored.clone());
    });
    stored
}
