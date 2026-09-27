use mel_foundation::{Offset, RRect, Radius, Rect};

fn corners(rect: RRect) -> [Radius; 4] {
    [
        rect.top_left,
        rect.top_right,
        rect.bottom_right,
        rect.bottom_left,
    ]
}

fn with_corners(bounds: Rect, radii: [Radius; 4]) -> RRect {
    RRect::from_rect_corners(bounds, radii[0], radii[1], radii[2], radii[3])
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}

#[test]
fn const_constructors_defaults_and_corner_order() {
    const BOUNDS: Rect = Rect::from_ltwh(10.0, 20.0, 100.0, 80.0);
    const CIRCLE: Radius = Radius::circular(4.0);
    const ELLIPSE: Radius = Radius::elliptical(8.0, 2.0);
    const UNIFORM: RRect = RRect::from_rect_radius(BOUNDS, CIRCLE);
    const MIXED: RRect =
        RRect::from_rect_corners(BOUNDS, CIRCLE, ELLIPSE, Radius::ZERO, Radius::circular(1.0));
    assert_eq!(CIRCLE, Radius::elliptical(4.0, 4.0));
    assert_eq!(corners(UNIFORM), [CIRCLE; 4]);
    assert_eq!(
        corners(MIXED),
        [CIRCLE, ELLIPSE, Radius::ZERO, Radius::circular(1.0)]
    );
    assert_eq!(MIXED.outer_rect(), BOUNDS);
    assert_eq!(Radius::default(), Radius::ZERO);
    assert_eq!(RRect::default(), RRect::ZERO);
    assert_eq!(MIXED.scale_radii(), Some(MIXED));
}

#[test]
fn constructors_preserve_invalid_values_and_signed_zero() {
    let radius = Radius::elliptical(-0.0, -3.0);
    let bounds = Rect::from_ltrb(5.0, 4.0, -5.0, -4.0);
    let rounded = RRect::from_rect_radius(bounds, radius);
    assert!(rounded.top_left.x.is_sign_negative());
    assert_eq!(corners(rounded), [radius; 4]);
    assert_eq!(rounded.outer_rect(), bounds);
    assert!(rounded.scale_radii().is_none());
    let nan = Radius::elliptical(f64::NAN, f64::INFINITY);
    assert_ne!(nan, nan);
    let invalid = RRect::from_rect_radius(bounds, nan);
    assert_ne!(invalid, invalid);
    assert!(invalid.top_left.x.is_nan());
    assert_eq!(invalid.bottom_right.y, f64::INFINITY);
    let valid_zero = RRect::from_rect_radius(Rect::ZERO, Radius::circular(-0.0));
    assert!(
        valid_zero
            .scale_radii()
            .unwrap()
            .top_left
            .x
            .is_sign_negative()
    );
}

#[test]
fn square_corners_match_half_open_rect_containment() {
    let bounds = Rect::from_ltrb(-5.0, 3.0, 5.0, 23.0);
    for radius in [
        Radius::ZERO,
        Radius::elliptical(3.0, 0.0),
        Radius::elliptical(0.0, 8.0),
    ] {
        let rounded = RRect::from_rect_radius(bounds, radius);
        for x in -6..=6 {
            for y in 2..=24 {
                let point = Offset::new(x as f64, y as f64);
                assert_eq!(rounded.contains(point), bounds.contains(point));
            }
        }
    }
}

#[test]
fn circular_arcs_are_included_but_outer_right_and_bottom_are_not() {
    let rounded = RRect::from_rect_radius(
        Rect::from_ltwh(0.0, 0.0, 100.0, 100.0),
        Radius::circular(10.0),
    );
    // The 3-4-5 triangle gives exact on-arc samples away from outer edges.
    for point in [
        (4.0, 2.0),
        (96.0, 2.0),
        (96.0, 98.0),
        (4.0, 98.0),
        (10.0, 0.0),
        (0.0, 10.0),
    ] {
        assert!(rounded.contains(Offset::new(point.0, point.1)), "{point:?}");
    }
    for point in [
        (0.0, 0.0),
        (99.0, 1.0),
        (99.0, 99.0),
        (1.0, 99.0),
        (100.0, 10.0),
        (10.0, 100.0),
    ] {
        assert!(
            !rounded.contains(Offset::new(point.0, point.1)),
            "{point:?}"
        );
    }
}

#[test]
fn each_corner_uses_its_own_elliptical_radii() {
    let bounds = Rect::from_ltwh(10.0, 20.0, 100.0, 80.0);
    let rounded = with_corners(
        bounds,
        [
            Radius::elliptical(20.0, 10.0),
            Radius::elliptical(30.0, 20.0),
            Radius::elliptical(40.0, 30.0),
            Radius::elliptical(10.0, 40.0),
        ],
    );
    for (index, radius) in corners(rounded).into_iter().enumerate() {
        let point_at = |x: f64, y: f64| {
            Offset::new(
                if index == 0 || index == 3 {
                    bounds.left + x
                } else {
                    bounds.right - x
                },
                if index < 2 {
                    bounds.top + y
                } else {
                    bounds.bottom - y
                },
            )
        };
        assert!(rounded.contains(point_at(radius.x * 0.5, radius.y * 0.25)));
        assert!(!rounded.contains(point_at(radius.x * 0.1, radius.y * 0.1)));
        // Removing only this corner makes the outer-corner sample square.
        let mut changed = corners(rounded);
        changed[index] = Radius::ZERO;
        assert!(with_corners(bounds, changed).contains(point_at(0.1, 0.1)));
    }
}

