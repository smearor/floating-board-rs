//! Shared geometry primitives: axis-aligned rectangles (including rotated
//! bounds, scanline rasterization and bounds clamping) and rotation storage.

mod rect;
mod rotation;

pub use rect::Rect;
pub use rotation::RotationRad;
