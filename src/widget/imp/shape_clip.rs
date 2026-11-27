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
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;

/// `sqrt(3)` - the flat-top hexagon's height factor
/// (`height = sqrt(3) * circumradius`).
const SQRT_3: f64 = 1.732_050_807_568_877_2;

/// Implementation of [`ShapeClip`].
pub struct ShapeClipImpl {
    /// The silhouette the clip cuts to.
    pub shape: Cell<CardShape>,
    /// Fraction of the outer radius that is the hole on
    /// `CardShape::Annulus` - the clip cuts a band, not a disc.
    /// Unused on every other shape.
    pub inner_ratio: Cell<f64>,
}

impl Default for ShapeClipImpl {
    fn default() -> Self {
        Self {
            shape: Cell::new(CardShape::default()),
            inner_ratio: Cell::new(0.5),
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
        let (width, height) = (f64::from(self.obj().width()), f64::from(self.obj().height()));
        let builder = gtk4::gsk::PathBuilder::new();
        match self.shape.get() {
            CardShape::Circle => {
                builder.add_circle(&gtk4::graphene::Point::new((width / 2.0) as f32, (height / 2.0) as f32), (width.min(height) / 2.0) as f32);
            }
            CardShape::Semicircle => {
                // A full circle centered on the flat edge: inside the
                // widget's bounds only the dome remains.
                builder.add_circle(&gtk4::graphene::Point::new((width / 2.0) as f32, height as f32), ((width / 2.0).min(height)) as f32);
            }
            shape => {
                let vertices: Vec<Vec2<()>> = match shape {
                    CardShape::Octagon => {
                        let inradius = (width / 2.0).min(height / 2.0);
                        Octagon::with_inradius(Vec2::<()>::new(width / 2.0, height / 2.0), inradius, PolygonOrientation::FlatTop)
                            .vertices()
                            .to_vec()
                    }
                    _ => {
                        let radius = (width / 2.0).min(height / SQRT_3);
                        Hexagon::flat_top(Vec2::<()>::new(width / 2.0, height / 2.0), radius).vertices().to_vec()
                    }
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
        let (width, height) = (f64::from(self.obj().width()), f64::from(self.obj().height()));
        match self.shape.get() {
            CardShape::Circle => Circle::<()>::new(Vec2::new(width / 2.0, height / 2.0), width.min(height) / 2.0).contains(&Vec2::new(x, y)),
            CardShape::Annulus => {
                let outer = width.min(height) / 2.0;
                Annulus::<()>::new(Vec2::new(width / 2.0, height / 2.0), outer * self.inner_ratio.get(), outer).contains(&Vec2::new(x, y))
            }
            CardShape::Semicircle => {
                let radius = (width / 2.0).min(height);
                Circle::<()>::new(Vec2::new(width / 2.0, height), radius).contains(&Vec2::new(x, y)) && y <= height
            }
            CardShape::Octagon => {
                let inradius = (width / 2.0).min(height / 2.0);
                Octagon::with_inradius(Vec2::<()>::new(width / 2.0, height / 2.0), inradius, PolygonOrientation::FlatTop).contains(&Vec2::new(x, y))
            }
            _ => {
                let radius = (width / 2.0).min(height / SQRT_3);
                Hexagon::flat_top(Vec2::<()>::new(width / 2.0, height / 2.0), radius).contains(&Vec2::new(x, y))
            }
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
