//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxColourPickerCtrl — a button that opens a colour-chooser dialog.
//!
//! On Windows there is no built-in colour-picker button, so we use a
//! `BUTTON` (push button) and open the standard `ChooseColorW` common
//! dialog when the button is clicked. The button label shows the
//! current colour as a hex string (e.g. `#FF8800`).
//!
//! Use [`ColourPickerCtrl::new`] to create the control, then call
//! [`ColourPickerCtrl::get_colour`] / [`ColourPickerCtrl::set_colour`]
//! to read or write the colour, and [`ColourPickerCtrl::on_change`] to
//! be notified when the user picks a new colour.

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::{Colour, Rect};
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseColorW, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// `BS_PUSHBUTTON` is exported as `i32` by windows-sys 0.59, while
// `WS_CHILD` / `WS_VISIBLE` are `u32`. Defining a local `u32` constant
// lets us mix the styles in a single bitwise-OR expression.
#[cfg(target_os = "windows")]
const BS_PUSHBUTTON_LOCAL: u32 = 0x0000_0000;

// ── Inner type ─────────────────────────────────────────────────────────

struct ColourPickerCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    colour: Colour,
    enabled: bool,
    visible: bool,
    /// Custom-colour palette for `ChooseColorW`. This buffer must remain
    /// alive for the entire time the dialog can be opened, so we keep it
    /// here in the inner state.
    #[cfg(target_os = "windows")]
    custom_colors: [u32; 16],
}

// ── Public type ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ColourPickerCtrl {
    inner: Rc<RefCell<ColourPickerCtrlInner>>,
}

impl ColourPickerCtrl {
    /// Create a new colour-picker as a child of `parent`.
    ///
    /// The initial colour is black.
    pub fn new<W: Window>(parent_in: &W) -> Self {
        Self::with_colour(parent_in, Colour::BLACK)
    }

    /// Create a new colour-picker with the given initial colour.
    pub fn with_colour<W: Window>(parent_in: &W, colour: Colour) -> Self {
        let id = next_control_id();
        let initial = format!("#{:02X}{:02X}{:02X}", colour.r, colour.g, colour.b);

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(&initial);
            let wide_class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON_LOCAL,
                0,
                0,
                140,
                28,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        ColourPickerCtrl {
            inner: Rc::new(RefCell::new(ColourPickerCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 140, 28),
                colour,
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                custom_colors: [0xFFFFFF; 16],
            })),
        }
    }

    /// Return the currently-selected colour.
    pub fn get_colour(&self) -> Colour {
        self.inner.borrow().colour
    }

    /// Set the current colour (does not show the dialog).
    pub fn set_colour(&self, colour: Colour) {
        {
            let mut state = self.inner.borrow_mut();
            state.colour = colour;
        }
        #[cfg(target_os = "windows")]
        {
            let state = self.inner.borrow();
            let label = format!(
                "#{:02X}{:02X}{:02X}",
                state.colour.r, state.colour.g, state.colour.b
            );
            let wide = to_wide(&label);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowTextW(state.hwnd, wide.as_ptr());
            }
        }
    }

    /// Show the standard Windows `ChooseColorW` dialog and, if the user
    /// clicked OK, update the stored colour to the chosen value and
    /// invoke the registered `on_change` callback.
    ///
    /// This is normally called automatically by the button click, but
    /// you can also call it directly to open the dialog programmatically.
    pub fn show_dialog<F: FnMut(Colour) + 'static>(
        &self,
        _frame: Option<&Frame>,
        mut on_change: F,
    ) -> bool {
        #[cfg(target_os = "windows")]
        {
            let mut state = self.inner.borrow_mut();
            let initial = state.colour.to_colorref();
            state.custom_colors[0] = initial;
            let mut cc = CHOOSECOLORW {
                lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                hwndOwner: state.hwnd,
                hInstance: std::ptr::null_mut(),
                rgbResult: initial,
                lpCustColors: state.custom_colors.as_mut_ptr(),
                Flags: CC_RGBINIT | CC_FULLOPEN,
                lCustData: 0,
                lpTemplateName: std::ptr::null(),
                lpfnHook: None,
            };
            // SAFETY: FFI call to ChooseColorW; the dialog struct is fully initialised and the user callback is the matching Rust closure.
            let ok = unsafe { ChooseColorW(&mut cc) };
            if ok != 0 {
                let r = (cc.rgbResult & 0xFF) as u8;
                let g = ((cc.rgbResult >> 8) & 0xFF) as u8;
                let b = ((cc.rgbResult >> 16) & 0xFF) as u8;
                let new_colour = Colour::new(r, g, b, 255);
                state.colour = new_colour;
                let label = format!("#{:02X}{:02X}{:02X}", r, g, b);
                let wide = to_wide(&label);
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SetWindowTextW(state.hwnd, wide.as_ptr());
                }
                drop(state);
                on_change(new_colour);
                return true;
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, on_change);
        }
        false
    }

    /// Register a callback that fires when the user picks a new colour
    /// via the colour-chooser dialog.
    pub fn on_change<F: FnMut(Colour) + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                // The user clicked the button: open the colour dialog.
                // If they pick a colour, invoke the user callback.
                #[cfg(target_os = "windows")]
                {
                    let mut state = inner.borrow_mut();
                    let initial = state.colour.to_colorref();
                    state.custom_colors[0] = initial;
                    let mut cc = CHOOSECOLORW {
                        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                        hwndOwner: state.hwnd,
                        hInstance: std::ptr::null_mut(),
                        rgbResult: initial,
                        lpCustColors: state.custom_colors.as_mut_ptr(),
                        Flags: CC_RGBINIT | CC_FULLOPEN,
                        lCustData: 0,
                        lpTemplateName: std::ptr::null(),
                        lpfnHook: None,
                    };
                    // SAFETY: FFI call to ChooseColorW; the dialog struct is fully initialised and the user callback is the matching Rust closure.
                    let ok = unsafe { ChooseColorW(&mut cc) };
                    if ok != 0 {
                        let r = (cc.rgbResult & 0xFF) as u8;
                        let g = ((cc.rgbResult >> 8) & 0xFF) as u8;
                        let b = ((cc.rgbResult >> 16) & 0xFF) as u8;
                        let new_colour = Colour::new(r, g, b, 255);
                        state.colour = new_colour;
                        let label = format!("#{:02X}{:02X}{:02X}", r, g, b);
                        let wide = to_wide(&label);
                        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                        unsafe {
                            SetWindowTextW(state.hwnd, wide.as_ptr());
                        }
                        drop(state);
                        callback(new_colour);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = &inner;
                }
            }),
        );
    }

    /// Get the control ID.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ───────────────────────────────────────────────────────

impl Widget for ColourPickerCtrlInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        0
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}
