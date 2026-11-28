//! Pie menu on `CardShape::Annulus` - a context menu as a ring of
//! wedge items around a see-through center, the classic radial menu
//! layout. Roughly modeled on `smearor-wrot-pie-menu`: items carry a
//! label and a color, the hovered wedge highlights and activation
//! fires on click.
//!
//! The ring's content is display-only like every `Annulus` face, so
//! the interaction rides card-level controllers: an
//! `EventControllerMotion` tracks the hovered wedge and a
//! `GestureClick` activates it - both map the pointer into the face's
//! frame via `compute_point` and hit-test radius + angle. Clicks in
//! the cut corners and inside the hole pass straight through to
//! whatever is underneath. Two anchored chrome satellites orbit left
//! and right of the ring and rotate the whole menu card.
//!
//! Activating a wedge repaints the board background in the wedge's
//! color and names the selection in the status bar at the window's
//! bottom edge.
//!
//! Run with `cargo run --example pie_menu`.

mod menu;

use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::CardContent;
use floating_board::CardId;
use floating_board::CardPosition;
use floating_board::CardSatellites;
use floating_board::CardShape;
use floating_board::ChromeElement;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::RadialConfig;
use floating_board::SatelliteConfig;
use floating_board::SatelliteDistribution;
use floating_board::SatelliteLayout;
use floating_board::SatelliteRotationMode;
use gtk4::CssProvider;
use gtk4::EventControllerMotion;
use gtk4::GestureClick;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::gdk::Display;
use gtk4::graphene::Point;
use gtk4::prelude::*;
use menu::HOLE_RATIO;
use menu::ITEMS;
use menu::MenuColor;
use menu::MenuState;
use menu::build_menu_face;
use menu::menu_color;
use menu::wedge_at;
use menu::wedge_color;
use smearor_commons_drawing::gtk::drawing_area;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::QuarterTurn;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use std::cell::Cell;
use std::rc::Rc;

fn main() {
    gtk4::init().unwrap_or_else(|_| panic!("GTK4 must initialize"));
    let application = gtk4::Application::new(Some("io.smearor.floating-board.pie-menu"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("FloatingBoard - Pie Menu"));
    window.set_default_size(800, 640);
    let provider = CssProvider::new();
    provider.load_from_string(".status-bar { padding: 6px 12px; background: rgba(255, 255, 255, 0.06); color: #c9d4e0; font-size: 13px; }");
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let state = Rc::new(MenuState {
        hover: Cell::new(None),
        active: Cell::new(None),
        background: Cell::new(menu_color(MenuColor::Background)),
    });

    // The board filler repaints in the activated wedge's color.
    let fill_state = state.clone();
    let filler = drawing_area(None, move |context, size| {
        let (width, height) = (size.x, size.y);
        fill_state.background.get().apply(context);
        context.rectangle(0.0, 0.0, width, height);
        context.fill().unwrap_or_default();
    });
    filler.set_vexpand(true);
    let content = gtk4::Box::new(Orientation::Vertical, 0);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // The status bar at the window's bottom edge names the selection.
    let status = Label::new(Some("Pie menu - pick a wedge"));
    status.add_css_class("status-bar");
    status.set_xalign(0.0);
    let outer = gtk4::Box::new(Orientation::Vertical, 0);
    board.set_vexpand(true);
    outer.append(&board);
    outer.append(&status);

    // The pie menu itself: a wide annulus band, no chrome - the menu
    // is always open, its wedges are the whole interaction surface.
    let face = build_menu_face(Pixels::new(300.0), state.clone());
    board
        .add_card_with_id(
            "menu",
            &face,
            CardConfig {
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(400.0, 310.0))),
                shape: CardShape::Annulus,
                content: CardContent::Cover,
                hole_ratio: Some(HOLE_RATIO),
                // `Some(vec![])`, not `None`: a missing list falls
                // back to the legacy default slots (title + fold
                // controls) - the pie menu mounts no chrome at all.
                chrome: Some(vec![]),
                headerless: true,
                drag_handle: DragHandle::WholeCard,
                // Rotate steps anchored left (`Deg180`) and right
                // (`Deg0`) of the ring, `CounterRotate` keeps them on
                // screen while the menu spins.
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(RadialConfig {
                        distribution: SatelliteDistribution::Anchored,
                        ..Default::default()
                    }),
                    rotation_mode: SatelliteRotationMode::CounterRotate,
                    satellites: vec![
                        SatelliteConfig::chrome("ccw", ChromeElement::RotateCounterClockwise).with_anchor(Rotation::QuarterTurn(QuarterTurn::Deg180)),
                        SatelliteConfig::chrome("cw", ChromeElement::RotateClockwise).with_anchor(Rotation::QuarterTurn(QuarterTurn::Deg0)),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("menu"));

    // Hover tracking: a card-level motion controller maps the pointer
    // into the face's frame - inside the hole or the cut corners the
    // pick never reaches the card and the wedge hover clears.
    if let Ok(card) = board.card(&CardId::new("menu")) {
        let motion = EventControllerMotion::new();
        let face_weak = face.downgrade();
        let hover_state = state.clone();
        let moved = card.clone();
        motion.connect_motion(move |_, x, y| {
            let Some(face) = face_weak.upgrade() else { return };
            let Some(point) = face.compute_point(&moved, &Point::new(x as f32, y as f32)) else {
                return;
            };
            let wedge = wedge_at(
                Vec2::<Pixels>::new(f64::from(point.x()), f64::from(point.y())),
                face.allocated_size().to_f64().with_unit::<Pixels>(),
            );
            if wedge != hover_state.hover.get() {
                hover_state.hover.set(wedge);
                face.queue_draw();
            }
        });
        let face_weak = face.downgrade();
        let leave_state = state.clone();
        motion.connect_leave(move |_| {
            if leave_state.hover.replace(None).is_some()
                && let Some(face) = face_weak.upgrade()
            {
                face.queue_draw();
            }
        });
        card.add_controller(motion);

        // Activation: click a wedge to fire its action - the board
        // repaints in the wedge's color and the status bar names it.
        let click = GestureClick::new();
        click.set_button(0);
        let face_weak = face.downgrade();
        let active_state = state.clone();
        let status_weak = status.downgrade();
        let filler_weak = filler.downgrade();
        let clicked = card.clone();
        click.connect_released(move |_, _, x, y| {
            let Some(face) = face_weak.upgrade() else { return };
            let Some(point) = face.compute_point(&clicked, &Point::new(x as f32, y as f32)) else {
                return;
            };
            if let Some(index) = wedge_at(
                Vec2::<Pixels>::new(f64::from(point.x()), f64::from(point.y())),
                face.allocated_size().to_f64().with_unit::<Pixels>(),
            ) {
                active_state.active.set(Some(index));
                face.queue_draw();
                let (label, slot) = ITEMS[index];
                active_state.background.set(wedge_color(slot).brighten(-0.5).clamp());
                if let Some(filler) = filler_weak.upgrade() {
                    filler.queue_draw();
                }
                if let Some(status) = status_weak.upgrade() {
                    status.set_text(&format!("Selected: {label}"));
                }
                println!("pie menu: activated '{label}'");
            }
        });
        card.add_controller(click);
    }

    window.set_child(Some(&outer));
    window.present();
}
