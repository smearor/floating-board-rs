use crate::domain::CardId;
use crate::domain::CardShape;
use crate::domain::ChromeElement;
use crate::domain::ChromePlacement;
use crate::domain::ChromeSlot;
use crate::domain::CircleContent;
use crate::widget::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_layout::CardShapeLayout;
use crate::widget::card_signal::CARD_SIGNALS;
use crate::widget::css::CLASS_CARD;
use crate::widget::css::CLASS_CONTENT;
use glib::WeakRef;
use glib::subclass::Signal;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Overflow;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::glib;
use gtk4::graphene::Point;
use gtk4::gsk::Transform;
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::OrientableExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectImplExt;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::LazyLock;

/// Widget implementation for [`Card`].
///
/// The card is a view: it owns no geometry state. Position/rotation/scale live
/// exclusively on the `FloatingBoard`; the backlink here only delegates calls.
pub struct CardImpl {
    /// Semantic identifier of this card.
    pub id: RefCell<CardId>,
    /// Weak backlink to the owning board.
    pub board: RefCell<WeakRef<FloatingBoard>>,
    /// The installed palette's generated CSS class + provider, so
    /// `Card::set_palette` can uninstall a replaced palette.
    pub palette: RefCell<Option<(String, gtk4::CssProvider)>>,
    /// Header strip (drag affordance) - built by `chrome::mount_chrome`
    /// during `Card::build`; `gtk4::Box` for the builtin style,
    /// `adw::HeaderBar` for adwaita.
    pub header: OnceCell<gtk4::Widget>,
    /// Whether the header has chrome children (and may be shown);
    /// `set_folded` restores to this instead of unconditionally
    /// revealing an empty strip.
    pub header_shown: Cell<bool>,
    /// Overlay stacking the resize/rotate grips over the content.
    pub content_overlay: OnceCell<Overlay>,
    /// Container for the consumer's content widget.
    pub content_holder: OnceCell<gtk4::Box>,
    /// Resolved title - source of truth for `Card::title` regardless
    /// of which chrome renderer displays it.
    pub title_text: RefCell<String>,
    /// Title label inside the header (builtin style only).
    pub title_label: OnceCell<Label>,
    /// Grip icon used in `DragHandle::Handle` mode.
    pub handle_label: OnceCell<Label>,
    /// Fold toggle button (visible when `CardConfig::foldable`).
    pub fold_button: OnceCell<Button>,
    /// Close button (visible when `CardConfig::closable`).
    pub close_button: OnceCell<Button>,
    /// Bottom-right resize grip (`CardConfig::resizable`) - hidden by
    /// default, shown from `Card::build` and while unfolded.
    pub resize_grip: OnceCell<DrawingArea>,
    /// Whether the card offers the resize grip - kept so `set_folded`
    /// can gate the grip's visibility on the fold state.
    pub resizable: Cell<bool>,
    /// Bottom-left rotate grip (`CardConfig::rotate_grip`) - hidden by
    /// default, shown from `Card::build` and while unfolded.
    pub rotate_grip: OnceCell<DrawingArea>,
    /// Whether the card offers the rotate grip - kept so `set_folded`
    /// can gate the grip's visibility on the fold state.
    pub rotate_grip_enabled: Cell<bool>,
    /// Whether the card is currently folded to its header.
    pub folded: Cell<bool>,
    /// Container for `CardConfig::folded_content` - visible only while
    /// folded with a custom folded widget installed.
    pub folded_holder: OnceCell<gtk4::Box>,
    /// Upper bound on the reported width (`CardConfig::max_width`).
    pub max_width: Cell<Option<Pixels>>,
    /// Upper bound on the reported height (`CardConfig::max_height`).
    pub max_height: Cell<Option<Pixels>>,
    /// Fold-morph measure override (`CardFoldAnimation`): while a
    /// morph runs, `CardShapeLayout` reports this pinned size instead
    /// of measuring children - a `size_request` cannot shrink a card
    /// below its content's own minimum, so the footprint tween needs
    /// the layout-level override. `apply_folded_presentation` clears it.
    pub morph_measure: Cell<Option<Vec2<Pixels, i32>>>,
    /// The card's outline (`CardConfig::shape`) - drives the circle
    /// branches in `measure`/`size_allocate`/`contains`.
    pub shape: Cell<CardShape>,
    /// Content allocation mode for `CardShape::Circle`
    /// (`CardConfig::circle_content`).
    pub circle_content: Cell<CircleContent>,
    /// Angular span of the arc-chrome band (`CardConfig::arc_span`,
    /// default 90°) - read by `circle_allocate` and the band's draw
    /// func.
    pub arc_span: Cell<Radians>,
    /// Angular span of the bottom arc-chrome band
    /// (`CardConfig::arc_bottom_span`, `None` mirrors `arc_span`).
    pub arc_bottom_span: Cell<Radians>,
    /// The painted top-quarter annulus band the arc-chrome items sit
    /// on - allocated the full disc, drawn as a ring sector in the
    /// header color (`Card::mount_arc_chrome` only creates it when
    /// `arc_items` is non-empty).
    pub arc_band: OnceCell<DrawingArea>,
    /// Arc-chrome items on `CardShape::Circle` - `(widget, slot)`
    /// pairs in slot order, mounted by `Card::mount_arc_chrome` and
    /// placed on the top/bottom arc bands by `circle_allocate`.
    pub arc_items: RefCell<Vec<(gtk4::Widget, ChromeSlot)>>,
}

