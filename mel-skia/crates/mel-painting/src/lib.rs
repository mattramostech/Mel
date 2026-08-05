//! Painting layer: Canvas, Paint, Path.
//!
//! The only crate allowed to depend on `skia-safe`. Skia types never leak
//! upward — every layer above sees Mel's own wrappers only.
//!
//! For now (feature 003) the whole API is [`fill`]: a bootstrap that proves
//! Skia rasterizes into a caller-provided buffer. Mel's real `Canvas`/`Paint`
//! wrappers arrive in Phase 2 (features 020+) and replace it.

use skia_safe::{AlphaType, Color4f, ColorType, ImageInfo, surfaces};

/// Fills `pixels` with one solid color through a Skia canvas.
///
/// The buffer contract matches what the platform layer presents (`mel-platform`
/// documents the same contract on its `Frame`): 32-bit **BGRA**, premultiplied
/// alpha, tightly packed rows (`stride == width * 4`). `color` is
/// `[r, g, b, a]` with each channel in `0.0..=1.0`.
///
/// # Panics
///
/// Panics if `pixels.len() != width * height * 4` or if either dimension is
/// zero — both are programmer errors, not runtime conditions.
pub fn fill(pixels: &mut [u8], width: u32, height: u32, color: [f32; 4]) {
    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None, // untagged color space; color management is deferred (feature 013)
    );
    let mut surface = surfaces::wrap_pixels(&info, pixels, None, None)
        .expect("pixels must be a non-empty width * height * 4 BGRA buffer");
    let [r, g, b, a] = color;
    surface.canvas().clear(Color4f::new(r, g, b, a));
}

#[cfg(test)]
mod tests {
    use super::fill;

    /// A non-square size catches width/height swaps; the byte pattern pins
    /// down the BGRA order of the contract.
    #[test]
    fn fills_every_pixel_in_bgra_order() {
        let (width, height) = (2u32, 3u32);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill(&mut pixels, width, height, [1.0, 0.0, 0.0, 1.0]); // opaque red
        for pixel in pixels.chunks_exact(4) {
            assert_eq!(pixel, [0x00, 0x00, 0xFF, 0xFF]); // B, G, R, A
        }
    }

    /// The buffer is premultiplied: fully transparent means all-zero bytes,
    /// whatever the color channels said.
    #[test]
    fn premultiplies_alpha() {
        let mut pixels = vec![0xAAu8; 16];
        fill(&mut pixels, 2, 2, [1.0, 1.0, 1.0, 0.0]); // transparent white
        assert_eq!(pixels, [0u8; 16]);
    }

    #[test]
    #[should_panic(expected = "width * height * 4")]
    fn rejects_wrong_buffer_size() {
        let mut pixels = vec![0u8; 7];
        fill(&mut pixels, 2, 2, [0.0, 0.0, 0.0, 1.0]);
    }
}
