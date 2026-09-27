/// An 8-bit-per-channel sRGB color stored as `0xAARRGGBB`.
///
/// RGB channels are straight (not premultiplied by alpha); all bits, including
/// RGB at zero alpha, participate in equality and hashing. Packed channel order
/// is independent of host byte order and does not describe a raster buffer.
/// The default is transparent black. Wide gamut and color-space conversion are
/// not supported. Floating operations quantize their results back to bytes.
///
/// ```
/// use mel_foundation::Color;
/// let honey = Color::from_argb32(0xffff_c107);
/// let faded = honey.with_opacity(0.5).unwrap();
/// assert_eq!(faded.to_argb32(), 0x80ff_c107);
/// assert_eq!(faded.opacity(), 128.0 / 255.0);
/// assert_eq!(Color::lerp(Color::RED, Color::BLUE, 0.5),
///            Some(Color::from_argb(255, 128, 0, 128)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color(u32);

impl Color {
    // Channel-extrema palette, not Material swatches. GREEN is full intensity.
    pub const TRANSPARENT: Self = Self::from_argb32(0x0000_0000);
    pub const BLACK: Self = Self::from_argb32(0xff00_0000);
    pub const WHITE: Self = Self::from_argb32(0xffff_ffff);
    pub const RED: Self = Self::from_argb32(0xffff_0000);
    pub const GREEN: Self = Self::from_argb32(0xff00_ff00);
    pub const BLUE: Self = Self::from_argb32(0xff00_00ff);
    pub const YELLOW: Self = Self::from_argb32(0xffff_ff00);
    pub const CYAN: Self = Self::from_argb32(0xff00_ffff);
    pub const MAGENTA: Self = Self::from_argb32(0xffff_00ff);

    /// Preserve every bit of `0xAARRGGBB`; six-digit RGB values have zero alpha.
    pub const fn from_argb32(value: u32) -> Self {
        Self(value)
    }

    pub const fn from_argb(alpha: u8, red: u8, green: u8, blue: u8) -> Self {
        Self((alpha as u32) << 24 | (red as u32) << 16 | (green as u32) << 8 | blue as u32)
    }

    /// Construct from RGB bytes and checked opacity; see [`Self::with_opacity`].
    pub fn from_rgbo(red: u8, green: u8, blue: u8, opacity: f64) -> Option<Self> {
        Self::from_argb(0, red, green, blue).with_opacity(opacity)
    }

    pub const fn to_argb32(self) -> u32 {
        self.0
    }

    pub const fn alpha(self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub const fn red(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub const fn green(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub const fn blue(self) -> u8 {
        self.0 as u8
    }

    /// Quantized alpha divided by 255, in `[0, 1]`.
    pub fn opacity(self) -> f64 {
        f64::from(self.alpha()) / 255.0
    }

    /// Replace alpha, preserving RGB even when `alpha` is zero.
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self::from_argb(alpha, self.red(), self.green(), self.blue())
    }

    /// Replace (not multiply) alpha with `round(opacity * 255)`.
    ///
    /// Half-byte ties round upward. Returns `None` for values outside `[0, 1]`,
    /// infinities or NaN; signed zero is accepted. Reading opacity back may
    /// differ from the input because storage has only 256 alpha levels.
    pub fn with_opacity(self, opacity: f64) -> Option<Self> {
        if !(0.0..=1.0).contains(&opacity) {
            return None;
        }
        Some(self.with_alpha((opacity * 255.0).round() as u8))
    }

    /// Interpolate straight sRGB bytes and alpha independently.
    ///
    /// Each channel evaluates `a + (b - a) * t`, clamps to `[0, 255]`, then
    /// rounds to nearest (half-byte ties upward). Finite factors outside
    /// `[0, 1]` extrapolate with saturation, even if an intermediate overflows.
    /// Non-finite `t` returns `None`, including for equal endpoints.
    ///
    /// This is neither linear-light nor premultiplied-alpha interpolation.
    /// To fade without changing RGB, use `a.with_alpha(0)` as an endpoint;
    /// [`Self::TRANSPARENT`] is specifically transparent black.
    pub fn lerp(a: Self, b: Self, t: f64) -> Option<Self> {
        if !t.is_finite() {
            return None;
        }
        Some(Self::from_argb(
            lerp_channel(a.alpha(), b.alpha(), t),
            lerp_channel(a.red(), b.red(), t),
            lerp_channel(a.green(), b.green(), t),
            lerp_channel(a.blue(), b.blue(), t),
        ))
    }
}

fn lerp_channel(a: u8, b: u8, t: f64) -> u8 {
    let a = f64::from(a);
    let b = f64::from(b);
    // Subtract in floating point to allow descending channels. Finite t keeps
    // equal channels unchanged; overflow for unequal channels saturates below.
    (a + (b - a) * t).clamp(0.0, 255.0).round() as u8
}
