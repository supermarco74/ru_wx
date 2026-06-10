# dpi.rs

High-DPI awareness, scaling helpers, and per-monitor DPI queries.

## Purpose

Modern Windows reports effective DPI values from 96 (100% scaling) to 384 (400%), with common non-defaults at 120 / 144 / 192 / 240 / 288. Code that draws lines, places widgets, or sizes fonts without taking the per-monitor DPI into account looks blurry or misaligned on any non-100% monitor.

`ru_wx` makes the process "DPI aware" via `<dpiAwareness>PerMonitorV2</dpiAwareness>` in `app.manifest` (see [`platform`](./platform.md)). This module exposes the same values back to user code so user layouts, icons, and custom drawings can scale with the rest of the system.

## Key types

- **`Dpi(u32)`** — newtype wrapping a raw DPI value.
  - `Dpi::new(v)` — `0` is coerced to 96 so a misbehaving caller cannot divide by zero on the next `scale_factor`.
  - `Dpi::default()` is `Dpi::new(96)`.
  - `.value() -> u32` — the raw value.
  - `.scale_factor() -> f32` — rounded to 4 decimal places, so `Dpi::from_scale_factor(dpi.scale_factor())` round-trips to the original.
  - `.scale(i32) -> i32` — logical (96-DPI) → physical pixels.
  - `.unscale(i32) -> i32` — physical → logical.
  - `Display` prints e.g. `Dpi(192 / 200%)`.
- **`DpiAwareness`** — `#[repr(i32)]` enum: `Unaware = 0`, `SystemAware = 1`, `PerMonitorAware = 2`. Mirrors the Win32 `PROCESS_DPI_AWARENESS` values.
  - `from_win32(value)` is `pub(crate)` and maps unknown values to `Unaware`.

## Public functions (all `#[cfg(target_os = "windows")]`, with non-Windows stubs)

- `pub const SYSTEM_DPI: u32 = 96;` — the standard baseline.
- `get_system_dpi() -> Dpi` — wraps `GetDpiForSystem`; falls back to 96 on non-Windows.
- `get_dpi_for_window(hwnd: HWND) -> Dpi` — wraps `GetDpiForWindow` (Win10 1607+). Null `hwnd` → `get_system_dpi`.
- `get_dpi_for_point(x: i32, y: i32) -> Dpi` — `MonitorFromPoint` + `GetDpiForMonitor` with `MDT_EFFECTIVE_DPI`.
- `get_process_dpi_awareness() -> DpiAwareness` — wraps `GetProcessDpiAwareness`. Defaults to `PerMonitorAware` on failure / non-Windows.
- `set_process_dpi_awareness(level: DpiAwareness) -> bool` — wraps `SetProcessDpiAwareness`. No-op + returns `false` on non-Windows.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. System DPI (96 on a 100% monitor, 192 on 200%, etc.).
let sys_dpi = get_system_dpi();
println!("system dpi = {}", sys_dpi);    // e.g. "Dpi(192 / 200%)"

// 2. Convert a logical 96-DPI size to physical pixels for a specific window.
let frame_for_dpi = frame.clone();
frame.on_size(move |_evt| {
    let hwnd = frame_for_dpi.hwnd();
    let dpi  = get_dpi_for_window(hwnd);
    // Original 20-px margin in 96-DPI units, scaled to the current monitor.
    let margin_phys = dpi.scale(20);
    // ... use margin_phys for layout ...
});

// 3. Convert the other way: physical pixels back to logical units.
let phys: i32 = 200;
let logical = sys_dpi.unscale(phys);

// 4. Per-monitor DPI for an arbitrary point (e.g. cursor position):
let cursor_dpi = get_dpi_for_point(100, 100);

// 5. The Dpi helpers are also exposed on the Frame:
let frame_dpi = frame.dpi();
let factor    = frame.scale_factor();   // e.g. 2.0 on a 200% monitor

// 6. Inspect or override the process awareness level:
let lvl = get_process_dpi_awareness();   // typically PerMonitorAware
let ok  = set_process_dpi_awareness(DpiAwareness::PerMonitorAware);
```

The `app.manifest` already requests `PerMonitorV2`, so you almost never need to call `set_process_dpi_awareness` yourself; the helpers above exist for code that draws at physical pixel coordinates, sizes custom widgets, or wants to test fallback paths.

## Win32 notes

- The `app.manifest` already requests `PerMonitorV2`, so most user code never calls `set_process_dpi_awareness`; the helper exists for users who build without the manifest or want to drop awareness at runtime.
- `GetDpiForMonitor` requires the `Win32_Graphics_Gdi` feature, which the library enables.
- `#[allow(clippy::not_unsafe_ptr_arg_deref)]` is used on `get_dpi_for_window` because the `HWND` raw pointer is wrapped in FFI calls that the lint doesn't recognise as "already inside an `unsafe` block".

## Tests

The module locks in:

- `Dpi::new(0)` → `96`; non-zero preserved.
- `scale_factor` is `value / 96` for 96, 120, 144, 192, 240, 288, 384.
- `from_scale_factor` round-trips for the standard values, and handles NaN / infinity / non-positive input by returning the 96-DPI baseline (the function is total — never panics).
- `scale` / `unscale` are inverses; both are the identity at 96 DPI.
- `SYSTEM_DPI` is exactly 96.
- `Display` formatting contains both the raw value and the percent.
- `get_system_dpi` returns non-zero.

## See also

- [`app.manifest`](../app.manifest) (project root) — sets the `PerMonitorV2` awareness.
- [`platform/win32.rs`](../platform/win32.md) — `get_device_caps_dpi` for an HDC-based fallback.
- [`frame.rs`](../window/frame.md) — every `Frame` has a `dpi()` / `scale_factor()` accessor.
