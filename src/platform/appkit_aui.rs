//! macOS AUI helpers — floating frames, pane hosts, and caption bars.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSBezelStyle, NSBezierPath, NSButton, NSColor, NSEvent, NSFont, NSScreen,
    NSTextField, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use crate::chrome::aui_manager::{
    AuiDockArt, AuiDockArtGradient, AuiManager, AuiPaneButton, AuiSashSpec,
};
use crate::chrome::aui_tool_bar::AuiDockSide;
use crate::platform::ids::next_control_id;
use crate::window::frame::Frame;

use super::{
    alloc_widget_handle, button_target_new, child_view_for_handle, parent_view_for, record_parent,
    rect_to_ns_in_parent, set_view_frame, store_container, NativeHandle, Rect, MENU_TARGETS,
};

/// Minimum caption strip width for left/right dock sides (macOS points).
pub const SIDE_CAPTION_WIDTH: u32 = 72;

pub fn aui_side_caption_width(caption_height: u32) -> u32 {
    caption_height.max(SIDE_CAPTION_WIDTH)
}

const PANE_BTN_WIDTH: f64 = 18.0;
const PANE_CAPTION_PAD: f64 = 6.0;
const DRAG_THRESHOLD: f64 = 5.0;

thread_local! {
    static FLOATING_WINDOWS: RefCell<HashMap<NativeHandle, Retained<NSWindow>>> =
        RefCell::new(HashMap::new());
    static FLOATING_CONTENT: RefCell<HashMap<NativeHandle, Retained<NSView>>> =
        RefCell::new(HashMap::new());
    static PANE_HOSTS: RefCell<HashMap<(NativeHandle, String), PaneHostState>> =
        RefCell::new(HashMap::new());
    static PANE_BTN_COMMANDS: RefCell<HashMap<u16, (NativeHandle, String, AuiPaneButton)>> =
        RefCell::new(HashMap::new());
    static PANE_FLOATS: RefCell<HashMap<(NativeHandle, String), NativeHandle>> =
        RefCell::new(HashMap::new());
    static PANE_ACTION: RefCell<HashMap<NativeHandle, (*mut AuiManager, Frame)>> =
        RefCell::new(HashMap::new());
}

struct PaneHostState {
    host_handle: NativeHandle,
    host: Retained<FlippedView>,
    caption: Retained<AuiCaptionStripView>,
    title_field: Retained<NSTextField>,
    content: Retained<FlippedView>,
    child_handle: NativeHandle,
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    dock_side: AuiDockSide,
    floatable: bool,
    pinned: bool,
    buttons: Vec<(u16, Retained<NSButton>, AuiPaneButton)>,
}

#[derive(Default)]
struct FlippedViewIvars {}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FlippedViewIvars]
    struct FlippedView;

    unsafe impl NSObjectProtocol for FlippedView {}

    impl FlippedView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }
    }
);

impl FlippedView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FlippedViewIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

#[derive(Default)]
struct AuiCaptionStripIvars {
    pane_name: RefCell<String>,
    frame_handle: Cell<NativeHandle>,
    floatable: Cell<bool>,
    pinned: Cell<bool>,
    top_color: Cell<u32>,
    bottom_color: Cell<u32>,
    gradient: Cell<u8>,
    drag_start: Cell<(f64, f64)>,
    drag_active: Cell<bool>,
    auto_hide: Cell<bool>,
    auto_hidden: Cell<bool>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AuiCaptionStripIvars]
    struct AuiCaptionStripView;

    unsafe impl NSObjectProtocol for AuiCaptionStripView {}

    impl AuiCaptionStripView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let bounds = self.bounds();
            let top = self.ivars().top_color.get();
            let bottom = self.ivars().bottom_color.get();
            match self.ivars().gradient.get() {
                1 => draw_vertical_gradient(bounds, top, bottom),
                2 => draw_horizontal_gradient(bounds, top, bottom),
                _ => {
                    dock_color(top).setFill();
                    NSBezierPath::fillRect(bounds);
                }
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if !self.ivars().floatable.get() || self.ivars().pinned.get() {
                return;
            }
            let loc = self.convertPoint_fromView(event.locationInWindow(), None);
            self.ivars().drag_start.set((loc.x, loc.y));
            self.ivars().drag_active.set(false);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            if !self.ivars().floatable.get() || self.ivars().pinned.get() {
                return;
            }
            let loc = self.convertPoint_fromView(event.locationInWindow(), None);
            let (sx, sy) = self.ivars().drag_start.get();
            if (loc.x - sx).abs() >= DRAG_THRESHOLD || (loc.y - sy).abs() >= DRAG_THRESHOLD {
                self.ivars().drag_active.set(true);
            }
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            if self.ivars().auto_hidden.get() && self.ivars().auto_hide.get() {
                let frame_handle = self.ivars().frame_handle.get();
                let pane_name = self.ivars().pane_name.borrow().clone();
                request_pane_toggle_auto_hide(frame_handle, &pane_name);
                return;
            }
            if self.ivars().drag_active.take() {
                let frame_handle = self.ivars().frame_handle.get();
                let pane_name = self.ivars().pane_name.borrow().clone();
                request_pane_float(frame_handle, &pane_name);
            }
        }

        #[unsafe(method(mouseDoubleClick:))]
        fn mouse_double_click(&self, _event: &NSEvent) {
            let frame_handle = self.ivars().frame_handle.get();
            let pane_name = self.ivars().pane_name.borrow().clone();
            if is_pane_floating(frame_handle, &pane_name) {
                request_pane_dock(frame_handle, &pane_name);
            }
        }
    }
);

