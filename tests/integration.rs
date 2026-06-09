//! Cross-module integration tests.
//!
//! These tests live in the top-level `tests/` directory so they
//! can only see the **public** API of the crate (i.e. anything
//! re-exported from `lib.rs` and the `prelude` module). They are
//! the safety net that catches accidental leakage of
//! `pub(crate)` items into the public rustdoc output, and they
//! also serve as living documentation of the public API by
//! example.
//!
//! Anything that requires a real Win32 `HWND` (creating a
//! `Frame`, dispatching `WM_COMMAND`, painting a control) is
//! **not** testable from here on its own — the unit tests in
//! `src/frame.rs` use the `Frame::for_testing` constructor (which
//! is `pub(crate)` and therefore invisible to this file) to
//! exercise the platform-agnostic parts of the public surface.
//! The companion `examples/showcase_all.rs` binary is the
//! integration test for the windowed parts of the API.

// ---------- Public re-exports: `use ru_wx::*` ----------

#[test]
fn glob_import_brings_in_the_public_api() {
    // If any of these re-exports is accidentally removed, this
    // file will fail to compile, which is the desired behaviour.
    use ru_wx::*;

    // Re-exports from `accelerator`
    let _accel: Accelerator = Accelerator::parse("Ctrl+S").unwrap();
    let _mods: Modifiers = Modifiers::CTRL;
    let _vk: VirtualKey = VirtualKey::Char('S');
    let _: ParseError = ParseError::Empty;

    // Re-exports from `dpi`
    let _dpi: Dpi = Dpi::new(96);
    let _system_dpi: u32 = SYSTEM_DPI;

    // Re-exports from `geometry`
    let _rect: Rect = Rect::new(0, 0, 100, 100);
    let _colour: Colour = Colour::WHITE;

    // Re-exports from `sizer`
    let _sizer: BoxSizer = BoxSizer::horizontal();
    let _orient: Orientation = Orientation::Vertical;
}

#[test]
fn prelude_brings_in_the_everyday_api() {
    // The `prelude` module is a curated subset of `ru_wx` that
    // pulls in only the items a typical application needs. If
    // the prelude ever stops compiling, every downstream user
    // breaks at the same time, so we pin it here. We only
    // reference type names — the actual constructors require a
    // live `Frame` (and therefore a real `HWND`), which the
    // unit tests in `src/frame.rs` and the
    // `examples/showcase_all.rs` binary exercise in the
    // windowed parts of the API.
    use ru_wx::prelude::*;

    // The constructors of `Button` and `StaticText` are generic over
    // the `Window` parent type (`Button::new<W: Window>(...)`), so
    // they cannot be coerced to a non-generic `fn` pointer. The
    // type names themselves are in scope (which is what the prelude
    // contract actually guarantees), so we just reference them.
    let _app_t: fn() -> App = App::new;
    let _frame_builder_t: fn() -> FrameBuilder = Frame::builder;
    let _button_exists: Option<fn(&Frame, &str) -> Button> = None;
    let _text_exists: Option<fn(&Frame, &str) -> StaticText> = None;
    let _color_t: Colour = Colour::WHITE;
}

// ---------- Cross-module: Accelerator + Modifiers + VirtualKey ----------

#[test]
fn accelerator_via_modifiers_and_virtualkey_matches_parse() {
    // Constructing the same `Accelerator` two different ways
    // (struct literal vs. string parse) must produce equal
    // values. This pins the public constructor contract.
    use ru_wx::{Accelerator, Modifiers, VirtualKey};

    let from_struct = Accelerator::with_modifiers(VirtualKey::Char('S'), Modifiers::CTRL);
    let from_string = Accelerator::parse("Ctrl+S").unwrap();
    assert_eq!(from_struct, from_string);
}

#[test]
fn accelerator_parse_display_round_trip() {
    // For a representative sample of bindings, parsing the
    // `Display` output must round-trip back to the original
    // `Accelerator`. Catches accidental drift in the
    // canonical-order rules in `Modifiers::Display`.
    use ru_wx::Accelerator;

    for raw in [
        "Ctrl+S",
        "F5",
        "Alt+F4",
        "Ctrl+Alt+Shift+Z",
        "Escape",
        "Ctrl+1",
    ] {
        let parsed = Accelerator::parse(raw).unwrap();
        let displayed = parsed.to_string();
        let round_tripped = Accelerator::parse(&displayed).unwrap();
        assert_eq!(parsed, round_tripped, "round-trip failed for {raw:?}");
    }
}

