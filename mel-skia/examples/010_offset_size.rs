//! Pure geometry demo: cargo run -p mel-foundation --example 010_offset_size
use mel_foundation::{Offset, Size};

fn main() {
    let origin = Offset::new(12.0, 20.0);
    let size = Size::new(100.0, 80.0);
    let center = size.center(origin);
    println!("origin={origin:?}, size={size:?}, center={center:?}");
    assert!(size.contains(center - origin));
    assert!(!size.contains(size.bottom_right(origin) - origin));

    let grown = size + Offset::new(8.0, 4.0);
    let growth: Offset = grown - size;
    println!(
        "grown={grown:?}, growth={growth:?}, ratio={}",
        size.aspect_ratio()
    );
    assert_eq!(growth, Offset::new(8.0, 4.0));
    assert!(size.all_lt(grown));

    for t in [0.0, 0.5, 1.0, 1.5] {
        println!("t={t}: {:?}", origin.lerp(center, t));
    }
    let unbounded = Size::from_width(100.0);
    assert!(unbounded.is_infinite());
    assert!(unbounded.contains(Offset::new(50.0, 1_000_000.0)));
    assert_eq!(Size::ZERO.aspect_ratio(), 0.0);
    assert!(!Size::new(f64::NAN, 80.0).contains(Offset::ZERO));
    println!("unbounded={unbounded:?}; geometry checks passed");
}
