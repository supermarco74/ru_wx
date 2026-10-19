//! macOS ListCtrl backed by native [`NSTableView`] (view-based cells).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::NSDraggingInfo;
use objc2_app_kit::{
    NSBorderType, NSButton, NSControlStateValueOff, NSControlStateValueOn,
    NSControlTextEditingDelegate, NSDragOperation, NSEvent, NSImage, NSImageScaling, NSImageView,
    NSPasteboard, NSPasteboardTypeString, NSScrollView, NSTableCellView, NSTableColumn,
    NSTableColumnResizingOptions, NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewDropOperation, NSTableViewGridLineStyle, NSTableViewSelectionHighlightStyle,
    NSTableViewStyle, NSTextField, NSUserInterfaceItemIdentification, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSIndexSet, NSInteger, NSNotFound, NSObject, NSObjectProtocol,
    NSPoint, NSRect, NSSize, NSString, NSUInteger,
};

use super::{
    alloc_widget_handle, dispatch_notify, listctrl_defer_notify_row, listctrl_record_row_reordered,
    listctrl_row_height, parent_view_for, record_parent, resolve_frame_handle, ru_rect_to_ns,
    store_ns_view, NativeHandle, Rect, LISTCTRL_NOTIFY_COLUMN_CLICK, LISTCTRL_NOTIFY_ITEM_CHANGED,
    LISTCTRL_NOTIFY_ITEM_DBLCLK, LISTCTRL_NOTIFY_ROW_REORDERED,
};

thread_local! {
    static LIST_TABLES: RefCell<HashMap<NativeHandle, Retained<NSTableView>>> =
        RefCell::new(HashMap::new());
    static LIST_TABLE_CONTROLLERS: RefCell<HashMap<NativeHandle, Retained<ListTableController>>> =
        RefCell::new(HashMap::new());
    static LIST_TABLE_META: RefCell<HashMap<NativeHandle, (NativeHandle, u16)>> =
        RefCell::new(HashMap::new());
}

const LIST_DATA_CELL_ID: &str = "RuWxListDataCell";
const LIST_CHECKBOX_CELL_ID: &str = "RuWxListCheckboxCell";
/// Identificatore della colonna checkbox (selezione multipla). Non è nella
/// forma `col_N`, quindi `data_col_for_column_id` la scarta: resta fuori da
/// ordinamento, notifiche di click colonna e accesso alle celle dati.
const CHECKBOX_COLUMN_ID: &str = "col_checkbox";

pub(super) fn listctrl_is_native_table(list_handle: NativeHandle) -> bool {
    LIST_TABLES.with(|map| map.borrow().contains_key(&list_handle))
}

fn list_table_view(list_handle: NativeHandle) -> Option<Retained<NSTableView>> {
    LIST_TABLES.with(|map| map.borrow().get(&list_handle).cloned())
}

fn data_col_for_column_id(id: &str) -> Option<usize> {
    id.strip_prefix("col_")?.parse().ok()
}

fn column_id_for_data_col(col: usize) -> String {
    format!("col_{col}")
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

fn list_reuse_or_new_cell(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &ListTableController,
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

#[derive(Default)]
struct ListTableViewIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSTableView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ListTableViewIvars]
    #[name = "RuWxListTableView"]
    struct ListTableView;

    unsafe impl NSObjectProtocol for ListTableView {}

    impl ListTableView {
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            let list_handle = self.ivars().list_handle.get();
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let row = self.rowAtPoint(point);
            if row >= 0 {
                let indexes = NSIndexSet::indexSetWithIndex(row as NSUInteger);
                self.selectRowIndexes_byExtendingSelection(&indexes, false);
                sync_selection_from_table(list_handle, self);
            }
            super::appkit_accelerator::dispatch_frame_context_menu(
                self.ivars().frame_handle.get(),
                event,
                self,
            );
        }
    }
);

impl ListTableView {
    fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ListTableViewIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
        });
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

