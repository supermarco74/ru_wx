//! macOS user notifications via [`UNUserNotificationCenter`].
//!
//! User notifications require a proper `.app` bundle with `Info.plist`.
//! Running a bare Mach-O from `cargo run` or `examples_macos/` has no
//! bundle identifier and [`UNUserNotificationCenter::currentNotificationCenter`]
//! raises `NSInternalInconsistencyException`. All entry points therefore
//! check [`bundle_supports_user_notifications`] first and degrade gracefully.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Once};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_foundation::{MainThreadMarker, NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};

thread_local! {
    static NOTIFICATION_DELEGATE: RefCell<Option<Retained<NotificationDelegate>>> =
        RefCell::new(None);
    static BALLOON_CLICK: RefCell<Option<Box<dyn FnMut()>>> = RefCell::new(None);
}

#[derive(Default)]
struct NotificationDelegateIvars;

static INSTALL_ONCE: Once = Once::new();
static AUTH_ONCE: Once = Once::new();
static AUTHORIZED: AtomicBool = AtomicBool::new(false);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static BUNDLE_CHECKED: AtomicBool = AtomicBool::new(false);
static BUNDLE_OK: AtomicBool = AtomicBool::new(false);

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = NotificationDelegateIvars]
    #[name = "RuWxNotificationDelegate"]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion_handler: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion_handler.call((UNNotificationPresentationOptions::Banner
                | UNNotificationPresentationOptions::Sound,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive_response(
            &self,
            _center: &UNUserNotificationCenter,
            _response: &UNNotificationResponse,
            completion_handler: &block2::DynBlock<dyn Fn()>,
        ) {
            BALLOON_CLICK.with(|slot| {
                if let Some(handler) = slot.borrow_mut().as_mut() {
                    handler();
                }
            });
            completion_handler.call(());
        }
    }
);

impl NotificationDelegate {
    fn install(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(NotificationDelegateIvars);
        unsafe { msg_send![super(this), init] }
    }
}

/// `true` when the process is backed by a real app bundle (`.app` + `Info.plist`).
pub fn bundle_supports_user_notifications() -> bool {
    if BUNDLE_CHECKED.load(Ordering::Acquire) {
        return BUNDLE_OK.load(Ordering::Acquire);
    }
    let ok = {
        let bundle = NSBundle::mainBundle();
        if bundle.bundleIdentifier().is_some() {
            true
        } else {
            bundle
                .bundleURL()
                .path()
                .is_some_and(|path| path.to_string().ends_with(".app"))
        }
    };
    BUNDLE_OK.store(ok, Ordering::Release);
    BUNDLE_CHECKED.store(true, Ordering::Release);
    ok
}

fn install_notification_center() -> bool {
    if !bundle_supports_user_notifications() {
        return false;
    }
    INSTALL_ONCE.call_once(|| {
        let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
        let delegate = NotificationDelegate::install(mtm);
        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        NOTIFICATION_DELEGATE.with(|slot| *slot.borrow_mut() = Some(delegate));
    });
    true
}

fn request_authorization_if_needed() {
    if !bundle_supports_user_notifications() {
        return;
    }
    AUTH_ONCE.call_once(|| {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let (tx, rx) = mpsc::sync_channel(1);
        let block = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
            AUTHORIZED.store(granted.as_bool(), Ordering::SeqCst);
            let _ = tx.send(());
        });
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &block,
        );
        let _ = rx.recv_timeout(std::time::Duration::from_secs(10));
    });
}

/// Register a callback for the user activating a delivered notification.
pub fn notification_set_click_handler(handler: Option<Box<dyn FnMut()>>) {
    BALLOON_CLICK.with(|slot| *slot.borrow_mut() = handler);
    let _ = install_notification_center();
}

/// Post a short-lived banner notification. Returns `true` when queued successfully.
pub fn post_user_notification(title: &str, body: &str) -> bool {
    if !install_notification_center() {
        return false;
    }
    request_authorization_if_needed();

    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(body));

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let identifier = NSString::from_str(&format!("ru_wx.notification.{id}"));
    let request =
        UNNotificationRequest::requestWithIdentifier_content_trigger(&identifier, &content, None);

    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (tx, rx) = mpsc::sync_channel(1);
    let block = RcBlock::new(move |error: *mut NSError| {
        let _ = tx.send(error.is_null());
    });
    center.addNotificationRequest_withCompletionHandler(&request, Some(&block));
    rx.recv_timeout(std::time::Duration::from_secs(2))
        .unwrap_or(false)
}
