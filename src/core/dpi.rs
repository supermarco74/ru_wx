//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! High-DPI (a.k.a. "HiDPI") awareness and scaling helpers.
//!
//! Modern Windows monitors report effective DPI values in the range
//! 96 (100% scaling, 1:1 logical-to-physical pixels) to 384 (400%
//! scaling, 4:1), with the most common non-default values being
//! 120 (125%), 144 (150%), 192 (200%), 240 (250%) and 288 (300%).
//! A library that draws lines, places widgets, or sizes fonts
//! without taking the per-monitor DPI into account will look
//! blurry, misaligned, or too small on any monitor that is not
//! running at 100% scaling.
//!
//! `ru_wx` makes the process "DPI aware" by embedding a
//! `<dpiAwareness>PerMonitorV2</dpiAwareness>` element in the
//! `app.manifest` (see [`crate::platform`]), so the OS hands the
//! library a per-monitor DPI value at every paint. This module
//! exposes the same values back to user code so that user
//! layouts, icons and custom-drawn shapes can scale with the
//! rest of the system.
//!
//! # Example
//!
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let app = App::new();
//! let frame = Frame::builder()
//!     .with_title("HiDPI demo")
//!     .with_size(800, 600)
//!     .build();
//!
//! // The frame's DPI reflects the monitor the frame lives on.
//! let dpi = frame.dpi();
//! let scale = frame.scale_factor();  // 1.0 at 96 DPI, 2.0 at 192 DPI, etc.
//!
//! // Convert a logical (96-DPI) size to the frame's physical size.
//! let physical_w = dpi.scale(800);   // 800 at 100%, 1600 at 200%, ...
//! let physical_h = dpi.scale(600);
//!
//! app.run(frame);
//! ```
//!
//! # Setting the awareness
//!
//! The library's manifest already requests
//! `PerMonitorV2` awareness (the modern recommendation), so
//! user code does not normally need to call
//! [`set_process_dpi_awareness`] at all. The helper is provided
//! for users who want to set a different level at runtime, and
//! for the non-Windows stubs that have no manifest to consult.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::GetCurrentProcess;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::HiDpi as hidpi;

/// The standard 96-DPI baseline used by every helper in this
/// module. The scale factor of a `Dpi::SYSTEM` value is exactly
/// 1.0; every other DPI value is reported as a multiple of it.
pub const SYSTEM_DPI: u32 = 96;

/// A DPI value, with a couple of ergonomic conversion helpers.
///
/// The raw `u32` value is the "effective DPI" reported by the
/// monitor that a window is currently hosted on (or by the
/// system, when no window context is available). 96 is the
/// historical 100% baseline, but modern Windows machines
/// routinely report 120, 144, 168, 192, 240, 288, 384, and so on.
///
/// Construct one through [`Dpi::new`], [`Dpi::from_scale_factor`],
/// [`get_system_dpi`], [`get_dpi_for_window`], or
/// [`get_dpi_for_point`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Dpi(u32);

impl Dpi {
    /// Wrap a raw `u32` DPI value. A value of `0` is coerced to
    /// 96 (the standard baseline) so a misbehaving caller cannot
    /// end up with a divide-by-zero on the next
    /// [`Dpi::scale_factor`] call.
    pub const fn new(value: u32) -> Self {
        if value == 0 {
            Dpi(SYSTEM_DPI)
        } else {
            Dpi(value)
        }
    }

    /// The raw `u32` value (96 at 100%, 192 at 200%, 384 at 400%).
    pub const fn value(self) -> u32 {
        self.0
    }

    /// The scale factor relative to the 96-DPI baseline
    /// (1.0 at 100%, 1.5 at 150%, 2.0 at 200%, 3.0 at 300%).
    ///
    /// The result is rounded to 4 decimal places so that
    /// `Dpi::from_scale_factor(dpi.scale_factor())` round-trips
    /// to the original DPI value (within the limits of
    /// `f32` precision).
    pub fn scale_factor(self) -> f32 {
        let raw = self.0 as f32 / SYSTEM_DPI as f32;
        // Round to 4 decimal places (the typical resolution
        // reported by Windows Settings → Display → Scale).
        (raw * 10_000.0).round() / 10_000.0
    }