impl AuiCaptionStripView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AuiCaptionStripIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    fn apply_art(&self, art: &AuiDockArt) {
        self.ivars().top_color.set(art.active_caption());
        self.ivars()
            .bottom_color
            .set(darken_u32(art.active_caption(), 24));
        let grad = match art.gradient() {
            AuiDockArtGradient::None => 0,
            AuiDockArtGradient::Vertical => 1,
            AuiDockArtGradient::Horizontal => 2,
        };
        self.ivars().gradient.set(grad);
        self.setNeedsDisplay(true);
    }

    fn set_drag_policy(
        &self,
        frame_handle: NativeHandle,
        pane_name: &str,
        floatable: bool,
        pinned: bool,
    ) {
        self.ivars().frame_handle.set(frame_handle);
        *self.ivars().pane_name.borrow_mut() = pane_name.to_string();
        self.ivars().floatable.set(floatable);
        self.ivars().pinned.set(pinned);
    }

    fn set_auto_hide_state(&self, auto_hide: bool, auto_hidden: bool) {
        self.ivars().auto_hide.set(auto_hide);
        self.ivars().auto_hidden.set(auto_hidden);
    }
}

fn dock_color(c: u32) -> Retained<NSColor> {
    NSColor::colorWithCalibratedRed_green_blue_alpha(
        ((c >> 16) & 0xFF) as f64 / 255.0,
        ((c >> 8) & 0xFF) as f64 / 255.0,
        (c & 0xFF) as f64 / 255.0,
        1.0,
    )
}

fn darken_u32(color: u32, amount: u32) -> u32 {
    let r = ((color >> 16) & 0xFF).saturating_sub(amount);
    let g = ((color >> 8) & 0xFF).saturating_sub(amount);
    let b = (color & 0xFF).saturating_sub(amount);
    (r << 16) | (g << 8) | b
}

fn lerp_u32(a: u32, b: u32, t: f64) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ar = ((a >> 16) & 0xFF) as f64;
    let ag = ((a >> 8) & 0xFF) as f64;
    let ab = (a & 0xFF) as f64;
    let br = ((b >> 16) & 0xFF) as f64;
    let bg = ((b >> 8) & 0xFF) as f64;
    let bb = (b & 0xFF) as f64;
    let r = (ar + (br - ar) * t).round() as u32;
    let g = (ag + (bg - ag) * t).round() as u32;
    let b = (ab + (bb - ab) * t).round() as u32;
    (r << 16) | (g << 8) | b
}

fn draw_vertical_gradient(bounds: NSRect, top: u32, bottom: u32) {
    let steps = 24;
    let h = bounds.size.height.max(1.0);
    let step_h = h / steps as f64;
    for i in 0..steps {
        let t = i as f64 / (steps - 1).max(1) as f64;
        dock_color(lerp_u32(top, bottom, t)).setFill();
        let y = bounds.origin.y + i as f64 * step_h;
        let rect = NSRect::new(
            NSPoint::new(bounds.origin.x, y),
            NSSize::new(bounds.size.width, step_h + 1.0),
        );
        NSBezierPath::fillRect(rect);
    }
}

