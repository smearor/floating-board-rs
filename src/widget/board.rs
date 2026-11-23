use crate::domain::BoardError;
use crate::domain::BoardLayout;
use crate::domain::BoundsMode;
use crate::domain::CardAnchor;
use crate::domain::CardGeometry;
use crate::domain::CardId;
use crate::domain::CardPlacement;
use crate::domain::CardPosition;
use crate::domain::GenerationCounter;
use crate::domain::GotoRotation;
use crate::domain::GotoTransition;
use crate::domain::ScaleFactor;
use crate::domain::SeatAnchor;
use crate::domain::TurnDirection;
use crate::l10n::l10n;
use crate::widget::board_signal::BoardSignal;
use crate::widget::card::Card;
use crate::widget::card_alignment::CardAlignment;
use crate::widget::card_close_action::CardCloseAction;
use crate::widget::card_config::CardConfig;
use crate::widget::card_layer::CardLayer;
use crate::widget::css::CLASS_VIEW_CONTROLS;
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
use gtk4::Align;
use gtk4::Buildable;
use gtk4::Button;
use gtk4::ConstraintTarget;
use gtk4::EventControllerLegacy;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::GestureRotate;
use gtk4::GestureZoom;
use gtk4::Orientable;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::cairo::Region;
use gtk4::gdk::EventType;
use gtk4::gio::prelude::ListModelExtManual;
use gtk4::glib;
use gtk4::prelude::BoxExt;
use gtk4::prelude::ButtonExt;
use gtk4::prelude::EventControllerExt;
use gtk4::prelude::FixedExt;
use gtk4::prelude::GestureDragExt;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::LayoutManagerExt;
use gtk4::prelude::NativeExt;
use gtk4::prelude::SurfaceExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::WidgetExtManual;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use smearor_commons_animation::EasingFunction;
use smearor_commons_animation::Interpolate;
use smearor_commons_animation::Span;
use smearor_commons_animation::ThreePhaseSpan;
use smearor_commons_animation::TimedAnimation;
use smearor_commons_animation::Trajectory;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::ScaleFactor as ViewScaleFactor;
use smearor_commons_geometry::SimilarityTransform;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use smearor_commons_layout::LayoutItem;
use smearor_commons_layout::LayoutStrategy;
use smearor_commons_layout::Seat;
use smearor_commons_layout::View;
use smearor_commons_layout::WorkspaceSize;
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

/// Zoom factor applied per click of the corner `+` button (`-` uses
/// its reciprocal).
const VIEW_BUTTON_ZOOM_STEP: f64 = 1.25;

