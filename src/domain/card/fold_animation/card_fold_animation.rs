use super::FoldAxis;
use super::FoldPose;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::ScaleFactor;
use smearor_commons_geometry::Vec2;

/// How a foldable card transitions between its unfolded and folded
/// presentation. The duration comes from the card's
/// `animation_speed_ms` budget; reduced motion (`gtk-enable-animations`
/// off) and `Instant` both land atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CardFoldAnimation {
    /// Atomic swap - the folded state changes with no transition.
    #[default]
    Instant,
    /// The outgoing content fades out while the folded content fades
    /// in; the card footprint tweens linearly between both states.
    Crossfade,
    /// The card tweens its footprint through a zoom dip while the
    /// contents crossfade - e.g. a large clock morphing into its
    /// folded miniature.
    Shrink,
    /// Like `Shrink`, plus the card twirls a full revolution on top of
    /// its own rotation through the zoom dip - rotation starts and
    /// ends at the same angle but sweeps the transition.
    Spin,
}

impl CardFoldAnimation {
    /// The animated pose at eased progress `t` (`Fraction::ZERO` =
    /// start state, `Fraction::ONE` = end state) between the two
    /// footprints `from`/`to`. `Instant` has no transition and returns
    /// `None`.
    pub fn pose(&self, t: Fraction, from: Vec2<Pixels>, to: Vec2<Pixels>) -> Option<FoldPose> {
        let (zoom_excursion, rotation_sweep) = match self {
            Self::Instant => return None,
            Self::Crossfade => (0.0, 0.0),
            Self::Shrink => (-0.45, 0.0),
            Self::Spin => (-0.5, std::f64::consts::TAU),
        };
        Some(FoldPose {
            footprint: Vec2::<Pixels>::new(FoldAxis::new(from.x, to.x, 0.0).value(t), FoldAxis::new(from.y, to.y, 0.0).value(t)),
            zoom: ScaleFactor::new(FoldAxis::new(1.0, 1.0, zoom_excursion).value(t)),
            rotation: FoldAxis::new(Radians::ZERO, Radians(rotation_sweep), Radians::ZERO).value(t),
            opacity_out: Fraction::ONE - t,
            opacity_in: t,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn footprint(x: f64, y: f64) -> Vec2<Pixels> {
        Vec2::<Pixels>::new(x, y)
    }

    #[test]
    fn instant_has_no_pose() {
        assert!(
            CardFoldAnimation::Instant
                .pose(Fraction::HALF, footprint(200.0, 300.0), footprint(80.0, 80.0))
                .is_none()
        );
    }

    #[test]
    fn animated_poses_start_and_end_at_the_real_states() {
        for kind in [CardFoldAnimation::Crossfade, CardFoldAnimation::Shrink, CardFoldAnimation::Spin] {
            let (from, to) = (footprint(200.0, 300.0), footprint(80.0, 80.0));
            let start = kind.pose(Fraction::ZERO, from, to).unwrap_or_else(|| unreachable!());
            let end = kind.pose(Fraction::ONE, from, to).unwrap_or_else(|| unreachable!());
            assert_eq!(start.footprint, from);
            assert_eq!(end.footprint, to);
            assert_eq!(start.zoom, ScaleFactor::ONE);
            assert_eq!(end.zoom, ScaleFactor::ONE);
            assert_eq!(start.rotation, Radians::ZERO);
            assert_eq!(start.opacity_out, Fraction::ONE);
            assert_eq!(end.opacity_in, Fraction::ONE);
        }
    }

    #[test]
    fn shrink_dips_zoom_mid_transition() {
        let pose = CardFoldAnimation::Shrink
            .pose(Fraction::HALF, footprint(200.0, 300.0), footprint(80.0, 80.0))
            .unwrap_or_else(|| unreachable!());
        assert!(pose.zoom.as_f64() < 0.6, "zoom {:?} should dip below 0.6 mid-shrink", pose.zoom);
        assert_eq!(pose.rotation, Radians::ZERO);
    }

    #[test]
    fn spin_twirls_a_full_revolution() {
        let mid = CardFoldAnimation::Spin
            .pose(Fraction::HALF, footprint(200.0, 300.0), footprint(80.0, 80.0))
            .unwrap_or_else(|| unreachable!());
        assert!((mid.rotation.as_f64() - std::f64::consts::PI).abs() < 1e-9);
        let end = CardFoldAnimation::Spin
            .pose(Fraction::ONE, footprint(200.0, 300.0), footprint(80.0, 80.0))
            .unwrap_or_else(|| unreachable!());
        // A full turn lands back at the card's own angle - the rotation
        // axis has the same visual value at both ends but swept 2*pi.
        assert!((end.rotation.as_f64() - std::f64::consts::TAU).abs() < 1e-9);
    }

    #[test]
    fn crossfade_fades_opacity_only() {
        let pose = CardFoldAnimation::Crossfade
            .pose(Fraction::HALF, footprint(200.0, 300.0), footprint(80.0, 80.0))
            .unwrap_or_else(|| unreachable!());
        assert_eq!(pose.zoom, ScaleFactor::ONE);
        assert_eq!(pose.rotation, Radians::ZERO);
        assert!((pose.opacity_out.as_f64() - 0.5).abs() < 1e-9);
        assert!((pose.opacity_in.as_f64() - 0.5).abs() < 1e-9);
    }
}
