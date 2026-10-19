//! macOS property grid — two-column sheet with inline value editing.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBezelStyle, NSBorderType, NSButton, NSControlStateValueOff, NSControlStateValueOn, NSEvent,
    NSPopUpButton, NSScrollView, NSTextField, NSView,
};

use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use super::{
    alloc_widget_handle, parent_view_for, record_parent, ru_rect_to_ns, store_ns_view,
    NativeHandle, Rect,
};

const PG_ROW_HEIGHT: f64 = 22.0;
const PG_COL_SEP: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyGridValueKind {
    String,
    Int,
    Float,
    Bool,
    Choice,
}

/// Row accessor: takes the row index and returns the displayed value.
pub type PropertyGridTextFn = Box<dyn Fn(usize) -> String>;
/// Row mutator: applies the edited value to row `usize`, returning success.
pub type PropertyGridApplyFn = Box<dyn FnMut(usize, &str) -> bool>;

/// Rust-side hooks registered by [`crate::adv::property_grid::PropertyGrid`].
pub struct PropertyGridCallbacks {
    pub row_count: Box<dyn Fn() -> usize>,
    pub name_at: PropertyGridTextFn,
    pub value_display_at: PropertyGridTextFn,
    pub kind_at: Box<dyn Fn(usize) -> PropertyGridValueKind>,
    pub choice_options_at: Box<dyn Fn(usize) -> Vec<String>>,
    pub name_col_width: Box<dyn Fn() -> f64>,
    pub apply_text: PropertyGridApplyFn,
    pub apply_bool: Box<dyn FnMut(usize, bool) -> bool>,
    pub apply_choice: Box<dyn FnMut(usize, usize) -> bool>,
    pub on_changed: Box<dyn FnMut(usize)>,
}

struct PropertyGridState {
    callbacks: Rc<RefCell<PropertyGridCallbacks>>,
    name_col_w: Cell<f64>,
    grid_view: Retained<PropertyGridView>,
}

thread_local! {
    static PROPERTY_GRIDS: RefCell<HashMap<NativeHandle, Rc<RefCell<PropertyGridState>>>> =
        RefCell::new(HashMap::new());
    static PROPERTY_GRID_VIEWS: RefCell<HashMap<NativeHandle, Retained<PropertyGridView>>> =
        RefCell::new(HashMap::new());
    static PROPERTY_GRID_EDITORS: RefCell<
        HashMap<NativeHandle, (PropertyGridEditor, usize)>,
    > = RefCell::new(HashMap::new());
}

enum PropertyGridEditor {
    Text(Retained<NSTextField>),
    Bool(Retained<NSButton>),
    Choice(Retained<NSPopUpButton>),
}

#[derive(Default)]
struct PropertyGridViewIvars {
    grid_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = PropertyGridViewIvars]
    struct PropertyGridView;

    unsafe impl NSObjectProtocol for PropertyGridView {}

    impl PropertyGridView {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().grid_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let Some((row, is_value)) = property_grid_hit_test(handle, point.x, point.y) else {
                property_grid_end_editor(handle, false);
                return;
            };
            if is_value {
                property_grid_start_editor(handle, row);
            } else {
                property_grid_end_editor(handle, true);
            }
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let handle = self.ivars().grid_handle.get();
            if PROPERTY_GRID_EDITORS.with(|m| m.borrow().contains_key(&handle)) {
                let key = event.keyCode();
                if key == 36 || key == 76 {
                    property_grid_end_editor(handle, true);
                    return;
                }
                if key == 53 {
                    property_grid_end_editor(handle, false);
                    return;
                }
            }
            unsafe { msg_send![super(self), keyDown: event] }
        }
    }
);

impl PropertyGridView {
    fn new(mtm: MainThreadMarker, frame: NSRect, grid_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(PropertyGridViewIvars {
            grid_handle: Cell::new(grid_handle),
        });
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

#[derive(Default)]
struct PropertyGridActionTargetIvars {
    grid_handle: Cell<NativeHandle>,
    row: Cell<usize>,
    kind: Cell<u8>,
}

define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = PropertyGridActionTargetIvars]
    struct PropertyGridActionTarget;

