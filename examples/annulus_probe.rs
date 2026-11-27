//! Render probe for the `CardShape::Annulus` showcase: the same four
//! cards as `annulus.rs`, snapshots the board into a PNG once laid
//! out and picks the hole (must pass through to the board), the ring
//! band (must land on the card) and the cut corners. Run with
//! `cargo run --example annulus_probe` - writes
//! /tmp/annulus_probe.png.

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
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::DrawingArea;
use gtk4::cairo::Context;
use gtk4::cairo::FillRule;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

/// The hole's fraction of the outer radius - shared by the card
/// config and every face's drawing so painted holes line up with the
/// real one.
const HOLE_RATIO: f64 = 0.5;

/// Paints `segments` alternating wedges around the band, offset by
/// `phase` - the classic lifebuoy / donut pattern.
fn paint_segments(context: &Context, center: (f64, f64), outer: f64, ratio: f64, phase: f64, segments: usize, colors: &[(f64, f64, f64)]) {
    let (cx, cy) = center;
    let inner = outer * ratio;
    for index in 0..segments {
        let start = phase + index as f64 * 2.0 * PI / segments as f64;
        let end = phase + (index + 1) as f64 * 2.0 * PI / segments as f64;
        context.new_path();
        context.arc(cx, cy, outer, start, end);
        context.arc_negative(cx, cy, inner, end, start);
        context.close_path();
        let (r, g, b) = colors[index % colors.len()];
        context.set_source_rgb(r, g, b);
        context.fill().unwrap_or_default();
    }
}

/// `Inscribed` face: a red/white lifebuoy ring centered in the square
/// allocation - the hole region is left unpainted where the band
/// leaves it, the square's corners cover the rest.
fn build_lifebuoy(size: Pixels) -> DrawingArea {
    let face = DrawingArea::new();
    face.set_content_width(f64::from(size) as i32);
    face.set_content_height(f64::from(size) as i32);
    face.set_widget_name("lifebuoy-content");
    face.set_draw_func(move |_area, context, width, height| {
        let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        let outer = cx.min(cy);
        // A faint disc shows the square's extent: `Inscribed`
        // paints over the hole region on purpose.
        context.set_source_rgba(0.96, 0.96, 0.98, 0.35);
        context.arc(cx, cy, outer, 0.0, 2.0 * PI);
        context.fill().unwrap_or_default();
        paint_segments(context, (cx, cy), outer, 0.95 * 0.55, 0.0, 8, &[(0.85, 0.20, 0.18), (0.95, 0.94, 0.90)]);
        context.set_source_rgba(0.55, 0.10, 0.08, 0.9);
        context.set_line_width(2.0);
        context.arc(cx, cy, outer, 0.0, 2.0 * PI);
        context.stroke().unwrap_or_default();
    });
    face
}

/// `Fill` face: a six-slice donut painted to the square's bounds -
/// the corners of the square visibly paint over the hole, showing
/// that `Fill` bounds (not the silhouette) are the limit.
fn build_donut(size: Pixels) -> DrawingArea {
    let face = DrawingArea::new();
    face.set_content_width(f64::from(size) as i32);
    face.set_content_height(f64::from(size) as i32);
    face.set_draw_func(move |_area, context, width, height| {
        let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        let outer = cx.min(cy);
        // Painted square corners: `Fill` lets them escape into the
        // hole and the cut regions.
        context.set_source_rgba(0.40, 0.28, 0.14, 0.55);
        context.rectangle(0.0, 0.0, f64::from(width), f64::from(height));
        context.fill().unwrap_or_default();
        paint_segments(context, (cx, cy), outer, HOLE_RATIO, 0.0, 6, &[(0.93, 0.62, 0.30), (0.60, 0.36, 0.18), (0.98, 0.80, 0.50)]);
    });
    face
}