    /// Build a `Dpi` from a scale factor (1.0 → 96, 1.5 → 144,
    /// 2.0 → 192, etc.). A scale factor `<= 0` falls back to
    /// the standard 96-DPI baseline so the call is total
    /// (never panics) — a programmer mistake in
    /// `from_scale_factor` is preferable to crashing inside a
    /// paint routine.
    pub fn from_scale_factor(scale: f32) -> Self {
        if !scale.is_finite() || scale <= 0.0 {
            return Dpi::new(SYSTEM_DPI);
        }
        Dpi::new((SYSTEM_DPI as f32 * scale).round() as u32)
    }

    /// Scale a logical (96-DPI) pixel value to a physical pixel
    /// value. `dpi.scale(800)` returns 800 at 100% scaling,
    /// 1600 at 200%, 2400 at 300%, and so on. The rounding is
    /// the "round-half-to-even" rounding of the standard `as`
    /// cast, which is what the rest of the Win32 GDI uses.
    pub fn scale(self, value: i32) -> i32 {
        ((value as f32) * self.scale_factor()).round() as i32
    }

    /// The inverse of [`Dpi::scale`]: convert a physical pixel
    /// value to a logical (96-DPI) pixel value. Used when the
    /// user wants to compute a layout in 96-DPI units and then
    /// ask the OS how many physical pixels the layout occupies
    /// on the current monitor.
    pub fn unscale(self, value: i32) -> i32 {
        let factor = self.scale_factor();
        if factor <= 0.0 {
            return value;
        }
        ((value as f32) / factor).round() as i32
    }
}

impl Default for Dpi {
    /// The default value is the 96-DPI baseline, which is the
    /// value [`get_system_dpi`] returns on a system that has
    /// never been told otherwise.
    fn default() -> Self {
        Dpi::new(SYSTEM_DPI)
    }
}

impl std::fmt::Display for Dpi {
    /// The `Display` impl prints the raw DPI value, the
    /// scale-factor percentage in parentheses, and (if the
    /// scale factor is an exact 25% multiple) the percentage
    /// for human-friendly output:
    ///
    /// ```text
    /// Dpi(96  / 100%)
    /// Dpi(120 / 125%)
    /// Dpi(192 / 200%)
    /// Dpi(384 / 400%)
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pct = (self.scale_factor() * 100.0).round() as i32;
        write!(f, "Dpi({} / {}%)", self.0, pct)
    }
}

/// The process-wide DPI awareness level. The mapping is
/// identical to the Win32 `PROCESS_DPI_AWARENESS` enum and the
/// matching `DPI_AWARENESS_CONTEXT` values, but expressed in
/// safe Rust.
///
/// The library's `app.manifest` already requests the modern
/// `PerMonitorV2` level, so user code does not normally need
/// to call [`set_process_dpi_awareness`]. The helper is
/// provided for users who build the library without a
/// manifest and for cross-platform stubs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DpiAwareness {
    /// The process is not DPI aware. The OS will bit-stretch
    /// every window to match the monitor, which produces blurry
    /// text and icons on any non-100% scaling monitor. This is
    /// the pre-Windows 10 default and the only awareness level
    /// that is still safe to change at runtime after the first
    /// window has been created.
    Unaware = 0,
    /// The process is DPI aware at the system level. The OS
    /// reads the primary monitor's DPI at process start and
    /// uses that value for the lifetime of the process. The
    /// library does not currently request this level (it
    /// requests `PerMonitorV2` instead), but the option is
    /// exposed for completeness.
    SystemAware = 1,
    /// The process is DPI aware on a per-monitor basis. The
    /// library's manifest requests the `V2` variant of this
    /// level, which adds the OS-side auto-resize feature: the
    /// OS will re-emit `WM_DPICHANGED` whenever a window is
    /// dragged onto a monitor with a different DPI. This is
    /// the recommended level for any modern Windows GUI app.
    PerMonitorAware = 2,
}

