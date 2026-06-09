//! Keyboard accelerators (mnemonics + hotkeys) and Win32 `ACCEL` tables.
//!
//! This module provides a small, focused API for declaring menu
//! shortcuts and global hotkeys in a portable-ish shape:
//!
//! * [`VirtualKey`] enumerates the keys we can bind to an accelerator.
//! * [`Modifiers`] is a 3-bit bitflag (`CTRL | ALT | SHIFT`).
//! * [`Accelerator`] pairs the two; it can be parsed from a string
//!   like `"Ctrl+Shift+P"`, displayed back as `"Ctrl+Shift+P"`,
//!   and converted to a Win32 `ACCEL` entry.
//!
//! The accelerator string format is intentionally similar to the
//! one used by `wxWidgets` and by VS Code's `keybindings.json`:
//!
//! ```text
//! Ctrl+S            # S with the Ctrl modifier
//! Ctrl+Shift+P      # P with Ctrl and Shift
//! F5                # function key F5, no modifier
//! Alt+F4            # F4 with the Alt modifier
//! Escape            # bare key, no modifier
//! ```
//!
//! Modifiers are accepted in either case (`Ctrl` / `ctrl`), and the
//! keys are case-insensitive too (`s` and `S` are the same key).
//!
//! # Win32 integration
//!
//! On Windows the [`Accelerator::to_accel`] method produces a
//! `windows_sys::Win32::UI::WindowsAndMessaging::ACCEL` value that
//! can be passed to `CreateAcceleratorTableW`. The companion
//! [`Frame::register_accelerator`](crate::frame::Frame::register_accelerator)
//! helper adds the binding to a per-frame table that the message
//! loop translates with `TranslateAcceleratorW` and dispatches as a
//! `WM_COMMAND` (so it reuses the same handler table as
//! `Menu::append`).
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! use ru_wx::accelerator::Accelerator;
//!
//! let app = App::new();
//! let frame = Frame::builder().with_title("Demo").build();
//!
//! let mut file = Menu::new("&File");
//! let open_id = file.append_with_shortcut(
//!     "&Open...",
//!     Accelerator::parse("Ctrl+O").unwrap(),
//!     &frame,
//!     || println!("open!"),
//! );
//! // Wire the accelerator into the frame's table as well so the
//! // shortcut fires even when the menu is not visible.
//! frame.register_accelerator(
//!     Accelerator::parse("Ctrl+O").unwrap(),
//!     open_id,
//! );
//! ```

use std::fmt;
use std::fmt::Write as _;

/// Bitflag of keyboard modifier keys that can prefix an accelerator.
///
/// The flag values are the same ones used by the Win32 `ACCEL`
/// `fVirt` byte (`FCONTROL = 0x08`, `FALT = 0x10`, `FSHIFT = 0x04`)
/// so a `Modifiers` value can be combined directly with
/// [`FVIRTKEY`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-accel)
/// in an `ACCEL` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(pub u8);

impl Modifiers {
    /// No modifier (the bare key alone triggers the accelerator).
    pub const NONE: Modifiers = Modifiers(0);
    /// The `Ctrl` (Control) key.
    pub const CTRL: Modifiers = Modifiers(0x08);
    /// The `Alt` key.
    pub const ALT: Modifiers = Modifiers(0x10);
    /// The `Shift` key.
    pub const SHIFT: Modifiers = Modifiers(0x04);

    /// Construct a `Modifiers` from its three component booleans.
    ///
    /// ```
    /// use ru_wx::accelerator::Modifiers;
    /// let m = Modifiers::from_bools(true, false, true);
    /// assert_eq!(m, Modifiers::CTRL | Modifiers::SHIFT);
    /// ```
    pub const fn from_bools(ctrl: bool, alt: bool, shift: bool) -> Self {
        let mut bits = 0u8;
        if ctrl {
            bits |= 0x08;
        }
        if alt {
            bits |= 0x10;
        }
        if shift {
            bits |= 0x04;
        }
        Modifiers(bits)
    }

