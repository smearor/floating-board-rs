//! Headless pick/contains probe: builds a board with one card, then
//! asks GTK what `pick` and `contains` report at the card's center -
//! no manual clicking needed. Run with `cargo run --example pick_probe`.

use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::CardId;
use floating_board::CardPosition;
use floating_board::CardShape;
use floating_board::FloatingBoard;
use gtk4::prelude::*;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use std::time::Duration;

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.pick-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("pick probe"));
    window.set_default_size(1280, 800);

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);

    let board = FloatingBoard::new(&content);
    let button = gtk4::Button::with_label("Probe button");
    button.connect_clicked(|_| println!("BUTTON CLICKED"));
    board
        .add_card_with_id(
            "probe",
            &button,
            CardConfig {
                title: Some("Probe".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(200.0, 300.0))),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("probe"));
    let rotated_button = gtk4::Button::with_label("Rotated");
    board
        .add_card_with_id(
            "rotated",
            &rotated_button,
            CardConfig {
                title: Some("Rotated".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(600.0, 300.0))),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("rotated"));
    let circle_button = gtk4::Button::with_label("Circle");
    board
        .add_card_with_id(
            "probe-circle",
            &circle_button,
            CardConfig {
                title: Some("Circle".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(210.0, 345.0))),
                shape: CardShape::Circle,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("probe-circle"));

    window.set_child(Some(&board));
    window.present();

    let board_rot = board.clone();
    gtk4::glib::timeout_add_local_once(Duration::from_millis(300), move || {
        for card in board_rot.cards() {
            if card.id() == CardId::new("rotated") {
                card.set_scale(floating_board::ScaleFactor::new(1.488));
                card.raise();
                card.set_rotation(smearor_commons_geometry::Rotation::Free(smearor_commons_geometry::RotationRad::new(10.995574287564276)));
            }
        }
    });
    let window_ref = window.clone();
    gtk4::glib::timeout_add_local_once(Duration::from_millis(1200), move || {
        let board_widget = window_ref.child().expect("board");
        for card in board.cards() {
            let geometry = card.geometry();
            println!(
                "card {:?} rotation {:?} measure={:?} alloc=({},{})",
                geometry.position,
                card.rotation(),
                card.measure_size(),
                card.width(),
                card.height()
            );
            let center_board = geometry.position.0;
            // card widget -> rotation -> scaling -> layer -> board
            let scaling = card.parent().and_then(|w| w.parent()).expect("scaling widget");
            let Some(center_scaling) = board_widget.compute_point(&scaling, &gtk4::graphene::Point::new(center_board.x as f32, center_board.y as f32)) else {
                println!("compute_point board->scaling failed");
                continue;
            };
            println!(
                "card {:?}: contains({:.1},{:.1}) = {}",
                geometry.position,
                center_scaling.x(),
                center_scaling.y(),
                scaling.contains(f64::from(center_scaling.x()), f64::from(center_scaling.y()))
            );
            let Some(center_window) = board_widget.compute_point(&window_ref, &gtk4::graphene::Point::new(center_board.x as f32, center_board.y as f32)) else {
                println!("compute_point board->window failed");
                continue;
            };
            let picked2 = window_ref.pick(
                f64::from(center_window.x()),
                f64::from(center_window.y()),
                gtk4::PickFlags::DEFAULT | gtk4::PickFlags::INSENSITIVE,
            );
            println!(
                "picked at window coords ({:.1},{:.1}): {:?}",
                center_window.x(),
                center_window.y(),
                picked2.as_ref().map(|w| w.type_().name())
            );
            if let Some(w) = picked2 {
                let mut chain = Vec::new();
                let mut cur = Some(w);
                while let Some(w) = cur {
                    chain.push(w.type_().name().to_string());
                    cur = w.parent();
                }
                println!("  ancestry: {}", chain.join(" <- "));
            }
            // scan a grid over the card's AABB and record pick targets + contains
            use std::collections::HashMap;
            let mut kinds: HashMap<String, u32> = HashMap::new();
            let mut contains_count = 0u32;
            let mut contains_checked = 0u32;
            for dx in -80..=80 {
                for dy in -60..=60 {
                    let px = f64::from(center_window.x()) + f64::from(dx) * 2.0;
                    let py = f64::from(center_window.y()) + f64::from(dy) * 2.0;
                    if let Some(p) = board_widget.compute_point(&scaling, &gtk4::graphene::Point::new(px as f32, py as f32)) {
                        contains_checked += 1;
                        if scaling.contains(f64::from(p.x()), f64::from(p.y())) {
                            contains_count += 1;
                        }
                    }
                    if let Some(w) = window_ref.pick(px, py, gtk4::PickFlags::DEFAULT | gtk4::PickFlags::INSENSITIVE) {
                        *kinds.entry(w.type_().name().to_string()).or_default() += 1;
                    }
                }
            }
            println!("  contains true {contains_count}/{contains_checked}");
            if card.id() == CardId::new("probe") {
                // Fixed board-space points inside the "probe" card's
                // visible rect - some are also inside the circle card's
                // bounding-box corner that overlaps it.
                for off in [65.0_f64, 55.0, 45.0, 35.0] {
                    let board_p = gtk4::graphene::Point::new(210.0 - off as f32, 345.0 - off as f32);
                    let p = board_widget.compute_point(&window_ref, &board_p).unwrap_or_default();
                    if let Some(w) = window_ref.pick(f64::from(p.x()), f64::from(p.y()), gtk4::PickFlags::DEFAULT | gtk4::PickFlags::INSENSITIVE) {
                        let mut chain = Vec::new();
                        let mut cur = Some(w);
                        while let Some(w) = cur {
                            chain.push(w.type_().name().to_string());
                            cur = w.parent();
                        }
                        println!("  fixed pick off {}: {}", off, chain.join(" <- "));
                    }
                }
            }
            let mut sorted: Vec<_> = kinds.into_iter().collect();
            sorted.sort_by_key(|(_, c)| u32::MAX - c);
            for (name, count) in sorted.iter().take(8) {
                println!("  pick {name}: {count}");
            }
        }
        window_ref.close();
    });
}
