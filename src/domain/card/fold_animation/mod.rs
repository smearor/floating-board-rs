//! Fold transition presets (`CardConfig::fold_animation`): each
//! variant is a program over the card's animatable axes - width,
//! height, rotation, zoom and opacity - evaluated between a start
//! state and an end state. Axes whose start and end values agree can
//! still deviate mid-transition through their sinusoidal excursion.

mod card_fold_animation;
mod card_fold_timing;
mod fold_axis;
mod fold_pose;

pub use card_fold_animation::CardFoldAnimation;
pub use card_fold_timing::CardFoldTiming;
pub use fold_axis::FoldAxis;
pub use fold_pose::FoldPose;
