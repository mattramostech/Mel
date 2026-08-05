//! Platform layer: iOS app shell, display link, raster surface, touch input.
//!
//! UIKit (or winit — decided in feature 002) types never leak upward; layers
//! above see Mel's own pointer/keyboard event types only.
