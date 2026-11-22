use std::borrow::Borrow;
use std::fmt::Display;
use std::fmt::Formatter;

/// Stable, semantic identifier of a satellite attached to a card.
///
/// Consumers assign meaningful keys (e.g. `"fold"`, `"palette"`) which
/// stay stable inside persisted layouts and key the resolver's
/// [`crate::domain::SatellitePlacement`] results back to the satellite.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct SatelliteId(String);

impl SatelliteId {
    /// Creates a new satellite identifier from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the identifier as string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SatelliteId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for SatelliteId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for SatelliteId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for SatelliteId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for SatelliteId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satellite_id_preserves_semantic_key() {
        let id = SatelliteId::new("fold");
        assert_eq!(id.as_str(), "fold");
        assert_eq!(id.to_string(), "fold");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn satellite_id_serializes_as_plain_string() {
        let id = SatelliteId::new("fold");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"fold\"");
        let parsed: SatelliteId = serde_json::from_str("\"palette\"").unwrap();
        assert_eq!(parsed, SatelliteId::new("palette"));
    }
}
