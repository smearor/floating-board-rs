use crate::domain::ChromeElement;

/// What a satellite renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum SatelliteKind {
    /// A modularized chrome element rendered by the board, e.g.
    /// `Minimize` or `Close` - the widget comes from the chrome
    /// builders, the wiring from the card.
    Chrome(ChromeElement),
    /// A consumer-provided widget (color pickers, duplicate buttons,
    /// ...). The consumer wires its own signals; the board owns
    /// placement and lifetime only.
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let kind = SatelliteKind::Chrome(ChromeElement::Minimize);
        let json = serde_json::to_string(&kind).expect("serialize");
        assert_eq!(serde_json::from_str::<SatelliteKind>(&json).expect("deserialize"), kind);
        assert_eq!(serde_json::from_str::<SatelliteKind>("\"custom\"").expect("custom"), SatelliteKind::Custom);
    }
}
