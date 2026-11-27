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
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use tracing_subscriber::EnvFilter;

/// Traces a flat-top hexagon of `radius` centered at `center` as a
/// closed cairo path - the vertices come straight from the commons
/// `Hexagon` primitive.
fn hexagon_path(context: &Context, center: Vec2<Pixels>, radius: f64) {
    let vertices = Hexagon::flat_top(Vec2::<()>::new(center.x, center.y), radius).vertices();
    context.move_to(vertices[0].x, vertices[0].y);
    for vertex in &vertices[1..] {
        context.line_to(vertex.x, vertex.y);
    }
    context.close_path();
}

/// A game tile: a stroked flat-top hexagon on a sand-colored face
/// with a number token (value plus probability pips underneath) -
/// the look hexagon grids are known for.
fn build_tile(size: Pixels<i32>) -> gtk4::DrawingArea {
    let tile = gtk4::DrawingArea::new();
    tile.set_content_width(size.as_raw());
    tile.set_content_height(size.as_raw());
    tile.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        let radius = size.x.min(size.y) / 2.0 - 4.0;
        // The tile face.
        hexagon_path(context, center, radius);
        context.set_source_rgb(0.85, 0.71, 0.40);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.42, 0.30, 0.12);
        context.set_line_width(4.0);
        let _ = context.stroke();
        // The number token: a light disc, the value above, five pips
        // below (the "8" odds row).
        let token = Pixels::new(radius * 0.42);
        context.arc(center.x, center.y, token.as_f64(), 0.0, std::f64::consts::TAU);
        context.set_source_rgb(0.96, 0.93, 0.86);
        let _ = context.fill();
        context.set_source_rgb(0.20, 0.16, 0.10);
        context.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
        context.set_font_size(token.as_f64() * 1.1);
        if let Ok(extents) = context.text_extents("8") {
            context.move_to(center.x - extents.width() / 2.0, center.y + extents.height() / 2.0 - token.as_f64() * 0.15);
            let _ = context.show_text("8");
        }
        for pip in 0..5 {
            let offset = f64::from(pip - 2) * token.as_f64() * 0.28;
            context.arc(center.x + offset, center.y + token.as_f64() * 0.55, token.as_f64() * 0.07, 0.0, std::f64::consts::TAU);
            let _ = context.fill();
        }
    });
    tile
}

