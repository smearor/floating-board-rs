use crate::domain::CardGeometry;
use crate::domain::CardId;
use crate::domain::CardPosition;
use crate::domain::DragHandle;
use crate::domain::ScaleFactor;
use crate::l10n::l10n;
use crate::widget::board::FloatingBoard;
use crate::widget::card_config::CardConfig;
use crate::widget::css::CLASS_FOLDED;
use crate::widget::css::CLASS_HEADERLESS;
use crate::widget::imp::card::CardImpl;
use glib::object::Cast;
use glib::object::ObjectExt;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::Orientable;
use gtk4::Widget;
use gtk4::glib;
use gtk4::prelude::BoxExt;
use gtk4::prelude::ButtonExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use smearor_wrot_rotation::SmearorRotation;
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
    /// `card-moved`, `card-rotated`, `card-scaled`, `card-raised`, `card-folded`
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

        if config.headerless {
            card.add_css_class(CLASS_HEADERLESS);
        }
        let title = config.title.clone().unwrap_or_else(|| l10n().message("card-untitled"));
        if let Some(label) = imp.title_label.get() {
            label.set_text(&title);
            label.set_visible(!config.headerless);
        }
        if let Some(handle) = imp.handle_label.get() {
            handle.set_visible(config.drag_handle == DragHandle::Handle);
            handle.set_tooltip_text(Some(&l10n().message("card-handle")));
        }
        if let Some(fold_button) = imp.fold_button.get() {
            fold_button.set_visible(config.foldable);
            fold_button.set_tooltip_text(Some(&l10n().message("card-fold")));
            let card_weak = card.downgrade();
            fold_button.connect_clicked(move |_| {
                if let Some(card) = card_weak.upgrade() {
                    card.fold();
                }
            });
        }
        if let Some(close_button) = imp.close_button.get() {
            close_button.set_visible(config.closable);
            close_button.set_tooltip_text(Some(&l10n().message("card-close")));
            let board_weak = board.downgrade();
            let id = id.clone();
            close_button.connect_clicked(move |_| {
                if let Some(board) = board_weak.upgrade() {
                    let _ = board.remove_card(&id);
                }
            });
        }
        if let Some(holder) = imp.content_holder.get() {
            holder.append(content);
        }
        card
    }

    /// Semantic identifier of this card.
    pub fn id(&self) -> CardId {
        self.imp().id.borrow().clone()
    }

    /// The board this card belongs to, if it is still on one.
    pub fn board(&self) -> Option<FloatingBoard> {
        self.imp().board.borrow().upgrade()
    }

    /// Current geometry (center position, rotation, scale).
    pub fn geometry(&self) -> CardGeometry {
        self.board().and_then(|board| board.card_geometry(&self.id())).unwrap_or_default()
    }

    /// Applies a full geometry update (position + rotation + scale), clamped
    /// to the board bounds.
    pub fn set_geometry(&self, geometry: &CardGeometry) {
        if let Some(board) = self.board() {
            board.set_card_geometry(&self.id(), geometry);
        }
    }

    /// Current rotation.
    pub fn rotation(&self) -> SmearorRotation {
        self.geometry().rotation
    }

    /// Sets the rotation immediately (no animation).
    pub fn set_rotation(&self, rotation: SmearorRotation) {
        if let Some(board) = self.board() {
            board.set_card_rotation(&self.id(), rotation);
        }
    }

    /// Sets the rotation with the zoom-snap transition of `RotationWidget`.
    pub fn set_rotation_with_animation(&self, rotation: SmearorRotation) {
        if let Some(board) = self.board() {
            board.set_card_rotation_animated(&self.id(), rotation);
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

    /// Center position in board coordinates.
    pub fn position(&self) -> CardPosition {
        self.geometry().position
    }

    /// Moves the card so its center is at `position`, clamped to the bounds.
    pub fn set_position(&self, position: CardPosition) {
        let mut geometry = self.geometry();
        geometry.position = position;
        self.set_geometry(&geometry);
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

    /// Folds or unfolds the card: folding hides the content and keeps only the
    /// header strip (`.floating-card-folded`), unfolding restores it.
    pub fn set_folded(&self, folded: bool) {
        let imp = self.imp();
        if imp.folded.get() == folded {
            return;
        }
        imp.folded.set(folded);
        if let Some(holder) = imp.content_holder.get() {
            holder.set_visible(!folded);
        }
        if let Some(button) = imp.fold_button.get() {
            button.set_icon_name(if folded { "go-up-symbolic" } else { "go-down-symbolic" });
            button.set_tooltip_text(Some(&l10n().message(if folded { "card-unfold" } else { "card-fold" })));
        }
        if folded {
            self.add_css_class(CLASS_FOLDED);
        } else {
            self.remove_css_class(CLASS_FOLDED);
        }
        // Folding changes the measured size; the center anchor must be kept.
        if let Some(board) = self.board() {
            board.recenter_card(&self.id());
        }
        debug!(card = %self.id(), folded, "card folded state changed");
        self.emit_by_name::<()>("card-folded", &[]);
    }

    /// Whether the card is folded to its header.
    pub fn folded(&self) -> bool {
        self.imp().folded.get()
    }

    /// Rotated footprint of the card in board coordinates.
    ///
    /// The union of all card footprints forms the dynamic safe-area of the
    /// board (used e.g. by robofigure to avoid drawing under cards).
    pub fn footprint(&self) -> gtk4::cairo::Region {
        self.board()
            .and_then(|board| board.card_footprint(&self.id()).ok())
            .unwrap_or_else(gtk4::cairo::Region::create)
    }

    /// The consumer content widget of this card.
    pub fn content(&self) -> Option<Widget> {
        self.imp().content_holder.get().and_then(|holder| holder.first_child())
    }

    /// Emits `card-moved` (called by the board at drag end).
    pub(crate) fn emit_card_moved(&self) {
        self.emit_by_name::<()>("card-moved", &[]);
    }

    /// Emits `card-rotated` (called by the board on rotation changes).
    pub(crate) fn emit_card_rotated(&self) {
        self.emit_by_name::<()>("card-rotated", &[]);
    }

    /// Emits `card-scaled` (called by the board at pinch end).
    pub(crate) fn emit_card_scaled(&self) {
        self.emit_by_name::<()>("card-scaled", &[]);
    }

    /// Emits `card-raised` (called by the board on raise).
    pub(crate) fn emit_card_raised(&self) {
        self.emit_by_name::<()>("card-raised", &[]);
    }

    /// Widget acting as the drag affordance for the given handle mode.
    pub(crate) fn drag_source(&self, drag_handle: DragHandle) -> Widget {
        match drag_handle {
            DragHandle::Header => self.imp().header.get().cloned().map(|h| h.upcast()).unwrap_or_else(|| self.clone().upcast()),
            DragHandle::WholeCard => self.clone().upcast(),
            DragHandle::Handle => self
                .imp()
                .handle_label
                .get()
                .cloned()
                .map(|h| h.upcast())
                .unwrap_or_else(|| self.clone().upcast()),
        }
    }
}
