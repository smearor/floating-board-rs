//! Pick containment delegated to the ancestor [`Card`]'s silhouette.

use crate::widget::card::Card;
use gtk4::prelude::IsA;
use gtk4::prelude::StaticType;
use gtk4::prelude::WidgetExt;

/// Whether `x`/`y` (in `widget`'s own coordinates) lies inside the
/// silhouette of the card `widget` belongs to: the point is mapped
/// into card space via `compute_point` and tested by the card's own
/// `contains` - every intermediate container then reports the same
/// claimable region as the card itself, so picks in the cut regions
/// outside the shape pass through to the widgets below instead of
/// being swallowed as leaf claims.
pub(crate) fn within_card(widget: &impl IsA<gtk4::Widget>, x: f64, y: f64) -> bool {
    let widget = widget.as_ref();
    let Some(card) = widget.ancestor(Card::static_type()) else {
        return true;
    };
    widget
        .compute_point(&card, &gtk4::graphene::Point::new(x as f32, y as f32))
        .map(|point| card.contains(f64::from(point.x()), f64::from(point.y())))
        .unwrap_or_default()
}
