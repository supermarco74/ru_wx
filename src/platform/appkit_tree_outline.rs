//! macOS TreeCtrl backed by native [`NSOutlineView`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBorderType, NSControlTextEditingDelegate, NSImageScaling, NSImageView, NSOutlineView,
    NSOutlineViewDataSource, NSOutlineViewDelegate, NSScrollView, NSTableCellView, NSTableColumn,
    NSTableColumnResizingOptions, NSTextField, NSUserInterfaceItemIdentification, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSInteger, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
    NSUInteger,
};

use super::{
    alloc_widget_handle, dispatch_notify, image_list_image_at, parent_view_for, record_parent,
    resolve_frame_handle, ru_rect_to_ns, store_ns_view, tree_child_at_for_item,
    tree_children_count_for_item, tree_collect_expanded_nodes, tree_image_list_for,
    tree_init_storage, tree_node_has_children, tree_node_image_index, tree_node_text,
    tree_read_selected, tree_set_expanded, tree_write_selected, NativeHandle, Rect,
    TREE_NOTIFY_SELCHANGED,
};

thread_local! {
    static TREE_OUTLINES: RefCell<HashMap<NativeHandle, Retained<NSOutlineView>>> =
        RefCell::new(HashMap::new());
    static TREE_OUTLINE_CONTROLLERS: RefCell<
        HashMap<NativeHandle, Retained<TreeOutlineController>>,
    > = RefCell::new(HashMap::new());
    static TREE_ITEM_STRINGS: RefCell<HashMap<usize, Retained<NSString>>> =
        RefCell::new(HashMap::new());
    /// Handle dei `NSEvent` local monitor registrati da
    /// `create_tree_ctrl_native` per forzare `TREE_NOTIFY_SELCHANGED`
    /// su click ripetuto. Rilasciati in `tree_cleanup` insieme al
    /// resto delle risorse della tree-view.
    static TREE_OUTLINE_MONITORS: RefCell<HashMap<NativeHandle, Retained<AnyObject>>> =
        RefCell::new(HashMap::new());
}

const TREE_ROW_HEIGHT: f64 = 20.0;
const TREE_INDENT: f64 = 16.0;
const TREE_ICON_WIDTH: f64 = 18.0;
const TREE_COLUMN_ID: &str = "TreeColumn";
const TREE_CELL_ID: &str = "RuWxTreeCell";
const TREE_ITEM_PREFIX: &str = "ruwx-tree-node:";

pub(super) fn tree_is_native_outline(tree_handle: NativeHandle) -> bool {
    TREE_OUTLINES.with(|map| map.borrow().contains_key(&tree_handle))
}

fn outline_view(tree_handle: NativeHandle) -> Option<Retained<NSOutlineView>> {
    TREE_OUTLINES.with(|map| map.borrow().get(&tree_handle).cloned())
}

fn item_for_node(node_id: usize) -> Retained<NSString> {
    TREE_ITEM_STRINGS.with(|map| {
        map.borrow_mut()
            .entry(node_id)
            .or_insert_with(|| NSString::from_str(&format!("{TREE_ITEM_PREFIX}{node_id}")))
            .clone()
    })
}

/// Release all cached strings for a tree (clears the entire cache).
/// Call this when a tree is destroyed.
pub fn tree_cleanup(_tree_handle: NativeHandle) {
    TREE_ITEM_STRINGS.with(|map| {
        map.borrow_mut().clear();
    });
    // Rilascia anche l'eventuale monitor NSEvent registrato per la
    // tree-view: senza drop, `addLocalMonitorForEventsMatchingMask`
    // lascia un riferimento forte che mantiene vivo il closure (e
    // cattura eventuali state) ben oltre la vita della tree.
    TREE_OUTLINE_MONITORS.with(|m| m.borrow_mut().clear());
}

fn node_id_from_item(item: &AnyObject) -> Option<usize> {
    item.downcast_ref::<NSString>().and_then(|s| {
        s.to_string()
            .strip_prefix(TREE_ITEM_PREFIX)
            .and_then(|raw| raw.parse().ok())
    })
}

fn item_from_notification(notification: &objc2_foundation::NSNotification) -> Option<usize> {
    notification
        .userInfo()
        .and_then(|info| info.objectForKey(&NSString::from_str("NSObject")))
        .and_then(|obj| node_id_from_item(&obj))
}

fn outline_reuse_or_new_cell(
    mtm: MainThreadMarker,
    outline: &NSOutlineView,
    owner: &TreeOutlineController,
    build: impl FnOnce(MainThreadMarker) -> Retained<NSTableCellView>,
) -> Retained<NSTableCellView> {
    let id = NSString::from_str(TREE_CELL_ID);
    if let Some(view) = unsafe { outline.makeViewWithIdentifier_owner(&id, Some(owner)) } {
        if let Ok(cell) = view.downcast::<NSTableCellView>() {
            return cell;
        }
    }
    let cell = build(mtm);
    cell.setIdentifier(Some(&id));
    cell
}

