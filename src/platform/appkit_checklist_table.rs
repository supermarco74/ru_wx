//! macOS CheckListBox backed by native [`NSTableView`] with per-row checkbox cells.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBorderType, NSButton, NSButtonType, NSControlStateValueOff, NSControlStateValueOn,
    NSControlTextEditingDelegate, NSScrollView, NSTableCellView, NSTableColumn,
    NSTableColumnResizingOptions, NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewSelectionHighlightStyle, NSTableViewStyle, NSTextField,
    NSUserInterfaceItemIdentification, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSIndexSet, NSInteger, NSNotFound, NSObject, NSObjectProtocol, NSPoint,
    NSRect, NSSize, NSString, NSUInteger,
};

use super::{
    alloc_widget_handle, dispatch_command, listbox_appearance, parent_view_for, record_parent,
    resolve_frame_handle, ru_rect_to_ns, store_ns_view, NativeHandle, Rect,
};

thread_local! {
    static CHECKLIST_TABLES: RefCell<HashMap<NativeHandle, Retained<NSTableView>>> =
        RefCell::new(HashMap::new());
    static CHECKLIST_CONTROLLERS: RefCell<HashMap<NativeHandle, Retained<CheckListTableController>>> =
        RefCell::new(HashMap::new());
    static CHECKLIST_CHECK_TARGETS: RefCell<HashMap<NativeHandle, Retained<CheckListCheckTarget>>> =
        RefCell::new(HashMap::new());
}

const CHECKLIST_CELL_ID: &str = "RuWxCheckListCell";
const CHECKLIST_COL_ID: &str = "checklist_col";
const CHECKBOX_SIZE: f64 = 18.0;

fn checklist_table_view(list_handle: NativeHandle) -> Option<Retained<NSTableView>> {
    CHECKLIST_TABLES.with(|map| map.borrow().get(&list_handle).cloned())
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

fn checklist_reuse_or_new_cell(
    mtm: MainThreadMarker,
    table: &NSTableView,
    owner: &CheckListTableController,
    build: impl FnOnce(MainThreadMarker) -> Retained<NSTableCellView>,
) -> Retained<NSTableCellView> {
    let id = NSString::from_str(CHECKLIST_CELL_ID);
    if let Some(view) = unsafe { table.makeViewWithIdentifier_owner(&id, Some(owner)) } {
        if let Ok(cell) = view.downcast::<NSTableCellView>() {
            return cell;
        }
    }
    let cell = build(mtm);
    cell.setIdentifier(Some(&id));
    cell
}

fn fire_check_toggle(
    handle: NativeHandle,
    frame_handle: NativeHandle,
    command_id: u16,
    row: usize,
    table: &NSTableView,
) {
    super::checklist_toggle_checked(handle, row);
    super::checklist_set_last_row(handle, row);
    unsafe {
        table.deselectAll(None);
    }
    super::checklist_write_selection(handle, Vec::new());
    table.reloadData();
    dispatch_command(frame_handle, command_id);
}

#[derive(Default)]
struct CheckListCheckTargetIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = CheckListCheckTargetIvars]
    struct CheckListCheckTarget;

    unsafe impl NSObjectProtocol for CheckListCheckTarget {}

    impl CheckListCheckTarget {
        #[unsafe(method(ruWxCheckListCheckClicked:))]
        fn check_clicked(&self, sender: Option<&AnyObject>) {
            let row = sender
                .and_then(|s| s.downcast_ref::<NSButton>())
                .map(|b| b.tag())
                .unwrap_or(-1);
            if row < 0 {
                return;
            }
            let handle = self.ivars().list_handle.get();
            let Some(table) = checklist_table_view(handle) else {
                return;
            };
            fire_check_toggle(
                handle,
                self.ivars().frame_handle.get(),
                self.ivars().command_id.get(),
                row as usize,
                &table,
            );
        }
    }
);