// ---------- Cross-module: Dpi + scale/unscale ----------

#[test]
fn dpi_scale_unscale_round_trip() {
    // The Dpi newtype is the bridge between the user's logical
    // (96-DPI) coordinates and the physical pixels reported by
    // Windows. The round-trip must be exact for the common
    // scale factors.
    use ru_wx::Dpi;

    for raw in [96u32, 120, 144, 168, 192, 240, 288, 384] {
        let d = Dpi::new(raw);
        for logical in [0i32, 50, 100, 250, 800, 1234] {
            let physical = d.scale(logical);
            assert_eq!(d.unscale(physical), logical, "dpi={raw} logical={logical}");
        }
    }
}

#[test]
fn dpi_display_includes_value_and_percent() {
    // The `Display` impl is used in the showcase example's
    // status bar. Pin the format here so a refactor cannot
    // silently change the user-visible string.
    use ru_wx::Dpi;

    assert_eq!(Dpi::new(96).to_string(), "Dpi(96 / 100%)");
    assert_eq!(Dpi::new(120).to_string(), "Dpi(120 / 125%)");
    assert_eq!(Dpi::new(144).to_string(), "Dpi(144 / 150%)");
    assert_eq!(Dpi::new(192).to_string(), "Dpi(192 / 200%)");
    assert_eq!(Dpi::new(384).to_string(), "Dpi(384 / 400%)");
}

// ---------- Cross-module: Sizer getters added in v0.5.0 ----------

#[test]
fn box_sizer_getters_reflect_constructor() {
    // `BoxSizer::padding` and `BoxSizer::orientation` are the
    // getters added in v0.5.0 to make the sizer testable from
    // the outside. Pin their behaviour here.
    use ru_wx::{BoxSizer, Orientation};

    let mut h = BoxSizer::horizontal();
    assert_eq!(h.padding(), 5); // default
    assert!(matches!(h.orientation(), Orientation::Horizontal));
    h.set_padding(11);
    assert_eq!(h.padding(), 11);

    let v = BoxSizer::vertical();
    assert!(matches!(v.orientation(), Orientation::Vertical));
    assert_eq!(v.padding(), 5); // default
}

// ---------- Cross-module: Geometry ----------

#[test]
fn rect_contains_and_colorref_agree() {
    use ru_wx::{Colour, Rect};

    let r = Rect::new(10, 20, 30, 40);
    assert!(r.contains(10, 20));
    assert!(!r.contains(40, 60));

    // Pure red is 0x00BB_GG_RR = 0x0000_00FF.
    assert_eq!(Colour::new(0xFF, 0, 0, 0).to_colorref(), 0x0000_00FF);
    // Pure green is 0x00FF_00.
    assert_eq!(Colour::new(0, 0xFF, 0, 0).to_colorref(), 0x0000_FF00);
    // Pure blue is 0xFF_00_00.
    assert_eq!(Colour::new(0, 0, 0xFF, 0).to_colorref(), 0x00FF_0000);
}

// ---------- Cross-module: Accelerator + Modifier flags ----------

#[test]
fn modifiers_constants_match_the_win32_fvirt_bits() {
    // The Modifiers flags are the same bits that Win32 uses
    // for the `fVirt` byte of an `ACCEL` entry, but exposed
    // through a safe `u8` newtype. Lock the bit layout in.
    use ru_wx::Modifiers;

    assert_eq!(Modifiers::CTRL.0, 0x08); // FCONTROL
    assert_eq!(Modifiers::ALT.0, 0x10); // FALT
    assert_eq!(Modifiers::SHIFT.0, 0x04); // FSHIFT
    assert_eq!(Modifiers::NONE.0, 0x00);
    // Disjoint.
    assert_eq!(Modifiers::CTRL.0 & Modifiers::ALT.0, 0);
    assert_eq!(Modifiers::CTRL.0 & Modifiers::SHIFT.0, 0);
    assert_eq!(Modifiers::ALT.0 & Modifiers::SHIFT.0, 0);
}

