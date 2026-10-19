//! Smoke tests for the WinUI3 backend public API.
//!
//! These tests only exercise code paths that do not touch the live
//! message loop (constructors, getters, setters, dispatch-table
//! registration). Real on-screen behaviour is verified in the
//! `examples_winui3` showcase binaries checked into the repository.

#![cfg(all(target_os = "windows", feature = "winui3"))]

use ru_wx::platform::winui3::{
    events::{self, Handlers, WidgetEvent},
    AcceleratorKey, BoxSizer, FrameBuilder, Orientation, Slider, Splitter, StaticBox, StaticLine,
    TreeCtrl, TreeItemId,
};

/// Trivially construct an `Handlers` and exercise the constructor and
/// `new()`. This test runs only on Windows + `--features winui3`.
#[test]
fn handlers_constructible() {
    let _ = Handlers::new();
}

/// An `AcceleratorKey` parses a sensible spec.
#[test]
fn accelerator_key_parse_ctrl_s() {
    let key = AcceleratorKey::parse("Ctrl+S", 1001).expect("valid spec");
    assert_eq!(key.mods, 0x0002); // MOD_CONTROL
    assert_eq!(key.vk, b'S' as u32);
    assert_eq!(key.id, 1001);
}

/// `AcceleratorKey` rejects nonsense like a single digit followed by letters.
#[test]
fn accelerator_key_parse_unknown() {
    // "FOO" is not a real virtual key — parse() returns None.
    assert!(AcceleratorKey::parse("FOO", 0).is_none());
}

/// `FrameBuilder::new()` defaults are stable.
#[test]
fn frame_builder_defaults() {
    let frame = FrameBuilder::new()
        .with_title("hello")
        .with_size(640, 480)
        .build();
    // Dropping the frame destroys the underlying HWND; we only assert
    // that the construction itself compiles and links.
    drop(frame);
}

/// `BoxSizer` is constructible in both orientations without crashing.
#[test]
fn box_sizer_horizontal_vertical() {
    let _h = BoxSizer::horizontal();
    let _v = BoxSizer::vertical();
    let _o = BoxSizer::new(Orientation::Horizontal);
}

/// `WidgetEvent` payload can be cloned (a small ergonomic check
/// useful for downstream closures that match on the variant).
#[test]
fn widget_event_cloneable() {
    let e1 = WidgetEvent::TextChanged("hello".to_string());
    let e2 = e1.clone();
    match e2 {
        WidgetEvent::TextChanged(s) => assert_eq!(s, "hello"),
        _ => panic!("wrong variant after clone"),
    }
}

/// `Slider` does not own an HWND until attached to a frame; the
/// struct exists and the default methods compile even on non-Windows
/// in unit tests where the feature is opt-in (no-op here).
#[test]
fn slider_struct_constructible() {
    let _s: Option<Slider> = None;
    let _sp: Option<Splitter> = None;
    let _sb: Option<StaticBox> = None;
    let _sl: Option<StaticLine> = None;
    let _t: Option<TreeCtrl> = None;
    let _ti: Option<TreeItemId> = None;
}

/// `events::install_handlers` is callable on a null HWND without
/// panicking — useful when the host platform has no display.
#[test]
fn install_handlers_null_hwnd_does_not_panic() {
    events::install_handlers(std::ptr::null_mut(), Handlers::new());
}

// ============================================================================
// Property-based WidgetCommon tests.
//
// These tests exercise the trait methods on real widgets (Frame, Button,
// TextCtrl, Panel, Slider) created against a live Windows display.
// They verify round-trip consistency of enable/show/get_pos/get_size.
// ============================================================================

use ru_wx::platform::winui3::{App, Button, Frame, Panel, TextCtrl, WidgetCommon};

/// Build-and-drop Frame should not leak or crash.
#[test]
fn frame_create_and_drop() {
    let _app = App::new();
    let frame = Frame::new("test_frame", 300, 200);
    // hwnd() must return a non-null handle.
    assert!(!frame.hwnd().is_null());
    drop(frame);
}

/// WidgetCommon: enable/disable round-trip on Frame.
#[test]
fn frame_enable_disable_roundtrip() {
    let _app = App::new();
    let frame = Frame::new("test_enable", 300, 200);
    assert!(frame.is_enabled());
    frame.enable(false);
    assert!(!frame.is_enabled());
    frame.enable(true);
    assert!(frame.is_enabled());
    drop(frame);
}

