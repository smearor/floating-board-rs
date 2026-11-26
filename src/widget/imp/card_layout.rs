//! `LayoutManagerImpl` for [`CardShapeLayout`].
//!
//! Two shapes share one manager: `Rectangle` delegates measure and
//! allocate to a `GtkBoxLayout` instance (the same layout `GtkBox`
//! would run on its own), `Circle` derives the bounding square from the
//! content's natural size and allocates the content the centered
//! `CircleContent` square - the margin around it renders as the ring.

use crate::domain::CardShape;
use crate::widget::card::Card;
use crate::widget::card_layout::CardShapeLayout;
use glib::Properties;
use glib::object_subclass;
use glib::prelude::Cast;
use glib::prelude::ObjectExt;
use glib::subclass::object::DerivedObjectProperties;
use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::BoxLayout;
use gtk4::Orientation;
use gtk4::SizeRequestMode;
use gtk4::Widget;
use gtk4::prelude::LayoutManagerExt;
use gtk4::subclass::prelude::LayoutManagerImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::OrientableImpl;
use smearor_commons_geometry::Size;
use std::cell::Cell;

/// Implementation of [`CardShapeLayout`].
///
/// `Orientable` is implemented because `GtkBox` delegates its
/// `orientation` property to the layout manager - get/set without the
/// interface raises `GTK_IS_ORIENTABLE` criticals. Cards are always
/// vertical, so the property is stored but unused.
#[derive(Properties)]
#[properties(wrapper_type = CardShapeLayout)]
pub struct CardShapeLayoutImpl {
    /// Box-compatible orientation property - accepted, not interpreted.
    #[property(get, set, builder(Orientation::Vertical))]
    orientation: Cell<Orientation>,
    /// The `Rectangle` delegate - `GtkBox`'s own layout semantics.
    box_layout: BoxLayout,
}

impl Default for CardShapeLayoutImpl {
    fn default() -> Self {
        Self {
            orientation: Cell::new(Orientation::Vertical),
            box_layout: BoxLayout::new(Orientation::Vertical),
        }
    }
}

#[object_subclass]
impl ObjectSubclass for CardShapeLayoutImpl {
    const NAME: &'static str = "FloatingBoardCardShapeLayout";
    type Type = CardShapeLayout;
    type ParentType = gtk4::LayoutManager;
    type Interfaces = (gtk4::Orientable,);
}

#[glib::derived_properties]
impl ObjectImpl for CardShapeLayoutImpl {}

impl LayoutManagerImpl for CardShapeLayoutImpl {
    fn request_mode(&self, widget: &Widget) -> SizeRequestMode {
        let _ = widget;
        // What a vertical `GtkBoxLayout` would report for the card;
        // `request_mode` on the detached delegate asserts
        // (`priv->widget == NULL`).
        SizeRequestMode::HeightForWidth
    }

    fn measure(&self, widget: &Widget, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let Some(card) = widget.downcast_ref::<Card>() else {
            return self.box_layout.measure(widget, orientation, for_size);
        };
        let imp = card.imp();
        let (minimum, natural, minimum_baseline, natural_baseline) = match imp.morph_measure.get() {
            // Fold morph: the pinned pose size replaces the child
            // measure entirely - `size_request` alone cannot shrink a
            // card below its content's own minimum, so the footprint
            // tween needs the layout-level override.
            Some(pin) => match orientation {
                Orientation::Horizontal => (pin.x, pin.x, -1, -1),
                _ => (pin.y, pin.y, -1, -1),
            },
            None => match imp.shape.get() {
                CardShape::Circle => imp.circle_measure(),
                CardShape::Semicircle => imp.semicircle_measure(orientation),
                CardShape::Rectangle => self.box_layout.measure(widget, orientation, for_size),
            },
        };
        // `CardConfig::max_width`/`max_height` cap the reported size (the
        // diameter on a circle card); the parent layer allocates at most
        // the capped size and the default CSS clips the overflow.
        let limit = match orientation {
            Orientation::Horizontal => imp.max_width.get(),
            _ => imp.max_height.get(),
        };
        match limit {
            // Fractional caps floor so the reported size never exceeds the bound.
            Some(cap) => {
                let cap = cap.as_f64() as i32;
                (minimum.min(cap), natural.min(cap), -1, -1)
            }
            None => (minimum, natural, minimum_baseline, natural_baseline),
        }
    }

    fn allocate(&self, widget: &Widget, width: i32, height: i32, baseline: i32) {
        match widget.downcast_ref::<Card>().map(|card| card.imp().shape.get()) {
            // `Circle`: every visible child gets the centered content
            // square (`CircleContent::content_square_for` of the
            // allocated diameter); the margin around it renders the ring.
            Some(CardShape::Circle) => {
                if let Some(card) = widget.downcast_ref::<Card>() {
                    card.imp().circle_allocate(Size::new(width, height), baseline);
                }
            }
            // `Semicircle`: the dome allocates arc/bar chrome and the
            // bottom-aligned content extent itself.
            Some(CardShape::Semicircle) => {
                if let Some(card) = widget.downcast_ref::<Card>() {
                    card.imp().semicircle_allocate(Size::new(width, height), baseline);
                }
            }
            _ => self.box_layout.allocate(widget, width, height, baseline),
        }
    }
}

impl OrientableImpl for CardShapeLayoutImpl {}
