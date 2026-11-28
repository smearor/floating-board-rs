use crate::domain::CardContent;
use crate::domain::CardId;
use crate::domain::CardOutline;
use crate::domain::CardShape;
use crate::domain::ChromeSlot;
use crate::domain::DomeInsets;
use crate::domain::hexagon_content_for;
use crate::domain::hexagon_fill_extent;
use crate::domain::hexagon_fill_radius_for;
use crate::domain::hexagon_radius_for;
use crate::domain::octagon_content_for;
use crate::domain::octagon_fill_extent;
use crate::domain::octagon_fill_inradius_for;
use crate::domain::octagon_inradius_for;
use crate::widget::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_layout::CardShapeLayout;
use crate::widget::card_signal::CARD_SIGNALS;
use crate::widget::chrome_slots::ChromeSlots;
use crate::widget::css::CLASS_CARD;
use crate::widget::css::CLASS_CONTENT;
use crate::widget::silhouette_box::SilhouetteBox;
use crate::widget::silhouette_stage::SilhouetteStage;
use glib::WeakRef;
use glib::subclass::Signal;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Overflow;
use gtk4::Widget;
use gtk4::glib;
use gtk4::graphene::Point;
use gtk4::gsk::{FillRule, Path, PathBuilder, Transform};
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::OrientableExt;
use gtk4::prelude::SnapshotExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectImplExt;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
use gtk4::{Align, Button, CssProvider};
use gtk4::{DrawingArea, Snapshot};
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Distance;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::Size;
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
    pub palette: RefCell<Option<(String, CssProvider)>>,
    /// Header strip (drag affordance) - built by `chrome::mount_chrome`
    /// during `Card::build`; `gtk4::Box` for the builtin style,
    /// `adw::HeaderBar` for adwaita.
    pub header: OnceCell<Widget>,
    /// Whether the header has chrome children (and may be shown);
    /// `set_folded` restores to this instead of unconditionally
    /// revealing an empty strip.
    pub header_shown: Cell<bool>,
    /// Overlay stacking the resize/rotate grips over the content.
    pub content_overlay: OnceCell<SilhouetteStage>,
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
    /// Fraction of the outer radius that is the see-through hole on
    /// `CardShape::Annulus` - `CardConfig::hole_ratio` resolved with
    /// `DEFAULT_HOLE_RATIO`. Unused on every other shape.
    pub hole_ratio: Cell<Fraction>,
    /// Content allocation mode for `CardShape::Circle`
    /// (`CardConfig::content`).
    pub content: Cell<CardContent>,
    /// Whether the content subtree is pick-targetable - false for
    /// every immersive mode (`Fill`/`Clipped`/`Cover`) on a
    /// non-rectangular shape and always on `Annulus`, where no
    /// allocation can keep off the hole. `apply_folded_presentation`
    /// restores this after the fold transition's targeting resets.
    pub content_interactive: Cell<bool>,
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
    /// Arc-chrome items on `CardShape::Circle`/`Semicircle` -
    /// `(widget, slot)` pairs in slot order, mounted by
    /// `Card::mount_arc_chrome` and placed on the arc bands by
    /// `circle_allocate`/`semicircle_allocate`.
    pub arc_items: RefCell<Vec<(Widget, ChromeSlot)>>,
    /// The painted straight bar along the flat bottom edge of a
    /// `Semicircle` card (`Card::mount_arc_chrome` only creates it
    /// when `bar_items` is non-empty).
    pub bar_band: OnceCell<DrawingArea>,
    /// The `DrawingArea` painting the card background inside the
    /// hexagon silhouette clip (the widget's own CSS background
    /// renders outside the clip - it stays transparent).
    pub face: OnceCell<DrawingArea>,
    /// Bar-chrome items on `CardShape::Semicircle`/`Hexagon` - `Bar*`
    /// slots placed on the straight bottom bar's mid line by
    /// `semicircle_allocate`/`hexagon_allocate`.
    pub bar_items: RefCell<Vec<(Widget, ChromeSlot)>>,
    /// The painted straight bar along the flat top edge of a
    /// `Hexagon` card (`Card::mount_arc_chrome` only creates it
    /// when `top_items` is non-empty).
    pub top_band: OnceCell<DrawingArea>,
    /// Top-bar chrome items on `CardShape::Hexagon` - `Inline*`/
    /// `ArcTop*` slots placed on the straight top bar's mid line by
    /// `hexagon_allocate`.
    pub top_items: RefCell<Vec<(Widget, ChromeSlot)>>,
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
            hole_ratio: Cell::new(crate::widget::card_config::DEFAULT_HOLE_RATIO),
            content: Cell::new(CardContent::default()),
            content_interactive: Cell::new(true),
            arc_span: Cell::new(Degrees::QUARTER_TURN.to_radians()),
            arc_bottom_span: Cell::new(Degrees::QUARTER_TURN.to_radians()),
            arc_band: OnceCell::new(),
            arc_items: RefCell::new(Vec::new()),
            bar_band: OnceCell::new(),
            bar_items: RefCell::new(Vec::new()),
            top_band: OnceCell::new(),
            top_items: RefCell::new(Vec::new()),
            face: OnceCell::new(),
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
        let content_holder = SilhouetteBox::new(Orientation::Vertical, 0);
        content_holder.add_css_class(CLASS_CONTENT);
        content_holder.set_vexpand(true);

        // The overlay stacks the resize/rotate grips over the
        // content's corners without claiming layout space - the
        // silhouette-bound `SilhouetteStage` lets picks in the cut
        // regions outside the shape fall through to cards below.
        let content_overlay = crate::widget::silhouette_stage::SilhouetteStage::new();
        content_overlay.set_child(Some(&content_holder));

        let folded_holder = SilhouetteBox::new(Orientation::Horizontal, 0);
        folded_holder.add_css_class(crate::widget::css::CLASS_FOLDED_CONTENT);
        folded_holder.set_visible(false);
        folded_holder.set_halign(Align::Center);
        folded_holder.set_valign(Align::Center);

        // The stage stacks the folded pane over the content pane:
        // while folded - or mid-crossfade in a `CardFoldAnimation`
        // transition - both panes share the same area. The folded pane
        // is an overlay child, so it never claims layout space; the
        // folded state's size is pinned explicitly in `set_folded`.
        let stage = SilhouetteStage::new();
        stage.set_child(Some(&content_overlay));
        stage.add_overlay(&folded_holder);

        card.append(&stage);

        let _ = self.content_overlay.set(content_overlay);
        let _ = self.content_holder.set(content_holder.upcast());
        let _ = self.folded_holder.set(folded_holder.upcast());
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

