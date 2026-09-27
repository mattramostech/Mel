use std::collections::HashSet;

use mel_foundation::Color;

fn channels(color: Color) -> [u8; 4] {
    [color.alpha(), color.red(), color.green(), color.blue()]
}

#[test]
fn const_packing_channel_order_and_default() {
    const PACKED: Color = Color::from_argb32(0x1234_5678);
    const BYTES: Color = Color::from_argb(0x12, 0x34, 0x56, 0x78);
    const VALUE: u32 = BYTES.to_argb32();
    const CHANNELS: [u8; 4] = [PACKED.alpha(), PACKED.red(), PACKED.green(), PACKED.blue()];
    const CLEAR: Color = BYTES.with_alpha(0);
    assert_eq!(PACKED, BYTES);
    assert_eq!(VALUE, 0x1234_5678);
    assert_eq!(CHANNELS, [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(CLEAR.to_argb32(), 0x0034_5678);
    assert_eq!(Color::default(), Color::TRANSPARENT);
    assert_eq!(Color::from_argb32(0x00ff_ffff).alpha(), 0);
}

#[test]
fn every_byte_round_trips_in_each_position() {
    for value in 0..=255_u8 {
        for slot in 0..4 {
            let mut bytes = [17, 39, 83, 211];
            bytes[slot] = value;
            let color = Color::from_argb(bytes[0], bytes[1], bytes[2], bytes[3]);
            assert_eq!(channels(color), bytes);
            assert_eq!(color.to_argb32(), u32::from_be_bytes(bytes));
            assert_eq!(Color::from_argb32(color.to_argb32()), color);
        }
    }
}

#[test]
fn equality_and_hash_preserve_hidden_rgb() {
    let red = Color::RED.with_alpha(0);
    let blue = Color::BLUE.with_alpha(0);
    assert_ne!(red, blue);
    assert_ne!(red, Color::TRANSPARENT);
    let colors = HashSet::from([red, blue, Color::TRANSPARENT, red]);
    assert_eq!(colors.len(), 3);
    assert!(colors.contains(&Color::from_argb(0, 255, 0, 0)));
}

#[test]
fn palette_is_rgb_extrema_with_opaque_primaries() {
    for (color, expected) in [
        (Color::TRANSPARENT, 0x0000_0000),
        (Color::BLACK, 0xff00_0000),
        (Color::WHITE, 0xffff_ffff),
        (Color::RED, 0xffff_0000),
        (Color::GREEN, 0xff00_ff00),
        (Color::BLUE, 0xff00_00ff),
        (Color::YELLOW, 0xffff_ff00),
        (Color::CYAN, 0xff00_ffff),
        (Color::MAGENTA, 0xffff_00ff),
    ] {
        assert_eq!(color.to_argb32(), expected);
    }
}

#[test]
fn opacity_rounds_half_up_and_replaces_alpha() {
    let color = Color::from_argb(19, 21, 43, 65);
    for (opacity, alpha) in [
        (-0.0, 0),
        (0.0, 0),
        (0.1, 26),
        (0.25, 64),
        (0.5_f64.next_down(), 127),
        (0.5, 128),
        (0.5_f64.next_up(), 128),
        (0.75, 191),
        (0.9, 230),
        (1.0, 255),
    ] {
        let expected = Color::from_argb(alpha, 21, 43, 65);
        assert_eq!(color.with_opacity(opacity), Some(expected));
        assert_eq!(Color::from_rgbo(21, 43, 65, opacity), Some(expected));
        assert_eq!(expected.opacity(), f64::from(alpha) / 255.0);
    }
    assert_eq!(color.alpha(), 19);
}

#[test]
fn alpha_opacity_round_trip_and_quantization_error() {
    for alpha in 0..=255 {
        let color = Color::CYAN.with_alpha(alpha);
        assert_eq!(color.with_opacity(color.opacity()), Some(color));
    }
    for step in 0..=10_000 {
        let opacity = f64::from(step) / 10_000.0;
        let color = Color::MAGENTA.with_opacity(opacity).unwrap();
        assert!((color.opacity() - opacity).abs() <= 0.5 / 255.0 + f64::EPSILON);
        assert_eq!(color.with_alpha(255), Color::MAGENTA);
    }
}

#[test]
fn invalid_opacity_is_rejected_without_clamping_or_wrapping() {
    for opacity in [
        -f64::MIN_POSITIVE,
        -f64::from_bits(1),
        -1.0,
        1.0_f64.next_up(),
        2.0,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        assert!(Color::WHITE.with_opacity(opacity).is_none());
        assert!(Color::from_rgbo(21, 43, 65, opacity).is_none());
    }
}

#[test]
fn lerp_rounding_and_straight_transparent_endpoints() {
    assert_eq!(
        Color::lerp(Color::RED, Color::BLUE, 0.5),
        Some(Color::from_argb(255, 128, 0, 128))
    );
    assert_eq!(
        Color::lerp(Color::BLACK, Color::WHITE, 0.5),
        Some(Color::from_argb(255, 128, 128, 128))
    );
    let color = Color::from_argb(255, 101, 151, 201);
    assert_eq!(
        Color::lerp(color.with_alpha(0), color, 0.5),
        Some(Color::from_argb(128, 101, 151, 201))
    );
    assert_eq!(
        Color::lerp(Color::TRANSPARENT, color, 0.5),
        Some(Color::from_argb(128, 51, 76, 101))
    );
    // Zero alpha does not suppress interpolation of hidden RGB.
    assert_eq!(
        Color::lerp(Color::RED.with_alpha(0), Color::BLUE.with_alpha(0), 0.5),
        Some(Color::from_argb(0, 128, 0, 128))
    );
}

#[test]
fn lerp_matches_integer_rational_oracle_for_all_byte_pairs() {
    // Quarter-step factors admit an exact integer oracle, independent of the
    // implementation's floating-point arithmetic, including rounding ties.
    for a in 0..=255_u8 {
        for b in 0..=255_u8 {
            let start = Color::from_argb(a, b, 255 - a, 255 - b);
            let end = Color::from_argb(b, a, 255 - b, 255 - a);
            for numerator in [-8, -4, -2, 0, 1, 2, 3, 4, 6, 8, 12] {
                let actual = Color::lerp(start, end, f64::from(numerator) / 4.0).unwrap();
                let expected = std::array::from_fn(|i| {
                    let weighted = (4 - numerator) * i32::from(channels(start)[i])
                        + numerator * i32::from(channels(end)[i]);
                    ((weighted.clamp(0, 4 * 255) + 2) / 4) as u8
                });
                assert_eq!(channels(actual), expected, "a={a}, b={b}, n={numerator}");
            }
        }
    }
}

#[test]
fn lerp_extreme_finite_factors_saturate_and_preserve_equal_channels() {
    let a = Color::from_argb(73, 0, 255, 41);
    let b = Color::from_argb(73, 255, 0, 41);
    assert_eq!(Color::lerp(a, b, f64::MAX), Some(b));
    assert_eq!(Color::lerp(a, b, -f64::MAX), Some(a));
    for t in [-f64::MAX, -0.0, 0.0, f64::from_bits(1), 1.0, f64::MAX] {
        assert_eq!(Color::lerp(a, a, t), Some(a));
    }
    assert_eq!(Color::lerp(a, b, f64::from_bits(1)), Some(a));
}

#[test]
fn lerp_non_finite_factors_are_rejected_even_for_equal_colors() {
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Color::lerp(Color::BLACK, Color::WHITE, t).is_none());
        assert!(Color::lerp(Color::RED, Color::RED, t).is_none());
    }
}
