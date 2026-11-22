use crate::domain::CardGeometry;
use crate::domain::CardPosition;
use crate::domain::RadialConfig;
use crate::domain::SatelliteDistribution;
use crate::domain::SatelliteItem;
use crate::domain::SatelliteLayout;
use crate::domain::SatellitePlacement;
use crate::domain::SatelliteRotationMode;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::Vec2;
use smearor_commons_layout::EdgeStripLayout;
use smearor_commons_layout::LayoutError;
use smearor_commons_layout::LayoutItem;
use smearor_commons_layout::LayoutStrategy;
use smearor_commons_layout::OrbitItem;
use smearor_commons_layout::OrbitLayout;
use smearor_commons_layout::SideSet;
use smearor_commons_layout::orbit_radius_for;

/// Pure placement resolution: card geometry + footprint + layout +
/// measured items -> one [`SatellitePlacement`] per item, in workspace
/// coordinates.
///
/// The sidebar path lets `EdgeStripLayout` arrange items in the
/// card-local frame (card center at the origin, natural footprint),
/// then maps each placement through the card transform: scale, rotate
/// about the card center, translate. Widget orientation follows
/// `mode` (`CoRotate` -> card rotation, the upright modes -> no
/// rotation); positions are always card-local for a sidebar -
/// `CounterRotate` orbit semantics are radial matter (Phase 3).
/// Satellites inherit the card's scale factor (rigid docking).
///
/// The `Radial` path lets `OrbitLayout` place items on an arc around
/// the card-local center (`radius` = distance outward from the card's
/// bounding circle via `orbit_radius_for`), then maps through the same
/// card transform. `CounterRotate` is radial-only semantics: orbit
/// angles stay screen-fixed - positions scale and translate but do not
/// rotate with the card, so rotating the card orbits nothing while
/// dragging carries the whole ring along.
pub fn resolve(
    geometry: &CardGeometry,
    card_footprint: Vec2<Pixels>,
    layout: &SatelliteLayout,
    mode: SatelliteRotationMode,
    items: &[SatelliteItem],
) -> Result<Vec<SatellitePlacement>, LayoutError> {
    match layout {
        SatelliteLayout::Sidebar { sides } => resolve_sidebar(geometry, card_footprint, *sides, mode, items),
        SatelliteLayout::Radial(config) => resolve_radial(geometry, card_footprint, config, mode, items),
    }
}

/// The `Sidebar` path: `EdgeStripLayout` in the card-local frame, then
/// mapped through the card transform.
fn resolve_sidebar(
    geometry: &CardGeometry,
    card_footprint: Vec2<Pixels>,
    sides: SideSet,
    mode: SatelliteRotationMode,
    items: &[SatelliteItem],
) -> Result<Vec<SatellitePlacement>, LayoutError> {
    let bounds = Rect {
        min: card_footprint * -0.5,
        size: card_footprint,
    };
    let layout = EdgeStripLayout::<Pixels> { sides, ..Default::default() };
    let layout_items: Vec<LayoutItem<Pixels>> = items.iter().map(|item| LayoutItem::new(item.size, Rotation::ZERO)).collect();
    let placements = layout.arrange(bounds, &layout_items)?;
    let rotation = match mode {
        SatelliteRotationMode::CoRotate => geometry.rotation,
        SatelliteRotationMode::KeepCardAngle | SatelliteRotationMode::CounterRotate => Rotation::ZERO,
    };
    let card_rotation = geometry.rotation.to_rotation_rad();
    let scale = geometry.scale.value();
    let center = Vec2::<Pixels>::from(geometry.position);
    Ok(items
        .iter()
        .zip(placements.iter())
        .map(|(item, placement)| SatellitePlacement {
            id: item.id.clone(),
            geometry: CardGeometry {
                position: CardPosition::new(center + (placement.center * scale).rotated(card_rotation)),
                rotation,
                scale: geometry.scale,
            },
        })
        .collect())
}