    /// `true` if the `Ctrl` bit is set.
    pub const fn ctrl(self) -> bool {
        (self.0 & 0x08) != 0
    }

    /// `true` if the `Alt` bit is set.
    pub const fn alt(self) -> bool {
        (self.0 & 0x10) != 0
    }

    /// `true` if the `Shift` bit is set.
    pub const fn shift(self) -> bool {
        (self.0 & 0x04) != 0
    }

    /// `true` if no modifier bit is set.
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Modifiers;
    fn bitor(self, rhs: Modifiers) -> Modifiers {
        Modifiers(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Modifiers) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for Modifiers {
    type Output = Modifiers;
    fn bitand(self, rhs: Modifiers) -> Modifiers {
        Modifiers(self.0 & rhs.0)
    }
}

impl fmt::Display for Modifiers {
    /// Format is `Ctrl+Alt+Shift` (the empty set is `""`).
    ///
    /// The order is fixed: `Ctrl` first, then `Alt`, then `Shift`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl() {
            write!(f, "Ctrl+")?;
        }
        if self.alt() {
            write!(f, "Alt+")?;
        }
        if self.shift() {
            write!(f, "Shift+")?;
        }
        Ok(())
    }
}

/// A single, named key that can trigger an accelerator.
///
/// The enum deliberately covers the keys that are useful in a GUI
/// hotkey (function keys, the alphanumeric range, the navigation
/// cluster) and a small handful of well-known editing keys. It
/// mirrors the Win32 virtual-key constants for the most part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualKey {
    /// A single letter, e.g. `'S'`. Stored upper-case regardless of
    /// how the user wrote it; matching is case-insensitive.
    Char(char),
    /// Function keys F1 through F12.
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    /// The `Escape` key.
    Escape,
    /// The `Tab` key.
    Tab,
    /// The `Enter` (Return) key.
    Enter,
    /// The `Space` bar.
    Space,
    /// The `Backspace` key.
    Backspace,
    /// The `Delete` (forward-delete) key.
    Delete,
    /// The `Insert` key.
    Insert,
    /// The `Home` key.
    Home,
    /// The `End` key.
    End,
    /// The `Page Up` (`PageUp`) key.
    PageUp,
    /// The `Page Down` (`PageDown`) key.
    PageDown,
    /// The left-arrow key.
    Left,
    /// The right-arrow key.
    Right,
    /// The up-arrow key.
    Up,
    /// The down-arrow key.
    Down,
}

impl fmt::Display for VirtualKey {
    /// Render the key as a human-readable token, e.g. `S`, `F5`, `Escape`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VirtualKey::Char(c) => write!(f, "{}", c.to_ascii_uppercase()),
            VirtualKey::F1 => write!(f, "F1"),
            VirtualKey::F2 => write!(f, "F2"),
            VirtualKey::F3 => write!(f, "F3"),
            VirtualKey::F4 => write!(f, "F4"),
            VirtualKey::F5 => write!(f, "F5"),
            VirtualKey::F6 => write!(f, "F6"),
            VirtualKey::F7 => write!(f, "F7"),
            VirtualKey::F8 => write!(f, "F8"),
            VirtualKey::F9 => write!(f, "F9"),
            VirtualKey::F10 => write!(f, "F10"),
            VirtualKey::F11 => write!(f, "F11"),
            VirtualKey::F12 => write!(f, "F12"),
            VirtualKey::Escape => write!(f, "Escape"),
            VirtualKey::Tab => write!(f, "Tab"),
            VirtualKey::Enter => write!(f, "Enter"),
            VirtualKey::Space => write!(f, "Space"),
            VirtualKey::Backspace => write!(f, "Backspace"),
            VirtualKey::Delete => write!(f, "Delete"),
            VirtualKey::Insert => write!(f, "Insert"),
            VirtualKey::Home => write!(f, "Home"),
            VirtualKey::End => write!(f, "End"),
            VirtualKey::PageUp => write!(f, "PageUp"),
            VirtualKey::PageDown => write!(f, "PageDown"),
            VirtualKey::Left => write!(f, "Left"),
            VirtualKey::Right => write!(f, "Right"),
            VirtualKey::Up => write!(f, "Up"),
            VirtualKey::Down => write!(f, "Down"),
        }
    }
}

