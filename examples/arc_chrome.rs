//! Arc-chrome showcase: `CardShape::Circle` mounts its inline chrome
//! along the disc's top quarter - an "arc header" that mirrors the
//! header bar's order: leading slots left of the centered title,
//! trailing slots right of it.
//!
//! Card "dial" runs the builtin style with `CircleContent::Fill`: the
//! clock face fills the inner square, so the band's buttons float over
//! the disc's rim. The card folds onto a small clock disc.
//!
//! Card "compass" runs the Adwaita style (with `--features adwaita`):
//! the same band drawn as raised circular theme buttons over the
//! inscribed content's chrome ring, while two seat-pass actions orbit
//! as regular satellites - the arc/split in one picture.
//!
//! Card "pie" carries a palette: its band paints in the palette's
//! header color instead of the default chrome tone.
//!
//! Run with `cargo run --example arc_chrome` (optionally
//! `--features adwaita` for themed chrome).

use floating_board::CardAnchor;
use floating_board::CardColor;
use floating_board::CardConfig;
use floating_board::CardFoldAnimation;
use floating_board::CardId;
use floating_board::CardPalette;
use floating_board::CardPosition;
use floating_board::CardShape;
use floating_board::ChromeElement;
use floating_board::ChromeSlot;
use floating_board::ChromeStyle;
use floating_board::CircleContent;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::f64::consts::TAU;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// A `DrawingArea` rendering an analog clock face, ticking once per
/// second. Two instances exist: the dial's content and its
/// `folded_content` (a widget has exactly one parent).
fn build_clock(size: i32) -> gtk4::DrawingArea {
    let clock = gtk4::DrawingArea::new();
    clock.set_content_width(size);
    clock.set_content_height(size);
    clock.set_draw_func(|_area, context, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        let (center_x, center_y) = (width / 2.0, height / 2.0);
        let radius = width.min(height) / 2.0 - 3.0;
        context.arc(center_x, center_y, radius, 0.0, TAU);
        context.set_source_rgba(0.95, 0.95, 0.97, 0.92);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.22, 0.27, 0.36);
        context.set_line_width(2.5);
        let _ = context.stroke();
        for tick in 0..12 {
            let angle = f64::from(tick) * TAU / 12.0;
            let inner = radius - if tick % 3 == 0 { 11.0 } else { 6.0 };
            context.move_to(center_x + inner * angle.sin(), center_y - inner * angle.cos());
            context.line_to(center_x + (radius - 3.0) * angle.sin(), center_y - (radius - 3.0) * angle.cos());
            let _ = context.stroke();
        }
        let Ok(now) = gtk4::glib::DateTime::now_local() else { return };
        let seconds = now.seconds();
        let hands = [
            (
                ((now.hour() % 12) as f64 + (now.minute() as f64 + seconds / 60.0) / 60.0) * TAU / 12.0,
                radius * 0.5,
                5.0,
                (0.22, 0.27, 0.36),
            ),
            ((now.minute() as f64 + seconds / 60.0) * TAU / 60.0, radius * 0.72, 3.5, (0.22, 0.27, 0.36)),
            (seconds * TAU / 60.0, radius * 0.82, 1.5, (0.87, 0.2, 0.25)),
        ];
        for (angle, length, line_width, (r, g, b)) in hands {
            context.set_source_rgb(r, g, b);
            context.set_line_width(line_width);
            context.set_line_cap(gtk4::cairo::LineCap::Round);
            context.move_to(center_x, center_y);
            context.line_to(center_x + length * angle.sin(), center_y - length * angle.cos());
            let _ = context.stroke();
        }
        context.arc(center_x, center_y, 3.5, 0.0, TAU);
        let _ = context.fill();
    });
    gtk4::glib::timeout_add_local(Duration::from_secs(1), {
        let clock = clock.clone();
        move || {
            clock.queue_draw();
            gtk4::glib::ControlFlow::Continue
        }
    });
    clock
}

/// A compass dial: tick ring, cardinal letters and a north needle.
/// Draws its own disc - mounted as `CircleContent::Inscribed`, the
/// surrounding ring stays card chrome and the arc band reads like a
/// header strip.
fn build_compass(size: i32) -> gtk4::DrawingArea {
    let compass = gtk4::DrawingArea::new();
    compass.set_content_width(size);
    compass.set_content_height(size);
    compass.set_draw_func(|_area, context, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        let (center_x, center_y) = (width / 2.0, height / 2.0);
        let radius = width.min(height) / 2.0 - 3.0;
        context.arc(center_x, center_y, radius, 0.0, TAU);
        context.set_source_rgba(0.16, 0.24, 0.34, 0.95);
        let _ = context.fill_preserve();
        context.set_source_rgb(0.85, 0.88, 0.92);
        context.set_line_width(2.5);
        let _ = context.stroke();
        for tick in 0..24 {
            let angle = f64::from(tick) * TAU / 24.0;
            let inner = radius - if tick % 6 == 0 { 14.0 } else { 7.0 };
            context.move_to(center_x + inner * angle.sin(), center_y - inner * angle.cos());
            context.line_to(center_x + (radius - 3.0) * angle.sin(), center_y - (radius - 3.0) * angle.cos());
            let _ = context.stroke();
        }
        // North needle: a long red triangle + its white tail.
        let needle = radius * 0.62;
        for (length, half_width, (r, g, b)) in [(needle, 7.0, (0.87, 0.2, 0.25)), (-needle * 0.5, 7.0, (0.85, 0.88, 0.92))] {
            context.move_to(center_x - half_width, center_y);
            context.line_to(center_x + length, center_y);
            context.line_to(center_x + half_width, center_y);
            context.close_path();
            context.set_source_rgb(r, g, b);
            let _ = context.fill();
        }
        context.arc(center_x, center_y, 4.0, 0.0, TAU);
        context.set_source_rgb(0.85, 0.88, 0.92);
        let _ = context.fill();
    });
    compass
}

