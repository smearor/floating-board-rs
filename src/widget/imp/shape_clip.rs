//! `WidgetImpl` for [`ShapeClip`].

use crate::domain::CardShape;
use crate::widget::shape_clip::ShapeClip;
use gtk4::BinLayout;
use gtk4::glib;
use gtk4::prelude::SnapshotExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use std::cell::Cell;

/// Implementation of [`ShapeClip`].
pub struct ShapeClipImpl {
    /// The silhouette the clip cuts to.
    pub shape: Cell<CardShape>,
    /// Fraction of the outer radius that is the hole on
    /// `CardShape::Annulus` - the clip cuts a band, not a disc.
    /// Unused on every other shape.
    pub inner_ratio: Cell<Fraction>,
}

impl Default for ShapeClipImpl {
    fn default() -> Self {
        Self {
            shape: Cell::new(CardShape::default()),
            inner_ratio: Cell::new(Fraction::HALF),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ShapeClipImpl {
    const NAME: &'static str = "FloatingBoardShapeClip";
    type Type = ShapeClip;
    type ParentType = gtk4::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
    }
}

impl ObjectImpl for ShapeClipImpl {}

impl ShapeClipImpl {
    /// The clip path of the shape inscribed in the widget's
    /// allocation: the circle for `Circle`, the dome (a circle
    /// centered on the flat bottom edge) for `Semicircle`, the
    /// flat-top polygon path for `Hexagon`/`Octagon`.
    fn clip_path(&self) -> gtk4::gsk::Path {
        let size = self.obj().allocated_size().to_f64();
        let bounds = Rect::<()>::from_size(size);
        let builder = gtk4::gsk::PathBuilder::new();
        match self.shape.get() {
            CardShape::Circle => {
                builder.add_circle(&gtk4::graphene::Point::new((size.x / 2.0) as f32, (size.y / 2.0) as f32), (size.x.min(size.y) / 2.0) as f32);
            }
            CardShape::Semicircle => {
                // A full circle centered on the flat edge: inside the
                // widget's bounds only the dome remains.
                let dome = Semicircle::<()>::inscribed_in(&bounds);
                builder.add_circle(&gtk4::graphene::Point::new(dome.center.x as f32, dome.center.y as f32), dome.radius as f32);
            }
            shape => {
                let vertices: Vec<Vec2<()>> = match shape {
                    CardShape::Octagon => Octagon::inscribed_in(&bounds, PolygonOrientation::FlatTop).vertices().to_vec(),
                    _ => Hexagon::inscribed_in(&bounds, PolygonOrientation::FlatTop).vertices().to_vec(),
                };
                builder.move_to(vertices[0].x as f32, vertices[0].y as f32);
                for vertex in &vertices[1..] {
                    builder.line_to(vertex.x as f32, vertex.y as f32);
                }
                builder.close();
            }
        }
        builder.to_path()
    }

    /// Whether the point lies inside the clip shape.
    fn inside(&self, x: f64, y: f64) -> bool {
        let size = self.obj().allocated_size().to_f64();
        let bounds = Rect::<()>::from_size(size);
        match self.shape.get() {
            CardShape::Circle => Circle::<()>::new(Vec2::new(size.x / 2.0, size.y / 2.0), size.x.min(size.y) / 2.0).contains(&Vec2::new(x, y)),
            CardShape::Annulus => {
                let outer = size.x.min(size.y) / 2.0;
                Annulus::<()>::new(Vec2::new(size.x / 2.0, size.y / 2.0), outer * self.inner_ratio.get().as_f64(), outer).contains(&Vec2::new(x, y))
            }
            CardShape::Semicircle => Semicircle::<()>::inscribed_in(&bounds).contains(&Vec2::new(x, y)),
            CardShape::Octagon => Octagon::inscribed_in(&bounds, PolygonOrientation::FlatTop).contains(&Vec2::new(x, y)),
            _ => Hexagon::inscribed_in(&bounds, PolygonOrientation::FlatTop).contains(&Vec2::new(x, y)),
        }
    }
}

impl WidgetImpl for ShapeClipImpl {
    // Silhouette clip: everything the child paints outside the
    // inscribed shape is cut away. The default snapshot chain is
    // replaced so the child renders between `push_fill`/`pop`.
    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        let widget = self.obj();
        snapshot.push_fill(&self.clip_path(), gtk4::gsk::FillRule::Winding);
        if let Some(child) = widget.first_child() {
            widget.snapshot_child(&child, snapshot);
        }
        snapshot.pop();
    }

    // Strips the picks outside the clip shape: the widget reports
    // `false`, so the pick falls back to the card - the ring stays
    // a drag affordance over a content that fills its square.
    fn contains(&self, x: f64, y: f64) -> bool {
        self.inside(x, y)
    }
}
