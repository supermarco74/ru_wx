//! macOS printing — [`NSPrintPanel`], [`NSPageLayout`], [`NSPrintOperation`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSColor, NSPageLayout, NSPageLayoutResult, NSPrintAllPages, NSPrintCopies, NSPrintFirstPage,
    NSPrintInfo, NSPrintLastPage, NSPrintMustCollate, NSPrintOperation, NSPrintPanel,
    NSPrintPanelOptions, NSPrintPanelResult, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSNumber, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize, NSString,
};

use crate::core::geometry::Size;
use crate::printing::Printout;

thread_local! {
    static PRINT_PREVIEW_VIEWS: RefCell<HashMap<usize, Retained<PrintPreviewView>>> =
        RefCell::new(HashMap::new());
}

fn tenths_mm_to_points(v: i32) -> f64 {
    (v as f64 / 10.0) / 25.4 * 72.0
}

fn points_to_tenths_mm(v: f64) -> i32 {
    (v / 72.0 * 25.4 * 10.0).round() as i32
}

fn mm_to_points(v: u32) -> f64 {
    v as f64 / 25.4 * 72.0
}

fn points_to_mm(v: f64) -> u32 {
    (v / 72.0 * 25.4).round() as u32
}

fn apply_page_setup_to_print_info(print_info: &NSPrintInfo, paper_size: Size, margin_mm: u32) {
    print_info.setPaperSize(NSSize::new(
        tenths_mm_to_points(paper_size.width),
        tenths_mm_to_points(paper_size.height),
    ));
    let margin = mm_to_points(margin_mm);
    print_info.setLeftMargin(margin);
    print_info.setRightMargin(margin);
    print_info.setTopMargin(margin);
    print_info.setBottomMargin(margin);
}

fn read_page_setup_from_print_info(print_info: &NSPrintInfo) -> (Size, u32) {
    let paper = print_info.paperSize();
    let paper_size = Size::new(
        points_to_tenths_mm(paper.width),
        points_to_tenths_mm(paper.height),
    );
    let margin_mm = points_to_mm(print_info.leftMargin());
    (paper_size, margin_mm)
}

fn set_print_dialog_values(
    print_info: &NSPrintInfo,
    copies: u32,
    from_page: u32,
    to_page: u32,
    collate: bool,
) {
    unsafe {
        let dict = print_info.dictionary();
        let copies_key = ProtocolObject::from_ref(NSPrintCopies);
        let first_key = ProtocolObject::from_ref(NSPrintFirstPage);
        let last_key = ProtocolObject::from_ref(NSPrintLastPage);
        let collate_key = ProtocolObject::from_ref(NSPrintMustCollate);
        let all_pages_key = ProtocolObject::from_ref(NSPrintAllPages);
        dict.setObject_forKey(&NSNumber::numberWithUnsignedInt(copies), copies_key);
        dict.setObject_forKey(&NSNumber::numberWithUnsignedInt(from_page), first_key);
        dict.setObject_forKey(&NSNumber::numberWithUnsignedInt(to_page), last_key);
        dict.setObject_forKey(&NSNumber::numberWithBool(collate), collate_key);
        dict.setObject_forKey(&NSNumber::numberWithBool(false), all_pages_key);
    }
}

fn read_print_dialog_values(print_info: &NSPrintInfo) -> (u32, u32, u32, bool) {
    unsafe {
        let dict = print_info.dictionary();
        let copies = dict
            .objectForKey(NSPrintCopies)
            .and_then(|o| o.downcast::<NSNumber>().ok())
            .map(|n| n.unsignedIntValue())
            .unwrap_or(1);
        let from_page = dict
            .objectForKey(NSPrintFirstPage)
            .and_then(|o| o.downcast::<NSNumber>().ok())
            .map(|n| n.unsignedIntValue())
            .unwrap_or(1);
        let to_page = dict
            .objectForKey(NSPrintLastPage)
            .and_then(|o| o.downcast::<NSNumber>().ok())
            .map(|n| n.unsignedIntValue())
            .unwrap_or(1);
        let collate = dict
            .objectForKey(NSPrintMustCollate)
            .and_then(|o| o.downcast::<NSNumber>().ok())
            .map(|n| n.boolValue())
            .unwrap_or(false);
        (copies, from_page, to_page, collate)
    }
}

