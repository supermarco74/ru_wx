//! macOS embedded browser via `WKWebView`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSString, NSURLRequest, NSURL,
};
use objc2_web_kit::{WKNavigation, WKNavigationDelegate, WKWebView, WKWebViewConfiguration};

use crate::core::geometry::Rect;

use super::{child_view_for_handle, create_panel, NativeHandle};

/// Navigation callback kind (mirrors [`crate::adv::web_view_event::WebViewEventKind`]).
pub type WebViewNavKind = u8;
pub const WEBVIEW_NAV_REQUESTED: WebViewNavKind = 0;
pub const WEBVIEW_NAV_COMPLETE: WebViewNavKind = 1;
pub const WEBVIEW_NAV_ERROR: WebViewNavKind = 2;
pub const WEBVIEW_TITLE_CHANGED: WebViewNavKind = 3;

thread_local! {
    static WEBVIEWS: RefCell<HashMap<NativeHandle, Retained<WKWebView>>> =
        RefCell::new(HashMap::new());
    static WEBVIEW_HANDLERS: RefCell<
        HashMap<NativeHandle, Box<dyn FnMut(WebViewNavKind, String)>>,
    > = RefCell::new(HashMap::new());
    static WEBVIEW_DELEGATES: RefCell<HashMap<NativeHandle, Retained<WebNavDelegate>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct WebNavDelegateIvars {
    view_handle: Cell<NativeHandle>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = WebNavDelegateIvars]
    struct WebNavDelegate;

    unsafe impl NSObjectProtocol for WebNavDelegate {}

    unsafe impl WKNavigationDelegate for WebNavDelegate {
        #[unsafe(method(webView:didStartProvisionalNavigation:))]
        fn did_start(&self, web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            let handle = self.ivars().view_handle.get();
            fire_webview_event(handle, WEBVIEW_NAV_REQUESTED, url_string(web_view));
        }

        #[unsafe(method(webView:didFinishNavigation:))]
        fn did_finish(&self, web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            let handle = self.ivars().view_handle.get();
            fire_webview_event(handle, WEBVIEW_NAV_COMPLETE, url_string(web_view));
            if let Some(title) = title_string(web_view).filter(|t| !t.is_empty()) {
                fire_webview_event(handle, WEBVIEW_TITLE_CHANGED, title);
            }
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_provisional(
            &self,
            web_view: &WKWebView,
            _navigation: Option<&WKNavigation>,
            _error: &objc2_foundation::NSError,
        ) {
            let handle = self.ivars().view_handle.get();
            fire_webview_event(handle, WEBVIEW_NAV_ERROR, url_string(web_view));
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn did_fail(
            &self,
            web_view: &WKWebView,
            _navigation: Option<&WKNavigation>,
            _error: &objc2_foundation::NSError,
        ) {
            let handle = self.ivars().view_handle.get();
            fire_webview_event(handle, WEBVIEW_NAV_ERROR, url_string(web_view));
        }
    }
);

impl WebNavDelegate {
    fn new(mtm: MainThreadMarker, view_handle: NativeHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(WebNavDelegateIvars {
            view_handle: Cell::new(view_handle),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn url_string(web_view: &WKWebView) -> String {
    unsafe {
        web_view
            .URL()
            .and_then(|u| u.absoluteString())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }
}

fn title_string(web_view: &WKWebView) -> Option<String> {
    unsafe { web_view.title().map(|s| s.to_string()) }
}

fn fire_webview_event(handle: NativeHandle, kind: WebViewNavKind, url: String) {
    WEBVIEW_HANDLERS.with(|map| {
        if let Some(handler) = map.borrow_mut().get_mut(&handle) {
            handler(kind, url);
        }
    });
}

fn ns_rect_to_cg(rect: Rect) -> CGRect {
    CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(rect.width as f64, rect.height as f64),
    )
}

/// Register (or replace) the navigation event handler for a web view handle.
pub fn webview_set_event_handler(
    handle: NativeHandle,
    handler: Option<Box<dyn FnMut(WebViewNavKind, String)>>,
) {
    WEBVIEW_HANDLERS.with(|map| match handler {
        Some(h) => {
            map.borrow_mut().insert(handle, h);
        }
        None => {
            map.borrow_mut().remove(&handle);
        }
    });
}

/// Create a `WKWebView` child of `parent`.
pub fn create_web_view(parent: NativeHandle, initial_rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = create_panel(parent, initial_rect);
    let config = unsafe { WKWebViewConfiguration::new(mtm) };
    let this = mtm.alloc::<WKWebView>();
    let webview = unsafe {
        WKWebView::initWithFrame_configuration(this, ns_rect_to_cg(initial_rect), &config)
    };

    let delegate = WebNavDelegate::new(mtm, handle);
    WEBVIEW_DELEGATES.with(|d| d.borrow_mut().insert(handle, delegate.clone()));
    unsafe {
        webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    }

    if let Some(container) = child_view_for_handle(handle) {
        webview.setFrame(container.bounds());
        container.addSubview(&webview);
    }

    WEBVIEWS.with(|map| map.borrow_mut().insert(handle, webview));
    handle
}

pub fn webview_load_url(handle: NativeHandle, url: &str) {
    WEBVIEWS.with(|map| {
        let Some(webview) = map.borrow().get(&handle).cloned() else {
            return;
        };
        fire_webview_event(handle, WEBVIEW_NAV_REQUESTED, url.to_string());
        let Some(ns_url) = NSURL::URLWithString(&NSString::from_str(url)) else {
            fire_webview_event(handle, WEBVIEW_NAV_ERROR, url.to_string());
            return;
        };
        let request = NSURLRequest::requestWithURL(&ns_url);
        unsafe {
            let _ = webview.loadRequest(&request);
        }
    });
}

pub fn webview_load_html(handle: NativeHandle, html: &str) {
    WEBVIEWS.with(|map| {
        let Some(webview) = map.borrow().get(&handle).cloned() else {
            return;
        };
        fire_webview_event(handle, WEBVIEW_NAV_REQUESTED, String::new());
        unsafe {
            let _ = webview.loadHTMLString_baseURL(&NSString::from_str(html), None);
        }
    });
}

pub fn webview_reload(handle: NativeHandle) {
    WEBVIEWS.with(|map| {
        if let Some(webview) = map.borrow().get(&handle).cloned() {
            unsafe {
                let _ = webview.reload();
            }
        }
    });
}

pub fn webview_cleanup(handle: NativeHandle) {
    WEBVIEW_DELEGATES.with(|d| d.borrow_mut().remove(&handle));
    WEBVIEWS.with(|m| m.borrow_mut().remove(&handle));
}