#[derive(Default)]
struct TreeOutlineControllerIvars {
    tree_handle: Cell<NativeHandle>,
    frame_handle: Cell<NativeHandle>,
    notify_id: Cell<u16>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TreeOutlineControllerIvars]
    struct TreeOutlineController;

    unsafe impl NSObjectProtocol for TreeOutlineController {}

    unsafe impl NSControlTextEditingDelegate for TreeOutlineController {}

    unsafe impl NSOutlineViewDataSource for TreeOutlineController {}

    unsafe impl NSOutlineViewDelegate for TreeOutlineController {
        #[unsafe(method_id(outlineView:viewForTableColumn:item:))]
        unsafe fn outline_view_view_for_table_column_item(
            &self,
            outline_view: &NSOutlineView,
            table_column: Option<&NSTableColumn>,
            item: &AnyObject,
        ) -> Retained<NSView> {
            let mtm = MainThreadMarker::new().expect("main thread");
            let handle = self.ivars().tree_handle.get();
            let cell_view = if let Some(node_id) = node_id_from_item(item) {
                if let Some(table_column) = table_column {
                    let text = tree_node_text(handle, node_id);
                    let list_h = tree_image_list_for(handle);
                    let cell = outline_reuse_or_new_cell(mtm, outline_view, self, |mtm| {
                        NSTableCellView::new(mtm)
                    });
                    while let Some(sub) = cell.subviews().firstObject() {
                        sub.removeFromSuperview();
                    }
                    let mut x = 0.0;
                    if let Some(img_idx) = tree_node_image_index(handle, node_id).filter(|i| *i >= 0)
                    {
                        if let Some(img) = image_list_image_at(list_h, img_idx) {
                            let icon_h = TREE_ROW_HEIGHT - 4.0;
                            let icon = NSImageView::initWithFrame(
                                NSImageView::alloc(mtm),
                                NSRect::new(
                                    NSPoint::new(x, 2.0),
                                    NSSize::new(icon_h, icon_h),
                                ),
                            );
                            icon.setImage(Some(&img));
                            icon.setImageScaling(NSImageScaling::ScaleProportionallyDown);
                            cell.addSubview(&icon);
                            x += TREE_ICON_WIDTH;
                        }
                    }
                    let label = NSTextField::labelWithString(&NSString::from_str(&text), mtm);
                    label.setFrame(NSRect::new(
                        NSPoint::new(x, 0.0),
                        NSSize::new((table_column.width() - x).max(20.0), TREE_ROW_HEIGHT),
                    ));
                    cell.addSubview(&label);
                    unsafe {
                        cell.setTextField(Some(&label));
                    }
                    cell.setFrameSize(NSSize::new(table_column.width(), TREE_ROW_HEIGHT));
                    cell
                } else {
                    NSTableCellView::new(mtm)
                }
            } else {
                NSTableCellView::new(mtm)
            };
            Retained::into_super(cell_view)
        }

        #[unsafe(method(outlineViewSelectionDidChange:))]
        fn outline_view_selection_did_change(
            &self,
            _notification: &objc2_foundation::NSNotification,
        ) {
            let handle = self.ivars().tree_handle.get();
            let Some(outline) = outline_view(handle) else {
                return;
            };
            let row = outline.selectedRow();
            if row < 0 {
                tree_write_selected(handle, None);
            } else if let Some(item) = outline.itemAtRow(row) {
                if let Some(node_id) = node_id_from_item(&item) {
                    tree_write_selected(handle, Some(node_id));
                }
            }
            dispatch_notify(
                self.ivars().frame_handle.get(),
                self.ivars().notify_id.get(),
                TREE_NOTIFY_SELCHANGED,
            );
        }

        #[unsafe(method(outlineViewItemDidExpand:))]
        fn outline_view_item_did_expand(
            &self,
            notification: &objc2_foundation::NSNotification,
        ) {
            let handle = self.ivars().tree_handle.get();
            if let Some(node_id) = item_from_notification(notification) {
                tree_set_expanded(handle, node_id, true);
            }
        }

        #[unsafe(method(outlineViewItemDidCollapse:))]
        fn outline_view_item_did_collapse(
            &self,
            notification: &objc2_foundation::NSNotification,
        ) {
            let handle = self.ivars().tree_handle.get();
            if let Some(node_id) = item_from_notification(notification) {
                tree_set_expanded(handle, node_id, false);
            }
        }
    }

    impl TreeOutlineController {
        #[unsafe(method(outlineView:numberOfChildrenOfItem:))]
        unsafe fn outline_view_number_of_children_of_item(
            &self,
            _outline_view: &NSOutlineView,
            item: Option<&AnyObject>,
        ) -> NSInteger {
            let handle = self.ivars().tree_handle.get();
            let parent = item.and_then(node_id_from_item);
            tree_children_count_for_item(handle, parent) as NSInteger
        }

        #[unsafe(method_id(outlineView:child:ofItem:))]
        unsafe fn outline_view_child_of_item(
            &self,
            _outline_view: &NSOutlineView,
            index: NSInteger,
            item: Option<&AnyObject>,
        ) -> Retained<NSString> {
            let handle = self.ivars().tree_handle.get();
            let parent = item.and_then(node_id_from_item);
            let node_id = tree_child_at_for_item(handle, parent, index as usize).unwrap_or(0);
            item_for_node(node_id)
        }

        #[unsafe(method(outlineView:isItemExpandable:))]
        unsafe fn outline_view_is_item_expandable(
            &self,
            _outline_view: &NSOutlineView,
            item: &AnyObject,
        ) -> bool {
            let handle = self.ivars().tree_handle.get();
            node_id_from_item(item)
                .map(|node_id| tree_node_has_children(handle, node_id))
                .unwrap_or(false)
        }
    }
);

