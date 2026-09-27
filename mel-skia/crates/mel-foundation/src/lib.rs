//! Foundation layer: geometry, color, keys, diagnostics.
//!
//! The bottom of the stack — depends on nothing, not even Skia.

#![forbid(unsafe_code)]

mod geometry;
mod rect;
mod rrect;

pub use geometry::{Offset, Size};
pub use rect::Rect;
pub use rrect::{RRect, Radius};