    unsafe impl NSObjectProtocol for PropertyGridActionTarget {}

    impl PropertyGridActionTarget {
        #[unsafe(method(ruWxPropertyGridBool:))]
        fn bool_clicked(&self, _sender: Option<&AnyObject>) {
            let handle = self.ivars().grid_handle.get();
            let row = self.ivars().row.get();
            property_grid_end_editor(handle, true);
            let _ = row;
        }

        #[unsafe(method(ruWxPropertyGridChoice:))]
        fn choice_changed(&self, _sender: Option<&AnyObject>) {
            let handle = self.ivars().grid_handle.get();
            property_grid_end_editor(handle, true);
        }
    }
);

impl PropertyGridActionTarget {
    fn new(
        mtm: MainThreadMarker,
        grid_handle: NativeHandle,
        row: usize,
        kind: u8,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(PropertyGridActionTargetIvars {
            grid_handle: Cell::new(grid_handle),
            row: Cell::new(row),
            kind: Cell::new(kind),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn create_property_grid(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);

    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);

    let grid_view = PropertyGridView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
        handle,
    );
    scroll.setDocumentView(Some(&grid_view));

    PROPERTY_GRID_VIEWS.with(|m| m.borrow_mut().insert(handle, grid_view.clone()));
    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn property_grid_set_callbacks(handle: NativeHandle, callbacks: PropertyGridCallbacks) {
    let view = PROPERTY_GRID_VIEWS.with(|m| m.borrow().get(&handle).cloned());
    let Some(view) = view else {
        return;
    };
    PROPERTY_GRIDS.with(|m| {
        m.borrow_mut().insert(
            handle,
            Rc::new(RefCell::new(PropertyGridState {
                callbacks: Rc::new(RefCell::new(callbacks)),
                name_col_w: Cell::new(0.0),
                grid_view: view,
            })),
        );
    });
}

pub fn property_grid_set_name_col_w(handle: NativeHandle, width: f64) {
    if let Some(state) = PROPERTY_GRIDS.with(|m| m.borrow().get(&handle).cloned()) {
        state.borrow().name_col_w.set(width.max(40.0));
    }
    property_grid_redraw(handle);
}

pub fn property_grid_redraw(handle: NativeHandle) {
    property_grid_end_editor(handle, true);
    property_grid_redraw_inner(handle);
}

fn property_grid_redraw_inner(handle: NativeHandle) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };

    let state = PROPERTY_GRIDS.with(|m| m.borrow().get(&handle).cloned());
    let Some(state) = state else {
        return;
    };

    let view = state.borrow().grid_view.clone();
    while let Some(sub) = view.subviews().firstObject() {
        sub.removeFromSuperview();
    }

    let width = view.frame().size.width;
    let callbacks = state.borrow().callbacks.clone();
    let name_col_w = {
        let w = state.borrow().name_col_w.get();
        if w > 0.0 {
            w
        } else {
            let pct = (callbacks.borrow().name_col_width)();
            if pct > 0.0 {
                pct
            } else {
                width * 0.45
            }
        }
    };
    state.borrow().name_col_w.set(name_col_w);

    let row_count = (callbacks.borrow().row_count)();
    let total_h = row_count as f64 * PG_ROW_HEIGHT;
    view.setFrameSize(NSSize::new(width, total_h.max(PG_ROW_HEIGHT)));

    let sep = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    sep.setFrame(NSRect::new(
        NSPoint::new(name_col_w, 0.0),
        NSSize::new(PG_COL_SEP, total_h.max(PG_ROW_HEIGHT)),
    ));
    sep.setDrawsBackground(true);
    sep.setBackgroundColor(Some(&objc2_app_kit::NSColor::separatorColor()));
    view.addSubview(&sep);

