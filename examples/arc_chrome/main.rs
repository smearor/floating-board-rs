//! Arc-chrome showcase: `CardShape::Circle` mounts its inline chrome
//! along the disc's top quarter - an "arc header" that mirrors the
//! header bar's order: leading slots left of the centered title,
//! trailing slots right of it.
//!
//! Card "dial" runs the builtin style with `CardContent::Fill`: the
//! clock face fills the inner square, so the band's buttons float over
//! the disc's rim. The card folds onto a small clock disc.
//!
//! Card "compass" runs the Adwaita style (with `--features adwaita`)
//! on a wider 135° span: the rotate steps pin to the band's ends via
//! `InlineStart`/`InlineEnd` while the title rides the center zone -
//! the band drawn as raised circular theme buttons. A narrow 30°
//! bottom band mirrors it and hosts `Minimize`/`Close` on its
//! start/end slots. Its dial carries two needles: a green one fixed
//! to the card's "up" and a red one that keeps pointing to the
//! board's top edge no matter how the card is rotated.
//!
//! Card "pie" carries a palette and a second band: its bottom arc
//! mirrors the top one and hosts the seat-pass actions for a user
//! facing the card's bottom edge. One satellite dot per slice color
//! orbits the disc - a click grows the matching slice by 5% (red's
//! first click seeds a 10% slice); no slice drops below 10%.
//!
//! Card "gauge" runs `CardContent::Cover`: its dial paints
//! edge-to-edge, bounded only by the card's silhouette clip. A single
//! bottom band hosts `Minimize`/`Title`/`Close` - there is no top
//! arc. The cover content is display-only (non-targetable), so the
//! interaction rides the `+`/`-` satellites trimming the shared level.
//!
//! Run with `cargo run --example arc_chrome` (optionally
//! `--features adwaita` for themed chrome).

mod clock;
mod compass;
mod gauge;
mod pie;

