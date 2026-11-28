//! Render probe for `CardShape::Hexagon`: builds a hexagon card,
//! snapshots the board into a PNG once laid out, and exits. Run
//! with `cargo run --example hex_probe` - writes /tmp/hex_probe.png.

#[path = "../hexagon/die/mod.rs"]
mod die;
#[path = "../hexagon/gem/mod.rs"]
mod gem;
#[path = "../hexagon/hive/mod.rs"]
mod hive;
#[path = "../hexagon/tile/mod.rs"]
mod tile;

use die::build_die;
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
use gem::build_gem;
use gtk4::prelude::*;
use hive::build_hive;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::time::Duration;
use tile::build_tile;

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.hex-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("hex probe"));
    window.set_default_size(1280, 800);
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // The game tile: the inscribed square holds the token tile,
    // `[Minimize, "Tile", Close]` rides the straight bottom bar.
    board
        .add_card_with_id(
            "tile",
            &build_tile(Pixels::new(180)),
            CardConfig {
                title: Some("Tile".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(280.0, 380.0))),
                shape: CardShape::Hexagon,
                content: CardContent::Inscribed,
                chrome: Some(vec![
                    ChromeSlot::arc_top_start(ChromeElement::Minimize),
                    ChromeSlot::arc_top(ChromeElement::Title),
                    ChromeSlot::arc_top_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_tile(Pixels::new(80)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("tile"));

    // The die: six pips on the inscribed square, same bar layout.
    board
        .add_card_with_id(
            "die",
            &build_die(Pixels::new(160), 6),
            CardConfig {
                title: Some("Die".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(660.0, 340.0))),
                shape: CardShape::Hexagon,
                content: CardContent::Fill,
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_die(Pixels::new(72), 1).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("die"));

    // The honeycomb patch with a honey-amber palette: the hexagon
    // silhouette and the tessellated content rhyme.
    let mut palette = CardPalette::new();
    palette.insert(CardColor::Background, smearor_commons_color::Color::<f64>::new(0.36, 0.27, 0.10));
    palette.insert(CardColor::Header, smearor_commons_color::Color::<f64>::new(0.62, 0.47, 0.16));
    palette.insert(CardColor::Foreground, smearor_commons_color::Color::<f64>::new(0.98, 0.91, 0.72));
    board
        .add_card_with_id(
            "hive",
            &build_hive(Pixels::new(200)),
            CardConfig {
                title: Some("Hive".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(480.0, 585.0))),
                shape: CardShape::Hexagon,
                content: CardContent::Clipped,
                palette: Some(palette),
                chrome: Some(vec![
                    ChromeSlot::arc_top(ChromeElement::Title),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::PreviousSeat),
                    ChromeSlot::bar(ChromeElement::NextSeat),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_hive(Pixels::new(88)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Crossfade,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("hive"));

    // The gem (`CardContent::Cover`): edge-to-edge stone.
    board
        .add_card_with_id(
            "gem",
            &build_gem(Pixels::new(170)),
            CardConfig {
                title: Some("Gem".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(870.0, 590.0))),
                shape: CardShape::Hexagon,
                content: CardContent::Cover,
                chrome: Some(vec![ChromeSlot::arc_top(ChromeElement::Title), ChromeSlot::arc_top_end(ChromeElement::Close)]),
                folded_content: Some(build_gem(Pixels::new(76)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("gem"));

    window.set_child(Some(&board));
    window.present();

    // Pick probe: bounding-box corners must pass through to the
    // board filler (unnamed), centers/bars/vertices hit the named
    // content areas or chrome.
    let probe_board = board.clone();
    gtk4::glib::timeout_add_local_once(Duration::from_millis(600), move || {
        for (name, bx, by) in [
            ("tile corner", 130.0, 240.0),
            ("tile center", 280.0, 380.0),
            ("tile top bar", 280.0, 257.0),
            ("die corner", 560.0, 255.0),
            ("die center", 660.0, 340.0),
            ("hive corner", 355.0, 480.0),
            ("hive center", 480.0, 585.0),
            ("gem corner", 775.0, 480.0),
            ("gem vertex l", 770.0, 590.0),
            ("gem center", 870.0, 590.0),
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
        smearor_commons_fx::gtk::snapshot_to_png(&board, "/tmp/hex_probe.png").expect("png dump");
        eprintln!("wrote /tmp/hex_probe.png {}x{}", board.width(), board.height());
        std::process::exit(0);
    });
}
