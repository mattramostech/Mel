//! Painting layer: Canvas, Paint, Path.
//!
//! The only crate allowed to depend on `skia-safe`. Skia types never leak
//! upward — every layer above sees Mel's own wrappers only.
