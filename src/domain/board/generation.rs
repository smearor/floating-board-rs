use std::fmt::Display;
use std::fmt::Formatter;

/// Monotone generation counter of a card's layout animation.
///
/// Every new `apply_layout` glide increments the generation; frame
/// callbacks created for an older generation notice the mismatch and
/// stop themselves, so a superseded animation never fights the new one.
///
/// Newtype over `u64` so an animation generation can no longer be
/// confused with any other count (e.g. the insertion `SequenceNumber`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct GenerationCounter(pub u64);

impl GenerationCounter {
    /// Creates a generation counter.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// The counter value.
    pub fn value(&self) -> u64 {
        self.0
    }

    /// The next generation after this one.
    pub fn incremented(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for GenerationCounter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<u64> for GenerationCounter {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<GenerationCounter> for u64 {
    fn from(value: GenerationCounter) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremented_advances_by_one() {
        assert_eq!(GenerationCounter::new(3).incremented(), GenerationCounter::new(4));
    }

    #[test]
    fn displays_as_plain_number() {
        assert_eq!(format!("{}", GenerationCounter::new(7)), "7");
    }
}