// ---------- Cross-module: v0.5.1 runtime rebinding API ----------

#[test]
fn accelerator_rebinding_methods_have_expected_signatures() {
    // The v0.5.1 runtime-rebinding API is a set of three new
    // public methods on `Frame`. They require a live `HWND` to
    // actually mutate the in-memory table, so the unit tests in
    // `src/frame.rs` are the primary coverage (using
    // `Frame::for_testing`). From the integration layer we can
    // at least pin the function signatures so an accidental
    // rename, parameter-list change, or return-type change in
    // `frame.rs` is caught here.
    use ru_wx::{Accelerator, Frame};

    // Method with `&self`: the `&Frame` shows up as the first
    // argument of the function pointer.
    let _: fn(&Frame, Accelerator) -> bool = Frame::unregister_accelerator;
    let _: fn(&Frame) = Frame::clear_accelerators;
    let _: fn(&Frame, Accelerator, Accelerator, u16) -> bool = Frame::replace_accelerator;
}

#[test]
fn accelerator_rebinding_methods_are_reachable_through_the_prelude() {
    // The three new methods are part of the public surface that
    // `ru_wx::prelude` re-exports `Frame` from. If a future
    // refactor moves `Frame` out of the prelude (or moves the
    // new methods off `Frame`), this test will fail to compile,
    // which is the desired behaviour.
    use ru_wx::prelude::*;

    // `Frame` is in the prelude; the methods are inherent on it.
    let _: fn(&Frame, Accelerator) -> bool = Frame::unregister_accelerator;
    let _: fn(&Frame) = Frame::clear_accelerators;
    let _: fn(&Frame, Accelerator, Accelerator, u16) -> bool = Frame::replace_accelerator;
}

// ---------- Cross-module: v0.5.2 ListCtrl selection API ----------

#[test]
fn listctrl_selection_methods_have_expected_signatures() {
    // The v0.5.2 selection API is a set of six high-level methods
    // on `ListCtrl` (`select`, `deselect`, `clear_selection`,
    // `is_selected`, `get_selected_item_count`, `get_selected_items`)
    // plus two low-level helpers (`set_item_state`,
    // `get_item_state`) that give power-users direct access to the
    // underlying `LVM_SETITEMSTATE` / `LVM_GETITEMSTATE` messages.
    //
    // Constructing a real `ListCtrl` requires a live `HWND` (and
    // therefore a real `Frame`), so the unit tests in
    // `src/list_ctrl.rs` are the primary coverage. From the
    // integration layer we pin the function-pointer signatures so
    // an accidental rename, parameter-list change, or return-type
    // change in `list_ctrl.rs` is caught here.
    use ru_wx::{ListCtrl, ListCtrlStyle};

    // The constructor is generic over the `Window` parent type, so
    // it cannot be coerced to a non-generic `fn` pointer. We
    // reference the type names themselves (which is what the
    // public-API re-export contract actually guarantees) and
    // pin the enum variant surface, then pin the method
    // signatures as non-generic function pointers.
    let _style_exists: Option<ListCtrlStyle> = None;
    let _has_report: bool = matches!(ListCtrlStyle::Report, ListCtrlStyle::Report);
    let _has_list: bool = matches!(ListCtrlStyle::List, ListCtrlStyle::List);
    let _has_icon: bool = matches!(ListCtrlStyle::Icon, ListCtrlStyle::Icon);
    let _has_small_icon: bool = matches!(ListCtrlStyle::SmallIcon, ListCtrlStyle::SmallIcon);

    // High-level selection API (the wxWidgets-parity surface).
    let _: fn(&ListCtrl, usize) = ListCtrl::select;
    let _: fn(&ListCtrl, usize) = ListCtrl::deselect;
    let _: fn(&ListCtrl) = ListCtrl::clear_selection;
    let _: fn(&ListCtrl, usize) -> bool = ListCtrl::is_selected;
    let _: fn(&ListCtrl) -> usize = ListCtrl::get_selected_item_count;
    let _: fn(&ListCtrl) -> Vec<usize> = ListCtrl::get_selected_items;

    // Low-level state-bit helpers (power-user surface).
    let _: fn(&ListCtrl, usize, u32, u32) = ListCtrl::set_item_state;
    let _: fn(&ListCtrl, usize, u32) -> u32 = ListCtrl::get_item_state;
}