fn draw_horizontal_gradient(bounds: NSRect, left: u32, right: u32) {
    let steps = 24;
    let w = bounds.size.width.max(1.0);
    let step_w = w / steps as f64;
    for i in 0..steps {
        let t = i as f64 / (steps - 1).max(1) as f64;
        dock_color(lerp_u32(left, right, t)).setFill();
        let x = bounds.origin.x + i as f64 * step_w;
        let rect = NSRect::new(
            NSPoint::new(x, bounds.origin.y),
            NSSize::new(step_w + 1.0, bounds.size.height),
        );
        NSBezierPath::fillRect(rect);
    }
}

fn button_glyph(button: AuiPaneButton, pinned: bool) -> &'static str {
    match button {
        AuiPaneButton::Close => "×",
        AuiPaneButton::Maximize => "□",
        AuiPaneButton::Minimize => "−",
        AuiPaneButton::Pin => {
            if pinned {
                "P"
            } else {
                "p"
            }
        }
        AuiPaneButton::Custom(_) => "•",
    }
}

pub fn register_aui_pane_action_target(
    frame_handle: NativeHandle,
    mgr: *mut AuiManager,
    frame: Frame,
) {
    PANE_ACTION.with(|m| {
        if let Ok(mut map) = m.try_borrow_mut() {
            map.insert(frame_handle, (mgr, frame));
        }
    });
}

pub fn unregister_aui_pane_action_target(frame_handle: NativeHandle) {
    PANE_ACTION.with(|m| {
        m.borrow_mut().remove(&frame_handle);
    });
}

pub fn request_pane_float(frame_handle: NativeHandle, pane_name: &str) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let (x, y) = cursor_win32_screen_point(mtm);
    PANE_ACTION.with(|m| {
        if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
            // SAFETY: refreshed each `AuiManager::update`.
            unsafe {
                if let Some(mgr) = mgr.as_mut() {
                    mgr.float_pane(&frame, pane_name, x, y);
                }
            }
        }
    });
}

fn is_pane_floating(frame_handle: NativeHandle, pane_name: &str) -> bool {
    PANE_FLOATS.with(|m| {
        m.borrow()
            .contains_key(&(frame_handle, pane_name.to_string()))
    })
}

pub fn request_pane_dock(frame_handle: NativeHandle, pane_name: &str) {
    PANE_ACTION.with(|m| {
        if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
            unsafe {
                if let Some(mgr) = mgr.as_mut() {
                    mgr.dock_pane(&frame, pane_name);
                }
            }
        }
    });
}

fn request_pane_toggle_auto_hide(frame_handle: NativeHandle, pane_name: &str) {
    PANE_ACTION.with(|m| {
        if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
            unsafe {
                if let Some(mgr) = mgr.as_mut() {
                    mgr.toggle_auto_hide_pane(&frame, pane_name);
                }
            }
        }
    });
}

fn request_pane_sash_resize(
    frame_handle: NativeHandle,
    pane_name: &str,
    delta: i32,
    vertical: bool,
) {
    if delta == 0 {
        return;
    }
    PANE_ACTION.with(|m| {
        if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
            unsafe {
                if let Some(mgr) = mgr.as_mut() {
                    mgr.apply_sash_resize(&frame, pane_name, delta, vertical);
                }
            }
        }
    });
}

pub fn sync_aui_pane_host_pin(frame_handle: NativeHandle, pane_name: &str, pinned: bool) {
    PANE_HOSTS.with(|m| {
        if let Some(state) = m
            .borrow_mut()
            .get_mut(&(frame_handle, pane_name.to_string()))
        {
            state.pinned = pinned;
            state
                .caption
                .set_drag_policy(frame_handle, pane_name, state.floatable, pinned);
            for (_, btn, kind) in &state.buttons {
                if *kind == AuiPaneButton::Pin {
                    btn.setTitle(&NSString::from_str(button_glyph(
                        AuiPaneButton::Pin,
                        pinned,
                    )));
                }
            }
        }
    });
}

pub fn float_aui_pane_host(frame_handle: NativeHandle, pane_name: &str, rect: &Rect, title: &str) {
    let host = PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .map(|s| s.host.clone())
    });
    let Some(host) = host else {
        return;
    };
    let float_handle = create_floating_frame(title, *rect);
    floating_frame_show(float_handle);
    host.removeFromSuperview();
    if let Some(content) = FLOATING_CONTENT.with(|m| m.borrow().get(&float_handle).cloned()) {
        content.addSubview(&host);
        let size = content.frame().size;
        host.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(size.width, size.height),
        ));
        host.setAutoresizingMask(
            objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
        );
    }
    let _ = host;
    PANE_FLOATS.with(|m| {
        m.borrow_mut()
            .insert((frame_handle, pane_name.to_string()), float_handle);
    });
    install_float_close_handler(float_handle, frame_handle, pane_name);
}

