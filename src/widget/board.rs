use crate::domain::BoardError;
use crate::domain::BoardLayout;
use crate::domain::BoundsMode;
use crate::domain::CardAnchor;
use crate::domain::CardGeometry;
use crate::domain::CardId;
use crate::domain::CardPlacement;
use crate::domain::CardPosition;
use crate::domain::Rect;
use crate::domain::ScaleFactor;
use crate::domain::clamp_offset;
use crate::domain::contain_offset;
use crate::domain::resolve_anchor;
use crate::domain::rotate_vector;
use crate::domain::rotated_bounds;
use crate::domain::rotated_region_scanlines;
use crate::widget::card::Card;
use crate::widget::card_config::CardConfig;
use crate::widget::card_layer::CardLayer;
use crate::widget::css::ensure_default_css;
use crate::widget::imp::board::CardEntry;
use crate::widget::imp::board::FloatingBoardImpl;
use crate::widget::scaling::ScalingWidget;
use glib::object::Cast;
use glib::object::IsA;
use glib::prelude::ObjectExt;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;

use gtk4::Orientable;
use gtk4::Orientation;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::cairo::RectangleInt;
use gtk4::cairo::Region;
use gtk4::glib;
use gtk4::graphene::Point;
use gtk4::prelude::EventControllerExt;
use gtk4::prelude::FixedExt;
use gtk4::prelude::GestureDragExt;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use smearor_wrot_rotation::RotationControlHandler;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::cell::Cell;
use std::cell::RefCell;
use tracing::debug;
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
            let sequence = self.imp().sequence.get() + 1;
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
        let rotation_widget = RotationWidget::new(SmearorRotation::Deg0)
            .with_gesture_rotation_enabled(config.rotatable)
            // The card is nested inside the board's `Fixed` layer - the
            // surface-global input-region clamp would kill input for the
            // rest of the window (`smearor-wrot-rotation` >= 0.2.1).
            .with_input_region_enabled(false);
        rotation_widget.set_child(Some(&card));
        let scaling = ScalingWidget::new(&rotation_widget);
        layer.put(&scaling, 0.0, 0.0);

        let sequence = self.imp().sequence.replace(self.imp().sequence.get() + 1);
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
            pinch_start: Cell::new(1.0),
            sequence,
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
        self.emit_by_name::<()>("card-added", &[&card]);
        Ok(id)
    }

    /// Removes a card from the board.
    pub fn remove_card(&self, id: &CardId) -> Result<(), BoardError> {
        let entry = self.imp().cards.borrow_mut().remove(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
        if let Some(layer) = self.card_layer() {
            layer.remove(&entry.scaling);
        }
        debug!(card = %id, "card removed");
        self.emit_by_name::<()>("card-removed", &[&entry.card]);
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

    /// Snapshot of every card's geometry for consumer-side persistence.
    pub fn layout_snapshot(&self) -> BoardLayout {
        let cards = self.imp().cards.borrow();
        BoardLayout::new(
            cards
                .iter()
                .map(|(id, entry)| CardPlacement::new(id.clone(), self.entry_geometry(entry), entry.card.folded()))
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
            rotation_widget.set_rotation(placement.geometry.rotation);
            scaling.set_scale(placement.geometry.scale.value());
            card.set_folded(placement.folded);
            let cards = self.imp().cards.borrow();
            if let Some(entry) = cards.get(&placement.id) {
                self.apply_position(entry);
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
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
        rotation_widget.set_rotation(geometry.rotation);
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
    pub(crate) fn set_card_rotation(&self, id: &CardId, rotation: SmearorRotation) {
        let rotation_widget = self.imp().cards.borrow().get(id).map(|entry| entry.rotation_widget.clone());
        if let Some(rotation_widget) = rotation_widget {
            // The notify::rotation handler performs re-centering, clamping and
            // signal emission.
            rotation_widget.set_rotation(rotation);
        }
    }

    /// Sets the card rotation with the zoom-snap animation.
    pub(crate) fn set_card_rotation_animated(&self, id: &CardId, rotation: SmearorRotation) {
        let rotation_widget = self.imp().cards.borrow().get(id).map(|entry| entry.rotation_widget.clone());
        if let Some(rotation_widget) = rotation_widget {
            rotation_widget.set_rotation_with_animation(rotation.to_degrees() as f64);
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
        rotation_widget.set_rotation(SmearorRotation::Deg0);
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
                    if bounds.is_empty() {
                        entry.pending_anchor.replace(Some(anchor));
                    } else {
                        entry.pending_anchor.borrow_mut().take();
                        let (width, height) = self.predicted_size(entry);
                        entry.position.set(resolve_anchor(&anchor, &bounds, width, height, entry.config.anchor_margin));
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
            let (width, height) = self.displayed_size(entry);
            let position = entry.position.get();
            layer.remove(&entry.scaling);
            layer.put(&entry.scaling, position.x() - width / 2.0, position.y() - height / 2.0);
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
            if let Err(error) = region.union(&self.entry_footprint(entry)) {
                warn!(%error, "failed to union card footprint");
            }
        }
        region
    }

    /// Rotated footprint rasterized from the card's natural size, scale and
    /// rotation, centered on its board position.
    fn entry_footprint(&self, entry: &CardEntry) -> Region {
        let (_, natural_width, _, _) = entry.card.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = entry.card.measure(Orientation::Vertical, -1);
        let scale = entry.scale.get().value();
        let degrees = entry.rotation_widget.rotation() as f64;
        let position = entry.position.get();
        let center_x = position.x().round() as i32;
        let center_y = position.y().round() as i32;
        let region = Region::create();
        for (y, x_start, x_end) in rotated_region_scanlines(natural_width as f64 * scale, natural_height as f64 * scale, degrees) {
            let strip = RectangleInt::new(center_x + x_start, center_y + y, x_end - x_start, 1);
            if let Err(error) = region.union_rectangle(&strip) {
                warn!(%error, "failed to union footprint strip");
            }
        }
        region
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
        if bounds.is_empty() {
            return;
        }
        let mut resolved = false;
        {
            let cards = self.imp().cards.borrow();
            for entry in cards.values() {
                if let Some(anchor) = entry.pending_anchor.borrow_mut().take() {
                    let (width, height) = self.predicted_size(entry);
                    entry.position.set(resolve_anchor(&anchor, &bounds, width, height, entry.config.anchor_margin));
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
            entry.scaling.queue_resize();
            self.apply_position(entry);
            self.clamp_entry(entry);
            entry.card.clone()
        };
        card.emit_card_rotated();
        self.emit_geometry_changed();
    }

    fn on_card_pinch_begin(&self, id: &CardId) {
        if let Some(entry) = self.imp().cards.borrow().get(id) {
            entry.pinch_start.set(entry.scale.get().value());
        }
    }

    /// Live pinch zoom: `scale = start * zoom`, clamped to `scale_limits`.
    ///
    /// The card grows around the pinch midpoint - the card point under the
    /// fingers stays fixed - instead of around the card center, which would
    /// visually drift the card away from the gesture.
    fn on_card_pinch_changed(&self, id: &CardId, gesture: &gtk4::GestureZoom, zoom: f64) {
        {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let previous = entry.scale.get().value();
            let scale = ScaleFactor::new(entry.pinch_start.get() * zoom).clamped(&entry.config.scale_limits);
            if let Some((mid_x, mid_y)) = self.pinch_midpoint_in_layer(gesture, entry)
                && scale.value() != previous
            {
                let position = entry.position.get();
                let ratio = scale.value() / previous;
                entry
                    .position
                    .set(CardPosition::new(mid_x - (mid_x - position.x()) * ratio, mid_y - (mid_y - position.y()) * ratio));
            }
            entry.scale.set(scale);
            entry.scaling.set_scale(scale.value());
            self.apply_position(entry);
            if entry.config.bounds_mode == BoundsMode::Contained {
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
    }

    /// Pinch midpoint in card-layer (board) coordinates. `gesture.point()`
    /// reports in `scaling` widget coordinates, which are untransformed - the
    /// scale transform applies to its child only - so adding the stack's
    /// top-left layer offset yields board space.
    fn pinch_midpoint_in_layer(&self, gesture: &gtk4::GestureZoom, entry: &CardEntry) -> Option<(f64, f64)> {
        let (px, py) = gesture.point(None)?;
        let layer = self.card_layer()?;
        let (offset_x, offset_y) = layer.child_position(&entry.scaling);
        Some((offset_x + px, offset_y + py))
    }

    fn on_card_pinch_end(&self, id: &CardId) {
        let card = {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            self.clamp_entry(entry);
            entry.card.clone()
        };
        card.emit_card_scaled();
        self.emit_geometry_changed();
    }

    /// Records the card position and the pointer's board coordinates at drag
    /// begin; the update delta is measured between two board-space pointer
    /// samples, which stays stable while the card follows the pointer.
    fn on_card_drag_begin(&self, id: &CardId, gesture: &gtk4::GestureDrag) {
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
    fn on_card_drag_update(&self, id: &CardId, gesture: &gtk4::GestureDrag, offset_x: f64, offset_y: f64) {
        {
            let cards = self.imp().cards.borrow();
            let Some(entry) = cards.get(id) else {
                return;
            };
            let delta = self.drag_delta(gesture, offset_x, offset_y, entry);
            entry.position.set(entry.drag_start.get().translated(delta.0, delta.1));
            self.apply_position(entry);
            if entry.config.bounds_mode == BoundsMode::Contained {
                self.clamp_entry(entry);
            }
        }
        self.emit_geometry_changed();
    }

    fn on_card_drag_end(&self, id: &CardId, gesture: &gtk4::GestureDrag, offset_x: f64, offset_y: f64) {
        self.on_card_drag_update(id, gesture, offset_x, offset_y);
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

    /// Pointer position in card-layer (board) coordinates, taken from the
    /// event's surface coordinates - independent of the card's own transform.
    fn pointer_in_layer(&self, gesture: &gtk4::GestureDrag) -> Option<(f64, f64)> {
        let event = gesture.current_event()?;
        let (x, y) = event.position()?;
        let layer = self.card_layer()?;
        let native = layer.native()?;
        let point = native.compute_point(&layer, &Point::new(x as f32, y as f32))?;
        Some((point.x() as f64, point.y() as f64))
    }

    /// Drag delta in board coordinates: current pointer position minus the
    /// pointer position captured at drag begin. Falls back to undoing the
    /// inverse `rotate . scale` on the widget-space offset before the first
    /// allocation, when no surface mapping is available yet.
    fn drag_delta(&self, gesture: &gtk4::GestureDrag, offset_x: f64, offset_y: f64, entry: &CardEntry) -> (f64, f64) {
        if let (Some(now), Some((grab_x, grab_y))) = (self.pointer_in_layer(gesture), entry.drag_grab.get()) {
            return (now.0 - grab_x, now.1 - grab_y);
        }
        let degrees = entry.rotation_widget.rotation() as f64;
        let scale = entry.scale.get().value();
        let (dx, dy) = rotate_vector(offset_x, offset_y, degrees);
        (dx * scale, dy * scale)
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
            let (width, height) = self.displayed_size(entry);
            let position = entry.position.get();
            layer.remove(&entry.scaling);
            layer.put(&entry.scaling, position.x() - width / 2.0, position.y() - height / 2.0);
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

        if config.scalable {
            let zoom = gtk4::GestureZoom::new();
            let board_weak = self.downgrade();
            let card_id = id.clone();
            zoom.connect_begin(move |_, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_pinch_begin(&card_id);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            zoom.connect_scale_changed(move |gesture, zoom| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_pinch_changed(&card_id, gesture, zoom);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            zoom.connect_end(move |_, _| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_pinch_end(&card_id);
                }
            });
            scaling.add_controller(zoom);
        }

        // Raise on release: a click raises the card, and a finished drag ends
        // raised too. Raising on press would cancel the in-flight sequence.
        let click = gtk4::GestureClick::new();
        click.set_button(0);
        let board_weak = self.downgrade();
        let card_id = id.clone();
        click.connect_released(move |_, _, _, _| {
            if let Some(board) = board_weak.upgrade() {
                board.raise_card(&card_id);
            }
        });
        scaling.add_controller(click);

        if config.draggable {
            let drag = gtk4::GestureDrag::new();
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
                    board.on_card_drag_update(&card_id, gesture, offset_x, offset_y);
                }
            });
            let board_weak = self.downgrade();
            let card_id = id.clone();
            drag.connect_drag_end(move |gesture, offset_x, offset_y| {
                if let Some(board) = board_weak.upgrade() {
                    board.on_card_drag_end(&card_id, gesture, offset_x, offset_y);
                }
            });
            card.drag_source(config.drag_handle).add_controller(drag);
        }
    }

    /// Emits `geometry-changed` on the board.
    pub(crate) fn emit_geometry_changed(&self) {
        self.emit_by_name::<()>("geometry-changed", &[]);
    }

    fn card_layer(&self) -> Option<CardLayer> {
        self.imp().card_layer.get().cloned()
    }

    fn board_bounds(&self) -> Rect {
        match self.card_layer() {
            Some(layer) => Rect::new(0.0, 0.0, layer.width() as f64, layer.height() as f64),
            None => Rect::default(),
        }
    }

    /// Positions the transform stack so its displayed center matches
    /// `entry.position` (`gtk4::Fixed` positions by top-left). No-ops when the
    /// stack already sits at the target spot, avoiding needless relayouts.
    fn apply_position(&self, entry: &CardEntry) {
        let Some(layer) = self.card_layer() else {
            return;
        };
        let (width, height) = self.displayed_size(entry);
        let position = entry.position.get();
        let target_x = position.x() - width / 2.0;
        let target_y = position.y() - height / 2.0;
        let (current_x, current_y) = layer.child_position(&entry.scaling);
        if (current_x - target_x).abs() > 0.5 || (current_y - target_y).abs() > 0.5 {
            layer.move_(&entry.scaling, target_x, target_y);
        }
    }

    /// Pushes the card back inside the board bounds. `BoundsMode::Loose`
    /// cards keep only `min_visible` pixels inside; `Contained` cards get
    /// their whole footprint pulled back.
    fn clamp_entry(&self, entry: &CardEntry) {
        let bounds = self.board_bounds();
        if bounds.is_empty() {
            return;
        }
        let (width, height) = self.displayed_size(entry);
        let footprint = Rect::centered(entry.position.get(), width, height);
        let (dx, dy) = match entry.config.bounds_mode {
            BoundsMode::Loose => clamp_offset(&footprint, &bounds, entry.config.min_visible),
            BoundsMode::Contained => contain_offset(&footprint, &bounds),
        };
        if dx != 0.0 || dy != 0.0 {
            entry.position.set(entry.position.get().translated(dx, dy));
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
    fn displayed_size(&self, entry: &CardEntry) -> (f64, f64) {
        let (width, height) = self.predicted_size(entry);
        if width > 0.0 && height > 0.0 {
            (width, height)
        } else {
            (entry.scaling.width() as f64, entry.scaling.height() as f64)
        }
    }

    /// Predicted displayed size: rotated bounding box of the card's natural
    /// size, times the scale factor.
    fn predicted_size(&self, entry: &CardEntry) -> (f64, f64) {
        let (_, natural_width, _, _) = entry.card.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = entry.card.measure(Orientation::Vertical, -1);
        let scale = entry.scale.get().value();
        let degrees = entry.rotation_widget.rotation() as f64;
        rotated_bounds(natural_width as f64 * scale, natural_height as f64 * scale, degrees)
    }

    fn entry_geometry(&self, entry: &CardEntry) -> CardGeometry {
        CardGeometry {
            position: entry.position.get(),
            rotation: SmearorRotation::new(entry.rotation_widget.rotation()),
            scale: entry.scale.get(),
        }
    }
}