    for row in 0..row_count {
        let row_y = total_h - (row as f64 + 1.0) * PG_ROW_HEIGHT;
        let name = (callbacks.borrow().name_at)(row);
        let value = (callbacks.borrow().value_display_at)(row);

        let name_label = NSTextField::labelWithString(&NSString::from_str(&name), mtm);
        name_label.setFrame(NSRect::new(
            NSPoint::new(4.0, row_y),
            NSSize::new((name_col_w - 6.0).max(20.0), PG_ROW_HEIGHT),
        ));
        view.addSubview(&name_label);

        let value_label = NSTextField::labelWithString(&NSString::from_str(&value), mtm);
        value_label.setFrame(NSRect::new(
            NSPoint::new(name_col_w + PG_COL_SEP + 4.0, row_y),
            NSSize::new(
                (width - name_col_w - PG_COL_SEP - 8.0).max(20.0),
                PG_ROW_HEIGHT,
            ),
        ));
        view.addSubview(&value_label);

        if row + 1 < row_count {
            let line = NSTextField::labelWithString(&NSString::from_str(""), mtm);
            line.setFrame(NSRect::new(
                NSPoint::new(0.0, row_y),
                NSSize::new(width, 1.0),
            ));
            line.setDrawsBackground(true);
            line.setBackgroundColor(Some(&objc2_app_kit::NSColor::separatorColor()));
            view.addSubview(&line);
        }
    }
}

pub fn property_grid_close_editor(handle: NativeHandle) {
    property_grid_end_editor(handle, false);
}

fn property_grid_name_col_w(handle: NativeHandle) -> f64 {
    PROPERTY_GRIDS.with(|m| {
        m.borrow()
            .get(&handle)
            .map(|s| s.borrow().name_col_w.get())
            .unwrap_or(120.0)
    })
}

fn property_grid_hit_test(handle: NativeHandle, x: f64, y: f64) -> Option<(usize, bool)> {
    let callbacks = PROPERTY_GRIDS.with(|m| m.borrow().get(&handle).cloned())?;
    let row_count = (callbacks.borrow().callbacks.borrow().row_count)();
    if row_count == 0 {
        return None;
    }
    let view = callbacks.borrow().grid_view.clone();
    let total_h = view.frame().size.height;
    let local_y = total_h - y;
    if local_y < 0.0 {
        return None;
    }
    let row = (local_y / PG_ROW_HEIGHT).floor() as usize;
    if row >= row_count {
        return None;
    }
    let name_col_w = property_grid_name_col_w(handle);
    Some((row, x > name_col_w + PG_COL_SEP))
}

fn property_grid_value_cell_rect(handle: NativeHandle, row: usize) -> NSRect {
    let state = PROPERTY_GRIDS.with(|m| m.borrow().get(&handle).cloned());
    let Some(state) = state else {
        return NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, PG_ROW_HEIGHT));
    };
    let view = state.borrow().grid_view.clone();
    let width = view.frame().size.width;
    let total_h = view.frame().size.height;
    let row_y = total_h - (row as f64 + 1.0) * PG_ROW_HEIGHT + 1.0;
    let name_col_w = property_grid_name_col_w(handle);
    NSRect::new(
        NSPoint::new(name_col_w + PG_COL_SEP + 2.0, row_y),
        NSSize::new(
            (width - name_col_w - PG_COL_SEP - 4.0).max(40.0),
            PG_ROW_HEIGHT - 2.0,
        ),
    )
}