use clock::build_clock;
use compass::build_compass;
use compass::paint_compass;
use floating_board::Card;
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
use floating_board::ChromeStyle;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::SatelliteConfig;
use floating_board::SatelliteLayout;
use gauge::build_gauge;
use gauge::level_satellite;
use gtk4::CssProvider;
use gtk4::DrawingArea;
use gtk4::Orientation;
use gtk4::gdk::Display;
use gtk4::glib::closure_local;
use gtk4::prelude::*;
use pie::PieColor;
use pie::PieShares;
use pie::build_pie;
use pie::slice_satellite;
use smearor_commons_color::Color;
use smearor_commons_drawing::CairoTarget;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    #[cfg(feature = "adwaita")]
    {
        if let Err(error) = floating_board::adw::init() {
            panic!("libadwaita init failed: {error}");
        }
        eprintln!("adwaita active - the compass arc renders as themed circular buttons");
    }
    #[cfg(not(feature = "adwaita"))]
    eprintln!("built without `adwaita` - rerun with `--features adwaita` to see the themed compass chrome");
    let application = gtk4::Application::new(Some("io.smearor.floating-board.arc-chrome"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board arc chrome"));
    window.set_default_size(1100, 800);

    // Example-local styling: the pie's slice satellites are round
    // dots in their slice's color.
    let css = CssProvider::new();
    css.load_from_string(
        ".pie-dot { min-width: 34px; min-height: 34px; padding: 0; border-radius: 9999px; border-width: 2px; border-style: solid; }         .pie-blue { background: rgba(82, 158, 214, 0.55); border-color: rgba(82, 158, 214, 0.9); }         .pie-green { background: rgba(61, 184, 122, 0.55); border-color: rgba(61, 184, 122, 0.9); }         .pie-yellow { background: rgba(235, 168, 51, 0.55); border-color: rgba(235, 168, 51, 0.9); }         .pie-red { background: rgba(217, 64, 56, 0.55); border-color: rgba(217, 64, 56, 0.9); }         .gauge-btn { min-width: 34px; min-height: 34px; padding: 0; border-radius: 9999px; border-width: 2px; border-style: solid; font-weight: bold; }         .gauge-plus { background: rgba(89, 199, 143, 0.55); border-color: rgba(89, 199, 143, 0.9); }         .gauge-minus { background: rgba(150, 160, 190, 0.55); border-color: rgba(150, 160, 190, 0.9); }",
    );
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // The builtin arc header: leading slots sit left of the centered
    // title, trailing slots right of it - the header bar's order on a
    // disc.
    board
        .add_card_with_id(
            "dial",
            &build_clock(Pixels::new(280)),
            CardConfig {
                title: Some("Dial".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 400.0))),
                shape: CardShape::Circle,
                content: CardContent::Fill,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::RotateClockwise),
                    ChromeSlot::inline(ChromeElement::Close),
                ]),
                folded_content: Some(build_clock(Pixels::new(90)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("dial"));

    // The Adwaita arc: raised circular theme buttons over the chrome
    // ring; the compass face gains its world-fixed red needle once the
    // card exists.
    let compass_face = build_compass(Pixels::new(200));
    board
        .add_card_with_id(
            "compass",
            &compass_face,
            CardConfig {
                title: Some("Compass".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(750.0, 380.0))),
                shape: CardShape::Circle,
                // A wider arc than the 90° default: the band wraps a
                // third of the disc's rim. The narrow bottom band
                // mirrors it as a second header strip.
                arc_span: Some(Degrees::new(135.0)),
                arc_bottom_span: Some(Degrees::new(30.0)),
                chrome_style: ChromeStyle::Adwaita,
                // `InlineStart`/`InlineEnd` pin to the band's ends like
                // a header bar's packed ends; the title rides the
                // center zone. `Minimize`/`Close` pack the narrow
                // bottom band's start/end.
                chrome: Some(vec![
                    ChromeSlot::inline_start(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline_end(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_compass(Pixels::new(80)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("compass"));

    // The red needle counter-rotates against the card's rotation so it
    // always points to the board's top edge; the green one is drawn
    // card-fixed and rotates along. The card's `RotatedLayout` paints
    // the rotation without invalidating the face, so `card-rotated`
    // (emitted per `notify::rotation`) queues the redraw.
    if let Ok(card) = board.card(&CardId::new("compass")) {
        let weak_card = card.downgrade();
        compass_face.set_draw_func(move |_area, context, width, height| {
            let world_up = weak_card.upgrade().map(|card| -card.rotation().to_rotation_rad());
            let mut target = CairoTarget::new(context, Vec2::new(f64::from(width), f64::from(height)));
            paint_compass(&mut target, Vec2::<Pixels>::new(f64::from(width), f64::from(height)), world_up);
        });
        let face = compass_face.downgrade();
        card.connect_closure(
            "card-rotated",
            false,
            closure_local!(move |_card: Card| {
                if let Some(face) = face.upgrade() {
                    face.queue_draw();
                }
            }),
        );
    }

    // A small disc with a palette: the band takes the palette's
    // header color. The slot order mirrors a header bar - handle and
    // minimize lead, title centers, close packs the band's far end.
    let mut palette = CardPalette::new();
    palette.insert(CardColor::Background, Color::<f64>::new(0.30, 0.26, 0.36));
    palette.insert(CardColor::Header, Color::<f64>::new(0.45, 0.38, 0.55));
    palette.insert(CardColor::Foreground, Color::<f64>::new(0.92, 0.90, 0.95));
    // The face and its folded mini draw from the same shares - slice
    // satellites grow (or seed) their color's slice on click.
    let pie_shares = Rc::new(RefCell::new(PieShares::INITIAL));
    let pie_face = build_pie(Pixels::new(150), pie_shares.clone());
    board
        .add_card_with_id(
            "pie",
            &pie_face,
            CardConfig {
                title: Some("Pie".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(550.0, 600.0))),
                shape: CardShape::Circle,
                palette: Some(palette),
                // The bottom band mirrors the top one - the seat-pass
                // actions read correctly for a user facing the card's
                // bottom edge (start = the user's left).
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::arc_top_end(ChromeElement::Close),
                    ChromeSlot::bar_start(ChromeElement::PreviousSeat),
                    ChromeSlot::bar(ChromeElement::Minimize),
                    ChromeSlot::bar_end(ChromeElement::NextSeat),
                ]),
                folded_content: Some(build_pie(Pixels::new(60), pie_shares.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                // One dot per slice color, in draw order: blue, green
                // and yellow step their slice by +5%, red seeds a
                // 10% slice first and steps after.
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("blue", slice_satellite("pie-blue", &pie_face, &pie_shares, PieColor::Blue)),
                        SatelliteConfig::custom("green", slice_satellite("pie-green", &pie_face, &pie_shares, PieColor::Green)),
                        SatelliteConfig::custom("yellow", slice_satellite("pie-yellow", &pie_face, &pie_shares, PieColor::Yellow)),
                        SatelliteConfig::custom("red", slice_satellite("pie-red", &pie_face, &pie_shares, PieColor::Red)),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("pie"));

    // The gauge (`CardContent::Cover`): the dial reaches the
    // silhouette, no ring stays chrome. Its only band is the bottom
    // one - title centered between the packed Minimize/Close ends.
    // Content is display-only: the +/- satellites trim the level.
    let gauge_level = Rc::new(Cell::new(Fraction::new(0.45)));
    let gauge_face = build_gauge(Pixels::new(170), gauge_level.clone());
    board
        .add_card_with_id(
            "gauge",
            &gauge_face,
            CardConfig {
                title: Some("Gauge".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(920.0, 610.0))),
                shape: CardShape::Circle,
                content: CardContent::Cover,
                arc_bottom_span: Some(Degrees::new(80.0)),
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(Pixels::new(70), gauge_level.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("down", level_satellite("-", "gauge-minus", -0.1, &gauge_face, &gauge_level)),
                        SatelliteConfig::custom("up", level_satellite("+", "gauge-plus", 0.1, &gauge_face, &gauge_level)),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("gauge"));

    window.set_child(Some(&board));
    window.present();
}
