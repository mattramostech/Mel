//! Pure color demo: cargo run -p mel-foundation --example 013_color
use mel_foundation::Color;

fn main() {
    let honey = Color::from_argb32(0xffff_c107);
    let faded = honey.with_opacity(0.5).unwrap();
    assert_eq!(faded.to_argb32(), 0x80ff_c107);
    assert_eq!(faded.opacity(), 128.0 / 255.0);
    assert_eq!(Color::from_rgbo(255, 193, 7, 0.5), Some(faded));
    println!(
        "honey={:#010x}, half opacity={:#010x}, stored opacity={:.6}",
        honey.to_argb32(),
        faded.to_argb32(),
        faded.opacity()
    );

    for t in [-0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5] {
        let color = Color::lerp(Color::RED, Color::BLUE, t).unwrap();
        println!("red -> blue, t={t:4.2}: {:#010x}", color.to_argb32());
    }
    assert_eq!(
        Color::lerp(Color::RED, Color::BLUE, 0.5),
        Some(Color::from_argb(255, 128, 0, 128))
    );

    let fade = Color::lerp(honey.with_alpha(0), honey, 0.5).unwrap();
    let from_black = Color::lerp(Color::TRANSPARENT, honey, 0.5).unwrap();
    assert_eq!(fade, faded);
    assert_eq!(from_black, Color::from_argb(128, 128, 97, 4));
    println!(
        "fade with preserved RGB={:#010x}, from transparent black={:#010x}",
        fade.to_argb32(),
        from_black.to_argb32()
    );
    assert!(honey.with_opacity(1.1).is_none());
    assert!(honey.with_opacity(f64::NAN).is_none());
    assert!(Color::lerp(Color::BLACK, Color::WHITE, f64::INFINITY).is_none());
    println!("packing, opacity rounding, interpolation and invalid-input checks passed");
}