#[test]
fn every_side_can_limit_the_common_scale_without_mutating_the_input() {
    let bounds = Rect::from_ltwh(10.0, 20.0, 100.0, 80.0);
    for side in 0..4 {
        let mut radii = [Radius::circular(2.0); 4];
        match side {
            0 => {
                radii[0].x = 120.0;
                radii[1].x = 80.0;
            }
            1 => {
                radii[1].y = 90.0;
                radii[2].y = 70.0;
            }
            2 => {
                radii[2].x = 80.0;
                radii[3].x = 120.0;
            }
            _ => {
                radii[3].y = 70.0;
                radii[0].y = 90.0;
            }
        }
        let original = with_corners(bounds, radii);
        let fitted = original.scale_radii().unwrap();
        assert_eq!(fitted.outer_rect(), bounds);
        assert_eq!(corners(original), radii);
        for (actual, before) in corners(fitted).into_iter().zip(radii) {
            assert_eq!(actual, Radius::elliptical(before.x / 2.0, before.y / 2.0));
        }
        assert_eq!(fitted.scale_radii(), Some(fitted));
    }
    // Multiple competing sides: height, not width, determines the factor.
    let tall_radii = RRect::from_rect_radius(bounds, Radius::elliptical(100.0, 160.0));
    assert_eq!(
        tall_radii.scale_radii().unwrap().top_left,
        Radius::elliptical(25.0, 40.0)
    );
}

#[test]
fn overflow_in_adjacent_radius_sums_does_not_collapse_finite_geometry() {
    let bounds = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
    let rounded = RRect::from_rect_radius(bounds, Radius::circular(f64::MAX));
    let fitted = rounded.scale_radii().unwrap();
    for radius in corners(fitted) {
        close(radius.x, 50.0);
        close(radius.y, 50.0);
    }
    assert!(rounded.contains(Offset::new(50.0, 50.0)));
    assert!(!rounded.contains(Offset::new(1.0, 1.0)));
    assert!(rounded.safe_inner_rect().unwrap().contains(bounds.center()));
}

#[test]
fn both_diagonal_ellipses_must_accept_the_point() {
    let bounds = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
    let large = Radius::circular(100.0);
    for radii in [
        [large, Radius::ZERO, large, Radius::ZERO],
        [Radius::ZERO, large, Radius::ZERO, large],
    ] {
        let rounded = with_corners(bounds, radii);
        assert_eq!(rounded.scale_radii(), Some(rounded));
        assert!(rounded.contains(Offset::new(50.0, 50.0)));
        for (x, y) in [(20.0, 20.0), (80.0, 80.0), (20.0, 80.0), (80.0, 20.0)] {
            let expected = if radii[0] == large { x != y } else { x == y };
            assert_eq!(
                rounded.contains(Offset::new(x, y)),
                expected,
                "{radii:?}, ({x}, {y})"
            );
        }
    }
}

#[test]
fn invalid_radius_in_any_component_rejects_all_geometric_queries() {
    let bounds = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
    for index in 0..8 {
        for value in [-1.0, f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
            let mut radii = [Radius::circular(2.0); 4];
            if index % 2 == 0 {
                radii[index / 2].x = value;
            } else {
                radii[index / 2].y = value;
            }
            let rounded = with_corners(bounds, radii);
            assert!(rounded.scale_radii().is_none());
            assert!(rounded.safe_inner_rect().is_none());
            assert!(!rounded.contains(bounds.center()));
            assert!(!rounded.contains(Offset::new(1.0, 1.0)));
        }
    }
}

#[test]
fn invalid_bounds_and_points_never_pass_containment() {
    let bounds = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
    for index in 0..4 {
        for value in [f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
            let mut edges = [0.0, 0.0, 100.0, 100.0];
            edges[index] = value;
            let rounded = RRect::from_rect_radius(
                Rect::from_ltrb(edges[0], edges[1], edges[2], edges[3]),
                Radius::ZERO,
            );
            assert!(rounded.scale_radii().is_none());
            assert!(rounded.safe_inner_rect().is_none());
            assert!(!rounded.contains(bounds.center()));
        }
    }
    for bounds in [
        Rect::from_ltrb(1.0, 0.0, -1.0, 10.0),
        Rect::from_ltrb(0.0, 1.0, 10.0, -1.0),
        Rect::from_ltrb(-f64::MAX, 0.0, f64::MAX, 10.0),
        Rect::from_ltrb(0.0, -f64::MAX, 10.0, f64::MAX),
    ] {
        let rounded = RRect::from_rect_radius(bounds, Radius::ZERO);
        assert!(rounded.scale_radii().is_none());
        assert!(rounded.safe_inner_rect().is_none());
        assert!(!rounded.contains(Offset::ZERO));
        assert_eq!(rounded.outer_rect(), bounds);
    }
    let rounded = RRect::from_rect_radius(bounds, Radius::circular(2.0));
    for value in [f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
        assert!(!rounded.contains(Offset::new(value, 50.0)));
        assert!(!rounded.contains(Offset::new(50.0, value)));
    }
}

