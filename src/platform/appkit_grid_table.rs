//! macOS Grid backed by native [`NSTableView`] (view-based cells).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::NSDraggingInfo;
use objc2_app_kit::{
    NSControlSize, NSControlTextEditingDelegate, NSDragOperation, NSEvent, NSEventModifierFlags,
    NSImageScaling, NSImageView, NSLevelIndicator, NSLevelIndicatorStyle, NSPasteboard,
    NSPasteboardTypeString, NSProgressIndicator, NSProgressIndicatorStyle, NSScrollView,
    NSTableCellView, NSTableColumn, NSTableColumnResizingOptions, NSTableView,
    NSTableViewDataSource, NSTableViewDelegate, NSTableViewDropOperation, NSTableViewGridLineStyle,
    NSTableViewSelectionHighlightStyle, NSTableViewStyle, NSTextAlignment, NSTextField,
    NSUserInterfaceItemIdentification, NSView, NSViewBoundsDidChangeNotification,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSIndexSet, NSInteger, NSMutableIndexSet, NSNotFound,
    NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSPoint, NSRange, NSRect,
    NSSize, NSSortDescriptor, NSString, NSUInteger,
};

use super::{
    alloc_widget_handle, dispatch_notify, grid_append_column_kind_slots, grid_body_font,
    grid_cell_editing_enabled, grid_cell_progress_kind, grid_clone_root_view, grid_col_count,
    grid_col_sort_key, grid_frozen_col_count, grid_frozen_row_count, grid_get_focused_cell,
    grid_header_title, grid_image_list_icon_width, grid_init_cell_kinds, grid_is_column_hidden,
    grid_link_clicks_enabled, grid_queue_edit_start, grid_queue_link_click, grid_queue_paste_at,
    grid_record_cell_edited, grid_record_row_reordered, grid_resolve_cell_tooltip,
    grid_resolve_style, grid_row_col_highlight_enabled, grid_row_count, grid_row_drag_enabled,
    grid_row_height, grid_row_label_width, grid_row_labels_enabled, grid_set_field_alignment,
    grid_set_focused_cell, image_list_image_at, ns_color_from_rgba, parent_view_for, record_parent,
    resolve_frame_handle, ru_rect_to_ns, store_ns_view, GridAppearanceData, GridNativeKind,
    GridProgressCell, NativeHandle, Rect, GRID_ACTIVATE_CELL, GRID_ALTERNATING, GRID_APPEARANCE,
    GRID_CELL_STYLES, GRID_CHECKBOXES, GRID_COLUMNS, GRID_COLUMN_RESIZED, GRID_CONTEXT_MENU,
    GRID_FONT, GRID_HIGHLIGHT_CLICK, GRID_HIGHLIGHT_COL, GRID_HIGHLIGHT_ROW, GRID_IMAGE_LISTS,
    GRID_MIN_COLUMN_WIDTH, GRID_ROWS, GRID_ROW_CHECKED, GRID_ROW_COL_HIGHLIGHT, GRID_ROW_STYLES,
    GRID_SELECTED, GRID_SHOW_LINES, GRID_SORT, GRID_SORT_SYNCING, LISTCTRL_NOTIFY_ITEM_CHANGED,
};
use objc2_app_kit::{NSBorderType, NSScrollView as AppKitScrollView};

/// Inline editor parts for a grid cell: field plus its row/column position.
type GridInlineEditor = (Retained<NSTextField>, usize, usize);

thread_local! {
    static GRID_TABLES: RefCell<HashMap<NativeHandle, Retained<GridTableView>>> =
        RefCell::new(HashMap::new());
    static GRID_DELEGATES: RefCell<HashMap<NativeHandle, Retained<GridTableController>>> =
        RefCell::new(HashMap::new());
    static GRID_INLINE_EDITORS: RefCell<HashMap<NativeHandle, GridInlineEditor>> =
        RefCell::new(HashMap::new());
    static GRID_INLINE_FRAME: RefCell<HashMap<NativeHandle, (NativeHandle, u16)>> =
        RefCell::new(HashMap::new());
    static GRID_MAIN_SCROLLS: RefCell<HashMap<NativeHandle, Retained<NSScrollView>>> =
        RefCell::new(HashMap::new());
    static GRID_FROZEN_COL_TABLES: RefCell<HashMap<NativeHandle, Retained<GridTableView>>> =
        RefCell::new(HashMap::new());
    static GRID_FROZEN_COL_SCROLLS: RefCell<HashMap<NativeHandle, Retained<NSScrollView>>> =
        RefCell::new(HashMap::new());
    static GRID_FROZEN_ROW_TOP_TABLES: RefCell<HashMap<NativeHandle, Retained<GridTableView>>> =
        RefCell::new(HashMap::new());
    static GRID_FROZEN_ROW_TOP_SCROLLS: RefCell<HashMap<NativeHandle, Retained<NSScrollView>>> =
        RefCell::new(HashMap::new());
}

const GRID_CHECKBOX_COL_ID: &str = "__checkbox__";
const GRID_ROW_LABEL_COL_ID: &str = "__rowlabel__";
const GRID_ROW_LABEL_CELL_ID: &str = "RuWxGridRowLabelCell";
const GRID_DATA_CELL_ID: &str = "RuWxGridDataCell";
const GRID_PROGRESS_CELL_ID: &str = "RuWxGridProgressCell";
const GRID_PRIORITY_CELL_ID: &str = "RuWxGridPriorityCell";
const GRID_CHECK_CELL_ID: &str = "RuWxGridCheckCell";
const GRID_CELL_ICON_WIDTH: f64 = 16.0;
const GRID_SORT_INDICATOR_PAD: f64 = 14.0;
const GRID_ROW_HEIGHT_MAX: f64 = 240.0;
const GRID_ROW_CELL_VPAD: f64 = 4.0;

fn grid_table_leading_cols(handle: NativeHandle) -> usize {
    let mut n = 0usize;
    if grid_checkboxes_enabled(handle) {
        n += 1;
    }
    if grid_row_labels_enabled(handle) {
        n += 1;
    }
    n
}

fn grid_row_reference_table_column(handle: NativeHandle) -> NSInteger {
    if grid_checkboxes_enabled(handle) || grid_row_labels_enabled(handle) {
        0
    } else {
        grid_table_leading_cols(handle) as NSInteger
    }
}

fn grid_is_row_reference_table_column(handle: NativeHandle, table_col: NSInteger) -> bool {
    table_col >= 0 && table_col == grid_row_reference_table_column(handle)
}

fn grid_data_col_for_table_index(
    _handle: NativeHandle,
    table: &NSTableView,
    table_col: NSInteger,
) -> Option<usize> {
    if table_col < 0 {
        return None;
    }
    let table_column = table.tableColumns().objectAtIndex(table_col as NSUInteger);
    grid_data_col_for_table_column(&table_column)
}

fn grid_dispatch_highlight_click(
    frame_handle: NativeHandle,
    command_id: u16,
    handle: NativeHandle,
    row: Option<usize>,
    col: Option<usize>,
) {
    GRID_HIGHLIGHT_CLICK.with(|map| {
        map.borrow_mut().insert(handle, (row, col));
    });
    dispatch_notify(frame_handle, command_id, GRID_NOTIFY_HIGHLIGHT_CLICK);
}

pub fn grid_apply_row_column_highlights(grid_handle: NativeHandle) {
    use super::{GRID_HIGHLIGHT_COL, GRID_HIGHLIGHT_ROW};
    let (row, col) = (
        GRID_HIGHLIGHT_ROW.with(|m| m.borrow().get(&grid_handle).and_then(|r| *r)),
        GRID_HIGHLIGHT_COL.with(|m| m.borrow().get(&grid_handle).and_then(|c| *c)),
    );
    let frozen_cols = grid_frozen_col_count(grid_handle);
    for table in grid_all_table_views(grid_handle) {
        let highlighted = col.and_then(|data_col| {
            if grid_table_is_frozen_col(grid_handle, &table) {
                if data_col < frozen_cols {
                    grid_table_column_on_table(grid_handle, &table, data_col)
                } else {
                    None
                }
            } else if frozen_cols > 0 && data_col < frozen_cols {
                None
            } else {
                grid_table_column_on_table(grid_handle, &table, data_col)
            }
        });
        table.setHighlightedTableColumn(highlighted.as_deref());
        let _ = row;
    }
    grid_reload(grid_handle);
}

fn grid_set_column_sort_prototype(col: &NSTableColumn, data_col: usize) {
    let key = NSString::from_str(&grid_col_sort_key(data_col));
    let proto = NSSortDescriptor::sortDescriptorWithKey_ascending(Some(&key), true);
    col.setSortDescriptorPrototype(Some(&proto));
}

fn grid_table_column_on_table(
    grid_handle: NativeHandle,
    table: &NSTableView,
    data_col: usize,
) -> Option<Retained<NSTableColumn>> {
    let table_col_idx = grid_table_leading_cols(grid_handle) + data_col;
    let cols = table.tableColumns();
    if table_col_idx >= cols.count() {
        return None;
    }
    Some(cols.objectAtIndex(table_col_idx as NSUInteger))
}

fn grid_apply_sort_descriptors(grid_handle: NativeHandle) {
    let (sort_col, asc) = GRID_SORT.with(|m| {
        m.borrow()
            .get(&grid_handle)
            .map(|s| *s.borrow())
            .unwrap_or((None, None))
    });
    GRID_SORT_SYNCING.with(|m| {
        m.borrow_mut().insert(grid_handle, true);
    });
    for table in grid_all_table_views(grid_handle) {
        if let (Some(col), Some(ascending)) = (sort_col, asc) {
            let key = NSString::from_str(&grid_col_sort_key(col));
            let desc = NSSortDescriptor::sortDescriptorWithKey_ascending(Some(&key), ascending);
            let arr = NSArray::from_retained_slice(&[desc]);
            table.setSortDescriptors(&arr);
        } else {
            let empty = NSArray::<NSSortDescriptor>::from_retained_slice(&[]);
            table.setSortDescriptors(&empty);
        }
    }
    GRID_SORT_SYNCING.with(|m| {
        m.borrow_mut().insert(grid_handle, false);
    });
}

pub const GRID_NOTIFY_ITEM_CHANGED: u32 = LISTCTRL_NOTIFY_ITEM_CHANGED;
pub const GRID_NOTIFY_HEADER_CLICK: u32 = 0xFFFFFF94;
pub const GRID_NOTIFY_HIGHLIGHT_CLICK: u32 = 0xFFFFFF91;
pub const GRID_NOTIFY_ITEM_ACTIVATE: u32 = 0xFFFFFF96;
pub const GRID_NOTIFY_CONTEXT_MENU: u32 = 0xFFFFFF93;
pub const GRID_NOTIFY_COLUMN_RESIZED: u32 = 0xFFFFFF92;
pub const GRID_NOTIFY_FOCUS_CHANGED: u32 = 0xFFFFFF95;
pub const GRID_NOTIFY_CELL_EDITED: u32 = 0xFFFFFF97;
pub const GRID_NOTIFY_PASTE_REQUEST: u32 = 0xFFFFFF98;
pub const GRID_NOTIFY_EDIT_START: u32 = 0xFFFFFF99;
pub const GRID_NOTIFY_ROW_REORDERED: u32 = 0xFFFFFF90;
pub const GRID_NOTIFY_LINK_CLICK: u32 = 0xFFFFFF8F;

