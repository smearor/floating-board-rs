//! `WidgetImpl` for [`SilhouetteBox`].

use crate::widget::imp::silhouette_contains;
use crate::widget::silhouette_box::SilhouetteBox;
use gtk4::glib;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetImpl;

/// Implementation of [`SilhouetteBox`].
#[derive(Default)]
pub struct SilhouetteBoxImpl;

#[glib::object_subclass]
impl ObjectSubclass for SilhouetteBoxImpl {
    const NAME: &'static str = "FloatingBoardSilhouetteBox";
    type Type = SilhouetteBox;
    type ParentType = gtk4::Box;
}

impl ObjectImpl for SilhouetteBoxImpl {}

impl WidgetImpl for SilhouetteBoxImpl {
    // `gtk_widget_do_pick` consults `contains` only as a leaf check:
    // a plain box claims picks in the cut regions its allocation
    // covers outside the card's silhouette, swallowing them instead
    // of letting them fall through to the widgets below. Delegating
    // to the ancestor card's silhouette keeps the box's claimable
    // region identical to the card's own.
    fn contains(&self, x: f64, y: f64) -> bool {
        silhouette_contains::within_card(&*self.obj(), x, y)
    }
}

impl BoxImpl for SilhouetteBoxImpl {}
