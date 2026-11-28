//! Render probe for `CardShape::Octagon`: builds three octagon
//! cards, snapshots the board into a PNG once laid out, and exits.
//! Run with `cargo run --example oct_probe` - writes
//! /tmp/oct_probe.png.

#[path = "../octagon/chip/mod.rs"]
mod chip;
#[path = "../octagon/compass/mod.rs"]
mod compass;
#[path = "../octagon/stop/mod.rs"]
mod stop;

use chip::build_chip;
use compass::build_compass;
use floating_board::CardAnchor;
use floating_board::CardColor;
use floating_board::CardConfig;
use floating_board::CardContent;
use floating_board::CardFoldAnimation;
use floating_board::CardId;
use floating_board::CardPalette;
use floating_board::CardPosition;
use floating_board::CardShape;
use floating_board::ChromeElement;
use floating_board::ChromeSlot;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::time::Duration;
use stop::build_stop;

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.oct-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("oct probe"));
    window.set_default_size(1100, 800);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    board
        .add_card_with_id(
            "stop",
            &build_stop(Pixels::new(180)),
            CardConfig {
                title: Some("Stop".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(280.0, 380.0))),
                shape: CardShape::Octagon,
                content: CardContent::Cover,
                chrome: Some(vec![
                    ChromeSlot::arc_top_start(ChromeElement::Minimize),
                    ChromeSlot::arc_top(ChromeElement::Title),
                    ChromeSlot::arc_top_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_stop(Pixels::new(80)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("stop"));

    board
        .add_card_with_id(
            "chip",
            &build_chip(Pixels::new(160)),
            CardConfig {
                title: Some("Chip".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(660.0, 340.0))),
                shape: CardShape::Octagon,
                content: CardContent::Inscribed,
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_chip(Pixels::new(72)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("chip"));

    let mut palette = CardPalette::new();
    palette.insert(CardColor::Background, smearor_commons_color::Color::<f64>::new(0.16, 0.24, 0.36));
    palette.insert(CardColor::Header, smearor_commons_color::Color::<f64>::new(0.30, 0.44, 0.62));
    palette.insert(CardColor::Foreground, smearor_commons_color::Color::<f64>::new(0.86, 0.91, 0.97));
    board
        .add_card_with_id(
            "compass",
            &build_compass(Pixels::new(200)),
            CardConfig {
                title: Some("Compass".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(470.0, 545.0))),
                shape: CardShape::Octagon,
                content: CardContent::Clipped,
                palette: Some(palette),
                chrome: Some(vec![
                    ChromeSlot::arc_top(ChromeElement::Title),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::PreviousSeat),
                    ChromeSlot::bar(ChromeElement::NextSeat),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_compass(Pixels::new(88)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Crossfade,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("compass"));

    window.set_child(Some(&board));
    window.present();

    // Pick probe: bounding-box corners must pass through to the
    // board filler (unnamed), centers/bars hit the named content
    // areas or chrome.
    let probe_board = board.clone();
    gtk4::glib::timeout_add_local_once(Duration::from_millis(600), move || {
        for (name, bx, by) in [
            ("stop corner", 190.0, 290.0),
            ("stop center", 280.0, 380.0),
            ("stop top bar", 280.0, 300.0),
            ("chip corner", 535.0, 215.0),
            ("chip center", 660.0, 340.0),
            ("compass corner", 365.0, 435.0),
            ("compass center", 470.0, 545.0),
            ("compass bottom bar", 470.0, 645.0),
        ] {
            let picked = probe_board.pick(bx, by, gtk4::PickFlags::DEFAULT);
            eprintln!(
                "pick {name} ({bx},{by}) -> {}",
                picked.map(|w| format!("{} '{}'", w.type_().name(), w.widget_name())).unwrap_or_default()
            );
        }
    });

    // After layout, snapshot the whole board into a PNG and quit.
    gtk4::glib::timeout_add_local_once(Duration::from_millis(800), move || {
        smearor_commons_fx::gtk::snapshot_to_png(&board, "/tmp/oct_probe.png").expect("png dump");
        eprintln!("wrote /tmp/oct_probe.png {}x{}", board.width(), board.height());
        std::process::exit(0);
    });
}