const GRID_PANE_MAIN: u8 = 0;
const GRID_PANE_FROZEN_COL: u8 = 1;
const GRID_PANE_FROZEN_ROW_TOP: u8 = 2;

fn grid_checkboxes_enabled(handle: NativeHandle) -> bool {
    super::GRID_CHECKBOXES.with(|m| m.borrow().get(&handle).copied().unwrap_or(false))
}

fn grid_col_identifier(col: usize) -> Retained<NSString> {
    NSString::from_str(&format!("col_{col}"))
}

fn grid_data_col_for_table_column(table_column: &NSTableColumn) -> Option<usize> {
    let id = table_column.identifier().to_string();
    if id == GRID_CHECKBOX_COL_ID || id == GRID_ROW_LABEL_COL_ID {
        return None;
    }
    id.strip_prefix("col_")?.parse().ok()
}

fn grid_table_column_at(
    grid_handle: NativeHandle,
    data_col: usize,
) -> Option<Retained<NSTableColumn>> {
    GRID_TABLES.with(|map| {
        let table = map.borrow().get(&grid_handle)?.clone();
        let table_col_idx = grid_table_leading_cols(grid_handle) + data_col;
        let cols = table.tableColumns();
        if table_col_idx >= cols.count() {
            return None;
        }
        Some(cols.objectAtIndex(table_col_idx as NSUInteger))
    })
}

fn grid_apply_table_style(grid_handle: NativeHandle) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    let appearance = super::grid_appearance(grid_handle);
    let show_lines =
        super::GRID_SHOW_LINES.with(|m| m.borrow().get(&grid_handle).copied().unwrap_or(true));
    let grid_mask = if show_lines {
        NSTableViewGridLineStyle::SolidVerticalGridLineMask
            | NSTableViewGridLineStyle::SolidHorizontalGridLineMask
    } else {
        NSTableViewGridLineStyle::GridNone
    };
    let row_h = grid_row_height(grid_handle);
    for table in grid_all_table_views(grid_handle) {
        table.setGridStyleMask(grid_mask);
        table.setUsesAlternatingRowBackgroundColors(false);
        table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::None);
        table.setBackgroundColor(&ns_color_from_rgba(if appearance.system_theme {
            (255, 255, 255, 255)
        } else {
            appearance.alternating_odd
        }));
        table.setRowHeight(row_h);
    }
    let _ = mtm;
}

pub fn grid_reload(grid_handle: NativeHandle) {
    grid_apply_table_style(grid_handle);
    grid_sync_column_titles(grid_handle);
    for table in grid_all_table_views(grid_handle) {
        table.reloadData();
    }
    grid_note_row_heights_changed(grid_handle);
}

pub fn grid_note_row_heights_changed(grid_handle: NativeHandle) {
    for table in grid_all_table_views(grid_handle) {
        let count = grid_effective_row_count(grid_handle, &table);
        if count == 0 {
            continue;
        }
        let set = NSIndexSet::indexSetWithIndexesInRange(NSRange {
            location: 0,
            length: count as NSUInteger,
        });
        table.noteHeightOfRowsWithIndexesChanged(&set);
    }
    if let Some((w, h)) = super::native_view_size(grid_handle) {
        grid_sync_view_geometry(grid_handle, w, h);
    }
}

/// Repaint specific display rows without recalculating row heights.
pub fn grid_reload_display_rows(grid_handle: NativeHandle, display_rows: &[usize]) {
    if display_rows.is_empty() {
        return;
    }
    for table in grid_all_table_views(grid_handle) {
        let row_set = build_view_row_index_set(grid_handle, &table, display_rows);
        if row_set.count() == 0 {
            continue;
        }
        let col_count = table.tableColumns().count();
        if col_count == 0 {
            continue;
        }
        let col_set = NSIndexSet::indexSetWithIndexesInRange(NSRange {
            location: 0,
            length: col_count,
        });
        table.reloadDataForRowIndexes_columnIndexes(&row_set, &col_set);
    }
}

fn build_view_row_index_set(
    grid_handle: NativeHandle,
    table: &NSTableView,
    display_rows: &[usize],
) -> Retained<NSIndexSet> {
    let set = NSMutableIndexSet::new();
    for &display_row in display_rows {
        if grid_table_is_frozen_row_top(grid_handle, table) {
            let frozen = grid_frozen_row_count(grid_handle);
            if display_row < frozen {
                set.addIndex(display_row as NSUInteger);
            }
            continue;
        }
        if grid_frozen_row_count(grid_handle) > 0 && !grid_table_is_frozen_col(grid_handle, table) {
            let frozen = grid_frozen_row_count(grid_handle);
            if display_row >= frozen {
                set.addIndex((display_row - frozen) as NSUInteger);
            }
            continue;
        }
        set.addIndex(display_row as NSUInteger);
    }
    Retained::into_super(set)
}

fn grid_all_table_views(grid_handle: NativeHandle) -> Vec<Retained<GridTableView>> {
    let mut out = Vec::new();
    GRID_TABLES.with(|map| {
        if let Some(table) = map.borrow().get(&grid_handle) {
            out.push(table.clone());
        }
    });
    GRID_FROZEN_COL_TABLES.with(|map| {
        if let Some(table) = map.borrow().get(&grid_handle) {
            out.push(table.clone());
        }
    });
    GRID_FROZEN_ROW_TOP_TABLES.with(|map| {
        if let Some(table) = map.borrow().get(&grid_handle) {
            out.push(table.clone());
        }
    });
    out
}

fn grid_table_pane(table: &NSTableView) -> u8 {
    if let Some(gtv) = table.downcast_ref::<GridTableView>() {
        gtv.ivars().pane.get()
    } else {
        GRID_PANE_MAIN
    }
}

fn grid_table_is_frozen_col(_grid_handle: NativeHandle, table: &NSTableView) -> bool {
    grid_table_pane(table) == GRID_PANE_FROZEN_COL
}

fn grid_table_is_frozen_row_top(_grid_handle: NativeHandle, table: &NSTableView) -> bool {
    grid_table_pane(table) == GRID_PANE_FROZEN_ROW_TOP
}

fn grid_effective_row_count(grid_handle: NativeHandle, table: &NSTableView) -> usize {
    let total = grid_row_count(grid_handle);
    let frozen_rows = grid_frozen_row_count(grid_handle);
    if grid_table_is_frozen_row_top(grid_handle, table) {
        return frozen_rows.min(total);
    }
    if frozen_rows > 0 && !grid_table_is_frozen_col(grid_handle, table) {
        return total.saturating_sub(frozen_rows);
    }
    total
}

fn grid_map_view_row(grid_handle: NativeHandle, table: &NSTableView, view_row: usize) -> usize {
    let frozen_rows = grid_frozen_row_count(grid_handle);
    if frozen_rows > 0
        && !grid_table_is_frozen_col(grid_handle, table)
        && !grid_table_is_frozen_row_top(grid_handle, table)
    {
        view_row + frozen_rows
    } else {
        view_row
    }
}

fn grid_display_row_to_view_row(
    grid_handle: NativeHandle,
    table: &NSTableView,
    display_row: usize,
) -> usize {
    let frozen_rows = grid_frozen_row_count(grid_handle);
    if frozen_rows > 0
        && !grid_table_is_frozen_col(grid_handle, table)
        && !grid_table_is_frozen_row_top(grid_handle, table)
    {
        display_row.saturating_sub(frozen_rows)
    } else {
        display_row
    }
}

fn grid_editing_table_view(
    grid_handle: NativeHandle,
    display_row: usize,
    data_col: usize,
) -> Option<Retained<GridTableView>> {
    let frozen_cols = grid_frozen_col_count(grid_handle);
    let frozen_rows = grid_frozen_row_count(grid_handle);
    if frozen_cols > 0 && data_col < frozen_cols {
        return GRID_FROZEN_COL_TABLES.with(|m| m.borrow().get(&grid_handle).cloned());
    }
    if frozen_rows > 0 && display_row < frozen_rows {
        return GRID_FROZEN_ROW_TOP_TABLES.with(|m| m.borrow().get(&grid_handle).cloned());
    }
    GRID_TABLES.with(|m| m.borrow().get(&grid_handle).cloned())
}

fn grid_next_visible_col(
    handle: NativeHandle,
    from: usize,
    delta: i32,
    ncols: usize,
) -> Option<usize> {
    if delta == 0 {
        return if super::grid_is_column_hidden(handle, from) {
            None
        } else {
            Some(from)
        };
    }
    let step = if delta > 0 { 1i64 } else { -1i64 };
    let mut c = from as i64 + delta as i64;
    while c >= 0 && (c as usize) < ncols {
        let idx = c as usize;
        if !super::grid_is_column_hidden(handle, idx) {
            return Some(idx);
        }
        c += step;
    }
    None
}

fn grid_sync_column_titles(grid_handle: NativeHandle) {
    let columns = GRID_COLUMNS.with(|m| {
        m.borrow()
            .get(&grid_handle)
            .map(|c| c.borrow().clone())
            .unwrap_or_default()
    });
    for table in grid_all_table_views(grid_handle) {
        let frozen_cols = grid_frozen_col_count(grid_handle);
        let is_frozen_col = grid_table_is_frozen_col(grid_handle, &table);
        for (i, (title, _, _)) in columns.iter().enumerate() {
            if is_frozen_col && i >= frozen_cols {
                continue;
            }
            if !is_frozen_col && frozen_cols > 0 && i < frozen_cols {
                continue;
            }
            if let Some(col) = grid_table_column_on_table(grid_handle, &table, i) {
                col.setTitle(&NSString::from_str(title));
            }
        }
    }
}

