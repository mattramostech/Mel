use mel_foundation::{Offset, Rect, Size};

#[test]
fn const_constructors_and_offset_size_operator_agree() {
    const EDGES: Rect = Rect::from_ltrb(12.0, 20.0, 112.0, 100.0);
    const EXTENTS: Rect = Rect::from_ltwh(12.0, 20.0, 100.0, 80.0);
    const CENTERED: Rect = Rect::from_center(Offset::new(62.0, 60.0), 100.0, 80.0);
    const ORIGIN_SIZE: Rect =
        Rect::from_origin_size(Offset::new(12.0, 20.0), Size::new(100.0, 80.0));
    let operator: Rect = Offset::new(12.0, 20.0) & Size::new(100.0, 80.0);
    for rect in [EXTENTS, CENTERED, ORIGIN_SIZE, operator] {
        assert_eq!(rect, EDGES);
    }
    assert_eq!(EDGES.width(), 100.0);
    assert_eq!(EDGES.height(), 80.0);
    assert_eq!(EDGES.size(), Size::new(100.0, 80.0));
    assert_eq!(EDGES.top_left(), Offset::new(12.0, 20.0));
    assert_eq!(EDGES.center(), Offset::new(62.0, 60.0));
    assert_eq!(EDGES.bottom_right(), Offset::new(112.0, 100.0));
    assert_eq!(Rect::default(), Rect::ZERO);
}

#[test]
fn constructors_retain_inverted_edges_and_signed_zero() {
    let expected = Rect::from_ltrb(10.0, 20.0, 2.0, 8.0);
    for rect in [
        Rect::from_ltwh(10.0, 20.0, -8.0, -12.0),
        Rect::from_center(Offset::new(6.0, 14.0), -8.0, -12.0),
        Rect::from_origin_size(Offset::new(10.0, 20.0), Size::new(-8.0, -12.0)),
        Offset::new(10.0, 20.0) & Size::new(-8.0, -12.0),
    ] {
        assert_eq!(rect, expected);
        assert!(rect.is_empty());
        assert_eq!(rect.size(), Size::new(-8.0, -12.0));
        assert_eq!(rect.center(), Offset::new(6.0, 14.0));
    }
    let zero = Rect::from_ltrb(-0.0, 0.0, 0.0, -0.0);
    assert_eq!(zero, Rect::ZERO);
    assert!(zero.left.is_sign_negative());
    assert!(zero.bottom.is_sign_negative());
    assert!(zero.is_empty());
}

#[test]
fn containment_is_half_open_in_the_rectangles_coordinate_space() {
    let rect = Rect::from_ltrb(-5.0, 3.0, 5.0, 23.0);
    for (x, y, expected) in [
        (-5.0, 3.0, true),
        (-5.0, 22.0, true),
        (4.0, 3.0, true),
        (0.0, 13.0, true),
        (5.0, 3.0, false),
        (-5.0, 23.0, false),
        (5.0, 23.0, false),
        (-6.0, 13.0, false),
        (0.0, 2.0, false),
        (f64::NAN, 13.0, false),
        (0.0, f64::NAN, false),
    ] {
        assert_eq!(rect.contains(Offset::new(x, y)), expected);
    }
}

#[test]
fn empty_and_inverted_rectangles_never_contain_or_overlap() {
    let outer = Rect::from_ltrb(-10.0, -10.0, 10.0, 10.0);
    for empty in [
        Rect::ZERO,
        Rect::from_ltwh(0.0, -2.0, 0.0, 4.0),
        Rect::from_ltwh(-2.0, 0.0, 4.0, 0.0),
        Rect::from_ltrb(2.0, -2.0, -2.0, 2.0),
        Rect::from_ltrb(-2.0, 2.0, 2.0, -2.0),
        Rect::from_ltrb(2.0, 2.0, -2.0, -2.0),
    ] {
        assert!(empty.is_empty());
        assert!(!empty.contains(Offset::ZERO));
        assert!(!empty.contains(empty.top_left()));
        assert!(!empty.overlaps(outer));
        assert!(!outer.overlaps(empty));
        assert!(!empty.overlaps(empty));
        assert_eq!(empty.intersect(outer), empty);
    }
}

