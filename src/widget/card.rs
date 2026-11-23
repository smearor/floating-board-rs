use crate::domain::BoardError;
use crate::domain::CardGeometry;
use crate::domain::CardId;
use crate::domain::CardPalette;
use crate::domain::CardPosition;
use crate::domain::CardShape;
use crate::domain::CircleContent;
use crate::domain::DragHandle;
use crate::domain::ScaleFactor;
use crate::domain::TurnDirection;
use crate::l10n::l10n;
use crate::widget::board::FloatingBoard;
use crate::widget::card_config::CardConfig;
use crate::widget::card_layer::CardLayer;
use crate::widget::card_signal::CardSignal;
use crate::widget::circular_clip::CircularClip;
use crate::widget::css::CLASS_CARD_CIRCLE;
use crate::widget::css::CLASS_COLLAPSED;
use crate::widget::css::CLASS_FOLDED;
use crate::widget::css::CLASS_HEADERLESS;
use crate::widget::css::install_palette;
use crate::widget::imp::card::CardImpl;
use glib::object::Cast;
use glib::object::ObjectExt;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::Orientable;
use gtk4::Widget;
use gtk4::cairo::Region;
use gtk4::glib;
use gtk4::prelude::BoxExt;
use gtk4::prelude::ButtonExt;
use gtk4::prelude::FixedExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::QuarterTurn;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::RotationDirection;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use smearor_commons_layout::Seat;
use tracing::debug;