fn grid_rebuild_table_columns(grid_handle: NativeHandle) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    GRID_TABLES.with(|map| {
        let Some(table) = map.borrow().get(&grid_handle).cloned() else {
            return;
        };
        while table.numberOfColumns() > 0 {
            let cols = table.tableColumns();
            if let Some(col) = cols.firstObject() {
                table.removeTableColumn(&col);
            } else {
                break;
            }
        }
        if grid_checkboxes_enabled(grid_handle) {
            let col = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &NSString::from_str(GRID_CHECKBOX_COL_ID),
            );
            col.setTitle(&NSString::from_str(""));
            col.setWidth(22.0);
            col.setMinWidth(22.0);
            col.setMaxWidth(28.0);
            col.setResizingMask(NSTableColumnResizingOptions::NoResizing);
            table.addTableColumn(&col);
        }
        if grid_row_labels_enabled(grid_handle) {
            let w = grid_row_label_width(grid_handle).max(20) as f64;
            let col = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &NSString::from_str(GRID_ROW_LABEL_COL_ID),
            );
            col.setTitle(&NSString::from_str("#"));
            col.setWidth(w);
            col.setMinWidth(20.0);
            col.setMaxWidth(80.0);
            col.setResizingMask(NSTableColumnResizingOptions::NoResizing);
            table.addTableColumn(&col);
        }
        let columns = GRID_COLUMNS.with(|m| {
            m.borrow()
                .get(&grid_handle)
                .map(|c| c.borrow().clone())
                .unwrap_or_default()
        });
        for (i, (title, width, _align)) in columns.iter().enumerate() {
            let col = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &grid_col_identifier(i),
            );
            col.setTitle(&NSString::from_str(title));
            grid_set_column_sort_prototype(&col, i);
            let w = if *width > 0 { *width as f64 } else { 80.0 };
            col.setWidth(w);
            col.setMinWidth(GRID_MIN_COLUMN_WIDTH as f64);
            col.setResizingMask(NSTableColumnResizingOptions::UserResizingMask);
            if grid_is_column_hidden(grid_handle, i) {
                col.setHidden(true);
            }
            let frozen = grid_frozen_col_count(grid_handle);
            if frozen > 0 && i < frozen {
                col.setHidden(true);
            }
            table.addTableColumn(&col);
        }
    });
}

fn grid_cell_display_text(handle: NativeHandle, row: usize, data_col: usize) -> String {
    GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&handle) {
            rows.borrow()
                .get(row)
                .and_then(|(texts, _)| texts.get(data_col).cloned())
                .unwrap_or_default()
        } else {
            String::new()
        }
    })
}

fn grid_cell_image_index(handle: NativeHandle, row: usize, data_col: usize) -> Option<i32> {
    GRID_ROWS.with(|map| {
        let map_b = map.borrow();
        let rows_cell = map_b.get(&handle)?;
        let rows = rows_cell.borrow();
        let (_, images) = rows.get(row)?;
        images
            .get(data_col)
            .and_then(|i| *i)
            .filter(|idx| *idx >= 0)
    })
}

fn grid_column_align(handle: NativeHandle, data_col: usize) -> i32 {
    GRID_COLUMNS.with(|map| {
        map.borrow()
            .get(&handle)
            .and_then(|c| c.borrow().get(data_col).map(|(_, _, a)| *a))
            .unwrap_or(0)
    })
}

fn grid_stored_column_width(handle: NativeHandle, data_col: usize) -> f64 {
    GRID_COLUMNS.with(|map| {
        map.borrow()
            .get(&handle)
            .and_then(|c| c.borrow().get(data_col).map(|(_, w, _)| *w as f64))
            .unwrap_or(80.0)
    })
}

fn grid_column_width_for_measure(handle: NativeHandle, data_col: usize) -> f64 {
    grid_stored_column_width(handle, data_col).max(4.0)
}

fn grid_measure_multiline_text_height(
    mtm: MainThreadMarker,
    handle: NativeHandle,
    text: &str,
    width: f64,
) -> f64 {
    let base = grid_row_height(handle);
    let font = grid_body_font(mtm, handle);
    let line_h = font.boundingRectForFont().size.height.max(1.0);
    let mut line_count = 0.0;
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            line_count += 1.0;
        } else {
            let w = grid_measure_text_width(mtm, handle, paragraph);
            line_count += (w / width).ceil().max(1.0);
        }
    }
    (line_count * line_h + GRID_ROW_CELL_VPAD).clamp(base, GRID_ROW_HEIGHT_MAX)
}

fn grid_cell_content_height(
    mtm: MainThreadMarker,
    handle: NativeHandle,
    row: usize,
    data_col: usize,
) -> f64 {
    let base = grid_row_height(handle);
    if grid_is_column_hidden(handle, data_col) {
        return base;
    }
    if grid_cell_progress_kind(handle, row, data_col).is_some() {
        return base;
    }
    let text = grid_cell_display_text(handle, row, data_col);
    let display = text.replace('\r', "\n");
    if !display.contains('\n') {
        return base;
    }
    let col_width = grid_column_width_for_measure(handle, data_col);
    let icon_w = if grid_cell_image_index(handle, row, data_col).is_some() {
        grid_image_list_icon_width(handle).unwrap_or(GRID_CELL_ICON_WIDTH) + 3.0
    } else {
        2.0
    };
    let text_w = (col_width - icon_w - 2.0).max(1.0);
    grid_measure_multiline_text_height(mtm, handle, &display, text_w)
}

fn grid_row_display_height(
    mtm: MainThreadMarker,
    handle: NativeHandle,
    table: &NSTableView,
    view_row: usize,
) -> f64 {
    let base = grid_row_height(handle);
    let logical_row = grid_map_view_row(handle, table, view_row);
    let col_count = grid_col_count(handle);
    let mut max_h = base;
    for data_col in 0..col_count {
        let h = grid_cell_content_height(mtm, handle, logical_row, data_col);
        max_h = max_h.max(h);
    }
    max_h
}

fn grid_document_body_height(
    mtm: MainThreadMarker,
    handle: NativeHandle,
    table: &NSTableView,
) -> f64 {
    let count = grid_effective_row_count(handle, table);
    (0..count)
        .map(|view_row| grid_row_display_height(mtm, handle, table, view_row))
        .sum()
}

pub(super) fn grid_measure_text_width(
    mtm: MainThreadMarker,
    handle: NativeHandle,
    text: &str,
) -> f64 {
    let font = grid_body_font(mtm, handle);
    let adv = font.maximumAdvancement().width;
    (text.chars().count() as f64 * adv).max(adv)
}

fn grid_data_column_order(grid_handle: NativeHandle) -> Vec<usize> {
    GRID_TABLES.with(|map| {
        let Some(table) = map.borrow().get(&grid_handle).cloned() else {
            return Vec::new();
        };
        let cols = table.tableColumns();
        let count = cols.count();
        let mut order = Vec::new();
        for i in 0..count {
            let col = cols.objectAtIndex(i);
            if let Some(data_col) = grid_data_col_for_table_column(&col) {
                order.push(data_col);
            }
        }
        order
    })
}

fn grid_sync_selected_rows_from_table(grid_handle: NativeHandle, table: &NSTableView) {
    let rows = index_set_to_rows(table.selectedRowIndexes());
    GRID_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&grid_handle) {
            *sel.borrow_mut() = rows;
        }
    });
}

fn index_set_to_rows(indexes: Retained<NSIndexSet>) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut idx = indexes.firstIndex();
    let not_found = NSNotFound as NSUInteger;
    while idx != not_found {
        rows.push(idx);
        let next = indexes.indexGreaterThanIndex(idx);
        if next == not_found {
            break;
        }
        idx = next;
    }
    rows
}

fn grid_autosize_column_width(
    mtm: MainThreadMarker,
    grid_handle: NativeHandle,
    data_col: usize,
    include_header: bool,
) -> i32 {
    let mut max_w = GRID_MIN_COLUMN_WIDTH as f64;
    if include_header {
        let title = GRID_COLUMNS.with(|map| {
            map.borrow()
                .get(&grid_handle)
                .and_then(|c| c.borrow().get(data_col).map(|(t, _, _)| t.clone()))
                .unwrap_or_default()
        });
        let header = grid_header_title(grid_handle, data_col, &title);
        max_w = max_w.max(grid_measure_text_width(mtm, grid_handle, &header) + 16.0);
        let sorted = GRID_SORT.with(|map| {
            map.borrow().get(&grid_handle).and_then(|s| s.borrow().0) == Some(data_col)
        });
        if sorted {
            max_w += GRID_SORT_INDICATOR_PAD;
        }
    }
    let row_count = grid_row_count(grid_handle);
    for row in 0..row_count {
        let text = grid_cell_display_text(grid_handle, row, data_col);
        if !text.is_empty() {
            max_w = max_w.max(grid_measure_text_width(mtm, grid_handle, &text) + 8.0);
        }
        if grid_cell_image_index(grid_handle, row, data_col).is_some() {
            let icon_w = grid_image_list_icon_width(grid_handle).unwrap_or(GRID_CELL_ICON_WIDTH);
            max_w = max_w.max(icon_w + 8.0);
        }
    }
    max_w.ceil() as i32
}

fn grid_reuse_or_new_cell(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &GridTableController,
    ident: &str,
    build: impl FnOnce(MainThreadMarker) -> Retained<NSTableCellView>,
) -> Retained<NSTableCellView> {
    let id = NSString::from_str(ident);
    if let Some(view) = unsafe { table.makeViewWithIdentifier_owner(&id, Some(owner)) } {
        if let Ok(cell) = view.downcast::<NSTableCellView>() {
            return cell;
        }
    }
    let cell = build(mtm);
    cell.setIdentifier(Some(&id));
    cell
}

fn grid_progress_label_text(info: &GridProgressCell) -> String {
    if let Some(label) = &info.label {
        return label.clone();
    }
    format!("{}/{}", info.value, info.max)
}

fn grid_find_progress_gauge(cell: &NSTableCellView) -> Option<Retained<NSProgressIndicator>> {
    for sub in cell.subviews().iter() {
        if let Ok(gauge) = sub.downcast::<NSProgressIndicator>() {
            return Some(gauge);
        }
    }
    None
}

fn grid_find_progress_label(cell: &NSTableCellView) -> Option<Retained<NSTextField>> {
    for sub in cell.subviews().iter() {
        if let Ok(field) = sub.downcast::<NSTextField>() {
            return Some(field);
        }
    }
    None
}

