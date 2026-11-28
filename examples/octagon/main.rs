//! Octagon showcase: `CardShape::Octagon` is a flat-top octagon -
//! horizontal/vertical flat edges, diagonal edges cutting the four
//! corners at 45 degrees. The bounding box is the square `2a x 2a`
//! (`a` = inradius, center to flat edge); the content lives in the
//! inscribed square and every non-satellite chrome slot lines up on
//! a straight bar along a flat edge - there is no header strip and
//! no arc band. Taps in the bounding-box corners pass through to
//! whatever is underneath.
//!
//! The three cards demonstrate `CardContent`: "stop" is `Cover`
//! (the sign reaches the silhouette edge-to-edge), "chip" is
//! `Inscribed` (the plaque stays in the safe inscribed square) and
//! "compass" is `Clipped` (the rose's painted corners are cut at
//! the rim, the card face shows through).
//!
//! Card "stop" plays the shape's most famous role: a stop sign -
//! red face, white inner ring, bold lettering - with the chrome on
//! the top-edge bar, no rotation.
//!
//! Card "chip" is an octagonal casino plaque: the inscribed square
//! holds the wedge-marked chip; the footer bar carries
//! `[Minimize, rotate CCW, "Chip", rotate CW, Close]` - the 45
//! degree snap rhymes with the plaque's eightfold wedges.
//!
//! Card "compass" draws an eight-point compass rose - N/NE/E/... -
//! with both bars and a steel-blue palette; the eightfold symmetry
//! of the shape is the point.
//!
//! Run with `cargo run --example octagon` (optionally
//! `--features adwaita` for themed chrome).

mod chip;
mod compass;
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
use stop::build_stop;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    #[cfg(feature = "adwaita")]
    {
        if let Err(error) = floating_board::adw::init() {
            panic!("libadwaita init failed: {error}");
        }
        eprintln!("adwaita active");
    }
    #[cfg(not(feature = "adwaita"))]
    eprintln!("built without `adwaita` - rerun with `--features adwaita` for themed chrome");
    let application = gtk4::Application::new(Some("io.smearor.floating-board.octagon"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board octagon"));
    window.set_default_size(1100, 800);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // The stop sign (`CardContent::Cover`): the sign fills the
    // card edge-to-edge, `[Minimize, "Stop", Close]` rides the
    // top-edge bar - no rotation chrome.
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

    // The casino plaque (`CardContent::Inscribed`): wedge-marked
    // edges on the inscribed square - footer bar only, with
    // rotation steps flanking the title (45 degree snaps fit the
    // eightfold plaques).
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

    // The compass rose (`CardContent::Clipped`): eight arms on the
    // eightfold grid on a painted square - the clip cuts the corner
    // triangles, the card face shows through. Both bars plus seat
    // passing, steel-blue palette.
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
}
