//! `CardShapeLayout` - the card's shape-aware layout manager.

use crate::widget::imp::card_layout::CardShapeLayoutImpl;
use gtk4::glib;

glib::wrapper! {
    /// Layout manager of [`Card`](crate::widget::Card): a `Rectangle`
    /// card delegates to a `GtkBoxLayout` instance (the same layout
    /// semantics `GtkBox` would use), a `Circle` card gets the centered
    /// content square inside its bounding disc.
    ///
    /// A custom `LayoutManager` is required because GTK4 consults the
    /// layout manager for `measure` before the widget's `measure`
    /// vfunc - and `GtkBox` always carries one.
    pub struct CardShapeLayout(ObjectSubclass<CardShapeLayoutImpl>)
        @extends gtk4::LayoutManager,
        @implements gtk4::Orientable;
}
