//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxWidgets-style button catalogue — standard, flat, bitmap-only,
//! text+image (left / right), command link, toggle, menu drop-down,
//! and animated variants. Examples should call [`ButtonVariants`]
//! factories rather than re-implementing Win32 styles.

pub use crate::controls::button::BitmapAlign;

use crate::controls::animated_button::AnimatedButton;
use crate::controls::bitmap_button::BitmapButton;
use crate::controls::bitmap_toggle_button::BitmapToggleButton;
use crate::controls::button::Button;
use crate::controls::command_link_button::CommandLinkButton;
use crate::controls::menu_button::MenuButton;
use crate::controls::toggle_button::ToggleButton;
use crate::dc::bitmap::Bitmap;
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// Human-readable name for each wx-style button variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Standard,
    Flat,
    BitmapOnly,
    TextWithImageLeft,
    TextWithImageRight,
    CommandLink,
    Toggle,
    BitmapToggle,
    MenuDropDown,
    Animated,
}

/// Unified handle for any button variant (for galleries / showcases).
#[derive(Clone)]
pub enum AnyButton {
    Push(Button),
    Bitmap(BitmapButton),
    Toggle(ToggleButton),
    BitmapToggle(BitmapToggleButton),
    CommandLink(CommandLinkButton),
    Menu(MenuButton),
    Animated(AnimatedButton),
}

impl AnyButton {
    pub fn kind(&self) -> ButtonKind {
        match self {
            AnyButton::Push(b) if b.is_flat() => ButtonKind::Flat,
            AnyButton::Push(b) if b.has_image_list() => {
                if b.bitmap_align() == Some(BitmapAlign::Right) {
                    ButtonKind::TextWithImageRight
                } else {
                    ButtonKind::TextWithImageLeft
                }
            }
            AnyButton::Push(_) => ButtonKind::Standard,
            AnyButton::Bitmap(_) => ButtonKind::BitmapOnly,
            AnyButton::Toggle(_) => ButtonKind::Toggle,
            AnyButton::BitmapToggle(_) => ButtonKind::BitmapToggle,
            AnyButton::CommandLink(_) => ButtonKind::CommandLink,
            AnyButton::Menu(_) => ButtonKind::MenuDropDown,
            AnyButton::Animated(_) => ButtonKind::Animated,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self.kind() {
            ButtonKind::Standard => "wxButton — standard",
            ButtonKind::Flat => "wxButton — flat / liscio",
            ButtonKind::BitmapOnly => "wxBitmapButton — solo immagine",
            ButtonKind::TextWithImageLeft => "wxButton — testo + immagine a sinistra",
            ButtonKind::TextWithImageRight => "wxButton — testo + immagine a destra",
            ButtonKind::CommandLink => "wxCommandLinkButton",
            ButtonKind::Toggle => "wxToggleButton",
            ButtonKind::BitmapToggle => "wxBitmapToggleButton",
            ButtonKind::MenuDropDown => "wxMenuButton — menu a discesa",
            ButtonKind::Animated => "wxButton — animazione interna",
        }
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        match self {
            AnyButton::Push(b) => b.as_widget_ref(),
            AnyButton::Bitmap(b) => b.as_widget_ref(),
            AnyButton::Toggle(b) => b.as_widget_ref(),
            AnyButton::BitmapToggle(b) => b.as_widget_ref(),
            AnyButton::CommandLink(b) => b.as_widget_ref(),
            AnyButton::Menu(b) => b.as_widget_ref(),
            AnyButton::Animated(b) => b.as_widget_ref(),
        }
    }

    /// Register a simple click handler (toggle variants fire on each click).
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        match self {
            AnyButton::Push(b) => b.on_click(frame, callback),
            AnyButton::Bitmap(b) => b.on_click(frame, callback),
            AnyButton::Toggle(b) => b.on_click(frame, callback),
            AnyButton::BitmapToggle(b) => {
                let mut f = callback;
                b.on_click(frame, move |_| f());
            }
            AnyButton::CommandLink(b) => b.on_click(frame, callback),
            AnyButton::Animated(b) => b.on_click(frame, callback),
            AnyButton::Menu(_) => {}
        }
    }

    /// Convenience helper for demo pages: update status field 0.
    pub fn on_click_status(&self, frame: &Frame, status: &crate::StatusBar, message: &str) {
        let msg = message.to_string();
        let status = status.clone();
        self.on_click(frame, move || status.set_status_text(&msg, 0));
    }

