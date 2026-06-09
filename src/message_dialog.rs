//! `wxMessageDialog` — class-based wrapper around [`message_box`].
//!
//! The free function [`message_box`] is a one-shot helper; this class
//! stores the message configuration and lets you show the dialog at
//! any time.
//!
//! # Example
//! ```no_run
//! use ru_wx::message_dialog::{MessageDialog, MessageDialogStyle, MessageDialogIcon};
//! use ru_wx::message_box::MessageBoxResult;
//! use ru_wx::frame::Frame;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let dlg = MessageDialog::new(
//!     &frame,
//!     "About this app",
//!     "MyApp 1.0\n\nA simple example.",
//!     MessageDialogStyle::Ok,
//!     MessageDialogIcon::Information,
//! );
//! let result = dlg.show_modal();
//! assert_eq!(result, MessageBoxResult::Ok);
//! ```

use crate::frame::Frame;
use crate::message_box::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxStyle};

/// Style of a message dialog. Mirrors [`MessageBoxStyle`].
pub type MessageDialogStyle = MessageBoxStyle;

/// Icon of a message dialog. Mirrors [`MessageBoxIcon`].
pub type MessageDialogIcon = MessageBoxIcon;

/// A modal message dialog.
pub struct MessageDialog<'a> {
    parent: &'a Frame,
    title: String,
    message: String,
    style: MessageDialogStyle,
    icon: MessageDialogIcon,
}

impl<'a> MessageDialog<'a> {
    /// Build a new message dialog.
    ///
    /// The dialog is not shown until [`MessageDialog::show_modal`] is
    /// called.
    pub fn new(
        parent: &'a Frame,
        title: &str,
        message: &str,
        style: MessageDialogStyle,
        icon: MessageDialogIcon,
    ) -> Self {
        MessageDialog {
            parent,
            title: title.to_string(),
            message: message.to_string(),
            style,
            icon,
        }
    }

    /// Update the message text. The dialog must be reshown for the
    /// change to take effect.
    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
    }

    /// Read the current message text.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Update the dialog title. The dialog must be reshown for the
    /// change to take effect.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Read the current dialog title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Update the button layout style.
    pub fn set_style(&mut self, style: MessageDialogStyle) {
        self.style = style;
    }

    /// Update the icon.
    pub fn set_icon(&mut self, icon: MessageDialogIcon) {
        self.icon = icon;
    }

    /// Show the dialog modally. Blocks until the user dismisses it.
    pub fn show_modal(&self) -> MessageBoxResult {
        message_box(
            self.parent,
            &self.message,
            &self.title,
            self.style,
            self.icon,
        )
    }
}
