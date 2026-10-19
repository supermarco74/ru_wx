//! macOS ListBox backed by native [`NSTableView`] (single-column, view-based cells).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBorderType, NSControlTextEditingDelegate, NSScrollView, NSTableCellView, NSTableColumn,
    NSTableColumnResizingOptions, NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewSelectionHighlightStyle, NSTableViewStyle, NSTextField,
    NSUserInterfaceItemIdentification, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSIndexSet, NSInteger, NSNotFound, NSObject, NSObjectProtocol, NSPoint,
    NSRect, NSSize, NSString, NSUInteger,
};

use super::{
    alloc_widget_handle, dispatch_command, dispatch_command_notify, listbox_appearance,
    parent_view_for, record_parent, resolve_frame_handle, ru_rect_to_ns, store_ns_view,
    NativeHandle, Rect,
};

thread_local! {
    static LISTBOX_TABLES: RefCell<HashMap<NativeHandle, Retained<NSTableView>>> =
        RefCell::new(HashMap::new());
    static LISTBOX_TABLE_CONTROLLERS: RefCell<HashMap<NativeHandle, Retained<ListBoxTableController>>> =
        RefCell::new(HashMap::new());
}

const LISTBOX_CELL_ID: &str = "RuWxListBoxCell";
const LISTBOX_COL_ID: &str = "listbox_col";

pub(super) fn listbox_is_native_table(list_handle: NativeHandle) -> bool {
    LISTBOX_TABLES.with(|map| map.borrow().contains_key(&list_handle))
}

fn listbox_table_view(list_handle: NativeHandle) -> Option<Retained<NSTableView>> {
    LISTBOX_TABLES.with(|map| map.borrow().get(&list_handle).cloned())
}

fn index_set_to_rows(indexes: Retained<NSIndexSet>) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut idx = indexes.firstIndex();
    let not_found = NSNotFound as NSUInteger;
    while idx != not_found {
        rows.push(idx as usize);
        let next = indexes.indexGreaterThanIndex(idx);
        if next == not_found {
            break;
        }
        idx = next;
    }
    rows
}

fn listbox_reuse_or_new_cell(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &ListBoxTableController,
    build: impl FnOnce(MainThreadMarker) -> Retained<NSTableCellView>,
) -> Retained<NSTableCellView> {
    let id = NSString::from_str(LISTBOX_CELL_ID);
    if let Some(view) = unsafe { table.makeViewWithIdentifier_owner(&id, Some(owner)) } {
        if let Ok(cell) = view.downcast::<NSTableCellView>() {
            return cell;
        }
    }
    let cell = build(mtm);
    unsafe {
        cell.setIdentifier(Some(&id));
    }
    cell
}

#[derive(Default)]
struct ListBoxTableControllerIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ListBoxTableControllerIvars]
    struct ListBoxTableController;

    unsafe impl NSObjectProtocol for ListBoxTableController {}

    unsafe impl NSControlTextEditingDelegate for ListBoxTableController {}

    unsafe impl NSTableViewDataSource for ListBoxTableController {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows_in_table_view(&self, _table_view: &NSTableView) -> NSInteger {
            let handle = self.ivars().list_handle.get();
            super::listbox_item_count(handle) as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for ListBoxTableController {
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
            let row_usize = row as usize;
            let text = super::listbox_item_text(handle, row_usize);
            let theme = listbox_appearance(handle);
            let row_h = theme.row_height;
            let font_size = (row_h * 0.52).clamp(11.0, 15.0);
            let selected = table_view
                .selectedRowIndexes()
                .containsIndex(row as NSUInteger);
            let width = table_column.unwrap().width();

            let cell = listbox_reuse_or_new_cell(mtm, table_view, self, |mtm| {
                NSTableCellView::new(mtm)
            });
            while let Some(sub) = cell.subviews().firstObject() {
                sub.removeFromSuperview();
            }

            let label = NSTextField::labelWithString(&NSString::from_str(&text), mtm);
            label.setFrame(NSRect::new(
                NSPoint::new(4.0, 0.0),
                NSSize::new((width - 8.0).max(8.0), row_h),
            ));
            label.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(font_size)));
            label.setDrawsBackground(true);
            if selected {
                label.setBackgroundColor(Some(&super::ns_color_from_rgba(theme.selection_bg)));
                label.setTextColor(Some(&super::ns_color_from_rgba(theme.selection_fg)));
            } else {
                let bg = if row_usize % 2 == 0 {
                    theme.background
                } else {
                    theme.alt_background
                };
                label.setBackgroundColor(Some(&super::ns_color_from_rgba(bg)));
                label.setTextColor(Some(&super::ns_color_from_rgba(theme.text)));
            }
            cell.addSubview(&label);
            unsafe {
                cell.setTextField(Some(&label));
            }
            cell.setFrameSize(NSSize::new(width, row_h));
            cell
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
            dispatch_command(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
            );
        }
    }

    impl ListBoxTableController {
        #[unsafe(method(listBoxDoubleClick:))]
        fn list_box_double_click(&self, table_view: &NSTableView) {
            let row = table_view.clickedRow();
            if row < 0 {
                return;
            }
            let handle = self.ivars().list_handle.get();
            super::listbox_write_selection(handle, vec![row as usize]);
            sync_selection_to_table(handle, table_view);
            dispatch_command_notify(
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                2,
            );
        }
    }
);