    /// For [`AnyButton::Menu`]: attach the drop-down menu popup.
    pub fn bind_menu_popup(&self, frame: &Frame) {
        if let AnyButton::Menu(m) = self {
            m.bind_popup(frame);
        }
    }

    /// Mutable access to the embedded menu (panics if not a menu button).
    pub fn menu_mut(&self) -> std::cell::RefMut<'_, crate::window::menu::Menu> {
        match self {
            AnyButton::Menu(m) => m.menu_mut(),
            _ => panic!("AnyButton::menu_mut called on a non-menu variant"),
        }
    }
}

/// Factory for all wxWidgets-style button types.
pub struct ButtonVariants;

impl ButtonVariants {
    /// Standard push button (`wxButton`, `BS_PUSHBUTTON`).
    pub fn standard<W: crate::core::widget::Window>(parent: &W, label: &str) -> AnyButton {
        AnyButton::Push(Button::new(parent, label))
    }

    /// Flat / borderless-looking push button (`BS_FLAT`).
    pub fn flat<W: crate::core::widget::Window>(parent: &W, label: &str) -> AnyButton {
        AnyButton::Push(Button::new_flat(parent, label))
    }

    /// Bitmap-only button (`wxBitmapButton`, `BS_BITMAP`).
    pub fn bitmap_only<W: crate::core::widget::Window>(
        parent: &W,
        bitmap: &Bitmap,
        w: i32,
        h: i32,
    ) -> AnyButton {
        AnyButton::Bitmap(BitmapButton::new(parent, bitmap, w, h))
    }

    /// Bitmap-only button from embedded SVG bytes.
    pub fn bitmap_only_svg<W: crate::core::widget::Window>(
        parent: &W,
        svg: &[u8],
        w: i32,
        h: i32,
    ) -> AnyButton {
        AnyButton::Bitmap(BitmapButton::new_from_svg_bytes(parent, svg, w, h))
    }

    /// Text button with a bitmap on the left (`BCM_SETIMAGELIST`).
    pub fn text_with_image_left<W: crate::core::widget::Window>(
        parent: &W,
        label: &str,
        svg: &[u8],
        icon_size: u32,
    ) -> AnyButton {
        AnyButton::Push(Button::new_with_svg_aligned(
            parent,
            label,
            svg,
            icon_size,
            BitmapAlign::Left,
        ))
    }

    /// Text button with a bitmap on the right.
    pub fn text_with_image_right<W: crate::core::widget::Window>(
        parent: &W,
        label: &str,
        svg: &[u8],
        icon_size: u32,
    ) -> AnyButton {
        AnyButton::Push(Button::new_with_svg_aligned(
            parent,
            label,
            svg,
            icon_size,
            BitmapAlign::Right,
        ))
    }

    /// Vista-style command link (`wxCommandLinkButton`).
    pub fn command_link<W: crate::core::widget::Window>(
        parent: &W,
        main: &str,
        note: &str,
    ) -> AnyButton {
        AnyButton::CommandLink(CommandLinkButton::new(parent, main, note))
    }

    /// Sticky toggle button (`wxToggleButton`).
    pub fn toggle<W: crate::core::widget::Window>(parent: &W, label: &str) -> AnyButton {
        AnyButton::Toggle(ToggleButton::new(parent, label))
    }

    /// Checkable bitmap button (`wxBitmapToggleButton`).
    pub fn bitmap_toggle<W: crate::core::widget::Window>(
        parent: &W,
        bitmap: &Bitmap,
        w: u32,
        h: u32,
    ) -> AnyButton {
        AnyButton::BitmapToggle(BitmapToggleButton::new(parent, bitmap, w, h))
    }

    /// Checkable bitmap button from embedded SVG bytes.
    pub fn bitmap_toggle_svg<W: crate::core::widget::Window>(
        parent: &W,
        svg: &[u8],
        size: u32,
    ) -> AnyButton {
        let bmp = Bitmap::from_svg_bytes(svg, size, size);
        AnyButton::BitmapToggle(BitmapToggleButton::new(parent, &bmp, size, size))
    }

    /// Button that pops up a menu (`wxMenuButton`). Call
    /// [`AnyButton::bind_menu_popup`] after adding menu items.
    pub fn menu_drop_down<W: crate::core::widget::Window>(parent: &W, label: &str) -> AnyButton {
        AnyButton::Menu(MenuButton::new(parent, label))
    }

    /// Animated bitmap button with built-in colour-cycle demo frames.
    pub fn animated_demo<W: crate::core::widget::Window>(parent: &W, frame: &Frame) -> AnyButton {
        AnyButton::Animated(AnimatedButton::demo(parent, frame))
    }
}
