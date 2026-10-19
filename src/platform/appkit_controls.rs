//! macOS controls not covered by the core AppKit factories.

use std::cell::Cell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBezelStyle, NSBox, NSBoxType, NSButton, NSButtonType, NSDatePicker, NSDatePickerElementFlags,
    NSDatePickerStyle, NSSearchField, NSStepper, NSTextField, NSTitlePosition, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSDateFormatter, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use super::{
    alloc_widget_handle, button_target_new, dispatch_command, parent_view_for, record_parent,
    resolve_frame_handle, ru_rect_to_ns, store_ns_view, NativeHandle, Rect,
};

thread_local! {
    static DATE_PICKERS: std::cell::RefCell<HashMap<NativeHandle, Retained<NSDatePicker>>> =
        std::cell::RefCell::new(HashMap::new());
    static SPIN_BUTTONS: std::cell::RefCell<HashMap<NativeHandle, Retained<NSStepper>>> =
        std::cell::RefCell::new(HashMap::new());
}

#[derive(Default)]
struct DatePickerTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = DatePickerTargetIvars]
    struct DatePickerTarget;

    unsafe impl NSObjectProtocol for DatePickerTarget {}

    impl DatePickerTarget {
        #[unsafe(method(ruWxDatePickerChanged:))]
        fn date_changed(&self, _sender: Option<&AnyObject>) {
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }
);

impl DatePickerTarget {
    fn new(mtm: MainThreadMarker, frame_handle: NativeHandle, command_id: u16) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DatePickerTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

#[derive(Default)]
struct SpinButtonTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SpinButtonTargetIvars]
    struct SpinButtonTarget;

    unsafe impl NSObjectProtocol for SpinButtonTarget {}

    impl SpinButtonTarget {
        #[unsafe(method(ruWxSpinButtonChanged:))]
        fn spin_changed(&self, _sender: Option<&AnyObject>) {
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }
);

