//! The UIKit side of the shell: a Rust-defined `UIApplicationDelegate` that
//! creates and owns the window (the classic, scene-less lifecycle — ADR 002,
//! debt 002-1), and a Rust-defined root `UIViewController` whose layout
//! callback is the render trigger (ADR 004). This module is the only place
//! allowed to speak Objective-C; every `unsafe` here is ObjC interop.

use core::cell::{Cell, OnceCell, RefCell};
use core::ptr;

use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_core_foundation::{CFData, CFRetained, CFType};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo,
};
use objc2_foundation::{NSDictionary, NSObject, NSObjectProtocol, NSString};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey, UIColor, UIScreen, UIView,
    UIViewController, UIWindow,
};

use crate::{Frame, SafeAreaInsets};

type DrawFn = Box<dyn FnMut(&mut Frame)>;

/// The viewport metrics one frame was rendered at. UIKit fires layout
/// passes liberally; memoizing the last snapshot keeps identical passes
/// from re-rastering the same frame (ADR 004).
#[derive(Clone, Copy, PartialEq)]
struct Metrics {
    width: u32,
    height: u32,
    scale: f64,
    safe_area: SafeAreaInsets,
}

thread_local! {
    /// Parked here by `run()`; invoked on every metrics change for the
    /// app's whole life. A thread-local (not a static) works because both
    /// ends are the main thread: `run()` asserts it, and UIKit guarantees
    /// it for delegate and view-controller callbacks.
    static DRAW: RefCell<Option<DrawFn>> = const { RefCell::new(None) };
    /// Metrics of the most recently presented frame.
    static LAST_METRICS: Cell<Option<Metrics>> = const { Cell::new(None) };
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
            // renders the first frame — the same path rotation takes.
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
        // The render trigger (ADR 004, mirroring FlutterViewController):
        // UIKit calls this whenever the view's bounds were (re)laid out —
        // first display, rotation, safe-area changes — the one place all
        // viewport metrics are current.
        #[unsafe(method(viewDidLayoutSubviews))]
        fn view_did_layout_subviews(&self) {
            // SAFETY: plain forward to `UIViewController`'s implementation
            // (documented to do nothing, called for protocol hygiene).
            let _: () = unsafe { msg_send![super(self), viewDidLayoutSubviews] };
            let view = self
                .view()
                .expect("viewDidLayoutSubviews implies the view is loaded");
            present(&view);
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

/// The single render path: reads the view's current metrics, has the app
/// draw a frame at them, and installs it as the view's layer contents.
fn present(view: &UIView) {
    let Some(window) = view.window() else {
        // Not attached to a window: nothing on screen to present to.
        return;
    };
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
        // Degenerate mid-transition layout; a real pass follows.
        return;
    }

    let safe_area = SafeAreaInsets {
        left: insets.left,
        top: insets.top,
        right: insets.right,
        bottom: insets.bottom,
    };
    let metrics = Metrics {
        width,
        height,
        scale,
        safe_area,
    };
    if LAST_METRICS.get() == Some(metrics) {
        return;
    }
    LAST_METRICS.set(Some(metrics));

    let mut frame = Frame {
        width,
        height,
        scale,
        safe_area,
        pixels: vec![0; width as usize * height as usize * 4],
    };
    DRAW.with_borrow_mut(|draw| {
        let draw = draw
            .as_mut()
            .expect("run() parks the draw callback before UIKit launches");
        draw(&mut frame);
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
/// buffer into a `CFData` — fine at metrics-change frequency; presentation
/// cost is revisited with the frame loop (005) and the GPU decision (008).
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

pub(crate) fn run(draw: DrawFn) -> ! {
    let mtm =
        MainThreadMarker::new().expect("mel_platform::run() must be called on the main thread");
    DRAW.with_borrow_mut(|slot| *slot = Some(draw));
    // Hand the thread to UIKit's event loop; it creates one `MelAppDelegate`
    // and calls `did_finish_launching` above.
    UIApplication::main(None, Some(&NSString::from_class(AppDelegate::class())), mtm)
}
