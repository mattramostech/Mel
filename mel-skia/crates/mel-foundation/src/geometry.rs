use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

/// A point or displacement in a caller-defined coordinate space.
///
/// Components accept all `f64` values. Arithmetic follows IEEE-754; equality
/// is exact and NaN is unequal to itself. `%` uses nonnegative Euclidean modulo
/// (as in Flutter), unlike scalar Rust `%`. Component comparisons use the
/// `all_*` methods rather than an ordering trait.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offset {
    pub dx: f64,
    pub dy: f64,
}

impl Offset {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const INFINITE: Self = Self::new(f64::INFINITY, f64::INFINITY);

    pub const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }

    /// Radians clockwise from +x when +y points down. Distance is explicit.
    pub fn from_direction(direction: f64, distance: f64) -> Self {
        Self::new(direction.cos() * distance, direction.sin() * distance)
    }

    pub fn distance_squared(self) -> f64 {
        self.dx * self.dx + self.dy * self.dy
    }

    /// Square root of the squared distance; intermediate squares may overflow.
    pub fn distance(self) -> f64 {
        self.distance_squared().sqrt()
    }

    /// Radians clockwise from +x when +y points down, in [-π, π].
    pub fn direction(self) -> f64 {
        self.dy.atan2(self.dx)
    }

    pub fn scale(self, scale_x: f64, scale_y: f64) -> Self {
        Self::new(self.dx * scale_x, self.dy * scale_y)
    }

    pub fn translate(self, dx: f64, dy: f64) -> Self {
        self + Self::new(dx, dy)
    }

    /// True only when both components are neither infinity nor NaN.
    pub fn is_finite(self) -> bool {
        self.dx.is_finite() && self.dy.is_finite()
    }

    /// True for either sign of infinity, unlike Flutter's positive-only check.
    pub fn is_infinite(self) -> bool {
        self.dx.is_infinite() || self.dy.is_infinite()
    }

    /// True when both components are strictly less than the corresponding ones.
    pub fn all_lt(self, other: Self) -> bool {
        self.dx < other.dx && self.dy < other.dy
    }

    /// True when both components are less than or equal to the corresponding ones.
    pub fn all_le(self, other: Self) -> bool {
        self.dx <= other.dx && self.dy <= other.dy
    }

    /// True when both components are strictly greater than the corresponding ones.
    pub fn all_gt(self, other: Self) -> bool {
        self.dx > other.dx && self.dy > other.dy
    }

    /// True when both components are greater than or equal to the corresponding ones.
    pub fn all_ge(self, other: Self) -> bool {
        self.dx >= other.dx && self.dy >= other.dy
    }

    /// Divide and round toward zero, retaining floating-point infinities/NaN.
    pub fn div_trunc(self, divisor: f64) -> Self {
        Self::new((self.dx / divisor).trunc(), (self.dy / divisor).trunc())
    }

    /// Component interpolation; `t` is not clamped, so extrapolation is allowed.
    /// Equal components are preserved; otherwise non-finite values follow IEEE-754.
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self::new(lerp(self.dx, other.dx, t), lerp(self.dy, other.dy, t))
    }
}

