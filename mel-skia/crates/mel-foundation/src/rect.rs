use std::ops::BitAnd;

use crate::{Offset, Size};

/// Axis-aligned bounds in a caller-defined coordinate space.
///
/// Edges accept every `f64` and are never normalized: inverted bounds are empty
/// but retain their coordinates. Containment includes top/left and excludes
/// bottom/right. NaN bounds never contain a point or overlap another rectangle.
/// Equality is exact, as for [`Offset`] and [`Size`].
///
/// ```
/// use mel_foundation::{Offset, Rect, Size};
/// let bounds = Offset::new(12.0, 20.0) & Size::new(100.0, 80.0);
/// assert_eq!(bounds, Rect::from_ltwh(12.0, 20.0, 100.0, 80.0));
/// assert!(bounds.contains(bounds.center()));
/// assert!(!bounds.contains(bounds.bottom_right()));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub const ZERO: Self = Self::from_ltrb(0.0, 0.0, 0.0, 0.0);

    pub const fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn from_ltwh(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self::from_ltrb(left, top, left + width, top + height)
    }

    pub const fn from_center(center: Offset, width: f64, height: f64) -> Self {
        Self::from_ltrb(
            center.dx - width / 2.0,
            center.dy - height / 2.0,
            center.dx + width / 2.0,
            center.dy + height / 2.0,
        )
    }

    /// Named equivalent of `origin & size`.
    pub const fn from_origin_size(origin: Offset, size: Size) -> Self {
        Self::from_ltwh(origin.dx, origin.dy, size.width, size.height)
    }

    pub fn width(self) -> f64 {
        self.right - self.left
    }

    pub fn height(self) -> f64 {
        self.bottom - self.top
    }

    pub fn size(self) -> Size {
        Size::new(self.width(), self.height())
    }

    pub fn top_left(self) -> Offset {
        Offset::new(self.left, self.top)
    }

    /// Midpoint of opposing edges; non-finite arithmetic may yield NaN.
    pub fn center(self) -> Offset {
        Offset::new(
            (self.left + self.right) / 2.0,
            (self.top + self.bottom) / 2.0,
        )
    }

    pub fn bottom_right(self) -> Offset {
        Offset::new(self.right, self.bottom)
    }

    /// True when all four edges are finite, regardless of derived extents.
    pub fn is_finite(self) -> bool {
        self.left.is_finite()
            && self.top.is_finite()
            && self.right.is_finite()
            && self.bottom.is_finite()
    }

    /// True for an edge of either infinity sign, as for [`Offset`].
    pub fn is_infinite(self) -> bool {
        self.left.is_infinite()
            || self.top.is_infinite()
            || self.right.is_infinite()
            || self.bottom.is_infinite()
    }

    pub fn has_nan(self) -> bool {
        self.left.is_nan() || self.top.is_nan() || self.right.is_nan() || self.bottom.is_nan()
    }

    /// True for zero or inverted extents. NaN alone does not imply emptiness.
    pub fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Includes top/left; excludes bottom/right. Empty or NaN bounds and NaN
    /// points never satisfy all four comparisons.
    pub fn contains(self, point: Offset) -> bool {
        point.dx >= self.left
            && point.dx < self.right
            && point.dy >= self.top
            && point.dy < self.bottom
    }

    pub fn shift(self, offset: Offset) -> Self {
        self.translate(offset.dx, offset.dy)
    }

    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self::from_ltrb(
            self.left + dx,
            self.top + dy,
            self.right + dx,
            self.bottom + dy,
        )
    }

    /// Move each edge outwards by `delta`; negative values shrink the bounds.
    pub fn inflate(self, delta: f64) -> Self {
        Self::from_ltrb(
            self.left - delta,
            self.top - delta,
            self.right + delta,
            self.bottom + delta,
        )
    }

    /// Move each edge inwards, retaining inverted bounds if edges cross.
    pub fn deflate(self, delta: f64) -> Self {
        self.inflate(-delta)
    }

    /// Inner edge bounds; disjoint inputs retain zero or negative extents.
    /// NaN propagates on each affected edge.
    pub fn intersect(self, other: Self) -> Self {
        Self::from_ltrb(
            max(self.left, other.left),
            max(self.top, other.top),
            min(self.right, other.right),
            min(self.bottom, other.bottom),
        )
    }

    /// Bounding expansion, corresponding to Flutter's `expandToInclude`.
    /// Empty operands still contribute their edges: [`Rect::ZERO`] is not an
    /// identity. Inverted operands are not normalized. NaN propagates per edge.
    pub fn union(self, other: Self) -> Self {
        Self::from_ltrb(
            min(self.left, other.left),
            min(self.top, other.top),
            max(self.right, other.right),
            max(self.bottom, other.bottom),
        )
    }

    /// True only for positive-area intersection. Unlike Flutter's degenerate
    /// cases, empty/inverted or NaN inputs never overlap; touching also fails.
    pub fn overlaps(self, other: Self) -> bool {
        let intersection = self.intersect(other);
        intersection.left < intersection.right && intersection.top < intersection.bottom
    }
}

impl BitAnd<Size> for Offset {
    type Output = Rect;

    fn bitand(self, rhs: Size) -> Rect {
        Rect::from_origin_size(self, rhs)
    }
}

// Rust's min/max discard a lone NaN. Preserve invalid edges instead of silently
// turning them into usable bounds, following the numeric policy of ADR 010.
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}
