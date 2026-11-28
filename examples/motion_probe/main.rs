//! Satellite motion probe (Phase 5): mounts a card with a radial
//! satellite set, then drives the choreography on a timer - mount
//! reveal, close, re-open, fold choreography, unfold - while GTK tick
//! callbacks advance the transitions. GTK criticals land on stderr;
//! the final state report goes to stdout. Run with
//! `cargo run --example motion_probe`.

use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::CardId;
use floating_board::CardPosition;
use floating_board::CardSatellites;
use floating_board::ChromeElement;
use floating_board::FloatingBoard;
use floating_board::SatelliteConfig;
use floating_board::SatelliteLayout;
use gtk4::Button;
use gtk4::Label;
use gtk4::prelude::*;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::time::Duration;

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.motion-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("motion probe"));
    window.set_default_size(1280, 800);

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let card_content = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    card_content.append(&Label::new(Some("motion probe card")));

    let board = FloatingBoard::new(&content);
    let id = board
        .add_card_with_id(
            "probe",
            &card_content,
            CardConfig {
                title: Some("Probe".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(400.0, 400.0))),
                // Rectangle card without folded_content folding to its
                // header: the crossfade morph must tween the footprint
                // height although the fading content still measures.
                fold_animation: floating_board::CardFoldAnimation::Crossfade,
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::chrome("min", ChromeElement::Minimize),
                        SatelliteConfig::chrome("close", ChromeElement::Close),
                        SatelliteConfig::chrome("cw", ChromeElement::RotateClockwise),
                        SatelliteConfig::custom("extra", Button::with_label("x")),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("probe"));

    // A circle card with folded_content + unfold_on_click: while
    // folded every satellite - Minimize included - must hide.
    let orb_content = gtk4::DrawingArea::new();
    orb_content.set_content_width(80);
    orb_content.set_content_height(80);
    let orb_folded = gtk4::DrawingArea::new();
    orb_folded.set_content_width(80);
    orb_folded.set_content_height(80);
    board
        .add_card_with_id(
            "orb-probe",
            &orb_content,
            CardConfig {
                title: Some("Orb probe".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(700.0, 300.0))),
                shape: floating_board::CardShape::Circle,
                foldable: true,
                folded_content: Some(orb_folded.upcast()),
                unfold_on_click: true,
                fold_animation: floating_board::CardFoldAnimation::Spin,
                fold_timing: floating_board::CardFoldTiming::Overlap(Fraction::new(0.3)),
                unfold_animation: Some(floating_board::CardFoldAnimation::Shrink),
                unfold_timing: Some(floating_board::CardFoldTiming::Sequential),
                drag_handle: floating_board::DragHandle::WholeCard,
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("a", Button::with_label("a")),
                        SatelliteConfig::custom("b", Button::with_label("b")),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .ok();

    window.set_child(Some(&board));
    window.present();

    // t=600ms: close the set (staggered collapse into the seed).
    {
        let board = board.clone();
        let id = id.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(600), move || {
            println!("t=600  set_satellites_open(false)");
            board.set_satellites_open(&id, false).ok();
        });
    }
    // t=1600ms: re-open (staggered emerge from the seed).
    {
        let board = board.clone();
        let id = id.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(1600), move || {
            println!("t=1600 set_satellites_open(true)");
            board.set_satellites_open(&id, true).ok();
        });
    }
    // t=2600ms: fold choreography - revert, animate closed, then fold.
    for card in board.cards() {
        let card = card.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(2600), move || {
            println!("t=2600 fold()");
            card.fold();
        });
    }
    // t=2700ms: mid-choreo report - the card should still stand
    // unfolded while the satellites shrink away.
    {
        let board = board.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(2700), move || {
            for card in board.cards() {
                println!("t=2700 folded={} satellites_open={:?}", card.folded(), board.satellites_open(&card.id()));
            }
        });
    }
    // t=3400ms: the choreography landed - the card is really folded now.
    {
        let board = board.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(3400), move || {
            for card in board.cards() {
                println!(
                    "t=3400 folded={} satellites_open={:?} size={}x{}",
                    card.folded(),
                    board.satellites_open(&card.id()),
                    card.width(),
                    card.height()
                );
            }
        });
    }
    // t=3900ms: late read - the choreography landed by now.
    {
        let board = board.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(3900), move || {
            for card in board.cards() {
                println!(
                    "t=3900 folded={} satellites_open={:?} size={}x{}",
                    card.folded(),
                    board.satellites_open(&card.id()),
                    card.width(),
                    card.height()
                );
            }
        });
    }
    // t=4200ms: unfold - the set replays its reveal.
    for card in board.cards() {
        let card = card.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(4200), move || {
            println!("t=4200 fold()");
            card.fold();
        });
    }
    // t=5200ms: final report + quit.
    {
        let board = board.clone();
        let application = application.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(5200), move || {
            for card in board.cards() {
                println!(
                    "t=5200 folded={} satellites_open={:?} size={}x{}",
                    card.folded(),
                    board.satellites_open(&card.id()),
                    card.width(),
                    card.height()
                );
            }
            println!("motion probe done");
            application.quit();
        });
    }
}
