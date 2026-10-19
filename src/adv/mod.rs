//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Advanced / specialised widgets that don't fit the standard control
//! taxonomy.
//!
//! * [`animation`] / [`animation_ctrl`] — animated GIF / APNG playback.
//! * [`media_ctrl`] — audio / video playback (MCI).
//! * [`property_grid`] — property / settings editor.
//! * [`tip_provider`] — [`TipProvider`](tip_provider::TipProvider) /
//!   [`show_tip`](tip_provider::show_tip) (C8): finestra tip a serie.
//! * [`wizard`] — multi-page setup wizard.

pub mod adv_ctrl_events;
pub mod animation;
pub mod animation_ctrl;
pub mod help_controller;
pub mod help_provider;
pub mod html_easy_printing;
pub mod html_link_event;
pub mod html_tag_handler;
pub mod html_window;
pub mod log_gui;
pub mod log_window;
pub mod media_ctrl;
pub mod property_grid;
pub mod property_grid_extras;
pub mod property_grid_iterator;
pub mod property_grid_manager;
pub mod rich_text_attr;
pub mod rich_text_buffer;
pub mod rich_text_ctrl;
pub mod rich_text_event;
pub mod rich_text_style;
pub mod rich_text_style_sheet;
pub mod simple_help_provider;
pub mod tip_provider;
pub mod web_view;
pub mod web_view_event;
pub mod web_view_handler;
pub mod wizard;
