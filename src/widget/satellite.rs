//! Satellite mounting and geometry sync, implemented on [`FloatingBoard`].
//!
//! Satellites are `CardLayer` children next to their card, never inside
//! it: each gets the same mini transform stack (`ScalingWidget` ->
//! `RotationWidget` -> satellite widget) a card gets, stacked directly
//! above `entry.scaling`. Placements come from the pure domain resolver
//! ([`resolve`]) and are re-applied on every card geometry change via
//! [`FloatingBoard::apply_position`], so satellites follow drag, rotate,
//! pinch and layout animations without their own tracking code.

use crate::domain::ChromeElement;
use crate::domain::SatelliteItem;
use crate::domain::SatelliteKind;
use crate::domain::SatelliteLayout;
use crate::domain::SatellitePlacement;
use crate::domain::TurnDirection;
use crate::domain::resolve;
use crate::widget::board::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_config::CardConfig;
use crate::widget::card_layer::CardLayer;
use crate::widget::chrome::ICON_MINIMIZE_SYMBOLIC;
use crate::widget::css::CLASS_SATELLITE;
use crate::widget::css::CLASS_SATELLITE_CHROME;
use crate::widget::css::CLASS_SATELLITE_CUSTOM;
use crate::widget::css::CLASS_SATELLITE_RADIAL;
use crate::widget::imp::card_entry::CardEntry;
use crate::widget::imp::satellite_entry::SatelliteEntry;
use crate::widget::satellite_config::SatelliteConfig;
use crate::widget::scaling::ScalingWidget;
use glib::object::Cast;
use gtk4::Widget;
use gtk4::prelude::FixedExt;
use gtk4::prelude::WidgetExt;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationDirection;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use smearor_wrot_rotation::RotationControlHandler;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::cell::Cell;
use tracing::warn;

impl FloatingBoard {
    /// Builds and mounts one [`SatelliteEntry`] per effective satellite
    /// config (chrome satellite slots plus `CardSatellites::satellites`),
    /// each stacked directly above the card's transform stack in config
    /// order. Satellites whose widget cannot be built are warned about
    /// and skipped.
    pub(crate) fn build_satellites(&self, layer: &CardLayer, card: &Card, card_scaling: &ScalingWidget, config: &CardConfig) -> Vec<SatelliteEntry> {
        let (settings, configs) = config.effective_satellites();
        let radial = matches!(settings.layout, SatelliteLayout::Radial(_));
        let mut entries = Vec::with_capacity(configs.len());
        for satellite in configs {
            let Some(widget) = satellite_widget(card, &satellite) else {
                continue;
            };
            if radial {
                widget.add_css_class(CLASS_SATELLITE_RADIAL);
            }
            // Same builder options as the card stack: no built-in rotate
            // gesture, no input-region clamp, natural child size.
            let rotation_widget = RotationWidget::new(SmearorRotation::DEG_0)
                .with_gesture_rotation_enabled(false)
                .with_input_region_enabled(false)
                .with_natural_child_size_enabled(true);
            rotation_widget.set_child(Some(&widget));
            let scaling = ScalingWidget::new(&rotation_widget);
            // Group z-order: the first satellite sits directly above the
            // card, each next directly above the previous satellite -
            // raising the card re-puts the whole group (see `raise_card`).
            let sibling = entries
                .last()
                .map_or_else(|| card_scaling.upcast_ref::<Widget>(), |entry: &SatelliteEntry| entry.scaling.upcast_ref::<Widget>());
            scaling.insert_after(layer, Some(sibling));
            entries.push(SatelliteEntry {
                id: satellite.id,
                anchor: satellite.anchor,
                widget,
                scaling,
                rotation_widget,
                last_geometry: Cell::new(None),
                open: Cell::new(settings.open),
            });
        }
        entries
    }

    /// Re-resolves every satellite placement from the card's current
    /// geometry and applies it to the satellite transform stacks -
    /// through the card's own view mapping, so a sticky card pins its
    /// satellites to the glass with it. Called from `apply_position`,
    /// the tail of every card geometry mutation.
    ///
    /// Visibility: satellites hide while their card is folded or hidden
    /// (v1 rule); a layout that does not fit hides them all with a
    /// warning rather than dropping entries.
    pub(crate) fn sync_satellites(&self, entry: &CardEntry) {
        let satellites = entry.satellites.borrow();
        if satellites.is_empty() {
            return;
        }
        let Some(layer) = self.card_layer() else {
            return;
        };
        let (settings, _) = entry.config.effective_satellites();
        let items: Vec<SatelliteItem> = satellites
            .iter()
            .map(|satellite| SatelliteItem {
                id: satellite.id.clone(),
                size: satellite.widget.measure_size().1.to_f64().with_unit::<Pixels>(),
                anchor: satellite.anchor,
            })
            .collect();
        let geometry = self.entry_geometry(entry);
        let (_, natural) = entry.card.measure_size();
        let footprint = natural.to_f64().with_unit::<Pixels>();
        // Folded or hidden cards hide their satellite set entirely.
        let shown = !entry.card.folded() && entry.scaling.is_visible();
        match resolve(&geometry, footprint, &settings.layout, settings.rotation_mode, &items) {
            Ok(placements) => {
                for satellite in satellites.iter() {
                    match placements.iter().find(|placement| placement.id == satellite.id) {
                        Some(placement) => {
                            satellite.scaling.set_visible(shown && satellite.open.get());
                            self.apply_satellite_placement(entry, satellite, &layer, placement);
                        }
                        None => {
                            satellite.scaling.set_visible(false);
                            satellite.last_geometry.set(None);
                        }
                    }
                }
            }
            Err(error) => {
                warn!(card = %entry.card.id(), %error, "satellite layout does not fit - hiding satellites");
                for satellite in satellites.iter() {
                    satellite.scaling.set_visible(false);
                    satellite.last_geometry.set(None);
                }
            }
        }
    }