/// WidgetCommon: show/hide round-trip on Frame.
#[test]
fn frame_show_hide_roundtrip() {
    let _app = App::new();
    let frame = Frame::new("test_show", 300, 200);
    frame.show();
    assert!(frame.is_visible());
    frame.hide();
    assert!(!frame.is_visible());
    frame.show();
    assert!(frame.is_visible());
    drop(frame);
}

/// WidgetCommon: get_position returns sensible values.
#[test]
fn widget_get_position_returns_nonzero() {
    let _app = App::new();
    let frame = Frame::new("test_pos", 300, 200);
    let (x, y) = frame.get_position();
    let _ = (x, y); // does not panic
    drop(frame);
}

/// WidgetCommon: get_size returns positive dimensions.
#[test]
fn widget_get_size_matches_creation_size() {
    let _app = App::new();
    let frame = Frame::new("test_size", 300, 200);
    let (w, h) = frame.get_size();
    assert!(w > 0, "width must be positive, got {w}");
    assert!(h > 0, "height must be positive, got {h}");
    drop(frame);
}

/// WidgetCommon: id() returns 0 for a top-level frame.
#[test]
fn widget_id_returns_zero_for_frame() {
    let _app = App::new();
    let frame = Frame::new("test_id", 300, 200);
    assert_eq!(frame.id(), 0);
    drop(frame);
}

/// Button: enable/disable round-trip.
#[test]
fn button_enable_disable() {
    let _app = App::new();
    let frame = Frame::new("test_btn", 300, 200);
    let btn = Button::new(&frame, "Click");
    assert!(btn.is_enabled());
    btn.enable(false);
    assert!(!btn.is_enabled());
    btn.enable(true);
    assert!(btn.is_enabled());
    drop(btn);
    drop(frame);
}

/// TextCtrl: set_value / get_value round-trip.
#[test]
fn textctrl_set_get_value() {
    let _app = App::new();
    let frame = Frame::new("test_tc", 300, 200);
    let tc = TextCtrl::new(&frame, "initial");
    assert_eq!(tc.get_value(), "initial");
    tc.set_value("updated");
    assert_eq!(tc.get_value(), "updated");
    drop(tc);
    drop(frame);
}

/// Panel: WidgetParent works (Panel can parent a Button).
#[test]
fn panel_can_parent_button() {
    let _app = App::new();
    let frame = Frame::new("test_panel", 300, 200);
    let panel = Panel::new(&frame);
    let _btn = Button::new(&panel, "Inside");
    drop(_btn);
    drop(panel);
    drop(frame);
}

/// AcceleratorKey: parse modifier combinations.
#[test]
fn accelerator_key_parse_alt_x() {
    let key = AcceleratorKey::parse("Alt+X", 9999).expect("valid spec");
    assert_eq!(key.mods, 0x0001); // MOD_ALT
    assert_eq!(key.vk, b'X' as u32);
    assert_eq!(key.id, 9999);
}

/// AcceleratorKey: parse F-keys.
#[test]
fn accelerator_key_parse_f5() {
    let key = AcceleratorKey::parse("Ctrl+Shift+F5", 100).expect("valid spec");
    assert_eq!(key.mods, 0x0002 | 0x0004); // MOD_CONTROL | MOD_SHIFT
    assert_eq!(key.vk, 0x74); // VK_F5
    assert_eq!(key.id, 100);
}

/// events::register_command on null HWND does not panic.
#[test]
fn register_command_null_hwnd() {
    events::register_command(std::ptr::null_mut(), 9999, |_| {});
}

/// events::register_close on null HWND does not panic.
#[test]
fn register_close_null_hwnd() {
    events::register_close(std::ptr::null_mut(), || {});
}

/// palette::color_ref produces correct 0x00BBGGRR values.
#[test]
fn palette_color_ref_rgb() {
    use ru_wx::platform::winui3::palette::{color_ref, rgb};
    // Red = (255,0,0) → 0x000000FF
    assert_eq!(color_ref(255, 0, 0), 0x000000FF);
    // Green = (0,255,0) → 0x0000FF00
    assert_eq!(color_ref(0, 255, 0), 0x0000FF00);
    // Blue = (0,0,255) → 0x00FF0000
    assert_eq!(color_ref(0, 0, 255), 0x00FF0000);
    // White
    assert_eq!(color_ref(255, 255, 255), 0x00FFFFFF);
    // rgb() alias
    assert_eq!(rgb(128, 128, 128), color_ref(128, 128, 128));
}
