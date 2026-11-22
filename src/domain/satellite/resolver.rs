use crate::domain::CardGeometry;
use crate::domain::CardPosition;
use crate::domain::SatelliteItem;
use crate::domain::SatelliteLayout;
use crate::domain::SatellitePlacement;
use crate::domain::SatelliteRotationMode;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::Vec2;
use smearor_commons_layout::{EdgeStripLayout, SideSet};
use smearor_commons_layout::LayoutError;
use smearor_commons_layout::LayoutItem;
use smearor_commons_layout::LayoutStrategy;

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
/// `Radial` resolves to nothing yet - the orbit path lands in Phase 3;
/// unplaced satellites stay hidden in the widget layer.
pub fn resolve(
    geometry: &CardGeometry,
    card_footprint: Vec2<Pixels>,
    layout: &SatelliteLayout,
    mode: SatelliteRotationMode,
    items: &[SatelliteItem],
) -> Result<Vec<SatellitePlacement>, LayoutError> {
    match layout {
        SatelliteLayout::Sidebar { sides } => resolve_sidebar(geometry, card_footprint, *sides, mode, items),
        SatelliteLayout::Radial(_) => Ok(Vec::new()),
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

    /// The radial layout is Phase-3 matter: it resolves nothing yet.
    #[test]
    fn radial_resolves_nothing_yet() {
        let placements = resolve(
            &card(0.0, 0.0, 0.0, 1.0),
            Vec2::new(200.0, 100.0),
            &SatelliteLayout::Radial(Default::default()),
            SatelliteRotationMode::CoRotate,
            &[item("a", 30.0, 30.0)],
        )
        .expect("empty");
        assert!(placements.is_empty());
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