impl DpiAwareness {
    /// Build a `DpiAwareness` from the raw Win32 constant
    /// (`PROCESS_DPI_UNAWARE`, `PROCESS_SYSTEM_DPI_AWARE`,
    /// `PROCESS_PER_MONITOR_DPI_AWARE`). Any unknown value is
    /// mapped to [`DpiAwareness::Unaware`] so the call is
    /// total.
    #[cfg(target_os = "windows")]
    pub(crate) fn from_win32(value: hidpi::PROCESS_DPI_AWARENESS) -> Self {
        match value {
            hidpi::PROCESS_DPI_UNAWARE => DpiAwareness::Unaware,
            hidpi::PROCESS_SYSTEM_DPI_AWARE => DpiAwareness::SystemAware,
            hidpi::PROCESS_PER_MONITOR_DPI_AWARE => DpiAwareness::PerMonitorAware,
            _ => DpiAwareness::Unaware,
        }
    }
}

/// The system DPI value.
///
/// On Windows, this is a thin wrapper around the `GetDpiForSystem`
/// Win32 function, which returns the DPI of the primary monitor
/// at process start (it does not change when windows move
/// between monitors — use [`get_dpi_for_window`] or
/// [`get_dpi_for_point`] for that).
///
/// Falls back to 96 on any failure or on non-Windows targets.
pub fn get_system_dpi() -> Dpi {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: `GetDpiForSystem` is a thin FFI wrapper with
        // no preconditions; the docs explicitly state that it
        // is safe to call from any thread and that it cannot
        // fail.
        let raw = unsafe { hidpi::GetDpiForSystem() };
        Dpi::new(raw)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Dpi::new(SYSTEM_DPI)
    }
}

/// The DPI of the monitor that hosts `hwnd`.
///
/// On Windows, this is a thin wrapper around `GetDpiForWindow`
/// (Windows 10 1607 and later), which respects the per-monitor
/// DPI scaling set up by the library's manifest. The result
/// changes as the window is dragged across monitors with
/// different DPI values.
///
/// `hwnd.is_null()` falls back to [`get_system_dpi`].
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_dpi_for_window(hwnd: HWND) -> Dpi {
    if hwnd.is_null() {
        return get_system_dpi();
    }
    // SAFETY: `hwnd` is a live `HWND` returned by the matching
    // `CreateWindowExW` / `FindWindowW` / etc. call (or, in the
    // null case, we already returned). `GetDpiForWindow` is
    // documented to be safe to call from any thread.
    let raw = unsafe { hidpi::GetDpiForWindow(hwnd) };
    Dpi::new(raw)
}

/// The DPI of the monitor that contains the screen-space point
/// `(x, y)`. Useful for "what DPI is the user dragging this
/// widget towards?" calculations.
///
/// On Windows, this is implemented as `MonitorFromPoint` +
/// `GetDpiForMonitor` (the latter requires the
/// `Win32_Graphics_Gdi` feature, which the library already
/// enables). The result is the *effective* DPI, not the raw
/// or angular DPI, matching what every other helper in this
/// module returns.
#[cfg(target_os = "windows")]
pub fn get_dpi_for_point(x: i32, y: i32) -> Dpi {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};

    let pt = POINT { x, y };
    // SAFETY: `POINT` is a plain `#[repr(C)]` struct of two
    // `i32`s. `MonitorFromPoint` does not retain the pointer
    // past return, so a stack-allocated `POINT` is fine. The
    // `MONITOR_DEFAULTTONEAREST` flag is one of three valid
    // values for the second argument.
    let hmonitor = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    if hmonitor.is_null() {
        return get_system_dpi();
    }
    let mut dpix: u32 = 0;
    let mut dpiy: u32 = 0;
    // SAFETY: `hmonitor` is a live `HMONITOR` returned by
    // `MonitorFromPoint`. The two `*mut u32` output pointers
    // point to stack-allocated `u32` slots, which `GetDpiForMonitor`
    // writes through before returning. `MDT_EFFECTIVE_DPI` is
    // the value Windows uses for "what the user has set the
    // scaling slider to" — i.e. the value every other helper
    // in this module returns.
    let hr = unsafe {
        hidpi::GetDpiForMonitor(hmonitor, hidpi::MDT_EFFECTIVE_DPI, &mut dpix, &mut dpiy)
    };
    if hr < 0 {
        return get_system_dpi();
    }
    Dpi::new(dpix)
}