// Cell builders mirror the parameters of the grid cell kinds they implement;
// bundling them would obscure the per-kind differences.
#[allow(clippy::too_many_arguments)]
fn grid_build_progress_cell_view(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &GridTableController,
    handle: NativeHandle,
    row: usize,
    data_col: usize,
    selected: bool,
    col_width: f64,
    row_h: f64,
    info: GridProgressCell,
) -> Retained<NSTableCellView> {
    let cell = grid_reuse_or_new_cell(mtm, table, owner, GRID_PROGRESS_CELL_ID, |mtm| {
        let cell = NSTableCellView::new(mtm);
        let gauge = NSProgressIndicator::initWithFrame(
            NSProgressIndicator::alloc(mtm),
            NSRect::new(NSPoint::new(2.0, 4.0), NSSize::new(80.0, 14.0)),
        );
        gauge.setStyle(NSProgressIndicatorStyle::Bar);
        gauge.setIndeterminate(false);
        gauge.setControlSize(NSControlSize::Small);
        cell.addSubview(&gauge);
        let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        label.setBordered(false);
        label.setEditable(false);
        label.setBezeled(false);
        label.setDrawsBackground(false);
        cell.addSubview(&label);
        cell
    });

    let (fg, bg) = grid_resolve_style(handle, row, data_col, selected);
    let col_width = col_width.max(4.0);
    let row_h = row_h.max(grid_row_height(handle));
    let label_text = grid_progress_label_text(&info);
    let label_w = grid_measure_text_width(mtm, handle, &label_text).min(col_width * 0.45) + 6.0;
    let gauge_w = (col_width - label_w - 6.0).max(24.0);
    let gauge_h = 14.0;
    let gauge_y = ((row_h - gauge_h) / 2.0).max(2.0);

    cell.setClipsToBounds(true);
    if let Some(gauge) = grid_find_progress_gauge(&cell) {
        gauge.setMinValue(0.0);
        gauge.setMaxValue(info.max.max(1) as f64);
        let value = (info.value as f64).min(gauge.maxValue());
        gauge.setDoubleValue(value);
        gauge.setHidden(false);
        gauge.setFrame(NSRect::new(
            NSPoint::new(3.0, gauge_y),
            NSSize::new(gauge_w, gauge_h),
        ));
    }
    if let Some(label) = grid_find_progress_label(&cell) {
        label.setStringValue(&NSString::from_str(&label_text));
        label.setDrawsBackground(true);
        label.setBackgroundColor(Some(&ns_color_from_rgba(bg)));
        label.setTextColor(Some(&ns_color_from_rgba(fg)));
        label.setFont(Some(&grid_body_font(mtm, handle)));
        label.setAlignment(NSTextAlignment::Right);
        label.setHidden(label_text.is_empty());
        label.setFrame(NSRect::new(
            NSPoint::new(3.0 + gauge_w + 2.0, 0.0),
            NSSize::new(label_w.max(0.0), row_h),
        ));
    }
    let tooltip = grid_resolve_cell_tooltip(handle, row, data_col, &label_text, gauge_w);
    if let Some(tip) = tooltip {
        cell.setToolTip(Some(&NSString::from_str(&tip)));
    } else {
        cell.setToolTip(None);
    }
    cell
}

#[allow(clippy::too_many_arguments)]
fn grid_build_row_label_cell_view(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &GridTableController,
    handle: NativeHandle,
    row: usize,
    selected: bool,
    col_width: f64,
    row_h: f64,
) -> Retained<NSTableCellView> {
    let cell = grid_reuse_or_new_cell(mtm, table, owner, GRID_ROW_LABEL_CELL_ID, |mtm| {
        let cell = NSTableCellView::new(mtm);
        let text_field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        text_field.setBordered(false);
        text_field.setEditable(false);
        text_field.setBezeled(false);
        text_field.setAlignment(NSTextAlignment::Center);
        unsafe {
            cell.setTextField(Some(&text_field));
        }
        cell.addSubview(&text_field);
        cell
    });
    let label = (row + 1).to_string();
    let (_, bg) = grid_resolve_style(handle, row, 0, selected);
    let col_width = col_width.max(4.0);
    let row_h = row_h.max(grid_row_height(handle));
    unsafe {
        cell.setClipsToBounds(true);
        if let Some(text_field) = cell.textField() {
            text_field.setStringValue(&NSString::from_str(&label));
            text_field.setDrawsBackground(true);
            text_field.setBackgroundColor(Some(&ns_color_from_rgba(bg)));
            text_field.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(col_width, row_h),
            ));
        }
    }
    cell
}

#[allow(clippy::too_many_arguments)]
fn grid_build_priority_cell_view(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &GridTableController,
    handle: NativeHandle,
    row: usize,
    data_col: usize,
    selected: bool,
    col_width: f64,
    row_h: f64,
    level: u32,
    label: &str,
) -> Retained<NSTableCellView> {
    let cell = grid_reuse_or_new_cell(mtm, table, owner, GRID_PRIORITY_CELL_ID, |mtm| {
        let cell = NSTableCellView::new(mtm);
        let indicator = NSLevelIndicator::initWithFrame(
            NSLevelIndicator::alloc(mtm),
            NSRect::new(NSPoint::new(2.0, 4.0), NSSize::new(48.0, 16.0)),
        );
        indicator.setLevelIndicatorStyle(NSLevelIndicatorStyle::Relevancy);
        indicator.setMaxValue(4.0);
        indicator.setMinValue(0.0);
        cell.addSubview(&indicator);
        let text_field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        text_field.setBordered(false);
        text_field.setEditable(false);
        text_field.setBezeled(false);
        text_field.setDrawsBackground(false);
        cell.addSubview(&text_field);
        cell
    });
    let (fg, bg) = grid_resolve_style(handle, row, data_col, selected);
    let col_width = col_width.max(4.0);
    let row_h = row_h.max(grid_row_height(handle));
    let label_w = grid_measure_text_width(mtm, handle, label).min(col_width * 0.55) + 6.0;
    let indicator_w = (col_width - label_w - 6.0).max(36.0);
    cell.setClipsToBounds(true);
    if let Some(indicator) = cell
        .subviews()
        .iter()
        .find_map(|sub| sub.downcast::<NSLevelIndicator>().ok())
    {
        indicator.setIntegerValue(level as isize);
        indicator.setFrame(NSRect::new(
            NSPoint::new(2.0, ((row_h - 16.0) / 2.0).max(2.0)),
            NSSize::new(indicator_w, 16.0),
        ));
    }
    if let Some(text_field) = cell
        .subviews()
        .iter()
        .find_map(|sub| sub.downcast::<NSTextField>().ok())
    {
        text_field.setStringValue(&NSString::from_str(label));
        text_field.setTextColor(Some(&ns_color_from_rgba(fg)));
        text_field.setDrawsBackground(true);
        text_field.setBackgroundColor(Some(&ns_color_from_rgba(bg)));
        text_field.setFont(Some(&grid_body_font(mtm, handle)));
        text_field.setFrame(NSRect::new(
            NSPoint::new(indicator_w + 4.0, 0.0),
            NSSize::new((col_width - indicator_w - 6.0).max(0.0), row_h),
        ));
    }
    cell
}

fn grid_move_focused_cell(
    table: &NSTableView,
    handle: NativeHandle,
    frame_handle: NativeHandle,
    command_id: u16,
    row_delta: i32,
    col_delta: i32,
) {
    let nrows = grid_row_count(handle);
    let ncols = super::grid_col_count(handle);
    if nrows == 0 || ncols == 0 {
        return;
    }
    let (row, col) = grid_get_focused_cell(handle).unwrap_or((0, 0));
    let new_row = (row as i64 + row_delta as i64).clamp(0, (nrows - 1) as i64) as usize;
    let new_col = if col_delta == 0 {
        col
    } else {
        match grid_next_visible_col(handle, col, col_delta, ncols) {
            Some(c) => c,
            None => return,
        }
    };
    if super::grid_is_column_hidden(handle, new_col) {
        return;
    }
    grid_set_focused_cell(handle, new_row, new_col);
    let idx = NSIndexSet::indexSetWithIndex(new_row as NSUInteger);
    table.selectRowIndexes_byExtendingSelection(&idx, false);
    table.scrollRowToVisible(new_row as NSInteger);
    dispatch_notify(frame_handle, command_id, GRID_NOTIFY_FOCUS_CHANGED);
}

#[allow(clippy::too_many_arguments)]
fn grid_build_data_cell_view(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &GridTableController,
    handle: NativeHandle,
    row: usize,
    data_col: usize,
    selected: bool,
    col_width: f64,
    row_h: f64,
) -> Retained<NSTableCellView> {
    let cell = grid_reuse_or_new_cell(mtm, table, owner, GRID_DATA_CELL_ID, |mtm| {
        let cell = NSTableCellView::new(mtm);
        let text_field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        text_field.setBordered(false);
        text_field.setEditable(false);
        text_field.setBezeled(false);
        unsafe {
            cell.setTextField(Some(&text_field));
        }
        cell.addSubview(&text_field);
        let image_view = NSImageView::new(mtm);
        unsafe {
            cell.setImageView(Some(&image_view));
        }
        cell.addSubview(&image_view);
        cell
    });

    let text = grid_cell_display_text(handle, row, data_col);
    let display_text = text.replace('\r', "\n");
    let is_multiline = display_text.contains('\n');
    let image_idx = grid_cell_image_index(handle, row, data_col);
    let (fg, bg) = grid_resolve_style(handle, row, data_col, selected);
    let align = grid_column_align(handle, data_col);
    let col_width = col_width.max(4.0);
    let row_h = row_h.max(grid_row_height(handle));
    let icon_w_max = grid_image_list_icon_width(handle).unwrap_or(GRID_CELL_ICON_WIDTH);
    let has_icon = image_idx.is_some();
    let icon_only = has_icon && text.is_empty();
    let narrow_icon = has_icon && col_width <= icon_w_max + 10.0;
    let hide_text = icon_only || narrow_icon;
    let text_x = if has_icon && !hide_text {
        icon_w_max + 3.0
    } else {
        2.0
    };
    let text_w = (col_width - text_x - 2.0).max(0.0);

    unsafe {
        cell.setClipsToBounds(true);
        if let Some(text_field) = cell.textField() {
            text_field.setStringValue(&NSString::from_str(&display_text));
            text_field.setDrawsBackground(true);
            text_field.setBackgroundColor(Some(&ns_color_from_rgba(bg)));
            text_field.setTextColor(Some(&ns_color_from_rgba(fg)));
            text_field.setFont(Some(&grid_body_font(mtm, handle)));
            grid_set_field_alignment(&text_field, align);
            if is_multiline {
                text_field.setMaximumNumberOfLines(0);
            } else {
                text_field.setMaximumNumberOfLines(1);
            }
            text_field.setHidden(hide_text || text_w < 1.0);
            text_field.setFrame(NSRect::new(
                NSPoint::new(text_x, 0.0),
                NSSize::new(text_w, row_h),
            ));
        }
        if let Some(image_view) = cell.imageView() {
            if let Some(idx) = image_idx {
                if let Some(list_h) = GRID_IMAGE_LISTS.with(|m| m.borrow().get(&handle).copied()) {
                    if let Some(img) = image_list_image_at(list_h, idx) {
                        image_view.setImage(Some(&img));
                        image_view.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                        image_view.setHidden(false);
                        let icon_w = (col_width - 4.0).min(icon_w_max).max(4.0);
                        let icon_x = if hide_text {
                            ((col_width - icon_w) / 2.0).max(1.0)
                        } else {
                            1.0
                        };
                        image_view.setFrame(NSRect::new(
                            NSPoint::new(icon_x, 2.0),
                            NSSize::new(icon_w, row_h - 4.0),
                        ));
                    } else {
                        image_view.setHidden(true);
                    }
                } else {
                    image_view.setHidden(true);
                }
            } else {
                image_view.setHidden(true);
            }
        }
        let tooltip = grid_resolve_cell_tooltip(handle, row, data_col, &text, text_w);
        if let Some(tip) = tooltip {
            cell.setToolTip(Some(&NSString::from_str(&tip)));
        } else {
            cell.setToolTip(None);
        }
    }
    cell
}