fn install_float_close_handler(
    float_handle: NativeHandle,
    frame_handle: NativeHandle,
    pane_name: &str,
) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let pane_name = pane_name.to_string();
    let delegate = FloatPaneDelegate::new(mtm, float_handle, frame_handle, pane_name);
    FLOATING_WINDOWS.with(|m| {
        if let Some(win) = m.borrow().get(&float_handle) {
            win.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        }
    });
    FLOAT_PANE_DELEGATES.with(|d| d.borrow_mut().push(delegate));
}

thread_local! {
    static FLOAT_PANE_DELEGATES: RefCell<Vec<Retained<FloatPaneDelegate>>> = RefCell::new(Vec::new());
}

#[derive(Default)]
struct FloatPaneDelegateIvars {
    float_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    pane_name: RefCell<String>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "RuWxAuiFloatPaneDelegate"]
    #[ivars = FloatPaneDelegateIvars]
    struct FloatPaneDelegate;

    unsafe impl NSObjectProtocol for FloatPaneDelegate {}

    unsafe impl NSWindowDelegate for FloatPaneDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &objc2_foundation::NSNotification) {
            let frame_handle = self.ivars().frame_handle.get();
            let pane_name = self.ivars().pane_name.borrow().clone();
            PANE_ACTION.with(|m| {
                if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
                    unsafe {
                        if let Some(mgr) = mgr.as_mut() {
                            mgr.dock_pane(&frame, &pane_name);
                        }
                    }
                }
            });
        }
    }
);

impl FloatPaneDelegate {
    fn new(
        mtm: MainThreadMarker,
        float_handle: NativeHandle,
        frame_handle: NativeHandle,
        pane_name: String,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FloatPaneDelegateIvars {
            float_handle: Cell::new(float_handle),
            frame_handle: Cell::new(frame_handle),
            pane_name: RefCell::new(pane_name),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn dock_aui_pane_host(frame_handle: NativeHandle, pane_name: &str) {
    let float_handle = PANE_FLOATS.with(|m| {
        m.borrow_mut()
            .remove(&(frame_handle, pane_name.to_string()))
    });
    let host = PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .map(|s| s.host.clone())
    });
    let parent_handle = PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .map(|s| s.parent_handle)
    });
    let Some(host) = host else {
        return;
    };
    let Some(parent_handle) = parent_handle else {
        return;
    };
    if let Some(float_handle) = float_handle {
        floating_frame_close(float_handle);
    }
    host.removeFromSuperview();
    if let Some(parent) = parent_view_for(parent_handle) {
        parent.addSubview(&host);
    }
}

fn layout_host_internals(state: &PaneHostState, caption_h: f64) {
    let total_w = state.host.bounds().size.width;
    let total_h = state.host.bounds().size.height;
    let cap = match state.dock_side {
        AuiDockSide::Left | AuiDockSide::Right => aui_side_caption_width(caption_h as u32) as f64,
        _ => caption_h.max(8.0),
    };

    match state.dock_side {
        AuiDockSide::Top | AuiDockSide::Bottom | AuiDockSide::Floating => {
            state.caption.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(total_w, cap),
            ));
            state.content.setFrame(NSRect::new(
                NSPoint::new(0.0, cap),
                NSSize::new(total_w, (total_h - cap).max(0.0)),
            ));
            layout_caption_controls(state, cap, total_w, true);
        }
        AuiDockSide::Left => {
            state.caption.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(cap, total_h),
            ));
            state.content.setFrame(NSRect::new(
                NSPoint::new(cap, 0.0),
                NSSize::new((total_w - cap).max(0.0), total_h),
            ));
            layout_caption_controls(state, cap, total_h, false);
        }
        AuiDockSide::Right => {
            state.content.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new((total_w - cap).max(0.0), total_h),
            ));
            state.caption.setFrame(NSRect::new(
                NSPoint::new((total_w - cap).max(0.0), 0.0),
                NSSize::new(cap, total_h),
            ));
            layout_caption_controls(state, cap, total_h, false);
        }
    }
}

