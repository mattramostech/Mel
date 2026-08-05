//! The UIKit side of the shell: a Rust-defined `UIApplicationDelegate` that
//! creates and owns the window (the classic, scene-less lifecycle — ADR 002,
//! debt 002-1). This module is the only place allowed to speak Objective-C;
//! every `unsafe` here is ObjC interop.

use core::cell::{OnceCell, RefCell};
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
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey, UIColor, UIScreen,
    UIViewController, UIWindow,
};

use crate::Frame;

type DrawFn = Box<dyn FnOnce(&mut Frame)>;

thread_local! {
    /// Parked here by `run()` until UIKit delivers `didFinishLaunching`.
    /// A thread-local (not a static) works because both ends are the main
    /// thread: `run()` asserts it, and UIKit guarantees it for delegates.
    static DRAW: RefCell<Option<DrawFn>> = const { RefCell::new(None) };
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
            let root_view = root
                .view()
                .expect("view controller loads its view on first access");
            root_view.setBackgroundColor(Some(&UIColor::whiteColor()));
            window.setRootViewController(Some(&root));

            // CPU-raster presentation (ADR 003): the app draws one frame into
            // a pixel buffer; Core Animation shows it as the root layer's
            // contents. Sized in logical points — scale factor is feature 004.
            let bounds = window.bounds();
            let width = bounds.size.width as u32;
            let height = bounds.size.height as u32;
            let mut frame = Frame {
                width,
                height,
                pixels: vec![0; width as usize * height as usize * 4],
            };
            let draw = DRAW
                .with_borrow_mut(Option::take)
                .expect("run() parks the draw callback before UIKit launches");
            draw(&mut frame);

            let image = cg_image_from(&frame);
            let contents: &CFType = &image;
            // SAFETY: `contents` accepts a CGImage (CALayer docs); CF types
            // are toll-free bridgeable to ObjC objects, which is exactly what
            // `CFType: AsRef<AnyObject>` encodes.
            unsafe { root_view.layer().setContents(Some(contents.as_ref())) };

            window.makeKeyAndVisible();

            if self.ivars().window.set(window).is_err() {
                unreachable!("application:didFinishLaunchingWithOptions: runs at most once");
            }
            true
        }
    }
);

/// Wraps a drawn frame into a `CGImage` for Core Animation. Copies the
/// buffer into a `CFData` — fine for one launch-time frame; presentation
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
            frame.width() as usize,
            frame.height() as usize,
            8,  // bits per component
            32, // bits per pixel
            frame.width() as usize * 4,
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