glib::wrapper! {
    /// A floating card on the board: header strip plus consumer content.
    ///
    /// Cards are views on board-owned state - `geometry`, `position`,
    /// `rotation` and `scale` always delegate to the `FloatingBoard` this card
    /// belongs to.
    ///
    /// # Signals
    ///
    /// `card-moved`, `card-rotated`, `card-scaled`, `card-raised`, `card-folded`,
    /// `card-seat-changed`
    pub struct Card(ObjectSubclass<CardImpl>)
        @extends Widget, gtk4::Box,
        @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

impl Card {
    /// Builds a card owned by `board`. Called from `FloatingBoard::add_card`.
    pub(crate) fn build(id: &CardId, content: &Widget, board: &FloatingBoard, config: &CardConfig) -> Self {
        let card: Self = glib::Object::builder().build();
        let imp = card.imp();
        imp.id.replace(id.clone());
        imp.board.replace(board.downgrade());
        imp.max_width.set(config.max_width);
        imp.max_height.set(config.max_height);

        imp.shape.set(config.shape);
        imp.circle_content.set(config.circle_content);
        if config.shape == CardShape::Circle {
            card.add_css_class(CLASS_CARD_CIRCLE);
        }
        if config.headerless {
            card.add_css_class(CLASS_HEADERLESS);
        }
        if let Some(palette) = &config.palette {
            imp.palette.replace(install_palette(&card, palette));
        }
        // Chrome is composed from `config.effective_chrome_slots()`:
        // inline elements mount into a header strip prepended to the
        // card, grips onto the content overlay, satellite placements
        // are validated and deferred to Phase 2/3.
        card.mount_chrome(config);
        if let Some(label) = imp.title_label.get() {
            label.set_visible(!config.headerless);
        }
        if config.shape == CardShape::Circle
            && config.circle_content == CircleContent::Clipped
            && let (Some(overlay), Some(holder)) = (imp.content_overlay.get(), imp.content_holder.get())
        {
            // `Clipped`: a `CircularClip` between overlay and holder cuts
            // render and pick at the inner circle - crescent picks fall
            // back to the card, keeping the ring a drag affordance.
            let clip = CircularClip::new();
            overlay.set_child(None::<&Widget>);
            clip.set_child(holder);
            overlay.set_child(Some(&clip));
        }
        if let Some(holder) = imp.content_holder.get() {
            // Configured as margin on our own holder rather than padding on
            // the consumer widget: the gap shows the card's background and
            // does not clobber margins the consumer may set on `content`.
            let padding = config.content_padding.as_f64() as i32;
            holder.set_margin_top(padding);
            holder.set_margin_bottom(padding);
            holder.set_margin_start(padding);
            holder.set_margin_end(padding);
            holder.append(content);
        }
        if let (Some(folded), Some(holder)) = (&config.folded_content, imp.folded_holder.get()) {
            holder.append(folded);
        }
        card
    }

    /// Semantic identifier of this card.
    pub fn id(&self) -> CardId {
        self.imp().id.borrow().clone()
    }

    /// The title rendered in the header strip (the resolved `CardConfig`
    /// title or the localized fallback).
    pub fn title(&self) -> String {
        self.imp().title_text.borrow().clone()
    }

    /// The board this card belongs to, if it is still on one.
    pub fn board(&self) -> Option<FloatingBoard> {
        self.imp().board.borrow().upgrade()
    }

    /// Current geometry (center position, rotation, scale). For a
    /// `sticky` card the position is expressed in widget pixels; all
    /// other cards report workspace coordinates.
    pub fn geometry(&self) -> CardGeometry {
        self.board().and_then(|board| board.card_geometry(&self.id())).unwrap_or_default()
    }

    /// Applies a full geometry update (position + rotation + scale), clamped
    /// to the board bounds. For a `sticky` card `geometry` is expressed in
    /// widget pixels.
    pub fn set_geometry(&self, geometry: &CardGeometry) {
        if let Some(board) = self.board() {
            board.set_card_geometry(&self.id(), geometry);
        }
    }

    /// Current rotation.
    pub fn rotation(&self) -> Rotation {
        self.geometry().rotation
    }

    /// Sets the rotation immediately (no animation).
    pub fn set_rotation(&self, rotation: Rotation) {
        if let Some(board) = self.board() {
            board.set_card_rotation(&self.id(), rotation);
        }
    }

    /// Sets the rotation with the zoom-snap transition of `RotationWidget`.
    pub fn set_rotation_with_animation(&self, rotation: Rotation) {
        if let Some(board) = self.board() {
            board.set_card_rotation_animated(&self.id(), rotation);
        }
    }

    /// Rotates one snap step in `direction` with the zoom-snap
    /// transition. `QuarterTurn::snap_target` semantics: snap zones
    /// reach 45 degrees either side of each turn, so a card already
    /// past halfway steps to the turn after the engaged zone - every
    /// call produces a visible ~90 degree step. This is the wiring
    /// behind `ChromeElement::RotateClockwise`/`RotateCounterClockwise`.
    pub fn rotate_to_next_quarter(&self, direction: RotationDirection) {
        let target = QuarterTurn::snap_target(self.rotation().to_rotation_rad(), direction);
        self.set_rotation_with_animation(Rotation::QuarterTurn(target));
    }

    /// Snaps the rotation back to `Rotation::ZERO` with the zoom-snap
    /// transition.
    pub fn rotate_to_zero(&self) {
        self.set_rotation_with_animation(Rotation::ZERO);
    }

    /// Passes the card one table side in `direction`
    /// (`FloatingBoard::rotate_to_next_user`): the geometric next-seat
    /// step, not roster-aware. For "next *occupied* seat" consumers keep
    /// their roster and call
    /// `FloatingBoard::rotate_to_next_occupied_seat` - e.g. from a
    /// custom satellite. This is the wiring behind
    /// `ChromeElement::NextSeat`/`PreviousSeat`.
    pub fn pass_to_next_user(&self, direction: TurnDirection, move_to_edge: bool) -> Result<(), BoardError> {
        match self.board() {
            Some(board) => board.rotate_to_next_user(&self.id(), direction, move_to_edge),
            None => Err(BoardError::UnknownCard(self.id())),
        }
    }

    /// Glides the card to `target` on the frame clock - the animated
    /// counterpart of [`Self::set_geometry`].
    ///
    /// See [`FloatingBoard::glide_card_geometry`]: ticks apply the
    /// eased geometry through the regular update path, and a newly
    /// started glide supersedes a running one.
    pub fn glide_to(&self, target: &CardGeometry) {
        if let Some(board) = self.board() {
            let _ = board.glide_card_geometry(&self.id(), *target);
        }
    }

    /// Current scale factor.
    pub fn scale(&self) -> ScaleFactor {
        self.geometry().scale
    }

    /// Sets the scale factor, clamped to the card's `scale_limits`.
    pub fn set_scale(&self, scale: ScaleFactor) {
        if let Some(board) = self.board() {
            board.set_card_scale(&self.id(), scale);
        }
    }

    /// The seat this card is assigned to on the tabletop edge, if any.
    ///
    /// The assignment is decoupled from the visual rotation: rotating
    /// the board or freely spinning the card does not change which user
    /// owns it. Passing the card to the next user advances the seat.
    pub fn seat(&self) -> Option<Seat> {
        self.board().and_then(|board| board.card_seat(&self.id()))
    }

    /// Assigns the card to a seat (or clears the assignment with
    /// `None`). Emits `card-seat-changed` when the assignment differs.
    pub fn set_seat(&self, seat: Option<Seat>) {
        if let Some(board) = self.board() {
            let _ = board.set_card_seat(&self.id(), seat);
        }
    }

    /// Whether the card is pinned to the screen: a sticky card keeps
    /// its on-screen position, rotation and zoom while the view camera
    /// pans, zooms or rotates (`CardConfig::sticky`).
    pub fn sticky(&self) -> bool {
        self.board().and_then(|board| board.card_sticky(&self.id()).ok()).unwrap_or(false)
    }

    /// Pins the card to the screen (`true`) or releases it back onto
    /// the workspace (`false`). Toggling never moves the card
    /// visually - the stored geometry converts between workspace and
    /// widget coordinates at the seam. See
    /// [`FloatingBoard::set_card_sticky`].
    pub fn set_sticky(&self, sticky: bool) {
        if let Some(board) = self.board() {
            let _ = board.set_card_sticky(&self.id(), sticky);
        }
    }

    /// Center position in workspace coordinates.
    pub fn position(&self) -> CardPosition {
        self.geometry().position
    }

    /// The card's rendered bounds in board coordinates - the
    /// axis-aligned box `compute_bounds` reports for the actual widget
    /// with every rotation and scale of its transform stack applied.
    /// This is the ground truth of where the card draws; the model
    /// footprint only matches it when the layout kept the card
    /// centered on its stored position. `None` while the card is not
    /// on a board or not realized yet.
    pub fn rendered_bounds(&self) -> Option<Rect<Pixels>> {
        let board = self.board()?;
        self.compute_bounds(&board).map(Rect::from)
    }

    /// The transform stack's stored position on the card layer - the
    /// `(x, y)` the layer was last asked to place it at (`put`). The
    /// widget chain is card -> rotation -> scaling -> layer, so the
    /// scaling widget's parent is the layer and `child_position`
    /// reports the stored spot. `None` while the widget tree is not
    /// attached yet. Comparing `placed` with the rendered bounds shows
    /// whether the layer actually moved the stack where
    /// `apply_position` put it; comparing `placed + size/2` with
    /// `position` shows whether the board's model and its placement
    /// agree.
    pub fn placed_position(&self) -> Option<Vec2<Pixels>> {
        let scaling = self.parent()?.parent()?;
        let layer = scaling.parent()?;
        let layer = layer.downcast::<CardLayer>().ok()?;
        let (x, y) = layer.child_position(&scaling);
        Some(Vec2::new(x, y))
    }

    /// Every ancestor of the card up to the root with its widget
    /// type, its `(0,0)` origin mapped to board coordinates and its
    /// GTK allocation - the layer-by-layer view that pinpoints which
    /// level of the card -> rotation -> scaling -> layer chain
    /// introduces a render-vs-placement offset. Empty while the card
    /// is not on a board.
    pub fn ancestor_chain(&self) -> Vec<String> {
        let Some(board) = self.board() else {
            return Vec::new();
        };
        let mut chain = Vec::new();
        let mut widget: Option<Widget> = Some(self.clone().upcast());
        while let Some(current) = widget {
            let origin = current.compute_point(&board, &gtk4::graphene::Point::new(0.0, 0.0));
            let bounds = current.compute_bounds(&current);
            chain.push(format!("{}: origin={:?} alloc={:?}", current.type_().name(), origin, bounds));
            widget = current.parent();
        }
        chain
    }

    /// Moves the card so its center is at `position`, clamped to the bounds.
    pub fn set_position(&self, position: CardPosition) {
        let mut geometry = self.geometry();
        geometry.position = position;
        self.set_geometry(&geometry);
    }

    /// Performs the card's configured close action
    /// (`CardCloseAction::Remove` or `Hide`) - the programmatic
    /// equivalent of clicking the header's close button.
    pub fn close(&self) {
        if let Some(board) = self.board() {
            let _ = board.close_card(&self.id());
        }
    }

    /// Moves the card to the top of the card layer.
    pub fn raise(&self) {
        if let Some(board) = self.board() {
            board.raise_card(&self.id());
        }
    }

    /// Resets position, rotation and scale to the card's configured initial
    /// geometry (the `position` anchor, `Deg0`, scale `1.0`).
    pub fn reset(&self) {
        if let Some(board) = self.board() {
            let _ = board.reset_card(&self.id());
        }
    }

    /// Toggles the folded state.
    pub fn fold(&self) {
        self.set_folded(!self.folded());
    }

    /// Folds or unfolds the card. With `CardConfig::folded_content` set,
    /// folding swaps the whole card - header included - for that widget
    /// (`.floating-card-collapsed` strips the chrome); without it, folding
    /// hides the content and keeps only the header strip
    /// (`.floating-card-folded`). Unfolding restores the full card.
    /// Installs or replaces the card's color palette at runtime -
    /// the same scoped-CSS mechanism `CardConfig::palette` uses at
    /// build time. A replaced palette's provider is uninstalled so
    /// repeated re-coloring does not stack rules; an empty palette
    /// restores the themed colors.
    pub fn set_palette(&self, palette: &CardPalette) {
        let imp = self.imp();
        if let Some((class, provider)) = imp.palette.take() {
            self.remove_css_class(&class);
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::style_context_remove_provider_for_display(&display, &provider);
            }
        }
        imp.palette.replace(install_palette(self, palette));
    }

    pub fn set_folded(&self, folded: bool) {
        let imp = self.imp();
        if imp.folded.get() == folded {
            return;
        }
        imp.folded.set(folded);
        let collapsed = imp.folded_holder.get().is_some_and(|holder| holder.first_child().is_some());
        if let Some(holder) = imp.content_holder.get() {
            if folded {
                // Pin the card width while folded to the header: without
                // the content the natural width shrinks and the fold
                // button would end up outside the shrunken header. A
                // collapsed card is meant to shrink instead.
                if !collapsed {
                    let (_, natural) = self.measure_size();
                    if imp.shape.get() == CardShape::Circle {
                        // A folded circle collapses onto its ring: the
                        // diameter stays pinned so the empty disc keeps
                        // the card's size.
                        let diameter = natural.x.max(natural.y);
                        self.set_size_request(diameter, diameter);
                    } else {
                        self.set_size_request(natural.x, -1);
                    }
                }
            } else {
                self.set_size_request(-1, -1);
            }
            holder.set_visible(!folded);
        }
        // A custom folded widget replaces the header too; restore it on
        // unfold (the header is always visible while unfolded, also in
        // headerless mode where CSS slims it).
        if let Some(header) = imp.header.get()
            && collapsed
        {
            header.set_visible(!folded && imp.header_shown.get());
        }
        if let Some(holder) = imp.folded_holder.get() {
            holder.set_visible(folded && collapsed);
        }
        // The grips sit on the content overlay - hide them while folded.
        if let Some(grip) = imp.resize_grip.get() {
            grip.set_visible(!folded && imp.resizable.get());
        }
        if let Some(grip) = imp.rotate_grip.get() {
            grip.set_visible(!folded && imp.rotate_grip_enabled.get());
        }
        if let Some(button) = imp.fold_button.get() {
            // Only the builtin `go-*` icon pair toggles - adwaita and
            // satellite minimize buttons keep their symbolic icon.
            if button.icon_name().is_some_and(|icon| icon.starts_with("go-")) {
                button.set_icon_name(if folded { "go-up-symbolic" } else { "go-down-symbolic" });
            }
            button.set_tooltip_text(Some(&l10n().message(if folded { "card-unfold" } else { "card-fold" })));
        }
        if folded {
            self.add_css_class(CLASS_FOLDED);
            if collapsed {
                self.add_css_class(CLASS_COLLAPSED);
            }
        } else {
            self.remove_css_class(CLASS_FOLDED);
            self.remove_css_class(CLASS_COLLAPSED);
        }
        // Folding changes the measured size; the center anchor must be kept.
        if let Some(board) = self.board() {
            board.recenter_card(&self.id());
        }
        debug!(card = %self.id(), folded, "card folded state changed");
        self.emit_by_name::<()>(CardSignal::Folded.name(), &[]);
    }

    /// Whether the card is folded to its header.
    pub fn folded(&self) -> bool {
        self.imp().folded.get()
    }

    /// Rotated footprint of the card in widget (rendered) coordinates.
    ///
    /// The union of all card footprints forms the dynamic safe-area of the
    /// board (used e.g. by robofigure to avoid drawing under cards).
    pub fn footprint(&self) -> Region {
        self.board()
            .and_then(|board| board.card_footprint(&self.id()).ok())
            .unwrap_or_else(Region::create)
    }

    /// The consumer content widget of this card.
    pub fn content(&self) -> Option<Widget> {
        self.imp().content_holder.get().and_then(|holder| holder.first_child())
    }

    /// Emits `card-moved` (called by the board at drag end).
    pub(crate) fn emit_card_moved(&self) {
        self.emit_by_name::<()>(CardSignal::Moved.name(), &[]);
    }

    /// Emits `card-rotated` (called by the board on rotation changes).
    pub(crate) fn emit_card_rotated(&self) {
        self.emit_by_name::<()>(CardSignal::Rotated.name(), &[]);
    }

    /// Emits `card-scaled` (called by the board at pinch end).
    pub(crate) fn emit_card_scaled(&self) {
        self.emit_by_name::<()>(CardSignal::Scaled.name(), &[]);
    }

    /// Emits `card-raised` (called by the board on raise).
    pub(crate) fn emit_card_raised(&self) {
        self.emit_by_name::<()>(CardSignal::Raised.name(), &[]);
    }

    /// Emits `card-seat-changed` (called by the board when the seat
    /// assignment changes).
    pub(crate) fn emit_card_seat_changed(&self) {
        self.emit_by_name::<()>(CardSignal::SeatChanged.name(), &[]);
    }

    /// Widget acting as the bottom-right resize affordance, if built.
    pub(crate) fn resize_grip(&self) -> Option<Widget> {
        self.imp().resize_grip.get().cloned().map(|grip| grip.upcast())
    }

    /// Widget acting as the bottom-left rotate affordance, if built.
    pub(crate) fn rotate_grip(&self) -> Option<Widget> {
        self.imp().rotate_grip.get().cloned().map(|grip| grip.upcast())
    }

    /// Widget acting as the drag affordance for the given handle mode
    /// and the card's current folded state. `WholeCardFolded` and
    /// `WholeCardUnfolded` drag the whole card in their named state and
    /// fall back to the header strip in the other - an unfolded
    /// `WholeCardFolded` card keeps its interactive content usable.
    pub(crate) fn drag_source(&self, drag_handle: DragHandle) -> Widget {
        let folded = self.imp().folded.get();
        let whole_card = match drag_handle {
            DragHandle::WholeCard => true,
            DragHandle::WholeCardFolded => folded,
            DragHandle::WholeCardUnfolded => !folded,
            DragHandle::Header | DragHandle::Handle => false,
        };
        if whole_card {
            return self.clone().upcast();
        }
        match drag_handle {
            DragHandle::Handle => self
                .imp()
                .handle_label
                .get()
                .cloned()
                .map(|h| h.upcast())
                .unwrap_or_else(|| self.clone().upcast()),
            _ => self.imp().header.get().cloned().unwrap_or_else(|| self.clone().upcast()),
        }
    }
}
