//! Pure geometry demo: cargo run -p mel-foundation --example 011_rect
use mel_foundation::{Offset, Rect, Size};

fn main() {
    let bounds = Offset::new(12.0, 20.0) & Size::new(100.0, 80.0);
    let clip = Rect::from_center(Offset::new(100.0, 80.0), 40.0, 40.0);
    let visible = bounds.intersect(clip);
    let combined = bounds.union(clip);
    println!("bounds={bounds:?}, center={:?}", bounds.center());
    println!("intersection={visible:?}, union={combined:?}");
    assert_eq!(visible, Rect::from_ltrb(80.0, 60.0, 112.0, 100.0));
    assert_eq!(combined, Rect::from_ltrb(12.0, 20.0, 120.0, 100.0));
    assert!(bounds.overlaps(clip));
    assert!(bounds.contains(bounds.top_left()));
    assert!(!bounds.contains(bounds.bottom_right()));

    let moved = bounds.shift(Offset::new(200.0, 0.0));
    let disjoint = bounds.intersect(moved);
    assert!(disjoint.is_empty());
    assert_eq!(disjoint.width(), -100.0);
    assert!(!bounds.overlaps(moved));
    assert_eq!(bounds.inflate(4.0).deflate(4.0), bounds);

    let empty = Rect::from_ltwh(20.0, 30.0, 0.0, 10.0);
    let inverted = bounds.deflate(60.0);
    assert!(inverted.is_empty());
    assert!(!bounds.overlaps(empty));
    assert!(!bounds.overlaps(inverted));
    let invalid = Rect::from_ltrb(f64::NAN, 20.0, 112.0, 100.0);
    assert!(invalid.has_nan());
    assert!(!bounds.overlaps(invalid));
    println!("disjoint={disjoint:?}, deflated={inverted:?}; rectangle checks passed");
}
