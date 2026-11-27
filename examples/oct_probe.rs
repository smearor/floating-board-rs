//! Render probe for `CardShape::Octagon`: builds three octagon
//! cards, snapshots the board into a PNG once laid out, and exits.
//! Run with `cargo run --example oct_probe` - writes
//! /tmp/oct_probe.png.

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
use std::time::Duration;

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.oct-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

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
    stop.set_widget_name("stop-content");
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
    chip.set_widget_name("chip-content");
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
    compass.set_widget_name("compass-content");
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
        let (width, height) = (board.width() as f64, board.height() as f64);
        let snapshot = gtk4::Snapshot::new();
        let paintable = gtk4::WidgetPaintable::new(Some(&board));
        paintable.snapshot(&snapshot, width, height);
        let Some(node) = snapshot.to_node() else {
            eprintln!("no render node");
            std::process::exit(1);
        };
        let Some(surface) = board.native().and_then(|native| native.surface()) else {
            eprintln!("no surface");
            std::process::exit(1);
        };
        let renderer = gtk4::gsk::Renderer::for_surface(&surface).expect("renderer");
        let texture = renderer.render_texture(&node, Some(&gtk4::graphene::Rect::new(0.0, 0.0, width as f32, height as f32)));
        texture.save_to_png("/tmp/oct_probe.png").expect("png");
        eprintln!("wrote /tmp/oct_probe.png {width}x{height}");
        std::process::exit(0);
    });
}