#[allow(clippy::too_many_arguments)]
fn grid_build_checkbox_cell_view(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &GridTableController,
    handle: NativeHandle,
    row: usize,
    selected: bool,
    col_width: f64,
    row_h: f64,
) -> Retained<NSTableCellView> {
    let cell = grid_reuse_or_new_cell(mtm, table, owner, GRID_CHECK_CELL_ID, |mtm| {
        let cell = NSTableCellView::new(mtm);
        let text_field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        text_field.setBordered(false);
        text_field.setEditable(false);
        text_field.setBezeled(false);
        text_field.setAlignment(NSTextAlignment::Center);
        unsafe {
            cell.setTextField(Some(&text_field));
        }
        cell.addSubview(&text_field);
        cell
    });
    let checked = GRID_ROW_CHECKED.with(|map| {
        map.borrow()
            .get(&handle)
            .and_then(|c| c.borrow().get(row).copied())
            .unwrap_or(false)
    });
    let sym = if checked { "☑" } else { "☐" };
    let (_, bg) = grid_resolve_style(handle, row, 0, selected);
    let col_width = col_width.max(4.0);
    let row_h = row_h.max(grid_row_height(handle));
    unsafe {
        cell.setClipsToBounds(true);
        if let Some(text_field) = cell.textField() {
            text_field.setStringValue(&NSString::from_str(sym));
            text_field.setDrawsBackground(true);
            text_field.setBackgroundColor(Some(&ns_color_from_rgba(bg)));
            text_field.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(col_width, row_h),
            ));
        }
    }
    cell
}

fn grid_dispatch_item_activate(
    table: &NSTableView,
    grid_handle: NativeHandle,
    frame_handle: NativeHandle,
    command_id: u16,
) {
    let row = table.selectedRow();
    if row < 0 {
        return;
    }
    let col = table.clickedColumn();
    let data_col = if col >= 0 {
        let table_column = table.tableColumns().objectAtIndex(col as NSUInteger);
        grid_data_col_for_table_column(&table_column).unwrap_or(0)
    } else {
        0
    };
    GRID_ACTIVATE_CELL.with(|map| {
        map.borrow_mut()
            .insert(grid_handle, (row as usize, data_col));
    });
    dispatch_notify(frame_handle, command_id, GRID_NOTIFY_ITEM_ACTIVATE);
}

