use crate::domain::SatelliteId;
use crate::domain::SatelliteKind;
use gtk4::Widget;
use smearor_commons_geometry::Rotation;

/// Configuration of one satellite in `CardSatellites::satellites`.
///
/// Widget-layer type: a `Custom` satellite carries its consumer-provided
/// widget, serialized like `CardConfig::folded_content` - skipped.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SatelliteConfig {
    /// Stable identifier - keys the resolver's placements back to this
    /// satellite.
    pub id: SatelliteId,
    /// What the satellite renders.
    pub kind: SatelliteKind,
    /// Fixed anchor angle honored by `SatelliteDistribution::Anchored`
    /// (a `QuarterTurn` anchor is `Rotation::QuarterTurn(..)`); `None`
    /// = flexible satellite distributed by the layout.
    #[cfg_attr(feature = "serde", serde(default))]
    pub anchor: Option<Rotation>,
    /// Consumer-provided widget - only meaningful for
    /// `SatelliteKind::Custom`; `None` there skips the satellite with a
    /// warning. Not serialized (like `folded_content`).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub widget: Option<Widget>,
}

impl SatelliteConfig {
    /// A chrome-element satellite (board-built widget, e.g. `Minimize`).
    pub fn chrome(id: impl Into<SatelliteId>, element: crate::domain::ChromeElement) -> Self {
        Self {
            id: id.into(),
            kind: SatelliteKind::Chrome(element),
            anchor: None,
            widget: None,
        }
    }

    /// A custom satellite around a consumer-provided `widget`.
    pub fn custom(id: impl Into<SatelliteId>, widget: impl Into<Widget>) -> Self {
        Self {
            id: id.into(),
            kind: SatelliteKind::Custom,
            anchor: None,
            widget: Some(widget.into()),
        }
    }

    /// Pins the satellite to `anchor` on the orbit - honored by
    /// `SatelliteDistribution::Anchored` (a `QuarterTurn` anchor is
    /// `Rotation::QuarterTurn(..)`); ignored by `Even`.
    pub fn with_anchor(mut self, anchor: Rotation) -> Self {
        self.anchor = Some(anchor);
        self
    }
}
