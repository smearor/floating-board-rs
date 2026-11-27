//! Render probe for the semicircle "protractor" card: a
//! `CardContent::Cover` dome with a narrow title-only top arc and a
//! straight bottom bar (Minimize/Close at the ends, rotate steps in
//! the center) - snapshots the board into a PNG once laid out and
//! exits. Run with `cargo run --example dome_probe` - writes
//! /tmp/dome_probe.png.

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
use gtk4::Orientation;
use gtk4::graphene::Point;
use gtk4::prelude::*;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

/// A `DrawingArea` rendering a protractor for `CardContent::Cover`:
/// the acrylic face and its degree ticks paint edge-to-edge - only
/// the card's silhouette clip bounds them, no ring stays chrome.
fn build_protractor(width: Pixels<i32>, reading: Rc<Cell<f64>>) -> gtk4::DrawingArea {
    let protractor = gtk4::DrawingArea::new();
    protractor.set_widget_name("protractor-content");
    protractor.set_content_width(width.as_raw());
    protractor.set_content_height(width.as_raw() / 2);
    protractor.set_draw_func(move |_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let radius = Pixels::new((size.x / 2.0).min(size.y));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y);
        context.arc(center.x, center.y, radius.as_f64(), PI, 0.0);
        context.close_path();
        context.set_source_rgba(0.95, 0.86, 0.55, 0.95);
        let _ = context.fill_preserve();
        context.set_source_rgba(0.55, 0.42, 0.18, 0.9);
        context.set_line_width(2.0);
        let _ = context.stroke();
        context.set_line_width(2.0);
        for tick in 0..=18 {
            let (sin, cos) = RotationRad::new(PI - f64::from(tick) * PI / 18.0).sin_cos();
            let inner = radius - Pixels::new(if tick % 3 == 0 { 18.0 } else { 9.0 });
            let direction = Vec2::<Pixels>::new(cos, -sin);
            let from = center + direction * inner.as_f64();
            let to = center + direction * (radius - Pixels::new(3.0)).as_f64();
            context.move_to(from.x, from.y);
            context.line_to(to.x, to.y);
            let _ = context.stroke();
        }
        context.arc(center.x, center.y, (radius * 0.62).as_f64(), PI, 0.0);
        let _ = context.stroke();
        context.arc(center.x, center.y, (radius * 0.30).as_f64(), PI, 0.0);
        context.close_path();
        context.set_source_rgba(0.24, 0.20, 0.28, 0.92);
        let _ = context.fill();
        context.arc(center.x, center.y - Pixels::new(4.0).as_f64(), 3.0, 0.0, 2.0 * PI);
        context.set_source_rgba(0.55, 0.42, 0.18, 0.9);
        let _ = context.fill();
        let pivot = center - Vec2::<Pixels>::new(0.0, 4.0);
        let (sin, cos) = RotationRad::new(reading.get()).sin_cos();
        let direction = Vec2::<Pixels>::new(cos, -sin);
        let perpendicular = Vec2::<Pixels>::new(sin, cos);
        let tip = pivot + direction * (radius * 0.88).as_f64();
        let side_a = pivot + perpendicular * 8.0;
        let side_b = pivot - perpendicular * 8.0;
        context.move_to(tip.x, tip.y);
        context.line_to(side_a.x, side_a.y);
        context.line_to(side_b.x, side_b.y);
        context.close_path();
        context.set_source_rgba(0.76, 0.26, 0.20, 0.92);
        let _ = context.fill();
        context.arc(pivot.x, pivot.y, 5.0, 0.0, 2.0 * PI);
        let _ = context.fill();
    });
    protractor
}

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
            let (width, height) = (f64::from(face.width()), f64::from(face.height()));
            let radius = (width / 2.0).min(height);
            let (dx, dy) = (f64::from(point.x()) - width / 2.0, height - f64::from(point.y()));
            if dy >= 0.0 && dx.hypot(dy) <= radius {
                reading.set(dy.atan2(dx).clamp(0.0, PI));
                face.queue_draw();
            }
        });
        card.add_controller(click);
    }

    window.set_child(Some(&board));
    window.present();

    let probe_board = board.clone();
    gtk4::glib::timeout_add_local_once(Duration::from_millis(600), move || {
        for (name, bx, by) in [
            ("dome top-left corner", 150.0, 120.0),
            ("dome top-right corner", 450.0, 120.0),
            ("dome center", 300.0, 250.0),
            ("dome top arc", 300.0, 207.0),
            ("dome bottom bar", 300.0, 332.0),
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
        texture.save_to_png("/tmp/dome_probe.png").expect("png");
        eprintln!("wrote /tmp/dome_probe.png {width}x{height}");
        std::process::exit(0);
    });
}
