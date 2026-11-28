//! Render probe for the arc-chrome "gauge" card: a `CardContent::Cover`
//! circle with a bottom-only band and +/- satellites - snapshots the
//! board into a PNG once laid out and exits. Run with
//! `cargo run --example arc_probe` - writes /tmp/arc_probe.png.

#[path = "../arc_chrome/gauge/mod.rs"]
mod gauge;

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
use gauge::{build_gauge, level_satellite};
use glib::timeout_add_local_once;
use gtk4::CssProvider;
use gtk4::DrawingArea;
use gtk4::Orientation;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

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
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    let gauge_level = Rc::new(Cell::new(Fraction::new(0.45)));
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
    timeout_add_local_once(Duration::from_millis(600), move || {
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

    timeout_add_local_once(Duration::from_millis(800), move || {
        smearor_commons_fx::gtk::snapshot_to_png(&board, "/tmp/arc_probe.png").expect("png dump");
        eprintln!("wrote /tmp/arc_probe.png {}x{}", board.width(), board.height());
        std::process::exit(0);
    });
}
