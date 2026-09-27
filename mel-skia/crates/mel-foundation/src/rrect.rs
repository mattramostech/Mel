use crate::{Offset, Rect};

/// Horizontal and vertical radii of an ellipse, in caller-defined units.
///
/// Constructors preserve all `f64` values. Rounded-rectangle queries require
/// finite, nonnegative components. Either zero component makes a square corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Radius {
    pub x: f64,
    pub y: f64,
}

impl Radius {
    pub const ZERO: Self = Self::circular(0.0);

    pub const fn circular(radius: f64) -> Self {
        Self::elliptical(radius, radius)
    }

    pub const fn elliptical(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn is_valid(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.x >= 0.0 && self.y >= 0.0
    }

    fn scaled(self, factor: f64) -> Self {
        Self::elliptical(self.x * factor, self.y * factor)
    }
}

/// Axis-aligned bounds with one elliptical radius per corner.
///
/// Inputs are retained without normalization. Queries fit oversized radii
/// proportionally, without changing the original value. Invalid geometry
/// (negative/non-finite radii, inverted/non-finite bounds or overflowing
/// extents) has no fitted/inner bounds and contains no points.
///
/// ```
/// use mel_foundation::{Offset, Radius, Rect, RRect};
/// let rounded = RRect::from_rect_radius(
///     Rect::from_ltwh(10.0, 20.0, 100.0, 80.0),
///     Radius::circular(20.0),
/// );
/// assert!(rounded.contains(Offset::new(60.0, 60.0)));
/// assert!(!rounded.contains(rounded.outer_rect().top_left()));
/// assert!(rounded.safe_inner_rect().unwrap().contains(Offset::new(60.0, 60.0)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RRect {
    pub bounds: Rect,
    pub top_left: Radius,
    pub top_right: Radius,
    pub bottom_right: Radius,
    pub bottom_left: Radius,
}

impl RRect {
    pub const ZERO: Self = Self::from_rect_radius(Rect::ZERO, Radius::ZERO);

    pub const fn from_rect_radius(bounds: Rect, radius: Radius) -> Self {
        Self::from_rect_corners(bounds, radius, radius, radius, radius)
    }

    /// Corners are specified clockwise, starting at the top-left.
    pub const fn from_rect_corners(
        bounds: Rect,
        top_left: Radius,
        top_right: Radius,
        bottom_right: Radius,
        bottom_left: Radius,
    ) -> Self {
        Self {
            bounds,
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// The original rectangle, including empty, inverted or non-finite edges.
    pub const fn outer_rect(self) -> Rect {
        self.bounds
    }

    /// Fit adjacent radii to all four sides using a single factor at most one.
    ///
    /// Returns `None` for invalid geometry (see [`RRect`]). Zero extents are
    /// valid; a positive radius pair along a collapsed side scales all radii
    /// to zero. Arithmetic follows `f64` rounding/underflow semantics.
    pub fn scale_radii(self) -> Option<Self> {
        let width = self.bounds.width();
        let height = self.bounds.height();
        if !self.bounds.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || width < 0.0
            || height < 0.0
            || !self.top_left.is_valid()
            || !self.top_right.is_valid()
            || !self.bottom_right.is_valid()
            || !self.bottom_left.is_valid()
        {
            return None;
        }

        let mut factor: f64 = 1.0;
        for (first, second, limit) in [
            (self.top_left.x, self.top_right.x, width),
            (self.top_right.y, self.bottom_right.y, height),
            (self.bottom_right.x, self.bottom_left.x, width),
            (self.bottom_left.y, self.top_left.y, height),
        ] {
            let sum = first + second;
            if sum > limit {
                let ratio = if sum.is_finite() {
                    limit / sum
                } else {
                    // Finite radii can overflow when added. Divide relative to
                    // the larger component so the factor remains meaningful.
                    let larger = first.max(second);
                    (limit / larger) / (1.0 + first.min(second) / larger)
                };
                factor = factor.min(ratio);
            }
        }
        Some(Self::from_rect_corners(
            self.bounds,
            self.top_left.scaled(factor),
            self.top_right.scaled(factor),
            self.bottom_right.scaled(factor),
            self.bottom_left.scaled(factor),
        ))
    }

    /// Test fitted corner ellipses within half-open outer bounds.
    ///
    /// Arc boundaries are included; the outer right/bottom edges are excluded.
    /// Empty/invalid shapes and non-finite points never contain a point.
    pub fn contains(self, point: Offset) -> bool {
        if !self.bounds.contains(point) {
            return false;
        }
        let Some(fitted) = self.scale_radii() else {
            return false;
        };
        let left = point.dx - self.bounds.left;
        let top = point.dy - self.bounds.top;
        let right = self.bounds.right - point.dx;
        let bottom = self.bounds.bottom - point.dy;
        // Diagonally opposite corner regions can overlap even after fitting.
        // Every applicable ellipse must contain the point, not just the first.
        for (x, y, radius) in [
            (left, top, fitted.top_left),
            (right, top, fitted.top_right),
            (right, bottom, fitted.bottom_right),
            (left, bottom, fitted.bottom_left),
        ] {
            if x < radius.x && y < radius.y {
                let dx = (x - radius.x) / radius.x;
                let dy = (y - radius.y) / radius.y;
                if dx * dx + dy * dy > 1.0 {
                    return false;
                }
            }
        }
        true
    }

    /// Conservative inner bounds derived from the fitted corner ellipses.
    ///
    /// Each side is inset by its largest adjacent radius component times
    /// `1 - 1/sqrt(2)`. This is not a maximum-area rectangle search. Returns
    /// `None` for invalid geometry; empty bounds yield an empty rectangle.
    pub fn safe_inner_rect(self) -> Option<Rect> {
        let fitted = self.scale_radii()?;
        let inset = 1.0 - std::f64::consts::FRAC_1_SQRT_2;
        Some(Rect::from_ltrb(
            self.bounds.left + fitted.top_left.x.max(fitted.bottom_left.x) * inset,
            self.bounds.top + fitted.top_left.y.max(fitted.top_right.y) * inset,
            self.bounds.right - fitted.top_right.x.max(fitted.bottom_right.x) * inset,
            self.bounds.bottom - fitted.bottom_left.y.max(fitted.bottom_right.y) * inset,
        ))
    }
}