fn layout_caption_controls(state: &PaneHostState, cap: f64, span: f64, horizontal: bool) {
    let btn_count = state.buttons.len() as f64;
    let btn_total = btn_count * PANE_BTN_WIDTH;
    if horizontal {
        state.title_field.setFrame(NSRect::new(
            NSPoint::new(PANE_CAPTION_PAD, 1.0),
            NSSize::new(
                (span - btn_total - PANE_CAPTION_PAD * 2.0).max(20.0),
                cap - 2.0,
            ),
        ));
        let mut x = span - btn_total;
        for (_, btn, _) in &state.buttons {
            btn.setFrame(NSRect::new(
                NSPoint::new(x, 1.0),
                NSSize::new(PANE_BTN_WIDTH, cap - 2.0),
            ));
            x += PANE_BTN_WIDTH;
        }
    } else {
        state.title_field.setFrame(NSRect::new(
            NSPoint::new(1.0, PANE_CAPTION_PAD),
            NSSize::new(
                cap - 2.0,
                (span - btn_total - PANE_CAPTION_PAD * 2.0).max(20.0),
            ),
        ));
        let mut y = span - btn_total;
        for (_, btn, _) in &state.buttons {
            btn.setFrame(NSRect::new(
                NSPoint::new(1.0, y),
                NSSize::new(cap - 2.0, PANE_BTN_WIDTH),
            ));
            y += PANE_BTN_WIDTH;
        }
    }
}

fn rebuild_caption_buttons(
    mtm: MainThreadMarker,
    state: &mut PaneHostState,
    frame_handle: NativeHandle,
    pane_name: &str,
    buttons: &[AuiPaneButton],
    pinned: bool,
) {
    for (_, btn, _) in state.buttons.drain(..) {
        btn.removeFromSuperview();
    }
    PANE_BTN_COMMANDS.with(|map| {
        map.borrow_mut()
            .retain(|_, (fh, name, _)| !(*fh == frame_handle && name == pane_name));
    });

    let mut x_slot = 0.0;
    for button in buttons {
        let cmd = next_control_id();
        PANE_BTN_COMMANDS.with(|map| {
            map.borrow_mut()
                .insert(cmd, (frame_handle, pane_name.to_string(), *button));
        });
        let target = button_target_new(mtm, frame_handle, cmd);
        MENU_TARGETS.with(|m| m.borrow_mut().push(target.clone()));
        let btn = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(button_glyph(*button, pinned)),
                Some(&target),
                Some(sel!(ruWxButtonClicked:)),
                mtm,
            )
        };
        btn.setBezelStyle(NSBezelStyle::ShadowlessSquare);
        btn.setBordered(false);
        btn.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        btn.setFrame(NSRect::new(
            NSPoint::new(x_slot, 0.0),
            NSSize::new(PANE_BTN_WIDTH, PANE_BTN_WIDTH),
        ));
        state.caption.addSubview(&btn);
        state.buttons.push((cmd, btn, *button));
        x_slot += PANE_BTN_WIDTH;
    }
}

/// Ensure a pane host exists and wraps `child_handle`. Returns the host handle.
pub fn ensure_aui_pane_host(
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    pane_name: &str,
    child_handle: NativeHandle,
    dock_side: AuiDockSide,
    caption: &str,
    buttons: &[AuiPaneButton],
    art: &AuiDockArt,
    floatable: bool,
    pinned: bool,
) -> NativeHandle {
    ensure_pane_host(
        frame_handle,
        parent_handle,
        pane_name,
        child_handle,
        dock_side,
        caption,
        buttons,
        art,
        floatable,
        pinned,
    )
}