#[derive(Default)]
struct GridTableControllerIvars {
    grid_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = GridTableControllerIvars]
    struct GridTableController;

    unsafe impl NSObjectProtocol for GridTableController {}

    unsafe impl NSControlTextEditingDelegate for GridTableController {}

    unsafe impl NSTableViewDataSource for GridTableController {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows_in_table_view(&self, table_view: &NSTableView) -> NSInteger {
            let handle = self.ivars().grid_handle.get();
            grid_effective_row_count(handle, table_view) as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for GridTableController {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn table_view_view_for_table_column_row(
            &self,
            table_view: &NSTableView,
            table_column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Retained<NSView> {
            let mtm = MainThreadMarker::new().expect("main thread");
            let handle = self.ivars().grid_handle.get();
            let cell_view = if row < 0 || table_column.is_none() {
                NSTableCellView::new(mtm)
            } else {
                let view_row = row as usize;
                let row = grid_map_view_row(handle, table_view, view_row);
                let table_column = table_column.unwrap();
                let selected = table_view.selectedRow() == row as NSInteger;
                let col_width = table_column.width();
                let row_h = grid_row_display_height(mtm, handle, table_view, view_row);
                let id = table_column.identifier().to_string();
                if id == GRID_CHECKBOX_COL_ID {
                    grid_build_checkbox_cell_view(
                        mtm,
                        table_view,
                        self,
                        handle,
                        row,
                        selected,
                        col_width,
                        row_h,
                    )
                } else if id == GRID_ROW_LABEL_COL_ID {
                    grid_build_row_label_cell_view(
                        mtm,
                        table_view,
                        self,
                        handle,
                        row,
                        selected,
                        col_width,
                        row_h,
                    )
                } else if let Some(data_col) = grid_data_col_for_table_column(table_column) {
                    if let Some(info) = grid_cell_progress_kind(handle, row, data_col) {
                        if info.kind == GridNativeKind::Priority {
                            let label = info
                                .label
                                .as_deref()
                                .unwrap_or("");
                            grid_build_priority_cell_view(
                                mtm,
                                table_view,
                                self,
                                handle,
                                row,
                                data_col,
                                selected,
                                col_width,
                                row_h,
                                info.value,
                                label,
                            )
                        } else {
                            grid_build_progress_cell_view(
                                mtm,
                                table_view,
                                self,
                                handle,
                                row,
                                data_col,
                                selected,
                                col_width,
                                row_h,
                                info,
                            )
                        }
                    } else {
                        grid_build_data_cell_view(
                            mtm,
                            table_view,
                            self,
                            handle,
                            row,
                            data_col,
                            selected,
                            col_width,
                            row_h,
                        )
                    }
                } else {
                    NSTableCellView::new(mtm)
                }
            };
            Retained::into_super(cell_view)
        }

        #[unsafe(method(tableView:heightOfRow:))]
        fn table_view_height_of_row(
            &self,
            table_view: &NSTableView,
            row: NSInteger,
        ) -> f64 {
            let mtm = MainThreadMarker::new().expect("main thread");
            let handle = self.ivars().grid_handle.get();
            if row < 0 {
                return grid_row_height(handle);
            }
            grid_row_display_height(mtm, handle, table_view, row as usize)
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn table_view_selection_did_change(&self, notification: &NSNotification) {
            let handle = self.ivars().grid_handle.get();
            if let Some(obj) = notification.object() {
                if let Some(table) = obj.downcast_ref::<NSTableView>() {
                    grid_sync_selected_rows_from_table(handle, table);
                }
            }
            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                GRID_NOTIFY_ITEM_CHANGED,
            );
        }

        #[unsafe(method(tableView:didClickTableColumn:))]
        fn table_view_did_click_table_column(
            &self,
            _table_view: &NSTableView,
            table_column: &NSTableColumn,
        ) {
            let handle = self.ivars().grid_handle.get();
            if !grid_row_col_highlight_enabled(handle) {
                return;
            }
            if let Some(col) = grid_data_col_for_table_column(table_column) {
                grid_dispatch_highlight_click(
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                    handle,
                    None,
                    Some(col),
                );
            }
        }

        #[unsafe(method(tableViewColumnDidResize:))]
        fn table_view_column_did_resize(&self, notification: &NSNotification) {
            let handle = self.ivars().grid_handle.get();
            if super::grid_column_resize_syncing(handle) {
                return;
            }
            let Some(table_column) = notification
                .userInfo()
                .and_then(|info| info.objectForKey(&NSString::from_str("NSTableColumn")))
                .and_then(|obj| obj.downcast::<NSTableColumn>().ok())
            else {
                return;
            };
            let width = table_column.width() as i32;
            if let Some(data_col) = grid_data_col_for_table_column(&table_column) {
                GRID_COLUMNS.with(|map| {
                    if let Some(cols) = map.borrow().get(&handle) {
                        if let Some(entry) = cols.borrow_mut().get_mut(data_col) {
                            entry.1 = width.max(GRID_MIN_COLUMN_WIDTH);
                        }
                    }
                });
                GRID_COLUMN_RESIZED.with(|map| {
                    map.borrow_mut().insert(handle, (data_col, width));
                });
                super::grid_schedule_column_resize_notify(
                    handle,
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                );
            }
        }

        #[unsafe(method(tableViewColumnDidMove:))]
        fn table_view_column_did_move(&self, _notification: &NSNotification) {
            // Visual order is read on demand via [`grid_column_order`].
        }
    }

    impl GridTableController {
        #[unsafe(method(tableView:writeRowsWithIndexes:toPasteboard:))]
        fn table_view_write_rows_with_indexes_to_pasteboard(
            &self,
            _table_view: &NSTableView,
            row_indexes: &NSIndexSet,
            pboard: &NSPasteboard,
        ) -> Bool {
            let handle = self.ivars().grid_handle.get();
            if !grid_row_drag_enabled(handle) {
                return Bool::NO;
            }
            let mut rows = String::new();
            let mut idx = row_indexes.firstIndex() as NSInteger;
            while idx != NSNotFound {
                if !rows.is_empty() {
                    rows.push(',');
                }
                rows.push_str(&idx.to_string());
                idx = row_indexes.indexGreaterThanIndex(idx as NSUInteger) as NSInteger;
            }
            if rows.is_empty() {
                return Bool::NO;
            }
            let types = NSArray::from_slice(&[unsafe { NSPasteboardTypeString }]);
            unsafe {
                pboard.declareTypes_owner(&types, None);
            }
            pboard.setString_forType(&NSString::from_str(&rows), unsafe { NSPasteboardTypeString });
            Bool::YES
        }

        #[unsafe(method(tableView:validateDrop:proposedRow:proposedDropOperation:))]
        fn table_view_validate_drop(
            &self,
            _table_view: &NSTableView,
            _info: &ProtocolObject<dyn objc2_app_kit::NSDraggingInfo>,
            _row: NSInteger,
            drop_operation: NSTableViewDropOperation,
        ) -> NSDragOperation {
            let handle = self.ivars().grid_handle.get();
            if !grid_row_drag_enabled(handle) {
                return NSDragOperation::None;
            }
            if drop_operation == NSTableViewDropOperation::On {
                return NSDragOperation::None;
            }
            NSDragOperation::Move
        }

        #[unsafe(method(tableView:acceptDrop:row:dropOperation:))]
        fn table_view_accept_drop(
            &self,
            table_view: &NSTableView,
            info: &ProtocolObject<dyn NSDraggingInfo>,
            row: NSInteger,
            drop_operation: NSTableViewDropOperation,
        ) -> Bool {
            let handle = self.ivars().grid_handle.get();
            if !grid_row_drag_enabled(handle) || drop_operation == NSTableViewDropOperation::On {
                return Bool::NO;
            }
            let pboard = info.draggingPasteboard();
            let Some(raw) = pboard.stringForType(unsafe { NSPasteboardTypeString }) else {
                return Bool::NO;
            };
            let text = raw.to_string();
            let row_indices: Vec<&str> = text.split(',').collect();
            if row_indices.is_empty() {
                return Bool::NO;
            }
            // Parse all row indices
            let rows: Vec<isize> = row_indices
                .iter()
                .filter_map(|s| s.trim().parse().ok())
                .filter(|&r| r >= 0)
                .collect();
            if rows.is_empty() {
                return Bool::NO;
            }
            // Use the first row as the source for the reorder operation
            let from_view = rows[0];
            let from = grid_map_view_row(handle, table_view, from_view as usize);
            let to = if row < 0 {
                0
            } else {
                grid_map_view_row(handle, table_view, row as usize)
            };
            grid_record_row_reordered(handle, from, to);
            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                GRID_NOTIFY_ROW_REORDERED,
            );
            Bool::YES
        }
    }

);

impl GridTableController {
    fn new(
        mtm: MainThreadMarker,
        grid_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(GridTableControllerIvars {
            grid_handle: Cell::new(grid_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

#[derive(Default)]
struct GridTableViewIvars {
    grid_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
    pane: Cell<u8>,
}

define_class!(
    #[unsafe(super = NSTableView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = GridTableViewIvars]
    struct GridTableView;

    unsafe impl NSObjectProtocol for GridTableView {}

    impl GridTableView {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().grid_handle.get();
            grid_end_inline_edit(handle, true);
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let row = self.rowAtPoint(point);
            let col = self.columnAtPoint(point);
            if row < 0 && event.clickCount() >= 2 && col >= 0 {
                if let Some(data_col) = grid_data_col_for_table_index(handle, self, col) {
                    let _ = grid_autosize_column(handle, data_col, true);
                }
                unsafe {
                    let _: () = msg_send![super(self), mouseDown: event];
                }
                return;
            }
            if row >= 0
                && grid_row_col_highlight_enabled(handle)
                && grid_is_row_reference_table_column(handle, col)
            {
                grid_dispatch_highlight_click(
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                    handle,
                    Some(row as usize),
                    None,
                );
            }
            if row >= 0
                && grid_checkboxes_enabled(handle)
                && col == 0
            {
                if event.clickCount() == 1 {
                    let row = row as usize;
                    GRID_ROW_CHECKED.with(|map| {
                        if let Some(checked) = map.borrow().get(&handle) {
                            let mut vec = checked.borrow_mut();
                            if row >= vec.len() {
                                vec.resize(row + 1, false);
                            }
                            vec[row] = !vec[row];
                        }
                    });
                    grid_reload(handle);
                }
                return;
            }
            unsafe { msg_send![super(self), mouseDown: event] }
            if row >= 0 && col >= 0 {
                if let Some(data_col) = grid_data_col_for_table_index(handle, self, col) {
                    grid_set_focused_cell(handle, row as usize, data_col);
                    dispatch_notify(
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        GRID_NOTIFY_FOCUS_CHANGED,
                    );
                    if event.clickCount() == 1 && grid_link_clicks_enabled(handle) {
                        grid_queue_link_click(handle, row as usize, data_col);
                        dispatch_notify(
                            self.ivars().frame_handle.get(),
                            self.ivars().command_id.get(),
                            GRID_NOTIFY_LINK_CLICK,
                        );
                    }
                }
            }
            if event.clickCount() >= 2 {
                if row >= 0 && col >= 0 {
                    if let Some(data_col) = grid_data_col_for_table_index(handle, self, col) {
                        if grid_cell_editing_enabled(handle) {
                            grid_set_focused_cell(handle, row as usize, data_col);
                            grid_queue_edit_start(handle, row as usize, data_col);
                            dispatch_notify(
                                self.ivars().frame_handle.get(),
                                self.ivars().command_id.get(),
                                GRID_NOTIFY_EDIT_START,
                            );
                            return;
                        }
                    }
                }
                grid_dispatch_item_activate(
                    self,
                    self.ivars().grid_handle.get(),
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                );
            }
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let handle = self.ivars().grid_handle.get();
            let key = event.keyCode();
            let mods = event.modifierFlags();
            let cmd = mods.contains(NSEventModifierFlags::Command);
            if GRID_INLINE_EDITORS.with(|m| m.borrow().contains_key(&handle)) {
                if key == 36 || key == 76 {
                    grid_end_inline_edit(handle, true);
                    return;
                }
                if key == 53 {
                    grid_end_inline_edit(handle, false);
                    return;
                }
            }
            if key == 36 || key == 76 {
                grid_dispatch_item_activate(
                    self,
                    handle,
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                );
                return;
            }
            if key == 120 && grid_cell_editing_enabled(handle) {
                if let Some((row, col)) = grid_get_focused_cell(handle) {
                    grid_queue_edit_start(handle, row, col);
                    dispatch_notify(
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        GRID_NOTIFY_EDIT_START,
                    );
                }
                return;
            }
            if cmd && key == 9 {
                if let Some((row, col)) = grid_get_focused_cell(handle) {
                    grid_queue_paste_at(handle, row, col);
                    dispatch_notify(
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        GRID_NOTIFY_PASTE_REQUEST,
                    );
                }
                return;
            }
            let shift = mods.contains(NSEventModifierFlags::Shift);
            match key {
                123 => {
                    grid_move_focused_cell(
                        self,
                        handle,
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        0,
                        -1,
                    );
                    return;
                }
                124 => {
                    grid_move_focused_cell(
                        self,
                        handle,
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        0,
                        1,
                    );
                    return;
                }
                125 => {
                    grid_move_focused_cell(
                        self,
                        handle,
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        1,
                        0,
                    );
                    return;
                }
                126 => {
                    grid_move_focused_cell(
                        self,
                        handle,
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        -1,
                        0,
                    );
                    return;
                }
                48 if !cmd => {
                    grid_move_focused_cell(
                        self,
                        handle,
                        self.ivars().frame_handle.get(),
                        self.ivars().command_id.get(),
                        0,
                        if shift { -1 } else { 1 },
                    );
                    return;
                }
                _ => {}
            }
            unsafe { msg_send![super(self), keyDown: event] }
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            let handle = self.ivars().grid_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let row = self.rowAtPoint(point);
            let table_col = self.columnAtPoint(point);
            let in_header = row < 0 && self.headerView().is_some();
            let data_col = if table_col >= 0 {
                let table_column = self.tableColumns().objectAtIndex(table_col as NSUInteger);
                grid_data_col_for_table_column(&table_column).unwrap_or(0)
            } else {
                0
            };
            let (menu_row, menu_col) = if in_header || row < 0 {
                (usize::MAX, data_col)
            } else {
                (row as usize, data_col)
            };
            GRID_CONTEXT_MENU.with(|map| {
                map.borrow_mut()
                    .insert(handle, (in_header, menu_row, menu_col));
            });
            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                GRID_NOTIFY_CONTEXT_MENU,
            );
        }
    }
);

impl GridTableView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        grid_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
        pane: u8,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(GridTableViewIvars {
            grid_handle: Cell::new(grid_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
            pane: Cell::new(pane),
        });
        let table: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        table.setStyle(NSTableViewStyle::FullWidth);
        table.setColumnAutoresizingStyle(
            objc2_app_kit::NSTableViewColumnAutoresizingStyle::LastColumnOnlyAutoresizingStyle,
        );
        table.setAllowsColumnReordering(true);
        table.setAllowsColumnResizing(true);
        table.setAllowsMultipleSelection(true);
        table.setIntercellSpacing(NSSize::new(0.0, 0.0));
        table.setRowHeight(grid_row_height(grid_handle));
        table.setUsesAutomaticRowHeights(false);
        table
    }
}

pub fn create_grid_ctrl(parent: NativeHandle, command_id: u16, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    GRID_COLUMNS.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    GRID_ROWS.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    grid_init_cell_kinds(handle);
    GRID_SELECTED.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    GRID_ROW_CHECKED.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(Vec::new()));
    });
    GRID_CHECKBOXES.with(|map| {
        map.borrow_mut().insert(handle, false);
    });
    GRID_APPEARANCE.with(|map| {
        map.borrow_mut()
            .insert(handle, RefCell::new(GridAppearanceData::default()));
    });
    GRID_CELL_STYLES.with(|map| {
        map.borrow_mut()
            .insert(handle, RefCell::new(HashMap::new()));
    });
    GRID_ROW_STYLES.with(|map| {
        map.borrow_mut()
            .insert(handle, RefCell::new(HashMap::new()));
    });
    GRID_ALTERNATING.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new(None));
    });
    GRID_FONT.with(|map| {
        map.borrow_mut()
            .insert(handle, RefCell::new((String::new(), 11.0)));
    });
    GRID_SORT.with(|map| {
        map.borrow_mut().insert(handle, RefCell::new((None, None)));
    });
    GRID_HIGHLIGHT_ROW.with(|map| {
        map.borrow_mut().insert(handle, None);
    });
    GRID_HIGHLIGHT_COL.with(|map| {
        map.borrow_mut().insert(handle, None);
    });
    GRID_ROW_COL_HIGHLIGHT.with(|map| {
        map.borrow_mut().insert(handle, false);
    });
    GRID_SHOW_LINES.with(|map| {
        map.borrow_mut().insert(handle, true);
    });

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let frame_handle = resolve_frame_handle(parent);

    let scroll = AppKitScrollView::initWithFrame(AppKitScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);

    let table = GridTableView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
        handle,
        frame_handle,
        command_id,
        GRID_PANE_MAIN,
    );
    let controller = GridTableController::new(mtm, handle, frame_handle, command_id);
    let data_source = ProtocolObject::from_ref(&*controller);
    let delegate = ProtocolObject::from_ref(&*controller);
    unsafe {
        table.setDataSource(Some(data_source));
        table.setDelegate(Some(delegate));
    }

    scroll.setDocumentView(Some(&table));
    content.addSubview(&scroll);

    GRID_TABLES.with(|map| map.borrow_mut().insert(handle, table));
    GRID_MAIN_SCROLLS.with(|m| m.borrow_mut().insert(handle, scroll.clone()));
    GRID_DELEGATES.with(|map| map.borrow_mut().insert(handle, controller));
    grid_store_notify_targets(handle, frame_handle, command_id);
    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn grid_sync_view_geometry(grid_handle: NativeHandle, width: f64, height: f64) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    for table in grid_all_table_views(grid_handle) {
        let header = table
            .headerView()
            .map(|h| h.frame().size.height)
            .unwrap_or(24.0);
        let rows_h = grid_document_body_height(mtm, grid_handle, &table);
        let min_h = header + grid_row_height(grid_handle);
        let doc_h = rows_h + header;
        table.setFrameSize(NSSize::new(width.max(1.0), doc_h.max(height.max(min_h))));
    }
}

pub fn grid_rebuild_columns(grid_handle: NativeHandle) {
    grid_rebuild_table_columns(grid_handle);
}

pub fn grid_append_column(grid_handle: NativeHandle, title: &str, width: i32, align: i32) {
    GRID_COLUMNS.with(|map| {
        if let Some(cols) = map.borrow().get(&grid_handle) {
            cols.borrow_mut().push((title.to_string(), width, align));
        }
    });
    GRID_ROWS.with(|map| {
        if let Some(rows) = map.borrow().get(&grid_handle) {
            let mut vec = rows.borrow_mut();
            for (texts, images) in vec.iter_mut() {
                texts.push(String::new());
                images.push(None);
            }
        }
    });
    grid_append_column_kind_slots(grid_handle);
    grid_rebuild_table_columns(grid_handle);
    grid_reload(grid_handle);
}