#[test]
fn intersection_retains_touching_and_disjoint_bounds() {
    let rect = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
    for (other, expected, overlaps) in [
        (rect, rect, true),
        (
            Rect::from_ltrb(2.0, 3.0, 8.0, 9.0),
            Rect::from_ltrb(2.0, 3.0, 8.0, 9.0),
            true,
        ),
        (
            Rect::from_ltrb(5.0, 6.0, 15.0, 16.0),
            Rect::from_ltrb(5.0, 6.0, 10.0, 10.0),
            true,
        ),
        (
            Rect::from_ltrb(10.0, 2.0, 15.0, 8.0),
            Rect::from_ltrb(10.0, 2.0, 10.0, 8.0),
            false,
        ),
        (
            Rect::from_ltrb(2.0, 10.0, 8.0, 15.0),
            Rect::from_ltrb(2.0, 10.0, 8.0, 10.0),
            false,
        ),
        (
            Rect::from_ltrb(10.0, 10.0, 20.0, 20.0),
            Rect::from_ltrb(10.0, 10.0, 10.0, 10.0),
            false,
        ),
        (
            Rect::from_ltrb(12.0, 13.0, 20.0, 20.0),
            Rect::from_ltrb(12.0, 13.0, 10.0, 10.0),
            false,
        ),
    ] {
        assert_eq!(rect.intersect(other), expected);
        assert_eq!(other.intersect(rect), expected);
        assert_eq!(rect.overlaps(other), overlaps);
        assert_eq!(other.overlaps(rect), overlaps);
        assert_eq!(expected.is_empty(), !overlaps);
    }
}

#[test]
fn union_is_bounding_expansion_even_for_empty_or_inverted_inputs() {
    let rect = Rect::from_ltrb(10.0, 20.0, 30.0, 40.0);
    assert_eq!(rect.union(rect), rect);
    for (other, expected) in [
        (
            Rect::from_ltrb(25.0, 35.0, 50.0, 60.0),
            Rect::from_ltrb(10.0, 20.0, 50.0, 60.0),
        ),
        (
            Rect::from_ltrb(50.0, 60.0, 70.0, 80.0),
            Rect::from_ltrb(10.0, 20.0, 70.0, 80.0),
        ),
        (Rect::ZERO, Rect::from_ltrb(0.0, 0.0, 30.0, 40.0)),
        (
            Rect::from_ltrb(5.0, 6.0, 5.0, 6.0),
            Rect::from_ltrb(5.0, 6.0, 30.0, 40.0),
        ),
        // Right=5 must not become a left edge through normalization.
        (Rect::from_ltrb(50.0, 60.0, 5.0, 6.0), rect),
    ] {
        assert_eq!(rect.union(other), expected);
        assert_eq!(other.union(rect), expected);
    }
    let inverted = Rect::from_ltrb(4.0, 5.0, -4.0, -5.0);
    assert_eq!(inverted.union(inverted), inverted);
}

#[test]
fn inflation_deflation_and_translation_preserve_edge_information() {
    let rect = Rect::from_ltrb(10.0, 20.0, 30.0, 40.0);
    assert_eq!(rect.inflate(2.0), Rect::from_ltrb(8.0, 18.0, 32.0, 42.0));
    assert_eq!(rect.deflate(2.0), Rect::from_ltrb(12.0, 22.0, 28.0, 38.0));
    assert_eq!(rect.inflate(-2.0), rect.deflate(2.0));
    assert_eq!(rect.deflate(-2.0), rect.inflate(2.0));
    assert_eq!(rect.inflate(2.0).deflate(2.0), rect);
    assert_eq!(rect.inflate(0.0), rect);
    assert_eq!(rect.deflate(10.0), Rect::from_ltrb(20.0, 30.0, 20.0, 30.0));
    let inverted = rect.deflate(12.0);
    assert_eq!(inverted, Rect::from_ltrb(22.0, 32.0, 18.0, 28.0));
    assert!(inverted.is_empty());
    assert_eq!(inverted.center(), rect.center());
    for bounds in [rect, inverted, Rect::ZERO] {
        let shifted = bounds.shift(Offset::new(-5.0, 7.0));
        assert_eq!(shifted, bounds.translate(-5.0, 7.0));
        assert_eq!(
            shifted.top_left(),
            bounds.top_left() + Offset::new(-5.0, 7.0)
        );
        assert_eq!(shifted.size(), bounds.size());
        assert_eq!(shifted.translate(5.0, -7.0), bounds);
    }
}

#[test]
fn every_edge_participates_in_nonfinite_classification() {
    assert!(Rect::ZERO.is_finite());
    assert!(!Rect::ZERO.is_infinite());
    assert!(!Rect::ZERO.has_nan());
    for index in 0..4 {
        for value in [f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
            let mut edges = [0.0, 0.0, 10.0, 10.0];
            edges[index] = value;
            let rect = Rect::from_ltrb(edges[0], edges[1], edges[2], edges[3]);
            assert!(!rect.is_finite());
            assert_eq!(rect.is_infinite(), value.is_infinite());
            assert_eq!(rect.has_nan(), value.is_nan());
        }
    }
    let mixed = Rect::from_ltrb(f64::NAN, 0.0, f64::INFINITY, 10.0);
    assert!(mixed.has_nan() && mixed.is_infinite());
}