pub fn ensure_pane_host(
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    pane_name: &str,
    child_handle: NativeHandle,
    dock_side: AuiDockSide,
    caption: &str,
    buttons: &[AuiPaneButton],
    art: &AuiDockArt,
    floatable: bool,
    pinned: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let key = (frame_handle, pane_name.to_string());

    if PANE_HOSTS.with(|m| m.borrow().contains_key(&key)) {
        PANE_HOSTS.with(|m| {
            let mut map = m.borrow_mut();
            let state = map.get_mut(&key).expect("pane host");
            state.dock_side = dock_side;
            state.floatable = floatable;
            state.pinned = pinned;
            state.parent_handle = parent_handle;
            state
                .title_field
                .setStringValue(&NSString::from_str(caption));
            state.caption.apply_art(art);
            state
                .caption
                .set_drag_policy(frame_handle, pane_name, floatable, pinned);
            if state.buttons.len() != buttons.len()
                || state
                    .buttons
                    .iter()
                    .zip(buttons.iter())
                    .any(|((_, _, a), b)| a != b)
            {
                rebuild_caption_buttons(mtm, state, frame_handle, pane_name, buttons, pinned);
            } else {
                for (_, btn, kind) in &state.buttons {
                    if *kind == AuiPaneButton::Pin {
                        btn.setTitle(&NSString::from_str(button_glyph(
                            AuiPaneButton::Pin,
                            pinned,
                        )));
                    }
                }
            }
        });
        return PANE_HOSTS.with(|m| m.borrow()[&key].host_handle);
    }

    let host_handle = alloc_widget_handle();
    let host = FlippedView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 100.0)),
    );
    host.setClipsToBounds(true);

    let caption_view = AuiCaptionStripView::new(
        mtm,
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(100.0, art.caption_height() as f64),
        ),
    );
    caption_view.apply_art(art);
    caption_view.set_drag_policy(frame_handle, pane_name, floatable, pinned);

    let title = unsafe {
        NSTextField::initWithFrame(
            NSTextField::alloc(mtm),
            NSRect::new(NSPoint::new(4.0, 2.0), NSSize::new(80.0, 16.0)),
        )
    };
    title.setEditable(false);
    title.setBezeled(false);
    title.setDrawsBackground(false);
    title.setStringValue(&NSString::from_str(caption));
    title.setFont(Some(&NSFont::boldSystemFontOfSize(11.0)));
    caption_view.addSubview(&title);

    let content = FlippedView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, 80.0)),
    );
    content.setClipsToBounds(true);

    host.addSubview(&caption_view);
    host.addSubview(&content);

    if let Some(parent) = parent_view_for(parent_handle) {
        parent.addSubview(&host);
    }

    if let Some(child) = child_view_for_handle(child_handle) {
        child.removeFromSuperview();
        content.addSubview(&child);
        child.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(100.0, 80.0),
        ));
        child.setAutoresizingMask(
            objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        record_parent(child_handle, host_handle);
    }

    let mut state = PaneHostState {
        host_handle,
        host: host.clone(),
        caption: caption_view,
        title_field: title,
        content,
        child_handle,
        frame_handle,
        parent_handle,
        dock_side,
        floatable,
        pinned,
        buttons: Vec::new(),
    };
    rebuild_caption_buttons(mtm, &mut state, frame_handle, pane_name, buttons, pinned);

    store_container(host_handle, host.clone().into_super());
    record_parent(host_handle, parent_handle);

    PANE_HOSTS.with(|m| m.borrow_mut().insert(key, state));
    host_handle
}

pub fn layout_pane_host(host_handle: NativeHandle, caption_height: u32) {
    PANE_HOSTS.with(|m| {
        let mut map = m.borrow_mut();
        let Some(state) = map.values_mut().find(|s| s.host_handle == host_handle) else {
            return;
        };
        layout_host_internals(state, caption_height as f64);
        let child_handle = state.child_handle;
        let content = state.content.clone();
        drop(map);
        if let Some(child) = child_view_for_handle(child_handle) {
            let size = content.bounds().size;
            child.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(size.width, size.height),
            ));
        }
    });
}

pub fn pane_host_handle(frame_handle: NativeHandle, pane_name: &str) -> Option<NativeHandle> {
    PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .map(|s| s.host_handle)
    })
}

pub fn pane_button_commands_for_frame(
    frame_handle: NativeHandle,
) -> Vec<(u16, String, AuiPaneButton)> {
    PANE_BTN_COMMANDS.with(|map| {
        map.borrow()
            .iter()
            .filter(|(_, (fh, _, _))| *fh == frame_handle)
            .map(|(id, (_, name, btn))| (*id, name.clone(), *btn))
            .collect()
    })
}

pub fn lookup_pane_button_command(
    command_id: u16,
) -> Option<(NativeHandle, String, AuiPaneButton)> {
    PANE_BTN_COMMANDS.with(|map| map.borrow().get(&command_id).cloned())
}