/// A die face: `pips` pips in the standard grid pattern on a dark
/// face - content stays a square, matching the hexagon's inscribed
/// square.
fn build_die(size: Pixels<i32>, pips: u8) -> gtk4::DrawingArea {
    let die = gtk4::DrawingArea::new();
    die.set_content_width(size.as_raw());
    die.set_content_height(size.as_raw());
    die.set_draw_func(move |_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        let half = size.x.min(size.y) / 2.0 - 4.0;
        // `CardContent::Fill` allocates the rim-shrunk silhouette's
        // bounding box: the painted square background reaches into
        // the corner triangles a `Clipped` content would lose.
        context.set_source_rgb(0.08, 0.09, 0.12);
        let _ = context.paint();
        // The face: a flat-top hexagon keeps the die inside the
        // card's silhouette.
        hexagon_path(context, center, half);
        context.set_source_rgb(0.13, 0.14, 0.19);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.55, 0.58, 0.68);
        context.set_line_width(3.0);
        let _ = context.stroke();
        // Standard pip layout on a 3x3 grid.
        let grid = half * 0.52;
        let pip_radius = half * 0.14;
        let spots: &[(f64, f64)] = match pips {
            1 => &[(0.0, 0.0)],
            2 => &[(-1.0, -1.0), (1.0, 1.0)],
            3 => &[(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            4 => &[(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)],
            5 => &[(-1.0, -1.0), (1.0, -1.0), (0.0, 0.0), (-1.0, 1.0), (1.0, 1.0)],
            _ => &[(-1.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (1.0, 1.0)],
        };
        context.set_source_rgb(0.92, 0.93, 0.97);
        for (gx, gy) in spots {
            context.arc(center.x + gx * grid, center.y + gy * grid, pip_radius, 0.0, std::f64::consts::TAU);
            let _ = context.fill();
        }
    });
    die
}

/// A honeycomb patch: the center hexagon plus its six neighbors -
/// flat-top hexagons tile the plane with centers spaced `sqrt(3)*r`
/// horizontally and `1.5*r` vertically.
fn build_hive(size: Pixels<i32>) -> gtk4::DrawingArea {
    let hive = gtk4::DrawingArea::new();
    hive.set_content_width(size.as_raw());
    hive.set_content_height(size.as_raw());
    hive.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        let cell = size.x.min(size.y) / 5.5;
        // Neighbor centers for flat-top tiling: the cells above and
        // below share the flat edges (`sqrt(3) * r` apart), the four
        // diagonal cells share the slanted edges (`1.5r` over,
        // `sqrt(3) * r / 2` down) - edge-to-edge, no gaps.
        let neighbors: [(f64, f64); 6] = [(0.0, -1.732), (0.0, 1.732), (1.5, -0.866), (1.5, 0.866), (-1.5, -0.866), (-1.5, 0.866)];
        // Painted square background: under `CardContent::Clipped`
        // the corner triangles are cut away and the card face shows
        // through them (a `Fill` content would keep them painted).
        context.set_source_rgb(0.24, 0.17, 0.05);
        let _ = context.paint();
        context.set_line_width(2.5);
        for (index, (dx, dy)) in neighbors.iter().enumerate() {
            let cell_center = center + Vec2::<Pixels>::new(dx * cell, dy * cell);
            hexagon_path(context, cell_center, cell);
            context.set_source_rgb(0.32 + 0.06 * f64::from(index as i32 % 3), 0.24, 0.08);
            let _ = context.fill_preserve();
            context.set_source_rgb(0.95, 0.77, 0.28);
            let _ = context.stroke();
        }
        hexagon_path(context, center, cell);
        context.set_source_rgb(0.55, 0.38, 0.10);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.95, 0.77, 0.28);
        context.set_line_width(3.0);
        let _ = context.stroke();
    });
    hive
}

/// A brilliant-cut hexagonal gem: an inner "table" facet plus the
/// six crown facets between table edges and silhouette - `Cover`
/// content fills the card edge-to-edge.
fn build_gem(size: Pixels<i32>) -> gtk4::DrawingArea {
    let gem = gtk4::DrawingArea::new();
    gem.set_content_width(size.as_raw());
    gem.set_content_height(size.as_raw());
    gem.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        // `CardContent::Cover` allocates the card's whole bounding
        // box `2R x sqrt(3)R` - the gem reaches the silhouette.
        let radius = (size.x / 2.0).min(size.y / 1.732_050_8);
        let table = radius * 0.55;
        let outer = Hexagon::flat_top(Vec2::<()>::new(center.x, center.y), radius).vertices();
        let inner = Hexagon::flat_top(Vec2::<()>::new(center.x, center.y), table).vertices();
        // Crown facets: alternating shade quads between inner edges
        // and the outer silhouette.
        for facet in 0..6 {
            let next = (facet + 1) % 6;
            context.move_to(inner[facet].x, inner[facet].y);
            context.line_to(inner[next].x, inner[next].y);
            context.line_to(outer[next].x, outer[next].y);
            context.line_to(outer[facet].x, outer[facet].y);
            context.close_path();
            let shade = 0.42 + 0.10 * f64::from(facet as i32 % 3);
            context.set_source_rgb(0.15 * shade, 0.72 * shade, 0.78 * shade);
            let _ = context.fill_preserve();
            context.set_source_rgb(0.05, 0.20, 0.22);
            context.set_line_width(1.5);
            let _ = context.stroke();
        }
        // The table: a light flat-top facet.
        hexagon_path(context, center, table);
        context.set_source_rgb(0.55, 0.92, 0.94);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.08, 0.30, 0.33);
        context.set_line_width(2.0);
        let _ = context.stroke();
    });
    gem
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
    let application = gtk4::Application::new(Some("io.smearor.floating-board.hexagon"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board hexagon"));
    window.set_default_size(1100, 800);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
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