/// Two extents in a caller-defined coordinate space, without normalization.
///
/// Negative extents, infinities and NaN are accepted; this is not a validated
/// layout size. Numeric policy and Euclidean `%` match [`Offset`]. Subtracting
/// another Size returns an Offset; adding/subtracting an Offset returns a Size.
///
/// ```
/// use mel_foundation::{Offset, Size};
/// let size = Size::new(100.0, 80.0);
/// let origin = Offset::new(12.0, 20.0);
/// assert_eq!(size.center(origin), Offset::new(62.0, 60.0));
/// assert!(size.contains(size.center(origin) - origin));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const INFINITE: Self = Self::new(f64::INFINITY, f64::INFINITY);

    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub const fn square(dimension: f64) -> Self {
        Self::new(dimension, dimension)
    }

    pub const fn from_radius(radius: f64) -> Self {
        Self::square(radius * 2.0)
    }

    /// Fix the width; leave height unbounded.
    pub const fn from_width(width: f64) -> Self {
        Self::new(width, f64::INFINITY)
    }

    /// Fix the height; leave width unbounded.
    pub const fn from_height(height: f64) -> Self {
        Self::new(f64::INFINITY, height)
    }

    pub fn is_finite(self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }

    /// True for either sign of infinity, unlike Flutter's positive-only check.
    pub fn is_infinite(self) -> bool {
        self.width.is_infinite() || self.height.is_infinite()
    }

    /// Either extent is nonpositive. NaN alone does not imply emptiness.
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Width / height, with 0×0 mapping to 0. For either signed zero height,
    /// nonzero width gives infinity with width's sign. NaN always propagates.
    pub fn aspect_ratio(self) -> f64 {
        if self.width.is_nan() || self.height.is_nan() {
            f64::NAN
        } else if self.height != 0.0 {
            self.width / self.height
        } else if self.width == 0.0 {
            0.0
        } else {
            f64::INFINITY.copysign(self.width)
        }
    }

    pub const fn flipped(self) -> Self {
        Self::new(self.height, self.width)
    }

    /// Lesser extent magnitude; NaN propagates.
    pub fn shortest_side(self) -> f64 {
        // f64::min/max ignore a lone NaN, concealing invalid geometry.
        if self.width.is_nan() || self.height.is_nan() {
            f64::NAN
        } else {
            self.width.abs().min(self.height.abs())
        }
    }

    /// Greater extent magnitude; NaN propagates.
    pub fn longest_side(self) -> f64 {
        if self.width.is_nan() || self.height.is_nan() {
            f64::NAN
        } else {
            self.width.abs().max(self.height.abs())
        }
    }

    /// Local containment: includes top/left, excludes bottom/right.
    /// Empty sizes and NaN bounds/coordinates never contain a point.
    pub fn contains(self, point: Offset) -> bool {
        point.dx >= 0.0 && point.dx < self.width && point.dy >= 0.0 && point.dy < self.height
    }

    /// Top-left anchor relative to `origin`, the top-left of the size.
    pub fn top_left(self, origin: Offset) -> Offset {
        origin
    }

    /// Top-center anchor relative to the top-left `origin`.
    pub fn top_center(self, origin: Offset) -> Offset {
        Offset::new(origin.dx + self.width / 2.0, origin.dy)
    }

    /// Top-right anchor relative to the top-left `origin`.
    pub fn top_right(self, origin: Offset) -> Offset {
        Offset::new(origin.dx + self.width, origin.dy)
    }

    /// Center-left anchor relative to the top-left `origin`.
    pub fn center_left(self, origin: Offset) -> Offset {
        Offset::new(origin.dx, origin.dy + self.height / 2.0)
    }

    /// Center anchor relative to the top-left `origin`.
    pub fn center(self, origin: Offset) -> Offset {
        origin + Offset::new(self.width / 2.0, self.height / 2.0)
    }

    /// Center-right anchor relative to the top-left `origin`.
    pub fn center_right(self, origin: Offset) -> Offset {
        origin + Offset::new(self.width, self.height / 2.0)
    }

    /// Bottom-left anchor relative to the top-left `origin`.
    pub fn bottom_left(self, origin: Offset) -> Offset {
        Offset::new(origin.dx, origin.dy + self.height)
    }

    /// Bottom-center anchor relative to the top-left `origin`.
    pub fn bottom_center(self, origin: Offset) -> Offset {
        origin + Offset::new(self.width / 2.0, self.height)
    }

    /// Bottom-right anchor relative to the top-left `origin`.
    pub fn bottom_right(self, origin: Offset) -> Offset {
        origin + Offset::new(self.width, self.height)
    }

    /// Both extents must be strictly less than their counterparts.
    pub fn all_lt(self, other: Self) -> bool {
        self.width < other.width && self.height < other.height
    }

    /// Both extents must be less than or equal to their counterparts.
    pub fn all_le(self, other: Self) -> bool {
        self.width <= other.width && self.height <= other.height
    }

    /// Both extents must be strictly greater than their counterparts.
    pub fn all_gt(self, other: Self) -> bool {
        self.width > other.width && self.height > other.height
    }

    /// Both extents must be greater than or equal to their counterparts.
    pub fn all_ge(self, other: Self) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    /// Divide and round toward zero, retaining floating-point infinities/NaN.
    pub fn div_trunc(self, divisor: f64) -> Self {
        Self::new(
            (self.width / divisor).trunc(),
            (self.height / divisor).trunc(),
        )
    }

    /// Component interpolation; extrapolation and negative extents are allowed.
    /// Equal components are preserved; otherwise non-finite values follow IEEE-754.
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self::new(
            lerp(self.width, other.width, t),
            lerp(self.height, other.height, t),
        )
    }
}

impl Add for Offset {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.dx + rhs.dx, self.dy + rhs.dy)
    }
}

impl Sub for Offset {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.dx - rhs.dx, self.dy - rhs.dy)
    }
}

impl Neg for Offset {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.dx, -self.dy)
    }
}

impl Mul<f64> for Offset {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.dx * rhs, self.dy * rhs)
    }
}

impl Div<f64> for Offset {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.dx / rhs, self.dy / rhs)
    }
}

impl Rem<f64> for Offset {
    type Output = Self;
    fn rem(self, rhs: f64) -> Self {
        Self::new(modulo(self.dx, rhs), modulo(self.dy, rhs))
    }
}

impl Add<Offset> for Size {
    type Output = Self;
    fn add(self, rhs: Offset) -> Self {
        Self::new(self.width + rhs.dx, self.height + rhs.dy)
    }
}

impl Sub<Offset> for Size {
    type Output = Self;
    fn sub(self, rhs: Offset) -> Self {
        Self::new(self.width - rhs.dx, self.height - rhs.dy)
    }
}

impl Sub for Size {
    type Output = Offset;
    fn sub(self, rhs: Self) -> Offset {
        Offset::new(self.width - rhs.width, self.height - rhs.height)
    }
}

impl Mul<f64> for Size {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.width * rhs, self.height * rhs)
    }
}

impl Div<f64> for Size {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.width / rhs, self.height / rhs)
    }
}

impl Rem<f64> for Size {
    type Output = Self;
    fn rem(self, rhs: f64) -> Self {
        Self::new(modulo(self.width, rhs), modulo(self.height, rhs))
    }
}

fn modulo(value: f64, divisor: f64) -> f64 {
    let remainder = value.rem_euclid(divisor);
    // Dart's modulo also normalizes negative zero.
    if remainder == 0.0 { 0.0 } else { remainder }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    // Preserve equal infinities instead of producing infinity * 0 = NaN.
    if a == b { a } else { a * (1.0 - t) + b * t }
}