    /// Re-stacks a card's satellites directly above it - same put-on-top
    /// semantics `raise_card`/`restack` use for the card stack itself,
    /// so the group (card + its satellites) always raises together.
    /// Positions are preserved; only the child order changes.
    pub(crate) fn restack_satellites(&self, entry: &CardEntry, layer: &CardLayer) {
        for satellite in entry.satellites.borrow().iter() {
            let position = Vec2::<Pixels>::from(layer.child_position(&satellite.scaling));
            layer.remove(&satellite.scaling);
            layer.put(&satellite.scaling, position.x, position.y);
        }
    }

    /// Applies one resolved workspace placement to a satellite's stack
    /// (also recorded in `last_geometry` for the clamp union):
    /// rendered rotation = placement rotation + view rotation, rendered
    /// scale = placement scale x view zoom, position = view-mapped
    /// center - the same mapping `sync_rendered_transform` applies to
    /// the card.
    fn apply_satellite_placement(&self, entry: &CardEntry, satellite: &SatelliteEntry, layer: &CardLayer, placement: &SatellitePlacement) {
        let view = self.card_view(entry);
        let rendered_rotation = placement.geometry.rotation.to_rotation_rad() + view.rotation;
        satellite.rotation_widget.set_rotation(SmearorRotation::from(rendered_rotation));
        let rendered_scale = placement.geometry.scale.value() * self.card_view_zoom(entry);
        satellite.scaling.set_scale(rendered_scale);
        // `gtk4::Fixed` allocates (minimum width, natural height), so
        // centering on the natural size would displace the rendered
        // widget by (nat - min) / 2 - same contract the card stack
        // follows via `stack_top_left`.
        let (minimum, natural) = satellite.widget.measure_size();
        let min_size = (minimum.to_f64().with_unit::<Pixels>() * rendered_scale).rotated_bounds(rendered_rotation);
        let nat_size = (natural.to_f64().with_unit::<Pixels>() * rendered_scale).rotated_bounds(rendered_rotation);
        let rendered_size = Vec2::<Pixels>::new(min_size.x, nat_size.y);
        let widget = self.widget_bounds();
        let center = match widget.is_degenerate() {
            true => Vec2::<Pixels>::from(placement.geometry.position),
            false => view.to_widget(placement.geometry.position.into(), widget),
        };
        layer.move_(&satellite.scaling, center.x - rendered_size.x / 2.0, center.y - rendered_size.y / 2.0);
        satellite.last_geometry.set(Some(placement.geometry));
    }
}

/// Builds the satellite's widget: chrome elements come from the card's
/// own mount-agnostic builders (P1.5), custom satellites hand over the
/// consumer's widget (the board owns placement and lifetime only).
fn satellite_widget(card: &Card, config: &SatelliteConfig) -> Option<Widget> {
    let widget = match config.kind {
        SatelliteKind::Chrome(ChromeElement::Minimize) => card.build_minimize_button(ICON_MINIMIZE_SYMBOLIC).upcast(),
        SatelliteKind::Chrome(ChromeElement::Close) => card.build_close_button().upcast(),
        SatelliteKind::Chrome(ChromeElement::RotateClockwise) => card.build_rotate_button(RotationDirection::Clockwise).upcast(),
        SatelliteKind::Chrome(ChromeElement::RotateCounterClockwise) => card.build_rotate_button(RotationDirection::CounterClockwise).upcast(),
        SatelliteKind::Chrome(ChromeElement::NextSeat) => card.build_pass_button(TurnDirection::Clockwise).upcast(),
        SatelliteKind::Chrome(ChromeElement::PreviousSeat) => card.build_pass_button(TurnDirection::CounterClockwise).upcast(),
        SatelliteKind::Chrome(element) => {
            warn!(satellite = %config.id, element = ?element, "chrome element is not satellite-capable - satellite skipped");
            return None;
        }
        SatelliteKind::Custom => match &config.widget {
            Some(widget) => widget.clone(),
            None => {
                warn!(satellite = %config.id, "custom satellite without widget - skipped");
                return None;
            }
        },
    };
    widget.add_css_class(CLASS_SATELLITE);
    widget.add_css_class(match config.kind {
        SatelliteKind::Chrome(_) => CLASS_SATELLITE_CHROME,
        SatelliteKind::Custom => CLASS_SATELLITE_CUSTOM,
    });
    Some(widget)
}