/// A three-slice pie chart - fixed consumer colors, mounted as
/// `CircleContent::Inscribed` so the surrounding ring stays chrome.
fn build_pie(size: i32) -> gtk4::DrawingArea {
    let pie = gtk4::DrawingArea::new();
    pie.set_content_width(size);
    pie.set_content_height(size);
    pie.set_draw_func(|_area, context, width, height| {
        let (center_x, center_y) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        let radius = f64::from(width.min(height)) / 2.0 - 3.0;
        let slices = [(0.40, (0.32, 0.62, 0.84)), (0.35, (0.24, 0.72, 0.48)), (0.25, (0.92, 0.66, 0.20))];
        let mut angle = -std::f64::consts::FRAC_PI_2;
        for (share, (r, g, b)) in slices {
            context.move_to(center_x, center_y);
            context.arc(center_x, center_y, radius, angle, angle + share * TAU);
            context.close_path();
            context.set_source_rgb(r, g, b);
            let _ = context.fill();
            angle += share * TAU;
        }
    });
    pie
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    #[cfg(feature = "adwaita")]
    {
        if let Err(error) = floating_board::adw::init() {
            panic!("libadwaita init failed: {error}");
        }
        eprintln!("adwaita active - the compass arc renders as themed circular buttons");
    }
    #[cfg(not(feature = "adwaita"))]
    eprintln!("built without `adwaita` - rerun with `--features adwaita` to see the themed compass chrome");
    let application = gtk4::Application::new(Some("io.smearor.floating-board.arc-chrome"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board arc chrome"));
    window.set_default_size(1100, 800);

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    let filler = gtk4::DrawingArea::new();
    filler.set_vexpand(true);
    content.append(&filler);
    let board = FloatingBoard::new(&content);

    // The builtin arc header: leading slots sit left of the centered
    // title, trailing slots right of it - the header bar's order on a
    // disc.
    board
        .add_card_with_id(
            "dial",
            &build_clock(280),
            CardConfig {
                title: Some("Dial".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 400.0))),
                shape: CardShape::Circle,
                circle_content: CircleContent::Fill,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::RotateClockwise),
                    ChromeSlot::inline(ChromeElement::Close),
                ]),
                folded_content: Some(build_clock(90).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("dial"));

    // The Adwaita arc: raised circular theme buttons over the chrome
    // ring; the seat-pass actions stay `Satellite` placements and keep
    // orbiting below the card.
    board
        .add_card_with_id(
            "compass",
            &build_compass(200),
            CardConfig {
                title: Some("Compass".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(750.0, 380.0))),
                shape: CardShape::Circle,
                // A wider arc than the 90° default: the band wraps a
                // third of the disc's rim.
                arc_span: Some(Degrees::new(135.0)),
                chrome_style: ChromeStyle::Adwaita,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Close),
                    ChromeSlot::satellite(ChromeElement::PreviousSeat),
                    ChromeSlot::satellite(ChromeElement::NextSeat),
                ]),
                folded_content: Some(build_compass(80).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("compass"));

    // A small disc with a palette: the band takes the palette's
    // header color. The slot order mirrors a header bar - handle and
    // minimize lead, title centers, close packs the band's far end.
    let mut palette = CardPalette::new();
    palette.insert(CardColor::Background, Color::<f64>::new(0.30, 0.26, 0.36));
    palette.insert(CardColor::Header, Color::<f64>::new(0.45, 0.38, 0.55));
    palette.insert(CardColor::Foreground, Color::<f64>::new(0.92, 0.90, 0.95));
    board
        .add_card_with_id(
            "pie",
            &build_pie(150),
            CardConfig {
                title: Some("Pie".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(550.0, 600.0))),
                shape: CardShape::Circle,
                palette: Some(palette),
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::DragHandle),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Close),
                    ChromeSlot::inline(ChromeElement::RotateClockwise),
                ]),
                folded_content: Some(build_pie(60).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("pie"));

    window.set_child(Some(&board));
    window.present();
}
