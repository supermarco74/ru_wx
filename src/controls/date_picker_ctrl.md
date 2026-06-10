# date_picker_ctrl

`DatePickerCtrl` — wxWidgets-style date picker with an optional calendar drop-down. Backed
by the standard `SysDateTimePick32` common control.

## When to use

- The user must pick a single date (no time, no range) — birthdate, due date, log entry date.
- You want to support "no date" (the `DTS_SHOWNONE` style).
- You want either a calendar drop-down or a spin button (up/down) UI.

## Public types

```rust
/// A simple calendar date. `month` is 1..=12, `day` is 1..=31.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32, // 1..=12
    pub day: u32,   // 1..=31
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Self;
}

/// Date format for the control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    Short,  // locale's default short date (e.g. "06/05/2026")
    Long,   // locale's default long date (e.g. "Friday, June 5, 2026")
    Time,   // date + locale's default time
}

#[derive(Clone)]
pub struct DatePickerCtrl { /* Rc<RefCell<DatePickerCtrlInner>> */ }
```

## Public API

```rust
impl DatePickerCtrl {
    /// New picker as a child of `parent_in` (any `Window`).
    /// Short date, no "no date".
    pub fn new<W: Window>(parent_in: &W) -> Self;

    /// New picker with chosen format and (optionally) "no date" support.
    pub fn new_with_format<W: Window>(
        parent_in: &W,
        format: DateFormat,
        allow_none: bool,
    ) -> Self;

    /// New picker with spin buttons instead of a calendar drop-down.
    pub fn new_spin<W: Window>(parent_in: &W) -> Self;

    /// New picker that allows "no date" (DTS_SHOWNONE).
    pub fn allow_none<W: Window>(parent_in: &W) -> Self;

    /// Current date, or `None` if the control has no date set
    /// (only possible if the control was created with `allow_none`).
    pub fn get_value(&self) -> Option<Date>;

    /// Set the current date. If the control was created *without*
    /// `allow_none` and `value` is `None`, the call is a no-op.
    pub fn set_value(&self, value: Option<Date>);

    /// Register a callback that fires when the user picks a
    /// different date. Receives the new value: `Some(date)` if the
    /// user picked a real date, `None` if the control was cleared
    /// (only possible with `DTS_SHOWNONE` / `allow_none`).
    pub fn on_date_change<F: FnMut(Option<Date>) + 'static>(
        &self,
        frame: &Frame,
        mut callback: F,
    );

    pub fn id(&self) -> u16;
    pub fn as_widget_ref(&self) -> WidgetRef;
}
```

## Quick start

A complete, copy-pasteable "birthdate picker" example: a long-format
picker with "no date" support, seeded with a date, and a change callback
that logs both real and cleared values.

```rust,no_run
use ru_wx::prelude::*;

fn build_picker(frame: &Frame) -> DatePickerCtrl {
    // 1. Long-format picker with "no date" support (DTS_SHOWNONE).
    let picker = DatePickerCtrl::new_with_format(frame, DateFormat::Long, true);

    // 2. Seed with an initial value.
    picker.set_value(Some(Date::new(1990, 1, 1)));

    // 3. React to user picks. The callback receives Some(date) for a
    //    real pick, or None if the user cleared the checkbox.
    picker.on_date_change(frame, |new_value| {
        match new_value {
            Some(d)  => println!("picked: {}-{:02}-{:02}", d.year, d.month, d.day),
            None     => println!("cleared (no date)"),
        }
    });

    // 4. Read back the current value.
    if let Some(d) = picker.get_value() {
        println!("initial value: {}-{:02}-{:02}", d.year, d.month, d.day);
    }

    // 5. Pass to a sizer.
    // frame.set_sizer(...);
    picker
}

// Other constructor variants you can swap in:

#[allow(dead_code)]
fn build_spin_picker(frame: &Frame) -> DatePickerCtrl {
    // Spin buttons (up/down) instead of a calendar drop-down.
    DatePickerCtrl::new_spin(frame)
}

#[allow(dead_code)]
fn build_optional_picker(frame: &Frame) -> DatePickerCtrl {
    // "No date" support on top of the default short-format picker.
    DatePickerCtrl::allow_none(frame)
}
```

**Typical workflow**

1. Pick a constructor. The four are:
   - `new(parent)` — short format, no "no date".
   - `new_with_format(parent, format, allow_none)` — full control.
   - `new_spin(parent)` — spin buttons (up/down) instead of calendar.
   - `allow_none(parent)` — short format + "no date" checkbox.
