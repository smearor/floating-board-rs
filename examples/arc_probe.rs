//! Render probe for the arc-chrome "gauge" card: a `CardContent::Cover`
//! circle with a bottom-only band and +/- satellites - snapshots the
//! board into a PNG once laid out and exits. Run with
//! `cargo run --example arc_probe` - writes /tmp/arc_probe.png.

use floating_board::CardAnchor;
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
use floating_board::FloatingBoard;
use floating_board::SatelliteConfig;
use floating_board::SatelliteLayout;
use gtk4::Button;
use gtk4::CssProvider;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::TAU;
use std::rc::Rc;
use std::time::Duration;

/// A gauge dial painted edge-to-edge for `CardContent::Cover`: the
/// level ring rides the very rim of the disc, so only the card's
/// silhouette clip bounds it - no ring stays card chrome. `level`
/// (0.0..=1.0) is shared state, so the satellites can repaint the
/// dial via `queue_draw`.
fn build_gauge(size: Pixels<i32>, level: Rc<Cell<f64>>) -> gtk4::DrawingArea {
    let gauge = gtk4::DrawingArea::new();
    gauge.set_widget_name("gauge-content");
    gauge.set_content_width(size.as_raw());
    gauge.set_content_height(size.as_raw());
    gauge.set_draw_func(move |_area, context, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        let (center_x, center_y) = (width / 2.0, height / 2.0);
        let radius = width.min(height) / 2.0;
        context.arc(center_x, center_y, radius, 0.0, TAU);
        context.set_source_rgb(0.12, 0.14, 0.18);
        let _ = context.fill();
        let track = radius - 9.0;
        context.arc(center_x, center_y, track, 0.0, TAU);
        context.set_source_rgba(0.42, 0.48, 0.60, 0.35);
        context.set_line_width(9.0);
        let _ = context.stroke();
        let value = level.get().clamp(0.0, 1.0);
        context.arc(center_x, center_y, track, -FRAC_PI_2, -FRAC_PI_2 + value * TAU);
        context.set_source_rgb(0.35, 0.78, 0.56);
        context.set_line_cap(gtk4::cairo::LineCap::Round);
        let _ = context.stroke();
        context.set_line_width(1.5);
        context.set_source_rgba(0.85, 0.88, 0.95, 0.55);
        for tick in 0..24 {
            let angle = f64::from(tick) * TAU / 24.0;
            let inner = track - if tick % 6 == 0 { 16.0 } else { 9.0 };
            context.move_to(center_x + inner * angle.sin(), center_y - inner * angle.cos());
            context.line_to(center_x + (track - 4.0) * angle.sin(), center_y - (track - 4.0) * angle.cos());
            let _ = context.stroke();
        }
        let angle = -FRAC_PI_2 + value * TAU;
        let (sin, cos) = angle.sin_cos();
        context.move_to(center_x, center_y);
        context.line_to(center_x + (radius - 30.0) * -sin, center_y + (radius - 30.0) * cos);
        context.set_source_rgb(0.92, 0.93, 0.96);
        context.set_line_width(4.0);
        let _ = context.stroke();
        context.arc(center_x, center_y, 5.0, 0.0, TAU);
        let _ = context.fill();
    });
    gauge
}

fn level_satellite(label: &str, class: &str, delta: f64, face: &gtk4::DrawingArea, level: &Rc<Cell<f64>>) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("gauge-btn");
    button.add_css_class(class);
    let face = face.downgrade();
    let level = level.clone();
    button.connect_clicked(move |_| {
        level.set((level.get() + delta).clamp(0.0, 1.0));
        if let Some(face) = face.upgrade() {
            face.queue_draw();
        }
    });
    button
}

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.arc-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board arc probe"));
    window.set_default_size(600, 500);

    let css = CssProvider::new();
    css.load_from_string(
        ".gauge-btn { min-width: 34px; min-height: 34px; padding: 0; border-radius: 9999px; border-width: 2px; border-style: solid; font-weight: bold; }         .gauge-plus { background: rgba(89, 199, 143, 0.55); border-color: rgba(89, 199, 143, 0.9); }         .gauge-minus { background: rgba(150, 160, 190, 0.55); border-color: rgba(150, 160, 190, 0.9); }",
    );
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    let gauge_level = Rc::new(Cell::new(0.45));
    let gauge_face = build_gauge(Pixels::new(170), gauge_level.clone());
    board
        .add_card_with_id(
            "gauge",
            &gauge_face,
            CardConfig {
                title: Some("Gauge".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 250.0))),
                shape: CardShape::Circle,
                content: CardContent::Cover,
                arc_bottom_span: Some(Degrees::new(80.0)),
                palette: Some(CardPalette::new()),
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(Pixels::new(70), gauge_level.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: floating_board::DragHandle::WholeCard,
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

    let probe_board = board.clone();
    gtk4::glib::timeout_add_local_once(Duration::from_millis(600), move || {
        for (name, bx, by) in [
            ("gauge top corner", 245.0, 165.0),
            ("gauge center", 300.0, 250.0),
            ("gauge bottom bar", 300.0, 325.0),
            ("gauge crescent", 245.0, 210.0),
        ] {
            let picked = probe_board.pick(bx, by, gtk4::PickFlags::DEFAULT);
            eprintln!(
                "pick {name} ({bx},{by}) -> {}",
                picked.map(|w| format!("{} '{}'", w.type_().name(), w.widget_name())).unwrap_or_default()
            );
        }
    });

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
        texture.save_to_png("/tmp/arc_probe.png").expect("png");
        eprintln!("wrote /tmp/arc_probe.png {width}x{height}");
        std::process::exit(0);
    });
}
