//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Interactive controls — the widgets the user can click, type into,
//! pick from, or watch.
//!
//! Every concrete control in `ru_wx` is in this module. They are
//! organised in alphabetical order (no sub-grouping); the AI_INDEX
//! table is the source of truth for "which control for which task".
//!
//! Static / non-interactive display widgets (labels, dividers, …) are
//! in here too, alongside the interactive ones.

pub mod activity_indicator;
pub mod add_remove_ctrl;
pub mod bitmap_button;
pub mod bitmap_toggle_button;
pub mod animated_button;
pub mod button;
pub mod button_variants;
pub mod calendar_ctrl;
pub mod calendar_date_attr;
pub mod collapsible_pane;
pub mod collapsible_header_ctrl;
pub mod combo_ctrl;
pub mod command_link_button;
pub mod context_help_button;
pub mod check_list_box;
pub mod checkbox;
pub mod choice;
pub mod colour_picker_ctrl;
pub mod combo_box;
pub mod date_picker_ctrl;
pub mod dir_picker_ctrl;
pub mod editable_list_box;
pub mod file_ctrl;
pub mod file_picker_ctrl;
pub mod font_picker_ctrl;
pub mod gauge;
pub mod generic_dir_ctrl;
pub mod hyperlink_ctrl;
pub mod ip_address_ctrl;
pub mod list_box;
pub mod list_ctrl;
pub mod menu_button;
pub mod popup_ctrl;
pub mod owner_drawn_combo_box;
pub mod simple_html_list_box;
pub mod tree_list_ctrl;
pub mod control_events;
pub mod radio_box;
pub mod rearrange_list;
pub mod radio_button;
pub mod search_ctrl;
pub mod slider;
pub mod spin_button;
pub mod spin_ctrl;
pub mod spin_ctrl_double;
pub mod static_bitmap;
pub mod static_box;
pub mod static_line;
pub mod static_text;
pub mod text_ctrl;
pub mod toggle_button;
pub mod tree_ctrl;
pub mod v_list_box;
