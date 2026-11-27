//! `WidgetImpl`/`LayoutManagerImpl` for [`SilhouetteStage`]/[`StageLayout`].

use crate::widget::imp::silhouette_contains;
use crate::widget::silhouette_stage::SilhouetteStage;
use crate::widget::silhouette_stage::StageLayout;
use gtk4::Orientation;
use gtk4::glib;
use gtk4::graphene::Point;
use gtk4::gsk::Transform;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::layout_manager::LayoutManagerImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use std::cell::RefCell;

/// Implementation of [`SilhouetteStage`].
#[derive(Default)]
pub struct SilhouetteStageImpl {
    /// The base child (allocated the full extent); overlay children
    /// follow it in the child list and claim no layout space.
    pub child: RefCell<Option<gtk4::Widget>>,
}

#[glib::object_subclass]
impl ObjectSubclass for SilhouetteStageImpl {
    const NAME: &'static str = "FloatingBoardSilhouetteStage";
    type Type = SilhouetteStage;
    type ParentType = gtk4::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<StageLayout>();
    }
}

impl ObjectImpl for SilhouetteStageImpl {
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
    }
}

impl WidgetImpl for SilhouetteStageImpl {
    // `gtk_widget_do_pick` consults `contains` only as a leaf check:
    // a plain overlay claims picks in the cut regions its allocation
    // covers outside the card's silhouette, swallowing them instead
    // of letting them fall through to the widgets below. Delegating
    // to the ancestor card's silhouette keeps the stage's claimable
    // region identical to the card's own.
    fn contains(&self, x: f64, y: f64) -> bool {
        silhouette_contains::within_card(&*self.obj(), x, y)
    }
}

/// Implementation of [`StageLayout`].
#[derive(Default)]
pub struct StageLayoutImpl;

#[glib::object_subclass]
impl ObjectSubclass for StageLayoutImpl {
    const NAME: &'static str = "FloatingBoardStageLayout";
    type Type = StageLayout;
    type ParentType = gtk4::LayoutManager;
}

impl ObjectImpl for StageLayoutImpl {}

impl StageLayoutImpl {
    /// `overlay children share the base's extent.
    fn aligned_extent(align: gtk4::Align, natural: i32, total: i32) -> (i32, i32) {
        match align {
            gtk4::Align::Fill => (total, 0),
            gtk4::Align::Start => (natural, 0),
            gtk4::Align::End => (natural, total - natural),
            _ => (natural, (total - natural) / 2),
        }
    }
}

impl LayoutManagerImpl for StageLayoutImpl {
    // `gtk4::Overlay` semantics: only the base child measures - the
    // overlay children claim no layout space.
    fn measure(&self, widget: &gtk4::Widget, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        widget.first_child().map(|child| child.measure(orientation, for_size)).unwrap_or_default()
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
        let mut child = widget.first_child();
        let mut base = true;
        while let Some(current) = child {
            if base {
                current.allocate(width, height, baseline, None);
                base = false;
            } else {
                let (_, natural_width, _, _) = current.measure(Orientation::Horizontal, -1);
                let (_, natural_height, _, _) = current.measure(Orientation::Vertical, natural_width);
                let (child_width, x) = Self::aligned_extent(current.halign(), natural_width, width);
                let (child_height, y) = Self::aligned_extent(current.valign(), natural_height, height);
                let transform = Transform::new().translate(&Point::new(x as f32, y as f32));
                current.allocate(child_width, child_height, baseline, Some(transform));
            }
            child = current.next_sibling();
        }
    }
}