impl ListBoxTableController {
    fn new(
        mtm: MainThreadMarker,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ListBoxTableControllerIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn configure_table(
    table: &NSTableView,
    list_handle: NativeHandle,
    multi_select: bool,
    controller: &ListBoxTableController,
    mtm: MainThreadMarker,
) {
    table.setStyle(NSTableViewStyle::Plain);
    table.setColumnAutoresizingStyle(
        objc2_app_kit::NSTableViewColumnAutoresizingStyle::FirstColumnOnlyAutoresizingStyle,
    );
    table.setAllowsColumnReordering(false);
    table.setAllowsColumnResizing(false);
    table.setAllowsMultipleSelection(multi_select);
    table.setAllowsEmptySelection(true);
    table.setIntercellSpacing(NSSize::new(0.0, 0.0));
    table.setRowHeight(listbox_appearance(list_handle).row_height);
    table.setUsesAutomaticRowHeights(false);
    table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::Regular);
    table.setHeaderView(None);

    let id = NSString::from_str(LISTBOX_COL_ID);
    let col = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &id);
    col.setTitle(&NSString::from_str(""));
    col.setWidth(200.0);
    col.setMinWidth(24.0);
    col.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
    table.addTableColumn(&col);

    let ds = ProtocolObject::from_ref(controller);
    let del = ProtocolObject::from_ref(controller);
    unsafe {
        table.setDataSource(Some(ds));
        table.setDelegate(Some(del));
        table.setTarget(Some(controller.as_ref()));
        table.setDoubleAction(Some(sel!(listBoxDoubleClick:)));
    }
}

fn sync_selection_to_table(list_handle: NativeHandle, table: &NSTableView) {
    let selected = super::listbox_read_selection(list_handle);
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
    super::listbox_write_selection(list_handle, index_set_to_rows(table.selectedRowIndexes()));
}

pub fn create_list_box_native(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
    multi_select: bool,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    super::listbox_init_storage(handle, multi_select);

    let Some(content) = parent_view_for(parent) else {
        return handle;
    };

    let content_h = content.frame().size.height;
    let ns_rect = ru_rect_to_ns(content_h, &initial_rect);
    let frame_handle = resolve_frame_handle(parent);

    let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), ns_rect);
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setBorderType(NSBorderType::BezelBorder);
    scroll.setClipsToBounds(true);

    let table = NSTableView::initWithFrame(
        NSTableView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    let controller = ListBoxTableController::new(mtm, handle, frame_handle, command_id);
    configure_table(&table, handle, multi_select, &controller, mtm);

    scroll.setDocumentView(Some(&table));
    content.addSubview(&scroll);

    LISTBOX_TABLES.with(|map| map.borrow_mut().insert(handle, table));
    LISTBOX_TABLE_CONTROLLERS.with(|map| map.borrow_mut().insert(handle, controller));

    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn listbox_reload(list_handle: NativeHandle) {
    let Some(table) = listbox_table_view(list_handle) else {
        return;
    };
    let multi = super::listbox_is_multi_select(list_handle);
    table.setAllowsMultipleSelection(multi);
    table.setRowHeight(listbox_appearance(list_handle).row_height);
    table.reloadData();
    sync_selection_to_table(list_handle, &table);
}

pub fn listbox_apply_selection(list_handle: NativeHandle) {
    if let Some(table) = listbox_table_view(list_handle) {
        sync_selection_to_table(list_handle, &table);
        if let Some(idx) = super::listbox_read_selection(list_handle).first().copied() {
            if (idx as NSInteger) < table.numberOfRows() {
                table.scrollRowToVisible(idx as NSInteger);
            }
        }
    }
}