#[test]
fn listctrl_selection_methods_are_reachable_through_the_prelude() {
    // The new selection methods are inherent on `ListCtrl`, which
    // is re-exported from the `prelude` module. If a future
    // refactor moves `ListCtrl` out of the prelude, or moves the
    // selection methods off `ListCtrl`, this test will fail to
    // compile, which is the desired behaviour.
    use ru_wx::prelude::*;

    let _: fn(&ListCtrl, usize) = ListCtrl::select;
    let _: fn(&ListCtrl, usize) = ListCtrl::deselect;
    let _: fn(&ListCtrl) = ListCtrl::clear_selection;
    let _: fn(&ListCtrl, usize) -> bool = ListCtrl::is_selected;
    let _: fn(&ListCtrl) -> usize = ListCtrl::get_selected_item_count;
    let _: fn(&ListCtrl) -> Vec<usize> = ListCtrl::get_selected_items;
    let _: fn(&ListCtrl, usize, u32, u32) = ListCtrl::set_item_state;
    let _: fn(&ListCtrl, usize, u32) -> u32 = ListCtrl::get_item_state;
}

// ---------- Cross-module: v0.5.3 FileDialog multi-select API ----------

#[test]
fn file_dialog_multi_select_methods_have_expected_signatures() {
    // The v0.5.3 multi-select surface is two setters/getters
    // (`set_multi_select`, `is_multi_select`) and a new
    // modal show method (`show_modal_multi`) that returns a
    // `Vec<String>` of every file the user selected.
    //
    // As with `ListCtrl` and `Accelerator`, exercising these
    // methods requires a real `Frame` (and therefore a real
    // `HWND`), so the unit tests in `src/file_dialog.rs` are the
    // primary coverage. From the integration layer we pin the
    // function-pointer signatures so an accidental rename,
    // parameter-list change, or return-type change in
    // `file_dialog.rs` is caught here.
    use ru_wx::{FileDialog, FileDialogStyle};

    // Type-name pinning. The `new` constructor is generic over
    // the parent window type, so we cannot pin it as an `fn`
    // pointer, but we can at least confirm the enum variants
    // exist and the public type is reachable.
    let _style_exists: Option<FileDialogStyle> = None;
    let _open_is_open: bool = matches!(FileDialogStyle::Open, FileDialogStyle::Open);
    let _save_is_save: bool = matches!(FileDialogStyle::Save, FileDialogStyle::Save);
    let _open_ne_save: bool = FileDialogStyle::Open != FileDialogStyle::Save;

    // Multi-select setter and getter.
    let _: for<'a> fn(&'a mut FileDialog, bool) -> &'a mut FileDialog =
        FileDialog::set_multi_select;
    let _: fn(&FileDialog) -> bool = FileDialog::is_multi_select;

    // The new modal show method must return a `Vec<String>`.
    // We pin this as a function pointer; the call itself is
    // not made because it would block on a real dialog.
    let _: fn(&mut FileDialog) -> Vec<String> = FileDialog::show_modal_multi;
}

#[test]
fn file_dialog_multi_select_is_reachable_through_the_prelude() {
    // `FileDialog` and `FileDialogStyle` are re-exported from
    // `ru_wx::prelude`. If a future refactor moves them out of
    // the prelude, this test will fail to compile, which is the
    // desired behaviour.
    use ru_wx::prelude::*;

    let _: for<'a> fn(&'a mut FileDialog, bool) -> &'a mut FileDialog =
        FileDialog::set_multi_select;
    let _: fn(&FileDialog) -> bool = FileDialog::is_multi_select;
    let _: fn(&mut FileDialog) -> Vec<String> = FileDialog::show_modal_multi;
    let _open = FileDialogStyle::Open;
    let _save = FileDialogStyle::Save;
}
