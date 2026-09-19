//! The UIKit side of the shell: a Rust-defined `UIApplicationDelegate` that
//! creates and owns the window (the classic, scene-less lifecycle — ADR 002,
//! debt 002-1), and a Rust-defined root `UIViewController` whose layout and
//! touch callbacks *request* frames and whose display-link tick *grants*
//! them (ADR 005). This module is the only place allowed to speak
//! Objective-C; every `unsafe` here is ObjC interop.

use core::cell::{Cell, OnceCell, RefCell};
use core::ptr;

use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use objc2_core_foundation::{CFData, CFRetained, CFType};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo,
};
use objc2_foundation::{
    NSDictionary, NSObject, NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes, NSSet, NSString,
};
use objc2_quartz_core::CADisplayLink;
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey, UIColor, UIEvent,
    UIScreen, UITouch, UIView, UIViewController, UIWindow,
};

use crate::{App, Frame, SafeAreaInsets};

/// A viewport-metrics snapshot. UIKit fires layout passes liberally;
/// memoizing the snapshot of the last *requested* frame keeps identical
/// passes from re-requesting identical frames (ADR 004, re-aimed by 005).
#[derive(Clone, Copy, PartialEq)]
struct Metrics {
    width: u32,
    height: u32,
    scale: f64,
    safe_area: SafeAreaInsets,
}

thread_local! {
    /// Parked here by `run()`; the shell calls into it for the app's whole
    /// life. A thread-local (not a static) works because both ends are the
    /// main thread: `run()` asserts it, and UIKit guarantees it for
    /// delegate, view-controller and display-link callbacks.
    static APP: RefCell<Option<Box<dyn App>>> = const { RefCell::new(None) };
    /// Metrics of the most recently requested frame.
    static LAST_METRICS: Cell<Option<Metrics>> = const { Cell::new(None) };
    /// The vsync channel (ADR 005): a display link that stays paused
    /// whenever no frame is pending. Owned here rather than by the view
    /// controller because `request_frame()` — a free function — must reach
    /// it to unpause.
    static DISPLAY_LINK: OnceCell<Retained<CADisplayLink>> = const { OnceCell::new() };
}

/// The request side of the loop — Flutter's `scheduleFrame`: unpause the
/// link and let the next tick render. Setting `paused` is idempotent, so
/// requests coalesce until granted, exactly the contract `lib.rs`
/// documents. Must never call into `APP` (the caller may be mid-`draw`,
/// holding the borrow).
pub(crate) fn request_frame() {
    MainThreadMarker::new().expect("request_frame() must be called on the main thread");
    DISPLAY_LINK.with(|link| {
        // No link yet means UIKit hasn't finished launching; the first
        // layout pass always requests a frame, so nothing is lost.
        if let Some(link) = link.get() {
            link.setPaused(false);
        }
    });
}

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
        // screen-sized window, install the root view controller, make it key.
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

            // No drawing here (that was 003): the first layout pass after
            // `makeKeyAndVisible` reaches `viewDidLayoutSubviews`, which
            // requests the first frame — the same path rotation takes.
            let root = ViewController::new(mtm);
            let root_view = root
                .view()
                .expect("view controller loads its view on first access");
            root_view.setBackgroundColor(Some(&UIColor::whiteColor()));
            window.setRootViewController(Some(&root));
            window.makeKeyAndVisible();

            if self.ivars().window.set(window).is_err() {
                unreachable!("application:didFinishLaunchingWithOptions: runs at most once");
            }
            true
        }
    }
);

