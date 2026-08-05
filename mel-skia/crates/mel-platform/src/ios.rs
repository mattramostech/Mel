//! The UIKit side of the shell: a Rust-defined `UIApplicationDelegate` that
//! creates and owns the window (the classic, scene-less lifecycle — ADR 002,
//! debt 002-1). This module is the only place allowed to speak Objective-C;
//! every `unsafe` here is ObjC interop.

use core::cell::OnceCell;

use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSDictionary, NSObject, NSObjectProtocol, NSString};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey, UIColor, UIScreen,
    UIViewController, UIWindow,
};

/// Rust state attached to the delegate object. UIKit does not retain the key
/// window (unlike view controllers, which retain their views), so the
/// delegate must keep it alive or the app shows a black screen.
#[derive(Default)]
struct Ivars {
    window: OnceCell<Retained<UIWindow>>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements, and `AppDelegate`
    // does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MelAppDelegate"]
    #[ivars = Ivars]
    struct AppDelegate;

    impl AppDelegate {
        // UIKit instantiates the delegate class named in `run()` below.
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(Ivars::default());
            // SAFETY: plain forward to `NSObject`'s argument-less `init`.
            unsafe { msg_send![super(this), init] }
        }
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl UIApplicationDelegate for AppDelegate {
        // The launch hook. Same job as Flutter's FlutterAppDelegate: create a
        // screen-sized window, install a root view controller, make it key.
        #[unsafe(method(application:didFinishLaunchingWithOptions:))]
        fn did_finish_launching(
            &self,
            _application: &UIApplication,
            _launch_options: Option<&NSDictionary<UIApplicationLaunchOptionsKey, AnyObject>>,
        ) -> bool {
            // Delegate callbacks are guaranteed to arrive on the main thread.
            let mtm = MainThreadMarker::from(self);

            // `mainScreen` and `initWithFrame` carry scenes-era deprecations
            // that don't apply to a scene-less app: there is exactly one
            // screen and no window scene to attach to. Revisited if 008
            // adopts the scene lifecycle (debt 002-1).
            #[expect(deprecated, reason = "scene-less lifecycle (ADR 002, debt 002-1)")]
            let window = {
                let frame = UIScreen::mainScreen(mtm).bounds();
                UIWindow::initWithFrame(UIWindow::alloc(mtm), frame)
            };

            let root = UIViewController::new(mtm);
            root.view()
                .expect("view controller loads its view on first access")
                .setBackgroundColor(Some(&UIColor::whiteColor()));
            window.setRootViewController(Some(&root));
            window.makeKeyAndVisible();

            if self.ivars().window.set(window).is_err() {
                unreachable!("application:didFinishLaunchingWithOptions: runs at most once");
            }
            true
        }
    }
);

pub(crate) fn run() -> ! {
    let mtm =
        MainThreadMarker::new().expect("mel_platform::run() must be called on the main thread");
    // Hand the thread to UIKit's event loop; it creates one `MelAppDelegate`
    // and calls `did_finish_launching` above.
    UIApplication::main(None, Some(&NSString::from_class(AppDelegate::class())), mtm)
}
