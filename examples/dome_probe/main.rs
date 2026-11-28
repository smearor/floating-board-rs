//! Render probe for the semicircle "protractor" card: a
//! `CardContent::Cover` dome with a narrow title-only top arc and a
//! straight bottom bar (Minimize/Close at the ends, rotate steps in
//! the center) - snapshots the board into a PNG once laid out and
//! exits. Run with `cargo run --example dome_probe` - writes
//! /tmp/dome_probe.png.

#[path = "../semicircle/protractor/mod.rs"]
mod protractor;

use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::CardContent;
use floating_board::CardFoldAnimation;
use floating_board::CardId;
use floating_board::CardPosition;
use floating_board::CardShape;
use floating_board::ChromeElement;
use floating_board::ChromeSlot;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use glib::timeout_add_local_once;
use gtk4::Orientation;
use gtk4::PickFlags;
use gtk4::graphene::Point;
use gtk4::prelude::*;
use protractor::build_protractor;
use smearor_commons_fx::gtk::snapshot_to_png;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

fn main() {
    let application = gtk4::Application::new(Some("io.smearor.floating-board.dome-probe"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board dome probe"));
    window.set_default_size(600, 500);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    let protractor_reading = Rc::new(Cell::new(PI / 3.0));
    let protractor_face = build_protractor(Pixels::new(320), protractor_reading.clone());
    board
        .add_card_with_id(
            "protractor",
            &protractor_face,
            CardConfig {
                title: Some("Protractor".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 270.0))),
                shape: CardShape::Semicircle,
                content: CardContent::Cover,
                arc_span: Some(Degrees::new(45.0)),
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::bar(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_protractor(Pixels::new(110), protractor_reading.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Crossfade,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("protractor"));

    if let Ok(card) = board.card(&CardId::new("protractor")) {
        let click = gtk4::GestureClick::new();
        click.set_button(0);
        let face = protractor_face.downgrade();
        let reading = protractor_reading.clone();
        let clicked = card.clone();
        click.connect_released(move |_, _, x, y| {
            let Some(face) = face.upgrade() else { return };
            let Some(point) = face.compute_point(&clicked, &Point::new(x as f32, y as f32)) else {
                return;
            };
            let bounds = Rect::<Pixels>::from_size(face.allocated_size().to_f64().with_unit::<Pixels>());
            let dome = Semicircle::inscribed_in(&bounds);
            let at = Vec2::<Pixels>::new(f64::from(point.x()), f64::from(point.y()));
            if dome.contains(&at) {
                // The reading sweeps CCW from the flat edge's right
                // end - the y-up mirror of the vector's direction.
                let delta = at - dome.center;
                reading.set((-delta.direction()).as_f64().clamp(0.0, PI));
                face.queue_draw();
            }
        });
        card.add_controller(click);
    }

    window.set_child(Some(&board));
    window.present();

    let probe_board = board.clone();
    timeout_add_local_once(Duration::from_millis(600), move || {
        for (name, bx, by) in [
            ("dome top-left corner", 150.0, 120.0),
            ("dome top-right corner", 450.0, 120.0),
            ("dome center", 300.0, 250.0),
            ("dome top arc", 300.0, 207.0),
            ("dome bottom bar", 300.0, 332.0),
        ] {
            let picked = probe_board.pick(bx, by, PickFlags::DEFAULT);
            eprintln!(
                "pick {name} ({bx},{by}) -> {}",
                picked.map(|w| format!("{} '{}'", w.type_().name(), w.widget_name())).unwrap_or_default()
            );
        }
    });

    timeout_add_local_once(Duration::from_millis(800), move || {
        snapshot_to_png(&board, "/tmp/dome_probe.png").expect("png dump");
        eprintln!("wrote /tmp/dome_probe.png {}x{}", board.width(), board.height());
        std::process::exit(0);
    });
}
