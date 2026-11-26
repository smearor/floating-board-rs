//! Semicircle showcase: `CardShape::Semicircle` is a dome - the top
//! half of a circle on a flat bottom edge. Its inline chrome splits
//! by placement: `Inline*`/`ArcTop*` slots ride the dome's rim as an
//! arc header (like a circle's top band), while `Bar*` slots
//! mount a straight bar along the flat edge - two header strips with
//! different geometry on one card.
//!
//! Card "gauge" uses the top arc only: `CircleContent::Fill` gives the
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
//! Run with `cargo run --example semicircle` (optionally
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
use floating_board::CircleContent;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::f64::consts::PI;
use tracing_subscriber::EnvFilter;

// The dome faces' named colors - shared by the gauge dial and the
// awning's sunrise.
palette_definition! {
    /// The dome face's named colors.
    pub enum DomeColor {
        /// The dark dial face, painted at 95% opacity.
        Face => "face",
        /// The bright markings: rim, ticks, hub.
        Line => "line",
        /// The gauge needle.
        Needle => "needle"
    }
}

/// The sRGB values of the [`DomeColor`] slots.
static DOME_PALETTE: StaticPalette<DomeColor, f64> = static_palette! {
    DomeColor::Face => Color::<f64>::new(0.16, 0.22, 0.32),
    DomeColor::Line => Color::<f64>::new(0.85, 0.88, 0.92),
    DomeColor::Needle => Color::<f64>::new(0.87, 0.35, 0.28),
};

/// The color registered under `slot` in the dome palette - black on a
/// miss, unreachable while the definition stays complete.
fn dome_color(slot: DomeColor) -> Color<f64> {
    DOME_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

/// A `DrawingArea` rendering a speedometer face sized to the dome's
/// bounding box (`2r x r`): the dial's virtual circle centers on the
/// box's bottom middle, ticks fan across the rim, and the needle
/// holds a fixed reading. `angle` is the needle's rotation clockwise
/// off the dial's left end (-90°) - a static 40° reads mid-scale.
fn build_gauge(width: i32) -> gtk4::DrawingArea {
    let gauge = gtk4::DrawingArea::new();
    gauge.set_content_width(width);
    gauge.set_content_height(width / 2);
    gauge.set_draw_func(|_area, context, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        let radius = (width / 2.0).min(height) - 4.0;
        let (center_x, center_y) = (width / 2.0, height - 4.0);
        // The dome face: a half-disc arc from the left end over the
        // top to the right end, closed along the flat edge.
        context.arc(center_x, center_y, radius, PI, 0.0);
        context.close_path();
        dome_color(DomeColor::Face).apply_alpha(context, 0.95);
        let _ = context.fill_preserve();
        dome_color(DomeColor::Line).apply(context);
        context.set_line_width(2.5);
        let _ = context.stroke();
        // Tick marks fan across the rim: 13 ticks, long ones at the
        // quarter marks.
        for tick in 0..=12 {
            let angle = PI - f64::from(tick) * PI / 12.0;
            let inner = radius - if tick % 3 == 0 { 16.0 } else { 9.0 };
            context.move_to(center_x + inner * angle.cos(), center_y - inner * angle.sin());
            context.line_to(center_x + (radius - 4.0) * angle.cos(), center_y - (radius - 4.0) * angle.sin());
            let _ = context.stroke();
        }
        // The needle: a tapered pointer at a fixed mid-scale reading.
        let needle_angle = PI - (2.0 * PI / 3.0);
        let tail_angle = needle_angle + PI;
        context.move_to(center_x + radius * 0.72 * needle_angle.cos(), center_y - radius * 0.72 * needle_angle.sin());
        context.line_to(
            center_x + 12.0 * (needle_angle - PI / 2.0).cos() * 0.5,
            center_y - 12.0 * (needle_angle - PI / 2.0).sin() * 0.5,
        );
        context.line_to(center_x + radius * 0.12 * tail_angle.cos(), center_y - radius * 0.12 * tail_angle.sin());
        context.line_to(
            center_x + 12.0 * (needle_angle + PI / 2.0).cos() * 0.5,
            center_y - 12.0 * (needle_angle + PI / 2.0).sin() * 0.5,
        );
        context.close_path();
        dome_color(DomeColor::Needle).apply(context);
        let _ = context.fill();
        context.arc(center_x, center_y, 5.0, 0.0, 2.0 * PI);
        dome_color(DomeColor::Line).apply(context);
        let _ = context.fill();
    });
    gauge
}

/// A `DrawingArea` rendering a sunrise for the awning card - a
/// bottom-aligned square (`CircleContent::Inscribed` on a dome): a
/// sun rising over a horizon line with three rays above.
fn build_sunrise(size: i32) -> gtk4::DrawingArea {
    let sunrise = gtk4::DrawingArea::new();
    sunrise.set_content_width(size);
    sunrise.set_content_height(size);
    sunrise.set_draw_func(|_area, context, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        let (center_x, horizon_y) = (width / 2.0, height * 0.78);
        let radius = width.min(height) * 0.26;
        // The sun: a half-disc peeking over the horizon.
        context.arc(center_x, horizon_y, radius, PI, 0.0);
        context.close_path();
        context.set_source_rgba(0.98, 0.78, 0.30, 0.95);
        let _ = context.fill();
        // Three rays fanning above the sun.
        context.set_source_rgba(0.98, 0.78, 0.30, 0.85);
        context.set_line_width(4.0);
        context.set_line_cap(gtk4::cairo::LineCap::Round);
        for ray in [-0.6, 0.0, 0.6] {
            let angle = -PI / 2.0 + ray;
            context.move_to(center_x + (radius + 8.0) * angle.cos(), horizon_y + (radius + 8.0) * angle.sin());
            context.line_to(center_x + (radius + 22.0) * angle.cos(), horizon_y + (radius + 22.0) * angle.sin());
            let _ = context.stroke();
        }
        // The horizon line.
        context.set_source_rgba(0.45, 0.38, 0.55, 0.9);
        context.set_line_width(3.0);
        context.move_to(width * 0.08, horizon_y);
        context.line_to(width * 0.92, horizon_y);
        let _ = context.stroke();
    });
    sunrise
}

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
    // (`CircleContent::Fill` - it draws its own dial), the arc header
    // rides the rim with the title centered like a header bar's.
    board
        .add_card_with_id(
            "gauge",
            &build_gauge(280),
            CardConfig {
                title: Some("Gauge".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 360.0))),
                shape: CardShape::Semicircle,
                circle_content: CircleContent::Fill,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(100).upcast()),
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
            &build_sunrise(150),
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
                folded_content: Some(build_sunrise(56).upcast()),
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
            &build_gauge(220),
            CardConfig {
                title: Some("Meter".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(480.0, 620.0))),
                shape: CardShape::Semicircle,
                circle_content: CircleContent::Fill,
                arc_span: Some(Degrees::new(120.0)),
                chrome: Some(vec![
                    ChromeSlot::inline_start(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline_end(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(90).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("meter"));

    window.set_child(Some(&board));
    window.present();
}