/// `tan(22.5°) = sqrt(2) - 1` - a flat-top octagon's flat edge is
/// `2 * a * tan(22.5°)` wide for inradius `a`, and its diagonal
/// edges widen a cross-section at a 1:1 gain per unit height.
const TAN_22_5: f64 = 0.414_213_562_373_095_03;
/// Thickness of the arc-chrome band on `CardShape::Circle` (px): the
/// chrome items' centers ride the band's mid radius.
pub(crate) const ARC_CHROME_BAND: f64 = 36.0;
impl WidgetImpl for CardImpl {
    // `Circle`: only the disk inside the bounding square is pickable -
    // taps in the corners pass through to whatever is underneath
    // (through the board layers' `contains` chain, same as the rotated
    // corners of a rectangle card).
    fn contains(&self, x: f64, y: f64) -> bool {
        let bounds = Rect::<()>::from_size(self.obj().allocated_size().to_f64());
        // `Rectangle` keeps the rectangular parent test - every other
        // shape answers through its inscribed `CardOutline`.
        self.shape
            .get()
            .contains(&bounds, self.hole_ratio.get(), &Vec2::new(x, y))
            .unwrap_or_else(|| self.parent_contains(x, y))
    }

    /// Silhouette clip: the card's snapshot is clipped to the
    /// shape's path before children render - the single bound for
    /// `Cover` content on every shape (for `Circle`/`Semicircle`
    /// the path coincides with the CSS rounded background).
    fn snapshot(&self, snapshot: &Snapshot) {
        if let Some(path) = self.silhouette_path() {
            snapshot.push_fill(&path, FillRule::Winding);
            self.parent_snapshot(snapshot);
            snapshot.pop();
        } else {
            self.parent_snapshot(snapshot);
        }
    }
}