/// `goto_card` frames the card's footprint grown by this factor, so
/// the card lands slightly inset instead of filling the screen.
const GOTO_CARD_PADDING: f64 = 1.5;

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
        // `RotationWidget` defaults `animations_enabled` to false -
        // enable its zoom-snap rotation animation unless the session
        // requests reduced motion.
        rotation_widget.set_animations_enabled(gtk4::Settings::default().is_none_or(|settings| settings.is_gtk_enable_animations()));
        rotation_widget.set_child(Some(&card));
        let scaling = ScalingWidget::new(&rotation_widget);
        layer.put(&scaling, 0.0, 0.0);

        let satellites = self.build_satellites(&layer, &card, &scaling, &config);
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
            rotation: Cell::new(RotationRad::ZERO),
            scale: Cell::new(ScaleFactor::default()),
            pending_anchor: RefCell::new(pending_anchor),
            card_drag: RefCell::new(None),
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
            sticky: Cell::new(config.sticky),
            layout_generation: Cell::new(GenerationCounter::default()),
            glide_target: Cell::new(None),
            satellites: RefCell::new(satellites),
            satellites_generation: Cell::new(GenerationCounter::default()),
            fold_choreo_closing: Cell::new(false),
        };
        self.imp().cards.borrow_mut().insert(id.clone(), entry);

        // Satellite mount reveal (P5.2): an initially-open set emerges
        // from the collapsed seed instead of appearing at once.
        {
            let cards = self.imp().cards.borrow();
            if let Some(entry) = cards.get(&id) {
                let (settings, _) = entry.config.effective_satellites();
                if settings.open && !entry.satellites.borrow().is_empty() {
                    self.begin_satellite_transitions(entry, true, false);
                }
            }
        }

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
            for satellite in entry.satellites.borrow().iter() {
                layer.remove(&satellite.scaling);
            }
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
        let rendered_center = {
            let cards = self.imp().cards.borrow();
            match cards.get(&card.id()) {
                Some(entry) => self.card_view(entry).to_widget(position.0, self.widget_bounds()),
                None => position.0,
            }
        };
        CardAlignment {
            card_id: card.id(),
            position,
            placed: card.placed_position(),
            rendered,
            drift: rendered.map(|bounds| bounds.center() - rendered_center),
        }
    }

    /// Snapshot of every card's geometry for consumer-side persistence.
    ///
    /// Card positions are workspace coordinates; the snapshot also stores
    /// the workspace size and the configured view (`None` = default view).
    pub fn layout_snapshot(&self) -> BoardLayout {
        let cards = self.imp().cards.borrow();
        let mut layout = BoardLayout::new(
            cards
                .iter()
                .map(|(id, entry)| {
                    CardPlacement::new(id.clone(), self.entry_geometry(entry), entry.card.folded())
                        .with_seat(entry.seat.get())
                        .with_sticky(entry.sticky.get())
                })
                .collect(),
        );
        layout.workspace = self.imp().workspace_size.get();
        layout.view = *self.imp().view.borrow();
        layout.seat_anchor = self.imp().seat_anchor.get();
        layout
    }

    /// Restores a previously snapshotted layout.
    ///
    /// Unknown card ids are skipped (logged, not an error); geometry is
    /// clamped to the current workspace bounds so nothing restores
    /// off-screen. The stored workspace size and view are applied first.
    pub fn restore_layout(&self, layout: &BoardLayout) {
        self.cancel_view_flight();
        self.imp().workspace_size.set(layout.workspace);
        self.imp().view.replace(layout.view);
        self.imp().seat_anchor.set(layout.seat_anchor);
        self.on_board_allocated();
        for placement in &layout.cards {
            let (card, rotation_widget, scaling) = {
                let cards = self.imp().cards.borrow();
                match cards.get(&placement.id) {
                    Some(entry) => {
                        entry.sticky.set(placement.sticky);
                        entry.position.set(placement.geometry.position);
                        entry.scale.set(placement.geometry.scale.clamped(&entry.config.scale_limits));
                        entry.pending_anchor.borrow_mut().take();
                        // A restore supersedes in-flight glides: bumping
                        // the generation stops their ticks and retires
                        // the recorded target for seat fan-out.
                        entry.layout_generation.set(entry.layout_generation.get().incremented());
                        entry.glide_target.take();
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
            {
                let cards = self.imp().cards.borrow();
                if let Some(entry) = cards.get(&placement.id) {
                    rotation_widget.set_rotation(SmearorRotation::from(placement.geometry.rotation.to_rotation_rad() + self.card_view(entry).rotation));
                    scaling.set_scale(placement.geometry.scale.value() * self.card_view_zoom(entry));
                }
            }
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
                        // Sticky cards are pinned to the screen and do
                        // not participate in workspace layouts.
                        if !entry.sticky.get() {
                            selected.push((id.clone(), entry.scale.get(), self.layout_item(entry)));
                        }
                    }
                }
                None => {
                    let mut visible: Vec<(&CardId, &CardEntry)> = cards.iter().filter(|(_, entry)| entry.scaling.is_visible() && !entry.sticky.get()).collect();
                    visible.sort_by_key(|(_, entry)| entry.sequence);
                    for (id, entry) in visible {
                        selected.push((id.clone(), entry.scale.get(), self.layout_item(entry)));
                    }
                }
            }
        }

        let items: Vec<LayoutItem<Pixels>> = selected.iter().map(|(_, _, item)| *item).collect();
        let placements = strategy.arrange(self.workspace_bounds(), &items)?;

        for ((id, scale, _), placement) in selected.iter().zip(placements.iter()) {
            // The placement expresses the target footprint; the card's
            // configured scale is preserved.
            let target = CardGeometry {
                position: CardPosition::from(placement.center),
                rotation: placement.rotation,
                scale: *scale,
            };
            if animated {
                self.glide_card_geometry(id, target)?;
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
    /// The step is on table *sides* - use
    /// [`Self::rotate_to_next_occupied_seat`] to hop between occupied
    /// seats instead. The passed card slips into the nearest free spot
    /// at the seat's edge lane (or the lane behind it when the edge
    /// queue reaches farther than a second row would), never on top of
    /// another card.
    ///
    /// An explicitly assigned seat is advanced and persisted
    /// (`card-seat-changed`); unassigned cards stay unassigned - the
    /// next pass derives their seat from the rotation again.
    ///
    /// Returns [`BoardError::UnknownCard`] for an id not on the board.
    pub fn rotate_to_next_user(&self, id: &CardId, direction: TurnDirection, move_to_edge: bool) -> Result<(), BoardError> {
        let seat = self.card_seat_or_nearest(id)?;
        let target = match direction {
            TurnDirection::Clockwise => seat.clockwise(),
            TurnDirection::CounterClockwise => seat.counter_clockwise(),
        };
        self.glide_card_to_seat(id, target, move_to_edge, false)
    }

    /// Passes the card to the next *occupied* seat of `seats`.
    ///
    /// Like [`Self::rotate_to_next_user`], but the destination is the
    /// roster seat with the smallest clockwise (or counter-clockwise)
    /// distance along the table perimeter: empty sides are skipped and
    /// several seats on one side are visited in order. This is the
    /// semantic a consumer's user list carries - "next player", not
    /// "next edge".
    ///
    /// Unlike [`Self::rotate_to_next_user`], a reached roster seat is
    /// always assigned to the card (`card-seat-changed`), including
    /// previously unassigned cards: being passed to a user makes the
    /// card belong to their seat, and the persisted position is what
    /// lets repeated passes keep hopping the roster instead of
    /// re-deriving a mid-edge starting point.
    ///
    /// The passed card slips into the nearest free spot at the seat's
    /// edge lane (or the lane behind it when the edge queue reaches
    /// farther than a second row would), never on top of another card.
    ///
    /// Returns [`BoardError::UnknownCard`] for an id not on the board
    /// and [`BoardError::NoNextSeat`] when the roster holds no seat
    /// other than the card's current one (empty rosters included).
    pub fn rotate_to_next_occupied_seat(&self, id: &CardId, seats: &[Seat], direction: TurnDirection, move_to_edge: bool) -> Result<(), BoardError> {
        let seat = self.card_seat_or_nearest(id)?;
        let target = match direction {
            TurnDirection::Clockwise => seat.next_in(seats),
            TurnDirection::CounterClockwise => seat.previous_in(seats),
        };
        let Some(target) = target else {
            return Err(BoardError::NoNextSeat(id.clone()));
        };
        self.glide_card_to_seat(id, target, move_to_edge, true)
    }

    /// Current geometry of a card (rotation is read live from the widget).
    pub(crate) fn card_geometry(&self, id: &CardId) -> Option<CardGeometry> {
        self.imp().cards.borrow().get(id).map(|entry| self.entry_geometry(entry))
    }

    /// Applies a full geometry update, clamped to the board bounds.
    /// For a sticky card `geometry` is expressed in widget pixels,
    /// not workspace coordinates.
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
        {
            let cards = self.imp().cards.borrow();
            if let Some(entry) = cards.get(id) {
                rotation_widget.set_rotation(SmearorRotation::from(geometry.rotation.to_rotation_rad() + self.card_view(entry).rotation));
                scaling.set_scale(scale.value() * self.card_view_zoom(entry));
                self.apply_position(entry);
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
    }

    /// Sets the card rotation immediately (no animation).
    pub(crate) fn set_card_rotation(&self, id: &CardId, rotation: Rotation) {
        let cards = self.imp().cards.borrow();
        if let Some(entry) = cards.get(id) {
            // The notify::rotation handler performs re-centering, clamping and
            // signal emission. The widget carries the rendered rotation -
            // card rotation plus the view's on-screen rotation (identity
            // for sticky cards).
            let rendered = rotation.to_rotation_rad() + self.card_view(entry).rotation;
            entry.rotation_widget.set_rotation(SmearorRotation::from(rendered));
        }
    }

    /// Sets the card rotation with the zoom-snap animation.
    pub(crate) fn set_card_rotation_animated(&self, id: &CardId, rotation: Rotation) {
        let cards = self.imp().cards.borrow();
        if let Some(entry) = cards.get(id) {
            let rendered = rotation.to_rotation_rad() + self.card_view(entry).rotation;
            entry.rotation_widget.set_rotation_with_animation(rendered);
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
            entry.scaling.set_scale(scale.value() * self.card_view_zoom(entry));
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
        let (rotation_widget, scaling, sticky) = {
            let cards = self.imp().cards.borrow();
            let entry = cards.get(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
            entry.scale.set(ScaleFactor::default());
            (entry.rotation_widget.clone(), entry.scaling.clone(), entry.sticky.get())
        };
        rotation_widget.set_rotation(SmearorRotation::from(match sticky {
            true => RotationRad::ZERO,
            false => self.view().rotation,
        }));
        scaling.set_scale(match sticky {
            true => 1.0,
            false => self.view_zoom(),
        });
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
                    let bounds = match entry.sticky.get() {
                        true => self.widget_bounds(),
                        false => self.workspace_bounds(),
                    };
                    if bounds.is_degenerate() {
                        entry.pending_anchor.replace(Some(anchor));
                    } else {
                        entry.pending_anchor.borrow_mut().take();
                        let footprint = self.workspace_card_size(entry);
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
            let top_left = self.stack_top_left(entry);
            layer.remove(&entry.scaling);
            layer.put(&entry.scaling, top_left.x, top_left.y);
            self.restack_satellites(entry, &layer);
            entry.card.clone()
        };
        card.emit_card_raised();
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
            self.sync_satellites(entry);
            entry.card.clone()
        };
        self.emit_by_name::<()>(BoardSignal::CardVisibilityChanged.name(), &[&card]);
        self.emit_geometry_changed();
        Ok(())
    }

    /// Opens or closes the card's whole satellite set. The initial
    /// state comes from `CardSatellites::open`; a closed set hides
    /// like a folded card hides it - placements keep resolving in the
    /// background, so reopening lands on current positions. With
    /// animations enabled the satellites stagger-tween between the
    /// collapsed seed and their resolved placements (Phase 5).
    pub fn set_satellites_open(&self, id: &CardId, open: bool) -> Result<(), BoardError> {
        let cards = self.imp().cards.borrow();
        let Some(entry) = cards.get(id) else {
            return Err(BoardError::UnknownCard(id.clone()));
        };
        // Transitions begin from the pre-flip state: `begin`'s close
        // eligibility checks the logical `open` flag, so the flag commit
        // happens only after the batch was created.
        self.begin_satellite_transitions(entry, open, false);
        for satellite in entry.satellites.borrow().iter() {
            satellite.open.set(open);
        }
        self.sync_satellites(entry);
        Ok(())
    }

    /// Whether the card's satellite set is open - `None` for an
    /// unknown card, `true` for a card without satellites.
    pub fn satellites_open(&self, id: &CardId) -> Option<bool> {
        let cards = self.imp().cards.borrow();
        let entry = cards.get(id)?;
        Some(entry.satellites.borrow().first().is_none_or(|satellite| satellite.open.get()))
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

    /// Whether the card is pinned to the screen (`CardConfig::sticky`).
    pub fn card_sticky(&self, id: &CardId) -> Result<bool, BoardError> {
        let cards = self.imp().cards.borrow();
        let Some(entry) = cards.get(id) else {
            return Err(BoardError::UnknownCard(id.clone()));
        };
        Ok(entry.sticky.get())
    }

    /// Pins the card to the screen (`sticky = true`) or releases it
    /// back onto the workspace (`false`). A sticky card ignores the
    /// `View` camera entirely: it keeps its on-screen position,
    /// rotation and zoom while the view pans, zooms or rotates, and
    /// it clamps against the widget rect.
    ///
    /// Toggling never moves the card visually: the stored geometry
    /// switches space at the seam - pinning captures the current
    /// rendered geometry into widget pixels, releasing maps it back
    /// through `to_workspace`. Sticky cards are skipped by
    /// [`Self::apply_layout`], and passing one to a seat releases the
    /// pin first (`rotate_to_next_user`,
    /// `rotate_to_next_occupied_seat`).
    ///
    /// Returns [`BoardError::UnknownCard`] for an id not on the board.
    pub fn set_card_sticky(&self, id: &CardId, sticky: bool) -> Result<(), BoardError> {
        let cards = self.imp().cards.borrow();
        let Some(entry) = cards.get(id) else {
            return Err(BoardError::UnknownCard(id.clone()));
        };
        if entry.sticky.get() == sticky {
            return Ok(());
        }
        let view = self.view();
        let widget = self.widget_bounds();
        if sticky {
            // Workspace -> screen: capture the rendered geometry.
            if !widget.is_degenerate() {
                entry.position.set(CardPosition::from(view.to_widget(entry.position.get().into(), widget)));
            }
            entry.rotation.set((entry.rotation.get() + view.rotation).normalized());
            entry.scale.set(ScaleFactor::new(entry.scale.get().value() * self.view_zoom()));
        } else {
            // Screen -> workspace: map back through the view.
            if !widget.is_degenerate() {
                entry.position.set(CardPosition::from(view.to_workspace(entry.position.get().into(), widget)));
            }
            entry.rotation.set((entry.rotation.get() - view.rotation).normalized());
            entry.scale.set(ScaleFactor::new(entry.scale.get().value() / self.view_zoom()));
        }
        entry.sticky.set(sticky);
        self.sync_rendered_transform(entry);
        self.clamp_entry(entry);
        drop(cards);
        self.emit_geometry_changed();
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
        let scale = entry.scale.get().value() * self.card_view_zoom(entry);
        let view = self.card_view(entry);
        let center = view.to_widget(entry.position.get().into(), self.widget_bounds());
        let footprint = Rect::<Pixels>::centered(center, natural.to_f64().with_unit::<Pixels>() * scale);
        footprint.rotated_region(entry.rotation.get() + view.rotation).unwrap_or_else(|error| {
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

    /// Reconciles model and render state after each allocation pass -
    /// resolves pending anchors, syncs card transforms to the view and
    /// re-clamps. Called by the board's own layout manager
    /// (`FloatingBoardLayout::allocate`): GTK4 delegates allocation to
    /// the layout manager for widgets that have one (like `GtkBox`), so
    /// a `size_allocate` vfunc would never run.
    pub(crate) fn on_board_allocated(&self) {
        // smearor-wrot-rotation is designed for layer-shell surfaces and
        // narrows the surface input region to each card's footprint on every
        // allocation. Inside the board all card allocations run through this
        // pass, so restoring the full input region here wins per layout pass.
        // Skipped while a consumer manages the region itself
        // (`set_input_region_managed`) - e.g. touch-passthrough overlays that
        // restrict input to the cards.
        if !self.imp().input_region_managed.get()
            && let Some(surface) = self.native().and_then(|native| native.surface())
        {
            surface.set_input_region(None);
        }
        let workspace = self.workspace_bounds();
        if workspace.is_degenerate() {
            return;
        }
        let mut resolved = false;
        {
            let cards = self.imp().cards.borrow();
            for entry in cards.values() {
                if let Some(anchor) = entry.pending_anchor.borrow_mut().take() {
                    // Sticky anchors resolve against the widget rect -
                    // their geometry lives in screen coordinates.
                    let bounds = match entry.sticky.get() {
                        true => self.widget_bounds(),
                        false => workspace,
                    };
                    let footprint = self.workspace_card_size(entry);
                    entry.position.set(anchor.resolve(&bounds, footprint, entry.config.anchor_margin));
                    resolved = true;
                }
                // Reconcile render state with position state on every
                // allocation: `gtk4::Fixed` positions are only ever moved via
                // apply_position, so cards added before the first allocation
                // would otherwise stay at (0,0). The view zoom can also
                // change with the allocation (a `Fixed` workspace fits to
                // the widget), so the rendered transform is re-synced.
                self.sync_rendered_transform(entry);
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
            // The widget carries card rotation + view rotation; the model
            // keeps the workspace part. Synced before position/clamp work
            // reads it.
            entry.rotation.set(entry.rotation_widget.rotation() - self.card_view(entry).rotation);
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
    /// slot actually owns. The slot records the finger's workspace
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
                {
                    let cards = self.imp().cards.borrow();
                    if let Some(entry) = cards.get(id)
                        && let Some(point) = self.event_point_in_workspace(entry, controller, event)
                    {
                        self.activate_quad_touch(entry, sequence, point, card_drag, click);
                    }
                }
                self.emit_geometry_changed();
            }
            EventType::TouchUpdate => {
                {
                    let cards = self.imp().cards.borrow();
                    let Some(entry) = cards.get(id) else {
                        return;
                    };
                    let Some(point) = self.event_point_in_workspace(entry, controller, event) else {
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

    /// Activates a quad touch: records the finger's workspace position,
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

    /// Touch position of a raw event in widget coordinates - delegates to
    /// the layer, which owns the target coordinate space.
    fn event_point_in_layer(&self, controller: &EventControllerLegacy, event: &gtk4::gdk::Event) -> Option<Vec2<Pixels>> {
        self.card_layer()?.event_pointer(controller, event)
    }

    /// Touch position of a raw event in workspace coordinates - the
    /// layer point mapped back through the view (input path). `None`
    /// while the widget is degenerate (the inverse map cannot scale
    /// by zero).
    fn event_point_in_workspace(&self, entry: &CardEntry, controller: &EventControllerLegacy, event: &gtk4::gdk::Event) -> Option<Vec2<Pixels>> {
        let widget = self.widget_bounds();
        if widget.is_degenerate() {
            return None;
        }
        let layer_point = self.event_point_in_layer(controller, event)?;
        // Sticky cards live in widget coordinates - their input path
        // skips the view inverse just like their render path does.
        match entry.sticky.get() {
            true => Some(layer_point),
            false => Some(self.view().to_workspace(layer_point, widget)),
        }
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
        info!(points = ?points, position = ?entry.position.get(), scale = entry.scale.get().value(), rotation_deg = entry.rotation.get().to_degrees(), "quad baseline snapshot");
        entry.quad_baseline.replace(Some(QuadBaseline {
            position: entry.position.get(),
            scale: entry.scale.get(),
            rotation: entry.rotation.get(),
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
            let rendered = baseline.rotation + angle + self.card_view(entry).rotation;
            if rendered.signed_delta_to(entry.rotation_widget.rotation()).abs() > RotationRad::from_degrees(0.01) {
                baseline.rotated = true;
                entry.rotation_widget.set_rotation(SmearorRotation::from(rendered));
            }
        }
        if entry.config.scalable && scale != entry.scale.get() {
            baseline.scaled = true;
            entry.scale.set(scale);
            entry.scaling.set_scale(scale.value() * self.card_view_zoom(entry));
        }
        drop(baseline_ref);
        self.apply_position(entry);
        if entry.config.bounds_mode == BoundsMode::Contained {
            self.clamp_entry(entry);
        }
    }

    /// Records the card position and the pointer's workspace position at
    /// drag begin; the update delta is measured between two workspace pointer
    /// samples, which stays stable while the card follows the pointer.
    fn on_card_drag_begin(&self, id: &CardId, gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.drag_start.set(entry.position.get());
            entry.drag_grab.set(self.pointer_in_workspace(entry, gesture));
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
            entry.resize_grab.set(self.pointer_in_workspace(entry, gesture));
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
            entry.scaling.set_scale(scale.value() * self.card_view_zoom(entry));
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
    /// position; `None` until the pointer maps into workspace coordinates.
    fn resize_scale_for(&self, gesture: &GestureDrag, entry: &CardEntry) -> Option<ScaleFactor> {
        entry.resize_target(self.pointer_in_workspace(entry, gesture)?)
    }

    /// Captures the pointer's board position at rotate-drag begin. The
    /// pointer's angular sweep around the card center is the drag's
    /// reference frame - it decides the step direction.
    fn on_card_rotate_begin(&self, id: &CardId, gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.rotate_grab.set(self.pointer_in_workspace(entry, gesture));
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
            let Some(now) = self.pointer_in_workspace(entry, gesture) else {
                return;
            };
            let Some(target) = entry.rotate_target(now, entry.rotation.get()) else {
                return;
            };
            entry.rotate_stepped.set(true);
            debug!(card = %id, target_deg = target.to_degrees(), "rotate step");
            (entry.rotation_widget.clone(), target + self.card_view(entry).rotation)
        };
        rotation_widget.set_rotation_with_animation(target);
    }

    fn on_card_rotate_end(&self, id: &CardId, _gesture: &GestureDrag) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.rotate_grab.set(None);
        }
    }

    /// Pointer position in card-layer (widget) coordinates - delegates to
    /// the layer, which owns the target coordinate space.
    fn pointer_in_layer(&self, gesture: &GestureDrag) -> Option<Vec2<Pixels>> {
        self.card_layer()?.gesture_pointer(gesture)
    }

    /// Pointer position in workspace coordinates - the layer point mapped
    /// back through the view (input path). `None` while the widget is
    /// degenerate (the inverse map cannot scale by zero).
    fn pointer_in_workspace(&self, entry: &CardEntry, gesture: &GestureDrag) -> Option<Vec2<Pixels>> {
        let widget = self.widget_bounds();
        if widget.is_degenerate() {
            return None;
        }
        let layer_point = self.pointer_in_layer(gesture)?;
        match entry.sticky.get() {
            true => Some(layer_point),
            false => Some(self.view().to_workspace(layer_point, widget)),
        }
    }

    /// Drag delta in workspace coordinates: current pointer position minus
    /// the pointer position captured at drag begin. Falls back to undoing
    /// the card's own `rotate . scale` on the widget-space offset before
    /// the first allocation (the view factors cancel: the offset lives
    /// inside the card's rendered transform).
    fn drag_delta(&self, gesture: &GestureDrag, offset: Vec2<Pixels>, entry: &CardEntry) -> Vec2<Pixels> {
        if let (Some(now), Some(grab)) = (self.pointer_in_workspace(entry, gesture), entry.drag_grab.get()) {
            return now - grab;
        }
        let rotation = entry.rotation.get();
        let scale = entry.scale.get().value();
        offset.rotated(rotation) * scale
    }

    /// Re-resolves the drag affordance for the card's current folded
    /// state and moves the drag controller when the source widget
    /// changed (`WholeCardFolded`/`WholeCardUnfolded` switch between the
    /// card and the header strip on fold).
    fn refresh_drag_source(&self, entry: &CardEntry) {
        let Some(drag) = entry.card_drag.borrow().clone() else {
            return;
        };
        let source = entry.card.drag_source(entry.config.drag_handle);
        if drag.widget().as_ref() == Some(&source) {
            return;
        }
        if let Some(old) = drag.widget() {
            old.remove_controller(&drag);
        }
        source.add_controller(drag);
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
            let top_left = self.stack_top_left(entry);
            layer.remove(&entry.scaling);
            layer.put(&entry.scaling, top_left.x, top_left.y);
            self.restack_satellites(entry, &layer);
        }
    }

    fn wire_gestures(&self, id: &CardId, card: &Card, scaling: &ScalingWidget, rotation_widget: &RotationWidget, config: &CardConfig) {
        // Fold changes the card footprint - layout persistence and
        // input-region consumers track it through `geometry-changed`.
        // Fold choreography (P5.4): a fold first animates the satellite
        // set into the collapsed seed, then the card collapses; an
        // unfold replays the reveal.
        let board_weak = self.downgrade();
        let card_id = id.clone();
        card.connect_local("card-folded", false, move |_| {
            if let Some(board) = board_weak.upgrade() {
                {
                    let cards = board.imp().cards.borrow();
                    if let Some(entry) = cards.get(&card_id) {
                        board.refresh_drag_source(entry);
                        board.on_card_folded(entry);
                    }
                }
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
            // Name tag: the adwaita chrome strips foreign gestures off
            // the HeaderBar (its internal window-move drag fights the
            // card drag) but spares controllers carrying this name.
            drag.set_name(Some(crate::widget::chrome::CARD_GESTURE_NAME));
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
            // Click-to-unfold: `released` only fires for sequences no
            // other gesture claimed - the card drag claims at its
            // threshold, so taps unfold while drags never do.
            if config.unfold_on_click {
                let unfold_click = GestureClick::new();
                unfold_click.set_button(0);
                let card_weak = card.downgrade();
                unfold_click.connect_released(move |_, _, _, _| {
                    if let Some(card) = card_weak.upgrade()
                        && card.folded()
                    {
                        card.set_folded(false);
                    }
                });
                card.add_controller(unfold_click);
            }
            if let Some(entry) = self.imp().cards.borrow().get(id) {
                entry.card_drag.replace(Some(drag.clone()));
            }
            card_drag = Some(drag);
        }

        // Resize grip: a dedicated drag on the card's bottom-right
        // affordance scales the card around its center. The grip sits deeper
        // in the bubble chain than the card drag and the `scaling` click, so
        // its press wins the sequence claim before either can react.
        if let Some(grip) = card.resize_grip() {
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
        if let Some(grip) = card.rotate_grip() {
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

    pub(crate) fn card_layer(&self) -> Option<CardLayer> {
        self.imp().card_layer.get().cloned()
    }

    /// The rendering surface rectangle: the card layer's allocation in
    /// widget pixels, origin at `(0, 0)`.
    /// The viewport rectangle: the board widget's own allocation. Not
    /// the card layer's - a `GtkFixed`-style layer measures at its
    /// children bounding box, so its allocated size shifts with card
    /// positions and must not feed view mapping or seat placement.
    /// The layer sits at origin (0,0) inside the board, so both share
    /// the same coordinate space.
    pub(crate) fn widget_bounds(&self) -> Rect<Pixels> {
        Rect::new(Vec2::ZERO, self.allocated_size().to_f64().with_unit::<Pixels>())
    }

    /// The logical bounds the cards live in (workspace coordinates): the
    /// workspace extent for `FollowWidget`/`Fixed`, the view's shown
    /// region for `Infinite` - the view is the only edge an unbounded
    /// workspace has.
    pub fn workspace_bounds(&self) -> Rect<Pixels> {
        let widget = self.widget_bounds();
        self.imp()
            .workspace_size
            .get()
            .extent(widget.size)
            .unwrap_or_else(|| self.view().shown_region(widget.size))
    }

    /// The effective camera: the configured view or the default view
    /// covering the workspace extent (`Infinite` falls back to the
    /// widget rect - an identity mapping).
    pub fn view(&self) -> View<Pixels> {
        self.imp().view.borrow().unwrap_or_else(|| {
            let widget = self.widget_bounds();
            let region = self.imp().workspace_size.get().extent(widget.size).unwrap_or(widget);
            View::covering(region)
        })
    }

    /// Sets an explicit camera over the workspace: which workspace
    /// region maps onto the widget and at which on-screen rotation.
    /// Re-renders and re-clamps every card. Once set, the view is
    /// pinned - resizes keep it ([`Self::reset_view`] releases the pin).
    pub fn set_view(&self, view: View<Pixels>) {
        self.cancel_view_flight();
        self.pin_view(view);
    }

    /// Returns to the default view covering the workspace extent, which
    /// tracks the widget while it stays untouched.
    pub fn reset_view(&self) {
        self.cancel_view_flight();
        self.imp().view.replace(None);
        self.on_view_changed();
    }

    /// Whether empty-space gestures may move the camera - default
    /// `false`. When enabled, a drag on free space pans, a pinch zooms
    /// around the gesture center and a two-finger twist rotates around
    /// it. Card gestures keep precedence: they sit deeper in the
    /// bubble chain and claim sequences before these gestures see
    /// them, and touches starting on a card are denied up front.
    pub fn view_gestures(&self) -> bool {
        self.imp().view_gestures.get()
    }

    /// Enables or disables the empty-space view gestures.
    pub fn set_view_gestures(&self, enabled: bool) {
        self.imp().view_gestures.set(enabled);
    }

    /// Whether the corner view controls (zoom in, zoom out, reset)
    /// are overlaid on the board - default `false`. Styling via the
    /// `floating-board-view-controls` CSS class.
    pub fn view_buttons(&self) -> bool {
        self.imp().view_buttons.get()
    }

    /// Shows or hides the corner view-control buttons.
    pub fn set_view_buttons(&self, enabled: bool) {
        self.imp().view_buttons.set(enabled);
        if let Some(controls) = self.imp().view_controls.get() {
            controls.set_visible(enabled);
        }
    }

    /// Shifts the camera by `delta` workspace pixels (programmatic
    /// pan). Pins an explicit view - the default view no longer tracks
    /// the workspace extent afterwards ([`Self::reset_view`] releases
    /// the pin).
    pub fn pan_view(&self, delta: Vec2<Pixels>) {
        self.mutate_view(|view| view.pan(delta));
    }

    /// Zooms around the workspace `anchor`: a `factor` of 2 brings the
    /// view twice as close while the anchor keeps its widget position
    /// (zoom-to-point). Pins an explicit view like [`Self::pan_view`].
    pub fn zoom_view(&self, anchor: Vec2<Pixels>, factor: ViewScaleFactor) {
        self.mutate_view(|view| view.zoom_at(anchor, factor));
    }

    /// Rotates the camera by `delta` around the workspace `pivot`,
    /// which keeps its widget position. Pins an explicit view like
    /// [`Self::pan_view`].
    pub fn rotate_view(&self, pivot: Vec2<Pixels>, delta: RotationRad) {
        self.mutate_view(|view| view.rotate_around(pivot, delta));
    }

    /// Flies the camera onto a card: the target view centers the
    /// card's workspace footprint grown by `GOTO_CARD_PADDING`, and
    /// `rotation` decides whether the camera keeps its rotation or
    /// turns the card upright for the viewer. `transition` picks the
    /// flight - `Snap` applies the target instantly, `Glide` flies
    /// straight there and `Transit` pulls back to an overview of both
    /// regions first (Prezi-style). The target card's
    /// `animation_speed_ms` times the flight.
    ///
    /// A running flight is replaced by the next `goto_card`, and any
    /// programmatic or gesture-driven view change ends it. The
    /// destination is a pinned view like [`Self::set_view`] -
    /// [`Self::reset_view`] releases it.
    ///
    /// Returns [`BoardError::UnknownCard`] for an id not on the board.
    pub fn goto_card(&self, id: &CardId, transition: GotoTransition, rotation: GotoRotation) -> Result<(), BoardError> {
        let (geometry, footprint, sticky, duration) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            // The camera targets the in-flight glide destination, not
            // the mid-flight spot.
            let geometry = self.effective_geometry(entry);
            (
                geometry,
                self.workspace_footprint_at(entry, &geometry),
                entry.sticky.get(),
                Duration::from_millis(entry.config.animation_speed_ms),
            )
        };
        let from = self.view();
        let region = match sticky {
            // A sticky card is pinned to the glass - `goto_card` aims
            // the camera at the workspace region behind it.
            true => {
                let widget = self.widget_bounds();
                let rendered = self.rendered_footprint_at(self.imp().cards.borrow().get(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?, &geometry);
                let corners = rendered
                    .corners()
                    .into_iter()
                    .map(|corner| from.to_workspace(corner, widget))
                    .collect::<Vec<_>>();
                let behind = Rect::from_points(corners).unwrap_or(rendered);
                Rect::centered(behind.center(), behind.size * ViewScaleFactor::new(GOTO_CARD_PADDING))
            }
            false => Rect::centered(footprint.center(), footprint.size * ViewScaleFactor::new(GOTO_CARD_PADDING)),
        };
        let target = View {
            region,
            rotation: match (rotation, sticky) {
                (GotoRotation::Keep, _) | (GotoRotation::Upright, true) => from.rotation,
                // Rendered rotation is `card + view`, so a view of
                // `-card` draws the card upright on screen.
                (GotoRotation::Upright, false) => (-geometry.rotation.to_rotation_rad()).normalized(),
            },
        };
        self.fly_view_to(from, target, transition, duration);
        Ok(())
    }

    /// Flies the camera to an arbitrary target view - the public
    /// counterpart of `goto_card` for framings that are not a card
    /// close-up: an overview, or the seat corner a passed card
    /// travels to. `transition` picks the same `Snap`/`Glide`/
    /// `Transit` machinery, `duration` the flight time.
    pub fn goto_view(&self, target: View<Pixels>, transition: GotoTransition, duration: Duration) {
        self.fly_view_to(self.view(), target, transition, duration);
    }

    /// Where the pass-to-seat operations anchor seat edges - default
    /// `SeatAnchor::View` (the visible screen edge where users sit).
    pub fn seat_anchor(&self) -> SeatAnchor {
        self.imp().seat_anchor.get()
    }

    /// Sets the seat anchor for `rotate_to_next_user` and
    /// `rotate_to_next_occupied_seat`. `View` resolves seats in screen
    /// space - the card lands at the widget edge facing the user,
    /// whatever the camera does; `Workspace` keeps seats on the
    /// workspace extent (fixed positions on the virtual table).
    /// Generic `LayoutStrategy` runs are unaffected.
    ///
    /// With `View`, a seat edge beyond the workspace extent can land
    /// outside `workspace_bounds`; `clamp_entry` then pulls the card
    /// partially back inside.
    pub fn set_seat_anchor(&self, anchor: SeatAnchor) {
        self.imp().seat_anchor.set(anchor);
    }

    /// The bounds seat placements resolve against: the widget rect
    /// (screen space) under `SeatAnchor::View`, the workspace extent
    /// under `SeatAnchor::Workspace`. The view-anchored result is
    /// mapped back through `to_workspace` in `glide_card_to_seat`.
    fn seat_bounds(&self) -> Rect<Pixels> {
        match self.imp().seat_anchor.get() {
            SeatAnchor::View => self.widget_bounds(),
            SeatAnchor::Workspace => self.workspace_bounds(),
        }
    }

    /// Resolves the effective view (the default when unpinned), applies
    /// `f` and pins the result back as an explicit view.
    fn mutate_view(&self, f: impl FnOnce(&mut View<Pixels>)) {
        self.cancel_view_flight();
        let mut view = self.view();
        f(&mut view);
        self.pin_view(view);
    }

    /// Ends a running `goto_card` flight - its ticks notice the
    /// bumped generation and stop themselves.
    fn cancel_view_flight(&self) {
        self.imp().view_flight.set(self.imp().view_flight.get().incremented());
    }

    /// Stores `view` as the pinned camera and re-renders - the write
    /// path shared by `set_view`/`mutate_view` and the flight ticks.
    fn pin_view(&self, view: View<Pixels>) {
        self.imp().view.replace(Some(view));
        self.on_view_changed();
    }

    /// Moves the camera `from` its current view to `target` along the
    /// chosen `transition`. Takes over any running flight first.
    fn fly_view_to(&self, from: View<Pixels>, target: View<Pixels>, transition: GotoTransition, duration: Duration) {
        let generation = self.imp().view_flight.get().incremented();
        self.imp().view_flight.set(generation);
        match transition {
            GotoTransition::Snap => self.pin_view(target),
            GotoTransition::Glide => self.start_view_flight(Span::new(from, target), duration, generation),
            GotoTransition::Transit => {
                let transit = self.transit_view(from, target);
                self.start_view_flight(ThreePhaseSpan::new(from, transit, target, Fraction::new(0.5), Fraction::new(0.5)), duration, generation);
            }
        }
    }

    /// The pull-back waypoint of a `Transit` flight: a region covering
    /// both the current and the target shown region, at half the
    /// rotation arc - the Prezi overview the flight crosses through.
    fn transit_view(&self, from: View<Pixels>, target: View<Pixels>) -> View<Pixels> {
        let widget = self.widget_bounds().size;
        let corners = from
            .shown_region(widget)
            .corners()
            .into_iter()
            .chain(target.shown_region(widget).corners())
            .collect::<Vec<_>>();
        View {
            region: Rect::from_points(corners).unwrap_or(target.region),
            rotation: from.rotation.interpolate(target.rotation, Fraction::new(0.5)),
        }
    }

    /// Frame-clock view flight: ticks pin the eased view until the
    /// animation completes or `view_flight` moved on.
    fn start_view_flight<S: Trajectory<Value = View<Pixels>> + 'static>(&self, trajectory: S, duration: Duration, generation: GenerationCounter) {
        let mut animation = TimedAnimation::new(trajectory, duration, EasingFunction::EaseInOut);
        animation.start();
        let board = self.clone();
        self.add_tick_callback(move |_, _frame_clock| {
            if board.imp().view_flight.get() != generation {
                return ControlFlow::Break;
            }
            let Some(view) = animation.value() else {
                return ControlFlow::Break;
            };
            board.pin_view(view);
            if animation.is_complete() { ControlFlow::Break } else { ControlFlow::Continue }
        });
    }

    /// The workspace point currently shown at the widget center - the
    /// natural anchor for button-driven zooms.
    fn view_center(&self) -> Vec2<Pixels> {
        let widget = self.widget_bounds();
        self.view().to_workspace(widget.center(), widget)
    }

    /// `true` when `point` (layer coordinates) lands on empty space -
    /// outside every visible card's rendered footprint. Conservative:
    /// the footprint is the rotated bounding box, so at odd angles the
    /// denied zone is a bit larger than the visible card.
    fn empty_space_at(&self, point: Vec2<Pixels>) -> bool {
        let cards = self.imp().cards.borrow();
        !cards
            .values()
            .any(|entry| entry.card.is_visible() && self.rendered_footprint(entry).contains(&point))
    }

    /// `true` when the drag's start point is on empty space.
    fn drag_starts_on_empty_space(&self, gesture: &GestureDrag) -> bool {
        gesture
            .start_point()
            .map(|(x, y)| self.empty_space_at(Vec2::<Pixels>::new(x, y)))
            .unwrap_or(true)
    }

    /// `true` when every touch point the gesture tracks sits on empty
    /// space - a pinch or twist straddling a card leaves the camera
    /// alone (the card's own gestures get the fingers instead).
    fn gesture_on_empty_space<G: IsA<gtk4::Gesture>>(&self, gesture: &G) -> bool {
        gesture.sequences().iter().all(|sequence| {
            gesture
                .point(Some(sequence))
                .map(|(x, y)| self.empty_space_at(Vec2::<Pixels>::new(x, y)))
                .unwrap_or(true)
        })
    }

    /// Empty-space view gestures on the overlay: drag pans, pinch
    /// zooms around the gesture center, two-finger twist rotates
    /// around it. Attached to the overlay because the card layer
    /// reports `contains() = false` and is never an event target, so
    /// events on empty space bubble up from the content through the
    /// overlay. Each gesture is denied at begin unless `view_gestures`
    /// is on and all its touch points land on empty space; card
    /// gestures deeper in the bubble chain also claim card-bound
    /// sequences first.
    pub(crate) fn wire_view_gestures(&self) {
        let Some(overlay) = self.overlay() else { return };

        let pan = GestureDrag::new();
        pan.set_button(0);
        let board_weak = self.downgrade();
        pan.connect_drag_begin(move |gesture, _, _| {
            if let Some(board) = board_weak.upgrade() {
                board.on_view_pan_begin(gesture);
            }
        });
        let board_weak = self.downgrade();
        pan.connect_drag_update(move |gesture, _, _| {
            if let Some(board) = board_weak.upgrade() {
                board.on_view_pan_update(gesture);
            }
        });
        overlay.add_controller(pan);

        let zoom = GestureZoom::new();
        let board_weak = self.downgrade();
        zoom.connect_begin(move |gesture, _| {
            if let Some(board) = board_weak.upgrade() {
                board.on_view_zoom_begin(gesture);
            }
        });
        let board_weak = self.downgrade();
        zoom.connect_scale_changed(move |gesture, scale| {
            if let Some(board) = board_weak.upgrade() {
                board.on_view_zoom_changed(gesture, scale);
            }
        });
        overlay.add_controller(zoom);

        let rotate = GestureRotate::new();
        let board_weak = self.downgrade();
        rotate.connect_begin(move |gesture, _| {
            if let Some(board) = board_weak.upgrade() {
                board.on_view_rotate_begin(gesture);
            }
        });
        let board_weak = self.downgrade();
        rotate.connect_angle_changed(move |gesture, _angle, delta| {
            if let Some(board) = board_weak.upgrade() {
                board.on_view_rotate_changed(gesture, delta);
            }
        });
        overlay.add_controller(rotate);
    }

    /// Corner view controls: `+`/`-` zoom around the widget center and
    /// `⟲` returns to the default view. Overlaid bottom-right and
    /// visible while `view_buttons` is set.
    pub(crate) fn wire_view_controls(&self) {
        let Some(overlay) = self.overlay() else { return };
        let controls = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        controls.set_halign(Align::End);
        controls.set_valign(Align::End);
        controls.set_margin_end(8);
        controls.set_margin_bottom(8);
        controls.add_css_class(CLASS_VIEW_CONTROLS);
        controls.set_visible(self.imp().view_buttons.get());

        let zoom_in = Button::with_label("+");
        zoom_in.set_tooltip_text(Some(&l10n().message("view-zoom-in")));
        let zoom_out = Button::with_label("−");
        zoom_out.set_tooltip_text(Some(&l10n().message("view-zoom-out")));
        let reset = Button::with_label("⟲");
        reset.set_tooltip_text(Some(&l10n().message("view-reset")));

        let board_weak = self.downgrade();
        zoom_in.connect_clicked(move |_| {
            if let Some(board) = board_weak.upgrade() {
                board.zoom_view(board.view_center(), ViewScaleFactor::new(VIEW_BUTTON_ZOOM_STEP));
            }
        });
        let board_weak = self.downgrade();
        zoom_out.connect_clicked(move |_| {
            if let Some(board) = board_weak.upgrade() {
                board.zoom_view(board.view_center(), ViewScaleFactor::new(1.0 / VIEW_BUTTON_ZOOM_STEP));
            }
        });
        let board_weak = self.downgrade();
        reset.connect_clicked(move |_| {
            if let Some(board) = board_weak.upgrade() {
                board.reset_view();
            }
        });

        controls.append(&zoom_in);
        controls.append(&zoom_out);
        controls.append(&reset);
        overlay.add_overlay(&controls);
        let _ = self.imp().view_controls.set(controls);
    }

    /// Arms a view pan - denied unless `view_gestures` is on and the
    /// drag began on empty space.
    fn on_view_pan_begin(&self, gesture: &GestureDrag) {
        self.imp().view_pan_offset.set(Vec2::ZERO);
        if !self.imp().view_gestures.get() || !self.drag_starts_on_empty_space(gesture) {
            deny_gesture(gesture);
        }
    }

    /// Pans so the workspace point under the drag start follows the
    /// finger: the workspace-space delta between consecutive drag
    /// offsets is applied to the view region.
    fn on_view_pan_update(&self, gesture: &GestureDrag) {
        let (Some((start_x, start_y)), Some((dx, dy))) = (gesture.start_point(), gesture.offset()) else {
            return;
        };
        let previous_offset = self.imp().view_pan_offset.get();
        let offset = Vec2::<Pixels>::new(dx, dy);
        self.imp().view_pan_offset.set(offset);
        let start = Vec2::<Pixels>::new(start_x, start_y);
        let widget = self.widget_bounds();
        let view = self.view();
        let previous = view.to_workspace(start + previous_offset, widget);
        let now = view.to_workspace(start + offset, widget);
        self.pan_view(previous - now);
    }

    /// Arms the view zoom tracker - denied unless `view_gestures` is
    /// on and all touches sit on empty space.
    fn on_view_zoom_begin(&self, gesture: &GestureZoom) {
        self.imp().view_zoom_scale.set(ViewScaleFactor::ONE);
        if !self.imp().view_gestures.get() || !self.gesture_on_empty_space(gesture) {
            deny_gesture(gesture);
        }
    }

    /// Applies the incremental zoom factor around the gesture's
    /// bounding-box center (zoom-to-point in workspace coordinates).
    fn on_view_zoom_changed(&self, gesture: &GestureZoom, scale: f64) {
        let previous = self.imp().view_zoom_scale.get();
        let scale = ViewScaleFactor::new(scale);
        self.imp().view_zoom_scale.set(scale);
        let factor = f64::from(scale / previous);
        if !factor.is_finite() || factor <= 0.0 || (factor - 1.0).abs() < 1e-9 {
            return;
        }
        let Some((center_x, center_y)) = gesture.bounding_box_center() else { return };
        let anchor = self.view().to_workspace(Vec2::<Pixels>::new(center_x, center_y), self.widget_bounds());
        self.zoom_view(anchor, ViewScaleFactor::new(factor));
    }

    /// Arms the view rotate tracker - denied unless `view_gestures` is
    /// on and all touches sit on empty space.
    fn on_view_rotate_begin(&self, gesture: &GestureRotate) {
        self.imp().view_rotate_delta.set(RotationRad::ZERO);
        if !self.imp().view_gestures.get() || !self.gesture_on_empty_space(gesture) {
            deny_gesture(gesture);
        }
    }

    /// Applies the incremental twist around the gesture's
    /// bounding-box center.
    fn on_view_rotate_changed(&self, gesture: &GestureRotate, delta: f64) {
        let previous = self.imp().view_rotate_delta.get();
        let delta = RotationRad::new(delta);
        self.imp().view_rotate_delta.set(delta);
        let step = delta - previous;
        if step.as_f64().abs() < f64::EPSILON {
            return;
        }
        let Some((center_x, center_y)) = gesture.bounding_box_center() else { return };
        let pivot = self.view().to_workspace(Vec2::<Pixels>::new(center_x, center_y), self.widget_bounds());
        self.rotate_view(pivot, step);
    }

    /// The workspace extent mode: `FollowWidget` (default - the
    /// historical behavior where the board is exactly the widget),
    /// `Fixed` or `Infinite`.
    pub fn workspace_size(&self) -> WorkspaceSize<Pixels> {
        self.imp().workspace_size.get()
    }

    /// Sets the workspace extent and re-renders: pending anchors resolve
    /// against the new bounds, all cards re-sync their rendered
    /// transform and get re-clamped.
    pub fn set_workspace_size(&self, size: WorkspaceSize<Pixels>) {
        self.imp().workspace_size.set(size);
        self.on_board_allocated();
        self.emit_geometry_changed();
    }

    /// The camera a card renders through: the board's view for
    /// regular cards, the identity view (`covering` the widget,
    /// rotation zero) for sticky cards pinned to the screen - so
    /// `to_widget`, `to_workspace`, `+ rotation` and `x zoom` all
    /// become no-ops for them.
    pub(crate) fn card_view(&self, entry: &CardEntry) -> View<Pixels> {
        match entry.sticky.get() {
            true => View::covering(self.widget_bounds()),
            false => self.view(),
        }
    }

    /// The view zoom a card renders at - `1.0` for sticky cards, which
    /// carry their on-screen scale in `entry.scale` directly.
    pub(crate) fn card_view_zoom(&self, entry: &CardEntry) -> f64 {
        match entry.sticky.get() {
            true => 1.0,
            false => self.view_zoom(),
        }
    }

    /// Uniform scale of the view mapping - widget pixels per workspace
    /// pixel. `1.0` while the widget or the shown region is degenerate.
    fn view_zoom(&self) -> f64 {
        let widget = self.widget_bounds();
        let shown = self.view().shown_region(widget.size);
        if widget.size.x <= 0.0 || shown.size.x <= 0.0 {
            return 1.0;
        }
        widget.size.x / shown.size.x
    }

    /// Re-renders every card through the current view and re-clamps -
    /// after a view or workspace change.
    fn on_view_changed(&self) {
        {
            let cards = self.imp().cards.borrow();
            for entry in cards.values() {
                self.sync_rendered_transform(entry);
            }
        }
        // An infinite workspace clamps card *operations* against the
        // view region; re-clamping on view changes would pull
        // out-of-view cards along with the camera and lock panning to
        // the card cluster. For `FollowWidget`/`Fixed` the bounds are
        // view-independent, so this is a cheap no-op.
        if !matches!(self.imp().workspace_size.get(), WorkspaceSize::Infinite) {
            self.clamp_all();
        }
        self.emit_geometry_changed();
    }

    /// Re-composes the transform stack for the current view: rendered
    /// rotation = card rotation + view rotation, rendered scale =
    /// card scale x view zoom, position = `to_widget(center)`.
    /// `notify::rotation` syncs the model rotation back.
    fn sync_rendered_transform(&self, entry: &CardEntry) {
        let rendered_rotation = entry.rotation.get() + self.card_view(entry).rotation;
        if rendered_rotation.signed_delta_to(entry.rotation_widget.rotation()).abs() > RotationRad::from_degrees(0.0001) {
            entry.rotation_widget.set_rotation(SmearorRotation::from(rendered_rotation));
        }
        let rendered_scale = entry.scale.get().value() * self.card_view_zoom(entry);
        if (entry.scaling.scale() - rendered_scale).abs() > 1e-6 {
            entry.scaling.set_scale(rendered_scale);
        }
        self.apply_position(entry);
    }

    /// Card footprint in workspace coordinates: the displayed
    /// (unscaled by the view) size centered on the stored position.
    /// Used for clamping, layout strategies and seat fan-out.
    fn workspace_footprint(&self, entry: &CardEntry) -> Rect<Pixels> {
        self.workspace_footprint_at(entry, &self.entry_geometry(entry))
    }

    /// Workspace footprint for an arbitrary `geometry` (an in-flight
    /// glide target): the natural size at `geometry`'s scale rotated
    /// by its rotation, centered on its position.
    fn workspace_footprint_at(&self, entry: &CardEntry, geometry: &CardGeometry) -> Rect<Pixels> {
        let (_, natural) = entry.card.measure_size();
        let size = (natural.to_f64().with_unit::<Pixels>() * geometry.scale.value()).rotated_bounds(geometry.rotation.to_rotation_rad());
        let size = match size.x > 0.0 && size.y > 0.0 {
            true => size,
            false => self.rendered_size(entry) / self.card_view_zoom(entry),
        };
        Rect::centered(geometry.position.into(), size)
    }

    /// Footprint rectangle of the transform stack in layer (widget)
    /// coordinates: the rendered size centered on the view-mapped
    /// position. Used for `gtk4::Fixed` placement and z-restacking.
    fn rendered_footprint(&self, entry: &CardEntry) -> Rect<Pixels> {
        self.rendered_footprint_at(entry, &self.entry_geometry(entry))
    }

    /// Rendered footprint for an arbitrary `geometry` (an in-flight
    /// glide target): the natural size at `geometry`'s scale times
    /// view zoom, rotated by its rotation + view rotation, centered
    /// on the view-mapped position. Same degenerate-bounds fallback
    /// as [`Self::rendered_footprint`].
    fn rendered_footprint_at(&self, entry: &CardEntry, geometry: &CardGeometry) -> Rect<Pixels> {
        let view = self.card_view(entry);
        let (_, natural) = entry.card.measure_size();
        let size = (natural.to_f64().with_unit::<Pixels>() * (geometry.scale.value() * self.card_view_zoom(entry)))
            .rotated_bounds(geometry.rotation.to_rotation_rad() + view.rotation);
        let size = match size.x > 0.0 && size.y > 0.0 {
            true => size,
            false => entry.scaling.allocated_size().to_f64().with_unit::<Pixels>(),
        };
        let widget = self.widget_bounds();
        let center = match widget.is_degenerate() {
            true => geometry.position.into(),
            false => view.to_widget(geometry.position.into(), widget),
        };
        Rect::centered(center, size)
    }

    /// The size `gtk4::Fixed` allocates a transform stack at
    /// `geometry`: minimum width plus natural height at that width -
    /// Fixed's child contract is `(min, natural)`, not `(natural,
    /// natural)`. `ScalingLayout` then centers the natural-size child
    /// inside that allocation, so a stack moved to the *natural*
    /// footprint min renders displaced by `(nat - min) / 2`.
    fn stack_alloc_size(&self, entry: &CardEntry, geometry: &CardGeometry) -> Vec2<Pixels> {
        let (minimum, natural) = entry.card.measure_size();
        let scale = geometry.scale.value() * self.card_view_zoom(entry);
        let rotation = geometry.rotation.to_rotation_rad() + self.card_view(entry).rotation;
        let min_size = (minimum.to_f64().with_unit::<Pixels>() * scale).rotated_bounds(rotation);
        let nat_size = (natural.to_f64().with_unit::<Pixels>() * scale).rotated_bounds(rotation);
        Vec2::new(min_size.x, nat_size.y)
    }

    /// Top-left for `entry.scaling` on the card layer that centers the
    /// rendered stack on the view-mapped card position: the footprint
    /// center minus half the [`Self::stack_alloc_size`].
    fn stack_top_left(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let footprint = self.rendered_footprint(entry);
        let center = footprint.center();
        let alloc = self.stack_alloc_size(entry, &self.entry_geometry(entry));
        Vec2::new(center.x - alloc.x / 2.0, center.y - alloc.y / 2.0)
    }

    /// The geometry a card is effectively at for obstacle purposes:
    /// its in-flight glide target while the glide runs (its
    /// `layout_generation` still matches), otherwise the live
    /// geometry. Fan-out treats a card heading for a seat as already
    /// there, so passes during the animation do not stack on it.
    fn effective_geometry(&self, entry: &CardEntry) -> CardGeometry {
        match entry.glide_target.get() {
            Some((generation, target)) if generation == entry.layout_generation.get() => target,
            _ => self.entry_geometry(entry),
        }
    }

    /// Positions the transform stack so its rendered center matches the
    /// view-mapped `entry.position` (`gtk4::Fixed` positions by
    /// top-left). No-ops when the stack already sits at the target
    /// spot, avoiding needless relayouts.
    fn apply_position(&self, entry: &CardEntry) {
        let Some(layer) = self.card_layer() else {
            return;
        };
        let target = self.stack_top_left(entry);
        let current = Vec2::<Pixels>::from(layer.child_position(&entry.scaling));
        let delta = current - target;
        if delta.x.abs() > 0.5 || delta.y.abs() > 0.5 {
            layer.move_(&entry.scaling, target.x, target.y);
        }
        self.sync_satellites(entry);
    }

    /// The rect a card's footprint may occupy: the workspace extent -
    /// except under `SeatAnchor::View`, where the visible surface is
    /// the user-facing domain: the shown region maps onto the widget
    /// rect exactly, so cards may rest anywhere on screen (seats live
    /// at screen edges there, and `rotate_to_next_user` parks even
    /// unassigned cards at them). The domain is the bounding rect of
    /// extent and shown region - convex, so glide paths between
    /// workspace interior and screen edge stay inside.
    fn clamp_bounds(&self, entry: &CardEntry) -> Rect<Pixels> {
        // A sticky card lives on the glass: its bounds are the widget
        // rect itself, unaffected by workspace or camera.
        if entry.sticky.get() {
            return self.widget_bounds();
        }
        let workspace = self.workspace_bounds();
        match self.imp().seat_anchor.get() {
            SeatAnchor::View => {
                let shown = self.view().shown_region(self.widget_bounds().size);
                Rect::from_points([workspace.min, workspace.max(), shown.min, shown.max()]).unwrap_or(workspace)
            }
            SeatAnchor::Workspace => workspace,
        }
    }

    /// Pushes the card back inside [`Self::clamp_bounds`].
    /// `BoundsMode::Loose` cards keep only `min_visible` workspace
    /// pixels inside; `Contained` cards get their whole footprint
    /// pulled back.
    fn clamp_entry(&self, entry: &CardEntry) {
        let bounds = self.clamp_bounds(entry);
        if bounds.is_degenerate() {
            return;
        }
        let footprint = self.workspace_footprint(entry);
        let delta = match entry.config.bounds_mode {
            BoundsMode::Loose => footprint.clamp_offset(&bounds, entry.config.min_visible.value()),
            // Contained clamps the union of card and visible satellite
            // bounds - satellites are kept inside the board with it.
            BoundsMode::Contained => self.footprint_with_satellites(entry, footprint).contain_offset(&bounds),
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

    /// `Contained` clamp footprint: the union of the card's workspace
    /// footprint and every *visible* satellite's resolved bounds
    /// (P2.4) - a dragged card pulls its satellites back inside the
    /// board with it. Satellites without a placement (hidden, or not
    /// yet synced) contribute nothing.
    fn footprint_with_satellites(&self, entry: &CardEntry, card_footprint: Rect<Pixels>) -> Rect<Pixels> {
        let mut points = vec![card_footprint.min, card_footprint.max()];
        for satellite in entry.satellites.borrow().iter() {
            if !satellite.scaling.is_visible() {
                continue;
            }
            let Some(geometry) = satellite.last_geometry.get() else {
                continue;
            };
            let (_, natural) = satellite.widget.measure_size();
            let size = (natural.to_f64().with_unit::<Pixels>() * geometry.scale.value()).rotated_bounds(geometry.rotation.to_rotation_rad());
            let rect = Rect::centered(geometry.position.into(), size);
            points.push(rect.min);
            points.push(rect.max());
        }
        Rect::from_points(points).unwrap_or(card_footprint)
    }

    /// The card's extent in workspace units: the natural size at the
    /// card's own scale, rotated by the card rotation - the footprint
    /// layout, clamping and anchors see (view-independent). Falls back
    /// to the rendered size divided by the view zoom before measuring
    /// is possible.
    fn workspace_card_size(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let size = self.predicted_workspace_size(entry);
        if size.x > 0.0 && size.y > 0.0 {
            size
        } else {
            self.rendered_size(entry) / self.card_view_zoom(entry)
        }
    }

    /// The size the transform stack occupies on the layer in widget
    /// pixels: the predicted rendered size (natural size at card scale
    /// x view zoom, rotated by card + view rotation), falling back to
    /// the allocated size before measuring is possible.
    fn rendered_size(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let size = self.predicted_rendered_size(entry);
        if size.x > 0.0 && size.y > 0.0 {
            size
        } else {
            entry.scaling.allocated_size().to_f64().with_unit::<Pixels>()
        }
    }

    /// Predicted workspace extent: rotated bounding box of the card's
    /// natural size times the card scale.
    fn predicted_workspace_size(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let (_, natural) = entry.card.measure_size();
        (natural.to_f64().with_unit::<Pixels>() * entry.scale.get().value()).rotated_bounds(entry.rotation.get())
    }

    /// Predicted rendered extent: rotated bounding box of the card's
    /// natural size times the rendered scale (card scale x view zoom).
    fn predicted_rendered_size(&self, entry: &CardEntry) -> Vec2<Pixels> {
        let (_, natural) = entry.card.measure_size();
        let scale = entry.scale.get().value() * self.card_view_zoom(entry);
        (natural.to_f64().with_unit::<Pixels>() * scale).rotated_bounds(entry.rotation.get() + self.card_view(entry).rotation)
    }

    pub(crate) fn entry_geometry(&self, entry: &CardEntry) -> CardGeometry {
        CardGeometry {
            position: entry.position.get(),
            rotation: Rotation::quantized(entry.rotation.get()),
            scale: entry.scale.get(),
        }
    }

    /// Technical layout input for a card: displayed footprint, live
    /// rotation and explicit seat assignment.
    fn layout_item(&self, entry: &CardEntry) -> LayoutItem<Pixels> {
        LayoutItem {
            size: self.workspace_card_size(entry),
            rotation: Rotation::quantized(entry.rotation.get()),
            seat: entry.seat.get(),
        }
    }

    /// The card's effective seat for passing operations: its explicit
    /// assignment or the seat nearest its live rotation.
    fn card_seat_or_nearest(&self, id: &CardId) -> Result<Seat, BoardError> {
        let cards = self.imp().cards.borrow();
        let Some(entry) = cards.get(id) else {
            return Err(BoardError::UnknownCard(id.clone()));
        };
        Ok(entry.seat.get().unwrap_or_else(|| {
            // `View` derives the facing side from the rendered
            // rotation (what users see on screen); `Workspace` uses
            // the card's table-space rotation.
            match self.imp().seat_anchor.get() {
                SeatAnchor::View => Seat::nearest(entry.rotation.get() + self.card_view(entry).rotation),
                SeatAnchor::Workspace => Seat::nearest(self.entry_geometry(entry).rotation.to_rotation_rad()),
            }
        }))
    }

    /// Shared tail of the pass-to-seat operations: computes the target
    /// placement for `target_seat`, persists the seat
    /// (`card-seat-changed`) when the card already had an explicit
    /// assignment or `assign_seat` asks for it, and glides the card
    /// there.
    fn glide_card_to_seat(&self, id: &CardId, target_seat: Seat, move_to_edge: bool, assign_seat: bool) -> Result<(), BoardError> {
        // Passing a pinned card sends it onto the table: release the
        // screen pin first so the seat target lands in workspace
        // coordinates (the conversion keeps it visually in place).
        self.set_card_sticky(id, false)?;
        let anchor = self.imp().seat_anchor.get();
        let view = self.view();
        let widget = self.widget_bounds();
        let zoom = self.view_zoom();
        let (geometry, footprint, had_seat, margin) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            let geometry = self.entry_geometry(entry);
            // The footprint is expressed at the target orientation, so
            // the placement centers the *rotated* bounding box. `View`
            // works in screen space: the rendered size at the rendered
            // current rotation, the screen-pixel margin.
            let turn = target_seat.side.to_quarter_turn();
            let footprint = match anchor {
                SeatAnchor::View => footprint_for(self.rendered_size(entry), Rotation::quantized(entry.rotation.get() + view.rotation), turn),
                SeatAnchor::Workspace => footprint_for(self.workspace_card_size(entry), geometry.rotation, turn),
            };
            (
                geometry,
                footprint,
                entry.seat.get().is_some(),
                match anchor {
                    SeatAnchor::View => entry.config.anchor_margin.as_f64() * zoom,
                    SeatAnchor::Workspace => entry.config.anchor_margin.as_f64(),
                },
            )
        };

        if had_seat || assign_seat {
            self.set_card_seat(id, Some(target_seat))?;
        }

        let bounds = self.seat_bounds();
        let placement = if move_to_edge && !bounds.is_degenerate() {
            // Every other card physically occupies space near the seat:
            // the fan-out solver slips the passed card into the nearest
            // free lane slot beside them instead of stacking on top.
            // `View` runs the whole lane in screen space.
            let taken: Vec<(Vec2<Pixels>, Vec2<Pixels>)> = self
                .imp()
                .cards
                .borrow()
                .iter()
                .filter(|(other_id, entry)| *other_id != id && (anchor == SeatAnchor::View || !entry.sticky.get()))
                .map(|(_, entry)| {
                    // Cards gliding to a seat count at their
                    // destination: a pass issued while a sibling is
                    // still moving fans out around its landing spot
                    // instead of stacking on it.
                    let geometry = self.effective_geometry(entry);
                    let footprint = match anchor {
                        SeatAnchor::View => self.rendered_footprint_at(entry, &geometry),
                        SeatAnchor::Workspace => self.workspace_footprint_at(entry, &geometry),
                    };
                    (footprint.center(), footprint.size)
                })
                .collect();
            target_seat.fanned_placement(bounds, footprint, margin, &taken, margin)
        } else {
            target_seat.placement(bounds, footprint, margin)
        };
        let target = CardGeometry {
            // Degenerate bounds (unrealized board) fall back to
            // rotation-in-place, like `clamp_entry` early-returns.
            position: if move_to_edge && !bounds.is_degenerate() {
                CardPosition::from(match anchor {
                    SeatAnchor::View => view.to_workspace(placement.center, widget),
                    SeatAnchor::Workspace => placement.center,
                })
            } else {
                geometry.position
            },
            // Under `View` the placement's quarter turn is a *screen*
            // orientation: subtracting the camera rotation makes the
            // rendered card land on it. Under `Workspace` it is the
            // table-space rotation already.
            rotation: match anchor {
                SeatAnchor::View => Rotation::quantized(placement.rotation.to_rotation_rad() - view.rotation),
                SeatAnchor::Workspace => placement.rotation,
            },
            scale: geometry.scale,
        };

        self.glide_card_geometry(id, target)
    }

    /// Glides a card towards `target` on the frame clock. Each tick
    /// applies the eased `CardGeometry` through the regular
    /// `set_card_geometry` path, so re-centering, clamping and signal
    /// emission stay identical to an instant update.
    ///
    /// Starting a new glide bumps the card's `layout_generation`;
    /// ticks of a superseded animation notice the stale generation and
    /// stop themselves.
    ///
    /// Returns [`BoardError::UnknownCard`] for an id not on the board.
    pub fn glide_card_geometry(&self, id: &CardId, target: CardGeometry) -> Result<(), BoardError> {
        let (scaling, from, duration, generation) = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return Err(BoardError::UnknownCard(id.clone()));
            };
            let generation = entry.layout_generation.get().incremented();
            entry.layout_generation.set(generation);
            // Seat fan-out reads the destination as the card's
            // effective geometry, so passes during the glide fan out
            // around the landing spot instead of the mid-flight one.
            entry.glide_target.set(Some((generation, target)));
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
        scaling.add_tick_callback(move |_, _frame_clock| {
            let Some(geometry) = animation.value() else {
                return ControlFlow::Break;
            };
            {
                let cards = board.imp().cards.borrow();
                match cards.get(&id) {
                    Some(entry) if entry.layout_generation.get() == generation => {}
                    _ => return ControlFlow::Break,
                }
            }
            board.set_card_geometry(&id, &geometry);
            if animation.is_complete() {
                let cards = board.imp().cards.borrow();
                if let Some(entry) = cards.get(&id) {
                    entry.glide_target.take();
                }
                return ControlFlow::Break;
            }
            ControlFlow::Continue
        });
        Ok(())
    }
}
