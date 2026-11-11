use std::borrow::Borrow;
use std::fmt::Display;
use std::fmt::Formatter;

/// Stable, semantic identifier of a card on the board.
///
/// Consumers assign meaningful keys (e.g. `"controls"`, `"params"`) which stay
/// stable inside persisted layouts (`gui-cards.toml`). Auto-generated ids use
/// the `card-N` scheme.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct CardId(String);

impl CardId {
    /// Creates a new card identifier from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the identifier as string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CardId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for CardId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for CardId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for CardId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for CardId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_id_preserves_semantic_key() {
        let id = CardId::new("controls");
        assert_eq!(id.as_str(), "controls");
        assert_eq!(id.to_string(), "controls");
    }

    #[test]
    fn card_id_equality_and_ordering() {
        let a = CardId::from("a");
        let b = CardId::from("b");
        assert!(a < b);
        assert_eq!(a, CardId::from("a"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn card_id_serializes_as_plain_string() {
        let id = CardId::new("controls");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"controls\"");
        let parsed: CardId = serde_json::from_str("\"params\"").unwrap();
        assert_eq!(parsed, CardId::new("params"));
    }
}