impl TreeOutlineController {
    fn new(
        mtm: MainThreadMarker,
        tree_handle: NativeHandle,
        frame_handle: NativeHandle,
        notify_id: u16,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TreeOutlineControllerIvars {
            tree_handle: Cell::new(tree_handle),
            frame_handle: Cell::new(frame_handle),
            notify_id: Cell::new(notify_id),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn configure_outline(
    outline: &NSOutlineView,
    tree_handle: NativeHandle,
    controller: &TreeOutlineController,
    mtm: MainThreadMarker,
) {
    outline.setHeaderView(None);
    outline.setRowHeight(TREE_ROW_HEIGHT);
    outline.setIndentationPerLevel(TREE_INDENT);
    outline.setIndentationMarkerFollowsCell(true);
    outline.setAutoresizesOutlineColumn(true);
    outline.setAllowsMultipleSelection(false);
    outline.setIntercellSpacing(NSSize::new(0.0, 0.0));

    let col_id = NSString::from_str(TREE_COLUMN_ID);
    let col = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &col_id);
    col.setTitle(&NSString::from_str(""));
    col.setWidth(200.0);
    col.setMinWidth(40.0);
    col.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
    outline.addTableColumn(&col);
    unsafe {
        outline.setOutlineTableColumn(Some(&col));
    }

    let ds = ProtocolObject::from_ref(controller);
    let del = ProtocolObject::from_ref(controller);
    unsafe {
        outline.setDataSource(Some(ds));
        outline.setDelegate(Some(del));
    }
    let _ = tree_handle;
}

pub fn create_tree_ctrl_native(
    parent: NativeHandle,
    notify_id: u16,
    initial_rect: Rect,
) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = alloc_widget_handle();
    tree_init_storage(handle);

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

    let outline = NSOutlineView::initWithFrame(
        NSOutlineView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), ns_rect.size),
    );
    let controller = TreeOutlineController::new(mtm, handle, frame_handle, notify_id);
    configure_outline(&outline, handle, &controller, mtm);

    scroll.setDocumentView(Some(&outline));
    content.addSubview(&scroll);

    TREE_OUTLINES.with(|map| map.borrow_mut().insert(handle, outline.clone()));
    TREE_OUTLINE_CONTROLLERS.with(|map| map.borrow_mut().insert(handle, controller));

    store_ns_view(handle, scroll.into_super());
    record_parent(handle, parent);

    // Workaround macOS: `NSOutlineView` NON emette
    // `outlineViewSelectionDidChange:` quando l'utente riclicca su una
    // riga già selezionata. Per Explorer-equivalenza (Fase 6: click su
    // tree deve rinfrescare sempre la lista) intercettiamo i
    // `LeftMouseDown` globali, vediamo se cadono sull'outline e, se la
    // riga cliccata è la stessa già selezionata, forziamo manualmente
    // un re-dispatch di `TREE_NOTIFY_SELCHANGED`. L'osservatore vive
    // per la durata del widget (rimosso in `tree_cleanup`).
    let tree_handle_for_monitor = handle;
    let frame_handle_for_monitor = frame_handle;
    let block = RcBlock::new(
        move |event: *mut objc2_app_kit::NSEvent| -> *mut objc2_app_kit::NSEvent {
            if event.is_null() {
                return event;
            }
            let event_ref: &objc2_app_kit::NSEvent = unsafe { &*event };
            let location = event_ref.locationInWindow();
            let Some(outline) =
                TREE_OUTLINES.with(|m| m.borrow().get(&tree_handle_for_monitor).cloned())
            else {
                return event;
            };
            let point_in_outline = outline.convertPoint_fromView(location, None);
            let row = outline.rowAtPoint(point_in_outline);
            if row < 0 {
                return event;
            }
            let ctrl = TREE_OUTLINE_CONTROLLERS
                .with(|m| m.borrow().get(&tree_handle_for_monitor).cloned());
            if let Some(ctrl) = ctrl {
                let notify_id = ctrl.ivars().notify_id.get();
                // Dispatch SEMPRE quando il click cade su una riga del
                // tree. NSOutlineView non emette in modo affidabile
                // `outlineViewSelectionDidChange:` al primo click dopo
                // che la vista riceve il focus, quindi aspettare il
                // selettore nativo lasciava il primo click "morto"
                // (l'utente doveva cliccare due volte). Idempotente
                // lato app: la scan partita due volte viene scartata
                // dal check su `scan_generation` in `MSG_FILE_SCAN_DONE`.
                dispatch_notify(frame_handle_for_monitor, notify_id, TREE_NOTIFY_SELCHANGED);
            }
            event
        },
    );
    // Monitoriamo sia mouseDown che mouseUp. NSOutlineView cambia
    // `selectedRow` durante `mouseDown` ma la notifica delegate
    // arriva spesso solo al `mouseUp` su macOS Ventura+; per
    // coprire tutte le versioni di macOS dispatchiamo su entrambi.
    let mask = objc2_app_kit::NSEventMask::LeftMouseDown | objc2_app_kit::NSEventMask::LeftMouseUp;
    let monitor: Retained<AnyObject> = unsafe {
        objc2::msg_send![
            objc2::class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: mask,
            handler: &*block
        ]
    };
    TREE_OUTLINE_MONITORS.with(|m| m.borrow_mut().insert(handle, monitor));

    handle
}