define_class!(
    // SAFETY: `UIViewController` has no subclassing requirements beyond
    // main-thread use (encoded by `MainThreadOnly`), and `ViewController`
    // does not implement `Drop`.
    #[unsafe(super(UIViewController))]
    #[thread_kind = MainThreadOnly]
    #[name = "MelViewController"]
    struct ViewController;

    unsafe impl NSObjectProtocol for ViewController {}

    impl ViewController {
        // One-time setup, guaranteed to run before the first layout pass:
        // create the vsync channel, paused (ADR 005). Common run-loop modes
        // so ticks keep arriving while UIKit tracks a touch — Flutter's
        // VsyncWaiterIOS makes the same two choices.
        #[unsafe(method(viewDidLoad))]
        fn view_did_load(&self) {
            // SAFETY: plain forward to `UIViewController`'s implementation
            // (documented to do nothing, called for protocol hygiene).
            let _: () = unsafe { msg_send![super(self), viewDidLoad] };
            let target: &AnyObject = self;
            // SAFETY: the target outlives the link (the link retains it),
            // and `onDisplayLink:` is defined on this very class below with
            // the display-link selector signature the docs require.
            let link = unsafe {
                CADisplayLink::displayLinkWithTarget_selector(target, sel!(onDisplayLink:))
            };
            link.setPaused(true);
            // SAFETY: the main run loop is valid for the app's whole life,
            // and `NSRunLoopCommonModes` is a Foundation-defined mode
            // constant (an immutable extern static).
            unsafe {
                link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
            }
            DISPLAY_LINK.with(|slot| {
                if slot.set(link).is_err() {
                    unreachable!("viewDidLoad runs at most once for the lone root view controller");
                }
            });
        }

        // Frame requester #1 (mirroring FlutterViewController): UIKit calls
        // this whenever the view's bounds were (re)laid out — first display,
        // rotation, safe-area changes — the one place all viewport metrics
        // are current. Since 005 it renders nothing: it requests, and the
        // display-link tick draws.
        #[unsafe(method(viewDidLayoutSubviews))]
        fn view_did_layout_subviews(&self) {
            // SAFETY: plain forward to `UIViewController`'s implementation
            // (documented to do nothing, called for protocol hygiene).
            let _: () = unsafe { msg_send![super(self), viewDidLayoutSubviews] };
            let view = self
                .view()
                .expect("viewDidLayoutSubviews implies the view is loaded");
            let Some(metrics) = current_metrics(&view) else {
                // Degenerate mid-transition pass; a real one follows.
                return;
            };
            if LAST_METRICS.get() == Some(metrics) {
                return;
            }
            LAST_METRICS.set(Some(metrics));
            request_frame();
        }

        // Frame requester #2: the user poked the screen. The shell decides
        // nothing here; the app hears about it and may call
        // `request_frame()`. Positions, phases and pointer IDs are 007.
        #[unsafe(method(touchesBegan:withEvent:))]
        fn touches_began(&self, touches: &NSSet<UITouch>, event: Option<&UIEvent>) {
            // SAFETY: plain forward to `UIResponder`'s implementation
            // (responder-chain hygiene for an event we only observe).
            let _: () = unsafe { msg_send![super(self), touchesBegan: touches, withEvent: event] };
            APP.with_borrow_mut(|app| {
                let app = app
                    .as_mut()
                    .expect("run() parks the app before UIKit launches");
                app.on_touch();
            });
        }

        // The grant side (ADR 005): one tick per requested frame. Pausing
        // *before* the app draws lets `draw` itself request the next frame
        // (continuous animation) instead of having the request swallowed.
        #[unsafe(method(onDisplayLink:))]
        fn on_display_link(&self, link: &CADisplayLink) {
            link.setPaused(true);
            let view = self
                .view()
                .expect("the display link only exists after viewDidLoad loaded the view");
            render(&view);
        }
    }
);