#[test]
fn empty_bounds_stay_empty_and_fit_without_panics() {
    for bounds in [
        Rect::ZERO,
        Rect::from_ltwh(10.0, 20.0, 0.0, 80.0),
        Rect::from_ltwh(10.0, 20.0, 100.0, 0.0),
    ] {
        let rounded = RRect::from_rect_radius(bounds, Radius::circular(10.0));
        assert_eq!(
            rounded.scale_radii(),
            Some(RRect::from_rect_radius(bounds, Radius::ZERO))
        );
        assert_eq!(rounded.safe_inner_rect(), Some(bounds));
        assert!(!rounded.contains(bounds.center()));
    }
    assert_eq!(RRect::ZERO.scale_radii(), Some(RRect::ZERO));
    assert_eq!(RRect::ZERO.safe_inner_rect(), Some(Rect::ZERO));
}

#[test]
fn safe_inner_bounds_use_side_maxima_and_fitted_radii() {
    let bounds = Rect::from_ltwh(10.0, 20.0, 100.0, 80.0);
    let rounded = with_corners(
        bounds,
        [
            Radius::elliptical(10.0, 20.0),
            Radius::elliptical(30.0, 10.0),
            Radius::elliptical(20.0, 40.0),
            Radius::elliptical(40.0, 30.0),
        ],
    );
    let inner = rounded.safe_inner_rect().unwrap();
    let inset = 1.0 - std::f64::consts::FRAC_1_SQRT_2;
    for (actual, expected) in [
        (inner.left, 10.0 + 40.0 * inset),
        (inner.top, 20.0 + 20.0 * inset),
        (inner.right, 110.0 - 30.0 * inset),
        (inner.bottom, 100.0 - 40.0 * inset),
    ] {
        close(actual, expected);
    }
    assert_eq!(
        RRect::from_rect_radius(bounds, Radius::ZERO).safe_inner_rect(),
        Some(bounds)
    );
    let oversized = RRect::from_rect_radius(bounds, Radius::elliptical(100.0, 80.0));
    let fitted = RRect::from_rect_radius(bounds, Radius::elliptical(50.0, 40.0));
    assert_eq!(oversized.safe_inner_rect(), fitted.safe_inner_rect());
}

#[test]
fn sampled_ellipses_match_an_independent_horizontal_slice_oracle() {
    // At each height the four quarter-ellipses constrain an x interval. This
    // checks containment without repeating its corner-region branch logic.
    let bounds = Rect::from_ltwh(0.0, 0.0, 40.0, 30.0);
    let options = [
        Radius::ZERO,
        Radius::elliptical(0.0, 12.0),
        Radius::elliptical(8.0, 0.0),
        Radius::circular(6.0),
        Radius::elliptical(18.0, 10.0),
        Radius::elliptical(40.0, 30.0),
    ];
    for a in options {
        for b in options {
            for radii in [[a, b, a, b], [a, a, b, b], [a, b, b, a]] {
                let rounded = with_corners(bounds, radii);
                let fitted = rounded.scale_radii().unwrap();
                let inset_at = |radius: Radius, y: f64| {
                    if radius.y == 0.0 || y >= radius.y {
                        0.0
                    } else {
                        let normalized = (radius.y - y) / radius.y;
                        radius.x * (1.0 - (1.0 - normalized * normalized).sqrt())
                    }
                };
                for yi in 0..30 {
                    let y = yi as f64 + 0.37;
                    let left =
                        inset_at(fitted.top_left, y).max(inset_at(fitted.bottom_left, 30.0 - y));
                    let right = 40.0
                        - inset_at(fitted.top_right, y)
                            .max(inset_at(fitted.bottom_right, 30.0 - y));
                    for xi in 0..40 {
                        let x = xi as f64 + 0.19;
                        assert_eq!(
                            rounded.contains(Offset::new(x, y)),
                            x >= left && x <= right,
                            "{rounded:?}, ({x}, {y})"
                        );
                    }
                }
                let inner = rounded.safe_inner_rect().unwrap();
                // Stay away from exact arc endpoints, where f64 rounding can
                // put the mathematically on-curve point just outside.
                for x in 1..10 {
                    for y in 1..10 {
                        assert!(rounded.contains(Offset::new(
                            inner.left + inner.width() * x as f64 / 10.0,
                            inner.top + inner.height() * y as f64 / 10.0,
                        )));
                    }
                }
            }
        }
    }
}