pub fn grid_set_column_width(grid_handle: NativeHandle, col: usize, width: i32) {
    let width = width.max(GRID_MIN_COLUMN_WIDTH);
    GRID_COLUMNS.with(|map| {
        if let Some(cols) = map.borrow().get(&grid_handle) {
            if let Some(entry) = cols.borrow_mut().get_mut(col) {
                entry.1 = width;
            }
        }
    });
    if let Some(table_col) = grid_table_column_at(grid_handle, col) {
        let current = table_col.width().round() as i32;
        if (current - width).abs() <= 1 {
            return;
        }
        super::grid_begin_column_resize_sync(grid_handle);
        table_col.setWidth(width as f64);
        super::grid_end_column_resize_sync(grid_handle);
    }
}

pub fn grid_redraw_view(grid_handle: NativeHandle) {
    grid_reload(grid_handle);
    if let Some((w, h)) = super::native_view_size(grid_handle) {
        grid_sync_view_geometry(grid_handle, w, h);
    }
}

pub fn grid_set_checkboxes(grid_handle: NativeHandle, enabled: bool) {
    GRID_CHECKBOXES.with(|map| {
        map.borrow_mut().insert(grid_handle, enabled);
    });
    grid_rebuild_table_columns(grid_handle);
    grid_reload(grid_handle);
}

pub fn grid_select_row(grid_handle: NativeHandle, row: usize) {
    GRID_SELECTED.with(|map| {
        if let Some(sel) = map.borrow().get(&grid_handle) {
            *sel.borrow_mut() = vec![row];
        }
    });
    GRID_TABLES.with(|map| {
        if let Some(table) = map.borrow().get(&grid_handle) {
            let indexes = NSIndexSet::indexSetWithIndex(row as NSUInteger);
            table.selectRowIndexes_byExtendingSelection(&indexes, false);
            table.scrollRowToVisible(row as NSInteger);
        }
    });
}

pub fn grid_set_sort_indicator(
    grid_handle: NativeHandle,
    sort_col: Option<usize>,
    ascending: Option<bool>,
) {
    super::grid_update_sort_state(grid_handle, sort_col, ascending);
    grid_sync_column_titles(grid_handle);
    grid_apply_sort_descriptors(grid_handle);
}

pub fn grid_ensure_row_visible(grid_handle: NativeHandle, row: usize) {
    GRID_TABLES.with(|map| {
        if let Some(table) = map.borrow().get(&grid_handle) {
            table.scrollRowToVisible(row as NSInteger);
        }
    });
}

pub fn grid_autosize_column(grid_handle: NativeHandle, col: usize, include_header: bool) -> i32 {
    let Some(mtm) = MainThreadMarker::new() else {
        return GRID_MIN_COLUMN_WIDTH;
    };
    if col >= grid_col_count(grid_handle) {
        return GRID_MIN_COLUMN_WIDTH;
    }
    let width = grid_autosize_column_width(mtm, grid_handle, col, include_header);
    grid_set_column_width(grid_handle, col, width);
    width
}

pub fn grid_autosize_all_columns(grid_handle: NativeHandle, include_header: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let n = grid_col_count(grid_handle);
    for col in 0..n {
        let width = grid_autosize_column_width(mtm, grid_handle, col, include_header);
        grid_set_column_width(grid_handle, col, width);
    }
}

pub fn grid_column_order(grid_handle: NativeHandle) -> Vec<usize> {
    grid_data_column_order(grid_handle)
}

pub fn grid_set_column_order(grid_handle: NativeHandle, order: &[usize]) -> bool {
    if order.len() != grid_col_count(grid_handle) {
        return false;
    }
    let offset = grid_table_leading_cols(grid_handle);
    let mut current = grid_data_column_order(grid_handle);
    if current.len() != order.len() {
        return false;
    }
    GRID_TABLES.with(|map| {
        let Some(table) = map.borrow().get(&grid_handle).cloned() else {
            return false;
        };
        for (target_pos, &want) in order.iter().enumerate() {
            let Some(from_pos) = current.iter().position(|&c| c == want) else {
                return false;
            };
            if from_pos != target_pos {
                let from = (offset + from_pos) as NSInteger;
                let to = (offset + target_pos) as NSInteger;
                table.moveColumn_toColumn(from, to);
                let moved = current.remove(from_pos);
                current.insert(target_pos, moved);
            }
        }
        true
    })
}

pub fn grid_apply_column_hidden(grid_handle: NativeHandle, col: usize, hidden: bool) {
    if let Some(table_col) = grid_table_column_at(grid_handle, col) {
        table_col.setHidden(hidden);
    }
}

pub fn grid_begin_inline_edit(grid_handle: NativeHandle) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    if !grid_cell_editing_enabled(grid_handle) {
        return false;
    }
    let Some((row, data_col)) = grid_get_focused_cell(grid_handle) else {
        return false;
    };
    if grid_is_column_hidden(grid_handle, data_col) {
        return false;
    }
    grid_end_inline_edit(grid_handle, false);
    let Some(table) = grid_editing_table_view(grid_handle, row, data_col) else {
        return false;
    };
    let view_row = grid_display_row_to_view_row(grid_handle, &table, row);
    let table_col_idx = grid_table_leading_cols(grid_handle) + data_col;
    let rect = table.frameOfCellAtColumn_row(table_col_idx as NSInteger, view_row as NSInteger);
    let initial = grid_cell_display_text(grid_handle, row, data_col);
    let field = NSTextField::initWithFrame(NSTextField::alloc(mtm), rect);
    field.setStringValue(&NSString::from_str(&initial));
    field.setEditable(true);
    field.setBordered(true);
    field.setBezeled(true);
    field.setDrawsBackground(true);
    field.setFont(Some(&grid_body_font(mtm, grid_handle)));
    table.addSubview(&field);
    field.becomeFirstResponder();
    GRID_INLINE_EDITORS.with(|m| {
        m.borrow_mut().insert(grid_handle, (field, row, data_col));
    });
    true
}

pub fn grid_end_inline_edit(grid_handle: NativeHandle, accept: bool) -> bool {
    let editor = GRID_INLINE_EDITORS.with(|m| m.borrow_mut().remove(&grid_handle));
    let Some((field, row, data_col)) = editor else {
        return false;
    };
    let text = field.stringValue().to_string();
    field.removeFromSuperview();
    if accept {
        grid_record_cell_edited(grid_handle, row, data_col, text);
        if let Some((frame_handle, command_id)) =
            GRID_INLINE_FRAME.with(|m| m.borrow().get(&grid_handle).copied())
        {
            dispatch_notify(frame_handle, command_id, GRID_NOTIFY_CELL_EDITED);
        }
    }
    true
}

pub fn grid_store_notify_targets(
    grid_handle: NativeHandle,
    frame_handle: NativeHandle,
    command_id: u16,
) {
    GRID_INLINE_FRAME.with(|m| {
        m.borrow_mut()
            .insert(grid_handle, (frame_handle, command_id));
    });
}

fn grid_table_notify_targets(grid_handle: NativeHandle) -> (NativeHandle, u16) {
    GRID_TABLES.with(|m| {
        if let Some(table) = m.borrow().get(&grid_handle) {
            (
                table.ivars().frame_handle.get(),
                table.ivars().command_id.get(),
            )
        } else {
            GRID_INLINE_FRAME.with(|f| f.borrow().get(&grid_handle).copied().unwrap_or((0, 0)))
        }
    })
}

fn grid_frozen_columns_width(grid_handle: NativeHandle, count: usize) -> f64 {
    let mut w = 0.0;
    if grid_checkboxes_enabled(grid_handle) {
        w += 22.0;
    }
    if grid_row_labels_enabled(grid_handle) {
        w += grid_row_label_width(grid_handle).max(20) as f64;
    }
    GRID_COLUMNS.with(|m| {
        if let Some(cols) = m.borrow().get(&grid_handle) {
            for (_i, (_, width, _)) in cols.borrow().iter().enumerate().take(count) {
                w += if *width > 0 { *width as f64 } else { 80.0 };
            }
        }
    });
    w.max(40.0)
}

fn wire_grid_table(grid_handle: NativeHandle, table: &GridTableView) {
    GRID_DELEGATES.with(|m| {
        if let Some(controller) = m.borrow().get(&grid_handle) {
            let data_source = ProtocolObject::from_ref(&**controller);
            let delegate = ProtocolObject::from_ref(&**controller);
            unsafe {
                table.setDataSource(Some(data_source));
                table.setDelegate(Some(delegate));
            }
        }
    });
}

fn grid_rebuild_frozen_col_columns(grid_handle: NativeHandle, count: usize) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    GRID_FROZEN_COL_TABLES.with(|map| {
        let Some(table) = map.borrow().get(&grid_handle).cloned() else {
            return;
        };
        while table.numberOfColumns() > 0 {
            let cols = table.tableColumns();
            if let Some(col) = cols.firstObject() {
                table.removeTableColumn(&col);
            } else {
                break;
            }
        }
        if grid_checkboxes_enabled(grid_handle) {
            let col = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &NSString::from_str(GRID_CHECKBOX_COL_ID),
            );
            col.setTitle(&NSString::from_str(""));
            col.setWidth(22.0);
            col.setMinWidth(22.0);
            col.setMaxWidth(28.0);
            col.setResizingMask(NSTableColumnResizingOptions::NoResizing);
            table.addTableColumn(&col);
        }
        if grid_row_labels_enabled(grid_handle) {
            let w = grid_row_label_width(grid_handle).max(20) as f64;
            let col = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &NSString::from_str(GRID_ROW_LABEL_COL_ID),
            );
            col.setTitle(&NSString::from_str("#"));
            col.setWidth(w);
            col.setMinWidth(20.0);
            col.setMaxWidth(80.0);
            col.setResizingMask(NSTableColumnResizingOptions::NoResizing);
            table.addTableColumn(&col);
        }
        let columns = GRID_COLUMNS.with(|m| {
            m.borrow()
                .get(&grid_handle)
                .map(|c| c.borrow().clone())
                .unwrap_or_default()
        });
        for (i, (title, width, _align)) in columns.iter().enumerate().take(count) {
            let col = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &grid_col_identifier(i),
            );
            col.setTitle(&NSString::from_str(title));
            let w = if *width > 0 { *width as f64 } else { 80.0 };
            col.setWidth(w);
            col.setMinWidth(GRID_MIN_COLUMN_WIDTH as f64);
            col.setResizingMask(NSTableColumnResizingOptions::UserResizingMask);
            table.addTableColumn(&col);
        }
    });
}