/// `Clipped` face: same square extent as `Fill` but the `ShapeClip`
/// cuts it to the band - the frosting stays inside the ring, corners
/// and hole show card surface beneath.
fn build_bezel(size: Pixels) -> DrawingArea {
    let face = DrawingArea::new();
    face.set_content_width(f64::from(size) as i32);
    face.set_content_height(f64::from(size) as i32);
    face.set_draw_func(move |_area, context, width, height| {
        let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        let outer = cx.min(cy);
        context.set_source_rgb(0.22, 0.32, 0.42);
        context.rectangle(0.0, 0.0, f64::from(width), f64::from(height));
        context.fill().unwrap_or_default();
        // Tick ring: the clip leaves only what lands on the band.
        context.set_source_rgba(0.90, 0.92, 0.95, 0.85);
        for index in 0..36 {
            let angle = index as f64 * 2.0 * PI / 36.0;
            let (inner_r, outer_r) = if index % 3 == 0 {
                (outer * HOLE_RATIO, outer)
            } else {
                (outer * 0.75, outer * 0.95)
            };
            context.move_to(cx + inner_r * angle.cos(), cy + inner_r * angle.sin());
            context.line_to(cx + outer_r * angle.cos(), cy + outer_r * angle.sin());
        }
        context.set_line_width(4.0);
        context.stroke().unwrap_or_default();
    });
    face
}

/// `Cover` face: gear teeth and spokes spanning the whole bounding
/// box - the card's silhouette clip leaves only the toothed ring.
fn paint_gear(context: &Context, center: (f64, f64), outer: f64, ratio: f64, phase: f64) {
    let (cx, cy) = center;
    let inner = outer * ratio;
    let teeth = 12;
    let pitch = 2.0 * PI / f64::from(teeth);
    // Teeth: trapezoid ring around the rim.
    context.set_source_rgb(0.62, 0.63, 0.66);
    context.new_path();
    for index in 0..teeth {
        let a = phase + f64::from(index) * pitch;
        for (t, r) in [(0.00, outer * 0.92), (0.15, outer), (0.45, outer), (0.60, outer * 0.92)] {
            let angle = a + t * pitch;
            context.line_to(cx + r * angle.cos(), cy + r * angle.sin());
        }
    }
    context.close_path();
    context.set_fill_rule(FillRule::EvenOdd);
    context.arc(cx, cy, inner, 0.0, 2.0 * PI);
    context.fill().unwrap_or_default();
    // Spokes crossing the band.
    context.set_source_rgb(0.45, 0.46, 0.50);
    context.set_line_width(outer * 0.09);
    for index in 0..4 {
        let angle = phase + index as f64 * PI / 2.0 + PI / 4.0;
        context.move_to(cx + inner * angle.cos(), cy + inner * angle.sin());
        context.line_to(cx + outer * 0.92 * angle.cos(), cy + outer * 0.92 * angle.sin());
    }
    context.stroke().unwrap_or_default();
    // Hub ring around the hole.
    context.set_source_rgb(0.30, 0.30, 0.33);
    context.arc(cx, cy, inner * 1.08, 0.0, 2.0 * PI);
    context.set_line_width(outer * 0.05);
    context.stroke().unwrap_or_default();
}

fn build_gear(size: Pixels, phase: Rc<Cell<f64>>) -> DrawingArea {
    let face = DrawingArea::new();
    face.set_content_width(f64::from(size) as i32);
    face.set_content_height(f64::from(size) as i32);
    face.set_draw_func(move |_area, context, width, height| {
        let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        paint_gear(context, (cx, cy), cx.min(cy), HOLE_RATIO, phase.get());
    });
    face
}

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
        &gtk4::gdk::Display::default().unwrap_or_else(|| panic!("display")),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let under_face = gtk4::DrawingArea::new();
    under_face.set_widget_name("under-content");
    under_face.set_content_width(1000);
    under_face.set_content_height(500);
    under_face.set_draw_func(|_area, context, width, height| {
        context.set_source_rgb(0.55, 0.25, 0.30);
        context.rectangle(0.0, 0.0, f64::from(width), f64::from(height));
        context.fill().unwrap_or_default();
    });
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
    gtk4::glib::timeout_add_local_once(Duration::from_millis(500), {
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
    gtk4::glib::timeout_add_local_once(Duration::from_millis(1200), {
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
    gtk4::glib::timeout_add_local_once(Duration::from_millis(400), {
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
    gtk4::glib::timeout_add_local_once(Duration::from_millis(1800), move || {
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
            let picked = probe_board.pick(bx, by, gtk4::PickFlags::DEFAULT);
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

    gtk4::glib::timeout_add_local_once(Duration::from_millis(2000), move || {
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
        texture.save_to_png("/tmp/annulus_probe.png").expect("png");
        eprintln!("wrote /tmp/annulus_probe.png {width}x{height}");
        std::process::exit(0);
    });
}