#[derive(Default)]
struct ListTableControllerIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ListTableControllerIvars]
    struct ListTableController;

    unsafe impl NSObjectProtocol for ListTableController {}

    unsafe impl NSControlTextEditingDelegate for ListTableController {}

    unsafe impl NSTableViewDataSource for ListTableController {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows_in_table_view(&self, _table_view: &NSTableView) -> NSInteger {
            let handle = self.ivars().list_handle.get();
            super::listctrl_effective_row_count(handle) as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for ListTableController {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn table_view_view_for_table_column_row(
            &self,
            table_view: &NSTableView,
            table_column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Retained<NSView> {
            let mtm = MainThreadMarker::new().expect("main thread");
            let handle = self.ivars().list_handle.get();
            let cell_view = if table_column.is_none() || row < 0 {
                NSTableCellView::new(mtm)
            } else {
            let table_column = table_column.unwrap();
            if table_column.identifier().to_string() == CHECKBOX_COLUMN_ID {
                self.build_checkbox_cell(mtm, table_view, table_column, row)
            } else {
            let row_usize = row as usize;
            let col = data_col_for_column_id(&table_column.identifier().to_string()).unwrap_or(0);
            let text = super::listctrl_cell_text(handle, row_usize, col);
            let row_h = listctrl_row_height(handle);
            let font_size = (row_h * 0.52).clamp(11.0, 15.0);
            let full_row = super::listctrl_full_row_select(handle);
            let selected = table_view
                .selectedRowIndexes()
                .containsIndex(row as NSUInteger);

            let cell = list_reuse_or_new_cell(mtm, table_view, self, LIST_DATA_CELL_ID, |mtm| {
                NSTableCellView::new(mtm)
            });

            while let Some(sub) = cell.subviews().firstObject() {
                sub.removeFromSuperview();
            }

            if let Some(style) = super::listctrl_cell_style(handle, row_usize, col) {
                if let Some((r, g, b, a)) = style.background {
                    let bg = objc2_app_kit::NSColor::colorWithCalibratedRed_green_blue_alpha(
                        r as f64 / 255.0,
                        g as f64 / 255.0,
                        b as f64 / 255.0,
                        a as f64 / 255.0,
                    );
                    let bg_field = NSTextField::initWithFrame(
                        NSTextField::alloc(mtm),
                        NSRect::new(
                            NSPoint::new(0.0, 0.0),
                            NSSize::new(table_column.width(), row_h),
                        ),
                    );
                    bg_field.setEditable(false);
                    bg_field.setBezeled(false);
                    bg_field.setDrawsBackground(true);
                    bg_field.setBackgroundColor(Some(&bg));
                    bg_field.setStringValue(&NSString::from_str(""));
                    cell.addSubview(&bg_field);
                }
            }

            let mut x = 0.0;
            if col == 0 {
                if let Some(img_idx) = super::listctrl_row_image_index(handle, row_usize) {
                    if img_idx >= 0 {
                        if let Some(img) = super::listctrl_image_for_row(handle, img_idx) {
                            let icon_h = row_h - 4.0;
                            let icon = NSImageView::initWithFrame(
                                NSImageView::alloc(mtm),
                                NSRect::new(
                                    NSPoint::new(2.0, 2.0),
                                    NSSize::new(icon_h, icon_h),
                                ),
                            );
                            icon.setImage(Some(&img));
                            icon.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                            cell.addSubview(&icon);
                            x = icon_h + 6.0;
                        }
                    }
                }
            }

            let label = NSTextField::labelWithString(&NSString::from_str(&text), mtm);
            label.setFrame(NSRect::new(
                NSPoint::new(x, 0.0),
                NSSize::new((table_column.width() - x).max(8.0), row_h),
            ));
            let font = objc2_app_kit::NSFont::systemFontOfSize(font_size);
            label.setFont(Some(&font));
            if let Some(style) = super::listctrl_cell_style(handle, row_usize, col) {
                if let Some((r, g, b, a)) = style.foreground {
                    if !(selected && full_row) {
                        let fg = objc2_app_kit::NSColor::colorWithCalibratedRed_green_blue_alpha(
                            r as f64 / 255.0,
                            g as f64 / 255.0,
                            b as f64 / 255.0,
                            a as f64 / 255.0,
                        );
                        label.setTextColor(Some(&fg));
                    }
                }
            } else if selected && full_row {
                label.setTextColor(Some(&objc2_app_kit::NSColor::selectedTextColor()));
            }
            cell.addSubview(&label);
            unsafe {
                cell.setTextField(Some(&label));
            }
            cell.setFrameSize(NSSize::new(table_column.width(), row_h));
            cell
            }
            };
            Retained::into_super(cell_view)
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn table_view_selection_did_change(&self, notification: &objc2_foundation::NSNotification) {
            let handle = self.ivars().list_handle.get();
            if let Some(obj) = notification.object() {
                if let Some(table) = obj.downcast_ref::<NSTableView>() {
                    sync_selection_from_table(handle, table);
                }
            }
            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                LISTCTRL_NOTIFY_ITEM_CHANGED,
            );
        }

        #[unsafe(method(tableView:shouldSelectRow:))]
        fn table_view_should_select_row(
            &self,
            _table_view: &NSTableView,
            row: NSInteger,
        ) -> Bool {
            if row >= 0 {
                super::listctrl_set_last_hit(self.ivars().list_handle.get(), row as usize, 0);
            }
            Bool::YES
        }

        #[unsafe(method(tableView:didClickTableColumn:))]
        fn table_view_did_click_table_column(
            &self,
            _table_view: &NSTableView,
            table_column: &NSTableColumn,
        ) {
            // Click sull'header colonna: equivalente macOS di
            // `LVN_COLUMNCLICK`. L'indice colonna viaggia nel payload
            // della notifica differita e sarà decodificato da
            // `list_column_click_index` (shell.rs).
            if let Some(col) = data_col_for_column_id(&table_column.identifier().to_string()) {
                listctrl_defer_notify_row(
                    self.ivars().frame_handle.get(),
                    self.ivars().command_id.get(),
                    LISTCTRL_NOTIFY_COLUMN_CLICK,
                    col as isize,
                );
            }
        }
    }

    impl ListTableController {
        #[unsafe(method(tableView:writeRowsWithIndexes:toPasteboard:))]
        fn table_view_write_rows_with_indexes_to_pasteboard(
            &self,
            _table_view: &NSTableView,
            row_indexes: &NSIndexSet,
            pboard: &NSPasteboard,
        ) -> Bool {
            let handle = self.ivars().list_handle.get();
            if !super::listctrl_row_reorder_enabled(handle) {
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
            _info: &ProtocolObject<dyn NSDraggingInfo>,
            _row: NSInteger,
            drop_operation: NSTableViewDropOperation,
        ) -> NSDragOperation {
            let handle = self.ivars().list_handle.get();
            if !super::listctrl_row_reorder_enabled(handle)
                || drop_operation == NSTableViewDropOperation::On
            {
                return NSDragOperation::None;
            }
            NSDragOperation::Move
        }

        #[unsafe(method(tableView:acceptDrop:row:dropOperation:))]
        fn table_view_accept_drop(
            &self,
            _table_view: &NSTableView,
            info: &ProtocolObject<dyn NSDraggingInfo>,
            row: NSInteger,
            drop_operation: NSTableViewDropOperation,
        ) -> Bool {
            let handle = self.ivars().list_handle.get();
            if !super::listctrl_row_reorder_enabled(handle)
                || drop_operation == NSTableViewDropOperation::On
            {
                return Bool::NO;
            }
            let pboard = info.draggingPasteboard();
            let Some(raw) = pboard.stringForType(unsafe { NSPasteboardTypeString }) else {
                return Bool::NO;
            };
            let text = raw.to_string();
            let Some(from_str) = text.split(',').next() else {
                return Bool::NO;
            };
            let Ok(from) = from_str.trim().parse::<usize>() else {
                return Bool::NO;
            };
            let mut to = if row < 0 { 0 } else { row as usize };
            if from <= to {
                to = to.saturating_sub(1);
            }
            if from == to {
                return Bool::NO;
            }
            listctrl_record_row_reordered(handle, from, to);
            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                LISTCTRL_NOTIFY_ROW_REORDERED,
            );
            Bool::YES
        }

        #[unsafe(method(listTableDoubleClick:))]
        fn list_table_double_click(&self, table_view: &NSTableView) {
            let row = table_view.clickedRow();
            if row < 0 {
                return;
            }
            let handle = self.ivars().list_handle.get();
            super::listctrl_select_model(handle, row as usize);
            sync_selection_to_table(handle, table_view);
            listctrl_defer_notify_row(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                LISTCTRL_NOTIFY_ITEM_DBLCLK,
                row,
            );
        }

        #[unsafe(method(listTableCheckboxToggle:))]
        fn list_table_checkbox_toggle(&self, sender: &NSButton) {
            // La riga è nel tag del bottone (impostato da
            // `build_checkbox_cell`): i checkbox vivono dentro le celle
            // riciclate, quindi il tag va riassegnato a ogni costruzione.
            let row = sender.tag();
            if row < 0 {
                return;
            }
            let handle = self.ivars().list_handle.get();
            let checked = !super::listctrl_is_checked(handle, row as usize);
            super::listctrl_set_checked(handle, row as usize, checked);
            // Feedback immediato; il redraw differito riallinea lo stato.
            sender.setState(if checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    }
);

impl ListTableController {
    fn new(
        mtm: MainThreadMarker,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ListTableControllerIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }

    /// Cella della colonna checkbox: un `NSButton` a spunta centrato, con
    /// target il controller e la riga nel tag del bottone.
    fn build_checkbox_cell(
        &self,
        mtm: MainThreadMarker,
        table_view: &NSTableView,
        table_column: &NSTableColumn,
        row: NSInteger,
    ) -> Retained<NSTableCellView> {
        let handle = self.ivars().list_handle.get();
        let row_h = listctrl_row_height(handle);
        let cell = list_reuse_or_new_cell(mtm, table_view, self, LIST_CHECKBOX_CELL_ID, |mtm| {
            NSTableCellView::new(mtm)
        });
        while let Some(sub) = cell.subviews().firstObject() {
            sub.removeFromSuperview();
        }
        let cb = unsafe {
            NSButton::checkboxWithTitle_target_action(
                &NSString::from_str(""),
                Some(self),
                Some(sel!(listTableCheckboxToggle:)),
                mtm,
            )
        };
        let side = 16.0_f64;
        let col_w = table_column.width();
        cb.setFrame(NSRect::new(
            NSPoint::new(
                ((col_w - side) / 2.0).max(0.0),
                ((row_h - side) / 2.0).max(0.0),
            ),
            NSSize::new(side, side),
        ));
        cb.setTag(row);
        cb.setState(if super::listctrl_is_checked(handle, row as usize) {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        cell.addSubview(&cb);
        cell.setFrameSize(NSSize::new(col_w, row_h));
        cell
    }
}

fn configure_table(
    table: &NSTableView,
    list_handle: NativeHandle,
    controller: &ListTableController,
) {
    table.setStyle(NSTableViewStyle::FullWidth);
    table.setColumnAutoresizingStyle(
        objc2_app_kit::NSTableViewColumnAutoresizingStyle::LastColumnOnlyAutoresizingStyle,
    );
    table.setAllowsColumnReordering(false);
    table.setAllowsColumnResizing(true);
    table.setAllowsMultipleSelection(false);
    table.setIntercellSpacing(NSSize::new(0.0, 0.0));
    table.setRowHeight(listctrl_row_height(list_handle));
    table.setUsesAutomaticRowHeights(false);
    table.setGridStyleMask(
        NSTableViewGridLineStyle::SolidHorizontalGridLineMask
            | NSTableViewGridLineStyle::SolidVerticalGridLineMask,
    );
    // La selezione va sempre evidenziata: `None` disattiverebbe del
    // tutto l'highlight della riga cliccata.
    table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::Regular);
    let ds = ProtocolObject::from_ref(controller);
    let del = ProtocolObject::from_ref(controller);
    unsafe {
        table.setDataSource(Some(ds));
        table.setDelegate(Some(del));
        table.setTarget(Some(controller.as_ref()));
        table.setDoubleAction(Some(sel!(listTableDoubleClick:)));
    }
    listctrl_apply_row_reorder(list_handle);
}

pub fn create_list_ctrl_native(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    super::listctrl_init_storage(handle);

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let frame_handle = resolve_frame_handle(parent);

    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setHasHorizontalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);
    scroll.setClipsToBounds(true);

    let table = ListTableView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
        handle,
        frame_handle,
    );
    let controller = ListTableController::new(mtm, handle, frame_handle, command_id);
    configure_table(&table, handle, &controller);

    scroll.setDocumentView(Some(&table));
    content.addSubview(&scroll);

    LIST_TABLES.with(|map| map.borrow_mut().insert(handle, table.clone().into_super()));
    LIST_TABLE_CONTROLLERS.with(|map| map.borrow_mut().insert(handle, controller));
    LIST_TABLE_META.with(|map| {
        map.borrow_mut().insert(handle, (frame_handle, command_id));
    });

    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

/// Mostra/nasconde la freccia di sort sull'header della colonna `col`
/// usando l'`indicatorImage` nativa di `NSTableView`.
pub fn listctrl_set_header_sort_indicator(
    list_handle: NativeHandle,
    col: i32,
    ascending: bool,
    show: bool,
) {
    if col < 0 {
        return;
    }
    let Some(table) = list_table_view(list_handle) else {
        return;
    };
    let wanted = column_id_for_data_col(col as usize);
    let columns = table.tableColumns();
    for i in 0..columns.count() {
        let column = columns.objectAtIndex(i);
        if column.identifier().to_string() != wanted {
            continue;
        }
        let image = if show {
            let symbol = if ascending {
                "arrowtriangle.up.fill"
            } else {
                "arrowtriangle.down.fill"
            };
            NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str(symbol),
                None,
            )
        } else {
            None
        };
        table.setIndicatorImage_inTableColumn(image.as_deref(), &column);
    }
}

pub fn listctrl_rebuild_columns(list_handle: NativeHandle) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(table) = list_table_view(list_handle) else {
        return;
    };
    while table.tableColumns().count() > 0 {
        let col = table.tableColumns().objectAtIndex(0);
        table.removeTableColumn(&col);
    }
    let columns = super::listctrl_read_columns(list_handle);
    // Colonna checkbox in testa quando la selezione multipla è attiva:
    // larghezza fissa, senza titolo né indicatori di sort.
    if super::listctrl_checkboxes_enabled(list_handle) {
        let id = NSString::from_str(CHECKBOX_COLUMN_ID);
        let col = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &id);
        col.setTitle(&NSString::from_str(""));
        col.setWidth(26.0);
        col.setMinWidth(26.0);
        col.setMaxWidth(26.0);
        table.addTableColumn(&col);
    }
    for (i, (title, width)) in columns.iter().enumerate() {
        let id = NSString::from_str(&column_id_for_data_col(i));
        let col = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &id);
        col.setTitle(&NSString::from_str(title));
        let w = if *width > 0 { *width as f64 } else { 120.0 };
        col.setWidth(w);
        col.setMinWidth(24.0);
        col.setResizingMask(NSTableColumnResizingOptions::UserResizingMask);
        table.addTableColumn(&col);
    }
    if columns.is_empty() {
        let id = NSString::from_str(&column_id_for_data_col(0));
        let col = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &id);
        col.setTitle(&NSString::from_str(""));
        col.setWidth(120.0);
        table.addTableColumn(&col);
    }
}