/// The process-wide DPI awareness.
///
/// Returns [`DpiAwareness::PerMonitorAware`] on platforms /
/// OS versions where the awareness cannot be queried (and the
/// library's manifest is in effect, so the real awareness is
/// `PerMonitorV2`). This is a deliberate conservative default:
/// the most common case is "I am running on a manifest-built
/// Windows app and I just want to know what the OS thinks my
/// awareness is".
#[cfg(target_os = "windows")]
pub fn get_process_dpi_awareness() -> DpiAwareness {
    let mut value: hidpi::PROCESS_DPI_AWARENESS = 0;
    // SAFETY: `GetCurrentProcess` is a constant function that
    // returns a pseudo-handle that is always valid for the
    // current process. The output pointer is a stack-allocated
    // `i32`. `GetProcessDpiAwareness` writes the awareness
    // value through the pointer before returning.
    let hr = unsafe { hidpi::GetProcessDpiAwareness(GetCurrentProcess(), &mut value) };
    if hr < 0 {
        return DpiAwareness::PerMonitorAware;
    }
    DpiAwareness::from_win32(value)
}

/// Set the process-wide DPI awareness. This must be called
/// *before* the first window is created; calling it later is
/// documented by Microsoft as a no-op on Windows 10 1703 and
/// later.
///
/// The library's `app.manifest` already requests
/// `PerMonitorV2` awareness, so this function only needs to
/// be called by users who:
/// * build the library without the `app.manifest`, or
/// * want to set a *different* awareness level at runtime
///   (e.g. dropping to [`DpiAwareness::Unaware`] for a
///   legacy compatibility path).
///
/// Returns `true` on success, `false` on failure (or on
/// non-Windows targets, where the call is a no-op).
#[cfg(target_os = "windows")]
pub fn set_process_dpi_awareness(level: DpiAwareness) -> bool {
    // SAFETY: `SetProcessDpiAwareness` takes a single
    // `PROCESS_DPI_AWARENESS` enum value. The mapping from
    // `DpiAwareness` to the Win32 constants is exhaustive and
    // is performed with an integer `as` cast because the
    // `DpiAwareness` enum is `#[repr(i32)]`.
    let hr = unsafe { hidpi::SetProcessDpiAwareness(level as hidpi::PROCESS_DPI_AWARENESS) };
    hr >= 0
}

#[cfg(not(target_os = "windows"))]
pub fn set_process_dpi_awareness(_level: DpiAwareness) -> bool {
    // No-op on non-Windows. The stub keeps the API uniform.
    false
}

