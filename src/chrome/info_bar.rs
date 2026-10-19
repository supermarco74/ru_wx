//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Information banner (`wxInfoBar`): composto da icona di severità
//! (glifo Segoe MDL2 Assets), etichetta di testo e pulsante `×`.
//! La severità è esposta anche come tooltip esteso con i dettagli.

use std::rc::Rc;

use crate::controls::button::Button;
use crate::controls::static_text::StaticText;
use crate::core::font::{Font, FontDesc};
use crate::core::geometry::Colour;
use crate::core::tokens::{FONT_SIZE, SPACING_L, SPACING_XS};
use crate::core::tooltip::ToolTip;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

/// Lato della casella dell'icona (glifo): deriva dai token di spazio.
const GLYPH_BOX: i32 = SPACING_L + SPACING_XS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoBarMessageType {
    Info,
    Warning,
    Error,
}

/// Glifo Segoe MDL2 Assets per ciascuna severità (a sinistra del testo).
fn glyph_for(kind: InfoBarMessageType) -> &'static str {
    match kind {
        InfoBarMessageType::Info => "\u{e946}",    // Information
        InfoBarMessageType::Warning => "\u{e7ba}", // Warning
        InfoBarMessageType::Error => "\u{e783}",   // Error
    }
}

/// Colore del glifo, coerente con [`InfoBar::background_colour`].
fn severity_colour(kind: InfoBarMessageType) -> Colour {
    match kind {
        InfoBarMessageType::Info => Colour::new(0, 120, 215, 255), // blu info
        InfoBarMessageType::Warning => Colour::new(191, 121, 0, 255), // ambra warning
        InfoBarMessageType::Error => Colour::new(196, 43, 28, 255), // rosso error
    }
}

#[derive(Clone)]
pub struct InfoBar {
    label: StaticText,
    icon: StaticText,
    dismiss: Button,
    kind: InfoBarMessageType,
    /// Font "Segoe MDL2 Assets" condiviso: deve restare vivo finché il
    /// controllo lo usa (`WM_SETFONT` conserva l'`HFONT`).
    icon_font: Rc<Font>,
    /// Tooltip esteso con i dettagli (opzionale).
    details: Option<ToolTip>,
    /// Handle nativo del contenitore padre: serve per applicare il
    /// colore dell'icona tramite `WM_CTLCOLORSTATIC`.
    parent_hwnd: crate::platform::NativeHandle,
}

impl InfoBar {
    pub fn new<W: Window>(parent: &W) -> Self {
        let icon = StaticText::new(parent, glyph_for(InfoBarMessageType::Info));
        // Casella fissa per il glifo: il sizer conserva questa size.
        icon.as_widget_ref()
            .borrow_mut()
            .set_size(GLYPH_BOX as u32, GLYPH_BOX as u32);
        let icon_font = Rc::new(Font::new(FontDesc::new("Segoe MDL2 Assets", FONT_SIZE)));
        icon.set_font(&icon_font);
        let mut this = Self {
            label: StaticText::new(parent, ""),
            icon,
            dismiss: Button::new(parent, "×"),
            kind: InfoBarMessageType::Info,
            icon_font,
            details: None,
            parent_hwnd: crate::platform::window_handle(parent),
        };
        this.apply_icon_style();
        this
    }

    pub fn show_message(&mut self, text: &str, kind: InfoBarMessageType) {
        self.kind = kind;
        self.label.set_label(text);
        self.apply_icon_style();
    }

    /// Come [`show_message`](Self::show_message), ma aggiunge `details`
    /// come tooltip esteso (testo prolungato) sull'etichetta.
    pub fn show_message_with_details(
        &mut self,
        text: &str,
        kind: InfoBarMessageType,
        details: &str,
    ) {
        self.show_message(text, kind);
        match self.details.take() {
            Some(tip) => {
                // Il tooltip è già agganciato: aggiorna il testo live.
                tip.set_text(details);
                self.details = Some(tip);
            }
            None => {
                let tip = ToolTip::new(details);
                tip.attach(&self.label.as_widget_ref());
                self.details = Some(tip);
            }
        }
    }

    pub fn bind_dismiss(&self, frame: &Frame) {
        let label = self.label.clone();
        let icon = self.icon.clone();
        let bid = self.dismiss.id();
        frame.register_command_handler(
            bid,
            Box::new(move || {
                label.set_label("");
                icon.set_label("");
            }),
        );
    }

    /// Dismiss with [`InfoBarEvent`](crate::InfoBarEvent) payload (`wxInfoBarEvent`).
    pub fn on_info_bar_event<F: FnMut(&crate::chrome::info_bar_event::InfoBarEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let kind = self.kind;
        let bid = self.dismiss.id();
        frame.register_command_handler(
            bid,
            Box::new(move || {
                f(&crate::chrome::info_bar_event::InfoBarEvent::new(
                    crate::chrome::info_bar_event::InfoBarEventKind::Dismissed,
                    kind,
                    "",
                ));
            }),
        );
    }

    pub fn background_colour(&self) -> Colour {
        match self.kind {
            InfoBarMessageType::Info => Colour::new(220, 235, 255, 255),
            InfoBarMessageType::Warning => Colour::new(255, 245, 200, 255),
            InfoBarMessageType::Error => Colour::new(255, 220, 220, 255),
        }
    }

    pub fn label_widget(&self) -> WidgetRef {
        self.label.as_widget_ref()
    }

    /// Icona di severità a sinistra del testo (glifo Segoe MDL2 Assets).
    pub fn icon_widget(&self) -> WidgetRef {
        self.icon.as_widget_ref()
    }

    pub fn dismiss_widget(&self) -> WidgetRef {
        self.dismiss.as_widget_ref()
    }

    /// Aggiorna glifo e colore dell'icona in base alla severità corrente.
    fn apply_icon_style(&mut self) {
        self.icon.set_label(glyph_for(self.kind));
        let colour = severity_colour(self.kind);
        // macOS: colore diretto sul label.
        self.icon.set_foreground_colour(colour);
        // Windows: il colore del STATIC passa da WM_CTLCOLORSTATIC sul
        // padre, intercettato dal subclass del frame (hint per-widget).
        #[cfg(target_os = "windows")]
        {
            use crate::platform::winui3::subclass::{install_frame_subclass, set_foreground};
            // `InvalidateRect` vive in `Graphics::Gdi` (non in
            // `WindowsAndMessaging`) in windows-sys 0.59.
            use windows_sys::Win32::Graphics::Gdi::InvalidateRect;

            let icon_hwnd = self.icon.as_widget_ref().borrow().native_handle();
            if icon_hwnd != 0 {
                install_frame_subclass(self.parent_hwnd as _);
                set_foreground(icon_hwnd as _, colour.to_colorref());
                unsafe {
                    InvalidateRect(icon_hwnd as _, std::ptr::null(), 1);
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = self.parent_hwnd;
    }
}