pub fn listctrl_reload(list_handle: NativeHandle) {
    let Some(table) = list_table_view(list_handle) else {
        return;
    };
    table.setRowHeight(listctrl_row_height(list_handle));
    // Vedi `configure_table`: l'highlight deve restare attivo.
    table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::Regular);
    listctrl_rebuild_columns(list_handle);
    table.reloadData();
    sync_selection_to_table(list_handle, &table);
}

fn sync_selection_to_table(list_handle: NativeHandle, table: &NSTableView) {
    let selected = super::listctrl_read_selected(list_handle);
    if selected.is_empty() {
        unsafe {
            table.deselectAll(None);
        }
        return;
    }
    if selected.len() == 1 {
        let idx = NSIndexSet::indexSetWithIndex(selected[0] as NSUInteger);
        table.selectRowIndexes_byExtendingSelection(&idx, false);
        return;
    }
    let set = objc2_foundation::NSMutableIndexSet::new();
    for idx in selected {
        set.addIndex(idx as NSUInteger);
    }
    table.selectRowIndexes_byExtendingSelection(&set, false);
}

fn sync_selection_from_table(list_handle: NativeHandle, table: &NSTableView) {
    super::listctrl_write_selected(list_handle, index_set_to_rows(table.selectedRowIndexes()));
}