impl Default for CardImpl {
    fn default() -> Self {
        Self {
            id: RefCell::new(CardId::new("")),
            board: RefCell::new(WeakRef::new()),
            palette: RefCell::new(None),
            header: OnceCell::new(),
            header_shown: Cell::new(false),
            content_overlay: OnceCell::new(),
            content_holder: OnceCell::new(),
            title_text: RefCell::new(String::new()),
            title_label: OnceCell::new(),
            handle_label: OnceCell::new(),
            fold_button: OnceCell::new(),
            close_button: OnceCell::new(),
            resize_grip: OnceCell::new(),
            resizable: Cell::new(false),
            rotate_grip: OnceCell::new(),
            rotate_grip_enabled: Cell::new(false),
            folded: Cell::new(false),
            folded_holder: OnceCell::new(),
            morph_measure: Cell::new(None),
            max_width: Cell::new(None),
            max_height: Cell::new(None),
            shape: Cell::new(CardShape::default()),
            circle_content: Cell::new(CircleContent::default()),
            arc_span: Cell::new(Degrees::QUARTER_TURN.to_radians()),
            arc_bottom_span: Cell::new(Degrees::QUARTER_TURN.to_radians()),
            arc_band: OnceCell::new(),
            arc_items: RefCell::new(Vec::new()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for CardImpl {
    const NAME: &'static str = "FloatingBoardCard";
    type Type = Card;
    type ParentType = gtk4::Box;

    fn class_init(klass: &mut Self::Class) {
        // Shape-aware layout: GTK4 consults the layout manager for
        // `measure`/`allocate` before the widget vfuncs, and `GtkBox`
        // always carries one - the circle sizing only works from here.
        klass.set_layout_manager_type::<CardShapeLayout>();
    }
}

impl ObjectImpl for CardImpl {
    fn constructed(&self) {
        self.parent_constructed();
        let card = self.obj();
        card.set_orientation(Orientation::Vertical);
        // No `set_spacing(0)`: it delegates to `gtk_box_layout_*` and
        // hard-asserts on our custom `CardShapeLayout`; the delegate
        // inside defaults to spacing 0 anyway.
        // Clips content that exceeds a `max_width`/`max_height` cap and keeps
        // it inside the rounded corners (GTK CSS has no `overflow` property).
        card.set_overflow(Overflow::Hidden);
        card.add_css_class(CLASS_CARD);

        // The header strip is NOT built here: `chrome::mount_chrome`
        // composes it during `Card::build` from the config's chrome
        // slot list (and prepends it before the content overlay).
        let content_holder = gtk4::Box::new(Orientation::Vertical, 0);
        content_holder.add_css_class(CLASS_CONTENT);
        content_holder.set_vexpand(true);

        // The overlay stacks the resize/rotate grips over the
        // content's corners without claiming layout space.
        let content_overlay = Overlay::new();
        content_overlay.set_child(Some(&content_holder));

        let folded_holder = gtk4::Box::new(Orientation::Horizontal, 0);
        folded_holder.add_css_class(crate::widget::css::CLASS_FOLDED_CONTENT);
        folded_holder.set_visible(false);
        folded_holder.set_halign(gtk4::Align::Center);
        folded_holder.set_valign(gtk4::Align::Center);

        // The stage stacks the folded pane over the content pane:
        // while folded - or mid-crossfade in a `CardFoldAnimation`
        // transition - both panes share the same area. The folded pane
        // is an overlay child, so it never claims layout space; the
        // folded state's size is pinned explicitly in `set_folded`.
        let stage = Overlay::new();
        stage.set_child(Some(&content_overlay));
        stage.add_overlay(&folded_holder);

        card.append(&stage);

        let _ = self.content_overlay.set(content_overlay);
        let _ = self.content_holder.set(content_holder);
        let _ = self.folded_holder.set(folded_holder);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| CARD_SIGNALS.iter().map(|signal| signal.signal()).collect());
        &SIGNALS
    }
}

/// Width of the ring band around a circle card's inner circle (px).
/// The band is card chrome: it carries the card's background and stays
/// a drag affordance; the content never reaches into it.
const CIRCLE_RING_BAND: f64 = 8.0;
/// Thickness of the arc-chrome band on `CardShape::Circle` (px): the
/// chrome items' centers ride the band's mid radius.
pub(crate) const ARC_CHROME_BAND: f64 = 36.0;
/// Default arc-chrome band span - the disc's top quarter (±45°).
const ARC_HALF_SPAN: f64 = std::f64::consts::FRAC_PI_4;
/// Angular distance kept around a centered arc `Title` at the default
/// span (rad); `arc_angle` scales the gap with the configured span.
const ARC_TITLE_GAP: f64 = std::f64::consts::PI / 15.0;

/// Evenly spreads `count` directions over `[from, to]` as an `Arc`;
/// a lone item sits at `from` - groups pack from the band's outer
/// edge inward, like a header bar's packed ends.
fn spread(from: f64, to: f64, index: usize, count: usize) -> RotationRad {
    let t = if count <= 1 {
        Fraction::ZERO
    } else {
        Fraction::new(index as f64 / (count - 1) as f64)
    };
    Arc::new(RotationRad::new(from), RotationRad::new(to - from)).at(t)
}

/// Angular position of arc-chrome item `index` of `count` in the
/// legacy all-`Inline` packing (the band spanning `±half_span`):
/// items before `title_index` pack the left half of the band from its
/// outer edge inward, a `Title` item sits centered like a header
/// bar's title, and items after pack the right half from its outer
/// edge inward - the header bar's end-pack semantics. Without a title
/// the whole set spreads evenly over the band.
fn arc_angle(index: usize, count: usize, title_index: Option<usize>, half_span: f64) -> RotationRad {
    let title_gap = half_span * (ARC_TITLE_GAP / ARC_HALF_SPAN);
    let Some(title) = title_index else {
        return if count <= 1 {
            RotationRad::ZERO
        } else {
            spread(-half_span, half_span, index, count)
        };
    };
    if index == title {
        RotationRad::ZERO
    } else if index < title {
        spread(-half_span, -title_gap, index, title)
    } else {
        spread(half_span, title_gap, index - title - 1, count - title - 1)
    }
}

/// Disc-direction per arc-chrome item (radians, 0 = the disc's top,
/// positive clockwise): the top band packs items with `Start`
/// placements edge-inward on its left end, spreads `ArcTop`/`Inline`
/// items over the center zone and packs `End` items edge-inward on
/// its right end - header-bar zones on an arc. `ArcBottom*` items
/// land on the mirrored bottom band (offset π: its "start" edge sits
/// on the screen's right, the user's left when facing the card's
/// bottom). Each band spans its configured `arc_span` /
/// `arc_bottom_span`. A top band of pure `Inline` slots keeps the
/// legacy title-centered spread over the whole band.
fn arc_angles(items: &[ChromeSlot], top_half_span: f64, bottom_half_span: f64) -> Vec<RotationRad> {
    let top_gap = top_half_span * (ARC_TITLE_GAP / ARC_HALF_SPAN);
    let bottom_gap = bottom_half_span * (ARC_TITLE_GAP / ARC_HALF_SPAN);
    let mut zones: [Vec<usize>; 6] = Default::default();
    for (index, slot) in items.iter().enumerate() {
        match slot.placement {
            ChromePlacement::InlineStart | ChromePlacement::ArcTopStart => zones[0].push(index),
            ChromePlacement::Inline | ChromePlacement::ArcTop => zones[1].push(index),
            ChromePlacement::InlineEnd | ChromePlacement::ArcTopEnd => zones[2].push(index),
            ChromePlacement::ArcBottomStart => zones[3].push(index),
            ChromePlacement::ArcBottom => zones[4].push(index),
            ChromePlacement::ArcBottomEnd => zones[5].push(index),
            ChromePlacement::Satellite => {}
        }
    }
    let [top_start, top_center, top_end] = [zones[0].clone(), zones[1].clone(), zones[2].clone()];
    let mut angles = vec![RotationRad::ZERO; items.len()];
    // The legacy path: an unpinned top band of `Inline` items packs
    // around a centered title over the whole band.
    let legacy = top_start.is_empty() && top_end.is_empty() && top_center.iter().all(|&index| items[index].placement == ChromePlacement::Inline);
    if legacy && !top_center.is_empty() {
        let title_index = top_center.iter().position(|&index| items[index].element == ChromeElement::Title);
        for (position, &item) in top_center.iter().enumerate() {
            angles[item] = arc_angle(position, top_center.len(), title_index, top_half_span);
        }
    } else {
        for (position, &item) in top_start.iter().enumerate() {
            angles[item] = spread(-top_half_span, -top_gap, position, top_start.len());
        }
        for (position, &item) in top_center.iter().enumerate() {
            angles[item] = if top_center.len() == 1 {
                RotationRad::ZERO
            } else {
                spread(-top_gap, top_gap, position, top_center.len())
            };
        }
        for (position, &item) in top_end.iter().enumerate() {
            angles[item] = spread(top_half_span, top_gap, position, top_end.len());
        }
    }
    let bottom = RotationRad::new(std::f64::consts::PI);
    for (position, &item) in zones[3].iter().enumerate() {
        angles[item] = bottom + spread(-bottom_half_span, -bottom_gap, position, zones[3].len());
    }
    for (position, &item) in zones[4].iter().enumerate() {
        angles[item] = bottom
            + if zones[4].len() == 1 {
                RotationRad::ZERO
            } else {
                spread(-bottom_gap, bottom_gap, position, zones[4].len())
            };
    }
    for (position, &item) in zones[5].iter().enumerate() {
        angles[item] = bottom + spread(bottom_half_span, bottom_gap, position, zones[5].len());
    }
    angles
}

impl WidgetImpl for CardImpl {
    // `Circle`: only the disk inside the bounding square is pickable -
    // taps in the corners pass through to whatever is underneath
    // (through the board layers' `contains` chain, same as the rotated
    // corners of a rectangle card).
    fn contains(&self, x: f64, y: f64) -> bool {
        if self.shape.get() == CardShape::Circle {
            let (width, height) = (f64::from(self.obj().width()), f64::from(self.obj().height()));
            return Circle::<()>::new(Vec2::new(width / 2.0, height / 2.0), width.min(height) / 2.0).contains(&Vec2::new(x, y));
        }
        self.parent_contains(x, y)
    }
}

impl CardImpl {
    /// `Circle` measure: the card reports its bounding square - a
    /// diameter derived from the visible content child's natural size
    /// via `CircleContent::diameter_for` (the `folded_holder` while it
    /// carries a collapsed widget, else the `content_overlay`). Called
    /// from `CardShapeLayout`.
    pub(crate) fn circle_measure(&self) -> (i32, i32, i32, i32) {
        let content = self
            .folded_holder
            .get()
            .filter(|holder| holder.is_visible() && holder.first_child().is_some())
            .map(|holder| holder.clone().upcast::<Widget>())
            .or_else(|| self.content_overlay.get().map(|overlay| overlay.clone().upcast()));
        let Some(content) = content else {
            return (0, 0, -1, -1);
        };
        let (_, natural_width, _, _) = content.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = content.measure(Orientation::Vertical, -1);
        // A disc does not flex: minimum == natural keeps the bounding
        // square stable (the parent layer allocates at the minimum
        // width - a smaller value would render a shrunken circle whose
        // satellites still orbit the natural diameter).
        let diameter = self
            .circle_content
            .get()
            .diameter_for(f64::from(natural_width.max(natural_height)), CIRCLE_RING_BAND);
        (diameter.ceil() as i32, diameter.ceil() as i32, -1, -1)
    }

    /// `Circle` allocate: every visible child gets the centered content
    /// square (`CircleContent::content_square_for` of the allocated
    /// diameter); the margin around it renders as the ring. Arc-chrome
    /// items instead keep their natural size and sit upright on the
    /// top-quarter band's mid radius (`arc_angle`). Called from
    /// `CardShapeLayout`.
    pub(crate) fn circle_allocate(&self, width: i32, height: i32, baseline: i32) {
        let diameter = f64::from(width.min(height));
        let side = self.circle_content.get().content_square_for(diameter, CIRCLE_RING_BAND);
        let transform = Transform::new().translate(&Point::new(((f64::from(width) - side) / 2.0) as f32, ((f64::from(height) - side) / 2.0) as f32));
        let side = side.ceil() as i32;
        let arc_items = self.arc_items.borrow();
        let arc_band = self.arc_band.get();
        let slots: Vec<ChromeSlot> = arc_items.iter().map(|(_, slot)| *slot).collect();
        let angles = arc_angles(&slots, self.arc_span.get().as_f64() / 2.0, self.arc_bottom_span.get().as_f64() / 2.0);
        let arc_radius = diameter / 2.0 - ARC_CHROME_BAND / 2.0;
        let mut child = self.obj().first_child();
        while let Some(widget) = child {
            if widget.is_visible() {
                if arc_band.is_some_and(|band| band.upcast_ref::<Widget>() == &widget) {
                    // The painted band covers the whole disc square -
                    // its draw func clips itself to the arc sectors.
                    widget.allocate(width, height, baseline, None);
                } else if let Some(index) = arc_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let (sin, cos) = angles[index].sin_cos();
                    let center_x = f64::from(width) / 2.0 + arc_radius * sin;
                    let center_y = f64::from(height) / 2.0 - arc_radius * cos;
                    let arc_transform =
                        Transform::new().translate(&Point::new((center_x - f64::from(natural.x) / 2.0) as f32, (center_y - f64::from(natural.y) / 2.0) as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(arc_transform));
                } else {
                    widget.allocate(side, side, baseline, Some(transform.clone()));
                }
            }
            child = widget.next_sibling();
        }
    }
}

impl BoxImpl for CardImpl {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direction comparisons go through `sin_cos` - the normalized
    /// scalar wraps across the turn boundary.
    fn assert_angle(actual: RotationRad, expected_degrees: f64) {
        let (actual_sin, actual_cos) = actual.sin_cos();
        let expected = RotationRad::new(expected_degrees.to_radians());
        let (expected_sin, expected_cos) = expected.sin_cos();
        assert!(
            (actual_sin - expected_sin).abs() < 1e-9 && (actual_cos - expected_cos).abs() < 1e-9,
            "{actual:?} != {expected_degrees}deg"
        );
    }

    fn slots(placements: &[ChromePlacement]) -> Vec<ChromeSlot> {
        placements
            .iter()
            .map(|placement| ChromeSlot {
                element: ChromeElement::Minimize,
                placement: *placement,
            })
            .collect()
    }

    #[test]
    fn arc_angle_single_item_centers() {
        assert_angle(arc_angle(0, 1, None, ARC_HALF_SPAN), 0.0);
    }

    #[test]
    fn arc_angle_spreads_over_band_without_title() {
        assert_angle(arc_angle(0, 3, None, ARC_HALF_SPAN), -45.0);
        assert_angle(arc_angle(1, 3, None, ARC_HALF_SPAN), 0.0);
        assert_angle(arc_angle(2, 3, None, ARC_HALF_SPAN), 45.0);
    }

    #[test]
    fn arc_angle_title_pins_center() {
        assert_angle(arc_angle(1, 3, Some(1), ARC_HALF_SPAN), 0.0);
        assert_angle(arc_angle(0, 3, Some(1), ARC_HALF_SPAN), -45.0);
        assert_angle(arc_angle(2, 3, Some(1), ARC_HALF_SPAN), 45.0);
    }

    #[test]
    fn arc_angle_groups_around_title() {
        // [A, B, Title, C, D] - each side packs outer-edge inward, so
        // the first slot of each group sits at the band's end.
        assert_angle(arc_angle(0, 5, Some(2), ARC_HALF_SPAN), -45.0);
        assert_angle(arc_angle(1, 5, Some(2), ARC_HALF_SPAN), -12.0);
        assert_angle(arc_angle(3, 5, Some(2), ARC_HALF_SPAN), 45.0);
        assert_angle(arc_angle(4, 5, Some(2), ARC_HALF_SPAN), 12.0);
    }

    #[test]
    fn arc_angles_zones_pack_edges() {
        let angles = arc_angles(
            &slots(&[ChromePlacement::ArcTopStart, ChromePlacement::ArcTop, ChromePlacement::ArcTopEnd]),
            ARC_HALF_SPAN,
            ARC_HALF_SPAN,
        );
        assert_angle(angles[0], -45.0);
        assert_angle(angles[1], 0.0);
        assert_angle(angles[2], 45.0);
    }

    #[test]
    fn arc_angles_inline_ends_map_to_zones() {
        let angles = arc_angles(&slots(&[ChromePlacement::InlineStart, ChromePlacement::InlineEnd]), ARC_HALF_SPAN, ARC_HALF_SPAN);
        assert_angle(angles[0], -45.0);
        assert_angle(angles[1], 45.0);
    }

    #[test]
    fn arc_angles_bottom_band_is_mirrored() {
        // Bottom band: the "start" edge sits on the screen's right -
        // the user's left when facing the card's bottom.
        let angles = arc_angles(
            &slots(&[ChromePlacement::ArcBottomStart, ChromePlacement::ArcBottom, ChromePlacement::ArcBottomEnd]),
            ARC_HALF_SPAN,
            ARC_HALF_SPAN,
        );
        assert_angle(angles[0], 135.0);
        assert_angle(angles[1], 180.0);
        assert_angle(angles[2], 225.0);
    }

    #[test]
    fn arc_angles_bottom_band_uses_own_span() {
        // A narrow bottom band: start/end pack its 15deg half-span.
        let angles = arc_angles(
            &slots(&[ChromePlacement::ArcBottomStart, ChromePlacement::ArcBottomEnd]),
            ARC_HALF_SPAN,
            std::f64::consts::PI / 12.0,
        );
        assert_angle(angles[0], 165.0);
        assert_angle(angles[1], 195.0);
    }

    #[test]
    fn arc_angles_mixed_inline_plus_zone_uses_zones() {
        // One pinned slot switches the whole top band to zone packing:
        // remaining `Inline` items share the center zone.
        let angles = arc_angles(
            &slots(&[ChromePlacement::Inline, ChromePlacement::Inline, ChromePlacement::InlineEnd]),
            ARC_HALF_SPAN,
            ARC_HALF_SPAN,
        );
        assert_angle(angles[0], -12.0);
        assert_angle(angles[1], 12.0);
        assert_angle(angles[2], 45.0);
    }
}
