//! Render probe for the `CardShape::Annulus` showcase: the same four
//! cards as `annulus.rs`, snapshots the board into a PNG once laid
//! out and picks the hole (must pass through to the board), the ring
//! band (must land on the card) and the cut corners. Run with
//! `cargo run --example annulus_probe` - writes
//! /tmp/annulus_probe.png.

// The probe reuses the showcase's faces - `#[path]` modules keep
// them one implementation instead of a drifting twin copy.
#[path = "../annulus/bezel/mod.rs"]
mod bezel;
#[path = "../annulus/donut/mod.rs"]
mod donut;
#[path = "../annulus/gear/mod.rs"]
mod gear;
#[path = "../annulus/lifebuoy/mod.rs"]
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
use glib::timeout_add_local_once;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::DrawingArea;
use gtk4::PickFlags;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use lifebuoy::build_lifebuoy;
use smearor_commons_color::Color;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The hole's fraction of the outer radius - shared by the card
/// config and the `#[path]` faces' drawing (`crate::HOLE_RATIO`) so
/// painted holes line up with the real one.
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
    let application = gtk4::Application::new(Some("io.smearor.floating-board.annulus-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board annulus probe"));
    window.set_default_size(1150, 750);
    let provider = CssProvider::new();
    provider.load_from_string(
        "window { background: #26323e; }
         .gear-satellite { min-width: 28px; min-height: 28px; padding: 0; border-radius: 14px;
                           background: #3d5266; color: #d8e6f0; font-weight: bold; }",
    );
    gtk4::style_context_add_provider_for_display(
        &Display::default().unwrap_or_else(|| panic!("display")),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let under_face = drawing_target(Some(Size::new(1000, 500)), |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let _ = target.fill_rect(FillRect::new(bounds, Color::<f64>::new(0.55, 0.25, 0.30), 1.0));
    });
    under_face.set_widget_name("under-content");
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);
    board.set_widget_name("board");
    board.set_vexpand(true);
    board.set_hexpand(true);

    board
        .add_card_with_id(
            "under",
            &under_face,
            CardConfig {
                title: Some("Under".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(530.0, 450.0))),
                shape: CardShape::Rectangle,
                content: CardContent::Inscribed,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("under"));

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

    // Reproduce the live report: fold + unfold the donut, then re-pick
    // the hole - before the fold it reaches the `under` card.
    timeout_add_local_once(Duration::from_millis(500), {
        let board = board.clone();
        move || {
            if let Ok(donut) = board.card(&CardId::new("donut")) {
                donut.set_folded(true);
            }
            if let Ok(bezel) = board.card(&CardId::new("bezel")) {
                bezel.set_folded(true);
            }
        }
    });
    timeout_add_local_once(Duration::from_millis(1200), {
        let board = board.clone();
        move || {
            if let Ok(donut) = board.card(&CardId::new("donut")) {
                donut.set_folded(false);
            }
            if let Ok(bezel) = board.card(&CardId::new("bezel")) {
                bezel.set_folded(false);
            }
        }
    });
    let probe_board = board.clone();
    timeout_add_local_once(Duration::from_millis(400), {
        let board = board.clone();
        move || {
            for id in ["lifebuoy", "donut", "bezel", "gear"] {
                if let Ok(card) = board.card(&CardId::new(id)) {
                    eprintln!("card {id}: {}x{}", card.width(), card.height());
                    let mut child = card.first_child();
                    while let Some(widget) = child {
                        eprintln!("  {} '{}': {}x{}", widget.type_().name(), widget.widget_name(), widget.width(), widget.height());
                        child = widget.next_sibling();
                    }
                }
            }
        }
    });
    timeout_add_local_once(Duration::from_millis(1800), move || {
        for (name, bx, by) in [
            ("lifebuoy hole", 250.0, 330.0),
            ("lifebuoy band top", 250.0, 222.0),
            ("lifebuoy cut corner", 160.0, 240.0),
            ("lifebuoy top chrome", 250.0, 200.0),
            ("donut hole", 540.0, 330.0),
            ("bezel hole", 830.0, 330.0),
            ("gear hole", 540.0, 600.0),
            ("gear rim", 540.0, 500.0),
        ] {
            let picked = probe_board.pick(bx, by, PickFlags::DEFAULT);
            eprintln!(
                "pick {name} ({bx},{by}) -> {}",
                picked
                    .map(|w| {
                        let mut chain = vec![format!("{} '{}'", w.type_().name(), w.widget_name())];
                        let mut parent = w.parent();
                        while let Some(widget) = parent {
                            chain.push(format!("{} '{}'", widget.type_().name(), widget.widget_name()));
                            parent = widget.parent();
                        }
                        chain.join(" < ")
                    })
                    .unwrap_or_default()
            );
        }
    });

    timeout_add_local_once(Duration::from_millis(2000), move || {
        eprintln!("board {}x{} visible={}", board.width(), board.height(), board.is_visible());
        smearor_commons_fx::gtk::snapshot_to_png(&board, "/tmp/annulus_probe.png").expect("png dump");
        eprintln!("wrote /tmp/annulus_probe.png {}x{}", board.width(), board.height());
        std::process::exit(0);
    });
}
