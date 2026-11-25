use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::ScaleFactor;
use smearor_commons_geometry::Vec2;

/// The animated state of a folding card at one progress point - each
/// field maps to one animatable axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldPose {
    /// Card footprint in px, applied via the morph's layout pin
    /// (`CardImpl::morph_measure`).
    pub footprint: Vec2<Pixels>,
    /// Extra zoom factor composed on the card's rendered scale.
    pub zoom: ScaleFactor,
    /// Linear rotation sweep in radians composed on the card's
    /// rendered rotation. Deliberately [`Radians`], not `RotationRad`:
    /// a `Spin` pose sweeps `0 -> 2*pi`, and the wrap-aware
    /// shortest-arc semantics of `RotationRad` interpolation would
    /// collapse a full turn back to zero.
    pub rotation: Radians,
    /// Opacity of the content pane the transition leaves (start state).
    pub opacity_out: Fraction,
    /// Opacity of the content pane the transition enters (end state).
    pub opacity_in: Fraction,
}
