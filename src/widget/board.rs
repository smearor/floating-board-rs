use crate::domain::BoardError;
use crate::domain::BoardLayout;
use crate::domain::BoundsMode;
use crate::domain::CardAnchor;
use crate::domain::CardGeometry;
use crate::domain::CardId;
use crate::domain::CardPlacement;
use crate::domain::CardPosition;
use crate::domain::GenerationCounter;
use crate::domain::ScaleFactor;
use crate::domain::TurnDirection;
use crate::widget::board_signal::BoardSignal;
use crate::widget::card::Card;
use crate::widget::card_alignment::CardAlignment;
use crate::widget::card_close_action::CardCloseAction;
use crate::widget::card_config::CardConfig;
use crate::widget::card_layer::CardLayer;
use crate::widget::css::ensure_default_css;
use crate::widget::imp::board::FloatingBoardImpl;
use crate::widget::imp::card_entry::CardEntry;
use crate::widget::imp::quad_baseline::QuadBaseline;
use crate::widget::imp::tracked_touch::TrackedTouch;
use crate::widget::scaling::ScalingWidget;
use glib::ControlFlow;
use glib::Propagation;
use glib::object::Cast;
use glib::object::IsA;
use glib::prelude::ObjectExt;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::EventControllerLegacy;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::Orientable;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::cairo::Region;
use gtk4::gdk::EventType;
use gtk4::gio::prelude::ListModelExtManual;
use gtk4::glib;
use gtk4::prelude::FixedExt;
use gtk4::prelude::GestureDragExt;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::LayoutManagerExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use smearor_commons_animation::EasingFunction;
use smearor_commons_animation::Span;
use smearor_commons_animation::TimedAnimation;
use smearor_commons_animation::drive;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::SimilarityTransform;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use smearor_commons_layout::LayoutItem;
use smearor_commons_layout::LayoutStrategy;
use smearor_commons_layout::Seat;
use smearor_commons_layout::footprint_for;
use smearor_wrot_rotation::RotationControlHandler;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::info;
use tracing::warn;

