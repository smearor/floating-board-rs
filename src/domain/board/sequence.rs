use std::fmt::Display;
use std::fmt::Formatter;

/// Monotone insertion counter used for stable z-ordering and for naming
/// auto-generated `card-N` ids.
///
/// Newtype over `u64` so an insertion rank can no longer be confused with
/// any other count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SequenceNumber(pub u64);

impl SequenceNumber {
    /// Creates a sequence number.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// The counter value.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// The next sequence number after this one.
    pub fn incremented(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for SequenceNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<u64> for SequenceNumber {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<SequenceNumber> for u64 {
    fn from(value: SequenceNumber) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremented_advances_by_one() {
        assert_eq!(SequenceNumber::new(3).incremented(), SequenceNumber::new(4));
    }

    #[test]
    fn displays_as_plain_number() {
        assert_eq!(format!("{}", SequenceNumber::new(7)), "7");
    }
}
