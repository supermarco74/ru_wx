//! Wrapper per NSTrackingArea su macOS.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSView, NSObjectProtocol, NSRect};
use objc2_foundation::{MainThreadMarker, NSPoint};

use crate::core::system_events::WidgetId;

/// Eventi hover generati dal tracker.
#[derive(Debug, Clone)]
pub enum HoverEvent {
    Enter { x: f64, y: f64 },
    Exit,
    Move { x: f64, y: f64 },
}

/// Callback per gli eventi hover.
pub type HoverCallback = Box<dyn FnMut(HoverEvent) + 'static>;

// ── HoverTrackingView ──────────────────────────────────────────────────────────

thread_local! {
    static HOVER_CALLBACKS: RefCell<HashMap<usize, Rc<RefCell<Option<HoverCallback>>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct HoverViewIvars {
    callback_ptr: Option<usize>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = HoverViewIvars]
    #[name = "RuWxHoverTrackingView"]
    struct HoverView;

    unsafe impl NSObjectProtocol for HoverView {}

    impl HoverView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> Bool {
            Bool::YES
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            let frame = self.frame();
            eprintln!("[HOVER] Mouse entered view at ({:.1}, {:.1})", frame.origin.x, frame.origin.y);
            
            if let Some(ptr) = self.ivars().callback_ptr {
                HOVER_CALLBACKS.with(|m| {
                    if let Some(cb) = m.borrow_mut().get_mut(&ptr) {
                        if let Some(ref mut callback) = *cb {
                            callback(HoverEvent::Enter { x: frame.origin.x, y: frame.origin.y });
                        }
                    }
                });
            }
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            eprintln!("[HOVER] Mouse exited view");
            
            if let Some(ptr) = self.ivars().callback_ptr {
                HOVER_CALLBACKS.with(|m| {
                    if let Some(cb) = m.borrow_mut().get_mut(&ptr) {
                        if let Some(ref mut callback) = *cb {
                            callback(HoverEvent::Exit);
                        }
                    }
                });
            }
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            let location = unsafe { event.locationInWindow() };
            let frame = self.frame();
            let x = location.x - frame.origin.x;
            let y = location.y - frame.origin.y;
            
            if let Some(ptr) = self.ivars().callback_ptr {
                HOVER_CALLBACKS.with(|m| {
                    if let Some(cb) = m.borrow_mut().get_mut(&ptr) {
                        if let Some(ref mut callback) = *cb {
                            callback(HoverEvent::Move { x, y });
                        }
                    }
                });
            }
        }
    }
);

impl HoverView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HoverViewIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
    
    fn set_callback(&self, callback: Rc<RefCell<Option<HoverCallback>>) {
        let ptr = Rc::as_ptr(&callback) as usize;
        self.ivars().callback_ptr = Some(ptr);
        HOVER_CALLBACKS.with(|m| {
            m.borrow_mut().insert(ptr, callback);
        });
        // Abilitiamo il tracking
        self.enable_tracking();
    }
    
    fn enable_tracking(&self) {
        // Workaround: per ora usiamo il frame della view come tracking area
        // In pratca dovrebbe essere chiamato con una NSTrackingArea
        eprintln!("[HOVER] Tracking enabled");
    }
}

// ── HoverTracker ─────────────────────────────────────────────────────────────

/// Wrapper per abilitare hover tracking su una NSView.
pub struct HoverTracker {
    widget_handle: WidgetId,
    tracking_view: Retained<HoverView>,
    callback: Rc<RefCell<Option<HoverCallback>>,
}

impl HoverTracker {
    /// Crea un nuovo tracker per una vista.
    pub fn new<F>(view: &Retained<NSView>, widget_handle: WidgetId, callback: F) -> Self
    where
        F: FnMut(HoverEvent) + 'static,
    {
        let mtm = MainThreadMarker::new().expect("AppKit richiede il main thread");
        
        // Creiamo una view wrapper che gestisce gli eventi hover
        let tracking_view = HoverView::new(mtm, view.bounds());
        
        let callback = Rc::new(RefCell::new(Some(Box::new(callback)));
        tracking_view.set_callback(Rc::clone(&callback));
        
        // Aggiungiamo la tracking view come subview della vista originale
        unsafe {
            view.addSubview(&tracking_view);
        }
        
        Self {
            widget_handle,
            tracking_view,
            callback,
        }
    }
    
    /// Abilita il tracking.
    pub fn enable(&self) {
        self.tracking_view.enable_tracking();
    }
    
    /// Disabilita il tracking.
    pub fn disable(&self) {
        // Non implementato per ora
    }
}
