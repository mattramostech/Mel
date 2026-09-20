//! Painting layer: Canvas, Paint, Path.
//!
//! The only crate allowed to depend on `skia-safe`. Skia types never leak
//! upward — every layer above sees Mel's own wrappers only.
//!
//! For now the whole API is two bootstraps that prove Skia rasterizes into a
//! caller-provided buffer: [`fill`] (feature 003) and [`fill_rect`] (feature
//! 004). Mel's real `Canvas`/`Paint` wrappers arrive in Phase 2 (features
//! 020+) and replace both.

use skia_safe::{AlphaType, Color4f, ColorType, ImageInfo, Paint, Rect, surfaces};

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

/// Fills the axis-aligned rectangle `rect = [x, y, w, h]` (buffer pixels)
/// with one solid color through a Skia canvas, leaving every other pixel
/// untouched.
///
/// Same buffer contract as [`fill`]. The rectangle is clipped to the buffer;
/// an empty intersection (zero-sized or fully outside) is a no-op.
/// Anti-aliasing is off on purpose: edges land exactly on the pixels the
/// caller named, so a screenshot can measure them (feature 004's "Done when").
///
/// # Panics
///
/// Panics if `pixels.len() != width * height * 4` or if either dimension is
/// zero — both are programmer errors, not runtime conditions.
pub fn fill_rect(pixels: &mut [u8], width: u32, height: u32, rect: [f32; 4], color: [f32; 4]) {
    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None, // untagged color space; color management is deferred (feature 013)
    );
    let mut surface = surfaces::wrap_pixels(&info, pixels, None, None)
        .expect("pixels must be a non-empty width * height * 4 BGRA buffer");
    let [r, g, b, a] = color;
    let paint = Paint::new(Color4f::new(r, g, b, a), None); // defaults: fill style, no AA
    let [x, y, w, h] = rect;
    surface
        .canvas()
        .draw_rect(Rect::from_xywh(x, y, w, h), &paint);
}

#[cfg(test)]
mod tests {
    use super::{fill, fill_rect};

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

    /// The rect covers pixels [1..3)×[1..3) of a 4×4 buffer: the four inside
    /// pixels take the color, the twelve outside stay untouched (no AA bleed).
    #[test]
    fn fill_rect_touches_only_the_rect() {
        let (width, height) = (4u32, 4u32);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_rect(
            &mut pixels,
            width,
            height,
            [1.0, 1.0, 2.0, 2.0],
            [0.0, 1.0, 0.0, 1.0],
        );
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                let expected: [u8; 4] = if (1..3).contains(&x) && (1..3).contains(&y) {
                    [0x00, 0xFF, 0x00, 0xFF] // opaque green, in B G R A order
                } else {
                    [0, 0, 0, 0]
                };
                assert_eq!(pixels[i..i + 4], expected, "pixel ({x},{y})");
            }
        }
    }

    /// A rect hanging past every edge paints exactly the intersection with
    /// the buffer — here the x = 1 column of a 2×2 buffer.
    #[test]
    fn fill_rect_clips_to_the_buffer() {
        let (width, height) = (2u32, 2u32);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_rect(
            &mut pixels,
            width,
            height,
            [1.0, -5.0, 10.0, 20.0],
            [1.0, 0.0, 0.0, 1.0],
        );
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                let expected: [u8; 4] = if x == 1 {
                    [0x00, 0x00, 0xFF, 0xFF]
                } else {
                    [0; 4]
                };
                assert_eq!(pixels[i..i + 4], expected, "pixel ({x},{y})");
            }
        }
    }

    #[test]
    fn fill_rect_empty_rect_is_a_noop() {
        let mut pixels = vec![0xAAu8; 16];
        fill_rect(
            &mut pixels,
            2,
            2,
            [1.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(pixels, [0xAAu8; 16]);
    }
}
