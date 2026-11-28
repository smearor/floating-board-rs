//! Annulus showcase: `CardShape::Annulus` is a ring - a disc with a
//! circular hole. The silhouette clip cuts both bounds, so the hole
//! stays see-through and, because the ring is what `contains` tests,
//! clicks in the hole pass straight through to cards below - like a
//! lifebuoy hanging over the board. `hole_ratio` (default 0.5) sets
//! the hole's fraction of the outer radius.
//!
//! Four cards pair each `CardContent` variant with a different chrome
//! arrangement on the ring's mid-radius arc bands:
//!
//! - `lifebuoy` runs `CardContent::Inscribed` with a **top band**
//!   only: the content square centers in the ring (painting over the
//!   hole region - `Inscribed` bounds are unaffected by the hole),
//!   `[RotateCCW, "Lifebuoy", RotateCW]` rides the rim.
//! - `donut` runs `CardContent::Fill` with a **bottom band** only:
//!   the rim-shrunk square paints its corners over the hole, the bar
//!   hosts `Minimize`/`Title`/`Close` for a user facing the ring's
//!   bottom edge.
//! - `bezel` runs `CardContent::Clipped` with **both bands**: the
//!   same square face is clipped to the ring band itself - corners
//!   and hole show card surface. Rotate steps on the bottom bar spin
//!   the ring.
//! - `gear` runs `CardContent::Cover` with **no chrome**: the face
//!   paints the whole bounding box and the card's silhouette clip
//!   leaves only the toothed ring. Two satellites step the spin.
//!
//! Annulus content is display-only in every mode - even `Inscribed`
//! allocations span the hole, and picks there must fall through.
//!
//! Run with `cargo run --example annulus`.

mod bezel;
mod donut;
mod gear;
mod lifebuoy;

use bezel::build_bezel;
use donut::build_donut;
use floating_board::CardAnchor;
use floating_board::CardColor;
use floating_board::CardConfig;
use floating_board::CardContent;
use floating_board::CardFoldAnimation;
use floating_board::CardId;
use floating_board::CardPalette;
use floating_board::CardPosition;
use floating_board::CardSatellites;
use floating_board::CardShape;
use floating_board::ChromeElement;
use floating_board::ChromeSlot;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::SatelliteConfig;
use floating_board::SatelliteLayout;
use gear::build_gear;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use lifebuoy::build_lifebuoy;
use smearor_commons_color::Color;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::rc::Rc;

/// The hole's fraction of the outer radius - shared by the card
/// config and every face's drawing so painted holes line up with the
/// real one.
const HOLE_RATIO: Fraction = Fraction::HALF;

/// A round `+`/`-` satellite stepping the shared gear phase.
fn spin_satellite(name: &str, face: &DrawingArea, phase: &Rc<Cell<f64>>, step: f64) -> Button {
    let button = Button::with_label(if step > 0.0 { "+" } else { "\u{2212}" });
    button.set_widget_name(name);
    button.add_css_class("gear-satellite");
    let face = face.downgrade();
    let phase = phase.clone();
    button.connect_clicked(move |_| {
        if let Some(face) = face.upgrade() {
            phase.set(phase.get() + step);
            face.queue_draw();
        }
    });
    button
}

fn main() {
    gtk4::init().unwrap_or_else(|_| panic!("GTK4 must initialize"));
    let application = gtk4::Application::new(Some("io.smearor.floating-board.annulus"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("FloatingBoard - Annulus Rings"));
    window.set_default_size(1150, 750);
    let provider = CssProvider::new();
    provider.load_from_string(
        "window { background: #26323e; }
         .gear-satellite { min-width: 28px; min-height: 28px; padding: 0; border-radius: 14px;
                           background: #3d5266; color: #d8e6f0; font-weight: bold; }",
    );
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().unwrap_or_else(|| panic!("display")),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);
    board.set_widget_name("board");
    board.set_vexpand(true);
    board.set_hexpand(true);

    // Inscribed + top band: the lifebuoy's square sits centered in
    // the ring; the rim band carries the header chrome.
    board
        .add_card_with_id(
            "lifebuoy",
            &build_lifebuoy(Pixels::new(200.0)),
            CardConfig {
                title: Some("Lifebuoy".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(250.0, 330.0))),
                shape: CardShape::Annulus,
                content: CardContent::Inscribed,
                hole_ratio: Some(HOLE_RATIO),
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::RotateClockwise),
                ]),
                folded_content: Some(build_lifebuoy(Pixels::new(80.0)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("lifebuoy"));

    // Fill + bottom band: square corners paint over the hole; the
    // mirrored bar hosts chrome for a user at the bottom edge.
    board
        .add_card_with_id(
            "donut",
            &build_donut(Pixels::new(200.0)),
            CardConfig {
                title: Some("Donut".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(540.0, 330.0))),
                shape: CardShape::Annulus,
                content: CardContent::Fill,
                hole_ratio: Some(HOLE_RATIO),
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_donut(Pixels::new(80.0)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("donut"));

    // Clipped + both bands: the face is cut to the ring band; rotate
    // steps on the bottom bar spin the bezel.
    board
        .add_card_with_id(
            "bezel",
            &build_bezel(Pixels::new(200.0)),
            CardConfig {
                title: Some("Bezel".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(830.0, 330.0))),
                shape: CardShape::Annulus,
                content: CardContent::Clipped,
                hole_ratio: Some(HOLE_RATIO),
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::bar(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_bezel(Pixels::new(80.0)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("bezel"));

    // Cover + no chrome (`Some(vec![])` - `None` would fall back to
    // the legacy default slots): the gear paints the whole bounding
    // box, the silhouette clip leaves the toothed ring. `+`/`-`
    // satellites step the spin phase - the ring's interaction lives
    // on its satellites, not its display-only content.
    let gear_phase = Rc::new(Cell::new(0.0_f64));
    let gear_face = build_gear(Pixels::new(220.0), gear_phase.clone());
    // A transparent background palette: the face paints nothing, so
    // the gear's own drawing is all that renders inside the ring
    // silhouette - the ring "floats" over the board.
    let mut gear_palette = CardPalette::new();
    gear_palette.insert(CardColor::Background, Color::<f64>::new_rgba(0.0, 0.0, 0.0, 0.0));
    board
        .add_card_with_id(
            "gear",
            &gear_face,
            CardConfig {
                title: Some("Gear".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(540.0, 600.0))),
                shape: CardShape::Annulus,
                content: CardContent::Cover,
                hole_ratio: Some(HOLE_RATIO),
                palette: Some(gear_palette),
                chrome: Some(vec![]),
                folded_content: Some(build_gear(Pixels::new(80.0), gear_phase.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("cw", spin_satellite("gear-cw", &gear_face, &gear_phase, 0.15)),
                        SatelliteConfig::custom("ccw", spin_satellite("gear-ccw", &gear_face, &gear_phase, -0.15)),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("gear"));

    window.set_child(Some(&board));
    window.present();
}