impl CheckListCheckTarget {
    fn new(
        mtm: MainThreadMarker,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(CheckListCheckTargetIvars {
            list_handle: Cell::new(list_handle),
            frame_handle: Cell::new(frame_handle),
            command_id: Cell::new(command_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn check_target_for(list_handle: NativeHandle) -> Option<Retained<CheckListCheckTarget>> {
    CHECKLIST_CHECK_TARGETS.with(|map| map.borrow().get(&list_handle).cloned())
}

#[derive(Default)]
struct CheckListTableControllerIvars {
    list_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    command_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = CheckListTableControllerIvars]
    struct CheckListTableController;

    unsafe impl NSObjectProtocol for CheckListTableController {}

    unsafe impl NSControlTextEditingDelegate for CheckListTableController {}

    unsafe impl NSTableViewDataSource for CheckListTableController {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows_in_table_view(&self, _table_view: &NSTableView) -> NSInteger {
            let handle = self.ivars().list_handle.get();
            super::checklist_item_count(handle) as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for CheckListTableController {
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
                let text = super::checklist_item_text(handle, row_usize);
                let checked = super::checklist_is_checked(handle, row_usize);
                let theme = listbox_appearance(handle);
                let row_h = theme.row_height;
                let font_size = (row_h * 0.52).clamp(11.0, 15.0);
                let width = table_column.unwrap().width();
                let check_target = check_target_for(handle).expect("check target");

                let cell = checklist_reuse_or_new_cell(mtm, table_view, self, |mtm| {
                    NSTableCellView::new(mtm)
                });
                while let Some(sub) = cell.subviews().firstObject() {
                    sub.removeFromSuperview();
                }

                let check = unsafe {
                    NSButton::buttonWithTitle_target_action(
                        &NSString::from_str(""),
                        Some(&check_target),
                        Some(sel!(ruWxCheckListCheckClicked:)),
                        mtm,
                    )
                };
                check.setButtonType(NSButtonType::Switch);
                check.setTag(row);
                check.setState(if checked {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
                check.setFrame(NSRect::new(
                    NSPoint::new(4.0, (row_h - CHECKBOX_SIZE).max(0.0) / 2.0),
                    NSSize::new(CHECKBOX_SIZE, CHECKBOX_SIZE),
                ));

                let label_x = CHECKBOX_SIZE + 8.0;
                let label = NSTextField::labelWithString(&NSString::from_str(&text), mtm);
                label.setFrame(NSRect::new(
                    NSPoint::new(label_x, 0.0),
                    NSSize::new((width - label_x - 4.0).max(8.0), row_h),
                ));
                label.setFont(Some(&objc2_app_kit::NSFont::systemFontOfSize(font_size)));
                label.setDrawsBackground(true);
                let bg = if row_usize.is_multiple_of(2) {
                    theme.background
                } else {
                    theme.alt_background
                };
                label.setBackgroundColor(Some(&super::ns_color_from_rgba(bg)));
                label.setTextColor(Some(&super::ns_color_from_rgba(theme.text)));

                cell.addSubview(&check);
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
            let frame_handle = self.ivars().frame_handle.get();
            let command_id = self.ivars().command_id.get();
            if let Some(obj) = notification.object() {
                if let Some(table) = obj.downcast_ref::<NSTableView>() {
                    let rows = index_set_to_rows(table.selectedRowIndexes());
                    if rows.len() == 1 {
                        fire_check_toggle(handle, frame_handle, command_id, rows[0], table);
                    }
                }
            }
        }
    }
);

impl CheckListTableController {
    fn new(
        mtm: MainThreadMarker,
        list_handle: NativeHandle,
        frame_handle: NativeHandle,
        command_id: u16,
    ) -> Retained<Self> {
        let check_target = CheckListCheckTarget::new(mtm, list_handle, frame_handle, command_id);
        CHECKLIST_CHECK_TARGETS.with(|map| {
            map.borrow_mut().insert(list_handle, check_target);
        });
        let this = Self::alloc(mtm).set_ivars(CheckListTableControllerIvars {
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
    controller: &CheckListTableController,
    mtm: MainThreadMarker,
) {
    table.setStyle(NSTableViewStyle::Plain);
    table.setColumnAutoresizingStyle(
        objc2_app_kit::NSTableViewColumnAutoresizingStyle::FirstColumnOnlyAutoresizingStyle,
    );
    table.setAllowsColumnReordering(false);
    table.setAllowsColumnResizing(false);
    table.setAllowsMultipleSelection(false);
    table.setAllowsEmptySelection(true);
    table.setIntercellSpacing(NSSize::new(0.0, 0.0));
    table.setRowHeight(listbox_appearance(list_handle).row_height);
    table.setUsesAutomaticRowHeights(false);
    table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::None);
    table.setHeaderView(None);

    let id = NSString::from_str(CHECKLIST_COL_ID);
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
    }
}

fn sync_selection_to_table(list_handle: NativeHandle, table: &NSTableView) {
    let selected = super::checklist_read_selection(list_handle);
    if selected.is_empty() {
        unsafe {
            table.deselectAll(None);
        }
        return;
    }
    if let Some(idx) = selected.first() {
        let set = NSIndexSet::indexSetWithIndex(*idx as NSUInteger);
        table.selectRowIndexes_byExtendingSelection(&set, false);
    }
}

pub fn create_check_list_box_native(
    parent: NativeHandle,
    command_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();

    super::checklist_init_storage(handle);

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
    let controller = CheckListTableController::new(mtm, handle, frame_handle, command_id);
    configure_table(&table, handle, &controller, mtm);

    scroll.setDocumentView(Some(&table));
    content.addSubview(&scroll);

    CHECKLIST_TABLES.with(|map| map.borrow_mut().insert(handle, table));
    CHECKLIST_CONTROLLERS.with(|map| map.borrow_mut().insert(handle, controller));

    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);
    handle
}

pub fn checklist_reload(list_handle: NativeHandle) {
    let Some(table) = checklist_table_view(list_handle) else {
        return;
    };
    table.setRowHeight(listbox_appearance(list_handle).row_height);
    table.reloadData();
    sync_selection_to_table(list_handle, &table);
}

pub fn checklist_apply_selection(list_handle: NativeHandle) {
    if let Some(table) = checklist_table_view(list_handle) {
        sync_selection_to_table(list_handle, &table);
    }
}