#[derive(Default)]
struct PrintPreviewViewIvars {
    page_count: Cell<u32>,
    title: RefCell<String>,
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = PrintPreviewViewIvars]
    struct PrintPreviewView;

    unsafe impl NSObjectProtocol for PrintPreviewView {}

    impl PrintPreviewView {
        #[unsafe(method(knowsPageRange:))]
        fn knows_page_range(&self, range: *mut NSRange) -> Bool {
            if range.is_null() {
                return Bool::NO;
            }
            let count = self.ivars().page_count.get().max(1) as usize;
            unsafe {
                (*range).location = 1;
                (*range).length = count;
            }
            Bool::YES
        }

        #[unsafe(method(rectForPage:))]
        fn rect_for_page(&self, _page: isize) -> NSRect {
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(612.0, 792.0))
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
            let page_count = self.ivars().page_count.get().max(1);
            let title = self.ivars().title.borrow().clone();
            let current_page = NSPrintOperation::currentOperation(mtm)
                .map(|op| op.currentPage())
                .unwrap_or(1)
                .max(1) as u32;
            let label = format!("{title}\nPage {current_page} of {page_count}");
            let bounds = self.bounds();
            NSColor::blackColor().set();
            let label_ns = NSString::from_str(&label);
            // Use drawAtPoint:withAttributes: which is available on NSString
            let point = NSPoint::new(bounds.origin.x + 36.0, bounds.origin.y + bounds.size.height - 72.0);
            unsafe {
                let _: () = msg_send![&label_ns, drawAtPoint: point, withAttributes: std::ptr::null::<objc2::runtime::AnyObject>()];
            }
        }
    }
);

impl PrintPreviewView {
    fn new(mtm: MainThreadMarker, printout: &Printout) -> Retained<Self> {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(612.0, 792.0));
        let this = Self::alloc(mtm).set_ivars(PrintPreviewViewIvars {
            page_count: Cell::new(printout.page_count.max(1)),
            title: RefCell::new(printout.title.clone()),
        });
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }
}

fn print_panel_options() -> NSPrintPanelOptions {
    NSPrintPanelOptions::ShowsCopies
        | NSPrintPanelOptions::ShowsPageRange
        | NSPrintPanelOptions::ShowsPaperSize
        | NSPrintPanelOptions::ShowsOrientation
        | NSPrintPanelOptions::ShowsPreview
}

/// Show the system print panel. Returns `true` when the user clicks Print.
pub fn mac_show_print_dialog(
    copies: &mut u32,
    from_page: &mut u32,
    to_page: &mut u32,
    collate: &mut bool,
) -> bool {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let panel = NSPrintPanel::printPanel(mtm);
    panel.setOptions(print_panel_options());
    let print_info = NSPrintInfo::sharedPrintInfo();
    set_print_dialog_values(&print_info, *copies, *from_page, *to_page, *collate);
    let result = panel.runModalWithPrintInfo(&print_info);
    if result != NSPrintPanelResult::Printed.0 {
        return false;
    }
    let (c, from, to, col) = read_print_dialog_values(&print_info);
    *copies = c.max(1);
    *from_page = from.max(1);
    *to_page = to.max(*from_page);
    *collate = col;
    true
}

/// Show the system page-setup panel. Returns `true` when the user clicks OK.
pub fn mac_show_page_setup(paper_size: &mut Size, margin_mm: &mut u32) -> bool {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let page_layout = NSPageLayout::pageLayout(mtm);
    let print_info = NSPrintInfo::sharedPrintInfo();
    apply_page_setup_to_print_info(&print_info, *paper_size, *margin_mm);
    let result = page_layout.runModalWithPrintInfo(&print_info);
    if result != NSPageLayoutResult::Changed.0 {
        return false;
    }
    let (paper, margin) = read_page_setup_from_print_info(&print_info);
    *paper_size = paper;
    *margin_mm = margin;
    true
}

/// Show the native print panel with an embedded preview for `printout`.
pub fn mac_show_print_preview(printout: &Printout) -> bool {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let view = PrintPreviewView::new(mtm, printout);
    let view_ptr = view.as_ref() as *const PrintPreviewView as usize;
    PRINT_PREVIEW_VIEWS.with(|views| views.borrow_mut().insert(view_ptr, view.clone()));

    let print_info = NSPrintInfo::sharedPrintInfo();
    if printout.page_count > 0 {
        set_print_dialog_values(&print_info, 1, 1, printout.page_count, false);
    }

    let op = NSPrintOperation::printOperationWithView_printInfo(&view, &print_info);
    op.setShowsPrintPanel(true);
    op.setShowsProgressPanel(true);
    op.setJobTitle(Some(&NSString::from_str(&printout.title)));
    let panel = op.printPanel();
    panel.setOptions(print_panel_options());
    let result = op.runOperation();
    PRINT_PREVIEW_VIEWS.with(|views| views.borrow_mut().clear());
    result
}