/// Position a pane host in the frame and lay out caption + content.
pub fn position_pane_host(
    parent_handle: NativeHandle,
    host_handle: NativeHandle,
    rect: Rect,
    caption_height: u32,
) {
    set_view_frame(parent_handle, host_handle, rect);
    layout_pane_host(host_handle, caption_height);
}

// ── Floating frames (existing API) ─────────────────────────────────

/// Convert a Win32-style screen rectangle (top-left origin) to an AppKit
/// window frame (bottom-left origin).
pub fn win32_screen_rect_to_ns(mtm: MainThreadMarker, rect: &Rect) -> NSRect {
    let screen_h = NSScreen::mainScreen(mtm)
        .map(|s| s.frame().size.height)
        .unwrap_or(900.0);
    NSRect::new(
        NSPoint::new(rect.x as f64, screen_h - rect.y as f64 - rect.height as f64),
        NSSize::new(rect.width.max(40) as f64, rect.height.max(24) as f64),
    )
}

/// Win32-style screen coordinates for the current mouse position, with the
/// same −20/−10 px offset used by the Win32 AuiToolBar float path.
pub fn cursor_win32_screen_point(mtm: MainThreadMarker) -> (i32, i32) {
    use objc2_app_kit::NSEvent;
    let pt = NSEvent::mouseLocation();
    let screen_h = NSScreen::mainScreen(mtm)
        .map(|s| s.frame().size.height)
        .unwrap_or(900.0);
    ((pt.x - 20.0) as i32, (screen_h - pt.y - 10.0) as i32)
}

pub fn create_floating_frame(title: &str, rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;

    let ns_rect = win32_screen_rect_to_ns(mtm, &rect);
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            ns_rect,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str(title));

    let content = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    content.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    window.setContentView(Some(&content));

    FLOATING_WINDOWS.with(|m| m.borrow_mut().insert(handle, window));
    FLOATING_CONTENT.with(|m| m.borrow_mut().insert(handle, content.clone()));
    store_container(handle, content);
    handle
}

pub fn floating_frame_show(handle: NativeHandle) {
    FLOATING_WINDOWS.with(|m| {
        if let Some(win) = m.borrow().get(&handle) {
            win.makeKeyAndOrderFront(None);
        }
    });
}

pub fn floating_frame_close(handle: NativeHandle) {
    FLOATING_WINDOWS.with(|m| {
        if let Some(win) = m.borrow_mut().remove(&handle) {
            win.close();
        }
    });
    FLOATING_CONTENT.with(|m| {
        m.borrow_mut().remove(&handle);
    });
}

/// Borderless top-level popup (tool-window style).
pub fn create_borderless_popup(rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let ns_rect = win32_screen_rect_to_ns(mtm, &rect);
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            ns_rect,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setLevel(objc2_app_kit::NSFloatingWindowLevel);
    let content = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    window.setContentView(Some(&content));
    FLOATING_WINDOWS.with(|m| m.borrow_mut().insert(handle, window));
    FLOATING_CONTENT.with(|m| m.borrow_mut().insert(handle, content.clone()));
    store_container(handle, content);
    floating_frame_show(handle);
    handle
}

pub fn popup_close(handle: NativeHandle) {
    floating_frame_close(handle);
}

pub fn popup_set_opacity(handle: NativeHandle, opacity: u8) {
    let alpha = (opacity as f64 / 255.0).clamp(0.0, 1.0);
    FLOATING_WINDOWS.with(|m| {
        if let Some(win) = m.borrow().get(&handle) {
            win.setAlphaValue(alpha);
        }
    });
}

pub fn floating_frame_host_child(frame_handle: NativeHandle, child_handle: NativeHandle) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let Some(content) = FLOATING_CONTENT.with(|m| m.borrow().get(&frame_handle).cloned()) else {
        return;
    };
    let child = child_view_for_handle(child_handle);
    let Some(child) = child else {
        return;
    };
    child.removeFromSuperview();
    content.addSubview(&child);
    let size = content.frame().size;
    child.setFrame(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(size.width, size.height),
    ));
    child.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    record_parent(child_handle, frame_handle);
    let _ = mtm;
}

pub fn sync_aui_pane_auto_hide(
    frame_handle: NativeHandle,
    pane_name: &str,
    auto_hide: bool,
    auto_hidden: bool,
) {
    PANE_HOSTS.with(|m| {
        if let Some(state) = m.borrow().get(&(frame_handle, pane_name.to_string())) {
            state.caption.set_auto_hide_state(auto_hide, auto_hidden);
        }
    });
}

