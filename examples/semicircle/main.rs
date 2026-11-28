//! Semicircle showcase: `CardShape::Semicircle` is a dome - the top
//! half of a circle on a flat bottom edge. Its inline chrome splits
//! by placement: `Inline*`/`ArcTop*` slots ride the dome's rim as an
//! arc header (like a circle's top band), while `Bar*` slots
//! mount a straight bar along the flat edge - two header strips with
//! different geometry on one card.
//!
//! Card "gauge" uses the top arc only: `CardContent::Fill` gives the
//! speedometer face the whole dome box, and `[Minimize, "Gauge",
//! Close]` rides the rim - the classic header-on-top use case.
//!
//! Card "awning" uses the straight bar only: `Bar*` slots mount
//! `[Minimize, "Awning", Close]` as a linear strip on the flat edge -
//! reads like a footer bar for a user facing the dome. A palette
//! tints both the bar and the dome.
//!
//! Card "meter" uses both at once: the rotate steps pin to the arc
//! band's ends around the centered title, while `Minimize`/`Close`
//! sit on the straight bar's start/end - top and bottom header in a
//! single card.
//!
//! Card "protractor" runs `CardContent::Cover`: its degree marks ride
//! the very rim of the dome, bounded only by the silhouette clip. A
//! narrow top arc holds just the title; the straight bar packs
//! `Minimize`/`Close` at its ends while the rotate steps ride the
//! center zone - interactive chrome on the bar itself. Cover content
//! is display-only (non-targetable), so its swing arm reads clicks
//! through a card-level gesture mapped into the face's frame.
//!
//! Run with `cargo run --example semicircle` (optionally
//! `--features adwaita` for themed chrome).

mod gauge;
mod protractor;
mod sunrise;

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
use gauge::build_gauge;
use gtk4::Orientation;
use gtk4::graphene::Point;
use gtk4::prelude::*;
use protractor::build_protractor;
use smearor_commons_color::Color;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use sunrise::build_sunrise;
use tracing_subscriber::EnvFilter;

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
    let application = gtk4::Application::new(Some("io.smearor.floating-board.semicircle"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board semicircle"));
    window.set_default_size(1100, 800);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // Top-bar use case: the gauge's face fills the dome box
    // (`CardContent::Fill` - it draws its own dial), the arc header
    // rides the rim with the title centered like a header bar's.
    board
        .add_card_with_id(
            "gauge",
            &build_gauge(Pixels::new(280)),
            CardConfig {
                title: Some("Gauge".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 360.0))),
                shape: CardShape::Semicircle,
                content: CardContent::Fill,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(Pixels::new(100)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("gauge"));

    // Bottom-bar use case: no top chrome at all - `Bar*` slots
    // mount a straight strip on the dome's flat edge, centered like a
    // footer bar for a user facing the dome. The palette's header
    // color tints the bar.
    let mut palette = CardPalette::new();
    palette.insert(CardColor::Background, Color::<f64>::new(0.30, 0.26, 0.36));
    palette.insert(CardColor::Header, Color::<f64>::new(0.45, 0.38, 0.55));
    palette.insert(CardColor::Foreground, Color::<f64>::new(0.92, 0.90, 0.95));
    board
        .add_card_with_id(
            "awning",
            &build_sunrise(Pixels::new(150)),
            CardConfig {
                title: Some("Awning".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(640.0, 380.0))),
                shape: CardShape::Semicircle,
                palette: Some(palette),
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_sunrise(Pixels::new(56)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Crossfade,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("awning"));

    // Both bars at once: the rotate steps pin to the arc band's ends
    // around the centered title, while `Minimize`/`Close` sit on the
    // straight bar's start/end. A wider 120° arc leaves the rim's
    // shoulders bare.
    board
        .add_card_with_id(
            "meter",
            &build_gauge(Pixels::new(220)),
            CardConfig {
                title: Some("Meter".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(480.0, 620.0))),
                shape: CardShape::Semicircle,
                content: CardContent::Fill,
                arc_span: Some(Degrees::new(120.0)),
                chrome: Some(vec![
                    ChromeSlot::inline_start(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline_end(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(Pixels::new(90)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("meter"));

    // The protractor (`CardContent::Cover`): the degree face reaches
    // the silhouette, no ring stays chrome. A narrow top arc carries
    // only the title; on the straight bar `Minimize`/`Close` pack the
    // ends while the rotate steps ride the center - they step the
    // dome in snap turns, interactive chrome on the bar itself.
    let protractor_reading = Rc::new(Cell::new(PI / 3.0));
    let protractor_face = build_protractor(Pixels::new(320), protractor_reading.clone());
    board
        .add_card_with_id(
            "protractor",
            &protractor_face,
            CardConfig {
                title: Some("Protractor".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(880.0, 610.0))),
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

    // Interactive cover content: the face is non-targetable (GTK
    // offers no non-rectangular pick masks), so a card-level click
    // reads the release point mapped into the face's frame - inside
    // the dome the swing arm swings to the tapped angle, clicks in
    // the cut corners never reach it and fall through to cards
    // below.
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
}