fn property_grid_start_editor(handle: NativeHandle, row: usize) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    property_grid_end_editor(handle, true);

    let state = PROPERTY_GRIDS.with(|m| m.borrow().get(&handle).cloned());
    let Some(state) = state else {
        return;
    };
    let callbacks = state.borrow().callbacks.clone();
    let kind = (callbacks.borrow().kind_at)(row);
    let rect = property_grid_value_cell_rect(handle, row);
    let view = state.borrow().grid_view.clone();

    match kind {
        PropertyGridValueKind::String
        | PropertyGridValueKind::Int
        | PropertyGridValueKind::Float => {
            let initial = (callbacks.borrow().value_display_at)(row);
            let field = NSTextField::initWithFrame(NSTextField::alloc(mtm), rect);
            field.setStringValue(&NSString::from_str(&initial));
            field.setEditable(true);
            field.setBordered(true);
            field.setBezeled(true);
            field.setDrawsBackground(true);
            view.addSubview(&field);
            field.becomeFirstResponder();
            PROPERTY_GRID_EDITORS.with(|m| {
                m.borrow_mut()
                    .insert(handle, (PropertyGridEditor::Text(field.clone()), row));
            });
        }
        PropertyGridValueKind::Bool => {
            let display = (callbacks.borrow().value_display_at)(row);
            let checked = display.eq_ignore_ascii_case("true");
            let btn = NSButton::initWithFrame(NSButton::alloc(mtm), rect);
            btn.setTitle(&NSString::from_str(if checked {
                "☑ true"
            } else {
                "☐ false"
            }));
            btn.setBezelStyle(NSBezelStyle::SmallSquare);
            btn.setButtonType(objc2_app_kit::NSButtonType::Switch);
            btn.setState(if checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            let target = PropertyGridActionTarget::new(mtm, handle, row, 1);
            unsafe {
                btn.setTarget(Some(&target));
                btn.setAction(Some(sel!(ruWxPropertyGridBool:)));
            }
            view.addSubview(&btn);
            btn.becomeFirstResponder();
            PROPERTY_GRID_EDITORS.with(|m| {
                m.borrow_mut()
                    .insert(handle, (PropertyGridEditor::Bool(btn.clone()), row));
            });
        }
        PropertyGridValueKind::Choice => {
            let options = (callbacks.borrow().choice_options_at)(row);
            let popup =
                NSPopUpButton::initWithFrame_pullsDown(NSPopUpButton::alloc(mtm), rect, false);
            for opt in &options {
                popup.addItemWithTitle(&NSString::from_str(opt));
            }
            let display = (callbacks.borrow().value_display_at)(row);
            popup.selectItemWithTitle(&NSString::from_str(&display));
            let target = PropertyGridActionTarget::new(mtm, handle, row, 2);
            unsafe {
                popup.setTarget(Some(&target));
                popup.setAction(Some(sel!(ruWxPropertyGridChoice:)));
            }
            view.addSubview(&popup);
            popup.becomeFirstResponder();
            PROPERTY_GRID_EDITORS.with(|m| {
                m.borrow_mut()
                    .insert(handle, (PropertyGridEditor::Choice(popup.clone()), row));
            });
        }
    }
}

pub fn property_grid_end_editor(handle: NativeHandle, accept: bool) {
    let editor = PROPERTY_GRID_EDITORS.with(|m| m.borrow_mut().remove(&handle));
    let Some((editor, row)) = editor else {
        return;
    };

    let state = PROPERTY_GRIDS.with(|m| m.borrow().get(&handle).cloned());
    let Some(state) = state else {
        return;
    };
    let callbacks = state.borrow().callbacks.clone();

    let changed = if accept {
        match editor {
            PropertyGridEditor::Text(field) => {
                let text = field.stringValue().to_string();
                field.removeFromSuperview();
                let ok = (callbacks.borrow_mut().apply_text)(row, &text);
                if ok {
                    (callbacks.borrow_mut().on_changed)(row);
                }
                ok
            }
            PropertyGridEditor::Bool(btn) => {
                let new_val = btn.state() == NSControlStateValueOn;
                btn.removeFromSuperview();
                let ok = (callbacks.borrow_mut().apply_bool)(row, new_val);
                if ok {
                    (callbacks.borrow_mut().on_changed)(row);
                }
                ok
            }
            PropertyGridEditor::Choice(popup) => {
                let sel = popup.indexOfSelectedItem().max(0) as usize;
                popup.removeFromSuperview();
                let ok = (callbacks.borrow_mut().apply_choice)(row, sel);
                if ok {
                    (callbacks.borrow_mut().on_changed)(row);
                }
                ok
            }
        }
    } else {
        match editor {
            PropertyGridEditor::Text(f) => f.removeFromSuperview(),
            PropertyGridEditor::Bool(b) => b.removeFromSuperview(),
            PropertyGridEditor::Choice(p) => p.removeFromSuperview(),
        }
        false
    };

    if changed || !accept {
        property_grid_redraw_inner(handle);
    }
}