thread_local! {
    static SASH_VIEWS: RefCell<HashMap<(NativeHandle, String), Retained<AuiSashView>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct AuiSashIvars {
    frame_handle: Cell<NativeHandle>,
    pane_name: RefCell<String>,
    vertical: Cell<bool>,
    sash_color: Cell<u32>,
    drag_origin: Cell<f64>,
    drag_last: Cell<f64>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AuiSashIvars]
    struct AuiSashView;

    unsafe impl NSObjectProtocol for AuiSashView {}

    impl AuiSashView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            dock_color(self.ivars().sash_color.get()).setFill();
            NSBezierPath::fillRect(self.bounds());
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let loc = if self.ivars().vertical.get() {
                self.convertPoint_fromView(event.locationInWindow(), None).x
            } else {
                self.convertPoint_fromView(event.locationInWindow(), None).y
            };
            self.ivars().drag_origin.set(loc);
            self.ivars().drag_last.set(loc);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let loc = if self.ivars().vertical.get() {
                self.convertPoint_fromView(event.locationInWindow(), None).x
            } else {
                self.convertPoint_fromView(event.locationInWindow(), None).y
            };
            let last = self.ivars().drag_last.get();
            let delta = (loc - last).round() as i32;
            if delta != 0 {
                request_pane_sash_resize(
                    self.ivars().frame_handle.get(),
                    &self.ivars().pane_name.borrow(),
                    delta,
                    self.ivars().vertical.get(),
                );
                self.ivars().drag_last.set(loc);
            }
        }

        #[unsafe(method(resetCursorRects))]
        fn reset_cursor_rects(&self) {
            let cursor = if self.ivars().vertical.get() {
                unsafe { objc2_app_kit::NSCursor::resizeLeftRightCursor() }
            } else {
                unsafe { objc2_app_kit::NSCursor::resizeUpDownCursor() }
            };
            unsafe {
                let _: () = msg_send![self, addCursorRect: self.bounds(), cursor: &*cursor];
            }
        }
    }
);

impl AuiSashView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AuiSashIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    fn configure(
        &self,
        frame_handle: NativeHandle,
        pane_name: &str,
        vertical: bool,
        sash_color: u32,
    ) {
        self.ivars().frame_handle.set(frame_handle);
        *self.ivars().pane_name.borrow_mut() = pane_name.to_string();
        self.ivars().vertical.set(vertical);
        self.ivars().sash_color.set(sash_color);
        self.resetCursorRects();
        self.setNeedsDisplay(true);
    }
}

pub fn sync_aui_sashes(parent: NativeHandle, frame_handle: NativeHandle, specs: &[AuiSashSpec]) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let Some(parent_view) = parent_view_for(parent) else {
        return;
    };
    let wanted: std::collections::HashSet<(NativeHandle, String)> = specs
        .iter()
        .map(|s| (frame_handle, s.pane_name.clone()))
        .collect();

    SASH_VIEWS.with(|m| {
        let mut map = m.borrow_mut();
        map.retain(|key, view| {
            if wanted.contains(key) {
                true
            } else {
                view.removeFromSuperview();
                false
            }
        });
        for spec in specs {
            let key = (frame_handle, spec.pane_name.clone());
            let ns_rect = rect_to_ns_in_parent(parent, &spec.rect);
            if let Some(view) = map.get(&key) {
                view.setFrame(ns_rect);
                view.configure(
                    frame_handle,
                    &spec.pane_name,
                    spec.vertical,
                    spec.sash_color,
                );
            } else {
                let view = AuiSashView::new(mtm, ns_rect);
                view.configure(
                    frame_handle,
                    &spec.pane_name,
                    spec.vertical,
                    spec.sash_color,
                );
                parent_view.addSubview(&view);
                map.insert(key, view);
            }
        }
    });
}

/// Backing scale factor (1.0 = 96 DPI, 2.0 = Retina) for a view or frame handle.
pub fn view_backing_scale_factor(handle: NativeHandle) -> f32 {
    if let Some(view) = child_view_for_handle(handle) {
        if let Some(window) = view.window() {
            return window.backingScaleFactor() as f32;
        }
    }
    FLOATING_CONTENT.with(|m| {
        if let Some(content) = m.borrow().get(&handle) {
            if let Some(window) = content.window() {
                return window.backingScaleFactor() as f32;
            }
        }
        1.0
    })
}