#[test]
fn nan_is_preserved_by_set_operations_and_never_overlaps() {
    let valid = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
    for index in 0..4 {
        let mut edges = [0.0, 0.0, 10.0, 10.0];
        edges[index] = f64::NAN;
        let invalid = Rect::from_ltrb(edges[0], edges[1], edges[2], edges[3]);
        assert_ne!(invalid, invalid);
        assert!(!invalid.is_empty());
        assert!(!invalid.contains(Offset::new(5.0, 5.0)));
        assert!(!invalid.overlaps(valid));
        assert!(!valid.overlaps(invalid));
        assert!(!invalid.overlaps(invalid));
        for result in [
            valid.intersect(invalid),
            invalid.intersect(valid),
            valid.union(invalid),
            invalid.union(valid),
        ] {
            for (actual, expected) in [result.left, result.top, result.right, result.bottom]
                .into_iter()
                .zip(edges)
            {
                if expected.is_nan() {
                    assert!(actual.is_nan());
                } else {
                    assert_eq!(actual, expected);
                }
            }
        }
    }
    assert!(Rect::from_ltrb(f64::NAN, 5.0, 10.0, 0.0).is_empty());
    assert!(valid.inflate(f64::NAN).has_nan());
    assert!(valid.translate(f64::NAN, 0.0).has_nan());
}

#[test]
fn infinite_bounds_follow_edge_comparisons_and_ieee_arithmetic() {
    let unbounded = Rect::from_ltrb(
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::INFINITY,
    );
    let finite = Rect::from_ltwh(10.0, 20.0, 30.0, 40.0);
    assert!(!unbounded.is_empty());
    assert!(unbounded.contains(Offset::new(f64::MAX, -f64::MAX)));
    assert!(unbounded.contains(Offset::new(f64::NEG_INFINITY, 0.0)));
    assert!(!unbounded.contains(Offset::INFINITE));
    assert!(unbounded.overlaps(finite));
    assert_eq!(unbounded.intersect(finite), finite);
    assert_eq!(unbounded.union(finite), unbounded);
    assert_eq!(unbounded.size(), Size::INFINITE);
    assert!(unbounded.center().dx.is_nan());
    assert!(unbounded.center().dy.is_nan());
    assert_eq!(
        Rect::from_center(Offset::ZERO, f64::INFINITY, f64::INFINITY),
        unbounded
    );
    assert_eq!(finite.inflate(f64::INFINITY), unbounded);
    assert!(finite.deflate(f64::INFINITY).is_empty());
    for sign in [f64::INFINITY, f64::NEG_INFINITY] {
        let collapsed = Rect::from_ltrb(sign, 0.0, sign, 10.0);
        assert!(collapsed.width().is_nan());
        assert!(collapsed.is_empty());
        assert!(!collapsed.has_nan());
        assert!(!collapsed.overlaps(unbounded));
    }
    let overflow = Rect::from_ltrb(-f64::MAX, 0.0, f64::MAX, 1.0);
    assert!(overflow.is_finite());
    assert!(!overflow.is_infinite());
    assert!(overflow.width().is_infinite());
    assert_eq!(overflow.center(), Offset::new(0.0, 0.5));
}

#[test]
fn rectangle_operations_agree_with_sampled_point_sets() {
    // Integer edges and half-integer samples ensure every positive intersection
    // has a witness. Include all edge orderings, including empty/inverted bounds.
    let mut rectangles = Vec::new();
    for left in -1..=1 {
        for top in -1..=1 {
            for right in -1..=1 {
                for bottom in -1..=1 {
                    rectangles.push(Rect::from_ltrb(
                        left as f64,
                        top as f64,
                        right as f64,
                        bottom as f64,
                    ));
                }
            }
        }
    }
    for a in &rectangles {
        for b in &rectangles {
            let intersection = a.intersect(*b);
            let union = a.union(*b);
            let mut shared_point = false;
            for x in -2..=2 {
                for y in -2..=2 {
                    let point = Offset::new(x as f64 / 2.0, y as f64 / 2.0);
                    let in_both = a.contains(point) && b.contains(point);
                    shared_point |= in_both;
                    assert_eq!(intersection.contains(point), in_both);
                    if a.contains(point) || b.contains(point) {
                        assert!(union.contains(point));
                    }
                }
            }
            assert_eq!(a.overlaps(*b), shared_point, "{a:?}, {b:?}");
            assert_eq!(intersection, b.intersect(*a));
            assert_eq!(union, b.union(*a));
        }
    }
}