pub fn tree_reload(tree_handle: NativeHandle) {
    let Some(outline) = outline_view(tree_handle) else {
        return;
    };
    let expanded = tree_collect_expanded_nodes(tree_handle);
    outline.reloadData();
    for node_id in expanded {
        let item = item_for_node(node_id);
        unsafe {
            outline.expandItem_expandChildren(Some(item.as_ref()), false);
        }
    }
    tree_sync_selection_to_outline(tree_handle, &outline);
}

fn tree_sync_selection_to_outline(tree_handle: NativeHandle, outline: &NSOutlineView) {
    let Some(sel) = tree_read_selected(tree_handle) else {
        unsafe {
            outline.deselectAll(None);
        }
        return;
    };
    let item = item_for_node(sel);
    let row = unsafe { outline.rowForItem(Some(item.as_ref())) };
    if row >= 0 {
        let idx = objc2_foundation::NSIndexSet::indexSetWithIndex(row as NSUInteger);
        outline.selectRowIndexes_byExtendingSelection(&idx, false);
        outline.scrollRowToVisible(row);
    }
}

pub fn tree_expand_item(tree_handle: NativeHandle, node_id: usize) {
    tree_set_expanded(tree_handle, node_id, true);
    if let Some(outline) = outline_view(tree_handle) {
        let item = item_for_node(node_id);
        unsafe {
            outline.expandItem_expandChildren(Some(item.as_ref()), false);
        }
    }
}

pub fn tree_collapse_item(tree_handle: NativeHandle, node_id: usize) {
    tree_set_expanded(tree_handle, node_id, false);
    if let Some(outline) = outline_view(tree_handle) {
        let item = item_for_node(node_id);
        unsafe {
            outline.collapseItem_collapseChildren(Some(item.as_ref()), false);
        }
    }
}

/// Imposta la dimensione del font (in punti) sulla NSOutlineView.
/// Usato dalle preferenze utente (`Strumenti → Opzioni…`).
pub fn tree_set_font_size_native(tree_handle: NativeHandle, point_size: f64) {
    if let Some(outline) = outline_view(tree_handle) {
        let pt = point_size.clamp(8.0, 28.0);
        let font = objc2_app_kit::NSFont::systemFontOfSize(pt);
        outline.setFont(Some(&font));
        // Allinea l'altezza riga al nuovo font per evitare troncamenti.
        let line_h = (pt * 1.6).max(18.0);
        outline.setRowHeight(line_h);
        outline.reloadData();
    }
}

pub fn tree_select_item_native(tree_handle: NativeHandle, node_id: usize) {
    tree_write_selected(tree_handle, Some(node_id));
    if let Some(outline) = outline_view(tree_handle) {
        tree_sync_selection_to_outline(tree_handle, &outline);
    }
}
