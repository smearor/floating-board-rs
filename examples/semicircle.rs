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
use gtk4::Orientation;
use gtk4::graphene::Point;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::PI;
use std::rc::Rc;
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
/// holds a fixed reading at mid-scale.
fn build_gauge(width: Pixels<i32>) -> gtk4::DrawingArea {
    let gauge = gtk4::DrawingArea::new();
    gauge.set_content_width(width.as_raw());
    gauge.set_content_height(width.as_raw() / 2);
    gauge.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let radius = Pixels::new((size.x / 2.0).min(size.y) - 4.0);
        // The dial's virtual circle centers on the box's bottom middle.
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y - 4.0);
        // The dome face: a half-disc arc from the left end over the
        // top to the right end, closed along the flat edge.
        context.arc(center.x, center.y, radius.as_f64(), PI, 0.0);
        context.close_path();
        dome_color(DomeColor::Face).apply_alpha(context, 0.95);
        let _ = context.fill_preserve();
        dome_color(DomeColor::Line).apply(context);
        context.set_line_width(2.5);
        let _ = context.stroke();
        // Tick marks fan across the rim: 13 ticks, long ones at the
        // quarter marks.
        let rim = radius - Pixels::new(4.0);
        for tick in 0..=12 {
            let (sin, cos) = RotationRad::new(PI - f64::from(tick) * PI / 12.0).sin_cos();
            let inner = radius - Pixels::new(if tick % 3 == 0 { 16.0 } else { 9.0 });
            let direction = Vec2::<Pixels>::new(cos, -sin);
            let from = center + direction * inner.as_f64();
            let to = center + direction * rim.as_f64();
            context.move_to(from.x, from.y);
            context.line_to(to.x, to.y);
            let _ = context.stroke();
        }
        // The needle: a tapered pointer at a fixed mid-scale reading.
        // No tail - the pivot sits on the flat edge, a tail would dip
        // below the widget's bottom and clip.
        let (sin, cos) = RotationRad::new(PI - 2.0 * PI / 3.0).sin_cos();
        let direction = Vec2::<Pixels>::new(cos, -sin);
        let perpendicular = Vec2::<Pixels>::new(sin, cos);
        let tip = center + direction * (radius * 0.72).as_f64();
        let side_a = center + perpendicular * 6.0;
        let side_b = center - perpendicular * 6.0;
        context.move_to(tip.x, tip.y);
        context.line_to(side_a.x, side_a.y);
        context.line_to(side_b.x, side_b.y);
        context.close_path();
        dome_color(DomeColor::Needle).apply(context);
        let _ = context.fill();
        context.arc(center.x, center.y, 3.0, 0.0, 2.0 * PI);
        dome_color(DomeColor::Line).apply(context);
        let _ = context.fill();
    });
    gauge
}

/// A `DrawingArea` rendering a sunrise for the awning card - a
/// bottom-aligned square (`CardContent::Inscribed` on a dome): a
/// sun rising over a horizon line with three rays above.
fn build_sunrise(size: Pixels<i32>) -> gtk4::DrawingArea {
    let sunrise = gtk4::DrawingArea::new();
    sunrise.set_content_width(size.as_raw());
    sunrise.set_content_height(size.as_raw());
    sunrise.set_draw_func(|_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        // The horizon sits on the dome's flat edge: the sun rises
        // over the card's bottom line.
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y - 4.0);
        let radius = Pixels::new(size.x.min(size.y) * 0.26);
        // The sun: a half-disc peeking over the horizon.
        context.arc(center.x, center.y, radius.as_f64(), PI, 0.0);
        context.close_path();
        context.set_source_rgba(0.98, 0.78, 0.30, 0.95);
        let _ = context.fill();
        // Three rays fanning above the sun.
        context.set_source_rgba(0.98, 0.78, 0.30, 0.85);
        context.set_line_width(4.0);
        context.set_line_cap(gtk4::cairo::LineCap::Round);
        for ray in [-0.6, 0.0, 0.6] {
            let (sin, cos) = RotationRad::new(-FRAC_PI_2 + ray).sin_cos();
            let direction = Vec2::<Pixels>::new(cos, sin);
            let from = center + direction * (radius + Pixels::new(8.0)).as_f64();
            let to = center + direction * (radius + Pixels::new(22.0)).as_f64();
            context.move_to(from.x, from.y);
            context.line_to(to.x, to.y);
            let _ = context.stroke();
        }
        // The horizon line.
        context.set_source_rgba(0.45, 0.38, 0.55, 0.9);
        context.set_line_width(3.0);
        context.move_to(size.x * 0.08, center.y);
        context.line_to(size.x * 0.92, center.y);
        let _ = context.stroke();
    });
    sunrise
}

/// A `DrawingArea` rendering a protractor for `CardContent::Cover`:
/// the acrylic face and its degree ticks paint edge-to-edge - only
/// the card's silhouette clip bounds them, no ring stays chrome.
/// `reading` (radians from the flat edge's right end, 0..PI) is
/// shared state: the swing arm follows it, so a card-level click
/// gesture can repaint via `queue_draw`.
fn build_protractor(width: Pixels<i32>, reading: Rc<Cell<f64>>) -> gtk4::DrawingArea {
    let protractor = gtk4::DrawingArea::new();
    protractor.set_content_width(width.as_raw());
    protractor.set_content_height(width.as_raw() / 2);
    protractor.set_draw_func(move |_area, context, width, height| {
        let size = Vec2::<Pixels>::new(f64::from(width), f64::from(height));
        let radius = Pixels::new((size.x / 2.0).min(size.y));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y);
        // The acrylic face: a half-disc edge-to-edge.
        context.arc(center.x, center.y, radius.as_f64(), PI, 0.0);
        context.close_path();
        context.set_source_rgba(0.95, 0.86, 0.55, 0.95);
        let _ = context.fill_preserve();
        context.set_source_rgba(0.55, 0.42, 0.18, 0.9);
        context.set_line_width(2.0);
        let _ = context.stroke();
        // Degree ticks on the rim: one per 10 deg, long at 30 deg.
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
        // The inner scale arc and the classic half-disc cutout on the
        // flat edge give it the protractor look.
        context.arc(center.x, center.y, (radius * 0.62).as_f64(), PI, 0.0);
        let _ = context.stroke();
        context.arc(center.x, center.y, (radius * 0.30).as_f64(), PI, 0.0);
        context.close_path();
        context.set_source_rgba(0.24, 0.20, 0.28, 0.92);
        let _ = context.fill();
        // The origin mark on the flat edge.
        context.arc(center.x, center.y - Pixels::new(4.0).as_f64(), 3.0, 0.0, 2.0 * PI);
        context.set_source_rgba(0.55, 0.42, 0.18, 0.9);
        let _ = context.fill();
        // The swing arm: a tapered needle reading `reading` degrees
        // over the flat edge, pivoting on the origin mark.
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
}