fn ensure_frozen_col_pane(mtm: MainThreadMarker, grid_handle: NativeHandle, count: usize) {
    if GRID_FROZEN_COL_TABLES.with(|m| m.borrow().contains_key(&grid_handle)) {
        grid_rebuild_frozen_col_columns(grid_handle, count);
        return;
    }
    let Some(main_scroll) =
        grid_clone_root_view(grid_handle).and_then(|v| v.downcast::<NSScrollView>().ok())
    else {
        return;
    };
    GRID_MAIN_SCROLLS.with(|m| {
        m.borrow_mut().insert(grid_handle, main_scroll.clone());
    });
    let Some(parent) = (unsafe { main_scroll.superview() }) else {
        return;
    };
    let full = main_scroll.frame();
    let frozen_w = grid_frozen_columns_width(grid_handle, count);
    let (frame_handle, command_id) = grid_table_notify_targets(grid_handle);

    let mut main_frame = full;
    main_frame.origin.x += frozen_w;
    main_frame.size.width = (full.size.width - frozen_w).max(40.0);
    main_scroll.setFrame(main_frame);

    let mut frozen_frame = full;
    frozen_frame.size.width = frozen_w;
    let frozen_scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frozen_frame);
    frozen_scroll.setHasVerticalScroller(false);
    frozen_scroll.setHasHorizontalScroller(false);
    frozen_scroll.setAutohidesScrollers(true);
    frozen_scroll.setBorderType(objc2_app_kit::NSBorderType::BezelBorder);

    let frozen_table = GridTableView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), frozen_frame.size),
        grid_handle,
        frame_handle,
        command_id,
        GRID_PANE_FROZEN_COL,
    );
    wire_grid_table(grid_handle, &frozen_table);
    frozen_scroll.setDocumentView(Some(&frozen_table));
    parent.addSubview(&frozen_scroll);

    GRID_FROZEN_COL_SCROLLS.with(|m| {
        m.borrow_mut().insert(grid_handle, frozen_scroll);
    });
    GRID_FROZEN_COL_TABLES.with(|m| {
        m.borrow_mut().insert(grid_handle, frozen_table);
    });
    grid_rebuild_frozen_col_columns(grid_handle, count);
}

fn remove_frozen_col_pane(grid_handle: NativeHandle) {
    if let Some(frozen_scroll) =
        GRID_FROZEN_COL_SCROLLS.with(|m| m.borrow_mut().remove(&grid_handle))
    {
        frozen_scroll.removeFromSuperview();
    }
    GRID_FROZEN_COL_TABLES.with(|m| {
        m.borrow_mut().remove(&grid_handle);
    });
    if let Some(main_scroll) = GRID_MAIN_SCROLLS.with(|m| m.borrow().get(&grid_handle).cloned()) {
        if let Some(parent) = unsafe { main_scroll.superview() } {
            let full = parent.bounds();
            main_scroll.setFrame(full);
        }
    }
}

fn ensure_frozen_row_top_pane(mtm: MainThreadMarker, grid_handle: NativeHandle, count: usize) {
    if GRID_FROZEN_ROW_TOP_TABLES.with(|m| m.borrow().contains_key(&grid_handle)) {
        return;
    }
    let main_scroll = GRID_MAIN_SCROLLS
        .with(|m| m.borrow().get(&grid_handle).cloned())
        .or_else(|| {
            grid_clone_root_view(grid_handle).and_then(|v| v.downcast::<NSScrollView>().ok())
        });
    let Some(main_scroll) = main_scroll else {
        return;
    };
    GRID_MAIN_SCROLLS.with(|m| {
        m.borrow_mut().insert(grid_handle, main_scroll.clone());
    });
    let Some(parent) = (unsafe { main_scroll.superview() }) else {
        return;
    };
    let full = main_scroll.frame();
    let row_h = grid_row_height(grid_handle);
    let top_h = (row_h * count as f64 + 28.0).min(full.size.height * 0.5);
    let (frame_handle, command_id) = grid_table_notify_targets(grid_handle);

    let mut main_frame = full;
    main_frame.origin.y += top_h;
    main_frame.size.height = (full.size.height - top_h).max(row_h);
    main_scroll.setFrame(main_frame);

    let top_frame = NSRect::new(
        NSPoint::new(full.origin.x, full.origin.y + full.size.height - top_h),
        NSSize::new(full.size.width, top_h),
    );
    let top_scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), top_frame);
    top_scroll.setHasVerticalScroller(false);
    top_scroll.setHasHorizontalScroller(false);
    top_scroll.setBorderType(objc2_app_kit::NSBorderType::BezelBorder);

    let top_table = GridTableView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), top_frame.size),
        grid_handle,
        frame_handle,
        command_id,
        GRID_PANE_FROZEN_ROW_TOP,
    );
    wire_grid_table(grid_handle, &top_table);
    top_scroll.setDocumentView(Some(&top_table));
    parent.addSubview(&top_scroll);

    GRID_FROZEN_ROW_TOP_SCROLLS.with(|m| {
        m.borrow_mut().insert(grid_handle, top_scroll);
    });
    GRID_FROZEN_ROW_TOP_TABLES.with(|m| {
        m.borrow_mut().insert(grid_handle, top_table);
    });
    grid_rebuild_table_columns(grid_handle);
}

fn remove_frozen_row_top_pane(grid_handle: NativeHandle) {
    if let Some(top_scroll) =
        GRID_FROZEN_ROW_TOP_SCROLLS.with(|m| m.borrow_mut().remove(&grid_handle))
    {
        top_scroll.removeFromSuperview();
    }
    GRID_FROZEN_ROW_TOP_TABLES.with(|m| {
        m.borrow_mut().remove(&grid_handle);
    });
    if let Some(main_scroll) = GRID_MAIN_SCROLLS.with(|m| m.borrow().get(&grid_handle).cloned()) {
        if let Some(parent) = unsafe { main_scroll.superview() } {
            let full = parent.bounds();
            main_scroll.setFrame(full);
        }
    }
}

thread_local! {
    static GRID_SCROLL_SYNC_OBSERVERS: RefCell<
        HashMap<NativeHandle, Retained<GridScrollSyncObserver>>,
    > = RefCell::new(HashMap::new());
}

#[derive(Default)]
struct GridScrollSyncObserverIvars {
    grid_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = GridScrollSyncObserverIvars]
    struct GridScrollSyncObserver;

    unsafe impl NSObjectProtocol for GridScrollSyncObserver {}

    impl GridScrollSyncObserver {
        #[unsafe(method(gridScrollClipBoundsDidChange:))]
        fn clip_bounds_did_change(&self, _note: &NSNotification) {
            let handle = self.ivars().grid_handle.get();
            grid_sync_frozen_scroll_offsets(handle);
        }
    }
);

impl GridScrollSyncObserver {
    fn new(mtm: MainThreadMarker, grid_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(GridScrollSyncObserverIvars {
            grid_handle: Cell::new(grid_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn grid_sync_frozen_scroll_offsets(grid_handle: NativeHandle) {
    let Some(main_scroll) = GRID_MAIN_SCROLLS.with(|m| m.borrow().get(&grid_handle).cloned())
    else {
        return;
    };
    let main_clip = main_scroll.contentView();
    let origin = main_clip.bounds().origin;

    if let Some(frozen_scroll) =
        GRID_FROZEN_COL_SCROLLS.with(|m| m.borrow().get(&grid_handle).cloned())
    {
        let clip = frozen_scroll.contentView();
        let mut bounds = clip.bounds();
        if (bounds.origin.y - origin.y).abs() > 0.5 {
            bounds.origin.y = origin.y;
            clip.scrollToPoint(bounds.origin);
            frozen_scroll.reflectScrolledClipView(&clip);
        }
    }

    if let Some(top_scroll) =
        GRID_FROZEN_ROW_TOP_SCROLLS.with(|m| m.borrow().get(&grid_handle).cloned())
    {
        let clip = top_scroll.contentView();
        let mut bounds = clip.bounds();
        if (bounds.origin.x - origin.x).abs() > 0.5 {
            bounds.origin.x = origin.x;
            clip.scrollToPoint(bounds.origin);
            top_scroll.reflectScrolledClipView(&clip);
        }
    }
}

pub fn grid_install_frozen_scroll_sync(grid_handle: NativeHandle) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if GRID_SCROLL_SYNC_OBSERVERS.with(|m| m.borrow().contains_key(&grid_handle)) {
        return;
    }
    let Some(main_scroll) = GRID_MAIN_SCROLLS.with(|m| m.borrow().get(&grid_handle).cloned())
    else {
        return;
    };
    let clip = main_scroll.contentView();
    clip.setPostsBoundsChangedNotifications(true);
    let observer = GridScrollSyncObserver::new(mtm, grid_handle);
    let center = NSNotificationCenter::defaultCenter();
    unsafe {
        center.addObserver_selector_name_object(
            &observer,
            sel!(gridScrollClipBoundsDidChange:),
            Some(NSViewBoundsDidChangeNotification),
            Some(&clip),
        );
    }
    GRID_SCROLL_SYNC_OBSERVERS.with(|m| {
        m.borrow_mut().insert(grid_handle, observer);
    });
}

pub fn grid_remove_frozen_scroll_sync(grid_handle: NativeHandle) {
    let center = NSNotificationCenter::defaultCenter();
    GRID_SCROLL_SYNC_OBSERVERS.with(|m| {
        if let Some(observer) = m.borrow_mut().remove(&grid_handle) {
            unsafe {
                center.removeObserver(&observer);
            }
        }
    });
}

pub fn grid_apply_header_appearance(grid_handle: NativeHandle) {
    let appearance = super::grid_appearance(grid_handle);
    if appearance.system_theme {
        return;
    }
    let bg = ns_color_from_rgba(appearance.header_background);
    let fg = ns_color_from_rgba(appearance.header_foreground);
    for table in grid_all_table_views(grid_handle) {
        let count = table.numberOfColumns();
        for idx in 0..count {
            let col = table.tableColumns().objectAtIndex(idx as NSUInteger);
            let cell = col.headerCell();
            cell.setBackgroundColor(Some(&bg));
            cell.setTextColor(Some(&fg));
        }
        if let Some(header) = table.headerView() {
            header.setNeedsDisplay(true);
        }
    }
}

pub fn grid_sync_frozen_panes(grid_handle: NativeHandle) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let frozen_cols = grid_frozen_col_count(grid_handle).min(grid_col_count(grid_handle));
    let frozen_rows = grid_frozen_row_count(grid_handle);
    if frozen_cols > 0 {
        ensure_frozen_col_pane(mtm, grid_handle, frozen_cols);
    } else {
        remove_frozen_col_pane(grid_handle);
    }
    if frozen_rows > 0 {
        ensure_frozen_row_top_pane(mtm, grid_handle, frozen_rows);
    } else {
        remove_frozen_row_top_pane(grid_handle);
    }
    grid_rebuild_table_columns(grid_handle);
    if frozen_cols > 0 {
        grid_rebuild_frozen_col_columns(grid_handle, frozen_cols);
    }
    grid_reload(grid_handle);
    grid_install_frozen_scroll_sync(grid_handle);
    grid_sync_frozen_scroll_offsets(grid_handle);
}

pub fn grid_apply_row_drag(grid_handle: NativeHandle) {
    let enabled = grid_row_drag_enabled(grid_handle);
    let types = NSArray::from_slice(&[unsafe { NSPasteboardTypeString }]);
    for table in grid_all_table_views(grid_handle) {
        if enabled {
            table.setDraggingSourceOperationMask_forLocal(NSDragOperation::Move, true);
            table.registerForDraggedTypes(&types);
        }
    }
}