/// Traces a closed polygon path through `vertices` on `builder` -
/// shared by the card's silhouette clip for `Hexagon`/`Octagon`.
fn polygon_path(builder: &PathBuilder, vertices: &[Vec2<()>]) {
    builder.move_to(vertices[0].x as f32, vertices[0].y as f32);
    for vertex in &vertices[1..] {
        builder.line_to(vertex.x as f32, vertex.y as f32);
    }
    builder.close();
}

impl CardImpl {
    /// The silhouette path of the card, relative to its bounding
    /// box: the inscribed circle for `Circle`, the dome (a circle
    /// centered on the flat bottom edge - inside the widget bounds
    /// only the dome remains) for `Semicircle`, the flat-top
    /// polygon for `Hexagon`/`Octagon`. `None` for `Rectangle`.
    fn silhouette_path(&self) -> Option<Path> {
        let bounds = Rect::<()>::from_size(self.obj().allocated_size().to_f64());
        let builder = PathBuilder::new();
        match self.shape.get().outline(&bounds, self.hole_ratio.get()) {
            // The dome adds its virtual circle - the bounds box cuts
            // it back to the dome's upper half.
            Some(CardOutline::Circle(circle)) => {
                builder.add_circle(&Point::new(circle.center.x as f32, circle.center.y as f32), circle.radius as f32);
            }
            Some(CardOutline::Semicircle(dome)) => {
                builder.add_circle(&Point::new(dome.center.x as f32, dome.center.y as f32), dome.radius as f32);
            }
            Some(CardOutline::Hexagon(hexagon)) => {
                polygon_path(&builder, &hexagon.vertices());
            }
            Some(CardOutline::Octagon(octagon)) => {
                polygon_path(&builder, &octagon.vertices());
            }
            // The ring: the outer circle plus the hole circle added
            // reversed - under the winding fill rule the hole area
            // stays outside the clip, so it renders transparent.
            Some(CardOutline::Annulus(annulus)) => {
                let center = Point::new(annulus.center.x as f32, annulus.center.y as f32);
                builder.add_circle(&center, annulus.outer as f32);
                let hole = PathBuilder::new();
                hole.add_circle(&center, annulus.inner as f32);
                builder.add_reverse_path(&hole.to_path());
            }
            None => return None,
        }
        Some(builder.to_path())
    }

    /// `Circle` measure: the card reports its bounding square - a
    /// diameter derived from the visible content child's natural size
    /// via `CircleContent::diameter_for` (the `folded_holder` while it
    /// carries a collapsed widget, else the `content_overlay`). Called
    /// from `CardShapeLayout`.
    /// The widget whose natural size drives the shape measure: the
    /// `folded_holder` while it carries a collapsed widget, else the
    /// `content_overlay`.
    fn visible_content_widget(&self) -> Option<Widget> {
        self.folded_holder
            .get()
            .filter(|holder| holder.is_visible() && holder.first_child().is_some())
            .map(|holder| holder.clone().upcast::<Widget>())
            .or_else(|| self.content_overlay.get().map(|overlay| overlay.clone().upcast()))
    }

    /// The natural size the folded card pins to when a custom folded
    /// widget is installed: the shape measure for the curved shapes
    /// (their silhouette decides the pin), the `folded_holder`'s own
    /// measure for `Rectangle`. `None` without a folded widget.
    pub(crate) fn folded_natural(&self) -> Option<Size<i32>> {
        match self.shape.get() {
            CardShape::Circle | CardShape::Annulus => {
                let (_, natural_width, _, natural_height) = self.circle_measure();
                Some(Size::new(natural_width, natural_height))
            }
            CardShape::Semicircle => {
                let (_, natural_width, ..) = self.semicircle_measure(Orientation::Horizontal);
                let (_, natural_height, ..) = self.semicircle_measure(Orientation::Vertical);
                Some(Size::new(natural_width, natural_height))
            }
            CardShape::Hexagon => {
                let (_, natural_width, ..) = self.hexagon_measure(Orientation::Horizontal);
                let (_, natural_height, ..) = self.hexagon_measure(Orientation::Vertical);
                Some(Size::new(natural_width, natural_height))
            }
            CardShape::Octagon => {
                let (_, natural_width, ..) = self.octagon_measure(Orientation::Horizontal);
                let (_, natural_height, ..) = self.octagon_measure(Orientation::Vertical);
                Some(Size::new(natural_width, natural_height))
            }
            CardShape::Rectangle => self.folded_holder.get().map(|holder| {
                let (_, natural_width, _, _) = holder.measure(Orientation::Horizontal, -1);
                let (_, natural_height, _, _) = holder.measure(Orientation::Vertical, -1);
                Size::new(natural_width, natural_height)
            }),
        }
    }

