# colour_picker_ctrl

`ColourPickerCtrl` — wxWidgets-style "pick a colour" button. Backed by a Win32 `BUTTON` control
that, on click, opens the standard `ChooseColorW` dialog. The button label shows the current
selection in hex (`#RRGGBB`).

## When to use

- You need a small UI affordance for picking a colour (text colour, background tint, brush
  colour, etc.).
- You want to react to colour changes via a callback or read the current value at any time.

## Public API

```rust
#[derive(Clone)]
pub struct ColourPickerCtrl { /* Rc<RefCell<ColourPickerCtrlInner>> */ }

impl ColourPickerCtrl {
    /// Create a new picker. Parent can be any `Window` (Frame, Panel, Tab page, etc.).
    pub fn new<W: Window>(parent_in: &W) -> Self;

    /// Create a new picker pre-set to `colour`.
    pub fn with_colour<W: Window>(parent_in: &W, colour: Colour) -> Self;

    /// Current colour (always present; defaults to black `0x000000`).
    pub fn get_colour(&self) -> Colour;

    /// Programmatically set the colour. Updates the button label
    /// (`#RRGGBB`) and stores the value for the next dialog open.
    pub fn set_colour(&self, colour: Colour);

    /// Open the standard colour dialog *now*, with a one-shot
    /// callback that fires if the user clicks OK. The first
    /// parameter is unused (`_frame: Option<&Frame>`) — the
    /// dialog is parented to the control's own HWND, not to a
    /// Frame, so any `Window` parent is fine. Returns `true` if
    /// the user picked a colour.
    pub fn show_dialog<F: FnMut(Colour) + 'static>(
        &self,
        _frame: Option<&Frame>,
        mut on_change: F,
    ) -> bool;

    /// Register a callback that fires when the user picks a new
    /// colour via the dialog. The callback signature is
    /// `FnMut(Colour)` (no `Frame` parameter). The dialog is
    /// parented to the control's own HWND.
    pub fn on_change<F: FnMut(Colour) + 'static>(
        &self,
        frame: &Frame,
        mut callback: F,
    );

    /// Win32 control id.
    pub fn id(&self) -> u16;

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef;
}
```

The `Colour` type is a plain `(u8, u8, u8)` wrapper exported from the prelude.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Build a colour picker on a frame (or any Window). The standard
//    pattern is to parent it directly to a Frame so the on_change callback
//    reaches the frame's command dispatcher.
let picker = ColourPickerCtrl::new(&frame);

// 2. Pre-set a colour.
let picker_red = ColourPickerCtrl::with_colour(&frame, Colour::RED);

// 3. Read / set the current colour at any time.
let current: Colour = picker.get_colour();
picker.set_colour(Colour::from_rgb(0x33, 0x66, 0x99));

// 4. React to the user picking a new colour.
let preview_panel_for_change = panel.clone();
picker.on_change(&frame, move |c: Colour| {
    println!("new colour: {c:?}");
    // Repaint a swatch, refresh a custom-drawn panel, etc.
    preview_panel_for_change.refresh();
});

// 5. Or open the dialog programmatically with a one-shot callback:
let preview_panel_for_dialog = panel.clone();
picker.show_dialog(Some(&frame), move |c: Colour| {
    preview_panel_for_dialog.refresh();
});

// 6. Use it in a sizer:
let sizer = BoxSizer::new(Orientation::Horizontal);
sizer.add(&picker.as_widget_ref(), 0, 0, 0);
```

`ColourPickerCtrl` is a `BUTTON` that, on click, opens the standard `ChooseColorW` dialog parented to its own `HWND`. The button label shows the current selection in hex (`#RRGGBB`). Custom-colour swatches (`custom_colors: [u32; 16]`) are remembered across openings.

## Win32 notes

- Wraps a `BUTTON` window class with `BS_PUSHBUTTON_LOCAL = 0x0000_0000` (the standard push
  button style is implicit; declared locally to be self-documenting).
- Initial size: `140 × 28` pixels. Re-layout via a sizer (`add_with_proportion`, etc.) is
  supported; `set_position` / `set_size` come for free from the `Widget` trait.
- Colour dialog: `ChooseColorW(CHOOSECOLORW { ... })` with `CC_FULLOPEN | CC_RGBINIT`.
  - `custom_colors: [u32; 16]` is held in inner state and pre-populated into the dialog so
    the standard "Custom Colors" swatches survive across openings.
  - `rgbResult` is initialised from the current colour (via `CC_RGBINIT`).
- On `BN_CLICKED` (Win32 `WM_COMMAND` wNotifyCode=0), the control:
  1. Reads the current colour.
  2. Opens `ChooseColorW` parented to its own `HWND`.
  3. On user OK, updates the inner colour, refreshes the button label, and fires the
     `on_change` callback.
- The `on_change` callback is registered on the parent `Frame`'s command-handler map via the
  control's `id`, not on a free `FnMut` slot. Calling `on_change` on a `ColourPickerCtrl`
  parented to a `Panel` works only if the panel forwards the command to the owning frame —
  the **standard pattern is `ColourPickerCtrl::new(&frame)`**.

## Tests

No unit tests in this module (the dialog is interactive and requires user input).

## Cross-references

- [frame](../window/frame.md) — typically the parent. The control's `id` is registered into the frame's
  command dispatch so the click reaches the `on_change` callback.
- [panel](../window/panel.md) — also a valid parent if commands are forwarded.
- [sizer](../containers/sizer.md) — `as_widget_ref()` to insert into a `BoxSizer`.
- [prelude](../prelude.md) — exports `Colour`.
