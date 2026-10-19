//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Layout containers — the "organise a window" layer.
//!
//! Sizers and the container widgets that host other widgets:
//! * [`auto_form`] — `AutoForm`, form generate da uno `TableSchema`.
//! * [`book`] — `Choicebook`, `Listbook`, `Toolbook`, `Treebook`.
//! * [`grid`] — editable-cell `Grid`.
//! * [`grid_sizer`] — `GridSizer`, `FlexGridSizer`.
//! * [`scroll_bar`] — standalone scrollbar.
//! * [`scrolled_window`] — auto-scrollbar panel.
//! * [`scrollable_panel`] — panel with vertical scroll + auto-panning.
//! * [`sizer`] — `BoxSizer` and the `Orientation` enum.
//! * [`simplebook`] — `Simplebook` page container (one visible page, no tab bar).
//! * [`splitter_window`] — two-pane splitter with draggable sash.
//! * [`tab`] — tabbed notebook.

pub mod auto_form;
pub mod book;
pub mod data_grid;
pub mod data_view;
pub mod data_view_bitmap_renderer;
pub mod data_view_choice_renderer;
pub mod data_view_toggle_renderer;
pub mod grid;
pub mod grid_advanced;
pub mod grid_bag_sizer;
pub mod grid_block;
pub mod grid_cell_attr;
pub mod grid_cell_bool_editor;
pub mod grid_cell_bool_renderer;
pub mod grid_cell_choice_editor;
pub mod grid_cell_date_editor;
pub mod grid_cell_editor;
pub mod grid_cell_float_editor;
pub mod grid_cell_number_editor;
pub mod grid_cell_number_renderer;
pub mod grid_cell_renderer;
pub mod grid_cell_string_renderer;
pub mod grid_cell_text_editor;
pub mod grid_coords;
pub mod grid_form_link;
pub mod grid_icons;
pub mod grid_range;
pub mod grid_sizer;
pub mod grid_string_table;
pub mod grid_table;
pub mod scroll_bar;
pub mod scrollable_panel;
pub mod scrolled_window;
pub mod simplebook;
pub mod sizer;
pub mod sizer_flags;
pub mod sizer_item;
pub mod sizer_spacer;
pub mod splitter_window;
pub mod static_box_sizer;
pub mod static_sizer;
pub mod std_dialog_button_sizer;
pub mod tab;
pub mod wrap_sizer;
