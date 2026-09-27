//! Foundation layer: geometry, color, keys, diagnostics.
//!
//! The bottom of the stack — depends on nothing, not even Skia.

#![forbid(unsafe_code)]

mod geometry;

pub use geometry::{Offset, Size};
