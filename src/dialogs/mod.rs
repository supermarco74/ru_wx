//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Modal and modeless dialogs.
//!
//! Each dialog in this module is a self-contained "pop up a window,
//! get a result" unit. From a file picker to a colour wheel, they all
//! follow the same shape — `Some(value)` on OK, `None` on cancel.

pub mod about_dialog;
pub mod color_dialog;
pub mod date_picker_dialog;
pub mod dir_dialog;
pub mod file_dialog;
pub mod find_replace_dialog;
pub mod font_dialog;
pub mod message_box;
pub mod message_dialog;
pub mod progress_dialog;
pub mod rearrange_dialog;
pub mod rich_text_formatting_dialog;
pub mod credential_entry_dialog;
pub mod property_sheet_dialog;
pub mod single_choice_dialog;
pub mod symbol_picker_dialog;
pub mod text_entry_dialog;