impl ViewController {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        // SAFETY: plain forward to `UIViewController`'s `init`; the subclass
        // adds no ivars that would need initializing first.
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

/// Reads the viewport metrics off the live view. `None` when there is
/// nothing to render to: detached from any window, or zero-sized (possible
/// mid-transition).
fn current_metrics(view: &UIView) -> Option<Metrics> {
    let window = view.window()?;
    let bounds = view.bounds();
    // The screen's scale (2×/3×): physical pixels per logical point. Not
    // `view.contentScaleFactor` — that mirrors the layer's `contentsScale`,
    // which UIKit leaves at 1.0 for layers it never draws into (ADR 004).
    let scale = window.screen().scale();
    let insets = view.safeAreaInsets();

    // The buffer lives in physical pixels — logical points × scale
    // (Flutter's devicePixelRatio). Rounding: logical sizes on real devices
    // multiply to whole pixels; anything else is a UIKit rounding artifact.
    let width = (bounds.size.width * scale).round() as u32;
    let height = (bounds.size.height * scale).round() as u32;
    if width == 0 || height == 0 {
        return None;
    }
    Some(Metrics {
        width,
        height,
        scale,
        safe_area: SafeAreaInsets {
            left: insets.left,
            top: insets.top,
            right: insets.right,
            bottom: insets.bottom,
        },
    })
}

/// The single render path (ADR 005) — runs only from the display-link tick:
/// reads the metrics *fresh* (never a request-time snapshot; the view may
/// have changed between request and grant), has the app draw a frame at
/// them, and installs it as the view's layer contents.
fn render(view: &UIView) {
    let Some(metrics) = current_metrics(view) else {
        // Nothing to present to: the granted request is dropped. Clearing
        // the memo makes the next real layout pass re-request the frame
        // instead of memoizing it away.
        LAST_METRICS.set(None);
        return;
    };
    let Metrics {
        width,
        height,
        scale,
        safe_area,
    } = metrics;

    let mut frame = Frame {
        width,
        height,
        scale,
        safe_area,
        pixels: vec![0; width as usize * height as usize * 4],
    };
    APP.with_borrow_mut(|app| {
        let app = app
            .as_mut()
            .expect("run() parks the app before UIKit launches");
        app.draw(&mut frame);
    });

    let image = cg_image_from(&frame);
    let layer = view.layer();
    // The image is physical-pixel-sized; telling the layer its scale makes
    // Core Animation composite it 1:1 instead of resampling (ADR 004).
    layer.setContentsScale(scale);
    let contents: &CFType = &image;
    // SAFETY: `contents` accepts a CGImage (CALayer docs); CF types are
    // toll-free bridgeable to ObjC objects, which is exactly what
    // `CFType: AsRef<AnyObject>` encodes.
    unsafe { layer.setContents(Some(contents.as_ref())) };
}

/// Wraps a drawn frame into a `CGImage` for Core Animation. Copies the
/// buffer into a `CFData` — paid once per *requested* frame since 005; the
/// GPU decision (008) owns the presentation-cost question.
fn cg_image_from(frame: &Frame) -> CFRetained<CGImage> {
    let data = CFData::from_bytes(frame.pixels());
    let provider =
        CGDataProvider::with_cf_data(Some(&data)).expect("a CFData-backed provider cannot fail");
    // Untagged device RGB; color management is deferred (feature 013).
    let space = CGColorSpace::new_device_rgb().expect("device RGB color space always exists");
    // Little-endian 32-bit with premultiplied alpha "first" = B,G,R,A in
    // memory — the exact layout `mel_painting::fill` writes (its unit tests
    // pin the byte order down).
    let bitmap_info = CGBitmapInfo(
        CGImageAlphaInfo::PremultipliedFirst.0 | CGImageByteOrderInfo::Order32Little.0,
    );
    // SAFETY: the geometry arguments describe `frame.pixels()` exactly —
    // width × height 32-bit pixels, tightly packed rows of `width * 4` bytes,
    // and the provider keeps its copy of the bytes alive for the image.
    unsafe {
        CGImage::new(
            frame.physical_width() as usize,
            frame.physical_height() as usize,
            8,  // bits per component
            32, // bits per pixel
            frame.physical_width() as usize * 4,
            Some(&space),
            bitmap_info,
            Some(&provider),
            ptr::null(), // no decode array
            false,       // no interpolation; pixels are presented 1:1
            CGColorRenderingIntent::RenderingIntentDefault,
        )
        .expect("a valid BGRA8888 image description")
    }
}

pub(crate) fn run(app: Box<dyn App>) -> ! {
    let mtm =
        MainThreadMarker::new().expect("mel_platform::run() must be called on the main thread");
    APP.with_borrow_mut(|slot| *slot = Some(app));
    // Hand the thread to UIKit's event loop; it creates one `MelAppDelegate`
    // and calls `did_finish_launching` above.
    UIApplication::main(None, Some(&NSString::from_class(AppDelegate::class())), mtm)
}