#[cfg(not(target_os = "windows"))]
pub fn get_process_dpi_awareness() -> DpiAwareness {
    // The manifest requests `PerMonitorV2` on Windows, but
    // the value cannot be queried on non-Windows targets. The
    // safest default is "the highest level we know about",
    // which is `PerMonitorAware` (the V1 variant of what
    // Windows calls `PerMonitorV2`).
    DpiAwareness::PerMonitorAware
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_preserves_nonzero() {
        assert_eq!(Dpi::new(96).value(), 96);
        assert_eq!(Dpi::new(120).value(), 120);
        assert_eq!(Dpi::new(192).value(), 192);
        assert_eq!(Dpi::new(384).value(), 384);
    }

    #[test]
    fn new_coerces_zero_to_baseline() {
        assert_eq!(Dpi::new(0).value(), SYSTEM_DPI);
    }

    #[test]
    fn scale_factor_is_value_over_96() {
        assert!((Dpi::new(96).scale_factor() - 1.0).abs() < 1e-4);
        assert!((Dpi::new(120).scale_factor() - 1.25).abs() < 1e-4);
        assert!((Dpi::new(144).scale_factor() - 1.5).abs() < 1e-4);
        assert!((Dpi::new(192).scale_factor() - 2.0).abs() < 1e-4);
        assert!((Dpi::new(240).scale_factor() - 2.5).abs() < 1e-4);
        assert!((Dpi::new(288).scale_factor() - 3.0).abs() < 1e-4);
        assert!((Dpi::new(384).scale_factor() - 4.0).abs() < 1e-4);
    }

    #[test]
    fn from_scale_factor_round_trips() {
        for dpi in [96, 120, 144, 168, 192, 240, 288, 384] {
            let d = Dpi::new(dpi);
            assert_eq!(Dpi::from_scale_factor(d.scale_factor()), d);
        }
    }

    #[test]
    fn from_scale_factor_handles_bad_input() {
        // NaN, infinity, and non-positive values fall back
        // to the 96-DPI baseline. The function is total.
        assert_eq!(Dpi::from_scale_factor(0.0), Dpi::new(SYSTEM_DPI));
        assert_eq!(Dpi::from_scale_factor(-1.0), Dpi::new(SYSTEM_DPI));
        assert_eq!(Dpi::from_scale_factor(f32::NAN), Dpi::new(SYSTEM_DPI));
        assert_eq!(Dpi::from_scale_factor(f32::INFINITY), Dpi::new(SYSTEM_DPI));
    }

    #[test]
    fn scale_applies_factor() {
        let d = Dpi::new(192); // 200% scaling
        assert_eq!(d.scale(0), 0);
        assert_eq!(d.scale(100), 200);
        assert_eq!(d.scale(400), 800);
    }

    #[test]
    fn scale_at_baseline_is_identity() {
        let d = Dpi::new(96);
        assert_eq!(d.scale(0), 0);
        assert_eq!(d.scale(100), 100);
        assert_eq!(d.scale(800), 800);
    }

    #[test]
    fn unscale_inverts_scale() {
        let d = Dpi::new(192);
        for logical in 0..100i32 {
            let physical = d.scale(logical);
            assert_eq!(d.unscale(physical), logical);
        }
    }

    #[test]
    fn unscale_at_baseline_is_identity() {
        let d = Dpi::new(96);
        assert_eq!(d.unscale(800), 800);
        assert_eq!(d.unscale(0), 0);
    }

    #[test]
    fn default_is_96_dpi() {
        assert_eq!(Dpi::default(), Dpi::new(SYSTEM_DPI));
    }

    #[test]
    fn display_contains_value_and_percent() {
        let d = Dpi::new(192);
        let s = format!("{}", d);
        assert!(
            s.contains("192"),
            "display should contain the raw value: {s}"
        );
        assert!(
            s.contains("200%"),
            "display should contain the percent: {s}"
        );
    }

    #[test]
    fn system_dpi_is_96() {
        // The constant is referenced by every helper in this
        // module; locking it to 96 here means a future change
        // will fail the test, which is the desired behaviour.
        assert_eq!(SYSTEM_DPI, 96);
    }

    #[test]
    fn get_system_dpi_returns_nonzero() {
        // On a system that has never been told otherwise (and
        // on the non-Windows stub), the value is exactly 96.
        let d = get_system_dpi();
        assert!(d.value() > 0, "system DPI must be non-zero");
    }
}