/// A binding between a key + modifier set and a command id.
///
/// The `key` and `modifiers` together describe *what* the user
/// presses; the command id is stored in the Win32 `ACCEL` entry
/// and is also the id used by [`Frame::register_command_handler`](crate::frame::Frame::register_command_handler)
/// and by [`Menu::append`](crate::menu::Menu::append) callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Accelerator {
    pub key: VirtualKey,
    pub modifiers: Modifiers,
}

impl Accelerator {
    /// Construct an accelerator with no modifiers.
    pub fn new(key: VirtualKey) -> Self {
        Accelerator {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// Construct an accelerator with the given modifiers.
    pub fn with_modifiers(key: VirtualKey, modifiers: Modifiers) -> Self {
        Accelerator { key, modifiers }
    }

    /// Parse a textual representation such as `"Ctrl+S"` or
    /// `"Alt+Shift+F4"`.
    ///
    /// The format is one or more modifier names joined with `+`,
    /// followed by `+<key>`, where `<key>` is one of:
    ///
    /// * a single ASCII letter (`S`, `P`, `A`, ...),
    /// * a digit (`0`..`9`),
    /// * a function-key name (`F1` .. `F12`),
    /// * a named key (`Escape`, `Tab`, `Enter`, `Space`, `Backspace`,
    ///   `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`,
    ///   `Left`, `Right`, `Up`, `Down`).
    ///
    /// Whitespace around the tokens is permitted and ignored.
    /// Modifier names are accepted in any case (`Ctrl` / `CTRL` /
    /// `ctrl` all work).
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Empty`] if the string is empty or
    /// whitespace only, [`ParseError::MissingKey`] if the string
    /// contains only modifiers, [`ParseError::InvalidToken`] if a
    /// token is neither a known modifier nor a known key,
    /// [`ParseError::DuplicateModifier`] if a modifier appears
    /// twice, and [`ParseError::InvalidChar`] if a "letter" or
    /// "digit" key is not ASCII.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseError::Empty);
        }

        let mut modifiers = Modifiers::NONE;
        let mut last_key: Option<VirtualKey> = None;

        for raw_token in trimmed.split('+') {
            let token = raw_token.trim();
            if token.is_empty() {
                return Err(ParseError::Empty);
            }

            // Try the modifier table first; if it doesn't match,
            // fall through to the key parser.
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "ctl" => {
                    if modifiers.ctrl() {
                        return Err(ParseError::DuplicateModifier("Ctrl"));
                    }
                    modifiers |= Modifiers::CTRL;
                    continue;
                }
                "alt" => {
                    if modifiers.alt() {
                        return Err(ParseError::DuplicateModifier("Alt"));
                    }
                    modifiers |= Modifiers::ALT;
                    continue;
                }
                "shift" => {
                    if modifiers.shift() {
                        return Err(ParseError::DuplicateModifier("Shift"));
                    }
                    modifiers |= Modifiers::SHIFT;
                    continue;
                }
                _ => {}
            }

            // Not a modifier — must be the key (only one is allowed).
            if last_key.is_some() {
                return Err(ParseError::MissingKey);
            }
            last_key = Some(parse_key(token)?);
        }

        let key = last_key.ok_or(ParseError::MissingKey)?;
        Ok(Accelerator { key, modifiers })
    }

    /// Render the accelerator in the canonical `"Ctrl+Shift+S"` form
    /// (no trailing `+`; bare key for empty modifier).
    pub fn display(&self) -> String {
        let mut out = String::new();
        if !self.modifiers.is_none() {
            // The trailing `+` is part of the `Display` impl for
            // Modifiers when at least one bit is set, so we just
            // append it.
            write!(&mut out, "{}", self.modifiers).unwrap();
        }
        write!(&mut out, "{}", self.key).unwrap();
        out
    }
}

impl fmt::Display for Accelerator {
    /// Delegates to [`Accelerator::display`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

/// Errors returned by [`Accelerator::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The input was empty or contained only whitespace.
    Empty,
    /// The input contained modifiers but no key.
    MissingKey,
    /// A token was not a known modifier and not a known key.
    InvalidToken(String),
    /// A modifier appeared more than once.
    DuplicateModifier(&'static str),
    /// A "letter" or "digit" key was outside the ASCII range.
    InvalidChar,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => f.write_str("accelerator string is empty"),
            ParseError::MissingKey => f.write_str("accelerator string has no key"),
            ParseError::InvalidToken(t) => write!(f, "unknown accelerator token: {t:?}"),
            ParseError::DuplicateModifier(m) => write!(f, "modifier {m} listed twice"),
            ParseError::InvalidChar => f.write_str("non-ASCII character in accelerator"),
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_key(token: &str) -> Result<VirtualKey, ParseError> {
    // Function keys first (case-insensitive).
    match token.to_ascii_uppercase().as_str() {
        "F1" => return Ok(VirtualKey::F1),
        "F2" => return Ok(VirtualKey::F2),
        "F3" => return Ok(VirtualKey::F3),
        "F4" => return Ok(VirtualKey::F4),
        "F5" => return Ok(VirtualKey::F5),
        "F6" => return Ok(VirtualKey::F6),
        "F7" => return Ok(VirtualKey::F7),
        "F8" => return Ok(VirtualKey::F8),
        "F9" => return Ok(VirtualKey::F9),
        "F10" => return Ok(VirtualKey::F10),
        "F11" => return Ok(VirtualKey::F11),
        "F12" => return Ok(VirtualKey::F12),
        "ESCAPE" | "ESC" => return Ok(VirtualKey::Escape),
        "TAB" => return Ok(VirtualKey::Tab),
        "ENTER" | "RETURN" => return Ok(VirtualKey::Enter),
        "SPACE" | "SPACEBAR" => return Ok(VirtualKey::Space),
        "BACKSPACE" | "BS" => return Ok(VirtualKey::Backspace),
        "DELETE" | "DEL" => return Ok(VirtualKey::Delete),
        "INSERT" | "INS" => return Ok(VirtualKey::Insert),
        "HOME" => return Ok(VirtualKey::Home),
        "END" => return Ok(VirtualKey::End),
        "PAGEUP" | "PGUP" | "PRIOR" => return Ok(VirtualKey::PageUp),
        "PAGEDOWN" | "PGDN" | "PGDWN" | "NEXT" => return Ok(VirtualKey::PageDown),
        "LEFT" => return Ok(VirtualKey::Left),
        "RIGHT" => return Ok(VirtualKey::Right),
        "UP" => return Ok(VirtualKey::Up),
        "DOWN" => return Ok(VirtualKey::Down),
        _ => {}
    }

    // A single character — accept either a letter or a digit.
    let mut chars = token.chars();
    let c = chars.next().unwrap();
    if chars.next().is_some() {
        return Err(ParseError::InvalidToken(token.to_string()));
    }
    if c.is_ascii_alphabetic() {
        Ok(VirtualKey::Char(c.to_ascii_uppercase()))
    } else if c.is_ascii_digit() {
        Ok(VirtualKey::Char(c))
    } else {
        Err(ParseError::InvalidChar)
    }
}

/// Convert a `VirtualKey` to the corresponding Win32 virtual-key
/// code. The result is the bare `VK_*` value (no `FVIRTKEY`
/// modifier); callers building an `ACCEL` entry need to OR in
/// `FVIRTKEY` themselves.
#[cfg(target_os = "windows")]
pub fn virtual_key_to_win32(key: VirtualKey) -> u16 {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2,
        VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME, VK_INSERT, VK_LEFT, VK_NEXT,
        VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
    };
    match key {
        VirtualKey::Char(c) if c.is_ascii_alphabetic() => c.to_ascii_uppercase() as u16,
        VirtualKey::Char(c) if c.is_ascii_digit() => c as u16,
        VirtualKey::Char(_) => 0,
        VirtualKey::F1 => VK_F1,
        VirtualKey::F2 => VK_F2,
        VirtualKey::F3 => VK_F3,
        VirtualKey::F4 => VK_F4,
        VirtualKey::F5 => VK_F5,
        VirtualKey::F6 => VK_F6,
        VirtualKey::F7 => VK_F7,
        VirtualKey::F8 => VK_F8,
        VirtualKey::F9 => VK_F9,
        VirtualKey::F10 => VK_F10,
        VirtualKey::F11 => VK_F11,
        VirtualKey::F12 => VK_F12,
        VirtualKey::Escape => VK_ESCAPE,
        VirtualKey::Tab => VK_TAB,
        VirtualKey::Enter => VK_RETURN,
        VirtualKey::Space => VK_SPACE,
        VirtualKey::Backspace => VK_BACK,
        VirtualKey::Delete => VK_DELETE,
        VirtualKey::Insert => VK_INSERT,
        VirtualKey::Home => VK_HOME,
        VirtualKey::End => VK_END,
        VirtualKey::PageUp => VK_PRIOR,
        VirtualKey::PageDown => VK_NEXT,
        VirtualKey::Left => VK_LEFT,
        VirtualKey::Right => VK_RIGHT,
        VirtualKey::Up => VK_UP,
        VirtualKey::Down => VK_DOWN,
    }
}

#[cfg(target_os = "windows")]
impl Accelerator {
    /// Convert the accelerator to a Win32 `ACCEL` entry bound to
    /// `command` as the menu / window command id.
    ///
    /// The `fVirt` byte is built as `FVIRTKEY | FNOINVERT |
    /// modifiers.0`, where `FNOINVERT` (0x02) prevents the menu
    /// item from being visually highlighted when the accelerator
    /// fires — the standard "fire-and-forget" behavior expected of
    /// modern menus.
    ///
    /// `FNOINVERT` is a well-known `winuser.h` constant that the
    /// `windows-sys 0.59` crate does not export; we define it
    /// locally to keep the FFI surface self-contained.
    pub fn to_accel(self, command: u16) -> windows_sys::Win32::UI::WindowsAndMessaging::ACCEL {
        const FVIRTKEY: u8 = 0x01;
        const FNOINVERT: u8 = 0x02;
        windows_sys::Win32::UI::WindowsAndMessaging::ACCEL {
            fVirt: FVIRTKEY | FNOINVERT | self.modifiers.0,
            key: virtual_key_to_win32(self.key),
            cmd: command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Modifiers ----------

    #[test]
    fn modifiers_constants_are_disjoint_bits() {
        assert_eq!(Modifiers::CTRL.0, 0x08);
        assert_eq!(Modifiers::ALT.0, 0x10);
        assert_eq!(Modifiers::SHIFT.0, 0x04);
        assert_eq!(Modifiers::NONE.0, 0x00);
        // No two share a bit.
        assert_eq!(Modifiers::CTRL.0 & Modifiers::ALT.0, 0);
        assert_eq!(Modifiers::CTRL.0 & Modifiers::SHIFT.0, 0);
        assert_eq!(Modifiers::ALT.0 & Modifiers::SHIFT.0, 0);
    }

    #[test]
    fn modifiers_from_bools_round_trip() {
        for ctrl in [false, true] {
            for alt in [false, true] {
                for shift in [false, true] {
                    let m = Modifiers::from_bools(ctrl, alt, shift);
                    assert_eq!(m.ctrl(), ctrl);
                    assert_eq!(m.alt(), alt);
                    assert_eq!(m.shift(), shift);
                }
            }
        }
    }

    #[test]
    fn modifiers_bitor_accumulates() {
        let m = Modifiers::CTRL | Modifiers::SHIFT;
        assert!(m.ctrl());
        assert!(!m.alt());
        assert!(m.shift());
    }

    #[test]
    fn modifiers_display_is_canonical_order() {
        assert_eq!(Modifiers::NONE.to_string(), "");
        assert_eq!(Modifiers::CTRL.to_string(), "Ctrl+");
        assert_eq!(Modifiers::ALT.to_string(), "Alt+");
        assert_eq!(Modifiers::SHIFT.to_string(), "Shift+");
        let m = Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT;
        assert_eq!(m.to_string(), "Ctrl+Alt+Shift+");
    }

    // ---------- VirtualKey ----------

    #[test]
    fn virtualkey_display_matches_parse() {
        for k in [
            VirtualKey::F1,
            VirtualKey::F12,
            VirtualKey::Escape,
            VirtualKey::Char('Z'),
            VirtualKey::Char('A'),
        ] {
            let s = k.to_string();
            // Round-trip the ones the parser knows. The parser
            // upper-cases letters, so use the canonical form.
            let can_parse = matches!(
                k,
                VirtualKey::Char(_)
                    | VirtualKey::F1
                    | VirtualKey::F2
                    | VirtualKey::F3
                    | VirtualKey::F4
                    | VirtualKey::F5
                    | VirtualKey::F6
                    | VirtualKey::F7
                    | VirtualKey::F8
                    | VirtualKey::F9
                    | VirtualKey::F10
                    | VirtualKey::F11
                    | VirtualKey::F12
            );
            if can_parse {
                assert_eq!(Accelerator::parse(&s).unwrap().key, k);
            }
        }
    }

    // ---------- parse ----------

    #[test]
    fn parse_plain_letter_lowercased() {
        let a = Accelerator::parse("s").unwrap();
        assert_eq!(a.key, VirtualKey::Char('S'));
        assert_eq!(a.modifiers, Modifiers::NONE);
    }

    #[test]
    fn parse_plain_letter_uppercased() {
        let a = Accelerator::parse("S").unwrap();
        assert_eq!(a.key, VirtualKey::Char('S'));
        assert_eq!(a.modifiers, Modifiers::NONE);
    }

    #[test]
    fn parse_ctrl_plus_letter() {
        let a = Accelerator::parse("Ctrl+O").unwrap();
        assert_eq!(a.key, VirtualKey::Char('O'));
        assert!(a.modifiers.ctrl());
    }

    #[test]
    fn parse_case_insensitive_modifier() {
        let a = Accelerator::parse("ctrl+shift+p").unwrap();
        assert_eq!(a.key, VirtualKey::Char('P'));
        assert!(a.modifiers.ctrl());
        assert!(a.modifiers.shift());
    }

    #[test]
    fn parse_all_three_modifiers() {
        let a = Accelerator::parse("Ctrl+Alt+Shift+X").unwrap();
        assert_eq!(a.key, VirtualKey::Char('X'));
        assert!(a.modifiers.ctrl());
        assert!(a.modifiers.alt());
        assert!(a.modifiers.shift());
    }

    #[test]
    fn parse_function_key() {
        let a = Accelerator::parse("F5").unwrap();
        assert_eq!(a.key, VirtualKey::F5);
        assert_eq!(a.modifiers, Modifiers::NONE);
    }

    #[test]
    fn parse_function_key_with_modifier() {
        let a = Accelerator::parse("Alt+F4").unwrap();
        assert_eq!(a.key, VirtualKey::F4);
        assert!(a.modifiers.alt());
    }

    #[test]
    fn parse_named_key_aliases() {
        assert_eq!(Accelerator::parse("Esc").unwrap().key, VirtualKey::Escape);
        assert_eq!(Accelerator::parse("Return").unwrap().key, VirtualKey::Enter);
        assert_eq!(Accelerator::parse("PgUp").unwrap().key, VirtualKey::PageUp);
        assert_eq!(
            Accelerator::parse("PgDn").unwrap().key,
            VirtualKey::PageDown
        );
        assert_eq!(Accelerator::parse("Del").unwrap().key, VirtualKey::Delete);
    }

    #[test]
    fn parse_named_key_with_modifier() {
        let a = Accelerator::parse("Ctrl+Escape").unwrap();
        assert_eq!(a.key, VirtualKey::Escape);
        assert!(a.modifiers.ctrl());
    }

    #[test]
    fn parse_handles_whitespace() {
        let a = Accelerator::parse("  Ctrl  +  S  ").unwrap();
        assert_eq!(a.key, VirtualKey::Char('S'));
        assert!(a.modifiers.ctrl());
    }

    #[test]
    fn parse_digit_key() {
        let a = Accelerator::parse("Alt+1").unwrap();
        assert_eq!(a.key, VirtualKey::Char('1'));
        assert!(a.modifiers.alt());
    }

    // ---------- parse errors ----------

    #[test]
    fn parse_empty_string_errors() {
        assert_eq!(Accelerator::parse(""), Err(ParseError::Empty));
        assert_eq!(Accelerator::parse("   "), Err(ParseError::Empty));
    }

    #[test]
    fn parse_modifier_only_errors() {
        assert_eq!(Accelerator::parse("Ctrl"), Err(ParseError::MissingKey));
        assert_eq!(Accelerator::parse("Ctrl+Alt"), Err(ParseError::MissingKey));
    }

    #[test]
    fn parse_unknown_token_errors() {
        // Bare "Foo" — no modifier, no recognised key, no single
        // letter. parse_key returns InvalidToken.
        assert!(matches!(
            Accelerator::parse("Foo"),
            Err(ParseError::InvalidToken(_))
        ));
        // "Ctrl+Bar" -> "Bar" is unknown, but we already saw a
        // modifier, so it's InvalidToken.
        assert!(matches!(
            Accelerator::parse("Ctrl+Bar"),
            Err(ParseError::InvalidToken(_))
        ));
    }

    #[test]
    fn parse_duplicate_modifier_errors() {
        assert!(matches!(
            Accelerator::parse("Ctrl+Ctrl+S"),
            Err(ParseError::DuplicateModifier(_))
        ));
        assert!(matches!(
            Accelerator::parse("Alt+Alt"),
            Err(ParseError::DuplicateModifier(_))
        ));
    }

    #[test]
    fn parse_two_keys_errors() {
        // Two non-modifier tokens -> MissingKey (we already parsed a key).
        assert!(matches!(
            Accelerator::parse("Ctrl+S+P"),
            Err(ParseError::MissingKey)
        ));
    }

    // ---------- display round-trip ----------

    #[test]
    fn display_round_trip_simple() {
        let a = Accelerator::parse("Ctrl+S").unwrap();
        assert_eq!(a.to_string(), "Ctrl+S");
        assert_eq!(Accelerator::parse(&a.to_string()).unwrap(), a);
    }

    #[test]
    fn display_round_trip_no_modifier() {
        let a = Accelerator::parse("F5").unwrap();
        assert_eq!(a.to_string(), "F5");
        assert_eq!(Accelerator::parse(&a.to_string()).unwrap(), a);
    }

    #[test]
    fn display_round_trip_three_modifiers() {
        let a = Accelerator::parse("Ctrl+Alt+Shift+Z").unwrap();
        assert_eq!(a.to_string(), "Ctrl+Alt+Shift+Z");
        assert_eq!(Accelerator::parse(&a.to_string()).unwrap(), a);
    }

    // ---------- Win32 FFI ----------

    #[cfg(target_os = "windows")]
    #[test]
    fn to_accel_produces_fvirtkey_plus_modifier_bits() {
        let a = Accelerator::parse("Ctrl+S").unwrap();
        let entry = a.to_accel(0x1234);
        // FVirt must contain FVIRTKEY (0x01), FNOINVERT (0x02) and
        // the FCONTROL bit (0x08).
        assert_eq!(entry.fVirt & 0x01, 0x01);
        assert_eq!(entry.fVirt & 0x02, 0x02);
        assert_eq!(entry.fVirt & 0x08, 0x08);
        // 'S' is ASCII 0x53.
        assert_eq!(entry.key, 0x53);
        assert_eq!(entry.cmd, 0x1234);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn to_accel_function_key() {
        let a = Accelerator::parse("F5").unwrap();
        let entry = a.to_accel(0xABCD);
        // VK_F5 == 0x74 (116).
        assert_eq!(entry.key, 0x74);
        assert_eq!(entry.cmd, 0xABCD);
    }
}
