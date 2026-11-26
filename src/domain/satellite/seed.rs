use crate::domain::CardGeometry;
use crate::domain::CardPosition;
use crate::domain::CardShape;
use crate::domain::SatelliteLayout;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;

/// The "collapsed" satellite placement a satellite emerges from on
/// open and sinks back into on close - the seed endpoint of the
/// open/close choreography (Phase 5).
///
/// Per layout: `Radial` satellites fly out of the card center along
/// their orbit angle (the radial pop); `Sidebar` satellites slide out
/// of the card edge - the seed sits where the ray from the card center
/// to the resolved position exits the card's contour. Both seeds carry
/// the resolved rotation at minimal scale, so the lerp grows the
/// satellite while it travels.
pub(crate) fn collapsed_seed(
    resolved: &CardGeometry,
    card_geometry: &CardGeometry,
    card_footprint: Vec2<Pixels>,
    card_shape: CardShape,
    layout: &SatelliteLayout,
) -> CardGeometry {
    let center = Vec2::<Pixels>::from(card_geometry.position);
    let offset = Vec2::<Pixels>::from(resolved.position) - center;
    let distance = offset.magnitude_f64();
    let edge = match (layout, distance > 1e-9) {
        (SatelliteLayout::Radial(_), _) | (_, false) => 0.0,
        (SatelliteLayout::Sidebar { .. }, true) => contour_support(offset / distance, card_footprint, card_shape),
    };
    CardGeometry {
        position: CardPosition::new(center + offset.normalized() * distance.min(edge)),
        rotation: resolved.rotation,
        scale: crate::domain::ScaleFactor::new(0.01),
    }
}

/// How far the card's outline extends along unit direction `dir`,
/// measured from its center: the disk radius for a `Circle` card, the
/// axis-aligned box support `(|dx| * w + |dy| * h) / 2` for a
/// `Rectangle` card (exact for the bbox - the visible corners of a
/// rotated card slightly overshoot, which is fine for an emerge seed).
fn contour_support(dir: Vec2<Pixels>, card_footprint: Vec2<Pixels>, card_shape: CardShape) -> f64 {
    match card_shape {
        // The dome orbits like its full circle - the bbox's inscribed
        // radius, a uniform gap all around.
        CardShape::Circle | CardShape::Semicircle => card_footprint.x / 2.0,
        CardShape::Rectangle => (dir.x.abs() * card_footprint.x + dir.y.abs() * card_footprint.y) / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smearor_commons_layout::SideSet;

    fn card(x: f64, y: f64) -> CardGeometry {
        CardGeometry {
            position: CardPosition::new(Vec2::<Pixels>::new(x, y)),
            ..Default::default()
        }
    }

    fn placed(x: f64, y: f64) -> CardGeometry {
        CardGeometry {
            position: CardPosition::new(Vec2::<Pixels>::new(x, y)),
            ..Default::default()
        }
    }

    fn sidebar() -> SatelliteLayout {
        SatelliteLayout::Sidebar { sides: SideSet::default() }
    }

    fn radial() -> SatelliteLayout {
        SatelliteLayout::Radial(RadialConfig::default())
    }

    use crate::domain::RadialConfig;

    /// Radial seeds collapse to the card center so satellites pop
    /// outward along their orbit angle.
    #[test]
    fn radial_seed_is_the_card_center() {
        let seed = collapsed_seed(&placed(500.0, 100.0), &card(500.0, 300.0), Vec2::new(200.0, 100.0), CardShape::Rectangle, &radial());
        let position = Vec2::<Pixels>::from(seed.position);
        assert_eq!(position, Vec2::new(500.0, 300.0));
        assert!(seed.scale.value() < 0.1);
    }

    /// A bottom-docked satellite's seed sits on the card's bottom
    /// edge, directly under the dock slot - the slide-out start.
    #[test]
    fn sidebar_seed_sits_on_the_card_edge() {
        let seed = collapsed_seed(&placed(500.0, 400.0), &card(500.0, 300.0), Vec2::new(200.0, 100.0), CardShape::Rectangle, &sidebar());
        let position = Vec2::<Pixels>::from(seed.position);
        assert!((position.x - 500.0).abs() < 1e-9);
        assert!((position.y - 350.0).abs() < 1e-9, "seed at {position:?}");
    }

    /// Diagonal targets exit through the box support point
    /// `(|dx| w + |dy| h) / 2` along the direction.
    #[test]
    fn sidebar_seed_follows_the_box_contour_diagonally() {
        let seed = collapsed_seed(&placed(600.0, 450.0), &card(500.0, 300.0), Vec2::new(200.0, 100.0), CardShape::Rectangle, &sidebar());
        let position = Vec2::<Pixels>::from(seed.position);
        // dir = (100,150)/180.28, support = (0.555*200 + 0.832*100)/2
        let dir = Vec2::<f64>::new(100.0, 150.0).normalized();
        let support = (dir.x.abs() * 200.0 + dir.y.abs() * 100.0) / 2.0;
        assert!((position.x - (500.0 + dir.x * support)).abs() < 1e-6, "x {}", position.x);
        assert!((position.y - (300.0 + dir.y * support)).abs() < 1e-6, "y {}", position.y);
    }

    /// A circle card's seed exits at the disk radius, not the bbox
    /// support.
    #[test]
    fn circle_seed_exits_at_the_disk_radius() {
        let seed = collapsed_seed(&placed(500.0, 480.0), &card(500.0, 300.0), Vec2::new(200.0, 200.0), CardShape::Circle, &sidebar());
        let position = Vec2::<Pixels>::from(seed.position);
        assert!((position.y - 400.0).abs() < 1e-9, "seed at {position:?}");
    }

    /// The seed keeps the resolved rotation - emerge twists nothing.
    #[test]
    fn seed_keeps_the_resolved_rotation() {
        let mut resolved = placed(500.0, 400.0);
        resolved.rotation = smearor_commons_geometry::Rotation::from_degrees(45.0);
        let seed = collapsed_seed(&resolved, &card(500.0, 300.0), Vec2::new(200.0, 100.0), CardShape::Rectangle, &radial());
        assert!((seed.rotation.degrees() - 45.0).abs() < 1e-9);
    }
}
