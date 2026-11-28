//! The pie menu's entries.

use super::color::WedgeColor;

/// One pie-menu entry: label plus its palette slot.
pub(crate) const ITEMS: &[(&str, WedgeColor)] = &[
    ("Cut", WedgeColor::Cut),
    ("Copy", WedgeColor::Copy),
    ("Paste", WedgeColor::Paste),
    ("Undo", WedgeColor::Undo),
    ("Rotate", WedgeColor::Rotate),
    ("Zoom", WedgeColor::Zoom),
    ("Pin", WedgeColor::Pin),
    ("Info", WedgeColor::Info),
];
