//! Render probe: circle + semicircle cards with `Cover` content -
//! verifies the card-level silhouette clip on every shape. A
//! square-painting content must be cut to the silhouette; bleeding
//! corners would indicate the clip fails.
use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::CardContent;
use floating_board::CardId;
use floating_board::CardPosition;
use floating_board::CardShape;
use floating_board::FloatingBoard;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// A square-painting content: if the card clip fails, the corners
/// bleed past the silhouette.
fn build_box(size: Pixels<i32>) -> gtk4::DrawingArea {
    let area = gtk4::DrawingArea::new();
    area.set_widget_name("box-content");
    area.set_content_width(size.as_raw());
    area.set_content_height(size.as_raw());
    area.set_draw_func(|_area, context, _width, _height| {
        context.set_source_rgb(0.80, 0.25, 0.20);
        let _ = context.paint();
    });
    area
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let application = gtk4::Application::new(Some("io.smearor.floating-board.clip-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_default_size(900, 600);
    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let board = FloatingBoard::new(&content);
    board
        .add_card_with_id(
            "disc",
            &build_box(Pixels::new(150)),
            CardConfig {
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(230.0, 300.0))),
                shape: CardShape::Circle,
                content: CardContent::Cover,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("disc"));
    board
        .add_card_with_id(
            "dome",
            &build_box(Pixels::new(150)),
            CardConfig {
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(620.0, 320.0))),
                shape: CardShape::Semicircle,
                content: CardContent::Cover,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("dome"));
    window.set_child(Some(&board));
    window.present();

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
        texture.save_to_png("/tmp/clip_probe.png").expect("png");
        eprintln!("wrote /tmp/clip_probe.png {width}x{height}");
        std::process::exit(0);
    });
}
