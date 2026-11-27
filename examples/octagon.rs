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
use gtk4::cairo::Context;
use gtk4::prelude::*;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Vec2;
use tracing_subscriber::EnvFilter;

/// Traces a flat-top octagon of `inradius` centered at `center` as
/// a closed cairo path - the vertices come straight from the
/// commons `Octagon` primitive.
fn octagon_path(context: &Context, center: Vec2<Pixels>, inradius: f64) {
    let vertices = Octagon::with_inradius(Vec2::<()>::new(center.x, center.y), inradius, PolygonOrientation::FlatTop).vertices();
    context.move_to(vertices[0].x, vertices[0].y);
    for vertex in &vertices[1..] {
        context.line_to(vertex.x, vertex.y);
    }
    context.close_path();
}

/// A stop sign: red face, white inner ring, bold lettering - the
/// look everyone reads as an octagon.
fn build_stop(size: Pixels<i32>) -> gtk4::DrawingArea {
    let stop = gtk4::DrawingArea::new();
    stop.set_content_width(size.as_raw());
    stop.set_content_height(size.as_raw());

    stop.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        // `CardContent::Cover` gives the widget the card's whole
        // bounding square - the sign reaches the silhouette
        // edge-to-edge.
        let inradius = size.x.min(size.y) / 2.0;
        // Red face with the white inner octagon ring.
        octagon_path(context, center, inradius);
        context.set_source_rgb(0.72, 0.11, 0.11);
        let _ = context.fill_preserve();
        octagon_path(context, center, inradius * 0.86);
        context.set_source_rgb(0.96, 0.96, 0.97);
        context.set_line_width(3.0);
        let _ = context.stroke();
        // The lettering fills the face.
        context.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
        context.set_font_size(inradius * 0.52);
        if let Ok(extents) = context.text_extents("STOP") {
            context.move_to(center.x - extents.width() / 2.0, center.y + extents.height() / 2.0);
            let _ = context.show_text("STOP");
        }
    });
    stop
}

/// An octagonal casino plaque: the flat edges carry alternating
/// wedge marks (the chip edge inserts), the center shows the
/// denomination.
fn build_chip(size: Pixels<i32>) -> gtk4::DrawingArea {
    let chip = gtk4::DrawingArea::new();
    chip.set_content_width(size.as_raw());
    chip.set_content_height(size.as_raw());
    chip.set_draw_func(move |_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        let inradius = size.x.min(size.y) / 2.0 - 2.0;
        // Deep green felt face.
        octagon_path(context, center, inradius);
        context.set_source_rgb(0.10, 0.32, 0.20);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.85, 0.80, 0.55);
        context.set_line_width(3.0);
        let _ = context.stroke();
        // The eight edge inserts: a light wedge on each edge midpoint,
        // rotated by `k * 45°` around the center - the plaques' own
        // eightfold mark.
        let wedge_outer = inradius * 0.97;
        let wedge_inner = inradius * 0.66;
        for edge in 0..8 {
            let angle = f64::from(edge) * std::f64::consts::FRAC_PI_4;
            let (sin, cos) = angle.sin_cos();
            // A thin trapezoid hugging the edge: outer edge half-length
            // `tan(22.5°) * outer`, inner a touch wider.
            let outer_half = wedge_outer * 0.414 * 0.7;
            let inner_half = wedge_inner * 0.414;
            let edge_x = |offset: f64, radial: f64| center.x + radial * cos - offset * sin;
            let edge_y = |offset: f64, radial: f64| center.y + radial * sin + offset * cos;
            context.move_to(edge_x(-outer_half, wedge_outer), edge_y(-outer_half, wedge_outer));
            context.line_to(edge_x(-inner_half, wedge_inner), edge_y(-inner_half, wedge_inner));
            context.line_to(edge_x(inner_half, wedge_inner), edge_y(inner_half, wedge_inner));
            context.line_to(edge_x(outer_half, wedge_outer), edge_y(outer_half, wedge_outer));
            context.close_path();
            context.set_source_rgb(0.90, 0.88, 0.78);
            let _ = context.fill();
        }
        // The denomination.
        context.set_source_rgb(0.93, 0.92, 0.85);
        context.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
        context.set_font_size(inradius * 0.38);
        if let Ok(extents) = context.text_extents("500") {
            context.move_to(center.x - extents.width() / 2.0, center.y + extents.height() / 2.0);
            let _ = context.show_text("500");
        }
    });
    chip
}

/// An eight-point compass rose: cardinal arms reach the flat-edge
/// midpoints, intercardinal arms the diagonal-edge midpoints - the
/// octagon's own eightfold grid.
fn build_compass(size: Pixels<i32>) -> gtk4::DrawingArea {
    let compass = gtk4::DrawingArea::new();
    compass.set_content_width(size.as_raw());
    compass.set_content_height(size.as_raw());
    compass.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        let inradius = size.x.min(size.y) / 2.0;
        // Painted square background: under `CardContent::Clipped`
        // the corner triangles are cut away and the card face shows
        // through them (a `Fill` content would keep them painted).
        context.set_source_rgb(0.10, 0.15, 0.24);
        let _ = context.paint();
        // Night-blue face.
        octagon_path(context, center, inradius);
        context.set_source_rgb(0.13, 0.20, 0.32);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.55, 0.67, 0.80);
        context.set_line_width(2.5);
        let _ = context.stroke();
        // Eight arms: each arm is a slim kite from the center to an
        // edge midpoint - cardinals (flat edges, k even) full length,
        // intercardinals (diagonal edges) shorter.
        for arm in 0..8 {
            let angle = f64::from(arm) * std::f64::consts::FRAC_PI_4 - std::f64::consts::FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            let cardinal = arm % 2 == 0;
            let tip = inradius * if cardinal { 0.92 } else { 0.62 };
            let wing = inradius * if cardinal { 0.16 } else { 0.11 };
            let wing_angle = angle + std::f64::consts::FRAC_PI_4;
            let (ws, wc) = wing_angle.sin_cos();
            context.move_to(center.x, center.y);
            context.line_to(center.x + tip * cos, center.y + tip * sin);
            context.line_to(center.x + wing * wc, center.y + wing * ws);
            context.close_path();
            if cardinal {
                context.set_source_rgb(0.91, 0.35, 0.32);
            } else {
                context.set_source_rgb(0.82, 0.86, 0.90);
            }
            let _ = context.fill();
        }
        // Center hub + north label.
        context.arc(center.x, center.y, inradius * 0.09, 0.0, std::f64::consts::TAU);
        context.set_source_rgb(0.95, 0.95, 0.96);
        let _ = context.fill();
        context.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
        context.set_font_size(inradius * 0.24);
        if let Ok(extents) = context.text_extents("N") {
            context.move_to(center.x - extents.width() / 2.0, center.y - inradius * 0.36);
            let _ = context.show_text("N");
        }
    });
    compass
}

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