pub fn listctrl_apply_selection(list_handle: NativeHandle, index: usize) {
    if let Some(table) = list_table_view(list_handle) {
        sync_selection_to_table(list_handle, &table);
        if (index as NSInteger) < table.numberOfRows() {
            table.scrollRowToVisible(index as NSInteger);
        }
    }
}

pub fn listctrl_apply_row_reorder(list_handle: NativeHandle) {
    let Some(table) = list_table_view(list_handle) else {
        return;
    };
    let enabled = super::listctrl_row_reorder_enabled(list_handle);
    if enabled {
        table.setDraggingSourceOperationMask_forLocal(NSDragOperation::Move, true);
        let types = NSArray::from_slice(&[unsafe { NSPasteboardTypeString }]);
        table.registerForDraggedTypes(&types);
    }
}

/// Imposta la dimensione del font (in punti tipografici) della
/// `NSTableView` nativa. Usato dalle preferenze utente
/// (`Strumenti → Opzioni…`).
pub fn listctrl_set_font_size_native(list_handle: NativeHandle, point_size: f64) {
    let Some(table) = list_table_view(list_handle) else {
        return;
    };
    let pt = point_size.clamp(8.0, 28.0);
    let font = objc2_app_kit::NSFont::systemFontOfSize(pt);
    table.setFont(Some(&font));
    // L'altezza riga deve crescere se il font è più grande,
    // altrimenti le righe vengono tagliate.
    let line_h = (pt * 1.6).max(18.0);
    super::listctrl_set_row_height_inner(list_handle, line_h);
    table.setRowHeight(line_h);
    table.reloadData();
}