impl SpinButtonTarget {
    fn new(mtm: MainThreadMarker, frame_handle: NativeHandle, command_id: u16) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SpinButtonTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn date_formatter() -> Retained<NSDateFormatter> {
    let fmt = NSDateFormatter::new();
    fmt.setDateFormat(Some(&NSString::from_str("yyyy-MM-dd")));
    fmt
}

pub fn create_static_line(
    parent: NativeHandle,
    initial_rect: Rect,
    horizontal: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let mut rect = initial_rect;
    if horizontal && rect.height == 0 {
        rect.height = 2;
    }
    if !horizontal && rect.width == 0 {
        rect.width = 2;
    }
    let line = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    line.setDrawsBackground(true);
    line.setBackgroundColor(Some(&objc2_app_kit::NSColor::separatorColor()));
    line.setFrame(ru_rect_to_ns(content_h, &rect));
    content.addSubview(&line);
    store_ns_view(handle, line.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn create_group_box(parent: NativeHandle, title: &str, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let box_view = NSBox::initWithFrame(NSBox::alloc(mtm), ns_rect);
    box_view.setTitle(&NSString::from_str(title));
    box_view.setTitlePosition(NSTitlePosition::AtTop);
    box_view.setBoxType(NSBoxType::Primary);
    content.addSubview(&box_view);
    store_ns_view(handle, box_view.into_super());
    record_parent(handle, parent);
    handle
}

pub fn set_group_box_title(handle: NativeHandle, title: &str) {
    if let Some(view) = super::child_view_for_handle(handle) {
        if let Some(box_view) = view.downcast_ref::<NSBox>() {
            box_view.setTitle(&NSString::from_str(title));
        }
    }
}

pub fn create_toggle_button(
    parent: NativeHandle,
    title: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let frame_handle = resolve_frame_handle(parent);
    let target = button_target_new(mtm, frame_handle, command_id);
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(&target),
            Some(sel!(ruWxButtonClicked:)),
            mtm,
        )
    };
    button.setButtonType(NSButtonType::PushOnPushOff);
    button.setBezelStyle(NSBezelStyle::Push);
    button.setFrame(ru_rect_to_ns(content_h, &initial_rect));
    content.addSubview(&button);
    store_ns_view(handle, button.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn toggle_button_set_pressed(handle: NativeHandle, pressed: bool) {
    super::checkbox_set_checked(handle, pressed);
}

pub fn toggle_button_is_pressed(handle: NativeHandle) -> bool {
    super::checkbox_is_checked(handle)
}

pub fn create_date_picker(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let picker = NSDatePicker::initWithFrame(
        NSDatePicker::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    picker.setDatePickerStyle(NSDatePickerStyle::TextFieldAndStepper);
    picker.setDatePickerElements(
        NSDatePickerElementFlags::YearMonthDay | NSDatePickerElementFlags::Era,
    );
    let frame_handle = resolve_frame_handle(parent);
    let target = DatePickerTarget::new(mtm, frame_handle, command_id);
    unsafe {
        picker.setTarget(Some(&target));
        picker.setAction(Some(sel!(ruWxDatePickerChanged:)));
    }
    content.addSubview(&picker);
    DATE_PICKERS.with(|m| m.borrow_mut().insert(handle, picker.clone()));
    store_ns_view(handle, picker.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn date_picker_get_value(handle: NativeHandle) -> Option<(i32, u32, u32)> {
    DATE_PICKERS.with(|m| {
        m.borrow().get(&handle).and_then(|picker| {
            let fmt = date_formatter();
            let text = fmt.stringFromDate(&picker.dateValue()).to_string();
            parse_ymd(&text)
        })
    })
}

pub fn date_picker_set_value(handle: NativeHandle, year: i32, month: u32, day: u32) {
    DATE_PICKERS.with(|m| {
        if let Some(picker) = m.borrow().get(&handle) {
            let text = format!("{year:04}-{month:02}-{day:02}");
            let fmt = date_formatter();
            if let Some(date) = fmt.dateFromString(&NSString::from_str(&text)) {
                picker.setDateValue(&date);
            }
        }
    });
}

fn parse_ymd(text: &str) -> Option<(i32, u32, u32)> {
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

// ── TimePickerCtrl (C1) ────────────────────────────────────────────────
//
// Variante di `create_date_picker` con elementi ora:minuti:secondi.
// Il picker viene memorizzato nella stessa `DATE_PICKERS` (è lo stesso
// `NSDatePicker`, con target/azione identici) — cambia solo la lettura/
// scrittura del valore, che usa il formatter `HH:mm:ss`.

pub fn create_time_picker(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    hour_minute_only: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let picker = NSDatePicker::initWithFrame(
        NSDatePicker::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    picker.setDatePickerStyle(NSDatePickerStyle::TextFieldAndStepper);
    let elements = if hour_minute_only {
        NSDatePickerElementFlags::HourMinute
    } else {
        NSDatePickerElementFlags::HourMinuteSecond
    };
    picker.setDatePickerElements(elements);
    let frame_handle = resolve_frame_handle(parent);
    let target = DatePickerTarget::new(mtm, frame_handle, command_id);
    unsafe {
        picker.setTarget(Some(&target));
        picker.setAction(Some(sel!(ruWxDatePickerChanged:)));
    }
    content.addSubview(&picker);
    DATE_PICKERS.with(|m| m.borrow_mut().insert(handle, picker.clone()));
    store_ns_view(handle, picker.into_super().into_super());
    record_parent(handle, parent);
    handle
}

fn time_formatter() -> Retained<NSDateFormatter> {
    let fmt = NSDateFormatter::new();
    fmt.setDateFormat(Some(&NSString::from_str("HH:mm:ss")));
    fmt
}

pub fn time_picker_get_value(handle: NativeHandle) -> Option<(u32, u32, u32)> {
    DATE_PICKERS.with(|m| {
        m.borrow().get(&handle).and_then(|picker| {
            let fmt = time_formatter();
            let text = fmt.stringFromDate(&picker.dateValue()).to_string();
            parse_hms(&text)
        })
    })
}

pub fn time_picker_set_value(handle: NativeHandle, hour: u32, minute: u32, second: u32) {
    DATE_PICKERS.with(|m| {
        if let Some(picker) = m.borrow().get(&handle) {
            let text = format!("{hour:02}:{minute:02}:{second:02}");
            let fmt = time_formatter();
            if let Some(date) = fmt.dateFromString(&NSString::from_str(&text)) {
                picker.setDateValue(&date);
            }
        }
    });
}

fn parse_hms(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.split(':');
    let hour: u32 = parts.next()?.trim().parse().ok()?;
    let minute: u32 = parts.next()?.trim().parse().ok()?;
    let second: u32 = parts.next()?.trim().parse().ok()?;
    Some((hour, minute, second))
}

pub fn create_calendar_ctrl(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let picker = NSDatePicker::initWithFrame(
        NSDatePicker::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    picker.setDatePickerStyle(NSDatePickerStyle::ClockAndCalendar);
    picker.setDatePickerElements(
        NSDatePickerElementFlags::YearMonthDay | NSDatePickerElementFlags::Era,
    );
    let frame_handle = resolve_frame_handle(parent);
    let target = DatePickerTarget::new(mtm, frame_handle, command_id);
    unsafe {
        picker.setTarget(Some(&target));
        picker.setAction(Some(sel!(ruWxDatePickerChanged:)));
    }
    content.addSubview(&picker);
    DATE_PICKERS.with(|m| m.borrow_mut().insert(handle, picker.clone()));
    store_ns_view(handle, picker.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn create_radio_box(
    parent: NativeHandle,
    title: &str,
    labels: &[&str],
    initial_rect: Rect,
) -> (NativeHandle, Vec<NativeHandle>, Vec<u16>) {
    let box_handle = create_group_box(parent, title, initial_rect);
    let row_height = 22;
    let padding_top = 18;
    let padding_x = 10;
    let radio_width = initial_rect.width.saturating_sub(2 * padding_x as u32);
    let mut radio_handles = Vec::with_capacity(labels.len());
    let mut radio_ids = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        let radio_id = crate::platform::next_control_id();
        let radio_rect = Rect::new(
            padding_x,
            padding_top + i as i32 * row_height,
            radio_width,
            row_height as u32,
        );
        let handle = super::create_radio_button(parent, label, radio_id, radio_rect);
        radio_handles.push(handle);
        radio_ids.push(radio_id);
    }
    (box_handle, radio_handles, radio_ids)
}

pub fn create_spin_button(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    min: i32,
    max: i32,
    initial: i32,
    wrap: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let stepper = NSStepper::initWithFrame(
        NSStepper::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    stepper.setMinValue(min as f64);
    stepper.setMaxValue(max as f64);
    stepper.setDoubleValue(initial.clamp(min, max) as f64);
    stepper.setIncrement(1.0);
    stepper.setValueWraps(wrap);
    let frame_handle = resolve_frame_handle(parent);
    let target = SpinButtonTarget::new(mtm, frame_handle, command_id);
    unsafe {
        stepper.setTarget(Some(&target));
        stepper.setAction(Some(sel!(ruWxSpinButtonChanged:)));
    }
    content.addSubview(&stepper);
    SPIN_BUTTONS.with(|m| m.borrow_mut().insert(handle, stepper.clone()));
    store_ns_view(handle, stepper.into_super().into_super());
    record_parent(handle, parent);
    handle
}

pub fn spin_button_set_range(handle: NativeHandle, min: i32, max: i32) {
    SPIN_BUTTONS.with(|m| {
        if let Some(stepper) = m.borrow().get(&handle) {
            stepper.setMinValue(min as f64);
            stepper.setMaxValue(max as f64);
            let v = stepper.doubleValue().round().clamp(min as f64, max as f64);
            stepper.setDoubleValue(v);
        }
    });
}

pub fn spin_button_set_value(handle: NativeHandle, value: i32) {
    SPIN_BUTTONS.with(|m| {
        if let Some(stepper) = m.borrow().get(&handle) {
            let clamped = (value as f64)
                .max(stepper.minValue())
                .min(stepper.maxValue());
            stepper.setDoubleValue(clamped);
        }
    });
}

pub fn spin_button_get_value(handle: NativeHandle) -> i32 {
    SPIN_BUTTONS.with(|m| {
        m.borrow()
            .get(&handle)
            .map(|stepper| stepper.doubleValue().round() as i32)
            .unwrap_or(0)
    })
}

pub fn create_bitmap_button(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    bitmap_handle: NativeHandle,
    width: u32,
    height: u32,
) -> NativeHandle {
    let handle = super::create_button(parent, "", command_id, initial_rect);
    let side = width.min(height);
    super::button_set_bitmap_icon_wired(parent, handle, command_id, bitmap_handle, side);
    handle
}

pub fn create_search_field(
    parent: NativeHandle,
    placeholder: &str,
    initial_rect: Rect,
    command_id: u16,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let field = NSSearchField::initWithFrame(
        NSSearchField::alloc(mtm),
        ru_rect_to_ns(content_h, &initial_rect),
    );
    field.setStringValue(&NSString::from_str(""));
    field.setPlaceholderString(Some(&NSString::from_str(placeholder)));
    field.setEditable(true);
    field.setBezeled(true);
    field.setSendsSearchStringImmediately(true);
    content.addSubview(&field);
    store_ns_view(handle, field.into_super().into_super().into_super());
    record_parent(handle, parent);
    super::text_field_enable_change_notify(handle, command_id);
    handle
}

pub fn create_command_link_button(
    parent: NativeHandle,
    main: &str,
    note: &str,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let label = format!("{main}\n{note}");
    let handle = super::create_button(parent, &label, command_id, initial_rect);
    if let Some(view) = super::child_view_for_handle(handle) {
        if let Some(button) = view.downcast_ref::<NSButton>() {
            button.setBezelStyle(NSBezelStyle::Disclosure);
        }
    }
    handle
}

/// Parts of the spin (double) control: field, stepper, target and decimal places.
type SpinDoubleParts = (
    Retained<NSTextField>,
    Retained<NSStepper>,
    Retained<SpinDoubleTarget>,
    u32,
);

thread_local! {
    static SPIN_DOUBLE_PARTS: std::cell::RefCell<HashMap<NativeHandle, SpinDoubleParts>> =
        std::cell::RefCell::new(HashMap::new());
}

#[derive(Default)]
struct SpinDoubleTargetIvars {
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
    field: Cell<Option<Retained<NSTextField>>>,
    digits: Cell<u32>,
}

define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SpinDoubleTargetIvars]
    struct SpinDoubleTarget;

    unsafe impl NSObjectProtocol for SpinDoubleTarget {}

    impl SpinDoubleTarget {
        #[unsafe(method(ruWxSpinDoubleChanged:))]
        fn spin_changed(&self, sender: Option<&AnyObject>) {
            if let Some(stepper) = sender.and_then(|s| s.downcast_ref::<NSStepper>()) {
                let digits = self.ivars().digits.get();
                let v = stepper.doubleValue();
                if let Some(field) = self.ivars().field.take() {
                    field.setStringValue(&NSString::from_str(&format!(
                        "{v:.prec$}",
                        prec = digits as usize
                    )));
                    self.ivars().field.set(Some(field));
                }
            }
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }
);

impl SpinDoubleTarget {
    fn new(
        mtm: MainThreadMarker,
        frame_handle: NativeHandle,
        command_id: u16,
        field: Retained<NSTextField>,
        digits: u32,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SpinDoubleTargetIvars {
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
            field: Cell::new(Some(field)),
            digits: Cell::new(digits),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn layout_spin_double_parts(handle: NativeHandle, width: f64, height: f64) {
    const STEPPER_WIDTH: f64 = 22.0;
    SPIN_DOUBLE_PARTS.with(|map| {
        if let Some((field, stepper, _, _)) = map.borrow().get(&handle) {
            field.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new((width - STEPPER_WIDTH).max(0.0), height),
            ));
            stepper.setFrame(NSRect::new(
                NSPoint::new((width - STEPPER_WIDTH).max(0.0), 0.0),
                NSSize::new(STEPPER_WIDTH.min(width), height),
            ));
        }
    });
}

fn format_spin_double(value: f64, digits: u32) -> String {
    format!("{value:.prec$}", prec = digits as usize)
}

// `min/max/increment/digits` map 1:1 onto the platform-independent
// `SpinCtrlDouble` creation parameters exposed by the crate.
#[allow(clippy::too_many_arguments)]
pub fn create_spin_ctrl_double(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    min: f64,
    max: f64,
    initial: f64,
    increment: f64,
    digits: u32,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    let Some(content) = parent_view_for(parent) else {
        return handle;
    };
    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let container = NSView::initWithFrame(NSView::alloc(mtm), ns_rect);
    let clamped = initial.max(min).min(max);
    let field = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    field.setEditable(true);
    field.setBezeled(true);
    field.setDrawsBackground(true);
    field.setStringValue(&NSString::from_str(&format_spin_double(clamped, digits)));
    let stepper = NSStepper::initWithFrame(
        NSStepper::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(22.0, ns_rect.size.height),
        ),
    );
    stepper.setMinValue(min);
    stepper.setMaxValue(max);
    stepper.setDoubleValue(clamped);
    stepper.setIncrement(increment.max(0.000_001));
    let frame_handle = resolve_frame_handle(parent);
    let target = SpinDoubleTarget::new(mtm, frame_handle, command_id, field.clone(), digits);
    unsafe {
        stepper.setTarget(Some(&target));
        stepper.setAction(Some(sel!(ruWxSpinDoubleChanged:)));
    }
    container.addSubview(&field);
    container.addSubview(&stepper);
    content.addSubview(&container);
    SPIN_DOUBLE_PARTS.with(|map| {
        map.borrow_mut()
            .insert(handle, (field, stepper, target, digits));
    });
    layout_spin_double_parts(handle, ns_rect.size.width, ns_rect.size.height);
    store_ns_view(handle, container);
    record_parent(handle, parent);
    handle
}

pub fn spin_double_set_range(handle: NativeHandle, min: f64, max: f64) {
    SPIN_DOUBLE_PARTS.with(|map| {
        if let Some((field, stepper, _, digits)) = map.borrow().get(&handle) {
            stepper.setMinValue(min);
            stepper.setMaxValue(max);
            let v = stepper.doubleValue().max(min).min(max);
            stepper.setDoubleValue(v);
            field.setStringValue(&NSString::from_str(&format_spin_double(v, *digits)));
        }
    });
}

pub fn spin_double_set_value(handle: NativeHandle, value: f64) {
    SPIN_DOUBLE_PARTS.with(|map| {
        if let Some((field, stepper, _, digits)) = map.borrow().get(&handle) {
            let clamped = value.max(stepper.minValue()).min(stepper.maxValue());
            stepper.setDoubleValue(clamped);
            field.setStringValue(&NSString::from_str(&format_spin_double(clamped, *digits)));
        }
    });
}

pub fn spin_double_get_value(handle: NativeHandle) -> f64 {
    SPIN_DOUBLE_PARTS.with(|map| {
        map.borrow()
            .get(&handle)
            .map(|(_, stepper, _, _)| stepper.doubleValue())
            .unwrap_or(0.0)
    })
}

pub fn spin_double_set_increment(handle: NativeHandle, increment: f64) {
    SPIN_DOUBLE_PARTS.with(|map| {
        if let Some((_, stepper, _, _)) = map.borrow().get(&handle) {
            stepper.setIncrement(increment.max(0.000_001));
        }
    });
}

pub fn spin_double_set_digits(handle: NativeHandle, digits: u32) {
    SPIN_DOUBLE_PARTS.with(|map| {
        if let Some((field, stepper, _, stored)) = map.borrow_mut().get_mut(&handle) {
            *stored = digits;
            let v = stepper.doubleValue();
            field.setStringValue(&NSString::from_str(&format_spin_double(v, digits)));
        }
    });
}

pub fn macos_widget_set_frame(parent: NativeHandle, child: NativeHandle, rect: Rect) {
    super::set_view_frame(parent, child, rect);
}

pub fn macos_widget_set_visible(child: NativeHandle, visible: bool) {
    super::set_view_visible(child, visible);
}

pub fn macos_widget_set_enabled(child: NativeHandle, enabled: bool) {
    if let Some(view) = super::child_view_for_handle(child) {
        if let Some(button) = view.downcast_ref::<NSButton>() {
            button.setEnabled(enabled);
        } else if let Some(field) = view.downcast_ref::<NSTextField>() {
            field.setEnabled(enabled);
        } else if let Some(picker) = view.downcast_ref::<NSDatePicker>() {
            picker.setEnabled(enabled);
        } else if let Some(stepper) = view.downcast_ref::<NSStepper>() {
            stepper.setEnabled(enabled);
        }
        // Non-control views (panels, boxes, etc.) don't have an enabled state.
        // We don't hide them as a fallback since hiding ≠ disabling.
    }
}

/// Release a date picker from the thread-local map.
pub fn mac_date_picker_destroy(handle: NativeHandle) {
    DATE_PICKERS.with(|m| {
        m.borrow_mut().remove(&handle);
    });
}

/// Release a spin button (stepper) from the thread-local map.
pub fn mac_spin_button_destroy(handle: NativeHandle) {
    SPIN_BUTTONS.with(|m| {
        m.borrow_mut().remove(&handle);
    });
}

/// Release a spin double control from the thread-local map.
pub fn mac_spin_double_destroy(handle: NativeHandle) {
    SPIN_DOUBLE_PARTS.with(|m| {
        m.borrow_mut().remove(&handle);
    });
}