glib::wrapper! {
    /// Composite widget stacking a transparent card layer over the consumer's
    /// content. Cards can be moved, rotated and scaled independently; gestures
    /// landing outside a card fall through to the content beneath.
    ///
    /// # Signals
    ///
    /// `card-added` (`Card`), `card-removed` (`Card`), `geometry-changed`
    pub struct FloatingBoard(ObjectSubclass<FloatingBoardImpl>)
        @extends Widget, gtk4::Box,
        @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

/// Minimum live points before the quad transform applies - a single touch
/// stays a plain touch and never moves the card.
const QUAD_MIN_POINTS: usize = 2;

/// Frame-to-frame angle jump (60°) treated as a degenerate finger crossing
/// (~180° flips when points cross the centroid) rather than real rotation.
const QUAD_ANGLE_JUMP: RotationRad = RotationRad::new(std::f64::consts::PI / 3.0);

/// How long a tracked touch may stay silent before it counts as dead - its
/// `TouchEnd` was routed to a claiming gesture or lost on the wire. Real
/// resting fingers still produce sensor jitter, so they never go silent.
const QUAD_STALE_MS: u64 = 1500;

/// Denies every sequence a gesture currently tracks (`set_state` is like
/// `set_sequence_state(.., Denied)` on all handled sequences; the per-sequence
/// variant is deprecated since GTK 4.10). The gesture stops receiving updates
/// and its `end`/`cancel` handlers run.
fn deny_gesture(gesture: &impl IsA<gtk4::Gesture>) {
    gesture.set_state(EventSequenceState::Denied);
}

impl FloatingBoard {
    /// Creates a board over `content`, which becomes the bottom layer inside
    /// the board's internal overlay.
    pub fn new(content: &impl IsA<Widget>) -> Self {
        ensure_default_css();
        let board: Self = glib::Object::builder().build();
        board.set_content(content);
        board
    }

    /// Replaces the bottom layer widget.
    pub fn set_content(&self, content: &impl IsA<Widget>) {
        if let Some(overlay) = self.overlay() {
            overlay.set_child(Some(content));
        }
    }

    /// The current bottom layer widget.
    pub fn content(&self) -> Option<Widget> {
        self.overlay().and_then(|overlay| overlay.child())
    }

    /// The internal `GtkOverlay`, exposed for edge cases. Consumers should not
    /// need this for regular card management.
    pub fn overlay(&self) -> Option<Overlay> {
        self.imp().overlay.get().cloned()
    }

    /// Adds a card with an auto-generated `card-N` id and returns its id.
    pub fn add_card(&self, content: &impl IsA<Widget>, config: CardConfig) -> CardId {
        loop {
            let sequence = self.imp().sequence.get().incremented();
            let id = CardId::new(format!("card-{sequence}"));
            match self.add_card_with_id(id.clone(), content, config.clone()) {
                Ok(id) => return id,
                Err(BoardError::DuplicateCard(_)) => continue,
                Err(_) => return id,
            }
        }
    }

    /// Adds a card with a caller-chosen semantic id (e.g. `"controls"`).
    ///
    /// The id stays stable in persisted layouts, so consumers can restore
    /// geometry per semantic key.
    pub fn add_card_with_id(&self, id: impl Into<CardId>, content: &impl IsA<Widget>, config: CardConfig) -> Result<CardId, BoardError> {
        let id = id.into();
        if self.imp().cards.borrow().contains_key(&id) {
            return Err(BoardError::DuplicateCard(id));
        }
        let Some(layer) = self.card_layer() else {
            return Err(BoardError::UnknownCard(id));
        };

        let card = Card::build(&id, content.upcast_ref(), self, &config);
        let rotation_widget = RotationWidget::new(SmearorRotation::DEG_0)
            // Rotation comes from the board's own quad transform gesture;
            // smearor's built-in rotate gesture stays off.
            .with_gesture_rotation_enabled(false)
            // The card is nested inside the board's `Fixed` layer - the
            // surface-global input-region clamp would kill input for the
            // rest of the window (`smearor-wrot-rotation` >= 0.2.1).
            .with_input_region_enabled(false)
            // Allocate the card its natural size at any angle - the legacy
            // inverse-solve shrinks non-square children near 45°.
            .with_natural_child_size_enabled(true);
        rotation_widget.set_animation_speed(config.animation_speed_ms);
        rotation_widget.set_child(Some(&card));
        let scaling = ScalingWidget::new(&rotation_widget);
        layer.put(&scaling, 0.0, 0.0);

        let sequence = self.imp().sequence.replace(self.imp().sequence.get().incremented());
        let (initial_position, pending_anchor) = match config.position {
            CardAnchor::Explicit(position) => (position, None),
            anchor => (CardPosition::default(), Some(anchor)),
        };
        let entry = CardEntry {
            card: card.clone(),
            scaling: scaling.clone(),
            rotation_widget: rotation_widget.clone(),
            config: config.clone(),
            position: Cell::new(initial_position),
            scale: Cell::new(ScaleFactor::default()),
            pending_anchor: RefCell::new(pending_anchor),
            drag_start: Cell::new(CardPosition::default()),
            drag_grab: Cell::new(None),
            resize_grab: Cell::new(None),
            resize_scale: Cell::new(ScaleFactor::default()),
            rotate_grab: Cell::new(None),
            rotate_stepped: Cell::new(false),
            quad_touches: RefCell::new(Vec::new()),
            quad_baseline: RefCell::new(None),
            claimed: RefCell::new(HashMap::new()),
            watched: RefCell::new(HashSet::new()),
            sequence,
            seat: Cell::new(config.seat),
            layout_generation: Cell::new(GenerationCounter::default()),
        };
        self.imp().cards.borrow_mut().insert(id.clone(), entry);

        // Place the stack immediately for explicit anchors; named anchors
        // resolve at the first board allocation.
        if pending_anchor.is_none() {
            let cards = self.imp().cards.borrow();
            if let Some(entry) = cards.get(&id) {
                self.apply_position(entry);
            }
        }

        self.wire_gestures(&id, &card, &scaling, &rotation_widget, &config);
        if config.z_index.is_some() {
            self.restack();
        }

        debug!(card = %id, "card added");
        self.emit_by_name::<()>(BoardSignal::CardAdded.name(), &[&card]);
        Ok(id)
    }

    /// Performs the card's configured close action: removes it
    /// (`CardCloseAction::Remove`, the default) or hides it
    /// (`CardCloseAction::Hide`) - the programmatic counterpart of the
    /// header's close button, independent of `CardConfig::closable`.
    pub fn close_card(&self, id: &CardId) -> Result<(), BoardError> {
        let close_action = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            entry.config.close_action
        };
        match close_action {
            CardCloseAction::Remove => self.remove_card(id),
            CardCloseAction::Hide => self.set_card_visible(id, false),
        }
    }

    /// Removes a card from the board.
    pub fn remove_card(&self, id: &CardId) -> Result<(), BoardError> {
        let entry = self.imp().cards.borrow_mut().remove(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
        if let Some(layer) = self.card_layer() {
            layer.remove(&entry.scaling);
        }
        debug!(card = %id, "card removed");
        self.emit_by_name::<()>(BoardSignal::CardRemoved.name(), &[&entry.card]);
        Ok(())
    }

    /// Looks up a card by id.
    pub fn card(&self, id: &CardId) -> Result<Card, BoardError> {
        self.imp()
            .cards
            .borrow()
            .get(id)
            .map(|entry| entry.card.clone())
            .ok_or_else(|| BoardError::UnknownCard(id.clone()))
    }

    /// All cards currently on the board, ordered by id.
    pub fn cards(&self) -> Vec<Card> {
        self.imp().cards.borrow().values().map(|entry| entry.card.clone()).collect()
    }

    /// The card's alignment state - model position vs stored layer
    /// placement vs rendered bounds. Diagnostics API for
    /// input-region coverage checks: [`CardAlignment::drift`] is the
    /// violation of the invariant `rendered.center() == position`
    /// and stays `None` while the card does not render.
    pub fn card_alignment(&self, card: &Card) -> CardAlignment {
        let position = card.geometry().position;
        let rendered = card.rendered_bounds();
        CardAlignment {
            card_id: card.id(),
            position,
            placed: card.placed_position(),
            rendered,
            drift: rendered.map(|bounds| bounds.center() - position.0),
        }
    }

    /// Snapshot of every card's geometry for consumer-side persistence.
    pub fn layout_snapshot(&self) -> BoardLayout {
        let cards = self.imp().cards.borrow();
        BoardLayout::new(
            cards
                .iter()
                .map(|(id, entry)| CardPlacement::new(id.clone(), self.entry_geometry(entry), entry.card.folded()).with_seat(entry.seat.get()))
                .collect(),
        )
    }

    /// Restores a previously snapshotted layout.
    ///
    /// Unknown card ids are skipped (logged, not an error); geometry is
    /// clamped to the current board bounds so nothing restores off-screen.
    pub fn restore_layout(&self, layout: &BoardLayout) {
        for placement in &layout.cards {
            let (card, rotation_widget, scaling) = {
                let cards = self.imp().cards.borrow();
                match cards.get(&placement.id) {
                    Some(entry) => {
                        entry.position.set(placement.geometry.position);
                        entry.scale.set(placement.geometry.scale.clamped(&entry.config.scale_limits));
                        entry.pending_anchor.borrow_mut().take();
                        (entry.card.clone(), entry.rotation_widget.clone(), entry.scaling.clone())
                    }
                    None => {
                        warn!(card = %placement.id, "skipping unknown card in layout restore");
                        continue;
                    }
                }
            };
            // Notify handlers re-borrow the card map, so widget calls happen
            // outside of it.
            rotation_widget.set_rotation(SmearorRotation(placement.geometry.rotation));
            scaling.set_scale(placement.geometry.scale.value());
            card.set_folded(placement.folded);
            let _ = self.set_card_seat(&placement.id, placement.seat);
            let cards = self.imp().cards.borrow();
            if let Some(entry) = cards.get(&placement.id) {
                self.apply_position(entry);
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
    }

    /// Arranges cards with an `smearor-commons-layout` strategy and
    /// applies the result in one shot (unanimated).
    ///
    /// `ids` selects which cards participate: `Some(&[CardId])`
    /// arranges just those (every id must exist), `None` arranges all
    /// currently visible cards in insertion order. Each card becomes a
    /// [`LayoutItem`] carrying its displayed footprint, its live
    /// rotation and the seat from `CardConfig::seat`; the returned
    /// placements are applied through the regular geometry path, so
    /// bounds clamping, scale limits and `geometry-changed` behave
    /// exactly like a manual manipulation.
    ///
    /// `Placement::size` is the target footprint - cards keep their
    /// configured scale, only position and rotation are applied.
    ///
    /// `animated` glides each card on the frame clock towards its
    /// placement (`EaseInOut`, duration from
    /// `CardConfig::animation_speed_ms`); `false` applies immediately.
    /// A new call supersedes animations still running on the same
    /// card.
    ///
    /// Returns [`BoardError::Layout`] when the strategy reports that
    /// the cards do not fit; the board is left untouched then.
    pub fn apply_layout(&self, strategy: &dyn LayoutStrategy<Pixels>, ids: Option<&[CardId]>, animated: bool) -> Result<(), BoardError> {
        let mut selected: Vec<(CardId, ScaleFactor, LayoutItem<Pixels>)> = Vec::new();
        {
            let cards = self.imp().cards.borrow();
            match ids {
                Some(ids) => {
                    for id in ids {
                        let Some(entry) = cards.get(id) else {
                            return Err(BoardError::UnknownCard(id.clone()));
                        };
                        selected.push((id.clone(), entry.scale.get(), self.layout_item(entry)));
                    }
                }
                None => {
                    let mut visible: Vec<(&CardId, &CardEntry)> = cards.iter().filter(|(_, entry)| entry.scaling.is_visible()).collect();
                    visible.sort_by_key(|(_, entry)| entry.sequence);
                    for (id, entry) in visible {
                        selected.push((id.clone(), entry.scale.get(), self.layout_item(entry)));
                    }
                }
            }
        }

        let items: Vec<LayoutItem<Pixels>> = selected.iter().map(|(_, _, item)| *item).collect();
        let placements = strategy.arrange(self.board_bounds(), &items)?;

        for ((id, scale, _), placement) in selected.iter().zip(placements.iter()) {
            // The placement expresses the target footprint; the card's
            // configured scale is preserved.
            let target = CardGeometry {
                position: CardPosition::from(placement.center),
                rotation: placement.rotation,
                scale: *scale,
            };
            if animated {
                self.glide_card_geometry(id, target);
            } else {
                self.set_card_geometry(id, &target);
            }
        }
        Ok(())
    }

    /// Passes the card to the next user's seat around the table.
    ///
    /// The starting seat is the card's explicit `seat` assignment -
    /// falling back to `Seat::nearest(rotation)` for unassigned cards -
    /// stepped once in `direction` (the `offset` along the edge is
    /// preserved). The card glides there animated: it rotates to the
    /// target side's reading orientation and, with `move_to_edge`
    /// (the common case), slides to `anchor_margin` in front of that
    /// edge. With `move_to_edge = false` it only rotates in place.
    ///
    /// An explicitly assigned seat is advanced and persisted
    /// (`card-seat-changed`); unassigned cards stay unassigned - the
    /// next pass derives their seat from the rotation again.
    ///
    /// Returns [`BoardError::UnknownCard`] for an id not on the board.
    pub fn rotate_to_next_user(&self, id: &CardId, direction: TurnDirection, move_to_edge: bool) -> Result<(), BoardError> {
        let (geometry, footprint, target_seat, had_seat, margin) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            let geometry = self.entry_geometry(entry);
            let seat = entry.seat.get().unwrap_or_else(|| Seat::nearest(geometry.rotation.to_rotation_rad()));
            let target_seat = match direction {
                TurnDirection::Clockwise => seat.clockwise(),
                TurnDirection::CounterClockwise => seat.counter_clockwise(),
            };
            // The footprint is expressed at the target orientation, so
            // the placement centers the *rotated* bounding box.
            let turn = target_seat.side.to_quarter_turn();
            (
                geometry,
                footprint_for(self.displayed_size(entry), geometry.rotation, turn),
                target_seat,
                entry.seat.get().is_some(),
                entry.config.anchor_margin.as_f64(),
            )
        };

        if had_seat {
            self.set_card_seat(id, Some(target_seat))?;
        }

        let bounds = self.board_bounds();
        let placement = target_seat.placement(bounds, footprint, margin);
        let target = CardGeometry {
            // Degenerate bounds (unrealized board) fall back to
            // rotation-in-place, like `clamp_entry` early-returns.
            position: if move_to_edge && !bounds.is_degenerate() {
                CardPosition::from(placement.center)
            } else {
                geometry.position
            },
            rotation: placement.rotation,
            scale: geometry.scale,
        };
        self.glide_card_geometry(id, target);
        Ok(())
    }

    /// Current geometry of a card (rotation is read live from the widget).
    pub(crate) fn card_geometry(&self, id: &CardId) -> Option<CardGeometry> {
        self.imp().cards.borrow().get(id).map(|entry| self.entry_geometry(entry))
    }

    /// Applies a full geometry update, clamped to the board bounds.
    pub(crate) fn set_card_geometry(&self, id: &CardId, geometry: &CardGeometry) {
        let (rotation_widget, scaling, scale) = {
            let cards = self.imp().cards.borrow();
            match cards.get(id) {
                Some(entry) => {
                    entry.position.set(geometry.position);
                    let scale = geometry.scale.clamped(&entry.config.scale_limits);
                    entry.scale.set(scale);
                    entry.pending_anchor.borrow_mut().take();
                    (entry.rotation_widget.clone(), entry.scaling.clone(), scale)
                }
                None => return,
            }
        };
        rotation_widget.set_rotation(SmearorRotation(geometry.rotation));
        scaling.set_scale(scale.value());
        let cards = self.imp().cards.borrow();
        if let Some(entry) = cards.get(id) {
            self.apply_position(entry);
            self.clamp_entry(entry);
        }
        drop(cards);
        self.emit_geometry_changed();
    }

    /// Sets the card rotation immediately (no animation).
    pub(crate) fn set_card_rotation(&self, id: &CardId, rotation: Rotation) {
        let rotation_widget = self.imp().cards.borrow().get(id).map(|entry| entry.rotation_widget.clone());
        if let Some(rotation_widget) = rotation_widget {
            // The notify::rotation handler performs re-centering, clamping and
            // signal emission.
            rotation_widget.set_rotation(SmearorRotation(rotation));
        }
    }

    /// Sets the card rotation with the zoom-snap animation.
    pub(crate) fn set_card_rotation_animated(&self, id: &CardId, rotation: Rotation) {
        let rotation_widget = self.imp().cards.borrow().get(id).map(|entry| entry.rotation_widget.clone());
        if let Some(rotation_widget) = rotation_widget {
            rotation_widget.set_rotation_with_animation(rotation.to_rotation_rad());
        }
    }

    /// Sets the card scale, clamped to its `scale_limits`.
    pub(crate) fn set_card_scale(&self, id: &CardId, scale: ScaleFactor) {
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let scale = scale.clamped(&entry.config.scale_limits);
            entry.scale.set(scale);
            entry.scaling.set_scale(scale.value());
            self.apply_position(entry);
            self.clamp_entry(entry);
            entry.card.clone()
        };
        card.emit_card_scaled();
        self.emit_geometry_changed();
    }

    /// Resets a card to its configured initial geometry: the `position`
    /// anchor re-resolved against the current bounds, `Deg0` rotation and
    /// scale `1.0`.
    ///
    /// Rotation goes through the widget first (the `notify::rotation` handler
    /// applies the interim state); the anchor then resolves against the
    /// reset footprint.
    pub fn reset_card(&self, id: &CardId) -> Result<(), BoardError> {
        let (rotation_widget, scaling) = {
            let cards = self.imp().cards.borrow();
            let entry = cards.get(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
            entry.scale.set(ScaleFactor::default());
            (entry.rotation_widget.clone(), entry.scaling.clone())
        };
        rotation_widget.set_rotation(SmearorRotation::DEG_0);
        scaling.set_scale(1.0);
        {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Ok(());
            };
            match entry.config.position {
                CardAnchor::Explicit(position) => {
                    entry.pending_anchor.borrow_mut().take();
                    entry.position.set(position);
                }
                anchor => {
                    let bounds = self.board_bounds();
                    if bounds.is_degenerate() {
                        entry.pending_anchor.replace(Some(anchor));
                    } else {
                        entry.pending_anchor.borrow_mut().take();
                        let footprint = self.predicted_size(entry);
                        entry.position.set(anchor.resolve(&bounds, footprint, entry.config.anchor_margin));
                    }
                }
            }
            self.apply_position(entry);
            self.clamp_entry(entry);
        }
        self.emit_geometry_changed();
        Ok(())
    }

    /// Moves the card to the top of the card layer.
    pub(crate) fn raise_card(&self, id: &CardId) {
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(layer) = self.card_layer() else {
                return;
            };
            let Some(entry) = cards.get(id) else {
                return;
            };
            let top_left = self.displayed_footprint(entry).min;
            layer.remove(&entry.scaling);
            layer.put(&entry.scaling, top_left.x, top_left.y);
            entry.card.clone()
        };
        card.emit_card_raised();
    }

    /// Re-centers every card on its stored position. Called from the card
    /// layer's `size_allocate`: child measures are final at that point, so
    /// cards whose natural size or rotation changed since the last `put`
    /// get re-positioned in the same pass - `apply_position` no-ops on
    /// cards that already sit at their target spot.
    pub(crate) fn reconcile_card_positions(&self) {
        let cards = self.imp().cards.borrow();
        for entry in cards.values() {
            self.apply_position(entry);
        }
    }

    /// Re-centers the card on its stored position and re-clamps it (called
    /// after size changes like folding).
    pub(crate) fn recenter_card(&self, id: &CardId) {
        let cards = self.imp().cards.borrow();
        if let Some(entry) = cards.get(id) {
            self.apply_position(entry);
            self.clamp_entry(entry);
        }
    }

    /// Rotated footprint of a card in card-layer (board) coordinates - for
    /// safe-area unions and input regions.
    pub fn card_footprint(&self, id: &CardId) -> Result<Region, BoardError> {
        let cards = self.imp().cards.borrow();
        let entry = cards.get(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
        Ok(self.entry_footprint(entry))
    }

    /// Union of every card's rotated footprint in card-layer (board)
    /// coordinates. Consumers restricting the surface input region to
    /// interactive chrome - e.g. touch-passthrough overlays - union this
    /// with their own regions after translating to surface coordinates.
    pub fn cards_region(&self) -> Region {
        let region = Region::create();
        for entry in self.imp().cards.borrow().values() {
            // Hidden cards keep their geometry but contribute no input
            // footprint - a stale rect would dead-zone the canvas.
            if !entry.scaling.is_visible() {
                continue;
            }
            if let Err(error) = region.union(&self.entry_footprint(entry)) {
                warn!(%error, "failed to union card footprint");
            }
        }
        region
    }

    /// Shows or hides the card's whole transform stack. A hidden card keeps
    /// its geometry and entry, drops out of [`Self::cards_region`] and can be
    /// re-shown here - the menu-driven counterpart of [`Self::remove_card`].
    /// Emits `card-visibility-changed` with the card.
    pub fn set_card_visible(&self, id: &CardId, visible: bool) -> Result<(), BoardError> {
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            if entry.scaling.is_visible() == visible {
                return Ok(());
            }
            entry.scaling.set_visible(visible);
            entry.card.clone()
        };
        self.emit_by_name::<()>(BoardSignal::CardVisibilityChanged.name(), &[&card]);
        self.emit_geometry_changed();
        Ok(())
    }

    /// The seat the card is currently assigned to, if any - the
    /// explicit `CardConfig::seat` or the seat it was passed to.
    /// Independent of the card's visual rotation.
    pub fn card_seat(&self, id: &CardId) -> Option<Seat> {
        self.imp().cards.borrow().get(id).and_then(|entry| entry.seat.get())
    }

    /// Assigns the card to a seat (or clears the assignment with
    /// `None`). Emits `card-seat-changed` on the card when the
    /// assignment differs; position and rotation are untouched.
    pub fn set_card_seat(&self, id: &CardId, seat: Option<Seat>) -> Result<(), BoardError> {
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            if entry.seat.get() == seat {
                return Ok(());
            }
            entry.seat.set(seat);
            entry.card.clone()
        };
        card.emit_card_seat_changed();
        Ok(())
    }

    /// Whether the card's transform stack is currently shown.
    pub fn is_card_visible(&self, id: &CardId) -> Result<bool, BoardError> {
        let cards = self.imp().cards.borrow();
        let Some(entry) = cards.get(id) else {
            return Err(BoardError::UnknownCard(id.clone()));
        };
        Ok(entry.scaling.is_visible())
    }

    /// Rotated footprint rasterized from the card's natural size, scale and
    /// rotation, centered on its board position.
    fn entry_footprint(&self, entry: &CardEntry) -> Region {
        let (_, natural) = entry.card.measure_size();
        let scale = entry.scale.get().value();
        let rotation = entry.rotation_widget.rotation();
        let footprint = Rect::<Pixels>::centered(entry.position.get().into(), natural.to_f64().with_unit::<Pixels>() * scale);
        footprint.rotated_region(rotation).unwrap_or_else(|error| {
            warn!(%error, "failed to build card footprint region");
            Region::create()
        })
    }

    /// Whether an external consumer manages the surface input region.
    pub fn input_region_managed(&self) -> bool {
        self.imp().input_region_managed.get()
    }

    /// Hands the surface input region to an external consumer: the board
    /// stops resetting it on every allocation (its default defense against
    /// `RotationWidget`'s own region narrowing). Consumers computing custom
    /// input regions - e.g. restricting input to cards plus chrome in
    /// touch-passthrough overlays - must set this so their region survives
    /// the next layout pass.
    pub fn set_input_region_managed(&self, managed: bool) {
        self.imp().input_region_managed.set(managed);
    }

    /// Called from `WidgetImpl::size_allocate`: resolves pending anchors and
    /// re-clamps all cards against the new bounds.
    pub(crate) fn on_board_allocated(&self) {
        let bounds = self.board_bounds();
        if bounds.is_degenerate() {
            return;
        }
        let mut resolved = false;
        {
            let cards = self.imp().cards.borrow();
            for entry in cards.values() {
                if let Some(anchor) = entry.pending_anchor.borrow_mut().take() {
                    let footprint = self.predicted_size(entry);
                    entry.position.set(anchor.resolve(&bounds, footprint, entry.config.anchor_margin));
                    resolved = true;
                }
                // Reconcile render state with position state on every
                // allocation: `gtk4::Fixed` positions are only ever moved via
                // apply_position, so cards added before the first allocation
                // would otherwise stay at (0,0).
                self.apply_position(entry);
            }
        }
        self.clamp_all();
        if resolved {
            self.emit_geometry_changed();
        }
    }

    /// `notify::rotation` on the card's `RotatedLayout`: re-centers the card
    /// (the rotated bounding box changes size), clamps and emits signals.
    fn on_card_rotated(&self, id: &CardId) {
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            // smearor-wrot-rotation queues allocation rather than resize, so
            // the RotatedLayout measure would stay stale; `layout_changed`
            // invalidates the measure chain and a 90 degree turn correctly
            // swaps the child's width/height.
            if let Some(manager) = entry.rotation_widget.layout_manager() {
                manager.layout_changed();
            }
            entry.scaling.queue_resize();
            self.apply_position(entry);
            self.clamp_entry(entry);
            entry.card.clone()
        };
        card.emit_card_rotated();
        self.emit_geometry_changed();
    }

    /// Slot activation at sequence begin: the quad drags are `exclusive`,
    /// so GTK's own `GtkGestureSingle` distribution hands each slot a
    /// different touch sequence and `begin` only fires for the sequence this
    /// slot actually owns. The slot records the finger's board-space
    /// position at touch-down, not only once GTK recognizes a drag.
    /// Routes a raw touch event from the quad tracker: `TouchBegin`
    /// registers the sequence as a live point, `TouchUpdate` moves it and
    /// feeds the similarity transform, `TouchEnd`/`TouchCancel` removes it.
    fn on_quad_touch_event(
        &self,
        id: &CardId,
        controller: &gtk4::EventControllerLegacy,
        event: &gtk4::gdk::Event,
        card_drag: Option<&gtk4::GestureDrag>,
        click: &gtk4::GestureClick,
    ) {
        let event_type = event.event_type();
        if !matches!(event_type, EventType::TouchBegin | EventType::TouchUpdate | EventType::TouchEnd | EventType::TouchCancel) {
            return;
        }
        // `event_sequence` is a plain (non-Option) binding - calling it on
        // sequence-less events asserts on a NULL pointer, so it may only run
        // after the touch-type check above.
        self.sweep_quad_touches(id);
        let sequence = event.event_sequence();
        match event_type {
            EventType::TouchBegin => {
                // Diagnostic: two TouchBegins for one physical contact
                // (slot + emulated/pointer dupe) would look identical
                // here - sequence address + device distinguishes them.
                info!(card = %id, sequence = ?sequence, "quad touch begin");
                if let Some(point) = self.event_point_in_layer(controller, event) {
                    let cards = self.imp().cards.borrow();
                    if let Some(entry) = cards.get(id) {
                        self.activate_quad_touch(entry, sequence, point, card_drag, click);
                    }
                }
                self.emit_geometry_changed();
            }
            EventType::TouchUpdate => {
                let Some(point) = self.event_point_in_layer(controller, event) else {
                    return;
                };
                {
                    let cards = self.imp().cards.borrow();
                    let Some(entry) = cards.get(id) else {
                        return;
                    };
                    let tracked = {
                        let mut touches = entry.quad_touches.borrow_mut();
                        let Some(tracked) = touches.iter_mut().find(|touch| touch.sequence == sequence) else {
                            return;
                        };
                        tracked.point = point;
                        tracked.last_event = Instant::now();
                        touches.len()
                    };
                    debug!(card = %id, tracked, point = ?point, "quad touch update");
                    self.apply_quad_transform(entry);
                }
                self.emit_geometry_changed();
            }
            EventType::TouchEnd | EventType::TouchCancel => {
                self.release_quad_touch(id, &sequence);
            }
            _ => {}
        }
    }

    /// Attaches a `sequence-state-changed` watcher to every `Gesture`
    /// controller in the card subtree that does not have one yet. The
    /// watcher marks claimed sequences on the entry - the mark
    /// survives the gesture's own reset, which a `sequence_state`
    /// query would not. Walked lazily on each sweep so gestures
    /// children add at runtime are covered too.
    fn watch_claims(&self, entry: &CardEntry) {
        let root = entry.scaling.clone().upcast::<gtk4::Widget>();
        let card_id = entry.card.id();
        let mut stack = vec![root];
        while let Some(widget) = stack.pop() {
            for controller in widget.observe_controllers().iter::<glib::Object>() {
                let Ok(controller) = controller else {
                    continue;
                };
                let Some(gesture) = controller.downcast_ref::<gtk4::Gesture>() else {
                    continue;
                };
                // The wrapper reference is a temporary - the stable
                // identity is the underlying GObject pointer.
                let address = glib::translate::ToGlibPtr::<*mut gtk4::ffi::GtkGesture>::to_glib_none(&gesture).0 as usize;
                if !entry.watched.borrow_mut().insert(address) {
                    continue;
                }
                let board_weak = self.downgrade();
                let card_id = card_id.clone();
                gesture.connect_sequence_state_changed(move |_, sequence, state| {
                    if state != EventSequenceState::Claimed {
                        return;
                    }
                    if let (Some(board), Some(sequence)) = (board_weak.upgrade(), sequence) {
                        let cards = board.imp().cards.borrow();
                        if let Some(entry) = cards.get(&card_id) {
                            entry.claimed.borrow_mut().insert(sequence.clone(), Instant::now());
                        }
                    }
                });
            }
            let mut child = widget.first_child();
            while let Some(ref sibling) = child {
                stack.push(sibling.clone());
                child = sibling.next_sibling();
            }
        }
    }

    /// Drops tracked touches that are no longer live. Two cases:
    /// a touch *claimed* by another gesture (its remaining events are routed
    /// to the claimer only) and a touch that went *silent* - its `TouchEnd`
    /// already went to a claimer or was lost on the wire, so only the event
    /// timestamp reveals it is dead. Without the sweep a dead touch lingers
    /// as a phantom point, keeps the transform "active" for a single new
    /// finger and distorts radius/angle. Claims are checked against every
    /// gesture in the card subtree - the board's own click/drag as well as
    /// child widgets (buttons, toggles), even ones added at runtime.
    fn sweep_quad_touches(&self, id: &CardId) {
        let pruned = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            self.watch_claims(entry);
            // Slot ids recycle: a mark older than the stale window can
            // only belong to a press that already ended - expire it so
            // the next touch on the same slot is not shadowed.
            entry.claimed.borrow_mut().retain(|_, ts| ts.elapsed() < Duration::from_millis(QUAD_STALE_MS));
            let pruned = {
                let mut touches = entry.quad_touches.borrow_mut();
                let claimed = entry.claimed.borrow();
                let before = touches.len();
                touches.retain(|touch| touch.last_event.elapsed() < Duration::from_millis(QUAD_STALE_MS) && !claimed.contains_key(&touch.sequence));
                before - touches.len()
            };
            if pruned > 0 {
                info!(card = %id, pruned, "quad touches pruned (claimed or stale)");
                if entry.quad_touches.borrow().len() >= QUAD_MIN_POINTS {
                    self.refresh_quad_baseline(entry);
                } else {
                    entry.quad_baseline.borrow_mut().take();
                }
            }
            pruned
        };
        if pruned > 0 {
            self.emit_geometry_changed();
        }
    }

    /// Drops a quad touch: re-baselines for the remaining fingers or
    /// finishes the transform and emits the collected signals.
    fn release_quad_touch(&self, id: &CardId, sequence: &gtk4::gdk::EventSequence) {
        let (card, moved, rotated, scaled) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let active = {
                let mut touches = entry.quad_touches.borrow_mut();
                let Some(index) = touches.iter().position(|touch| touch.sequence == *sequence) else {
                    return;
                };
                touches.remove(index);
                entry.claimed.borrow_mut().remove(sequence);
                touches.len()
            };
            info!(card = %id, active, "quad touch released");
            if active >= QUAD_MIN_POINTS {
                // A finger left while the transform is still running: snap a
                // fresh baseline so the remaining fingers continue smoothly.
                self.refresh_quad_baseline(entry);
                (entry.card.clone(), false, false, false)
            } else {
                let baseline = entry.quad_baseline.borrow_mut().take();
                self.clamp_entry(entry);
                let flags = baseline.map(|b| (b.moved, b.rotated, b.scaled)).unwrap_or_default();
                (entry.card.clone(), flags.0, flags.1, flags.2)
            }
        };
        if moved {
            card.emit_card_moved();
        }
        if rotated {
            card.emit_card_rotated();
        }
        if scaled {
            card.emit_card_scaled();
        }
        self.emit_geometry_changed();
    }

    /// Activates a quad touch: records the finger's board-space position,
    /// denies the card's own drag once the transform threshold is reached
    /// (no double-move while the quad transform runs) and (re-)takes the
    /// baseline snapshot.
    fn activate_quad_touch(
        &self,
        entry: &CardEntry,
        sequence: gtk4::gdk::EventSequence,
        point: Vec2<Pixels>,
        card_drag: Option<&gtk4::GestureDrag>,
        click: &gtk4::GestureClick,
    ) {
        if entry.claimed.borrow().contains_key(&sequence) {
            debug!(point = ?point, "quad touch skipped - sequence claimed");
            return;
        }
        let active = {
            let mut touches = entry.quad_touches.borrow_mut();
            touches.push(TrackedTouch {
                sequence,
                point,
                last_event: Instant::now(),
            });
            touches.len()
        };
        info!(active, point = ?point, "quad touch activated");
        if active == QUAD_MIN_POINTS {
            info!("quad transform active - denying card drag and click");
            if let Some(card_drag) = card_drag {
                deny_gesture(card_drag);
            }
            // GestureClick claims every sequence on press; the claim
            // steals the first touch's remaining events and makes the
            // sweep prune it, so the tracker could never
            // reach two live points. Denying releases the claims - both
            // sequences stream to the tracker again (raise-on-release
            // is suppressed mid-gesture).
            deny_gesture(click);
        }
        if active >= QUAD_MIN_POINTS {
            self.refresh_quad_baseline(entry);
        }
    }

    /// Touch position of a raw event in board coordinates - delegates to
    /// the layer, which owns the target coordinate space.
    fn event_point_in_layer(&self, controller: &EventControllerLegacy, event: &gtk4::gdk::Event) -> Option<Vec2<Pixels>> {
        self.card_layer()?.event_pointer(controller, event)
    }

    /// Takes a fresh baseline: current geometry plus the live quad points.
    /// Called on activation and whenever the point count changes, so adding
    /// or lifting fingers never produces a jump.
    fn refresh_quad_baseline(&self, entry: &CardEntry) {
        let points: Vec<Vec2<Pixels>> = entry.quad_touches.borrow().iter().map(|touch| touch.point).collect();
        let previous = entry.quad_baseline.borrow_mut().take();
        if points.len() < QUAD_MIN_POINTS {
            return;
        }
        let (moved, rotated, scaled) = previous.map(|b| (b.moved, b.rotated, b.scaled)).unwrap_or_default();
        info!(points = ?points, position = ?entry.position.get(), scale = entry.scale.get().value(), rotation_deg = entry.rotation_widget.rotation().to_degrees(), "quad baseline snapshot");
        entry.quad_baseline.replace(Some(QuadBaseline {
            position: entry.position.get(),
            scale: entry.scale.get(),
            rotation: entry.rotation_widget.rotation(),
            points,
            moved,
            rotated,
            scaled,
            applied_angle: RotationRad::ZERO,
        }));
    }

    /// Applies the similarity transform fitted between the baseline point
    /// cloud and the current points: the card center follows the same
    /// rotation/scale/translation the fingers describe.
    fn apply_quad_transform(&self, entry: &CardEntry) {
        let current: Vec<Vec2<Pixels>> = entry.quad_touches.borrow().iter().map(|touch| touch.point).collect();
        let mut baseline_ref = entry.quad_baseline.borrow_mut();
        let Some(baseline) = baseline_ref.as_mut() else {
            return;
        };
        let Some(fit) = SimilarityTransform::fit(&baseline.points, &current) else {
            return;
        };
        // Finger crossings through the centroid flip the point-cloud angle
        // by ~180° in a single update - far faster than a real hand can
        // rotate. Re-baseline on such jumps instead of applying them.
        if baseline.applied_angle.signed_delta_to(fit.rotation).abs() > QUAD_ANGLE_JUMP {
            drop(baseline_ref);
            self.refresh_quad_baseline(entry);
            return;
        }
        baseline.applied_angle = fit.rotation;
        let angle = if entry.config.rotatable { fit.rotation } else { RotationRad::ZERO };
        let scale = if entry.config.scalable {
            ScaleFactor::new(baseline.scale.value() * fit.scale.as_f64()).clamped(&entry.config.scale_limits)
        } else {
            baseline.scale
        };
        debug!(
            fitted_angle_deg = fit.rotation.to_degrees(),
            fitted_scale = fit.scale.as_f64(),
            applied_angle_deg = angle.to_degrees(),
            applied_scale = scale.value(),
            centroid_now = ?fit.current_centroid,
            "quad transform computed"
        );
        let applied_ratio = scale / baseline.scale;
        if entry.config.draggable {
            // The center follows the gated/clamped transform, not the raw
            // fit, so `apply` is recomputed with the effective angle/ratio.
            let rel = baseline.position.0 - fit.baseline_centroid;
            let position = CardPosition::from(fit.current_centroid + rel.rotated(angle) * f64::from(applied_ratio));
            if position != entry.position.get() {
                baseline.moved = true;
                entry.position.set(position);
            }
        }
        if entry.config.rotatable {
            let rotation = baseline.rotation + angle;
            if rotation.signed_delta_to(entry.rotation_widget.rotation()).abs() > RotationRad::from_degrees(0.01) {
                baseline.rotated = true;
                entry.rotation_widget.set_rotation(SmearorRotation::from(rotation));
            }
        }
        if entry.config.scalable && scale != entry.scale.get() {
            baseline.scaled = true;
            entry.scale.set(scale);
            entry.scaling.set_scale(scale.value());
        }
        drop(baseline_ref);
        self.apply_position(entry);
        if entry.config.bounds_mode == BoundsMode::Contained {
            self.clamp_entry(entry);
        }
    }

    /// Records the card position and the pointer's board coordinates at drag
    /// begin; the update delta is measured between two board-space pointer
    /// samples, which stays stable while the card follows the pointer.
    fn on_card_drag_begin(&self, id: &CardId, gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.drag_start.set(entry.position.get());
            entry.drag_grab.set(self.pointer_in_layer(gesture));
        }
    }

    /// Moves the card so it follows the pointer.
    ///
    /// `GestureDrag` reports offsets in the coordinate space of the drag
    /// source - which lives *inside* the rotating/scaling transform stack and
    /// *moves* with the card. Mapping that delta with `compute_point` under
    /// the current transform creates a feedback loop (the reference frame
    /// shifts as the card tracks the pointer) that shows up as jitter and
    /// roughly half pointer speed. Instead, the pointer's board position is
    /// taken straight from the event's surface coordinates, which are
    /// independent of the widget's own transform.
    fn on_card_drag_update(&self, id: &CardId, gesture: &GestureDrag, offset: Vec2<Pixels>) {
        {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let delta = self.drag_delta(gesture, offset, entry);
            let position = entry.drag_start.get().translated(delta);
            debug!(card = %id, delta = ?delta, position = ?(position.x().as_f64(), position.y().as_f64()), "drag update");
            entry.position.set(position);
            self.apply_position(entry);
            if entry.config.bounds_mode == BoundsMode::Contained {
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
    }

    fn on_card_drag_end(&self, id: &CardId, gesture: &GestureDrag, offset: Vec2<Pixels>) {
        self.on_card_drag_update(id, gesture, offset);
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            self.clamp_entry(entry);
            entry.card.clone()
        };
        card.emit_card_moved();
        self.emit_geometry_changed();
    }

    /// Captures the pointer's board position and the current scale at
    /// resize-drag begin. The radial distance card-center to pointer is the
    /// drag's reference frame - radial distance is rotation-invariant, so a
    /// rotated card resizes identically to an unrotated one.
    fn on_card_resize_begin(&self, id: &CardId, gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.resize_grab.set(self.pointer_in_layer(gesture));
            entry.resize_scale.set(entry.scale.get());
        }
    }

    /// Scales the card around its center by the ratio between the pointer's
    /// current and grab-time radial distance. `entry.position` is never
    /// touched - the card center stays put while the footprint grows or
    /// shrinks around it.
    fn on_card_resize_update(&self, id: &CardId, gesture: &GestureDrag) {
        {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let Some(scale) = self.resize_scale_for(gesture, entry) else {
                return;
            };
            debug!(card = %id, scale = scale.value(), "resize update");
            entry.scale.set(scale);
            entry.scaling.set_scale(scale.value());
            self.apply_position(entry);
            if entry.config.bounds_mode == BoundsMode::Contained {
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
    }

    fn on_card_resize_end(&self, id: &CardId, gesture: &GestureDrag) {
        self.on_card_resize_update(id, gesture);
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            entry.resize_grab.set(None);
            self.clamp_entry(entry);
            entry.card.clone()
        };
        card.emit_card_scaled();
        self.emit_geometry_changed();
    }

    /// The resize drag's scale target at the gesture's current pointer
    /// position; `None` until the pointer maps into board coordinates.
    fn resize_scale_for(&self, gesture: &GestureDrag, entry: &CardEntry) -> Option<ScaleFactor> {
        entry.resize_target(self.pointer_in_layer(gesture)?)
    }

    /// Captures the pointer's board position at rotate-drag begin. The
    /// pointer's angular sweep around the card center is the drag's
    /// reference frame - it decides the step direction.
    fn on_card_rotate_begin(&self, id: &CardId, gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.rotate_grab.set(self.pointer_in_layer(gesture));
            entry.rotate_stepped.set(false);
        }
    }

    /// Steps the rotation to the next quarter turn once the pointer's
    /// angular sweep around the card center passes the trigger. Only one
    /// step runs per drag - further swiping in the same gesture is
    /// ignored, the zoom-snap animation carries the card to the target.
    fn on_card_rotate_update(&self, id: &CardId, gesture: &GestureDrag) {
        let (rotation_widget, target) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let Some(now) = self.pointer_in_layer(gesture) else {
                return;
            };
            let Some(target) = entry.rotate_target(now, entry.rotation_widget.rotation()) else {
                return;
            };
            entry.rotate_stepped.set(true);
            debug!(card = %id, target_deg = target.to_degrees(), "rotate step");
            (entry.rotation_widget.clone(), target)
        };
        rotation_widget.set_rotation_with_animation(target);
    }

    fn on_card_rotate_end(&self, id: &CardId, _gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.rotate_grab.set(None);
        }
    }

    /// Pointer position in card-layer (board) coordinates - delegates to
    /// the layer, which owns the target coordinate space.
    fn pointer_in_layer(&self, gesture: &GestureDrag) -> Option<Vec2<Pixels>> {
        self.card_layer()?.gesture_pointer(gesture)
    }

    /// Drag delta in board coordinates: current pointer position minus the
    /// pointer position captured at drag begin. Falls back to undoing the
    /// inverse `rotate . scale` on the widget-space offset before the first
    /// allocation, when no surface mapping is available yet.
    fn drag_delta(&self, gesture: &GestureDrag, offset: Vec2<Pixels>, entry: &CardEntry) -> Vec2<Pixels> {
        if let (Some(now), Some(grab)) = (self.pointer_in_layer(gesture), entry.drag_grab.get()) {
            return now - grab;
        }
        let rotation = entry.rotation_widget.rotation();
        let scale = entry.scale.get().value();
        offset.rotated(rotation) * scale
    }

    /// Reorders the card layer by `z_index` (stable by insertion sequence).
    fn restack(&self) {
        let Some(layer) = self.card_layer() else {
            return;
        };
        let cards = self.imp().cards.borrow();
        let mut ordered: Vec<&CardEntry> = cards.values().collect();
        ordered.sort_by_key(|entry| (entry.config.z_index.unwrap_or(i32::MAX), entry.sequence));
        for entry in ordered {
            let top_left = self.displayed_footprint(entry).min;
            layer.remove(&entry.scaling);
            layer.put(&entry.scaling, top_left.x, top_left.y);
        }
    }

    fn wire_gestures(&self, id: &CardId, card: &Card, scaling: &ScalingWidget, rotation_widget: &RotationWidget, config: &CardConfig) {
        // Fold changes the card footprint - layout persistence and
        // input-region consumers track it through `geometry-changed`.
        let board_weak = self.downgrade();
        card.connect_local("card-folded", false, move |_| {
            if let Some(board) = board_weak.upgrade() {
                board.emit_geometry_changed();
            }
            None
        });

        // Rotation tracking: RotatedLayout exposes `rotation` as a notify-able
        // property, covering both gesture and programmatic changes.
        if let Some(manager) = rotation_widget.layout_manager() {
            let board_weak = self.downgrade();
            let card_id = id.clone();
            manager.connect_notify_local(Some("rotation"), move |_, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_rotated(&card_id);
                }
            });
        }

        // Raise on release: a click raises the card, and a finished drag ends
        // raised too. Raising on press would cancel the in-flight sequence.
        let click = GestureClick::new();
        click.set_button(0);
        let board_weak = self.downgrade();
        let card_id = id.clone();
        click.connect_released(move |_, _, _, _| {
            if let Some(board) = board_weak.upgrade() {
                board.raise_card(&card_id);
            }
        });
        scaling.add_controller(click.clone());

        let mut card_drag: Option<GestureDrag> = None;
        if config.draggable {
            let drag = GestureDrag::new();
            drag.set_button(0);
            let board_weak = self.downgrade();
            let card_id = id.clone();
            drag.connect_drag_begin(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_drag_begin(&card_id, gesture);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            drag.connect_drag_update(move |gesture, offset_x, offset_y| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_drag_update(&card_id, gesture, Vec2::new(offset_x, offset_y));
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            drag.connect_drag_end(move |gesture, offset_x, offset_y| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_drag_end(&card_id, gesture, Vec2::new(offset_x, offset_y));
                }
            });
            card.drag_source(config.drag_handle).add_controller(drag.clone());
            card_drag = Some(drag);
        }

        // Resize grip: a dedicated drag on the card's bottom-right
        // affordance scales the card around its center. The grip sits deeper
        // in the bubble chain than the card drag and the `scaling` click, so
        // its press wins the sequence claim before either can react.
        if config.resizable
            && let Some(grip) = card.resize_grip()
        {
            let resize = GestureDrag::new();
            resize.set_button(0);
            // Mark claimed sequences immediately: the grip drag claims a
            // press before the quad tracker sees it, and without the mark
            // the touch would register as a (phantom) transform point.
            let board_weak = self.downgrade();
            let card_id = id.clone();
            resize.connect_sequence_state_changed(move |_, sequence, state| {
                if state != EventSequenceState::Claimed {
                    return;
                }
                if let (Some(board), Some(sequence)) = (board_weak.upgrade(), sequence) {
                    let cards = board.imp().cards.borrow();
                    if let Some(entry) = cards.get(&card_id) {
                        entry.claimed.borrow_mut().insert(sequence.clone(), Instant::now());
                    }
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            resize.connect_drag_begin(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_resize_begin(&card_id, gesture);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            resize.connect_drag_update(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_resize_update(&card_id, gesture);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            resize.connect_drag_end(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_resize_end(&card_id, gesture);
                }
            });
            grip.add_controller(resize);
        }

        // Rotate grip: a dedicated drag on the card's bottom-left
        // affordance steps the rotation to the next quarter turn in the
        // swipe direction. Like the resize grip it sits deeper in the
        // bubble chain than the card drag and the `scaling` click, so
        // its press wins the sequence claim before either can react.
        if config.rotate_grip
            && let Some(grip) = card.rotate_grip()
        {
            let rotate = GestureDrag::new();
            rotate.set_button(0);
            // Same phantom-touch guard as the resize grip: mark claimed
            // sequences so the quad tracker never counts them.
            let board_weak = self.downgrade();
            let card_id = id.clone();
            rotate.connect_sequence_state_changed(move |_, sequence, state| {
                if state != EventSequenceState::Claimed {
                    return;
                }
                if let (Some(board), Some(sequence)) = (board_weak.upgrade(), sequence) {
                    let cards = board.imp().cards.borrow();
                    if let Some(entry) = cards.get(&card_id) {
                        entry.claimed.borrow_mut().insert(sequence.clone(), Instant::now());
                    }
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            rotate.connect_drag_begin(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_rotate_begin(&card_id, gesture);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            rotate.connect_drag_update(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_rotate_update(&card_id, gesture);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            rotate.connect_drag_end(move |gesture, _, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_rotate_end(&card_id, gesture);
                }
            });
            grip.add_controller(rotate);
        }

        // Quad transform tracker: a raw `EventControllerLegacy` observing
        // every touch sequence on the card. Unlike `GestureSingle` siblings
        // it sees all sequences without claiming any, so each finger maps to
        // a tracked point and interactive content keeps working. With
        // `QUAD_MIN_POINTS` or more live points a similarity transform
        // applies - centroid delta moves, radius ratio scales, point-cloud
        // rotation rotates - so move, rotate and zoom are one mechanism
        // instead of arbitrated gestures. `FBD_NO_QUAD` disables it for
        // A/B testing.
        let quad_enabled = std::env::var_os("FBD_NO_QUAD").is_none() && (config.draggable || config.rotatable || config.scalable);
        if quad_enabled {
            let tracker = EventControllerLegacy::new();
            let board_weak = self.downgrade();
            let card_id = id.clone();
            let click_for_tracker = click.clone();
            tracker.connect_event(move |controller, event| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_quad_touch_event(&card_id, controller, event, card_drag.as_ref(), &click_for_tracker);
                }
                Propagation::Proceed
            });
            scaling.add_controller(tracker);
        }
    }

    /// Emits `geometry-changed` on the board.
    pub(crate) fn emit_geometry_changed(&self) {
        self.emit_by_name::<()>(BoardSignal::GeometryChanged.name(), &[]);
    }

    fn card_layer(&self) -> Option<CardLayer> {
        self.imp().card_layer.get().cloned()
    }

    fn board_bounds(&self) -> Rect<Pixels> {
        self.card_layer()
            .map(|layer| Rect::new(Vec2::ZERO, layer.allocated_size().to_f64().with_unit()))
            .unwrap_or_default()
    }

    /// Footprint rectangle of the transform stack in board coordinates: the
    /// displayed size centered on the card's stored position.
    fn displayed_footprint(&self, entry: &CardEntry) -> Rect<Pixels> {
        Rect::centered(entry.position.get().into(), self.displayed_size(entry))
    }

    /// Positions the transform stack so its displayed center matches
    /// `entry.position` (`gtk4::Fixed` positions by top-left). No-ops when the
    /// stack already sits at the target spot, avoiding needless relayouts.
    fn apply_position(&self, entry: &CardEntry) {
        let Some(layer) = self.card_layer() else {
            return;
        };
        let target = self.displayed_footprint(entry).min;
        let current = Vec2::<Pixels>::from(layer.child_position(&entry.scaling));
        let delta = current - target;
        if delta.x.abs() > 0.5 || delta.y.abs() > 0.5 {
            layer.move_(&entry.scaling, target.x, target.y);
        }
    }

    /// Pushes the card back inside the board bounds. `BoundsMode::Loose`
    /// cards keep only `min_visible` pixels inside; `Contained` cards get
    /// their whole footprint pulled back.
    fn clamp_entry(&self, entry: &CardEntry) {
        let bounds = self.board_bounds();
        if bounds.is_degenerate() {
            return;
        }
        let footprint = self.displayed_footprint(entry);
        let delta = match entry.config.bounds_mode {
            BoundsMode::Loose => footprint.clamp_offset(&bounds, entry.config.min_visible.value()),
            BoundsMode::Contained => footprint.contain_offset(&bounds),
        };
        if delta != Vec2::ZERO {
            entry.position.set(entry.position.get().translated(delta));
            self.apply_position(entry);
        }
    }

    fn clamp_all(&self) {
        let cards = self.imp().cards.borrow();
        for entry in cards.values() {
            self.clamp_entry(entry);
        }
    }

    /// The size the transform stack occupies on the layer: the predicted size
    /// (what `gtk4::Fixed` will allocate next, reflecting pending rotation and
    /// scale changes), falling back to the allocated size before measuring is
    /// possible.
    fn displayed_size(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let size = self.predicted_size(entry);
        if size.x > 0.0 && size.y > 0.0 {
            size
        } else {
            entry.scaling.allocated_size().to_f64().with_unit::<Pixels>()
        }
    }

    /// Predicted displayed size: rotated bounding box of the card's natural
    /// size, times the scale factor.
    fn predicted_size(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let (_, natural) = entry.card.measure_size();
        (natural.to_f64().with_unit::<Pixels>() * entry.scale.get().value()).rotated_bounds(entry.rotation_widget.rotation())
    }

    fn entry_geometry(&self, entry: &CardEntry) -> CardGeometry {
        CardGeometry {
            position: entry.position.get(),
            rotation: Rotation::quantized(entry.rotation_widget.rotation()),
            scale: entry.scale.get(),
        }
    }

    /// Technical layout input for a card: displayed footprint, live
    /// rotation and explicit seat assignment.
    fn layout_item(&self, entry: &CardEntry) -> LayoutItem<Pixels> {
        LayoutItem {
            size: self.displayed_size(entry),
            rotation: Rotation::quantized(entry.rotation_widget.rotation()),
            seat: entry.seat.get(),
        }
    }

    /// Glides a card towards `target` on the frame clock. Each tick
    /// applies the eased `CardGeometry` through the regular
    /// `set_card_geometry` path, so re-centering, clamping and signal
    /// emission stay identical to an instant update.
    ///
    /// Starting a new glide bumps the card's `layout_generation`;
    /// ticks of a superseded animation notice the stale generation and
    /// stop themselves.
    fn glide_card_geometry(&self, id: &CardId, target: CardGeometry) {
        let (scaling, from, duration, generation) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let generation = entry.layout_generation.get().incremented();
            entry.layout_generation.set(generation);
            (
                entry.scaling.clone(),
                self.entry_geometry(entry),
                Duration::from_millis(entry.config.animation_speed_ms),
                generation,
            )
        };
        let mut animation = TimedAnimation::new(Span::new(from, target), duration, EasingFunction::EaseInOut);
        animation.start();
        let board = self.clone();
        let id = id.clone();
        drive(&scaling, animation, move |_, geometry| {
            {
                let cards = board.imp().cards.borrow();
                match cards.get(&id) {
                    Some(entry) if entry.layout_generation.get() == generation => {}
                    _ => return ControlFlow::Break,
                }
            }
            board.set_card_geometry(&id, &geometry);
            ControlFlow::Continue
        });
    }
}
