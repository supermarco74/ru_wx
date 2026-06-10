//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Core primitives shared across the UI toolkit.
//!
//! This is the lowest-level domain of `ru_wx` — types that every
//! other module depends on but that do not know about any specific
//! widget. The contents are:
//!
//! * [`accelerator`] — keyboard shortcut parsing.
//! * [`app`] — the `App` entry point and message-loop driver.
//! * [`busy_info`] — modal "please wait" overlay.
//! * [`dpi`] — per-monitor DPI queries and scaling helpers.
//! * [`font`] — font creation, measurement, drawing.
//! * [`geometry`] — `Rect`, `Colour`, `Point`, `Size`.
//! * [`log`] — pluggable multi-target / multi-level logger.
//! * [`timer`] — periodic timer on the message-loop thread.
//! * [`tooltip`] — non-modal tooltip attached to a widget.
//! * [`widget`] — the `Widget` / `Window` / `WidgetRef` traits every
//!   concrete type implements.

pub mod accelerator;
pub mod accelerator_table;
pub mod affine_matrix;
pub mod appearance;
pub mod array_string;
pub mod archive_entry;
pub mod archive_fs_handler;
pub mod tar_entry;
pub mod nc_hit_test_event;
pub mod query_layout_event;
pub mod calculate_layout_event;
pub mod cmdline_parser;
pub mod dynamic_library;
pub mod environment;
pub mod file_name;
pub mod internet_fs_handler;
pub mod long_long;
pub mod ulong_long;
pub mod regex;
pub mod text_buffer;
pub mod variant;
pub mod class_info;
pub mod ref_counter;
pub mod weak_ref;
pub mod window_update_locker;
pub mod event_filter;
pub mod translation;
pub mod scoped_ptr;
pub mod wx_any;
pub mod client_data;
pub mod array_int;
pub mod array_long;
pub mod array_double;
pub mod string_list;
pub mod geometry2d;
pub mod kill_focus_event;
pub mod set_focus_event;
pub mod nc_paint_event;
pub mod sys_command_event;
pub mod activate_app_event;
pub mod process_exit_event;
pub mod object_ref_data;
pub mod hash_set;
pub mod hash_map;
pub mod nc_calc_size_event;
pub mod mouse_capture_changed_event;
pub mod zip_entry;
pub mod path_env;
pub mod path_list;
pub mod sorted_array_string;
pub mod temp_dir;
pub mod text_file;
pub mod wx_dir;
pub mod wx_file;
pub mod datetime;
pub mod datetime_span;
pub mod platform_info;
pub mod string_tokenizer;
pub mod version_info;
pub mod zip_fs_handler;
pub mod app;
pub mod busy_info;
pub mod caret;
pub mod book_ctrl_event;
pub mod char_hook_event;
pub mod clipboard;
pub mod child_focus_event;
pub mod colour_database;
pub mod data_view_event;
pub mod display;
pub mod display_changed_event;
pub mod module;
pub mod object;
pub mod popup_window_event;
pub mod scroll_line_event;
pub mod filesystem_watcher_event;
pub mod container_events;
pub mod close_event;
pub mod command_event;
pub mod control_notify_events;
pub mod more_control_notify;
pub mod header_events;
pub mod progress_event;
pub mod property_grid_event;
pub mod wizard_event;
pub mod config;
pub mod context_help_event;
pub mod context_menu_event;
pub mod drop_files_event;
pub mod more_events;
pub mod busy_cursor;
pub mod cursor;
pub mod debug_context;
pub mod event_handler;
pub mod event_loop;
pub mod evt_loop_activator;
pub mod debug_report;
pub mod dir_traverser;
pub mod file_config;
pub mod file_ctrl_event;
pub mod file_system;
pub mod frame_events;
pub mod hyperlink_event;
pub mod item_container;
pub mod item_container_immutable;
pub mod memory_fs_handler;
pub mod notebook_event;
pub mod picker_events;
pub mod secret_store;
pub mod sizer_event;
pub mod temp_file;
pub mod init_dialog_event;
pub mod reg_config;
pub mod window_disabler;
pub mod message_queue;
pub mod menu_event;
pub mod process_util;
pub mod thread_util;
pub mod dpi;
pub mod font;
pub mod font_enumerator;
pub mod geometry;
pub mod mime_types;
pub mod mouse_wheel_event;
pub mod palette_events;
pub mod input_events;
pub mod mouse_events_ext;
pub mod mouse_state;
pub mod process_event;
pub mod scroll_win_event;
pub mod thread_helper;
pub mod thread_event;
pub mod uri;
pub mod window_lifecycle_events;
pub mod log;
pub mod sync_util;
pub mod system_settings;
pub mod timer;
pub mod timer_event;
pub mod tooltip;
pub mod validator;
pub mod widget;
pub mod window_events;
