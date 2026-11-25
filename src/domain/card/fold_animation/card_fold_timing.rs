use smearor_commons_geometry::Fraction;

/// Where in the fold choreography an animated fold
/// (`CardConfig::fold_animation`) plays relative to the satellite
/// set's close transitions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CardFoldTiming {
    /// Satellites sink into the card first; the fold morph plays once
    /// the last close transition lands. Reads as a causal sequence:
    /// first the contents, then the container.
    #[default]
    Sequential,
    /// The fold morph starts while the set is still closing - once the
    /// leading satellite's close progress reaches the enclosed
    /// fraction. `0.0` starts fully parallel with the close, values
    /// approaching `1.0` converge on `Sequential` behavior.
    Overlap(Fraction),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_is_the_default() {
        assert_eq!(CardFoldTiming::default(), CardFoldTiming::Sequential);
    }
}
