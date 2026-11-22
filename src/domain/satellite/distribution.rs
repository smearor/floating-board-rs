/// How satellites distribute along a radial layout's span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum SatelliteDistribution {
    /// Evenly spaced across the whole span (default).
    #[default]
    Even,
    /// Pinned to fixed anchor angles (e.g. `QuarterTurn`s) - satellites
    /// without an `anchor` fall back to `Even` distribution in the
    /// remaining gaps.
    Anchored,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_even() {
        assert_eq!(SatelliteDistribution::default(), SatelliteDistribution::Even);
    }
}
