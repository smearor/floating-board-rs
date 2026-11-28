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
use glib::timeout_add_local_once;
use gtk4::prelude::*;
use gtk4::{DrawingArea, Orientation};
use smearor_commons_color::Color;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// A square-painting content: if the card clip fails, the corners
/// bleed past the silhouette.
fn build_box(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let _ = target.fill_rect(FillRect::new(bounds, Color::<f64>::new(0.80, 0.25, 0.20), 1.0));
    });
    area.set_widget_name("box-content");
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
    timeout_add_local_once(Duration::from_millis(800), move || {
        smearor_commons_fx::gtk::snapshot_to_png(&board, "/tmp/clip_probe.png").expect("png dump");
        eprintln!("wrote /tmp/clip_probe.png {}x{}", board.width(), board.height());
        std::process::exit(0);
    });
}
