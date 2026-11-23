//! `WidgetImpl` for [`CircularClip`].

use crate::widget::circular_clip::CircularClip;
use gtk4::BinLayout;
use gtk4::glib;
use gtk4::graphene::Rect;
use gtk4::graphene::Size;
use gtk4::gsk::RoundedRect;
use gtk4::prelude::SnapshotExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Vec2;

/// Implementation of [`CircularClip`].
#[derive(Default)]
pub struct CircularClipImpl;

#[glib::object_subclass]
impl ObjectSubclass for CircularClipImpl {
    const NAME: &'static str = "FloatingBoardCircularClip";
    type Type = CircularClip;
    type ParentType = gtk4::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
    }
}

impl ObjectImpl for CircularClipImpl {}

impl WidgetImpl for CircularClipImpl {
    // Circular clip: everything the child paints outside the inscribed
    // circle is cut away. The default snapshot chain is replaced so the
    // child renders between `push_rounded_clip`/`pop`.
    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        let widget = self.obj();
        let (width, height) = (widget.width() as f32, widget.height() as f32);
        let corner = Size::new(width.min(height) / 2.0, width.min(height) / 2.0);
        let bounds = RoundedRect::new(Rect::new(0.0, 0.0, width, height), corner, corner, corner, corner);
        snapshot.push_rounded_clip(&bounds);
        if let Some(child) = widget.first_child() {
            widget.snapshot_child(&child, snapshot);
        }
        snapshot.pop();
    }

    // Strips crescent picks: outside the inscribed circle the widget
    // reports `false`, so the pick falls back to the card - the ring
    // stays a drag affordance over a content that fills its square.
    fn contains(&self, x: f64, y: f64) -> bool {
        let widget = self.obj();
        let (width, height) = (f64::from(widget.width()), f64::from(widget.height()));
        Circle::<()>::new(Vec2::new(width / 2.0, height / 2.0), width.min(height) / 2.0).contains(&Vec2::new(x, y))
    }
}
