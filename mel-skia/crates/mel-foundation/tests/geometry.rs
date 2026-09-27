use mel_foundation::{Offset, Size};
use std::f64::consts::{FRAC_PI_2, PI};

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}

#[test]
fn constructors_are_const_and_do_not_normalize() {
    const POINT: Offset = Offset::new(-2.0, 3.0);
    const SQUARE: Size = Size::square(-4.0);
    const DIAMETER: Size = Size::from_radius(3.0);
    const WIDTH: Size = Size::from_width(10.0);
    const HEIGHT: Size = Size::from_height(20.0);
    assert_eq!(POINT.dx, -2.0);
    assert_eq!(POINT.dy, 3.0);
    assert_eq!(SQUARE, Size::new(-4.0, -4.0));
    assert_eq!(DIAMETER, Size::new(6.0, 6.0));
    assert_eq!(WIDTH, Size::new(10.0, f64::INFINITY));
    assert_eq!(HEIGHT, Size::new(f64::INFINITY, 20.0));
    assert_eq!(Offset::default(), Offset::ZERO);
    assert_eq!(Size::default(), Size::ZERO);
    assert_eq!(Offset::INFINITE, Offset::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(Size::INFINITE, Size::new(f64::INFINITY, f64::INFINITY));
    assert!(Offset::new(-0.0, 0.0).dx.is_sign_negative());
    assert!(Size::new(0.0, -0.0).height.is_sign_negative());
}

#[test]
fn offset_operator_table_and_helpers() {
    let a = Offset::new(8.0, -6.0);
    let b = Offset::new(-2.0, 3.0);
    for (actual, expected) in [
        (a + b, Offset::new(6.0, -3.0)),
        (a - b, Offset::new(10.0, -9.0)),
        (-a, Offset::new(-8.0, 6.0)),
        (a * -2.0, Offset::new(-16.0, 12.0)),
        (a / -2.0, Offset::new(-4.0, 3.0)),
        (a % 5.0, Offset::new(3.0, 4.0)),
        (a.div_trunc(4.0), Offset::new(2.0, -1.0)),
        (a.div_trunc(-4.0), Offset::new(-2.0, 1.0)),
        (a.scale(0.5, -2.0), Offset::new(4.0, 12.0)),
        (a.translate(-3.0, 2.0), Offset::new(5.0, -4.0)),
    ] {
        assert_eq!(actual, expected);
    }
    assert_eq!(a + Offset::ZERO, a);
    assert_eq!((a + b) - b, a);
    assert_eq!(-(-a), a);
}

#[test]
fn size_operator_table_preserves_result_types() {
    let size = Size::new(8.0, -6.0);
    let delta = Offset::new(-2.0, 3.0);
    for (actual, expected) in [
        (size + delta, Size::new(6.0, -3.0)),
        (size - delta, Size::new(10.0, -9.0)),
        (size * -2.0, Size::new(-16.0, 12.0)),
        (size / -2.0, Size::new(-4.0, 3.0)),
        (size % 5.0, Size::new(3.0, 4.0)),
        (size.div_trunc(4.0), Size::new(2.0, -1.0)),
        (size.div_trunc(-4.0), Size::new(-2.0, 1.0)),
    ] {
        assert_eq!(actual, expected);
    }
    let difference: Offset = size - Size::new(3.0, 2.0);
    assert_eq!(difference, Offset::new(5.0, -8.0));
    assert_eq!(Size::new(3.0, 2.0) + difference, size);
    assert_eq!((size + delta) - delta, size);
}

#[test]
fn component_comparisons_are_neither_lexicographic_nor_rust_ordering() {
    // Expected order: strictly less, <=, strictly greater, >=.
    for (x, y, expected) in [
        (3.0, 4.0, [true, true, false, false]),
        (2.0, 4.0, [false, true, false, false]),
        (3.0, 3.0, [false, true, false, false]),
        (2.0, 3.0, [false, true, false, true]),
        (1.0, 2.0, [false, false, true, true]),
        (1.0, 3.0, [false, false, false, true]),
        (2.0, 2.0, [false, false, false, true]),
        (1.0, 4.0, [false; 4]),
        (3.0, 2.0, [false; 4]),
        (f64::NAN, 4.0, [false; 4]),
        (3.0, f64::NAN, [false; 4]),
        (f64::INFINITY, f64::INFINITY, [true, true, false, false]),
    ] {
        let a = Offset::new(2.0, 3.0);
        let b = Offset::new(x, y);
        assert_eq!(
            [a.all_lt(b), a.all_le(b), a.all_gt(b), a.all_ge(b)],
            expected
        );
        let a = Size::new(2.0, 3.0);
        let b = Size::new(x, y);
        assert_eq!(
            [a.all_lt(b), a.all_le(b), a.all_gt(b), a.all_ge(b)],
            expected
        );
    }
    let nan = Offset::new(f64::NAN, 0.0);
    assert!(!nan.all_le(nan));
    assert!(!Size::new(0.0, f64::NAN).all_ge(Size::ZERO));
}

#[test]
fn equality_uses_ieee_components() {
    assert_eq!(Offset::new(-0.0, 0.0), Offset::ZERO);
    assert_eq!(Size::new(0.0, -0.0), Size::ZERO);
    assert_ne!(
        Offset::new(1.0, 2.0),
        Offset::new(1.0, 2.0 + f64::EPSILON * 2.0)
    );
    assert_ne!(Size::new(1.0, 2.0), Size::new(2.0, 1.0));
    let point = Offset::new(f64::NAN, 1.0);
    let size = Size::new(1.0, f64::NAN);
    assert_ne!(point, point);
    assert_ne!(size, size);
}

#[test]
fn finite_and_infinite_classification_checks_both_axes_and_signs() {
    for (x, y, finite, infinite) in [
        (0.0, -0.0, true, false),
        (-3.0, f64::MAX, true, false),
        (f64::INFINITY, 1.0, false, true),
        (1.0, f64::NEG_INFINITY, false, true),
        (f64::NEG_INFINITY, 1.0, false, true),
        (1.0, f64::INFINITY, false, true),
        (f64::NAN, 1.0, false, false),
        (1.0, f64::NAN, false, false),
        (f64::INFINITY, f64::NAN, false, true),
    ] {
        assert_eq!(Offset::new(x, y).is_finite(), finite);
        assert_eq!(Size::new(x, y).is_finite(), finite);
        assert_eq!(Offset::new(x, y).is_infinite(), infinite);
        assert_eq!(Size::new(x, y).is_infinite(), infinite);
    }
}

#[test]
fn distances_and_directions_use_screen_coordinates() {
    let point = Offset::new(3.0, 4.0);
    assert_eq!(point.distance_squared(), 25.0);
    assert_eq!(point.distance(), 5.0);
    assert_eq!(Offset::ZERO.distance(), 0.0);
    for (angle, expected) in [
        (0.0, Offset::new(2.0, 0.0)),
        (FRAC_PI_2, Offset::new(0.0, 2.0)),
        (PI, Offset::new(-2.0, 0.0)),
        (-FRAC_PI_2, Offset::new(0.0, -2.0)),
    ] {
        let point = Offset::from_direction(angle, 2.0);
        near(point.dx, expected.dx);
        near(point.dy, expected.dy);
        near(point.direction(), angle);
        near(point.distance(), 2.0);
    }
    assert_eq!(Offset::from_direction(0.0, -2.0), Offset::new(-2.0, 0.0));
    assert!(Offset::new(f64::MAX, 0.0).distance().is_infinite());
    assert!(Offset::new(f64::NAN, f64::INFINITY).distance().is_nan());
    assert!(Offset::new(1.0, f64::NAN).direction().is_nan());
}

#[test]
fn containment_includes_only_top_and_left_edges() {
    let size = Size::new(10.0, 20.0);
    for (point, expected) in [
        (Offset::ZERO, true),
        (Offset::new(-0.0, -0.0), true),
        (Offset::new(0.0, 19.0), true),
        (Offset::new(9.0, 0.0), true),
        (Offset::new(5.0, 10.0), true),
        (Offset::new(10.0, 10.0), false),
        (Offset::new(5.0, 20.0), false),
        (Offset::new(-f64::EPSILON, 1.0), false),
        (Offset::new(1.0, -f64::EPSILON), false),
        (Offset::new(f64::NAN, 1.0), false),
        (Offset::new(1.0, f64::NAN), false),
        (Offset::INFINITE, false),
    ] {
        assert_eq!(size.contains(point), expected, "{point:?}");
    }
}

#[test]
fn empty_and_unbounded_sizes_have_explicit_containment() {
    for size in [
        Size::ZERO,
        Size::new(0.0, 2.0),
        Size::new(2.0, -0.0),
        Size::new(-2.0, 3.0),
        Size::new(3.0, -2.0),
        Size::new(-2.0, -3.0),
    ] {
        assert!(size.is_empty());
        assert!(!size.contains(Offset::ZERO));
        assert!(!size.contains(Offset::new(-1.0, -1.0)));
    }
    assert!(!Size::INFINITE.is_empty());
    assert!(Size::INFINITE.contains(Offset::new(f64::MAX, f64::MAX)));
    assert!(!Size::INFINITE.contains(Offset::INFINITE));
    assert!(!Size::INFINITE.contains(Offset::new(f64::NEG_INFINITY, 0.0)));
    for size in [Size::new(f64::NAN, 2.0), Size::new(2.0, f64::NAN)] {
        assert!(!size.is_empty());
        assert!(!size.contains(Offset::ZERO));
    }
    assert!(Size::new(f64::NAN, -1.0).is_empty());
}

#[test]
fn all_nine_anchors_include_the_supplied_origin() {
    let size = Size::new(10.0, 20.0);
    let origin = Offset::new(-5.0, 3.0);
    let actual = [
        size.top_left(origin),
        size.top_center(origin),
        size.top_right(origin),
        size.center_left(origin),
        size.center(origin),
        size.center_right(origin),
        size.bottom_left(origin),
        size.bottom_center(origin),
        size.bottom_right(origin),
    ];
    let expected = [
        (-5.0, 3.0),
        (0.0, 3.0),
        (5.0, 3.0),
        (-5.0, 13.0),
        (0.0, 13.0),
        (5.0, 13.0),
        (-5.0, 23.0),
        (0.0, 23.0),
        (5.0, 23.0),
    ];
    for (actual, (x, y)) in actual.into_iter().zip(expected) {
        assert_eq!(actual, Offset::new(x, y));
    }
    assert_eq!(Size::ZERO.center(origin), origin);
    assert_eq!(
        Size::new(-10.0, -20.0).bottom_right(origin),
        Offset::new(-15.0, -17.0)
    );
    // Anchors on untouched axes must not compute 0 * infinity.
    assert_eq!(Size::INFINITE.top_left(origin), origin);
    assert_eq!(
        Size::INFINITE.top_center(origin),
        Offset::new(f64::INFINITY, 3.0)
    );
    assert_eq!(
        Size::INFINITE.center_left(origin),
        Offset::new(-5.0, f64::INFINITY)
    );
    let signed_origin = Offset::new(-0.0, -0.0);
    assert!(size.top_center(signed_origin).dy.is_sign_negative());
    assert!(size.top_right(signed_origin).dy.is_sign_negative());
    assert!(size.center_left(signed_origin).dx.is_sign_negative());
    assert!(size.bottom_left(signed_origin).dx.is_sign_negative());
}

#[test]
fn aspect_ratio_covers_zero_signs_negative_extents_and_infinity() {
    for (size, expected) in [
        (Size::new(16.0, 9.0), 16.0 / 9.0),
        (Size::new(-4.0, 2.0), -2.0),
        (Size::new(4.0, -2.0), -2.0),
        (Size::ZERO, 0.0),
        (Size::new(-0.0, -0.0), 0.0),
        (Size::new(2.0, 0.0), f64::INFINITY),
        (Size::new(2.0, -0.0), f64::INFINITY),
        (Size::new(-2.0, 0.0), f64::NEG_INFINITY),
        (Size::new(-2.0, -0.0), f64::NEG_INFINITY),
        (Size::new(0.0, 2.0), 0.0),
        (Size::new(2.0, f64::INFINITY), 0.0),
        (Size::new(f64::INFINITY, 2.0), f64::INFINITY),
    ] {
        assert_eq!(size.aspect_ratio(), expected, "{size:?}");
    }
    for size in [
        Size::INFINITE,
        Size::new(f64::NAN, 0.0),
        Size::new(f64::NAN, 2.0),
        Size::new(0.0, f64::NAN),
    ] {
        assert!(size.aspect_ratio().is_nan());
    }
}

#[test]
fn side_magnitudes_propagate_nan_and_flipping_preserves_extents() {
    for (size, shortest, longest) in [
        (Size::new(-3.0, 4.0), 3.0, 4.0),
        (Size::new(4.0, -3.0), 3.0, 4.0),
        (Size::new(-0.0, 0.0), 0.0, 0.0),
        (Size::new(f64::NEG_INFINITY, 2.0), 2.0, f64::INFINITY),
        (Size::INFINITE, f64::INFINITY, f64::INFINITY),
    ] {
        assert_eq!(size.shortest_side(), shortest);
        assert_eq!(size.longest_side(), longest);
        assert_eq!(size.flipped().flipped(), size);
    }
    assert_eq!(Size::new(-3.0, 4.0).flipped(), Size::new(4.0, -3.0));
    for size in [Size::new(f64::NAN, 2.0), Size::new(2.0, f64::NAN)] {
        assert!(size.shortest_side().is_nan());
        assert!(size.longest_side().is_nan());
    }
}

#[test]
fn modulo_is_euclidean_for_both_divisor_signs_and_normalizes_zero() {
    for divisor in [3.0, -3.0] {
        assert_eq!(Offset::new(-5.0, 5.0) % divisor, Offset::new(1.0, 2.0));
        assert_eq!(Size::new(-5.0, 5.0) % divisor, Size::new(1.0, 2.0));
        let point = Offset::new(-0.0, -6.0) % divisor;
        let size = Size::new(-0.0, -6.0) % divisor;
        assert_eq!(point, Offset::ZERO);
        assert!(!point.dx.is_sign_negative() && !point.dy.is_sign_negative());
        assert!(!size.width.is_sign_negative() && !size.height.is_sign_negative());
    }
    for divisor in [0.0, -0.0, f64::NAN] {
        let point = Offset::new(5.0, -5.0) % divisor;
        let size = Size::new(5.0, -5.0) % divisor;
        assert!(point.dx.is_nan() && point.dy.is_nan());
        assert!(size.width.is_nan() && size.height.is_nan());
    }
    assert!((Offset::INFINITE % 2.0).dx.is_nan());
    assert!((Size::INFINITE % 2.0).height.is_nan());
    assert_eq!(Offset::new(5.0, 0.0) % f64::INFINITY, Offset::new(5.0, 0.0));
}

#[test]
fn division_and_truncation_propagate_nonfinite_values_without_panics() {
    assert_eq!(
        Offset::new(1.0, -1.0) / 0.0,
        Offset::new(f64::INFINITY, f64::NEG_INFINITY)
    );
    assert_eq!(
        Size::new(1.0, -1.0) / -0.0,
        Size::new(f64::NEG_INFINITY, f64::INFINITY)
    );
    assert_eq!(
        Offset::new(1.0, -1.0).div_trunc(0.0),
        Offset::new(f64::INFINITY, f64::NEG_INFINITY)
    );
    assert_eq!(
        Size::new(1.0, -1.0).div_trunc(0.0),
        Size::new(f64::INFINITY, f64::NEG_INFINITY)
    );
    assert!((Offset::ZERO / 0.0).dx.is_nan());
    assert!((Size::ZERO / 0.0).width.is_nan());
    assert!(Offset::ZERO.div_trunc(0.0).dy.is_nan());
    assert!(Size::ZERO.div_trunc(0.0).height.is_nan());
    assert!((Offset::INFINITE * 0.0).dx.is_nan());
    assert!((Size::INFINITE * 0.0).width.is_nan());
    assert!((Offset::INFINITE - Offset::INFINITE).dy.is_nan());
    assert!((Size::INFINITE - Size::INFINITE).dy.is_nan());
}

#[test]
fn interpolation_handles_endpoints_midpoints_and_extrapolation() {
    for (t, x, y) in [
        (0.0, 2.0, 4.0),
        (0.5, 4.0, 8.0),
        (1.0, 6.0, 12.0),
        (-1.0, -2.0, -4.0),
        (2.0, 10.0, 20.0),
    ] {
        assert_eq!(
            Offset::new(2.0, 4.0).lerp(Offset::new(6.0, 12.0), t),
            Offset::new(x, y)
        );
        assert_eq!(
            Size::new(2.0, 4.0).lerp(Size::new(6.0, 12.0), t),
            Size::new(x, y)
        );
    }
    // Weighted interpolation avoids overflowing b - a for these finite inputs.
    assert_eq!(
        Offset::new(-f64::MAX, 0.0).lerp(Offset::new(f64::MAX, 0.0), 0.5),
        Offset::ZERO
    );
    assert_eq!(Size::INFINITE.lerp(Size::INFINITE, 0.0), Size::INFINITE);
    assert_eq!(
        Offset::INFINITE.lerp(Offset::INFINITE, 1.0),
        Offset::INFINITE
    );
    assert!(Offset::ZERO.lerp(Offset::INFINITE, 0.0).dx.is_nan());
    assert!(
        Size::ZERO
            .lerp(Size::new(f64::NAN, 1.0), 1.0)
            .width
            .is_nan()
    );
    assert!(
        Offset::ZERO
            .lerp(Offset::new(1.0, 1.0), f64::NAN)
            .dy
            .is_nan()
    );
    assert!(
        Size::ZERO
            .lerp(Size::new(1.0, 1.0), f64::INFINITY)
            .height
            .is_nan()
    );
}