/// The `Radial` path: `OrbitLayout` in the card-local frame, then the
/// card transform - [`SatelliteDistribution::Anchored`] forwards each
/// item's anchor verbatim, `Even` drops them. `OrbitLayout` never
/// fails (degenerate inputs yield deterministic placements), so this
/// path always resolves.
fn resolve_radial(
    geometry: &CardGeometry,
    card_footprint: Vec2<Pixels>,
    config: &RadialConfig,
    mode: SatelliteRotationMode,
    items: &[SatelliteItem],
) -> Result<Vec<SatellitePlacement>, LayoutError> {
    let bounds = Rect {
        min: card_footprint * -0.5,
        size: card_footprint,
    };
    // `radius` reads as the gap outward from the card's outline;
    // `orbit_radius_for` converts it to the centerline orbit radius.
    let layout = OrbitLayout::new(orbit_radius_for(bounds, config.radius), config.span);
    let anchored = config.distribution == SatelliteDistribution::Anchored;
    let orbit_items: Vec<OrbitItem<Pixels>> = items
        .iter()
        .map(|item| {
            let mut orbit_item = OrbitItem::new(item.size, Rotation::ZERO);
            if anchored && let Some(anchor) = item.anchor {
                orbit_item = orbit_item.with_anchor(anchor.to_rotation_rad());
            }
            orbit_item
        })
        .collect();
    let placements = layout.arrange(bounds, &orbit_items);
    let rotation = match mode {
        SatelliteRotationMode::CoRotate => geometry.rotation,
        SatelliteRotationMode::KeepCardAngle | SatelliteRotationMode::CounterRotate => Rotation::ZERO,
    };
    let card_rotation = geometry.rotation.to_rotation_rad();
    let scale = geometry.scale.value();
    let center = Vec2::<Pixels>::from(geometry.position);
    Ok(items
        .iter()
        .zip(placements.iter())
        .map(|(item, placement)| {
            let offset = placement.center * scale;
            // `CounterRotate` keeps orbit angles screen-fixed: the
            // offset does not rotate with the card.
            let offset = match mode {
                SatelliteRotationMode::CounterRotate => offset,
                _ => offset.rotated(card_rotation),
            };
            SatellitePlacement {
                id: item.id.clone(),
                geometry: CardGeometry {
                    position: CardPosition::new(center + offset),
                    rotation,
                    scale: geometry.scale,
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SatelliteId;
    use crate::domain::ScaleFactor;
    use smearor_commons_layout::SideSet;
    use smearor_commons_layout::TableSide;

    fn card(x: f64, y: f64, rotation_degrees: f64, scale: f64) -> CardGeometry {
        CardGeometry {
            position: CardPosition::new(Vec2::<Pixels>::new(x, y)),
            rotation: Rotation::from_degrees(rotation_degrees),
            scale: ScaleFactor::new(scale),
        }
    }

    fn item(id: &str, width: f64, height: f64) -> SatelliteItem {
        SatelliteItem::new(SatelliteId::new(id), Vec2::<Pixels>::new(width, height))
    }

    fn sidebar(side: TableSide) -> SatelliteLayout {
        SatelliteLayout::Sidebar { sides: SideSet::single(side) }
    }

    /// One 40x30 satellite below an unrotated 200x100 card at (500, 300):
    /// the strip docks `gap` (12) plus half its height below the edge.
    #[test]
    fn sidebar_docks_below_bottom_edge() {
        let placements = resolve(
            &card(500.0, 300.0, 0.0, 1.0),
            Vec2::new(200.0, 100.0),
            &sidebar(TableSide::Bottom),
            SatelliteRotationMode::CoRotate,
            &[item("a", 40.0, 30.0)],
        )
        .expect("fits");
        let position: Vec2<Pixels> = placements[0].geometry.position.into();
        assert!((position.x - 500.0).abs() < 1e-9);
        assert!((position.y - (300.0 + 50.0 + 12.0 + 15.0)).abs() < 1e-9);
    }

    /// A card turned 90 degrees carries its bottom dock to the card's
    /// left on screen - the position rotates about the card center.
    #[test]
    fn sidebar_position_rotates_with_card() {
        let placements = resolve(
            &card(500.0, 300.0, 90.0, 1.0),
            Vec2::new(200.0, 100.0),
            &sidebar(TableSide::Bottom),
            SatelliteRotationMode::CoRotate,
            &[item("a", 40.0, 30.0)],
        )
        .expect("fits");
        let position: Vec2<Pixels> = placements[0].geometry.position.into();
        // local (0, +77) rotated 90 degrees -> (-77, 0): left of center.
        assert!((position.x - (500.0 - 77.0)).abs() < 1e-9);
        assert!((position.y - 300.0).abs() < 1e-9);
        assert_eq!(placements[0].geometry.rotation, Rotation::from_degrees(90.0));
    }

    /// The dock offset scales with the card, and the satellite reports
    /// the card's scale factor for its own rendering.
    #[test]
    fn sidebar_scales_with_card() {
        let placements = resolve(
            &card(500.0, 300.0, 0.0, 2.0),
            Vec2::new(200.0, 100.0),
            &sidebar(TableSide::Bottom),
            SatelliteRotationMode::CoRotate,
            &[item("a", 40.0, 30.0)],
        )
        .expect("fits");
        let position: Vec2<Pixels> = placements[0].geometry.position.into();
        assert!((position.y - (300.0 + 77.0 * 2.0)).abs() < 1e-9);
        assert_eq!(placements[0].geometry.scale, ScaleFactor::new(2.0));
    }

    /// Upright modes keep positions card-local but zero the widget
    /// rotation - the satellite stays readable on screen.
    #[test]
    fn upright_modes_zero_widget_rotation() {
        for mode in [SatelliteRotationMode::KeepCardAngle, SatelliteRotationMode::CounterRotate] {
            let placements = resolve(
                &card(500.0, 300.0, 90.0, 1.0),
                Vec2::new(200.0, 100.0),
                &sidebar(TableSide::Bottom),
                mode,
                &[item("a", 40.0, 30.0)],
            )
            .expect("fits");
            assert_eq!(placements[0].geometry.rotation, Rotation::ZERO, "{mode:?}");
        }
    }

    /// Every side docks outside its own edge - the four `SideSet`
    /// combinations of a single side each place their satellite on the
    /// matching side of the footprint.
    #[test]
    fn every_single_side_docks_outside() {
        for (side, expected) in [
            (TableSide::Bottom, Vec2::<Pixels>::new(0.0, 1.0)),
            (TableSide::Top, Vec2::<Pixels>::new(0.0, -1.0)),
            (TableSide::Left, Vec2::<Pixels>::new(-1.0, 0.0)),
            (TableSide::Right, Vec2::<Pixels>::new(1.0, 0.0)),
        ] {
            let placements = resolve(
                &card(500.0, 300.0, 0.0, 1.0),
                Vec2::new(200.0, 100.0),
                &sidebar(side),
                SatelliteRotationMode::CoRotate,
                &[item("a", 30.0, 30.0)],
            )
            .expect("fits");
            let position: Vec2<Pixels> = placements[0].geometry.position.into();
            let offset = position - Vec2::new(500.0, 300.0);
            assert!(offset.x * expected.x + offset.y * expected.y > 40.0, "{side:?} docked at {offset:?}");
        }
    }

    /// An empty item list resolves to an empty placement list.
    #[test]
    fn empty_items_resolve_empty() {
        let placements = resolve(
            &card(0.0, 0.0, 0.0, 1.0),
            Vec2::new(200.0, 100.0),
            &sidebar(TableSide::Bottom),
            SatelliteRotationMode::CoRotate,
            &[],
        )
        .expect("empty");
        assert!(placements.is_empty());
    }

    fn radial(distribution: SatelliteDistribution) -> SatelliteLayout {
        SatelliteLayout::Radial(RadialConfig {
            distribution,
            ..Default::default()
        })
    }

    /// N flexible satellites spread evenly on the full circle: `Arc::FULL`
    /// spaces `i/n`, so four items land at 0/90/180/270 degrees.
    #[test]
    fn radial_even_full_circle() {
        let items: Vec<SatelliteItem> = ["a", "b", "c", "d"].iter().map(|id| item(id, 30.0, 30.0)).collect();
        let placements = resolve(
            &card(0.0, 0.0, 0.0, 1.0),
            Vec2::new(200.0, 100.0),
            &radial(SatelliteDistribution::Even),
            SatelliteRotationMode::CoRotate,
            &items,
        )
        .expect("resolves");
        assert_eq!(placements.len(), 4);
        // Direction 0 is +x (right); 90 degrees in y-down space is down.
        let center = |placement: &SatellitePlacement| Vec2::<Pixels>::from(placement.geometry.position);
        let radius = 200.0_f64.hypot(100.0) / 2.0 + 48.0;
        let expected = [(radius, 0.0), (0.0, radius), (-radius, 0.0), (0.0, -radius)];
        for (placement, (ex, ey)) in placements.iter().zip(expected) {
            let position = center(placement);
            assert!((position.x - ex).abs() < 1e-9, "{:?} expected x {ex}", placement.id);
            assert!((position.y - ey).abs() < 1e-9, "{:?} expected y {ey}", placement.id);
        }
    }

    /// A limited span covers both endpoints: three items on a 180-degree
    /// arc land at start, midpoint and end.
    #[test]
    fn radial_even_partial_span() {
        let layout = SatelliteLayout::Radial(RadialConfig {
            span: smearor_commons_geometry::Arc::new(smearor_commons_geometry::RotationRad::ZERO, smearor_commons_geometry::RotationRad::from_degrees(180.0)),
            ..Default::default()
        });
        let items: Vec<SatelliteItem> = ["a", "b", "c"].iter().map(|id| item(id, 30.0, 30.0)).collect();
        let placements = resolve(&card(0.0, 0.0, 0.0, 1.0), Vec2::new(200.0, 100.0), &layout, SatelliteRotationMode::CoRotate, &items).expect("resolves");
        let radius = 200.0_f64.hypot(100.0) / 2.0 + 48.0;
        let expected = [(radius, 0.0), (0.0, radius), (-radius, 0.0)];
        for (placement, (ex, ey)) in placements.iter().zip(expected) {
            let position = Vec2::<Pixels>::from(placement.geometry.position);
            assert!((position.x - ex).abs() < 1e-9 && (position.y - ey).abs() < 1e-9, "{:?} at {position:?}", placement.id);
        }
    }

    /// Orbit center coincides with the card's position (the card
    /// center) for a card away from the origin.
    #[test]
    fn radial_orbit_centered_on_card() {
        let items: Vec<SatelliteItem> = ["a", "b", "c", "d"].iter().map(|id| item(id, 30.0, 30.0)).collect();
        let placements = resolve(
            &card(330.0, 400.0, 0.0, 1.0),
            Vec2::new(227.0, 135.0),
            &radial(SatelliteDistribution::Even),
            SatelliteRotationMode::CoRotate,
            &items,
        )
        .expect("resolves");
        // Even on FULL: the mean of all positions is the orbit center.
        let sum = placements
            .iter()
            .fold(Vec2::<Pixels>::ZERO, |acc, placement| acc + Vec2::<Pixels>::from(placement.geometry.position));
        let mean = sum / placements.len() as f64;
        assert!(
            (mean.x - 330.0).abs() < 1e-9 && (mean.y - 400.0).abs() < 1e-9,
            "orbit center {mean:?} != card center (330,400)"
        );
    }

    /// `Anchored` pins anchored satellites to their card-local angle
    /// verbatim while flexible ones fill the gaps; `Even` ignores
    /// anchors entirely.
    #[test]
    fn radial_anchored_vs_even() {
        let mut pinned = item("pinned", 30.0, 30.0);
        pinned.anchor = Some(Rotation::from_degrees(90.0));
        let free = item("free", 30.0, 30.0);
        let items = vec![pinned, free];
        let anchored = resolve(
            &card(0.0, 0.0, 0.0, 1.0),
            Vec2::new(200.0, 100.0),
            &radial(SatelliteDistribution::Anchored),
            SatelliteRotationMode::CoRotate,
            &items,
        )
        .expect("resolves");
        let radius = 200.0_f64.hypot(100.0) / 2.0 + 48.0;
        let pinned_pos = Vec2::<Pixels>::from(anchored[0].geometry.position);
        assert!((pinned_pos.x).abs() < 1e-9 && (pinned_pos.y - radius).abs() < 1e-9, "pinned at {pinned_pos:?}");
        // The single flexible item sits at the midpoint of the one
        // gap (180 degrees from the anchor).
        let free_pos = Vec2::<Pixels>::from(anchored[1].geometry.position);
        assert!((free_pos.x).abs() < 1e-9 && (free_pos.y + radius).abs() < 1e-9, "free at {free_pos:?}");
        // `Even` ignores the anchor: two items space at 0 and 180.
        let even = resolve(
            &card(0.0, 0.0, 0.0, 1.0),
            Vec2::new(200.0, 100.0),
            &radial(SatelliteDistribution::Even),
            SatelliteRotationMode::CoRotate,
            &items,
        )
        .expect("resolves");
        let first = Vec2::<Pixels>::from(even[0].geometry.position);
        assert!((first.x - radius).abs() < 1e-9 && (first.y).abs() < 1e-9, "even at {first:?}");
    }

    /// Rotation modes on the orbit: `CoRotate` rotates positions and
    /// widgets with the card, `KeepCardAngle` rotates positions but
    /// keeps widgets upright, `CounterRotate` keeps orbit angles
    /// screen-fixed (position stays, widget upright).
    #[test]
    fn radial_rotation_modes() {
        let items = vec![item("a", 30.0, 30.0)];
        let radius = 200.0_f64.hypot(100.0) / 2.0 + 48.0;
        for mode in [
            SatelliteRotationMode::CoRotate,
            SatelliteRotationMode::KeepCardAngle,
            SatelliteRotationMode::CounterRotate,
        ] {
            for degrees in [0.0, 45.0, 90.0, 180.0] {
                let placements =
                    resolve(&card(0.0, 0.0, degrees, 1.0), Vec2::new(200.0, 100.0), &radial(SatelliteDistribution::Even), mode, &items).expect("resolves");
                let position = Vec2::<Pixels>::from(placements[0].geometry.position);
                let widget_rotation = placements[0].geometry.rotation;
                // A single item on a full circle sits at the span's
                // midpoint - 180 degrees, opposite the arc seam.
                let base = 180.0_f64;
                match mode {
                    SatelliteRotationMode::CoRotate => {
                        // The card's rotation carries the angle along.
                        let radians = (base + degrees).to_radians();
                        assert!(
                            (position.x - radius * radians.cos()).abs() < 1e-6 && (position.y - radius * radians.sin()).abs() < 1e-6,
                            "{mode:?} {degrees}deg at {position:?}"
                        );
                        assert_eq!(widget_rotation, Rotation::from_degrees(degrees));
                    }
                    SatelliteRotationMode::KeepCardAngle => {
                        let radians = (base + degrees).to_radians();
                        assert!((position.x - radius * radians.cos()).abs() < 1e-6, "{mode:?} {degrees}deg at {position:?}");
                        assert_eq!(widget_rotation, Rotation::ZERO);
                    }
                    SatelliteRotationMode::CounterRotate => {
                        // Screen-fixed: 180 degrees in workspace
                        // regardless of the card's rotation.
                        assert!((position.x + radius).abs() < 1e-9 && (position.y).abs() < 1e-9, "{mode:?} {degrees}deg at {position:?}");
                        assert_eq!(widget_rotation, Rotation::ZERO);
                    }
                }
            }
        }
    }

    /// The orbit radius inherits the card scale, same as the sidebar.
    #[test]
    fn radial_scales_with_card() {
        let items = vec![item("a", 30.0, 30.0)];
        let radius = 200.0_f64.hypot(100.0) / 2.0 + 48.0;
        let placements = resolve(
            &card(0.0, 0.0, 0.0, 2.0),
            Vec2::new(200.0, 100.0),
            &radial(SatelliteDistribution::Even),
            SatelliteRotationMode::CoRotate,
            &items,
        )
        .expect("resolves");
        let position = Vec2::<Pixels>::from(placements[0].geometry.position);
        // Angle 180 (span midpoint) at double radius.
        assert!((position.x + radius * 2.0).abs() < 1e-9, "scaled radius at {position:?}");
        assert_eq!(placements[0].geometry.scale, crate::domain::ScaleFactor::new(2.0));
    }

    /// Items too wide for the remaining edges propagate `DoesNotFit`
    /// instead of silently dropping satellites.
    #[test]
    fn overflow_propagates_layout_error() {
        let result = resolve(
            &card(0.0, 0.0, 0.0, 1.0),
            Vec2::new(100.0, 100.0),
            &sidebar(TableSide::Bottom),
            SatelliteRotationMode::CoRotate,
            &[item("a", 150.0, 30.0)],
        );
        assert!(result.is_err());
    }
}
