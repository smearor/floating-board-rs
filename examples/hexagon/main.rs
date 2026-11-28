//! Hexagon showcase: `CardShape::Hexagon` is a flat-top hexagon -
//! horizontal top and bottom edges, slanted left/right edges meeting
//! in vertices. The bounding box is `2R` wide and `sqrt(3) * R`
//! tall; the content lives in the inscribed square and every
//! non-satellite chrome slot lines up on the straight bar along the
//! flat bottom edge - there is no header strip and no arc band.
//! Taps in the bounding-box corners pass through to whatever is
//! underneath.
//!
//! The four cards demonstrate `CardContent`: "tile" is `Inscribed`
//! (the token stays in the safe inscribed square), "die" is `Fill`
//! (the face paints the rim-shrunk silhouette's corners), "hive" is
//! `Clipped` (the painted corners are cut at the rim, the card face
//! shows through) and "gem" is `Cover` (the stone fills the card
//! edge-to-edge).
//!
//! Card "tile" plays the game-tile look: a flat-top resource hex
//! with a number token - hexagons tile the plane, which is what
//! makes them interesting for boards.
//!
//! Card "die" is a plain die face: pips on a painted square that
//! reaches into the silhouette's corner triangles; the bar carries
//! `[Minimize, rotate CCW, "Die", rotate CW, Close]`.
//!
//! Card "hive" draws a mini honeycomb patch - seven tessellated
//! hexagons - with both bars. A palette tints the whole card
//! honey-amber.
//!
//! Card "gem" is a brilliant-cut stone filling the card
//! edge-to-edge - `Cover` gives the content the whole bounding box.
//!
//! Run with `cargo run --example hexagon` (optionally
//! `--features adwaita` for themed chrome).

mod die;
mod gem;
mod hive;
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
use gtk4::DrawingArea;
use gtk4::Orientation;
use gtk4::prelude::*;
use hive::build_hive;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use tile::build_tile;
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
    let application = gtk4::Application::new(Some("io.smearor.floating-board.hexagon"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board hexagon"));
    window.set_default_size(1100, 800);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // The game tile (`CardContent::Inscribed`): the inscribed
    // square holds the token tile, `[Minimize, "Tile", Close]`
    // rides the straight top bar - the hexagon's header along the
    // flat top edge.
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

    // The die (`CardContent::Fill`): six pips on a painted square
    // filling the rim-shrunk silhouette - footer bar only, with
    // rotation steps flanking the title.
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

    // The honeycomb patch (`CardContent::Clipped`) with a
    // honey-amber palette: the hexagon silhouette and the
    // tessellated content rhyme.
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

    // The gem (`CardContent::Cover`): a brilliant-cut stone filling
    // the card edge-to-edge - the silhouette is the only bound, no
    // ring stays card surface.
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
}
