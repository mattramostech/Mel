//! Pure geometry demo: cargo run -p mel-foundation --example 012_radius_rrect
use mel_foundation::{Offset, RRect, Radius, Rect};

fn main() {
    let bounds = Rect::from_ltwh(10.0, 20.0, 100.0, 80.0);
    let rounded = RRect::from_rect_corners(
        bounds,
        Radius::circular(20.0),
        Radius::elliptical(30.0, 10.0),
        Radius::ZERO,
        Radius::circular(8.0),
    );
    let inner = rounded.safe_inner_rect().unwrap();
    println!("outer={:?}, safe inner={inner:?}", rounded.outer_rect());
    assert_eq!(rounded.outer_rect(), bounds);
    assert!(rounded.contains(bounds.center()));
    assert!(inner.contains(bounds.center()));
    assert!(!rounded.contains(bounds.top_left()));
    assert!(rounded.contains(Offset::new(30.0, 20.0)));
    assert!(!rounded.contains(bounds.bottom_right()));

    let oversized = RRect::from_rect_radius(bounds, Radius::elliptical(100.0, 80.0));
    let fitted = oversized.scale_radii().unwrap();
    assert_eq!(fitted.top_left, Radius::elliptical(50.0, 40.0));
    assert_eq!(oversized.top_left, Radius::elliptical(100.0, 80.0));
    println!(
        "oversized radius={:?}, fitted={:?}",
        oversized.top_left, fitted.top_left
    );

    let diagonal = RRect::from_rect_corners(
        Rect::from_ltwh(0.0, 0.0, 100.0, 100.0),
        Radius::circular(100.0),
        Radius::ZERO,
        Radius::circular(100.0),
        Radius::ZERO,
    );
    assert!(diagonal.contains(Offset::new(50.0, 50.0)));
    assert!(!diagonal.contains(Offset::new(80.0, 80.0)));
    let invalid = RRect::from_rect_radius(bounds, Radius::circular(-1.0));
    assert!(invalid.scale_radii().is_none());
    assert!(!invalid.contains(bounds.center()));
    assert!(!RRect::ZERO.contains(Offset::ZERO));
    println!("corner, fitting, diagonal and invalid-geometry checks passed");
}