2. (Optional) Seed the control with `set_value(Some(date))`. Pass `None`
   to clear it (only meaningful with `allow_none = true`).
3. Register a change callback with `on_date_change(frame, |value| ...)`.
   The callback fires on `DTN_DATETIMECHANGE` and receives the new
   `Option<Date>` (`None` for the cleared state).
4. Read the current value any time with `get_value()`.
5. Pass to a sizer via `as_widget_ref()`.

**Notes**

- `Date` is a `Copy` value type — `pub year: i32, pub month: u32 (1..=12),
  pub day: u32 (1..=31)`. Construction via `Date::new(y, m, d)` does not
  validate ranges; the native control may reject or normalise out-of-range
  values.
- The `allow_none` flag is sticky for the lifetime of the control.
  If you need a "no date" mode that can be toggled, drop and re-create.
- `set_value(None)` is a no-op on a control created without `allow_none` —
  the date is unchanged.
- The change callback is delivered via the parent `Frame`'s
  `dtn_handlers` map (keyed by control id), not the simpler
  `notify_handlers` map. This is what lets it carry the
  `NMDATETIMECHANGE` payload, not just the notification code.
- Cross-platform: `on_date_change` is registered on every platform, but
  the `dtn_handlers` map is only ever invoked on Windows. The callback
  simply never fires on non-Windows hosts.

## Win32 notes

- Window class: `SysDateTimePick32`. Default rect: `160 × 24`.
- Styles:
  - `WS_CHILD | WS_VISIBLE` always.
  - `DTS_UPDOWN = 0x0001` for the spin-button variant.
  - `DTS_SHOWNONE = 0x0002` to enable the "no date" checkbox.
  - `DTS_LONGDATEFORMAT = 0x0004` for `DateFormat::Long`.
  - `DTS_TIMEFORMAT = 0x0009` for `DateFormat::Time`.
- Messages:
  - `DTM_GETSYSTEMTIME = 0x1001` — wParam unused, lParam → `SYSTEMTIME`; low word of the
    `LRESULT` is `GDT_VALID` (0) or `GDT_NONE` (1).
  - `DTM_SETSYSTEMTIME = 0x1002` — wParam is the `GDT_*` flag, lParam → `SYSTEMTIME`.
- A local `#[repr(C)]` `SystemTime` struct mirrors `<winnt.h>` (8 × `u16` fields:
  year/month/weekday/day/hour/minute/second/millisecond). The weekday is left as 0 on write —
  the control does not validate it.
- A local `#[repr(C)] NmDateTimeChange` struct (NMHDR + `dw_flags` + SystemTime) is used to
  parse the `NMDATETIMECHANGE` notification body. `to_option()` returns `Some(Date)` if
  `dw_flags == GDT_VALID`, else `None`.
- The `DTN_DATETIMECHANGE` notification code is `0xFFFFFD09` and is delivered via `WM_NOTIFY`.
  It carries a pointer to the `NMDATETIMECHANGE` body. The handler registered with
  `on_date_change` must take the full `lparam` (not just the code) so it can dereference
  this body. This is why the dispatch goes through the frame's `dtn_handlers` map (a
  `Box<dyn FnMut(isize)>` keyed by control id) and not the simpler `notify_handlers` map
  used by e.g. `Tab::on_selection_change`.
- Cross-platform: `on_date_change` is wired on every platform; the `dtn_handlers` map is
  never invoked on non-Windows hosts, so the callback simply never fires. This matches
  `Frame::set_drop_files_callback`'s cross-platform ergonomics.

## Tests

6 unit tests pinning the date-conversion contract (constants + struct round-trip + happy
path + cleared path + Copy/Eq). Landed in v0.5.7 as a hot-fix pin: a future refactor of
`on_date_change` cannot silently regress the value the user receives.

- `date_new_constructs_value`
- `date_is_copy_and_eq`
- `dtn_datetimechange_constant_value` (Windows-only) — pins `0xFFFFFD09`.
- `nm_date_time_change_to_option_valid` (Windows-only) — happy path.
- `nm_date_time_change_to_option_none` (Windows-only) — "no date" path.
- `systemtime_date_round_trip` (Windows-only) — `from_date` ∘ `to_date` is lossless.

## Cross-references

- [frame](../window/frame.md) — `DatePickerCtrl::new` accepts any `Window` parent, but the
  `on_date_change` callback is delivered via the parent `Frame`'s `dtn_handlers` map.
- [widget](../core/widget.md) — `as_widget_ref()` for sizers.
- [sizer](../containers/sizer.md)
- [prelude](../prelude.md) — `Date` is exported from the prelude.