    pub(crate) fn circle_measure(&self) -> (i32, i32, i32, i32) {
        let Some(content) = self.visible_content_widget() else {
            return (0, 0, -1, -1);
        };
        let (_, natural_width, _, _) = content.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = content.measure(Orientation::Vertical, -1);
        // A disc does not flex: minimum == natural keeps the bounding
        // square stable (the parent layer allocates at the minimum
        // width - a smaller value would render a shrunken circle whose
        // satellites still orbit the natural diameter).
        let diameter = self
            .content
            .get()
            .diameter_for(Distance::new(f64::from(natural_width.max(natural_height))), Distance::new(CIRCLE_RING_BAND));
        (diameter.as_f64().ceil() as i32, diameter.as_f64().ceil() as i32, -1, -1)
    }

    /// `Circle` allocate: every visible child gets the centered content
    /// square (`CircleContent::content_square_for` of the allocated
    /// diameter); the margin around it renders as the ring. Arc-chrome
    /// items instead keep their natural size and sit upright on the
    /// top-quarter band's mid radius (`arc_angle`). Called from
    /// `CardShapeLayout`.
    pub(crate) fn circle_allocate(&self, size: Size<i32>, baseline: i32) {
        let diameter = f64::from(size.x.min(size.y));
        let side = self
            .content
            .get()
            .content_square_for(Distance::new(diameter), Distance::new(CIRCLE_RING_BAND))
            .as_f64();
        let transform = Transform::new().translate(&Point::new(((f64::from(size.x) - side) / 2.0) as f32, ((f64::from(size.y) - side) / 2.0) as f32));
        let side = side.ceil() as i32;
        let arc_items = self.arc_items.borrow();
        let arc_band = self.arc_band.get();
        let angles = ChromeSlots::from(arc_items.as_slice()).arc_angles(self.arc_span.get() / 2.0, self.arc_bottom_span.get() / 2.0);
        // The chrome line: just inside the rim on a disc, on the
        // band's mid radius on a ring.
        let bounds = Rect::<()>::from_size(Vec2::new(f64::from(size.x), f64::from(size.y)));
        let arc_radius = self.shape.get().arc_chrome_radius(&bounds, self.hole_ratio.get(), ARC_CHROME_BAND);
        let mut child = self.obj().first_child();
        while let Some(widget) = child {
            if widget.is_visible() {
                if arc_band.is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                    || self.face.get().is_some_and(|face| face.upcast_ref::<Widget>() == &widget)
                {
                    // The painted band covers the whole disc square -
                    // its draw func clips itself to the arc sectors.
                    // The face covers the whole bounding box - the
                    // silhouette clip shapes it (the ring band on an
                    // `Annulus` card).
                    widget.allocate(size.x, size.y, baseline, None);
                } else if let Some(index) = arc_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let up = Vec2::<Pixels>::new(0.0, -1.0);
                    let center = Vec2::<Pixels>::new(f64::from(size.x) / 2.0, f64::from(size.y) / 2.0) + up.rotated(angles[index]) * arc_radius;
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let arc_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(arc_transform));
                } else {
                    widget.allocate(side, side, baseline, Some(transform.clone()));
                }
            }
            child = widget.next_sibling();
        }
    }

    /// `Semicircle` measure: the card reports its bounding dome - a
    /// `2r × r` box where `r` derives from the visible content
    /// child's natural size via `CircleContent::dome_radius_for` (the
    /// `folded_holder` while it carries a collapsed widget, else the
    /// `content_overlay`). The reported extent is `2r` horizontally
    /// and `r` vertically. Called from `CardShapeLayout`.
    pub(crate) fn semicircle_measure(&self, orientation: Orientation) -> (i32, i32, i32, i32) {
        let Some(content) = self.visible_content_widget() else {
            return (0, 0, -1, -1);
        };
        let (_, natural_width, _, _) = content.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = content.measure(Orientation::Vertical, -1);
        // A dome does not flex: minimum == natural keeps the bounding
        // box stable (the parent layer allocates at the minimum
        // width).
        // The measure must reserve the same chrome insets the
        // allocation applies - otherwise the content extent shrinks
        // below the child's natural size and GTK clips it.
        let insets = self.dome_insets();
        let radius = self
            .content
            .get()
            .dome_radius_for(Vec2::new(f64::from(natural_width), f64::from(natural_height)), insets);
        let extent = match orientation {
            Orientation::Horizontal => radius * 2.0,
            _ => radius,
        };
        (extent.as_f64().ceil() as i32, extent.as_f64().ceil() as i32, -1, -1)
    }

    /// The dome's chrome insets `(ring, bottom_inset)`: `ring`
    /// reserves the arc band's width inside the rim, `bottom_inset`
    /// lifts the content onto the straight bar's top. While folded
    /// both bands are hidden and reserve nothing.
    fn dome_insets(&self) -> DomeInsets {
        if self.folded.get() {
            return DomeInsets::new(Distance::new(CIRCLE_RING_BAND), Distance::ZERO);
        }
        let ring = Distance::new(CIRCLE_RING_BAND + if self.arc_items.borrow().is_empty() { 0.0 } else { ARC_CHROME_BAND });
        let bottom_inset = Distance::new(if self.bar_items.borrow().is_empty() { 0.0 } else { ARC_CHROME_BAND });
        DomeInsets::new(ring, bottom_inset)
    }

    /// `Semicircle` allocate: the dome's virtual circle centers at
    /// the bounding box's bottom middle with radius `min(w/2, h)`.
    /// Every visible child gets the bottom-aligned content extent
    /// (`CircleContent::dome_content_for`); arc items sit upright on
    /// the top band's mid radius and bar items upright on the bottom
    /// bar's mid line (`bar_offsets`). Called from `CardShapeLayout`.
    pub(crate) fn semicircle_allocate(&self, size: Size<i32>, baseline: i32) {
        let (width, height) = (f64::from(size.x), f64::from(size.y));
        let radius = Semicircle::<()>::inscribed_in(&Rect::<()>::from_size(Vec2::new(width, height))).radius;
        let arc_items = self.arc_items.borrow();
        let bar_items = self.bar_items.borrow();
        // Each painted band pushes the content inward: the arc band
        // grows the ring (the band rides the dome's rim), the straight
        // bar lifts the content's bottom edge onto the bar's top.
        // `dome_insets` mirrors `semicircle_measure` - folded cards
        // keep their full dome for the folded pane.
        let insets = self.dome_insets();
        let content = self.content.get().dome_content_for(Distance::new(radius), insets);
        let (content_width, content_height) = (content.x, content.y);
        let transform = Transform::new().translate(&Point::new(
            (width / 2.0 - content_width / 2.0) as f32,
            (height - content_height - insets.bottom_inset.as_f64()) as f32,
        ));
        let (content_width, content_height) = (content_width.ceil() as i32, content_height.ceil() as i32);
        // A dome has no bottom arc - `arc_items` holds top-zone slots
        // only; the bottom span is unused.
        let half_span = self.arc_span.get() / 2.0;
        let angles = ChromeSlots::from(arc_items.as_slice()).arc_angles(half_span, half_span);
        // The bar's safe half-extent at its mid line - item centers
        // are additionally clamped so an item's edge stays inside the
        // dome's chord.
        let bar_half_width = (radius.powi(2) - (ARC_CHROME_BAND / 2.0).powi(2)).max(0.0).sqrt();
        let bar_offsets = ChromeSlots::from(bar_items.as_slice()).bar_offsets(bar_half_width);
        let arc_radius = radius - ARC_CHROME_BAND / 2.0;
        let mut child = self.obj().first_child();
        while let Some(widget) = child {
            if widget.is_visible() {
                if self.arc_band.get().is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                    || self.bar_band.get().is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                {
                    // The painted bands cover the whole dome box -
                    // their draw funcs clip themselves to the shape.
                    widget.allocate(size.x, size.y, baseline, None);
                } else if let Some(index) = arc_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let up = Vec2::<Pixels>::new(0.0, -1.0);
                    let center = Vec2::<Pixels>::new(width / 2.0, height) + up.rotated(angles[index]) * arc_radius;
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let arc_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(arc_transform));
                } else if let Some(index) = bar_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let max_offset = (bar_half_width - f64::from(natural.x) / 2.0).max(0.0);
                    let center = Vec2::<Pixels>::new(width / 2.0 + bar_offsets[index].clamp(-max_offset, max_offset), height - ARC_CHROME_BAND / 2.0);
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let bar_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(bar_transform));
                } else {
                    widget.allocate(content_width, content_height, baseline, Some(transform.clone()));
                }
            }
            child = widget.next_sibling();
        }
    }

    /// The hexagon's bottom chrome inset: the `ARC_CHROME_BAND` the
    /// straight bar occupies along the flat bottom edge - `0` while
    /// folded (the band is hidden and the folded pane takes the full
    /// hexagon) or without bar items.
    fn hexagon_bottom_inset(&self) -> f64 {
        if self.folded.get() || self.bar_items.borrow().is_empty() {
            0.0
        } else {
            ARC_CHROME_BAND
        }
    }

    /// The hexagon's top chrome inset: the `ARC_CHROME_BAND` the
    /// straight bar occupies along the flat top edge - `0` while
    /// folded or without top items.
    fn hexagon_top_inset(&self) -> f64 {
        if self.folded.get() || self.top_items.borrow().is_empty() {
            0.0
        } else {
            ARC_CHROME_BAND
        }
    }

    /// `Hexagon` measure: the card reports its bounding box `2R x
    /// sqrt(3)*R` where `R` derives from the visible content child's
    /// natural size via `hexagon_radius_for` (the `folded_holder`
    /// while it carries a collapsed widget, else the
    /// `content_overlay`). Called from `CardShapeLayout`.
    pub(crate) fn hexagon_measure(&self, orientation: Orientation) -> (i32, i32, i32, i32) {
        let Some(content) = self.visible_content_widget() else {
            return (0, 0, -1, -1);
        };
        let (_, natural_width, _, _) = content.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = content.measure(Orientation::Vertical, -1);
        // Like the disc the hexagon does not flex: minimum == natural.
        // The bottom inset mirrors `hexagon_allocate` so the reported
        // extent never undersizes the content (same invariant
        // `dome_insets` maintains for the dome).
        let bar_inset = self.hexagon_bottom_inset() + self.hexagon_top_inset();
        let radius = match self.content.get() {
            CardContent::Inscribed => hexagon_radius_for(f64::from(natural_width.max(natural_height)), CIRCLE_RING_BAND, bar_inset),
            // `Fill`/`Clipped`: the rim-shrunk hexagon's bounding box
            // holds the content - the bars overlay it, so the bar
            // inset does not apply. `Cover` drops the ring too.
            CardContent::Fill | CardContent::Clipped => hexagon_fill_radius_for(f64::from(natural_width), f64::from(natural_height), CIRCLE_RING_BAND),
            CardContent::Cover => hexagon_fill_radius_for(f64::from(natural_width), f64::from(natural_height), 0.0),
        };
        let bounding = Hexagon::<()>::bounding_size_for_circumradius(radius, PolygonOrientation::FlatTop);
        let extent = match orientation {
            Orientation::Horizontal => bounding.x,
            _ => bounding.y,
        };
        (extent.ceil() as i32, extent.ceil() as i32, -1, -1)
    }

    /// `Hexagon` allocate: every visible child gets the centered
    /// inscribed square (`hexagon_content_for` of the allocated
    /// radius); bar items keep their natural size and sit upright on
    /// the straight bar's mid line along the flat bottom edge.
    /// Called from `CardShapeLayout`.
    pub(crate) fn hexagon_allocate(&self, size: Size<i32>, baseline: i32) {
        let (width, height) = (f64::from(size.x), f64::from(size.y));
        let radius = Hexagon::<()>::circumradius_bounded_by(Vec2::new(width, height), PolygonOrientation::FlatTop);
        let bar_items = self.bar_items.borrow();
        let top_items = self.top_items.borrow();
        let bar_inset = self.hexagon_bottom_inset() + self.hexagon_top_inset();
        // `Inscribed` gets the centered inscribed square,
        // `Fill`/`Clipped` the rim-shrunk hexagon's bounding box -
        // the content draws its own hexagon silhouette.
        let (content_width, content_height) = match self.content.get() {
            CardContent::Inscribed => {
                let side = hexagon_content_for(radius, CIRCLE_RING_BAND, bar_inset);
                (side, side)
            }
            CardContent::Fill | CardContent::Clipped => hexagon_fill_extent(radius, CIRCLE_RING_BAND),
            CardContent::Cover => hexagon_fill_extent(radius, 0.0),
        };
        let transform = Transform::new().translate(&Point::new(((width - content_width) / 2.0) as f32, ((height - content_height) / 2.0) as f32));
        let (content_width, content_height) = (content_width.ceil() as i32, content_height.ceil() as i32);
        // The bars' safe half-extent at their mid line: the hexagon's
        // cross-section there - `R/2` (the flat edge's half width)
        // grown by the slant gain `BAND * w / (4h)` over half the band.
        let bar_half_width = radius / 2.0 + ARC_CHROME_BAND * width / (4.0 * height);
        let bar_offsets = ChromeSlots::from(bar_items.as_slice()).bar_offsets(bar_half_width);
        let top_offsets = ChromeSlots::from(top_items.as_slice()).top_offsets(bar_half_width);
        let mut child = self.obj().first_child();
        while let Some(widget) = child {
            if widget.is_visible() {
                if self.bar_band.get().is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                    || self.top_band.get().is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                    || self.face.get().is_some_and(|face| face.upcast_ref::<Widget>() == &widget)
                {
                    // The face covers the whole bounding box - the
                    // silhouette clip shapes it.
                    widget.allocate(size.x, size.y, baseline, None);
                } else if let Some(index) = bar_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let max_offset = (bar_half_width - f64::from(natural.x) / 2.0).max(0.0);
                    let center = Vec2::<Pixels>::new(width / 2.0 + bar_offsets[index].clamp(-max_offset, max_offset), height - ARC_CHROME_BAND / 2.0);
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let bar_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(bar_transform));
                } else if let Some(index) = top_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let max_offset = (bar_half_width - f64::from(natural.x) / 2.0).max(0.0);
                    let center = Vec2::<Pixels>::new(width / 2.0 + top_offsets[index].clamp(-max_offset, max_offset), ARC_CHROME_BAND / 2.0);
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let top_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(top_transform));
                } else {
                    widget.allocate(content_width, content_height, baseline, Some(transform.clone()));
                }
            }
            child = widget.next_sibling();
        }
    }

    /// The octagon's bottom chrome inset: the `ARC_CHROME_BAND` the
    /// straight bar occupies along the flat bottom edge - `0` while
    /// folded (the band is hidden and the folded pane takes the full
    /// octagon) or without bar items.
    fn octagon_bottom_inset(&self) -> f64 {
        if self.folded.get() || self.bar_items.borrow().is_empty() {
            0.0
        } else {
            ARC_CHROME_BAND
        }
    }

    /// The octagon's top chrome inset: the `ARC_CHROME_BAND` the
    /// straight bar occupies along the flat top edge - `0` while
    /// folded or without top items.
    fn octagon_top_inset(&self) -> f64 {
        if self.folded.get() || self.top_items.borrow().is_empty() {
            0.0
        } else {
            ARC_CHROME_BAND
        }
    }

    /// `Octagon` measure: the card reports its bounding box `2a x 2a`
    /// where `a` (the inradius) derives from the visible content
    /// child's natural size via `octagon_inradius_for` (the
    /// `folded_holder` while it carries a collapsed widget, else the
    /// `content_overlay`). Called from `CardShapeLayout`.
    pub(crate) fn octagon_measure(&self, _orientation: Orientation) -> (i32, i32, i32, i32) {
        let Some(content) = self.visible_content_widget() else {
            return (0, 0, -1, -1);
        };
        let (_, natural_width, _, _) = content.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = content.measure(Orientation::Vertical, -1);
        // Like the disc the octagon does not flex: minimum == natural.
        // The bar inset mirrors `octagon_allocate` so the reported
        // extent never undersizes the content.
        let bar_inset = self.octagon_bottom_inset() + self.octagon_top_inset();
        let inradius = match self.content.get() {
            CardContent::Inscribed => octagon_inradius_for(f64::from(natural_width.max(natural_height)), CIRCLE_RING_BAND, bar_inset),
            // `Fill`/`Clipped`: the rim-shrunk octagon's bounding
            // square holds the content - the bars overlay it.
            // `Cover` drops the ring too.
            CardContent::Fill | CardContent::Clipped => octagon_fill_inradius_for(f64::from(natural_width.max(natural_height)), CIRCLE_RING_BAND),
            CardContent::Cover => octagon_fill_inradius_for(f64::from(natural_width.max(natural_height)), 0.0),
        };
        // The bounding box is the square `2a x 2a` - both
        // orientations report the same extent.
        let extent = 2.0 * inradius;
        (extent.ceil() as i32, extent.ceil() as i32, -1, -1)
    }

    /// `Octagon` allocate: every visible child gets the centered
    /// inscribed square (`octagon_content_for` of the allocated
    /// inradius); bar items keep their natural size and sit upright
    /// on the straight bar's mid line along a flat edge. Called from
    /// `CardShapeLayout`.
    pub(crate) fn octagon_allocate(&self, size: Size<i32>, baseline: i32) {
        let (width, height) = (f64::from(size.x), f64::from(size.y));
        let inradius = Octagon::inscribed_in(&Rect::<()>::from_size(Vec2::new(width, height)), PolygonOrientation::FlatTop).inradius();
        let bar_items = self.bar_items.borrow();
        let top_items = self.top_items.borrow();
        let bar_inset = self.octagon_bottom_inset() + self.octagon_top_inset();
        // `Inscribed` gets the centered inscribed square,
        // `Fill`/`Clipped` the rim-shrunk octagon's bounding box -
        // the content draws its own octagon silhouette.
        let side = match self.content.get() {
            CardContent::Inscribed => octagon_content_for(inradius, CIRCLE_RING_BAND, bar_inset),
            CardContent::Fill | CardContent::Clipped => octagon_fill_extent(inradius, CIRCLE_RING_BAND),
            CardContent::Cover => octagon_fill_extent(inradius, 0.0),
        };
        let transform = Transform::new().translate(&Point::new(((width - side) / 2.0) as f32, ((height - side) / 2.0) as f32));
        let side = side.ceil() as i32;
        // The bars' safe half-extent at their mid line: the octagon's
        // cross-section there - the flat edge's half width
        // `a * tan(22.5°)` grown by the diagonal's 1:1 gain `BAND/2`
        // over half the band.
        let bar_half_width = width / 2.0 * TAN_22_5 + ARC_CHROME_BAND / 2.0;
        let bar_offsets = ChromeSlots::from(bar_items.as_slice()).bar_offsets(bar_half_width);
        let top_offsets = ChromeSlots::from(top_items.as_slice()).top_offsets(bar_half_width);
        let mut child = self.obj().first_child();
        while let Some(widget) = child {
            if widget.is_visible() {
                if self.bar_band.get().is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                    || self.top_band.get().is_some_and(|band| band.upcast_ref::<Widget>() == &widget)
                    || self.face.get().is_some_and(|face| face.upcast_ref::<Widget>() == &widget)
                {
                    // The face covers the whole bounding box - the
                    // silhouette clip shapes it.
                    widget.allocate(size.x, size.y, baseline, None);
                } else if let Some(index) = bar_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let max_offset = (bar_half_width - f64::from(natural.x) / 2.0).max(0.0);
                    let center = Vec2::<Pixels>::new(width / 2.0 + bar_offsets[index].clamp(-max_offset, max_offset), height - ARC_CHROME_BAND / 2.0);
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let bar_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(bar_transform));
                } else if let Some(index) = top_items.iter().position(|(item, _)| *item == widget) {
                    let (_, natural) = widget.measure_size();
                    let max_offset = (bar_half_width - f64::from(natural.x) / 2.0).max(0.0);
                    let center = Vec2::<Pixels>::new(width / 2.0 + top_offsets[index].clamp(-max_offset, max_offset), ARC_CHROME_BAND / 2.0);
                    let origin = Rect::<Pixels>::centered(center, Vec2::new(f64::from(natural.x), f64::from(natural.y))).min;
                    let top_transform = Transform::new().translate(&Point::new(origin.x as f32, origin.y as f32));
                    widget.allocate(natural.x, natural.y, baseline, Some(top_transform));
                } else {
                    widget.allocate(side, side, baseline, Some(transform.clone()));
                }
            }
            child = widget.next_sibling();
        }
    }
}

impl BoxImpl for CardImpl {}
