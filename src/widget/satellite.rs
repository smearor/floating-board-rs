//! Satellite mounting and geometry sync, implemented on [`FloatingBoard`].
//!
//! Satellites are `CardLayer` children next to their card, never inside
//! it: each gets the same mini transform stack (`ScalingWidget` ->
//! `RotationWidget` -> satellite widget) a card gets, stacked directly
//! above `entry.scaling`. Placements come from the pure domain resolver
//! ([`resolve`]) and are re-applied on every card geometry change via
//! [`FloatingBoard::apply_position`], so satellites follow drag, rotate,
//! pinch and layout animations without their own tracking code.

use crate::domain::CardGeometry;
use crate::domain::CardPosition;
use crate::domain::CardShape;
use crate::domain::ChromeElement;
use crate::domain::SatelliteItem;
use crate::domain::SatelliteKind;
use crate::domain::SatelliteLayout;
use crate::domain::SatellitePlacement;
use crate::domain::ScaleFactor;
use crate::domain::TurnDirection;
use crate::domain::collapsed_seed;
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
use crate::widget::imp::satellite_entry::SatelliteTransition;
use crate::widget::satellite_config::SatelliteConfig;
use crate::widget::scaling::ScalingWidget;
use glib::ControlFlow;
use glib::object::Cast;
use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::Widget;
use gtk4::prelude::FixedExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::WidgetExtManual;
use smearor_commons_animation::EasingFunction;
use smearor_commons_animation::Interpolate;
use smearor_commons_animation::Span;
use smearor_commons_animation::TimedAnimation;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::RotationDirection;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use smearor_wrot_rotation::RotationControlHandler;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::cell::Cell;
use std::cell::RefCell;
use std::time::Duration;
use std::time::Instant;
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
                kind: satellite.kind,
                anchor: satellite.anchor,
                widget,
                scaling,
                rotation_widget,
                last_geometry: Cell::new(None),
                open: Cell::new(settings.open),
                transition: RefCell::new(None),
                last_local_geometry: Cell::new(None),
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
        let items = satellite_items(&satellites);
        let geometry = self.entry_geometry(entry);
        let (_, natural) = entry.card.measure_size();
        let footprint = natural.to_f64().with_unit::<Pixels>();
        // Folded or hidden cards hide their satellite set entirely.
        let folded = entry.card.folded();
        let shown = !folded && entry.scaling.is_visible();
        // Exception: a circle card has no header strip - its `Minimize`
        // satellite is the only fold/unfold control and stays visible
        // while folded (otherwise the fold would be a dead end).
        let fold_exception = folded && entry.scaling.is_visible() && entry.config.shape == CardShape::Circle && !entry.config.unfold_on_click;
        match resolve(&geometry, footprint, entry.config.shape, &settings.layout, settings.rotation_mode, &items) {
            Ok(placements) => {
                for satellite in satellites.iter() {
                    match placements.iter().find(|placement| placement.id == satellite.id) {
                        Some(placement) => {
                            // Retarget detection (P5.2): a placement that
                            // moved in card-local space (footprint or
                            // layout change, not a card gesture) tweens
                            // instead of jumping.
                            self.maybe_begin_move_transition(entry, satellite, placement, &geometry);
                            let effective =
                                self.effective_satellite_geometry(satellite, &placement.geometry, &geometry, footprint, entry.config.shape, &settings.layout);
                            // The circle `Minimize` fold exception also
                            // covers the choreography window: it never
                            // closes, so it must stay rendered while the
                            // rest of the set animates into the seed.
                            let fold_control = matches!(satellite.kind, SatelliteKind::Chrome(ChromeElement::Minimize))
                                && entry.config.shape == CardShape::Circle
                                && !entry.config.unfold_on_click
                                && (fold_exception || (entry.fold_choreo_closing.get() && entry.scaling.is_visible()));
                            // A running close transition keeps its
                            // satellite visible until it lands in the seed.
                            // During the fold choreography `open` only
                            // carries the reveal intent for the unfold -
                            // without this carve-out a satellite whose
                            // transition already retired would re-render
                            // at its resolved placement (the close-end
                            // flash).
                            let transitioning = satellite.transition.borrow().is_some();
                            let vis = fold_control || (shown && (transitioning || (satellite.open.get() && !entry.fold_choreo_closing.get())));
                            satellite.scaling.set_visible(vis);
                            // Transitions also fade: open fades in, close
                            // fades out, retargets stay opaque (P5.1).
                            satellite.scaling.set_opacity(transition_opacity(satellite));
                            self.apply_satellite_placement(entry, satellite, &layer, &effective);
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
    fn apply_satellite_placement(&self, entry: &CardEntry, satellite: &SatelliteEntry, layer: &CardLayer, geometry: &CardGeometry) {
        let view = self.card_view(entry);
        let rendered_rotation = geometry.rotation.to_rotation_rad() + view.rotation;
        satellite.rotation_widget.set_rotation(SmearorRotation::from(rendered_rotation));
        let rendered_scale = geometry.scale.value() * self.card_view_zoom(entry);
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
            true => Vec2::<Pixels>::from(geometry.position),
            false => view.to_widget(geometry.position.into(), widget),
        };
        layer.move_(&satellite.scaling, center.x - rendered_size.x / 2.0, center.y - rendered_size.y / 2.0);
        satellite.last_geometry.set(Some(*geometry));
    }

    /// The placement actually applied this frame (P5.1/5.2): the live
    /// resolved placement, or - while a [`SatelliteTransition`] is in
    /// flight - the interpolation between the frozen start (a
    /// mid-flight reversal or retarget spot) / collapsed seed and the
    /// live endpoint. Because the endpoint resolves live on every sync,
    /// card gestures keep carrying an animating satellite rigidly.
    fn effective_satellite_geometry(
        &self,
        satellite: &SatelliteEntry,
        resolved: &CardGeometry,
        card_geometry: &CardGeometry,
        card_footprint: Vec2<Pixels>,
        card_shape: CardShape,
        layout: &SatelliteLayout,
    ) -> CardGeometry {
        let transition = satellite.transition.borrow();
        let Some(transition) = transition.as_ref() else {
            return *resolved;
        };
        let progress = Fraction::new(transition.animation.value().unwrap_or(0.0));
        let seed = || collapsed_seed(resolved, card_geometry, card_footprint, card_shape, layout);
        let start = transition.from.unwrap_or_else(seed);
        let end = if transition.opening { *resolved } else { seed() };
        start.interpolate(end, progress)
    }

    /// Retarget tween (P5.2): when the resolved placement moved in
    /// card-local space - a footprint or layout change, not a card
    /// gesture - the satellite freezes its rendered spot and tweens to
    /// the new target instead of jumping. Card gestures transform the
    /// resolved placement identically, so the local placement stays
    /// constant and drags remain rigidly coupled.
    fn maybe_begin_move_transition(&self, entry: &CardEntry, satellite: &SatelliteEntry, placement: &SatellitePlacement, card_geometry: &CardGeometry) {
        let local = card_local_geometry(&placement.geometry, card_geometry);
        let previous = satellite.last_local_geometry.replace(Some(local));
        // Hidden satellites (closed set, folded card) resolve retargets
        // silently in the background: a move tween is `opening`-typed
        // and would re-show the satellite mid-flight - the
        // close-end/unfold flash. Reveals go through
        // `begin_satellite_transitions` instead.
        if !satellite.open.get()
            || !satellite.scaling.is_visible()
            || satellite.transition.borrow().is_some()
            || entry.card.folded()
            || entry.fold_choreo_closing.get()
            || !self.satellite_animations_enabled(entry)
        {
            return;
        }
        let Some(previous) = previous else {
            return;
        };
        if !local_geometry_differs(&previous, &local) {
            return;
        }
        let mut animation = TimedAnimation::new(Span::new(0.0, 1.0), satellite_transition_duration(entry), EasingFunction::EaseInOut);
        animation.start();
        *satellite.transition.borrow_mut() = Some(SatelliteTransition {
            animation,
            from: satellite.last_geometry.get(),
            opening: true,
        });
        self.start_satellite_motion(entry);
    }

    /// Begins a staggered open (`opening`) or close transition batch on
    /// every satellite (P5.1): each gets a `TimedAnimation` delayed by
    /// its config index, interpolating between the frozen mid-flight
    /// spot - or the collapsed seed on first open - and the live
    /// endpoint. `skip_fold_control` excludes the circle card's
    /// `Minimize` satellite, whose fold exception keeps it visible
    /// while folded. Reduced motion and `animation_speed_ms == 0`
    /// resolve instantly instead.
    pub(crate) fn begin_satellite_transitions(&self, entry: &CardEntry, opening: bool, skip_fold_control: bool) {
        if !self.satellite_animations_enabled(entry) {
            self.sync_satellites(entry);
            return;
        }
        let satellites = entry.satellites.borrow();
        if satellites.is_empty() {
            return;
        }
        let (settings, _) = entry.config.effective_satellites();
        let items = satellite_items(&satellites);
        let geometry = self.entry_geometry(entry);
        let (_, natural) = entry.card.measure_size();
        let footprint = natural.to_f64().with_unit::<Pixels>();
        if resolve(&geometry, footprint, entry.config.shape, &settings.layout, settings.rotation_mode, &items).is_err() {
            drop(satellites);
            self.sync_satellites(entry);
            return;
        }
        let duration = satellite_transition_duration(entry);
        // The stagger spreads the starts over the first 40% of the
        // shared duration so the set reads as one choreographed motion.
        let stagger = duration.mul_f64(0.4) / satellites.len().max(1) as u32;
        let now = Instant::now();
        let mut any = false;
        for (index, satellite) in satellites.iter().enumerate() {
            let fold_protected = skip_fold_control
                && !opening
                && entry.config.shape == CardShape::Circle
                && !entry.config.unfold_on_click
                && matches!(satellite.kind, SatelliteKind::Chrome(ChromeElement::Minimize));
            if fold_protected {
                continue;
            }
            // A close on a satellite that is already logically closed
            // (hidden set) would briefly re-show it mid-tween - skip.
            // `open` is the right discriminator, not `is_visible()`:
            // the fold choreography's revert sync legitimately hides
            // the satellites a call earlier, but they must still get
            // their close transitions.
            if !opening && !satellite.open.get() && satellite.transition.borrow().is_none() {
                continue;
            }
            // `from` semantics (P5.1): only a transition already in
            // flight freezes its rendered spot (mid-flight reversal).
            // A fresh open emerges from the collapsed seed - hidden
            // satellites keep receiving resolved placements via
            // background syncs, so `last_geometry` is NOT a frozen spot.
            let from = match satellite.transition.borrow().is_some() {
                true => satellite.last_geometry.get(),
                false if !opening => satellite.last_geometry.get(),
                false => None,
            };
            let easing = match opening {
                true => EasingFunction::BackOut { overshoot_amount: 1.2 },
                false => EasingFunction::EaseIn,
            };
            let mut animation = TimedAnimation::new(Span::new(0.0, 1.0), duration, easing);
            animation.start_at(now + stagger.mul_f64(index as f64));
            *satellite.transition.borrow_mut() = Some(SatelliteTransition { animation, from, opening });
            any = true;
        }
        drop(satellites);
        if any {
            self.start_satellite_motion(entry);
        }
        self.sync_satellites(entry);
    }

    /// Whether satellite placement animations run for `entry`: GTK's
    /// `gtk-enable-animations` setting (reduced motion) and the card's
    /// own `animation_speed_ms` budget both have to allow it.
    fn satellite_animations_enabled(&self, entry: &CardEntry) -> bool {
        entry.config.animation_speed_ms > 0 && gtk4::Settings::default().is_none_or(|settings| settings.is_gtk_enable_animations())
    }

    /// Starts (or restarts) the per-entry motion tick that re-syncs
    /// satellite transitions frame by frame until none are left. A new
    /// batch bumps `satellites_generation`, so a stale tick stops
    /// itself - the same pattern `layout_generation` uses for card
    /// glides. When the last transition of a fold choreography lands,
    /// the tick applies the reverted fold through `set_folded`, whose
    /// `card-folded` handler performs the final sync.
    pub(crate) fn start_satellite_motion(&self, entry: &CardEntry) {
        let generation = entry.satellites_generation.get().incremented();
        entry.satellites_generation.set(generation);
        let card_id = entry.card.id();
        self.add_tick_callback(move |board, _frame_clock| {
            enum Next {
                Running,
                Done,
                ApplyFold,
            }
            let next = {
                let cards = board.imp().cards.borrow();
                let Some(entry) = cards.get(&card_id) else {
                    return ControlFlow::Break;
                };
                if entry.satellites_generation.get() != generation {
                    return ControlFlow::Break;
                }
                for satellite in entry.satellites.borrow().iter() {
                    let finished = satellite
                        .transition
                        .borrow()
                        .as_ref()
                        .is_some_and(|transition| transition.animation.is_complete());
                    if finished {
                        satellite.transition.take();
                    }
                }
                if entry.satellites.borrow().iter().any(|satellite| satellite.transition.borrow().is_some()) {
                    board.sync_satellites(entry);
                    Next::Running
                } else if entry.fold_choreo_closing.get() {
                    // Fold choreography finalization happens through
                    // `set_folded`, which syncs itself - syncing here
                    // first would pop the satellites back to their open
                    // placements for a frame.
                    Next::ApplyFold
                } else {
                    board.sync_satellites(entry);
                    Next::Done
                }
            };
            if matches!(next, Next::ApplyFold) {
                let card = {
                    let cards = board.imp().cards.borrow();
                    cards.get(&card_id).map(|entry| entry.card.clone())
                };
                if let Some(card) = card {
                    card.set_folded(true);
                }
                return ControlFlow::Break;
            }
            match next {
                Next::Running => ControlFlow::Continue,
                Next::Done | Next::ApplyFold => ControlFlow::Break,
            }
        });
    }

    /// `card-folded` reaction (P5.4): a real fold first *reverts* the
    /// instant collapse - same frame, never rendered - animates the
    /// satellite set into the collapsed seed, and the motion tick
    /// applies the actual fold once the last transition lands. Unfolds
    /// replay the open transition. `fold_choreo_closing` marks the
    /// choreography's own intermediate states so it does not retrigger
    /// itself.
    pub(crate) fn on_card_folded(&self, entry: &CardEntry) {
        if entry.card.folded() {
            if entry.fold_choreo_closing.replace(false) {
                // The choreography's own delayed fold just landed.
                self.sync_satellites(entry);
                return;
            }
            let closeable = {
                let satellites = entry.satellites.borrow();
                satellites.iter().any(|satellite| {
                    satellite.open.get()
                        && !(entry.config.shape == CardShape::Circle
                            && !entry.config.unfold_on_click
                            && matches!(satellite.kind, SatelliteKind::Chrome(ChromeElement::Minimize)))
                })
            };
            if closeable && self.satellite_animations_enabled(entry) {
                entry.fold_choreo_closing.set(true);
                // Re-enters this handler with `folded == false` and the
                // choreography flag set - the branch below starts the
                // close transitions.
                entry.card.set_folded(false);
                return;
            }
            self.sync_satellites(entry);
            return;
        }
        if entry.fold_choreo_closing.get() {
            // The choreography's own revert - the card stands unfolded
            // again while the set animates closed; the tick re-folds at
            // the end even when no transition was actually created.
            self.begin_satellite_transitions(entry, false, true);
            self.start_satellite_motion(entry);
            return;
        }
        // A genuine unfold reveals the satellite set again.
        let open = entry.satellites.borrow().iter().any(|satellite| satellite.open.get());
        if open && self.satellite_animations_enabled(entry) {
            self.begin_satellite_transitions(entry, true, false);
            return;
        }
        self.sync_satellites(entry);
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

/// The rendered opacity during a transition: a fresh open (no frozen
/// start - it emerges from the seed) fades in, a close fades out.
/// Retargets and mid-flight reversals keep a frozen start and stay
/// opaque; steady state is fully opaque.
fn transition_opacity(satellite: &SatelliteEntry) -> f64 {
    let transition = satellite.transition.borrow();
    let Some(transition) = transition.as_ref() else {
        return 1.0;
    };
    let progress = transition.animation.value().unwrap_or(0.0);
    match (transition.opening, transition.from.is_some()) {
        (true, false) => progress,
        (true, true) => 1.0,
        (false, _) => 1.0 - progress,
    }
    .clamp(0.0, 1.0)
}

/// Live resolver input for the mounted satellites: ids, current natural
/// sizes and anchored angles.
fn satellite_items(satellites: &[SatelliteEntry]) -> Vec<SatelliteItem> {
    satellites
        .iter()
        .map(|satellite| SatelliteItem {
            id: satellite.id.clone(),
            size: satellite.widget.measure_size().1.to_f64().with_unit::<Pixels>(),
            anchor: satellite.anchor,
        })
        .collect()
}

/// The transition duration the card's `animation_speed_ms` budget
/// allows - clamped to a nonzero duration for `TimedAnimation`.
fn satellite_transition_duration(entry: &CardEntry) -> Duration {
    Duration::from_millis(entry.config.animation_speed_ms.max(1))
}

/// Expresses a resolved workspace placement in the card's local frame:
/// the position offset unrotated and unscaled, rotation and scale
/// relative to the card. Card gestures transform the resolved placement
/// identically, so a *changed* local placement means the placement
/// itself retargeted (footprint or layout change).
fn card_local_geometry(placement: &CardGeometry, card_geometry: &CardGeometry) -> CardGeometry {
    let center = Vec2::<Pixels>::from(card_geometry.position);
    let offset = Vec2::<Pixels>::from(placement.position) - center;
    let scale = card_geometry.scale.value().max(1e-9);
    CardGeometry {
        position: CardPosition::new(offset.rotated(-card_geometry.rotation.to_rotation_rad()) / scale),
        rotation: Rotation::quantized(placement.rotation.to_rotation_rad() - card_geometry.rotation.to_rotation_rad()),
        scale: ScaleFactor::new(placement.scale.value() / scale),
    }
}

/// Whether two card-local placements differ beyond a deadband - small
/// measure jitter must not retarget a tween.
fn local_geometry_differs(previous: &CardGeometry, current: &CardGeometry) -> bool {
    let position_delta = (Vec2::<Pixels>::from(previous.position) - Vec2::<Pixels>::from(current.position)).magnitude_f64();
    let rotation_delta = (previous.rotation.to_rotation_rad() - current.rotation.to_rotation_rad()).abs().to_degrees();
    let scale_delta = (previous.scale.value() - current.scale.value()).abs();
    position_delta > 2.0 || rotation_delta > 2.0 || scale_delta > 0.02
}
