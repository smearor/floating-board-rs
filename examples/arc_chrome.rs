//! Arc-chrome showcase: `CardShape::Circle` mounts its inline chrome
//! along the disc's top quarter - an "arc header" that mirrors the
//! header bar's order: leading slots left of the centered title,
//! trailing slots right of it.
//!
//! Card "dial" runs the builtin style with `CardContent::Fill`: the
//! clock face fills the inner square, so the band's buttons float over
//! the disc's rim. The card folds onto a small clock disc.
//!
//! Card "compass" runs the Adwaita style (with `--features adwaita`)
//! on a wider 135° span: the rotate steps pin to the band's ends via
//! `InlineStart`/`InlineEnd` while the title rides the center zone -
//! the band drawn as raised circular theme buttons. A narrow 30°
//! bottom band mirrors it and hosts `Minimize`/`Close` on its
//! start/end slots. Its dial carries two needles: a green one fixed
//! to the card's "up" and a red one that keeps pointing to the
//! board's top edge no matter how the card is rotated.
//!
//! Card "pie" carries a palette and a second band: its bottom arc
//! mirrors the top one and hosts the seat-pass actions for a user
//! facing the card's bottom edge. One satellite dot per slice color
//! orbits the disc - a click grows the matching slice by 5% (red's
//! first click seeds a 10% slice); no slice drops below 10%.
//!
//! Card "gauge" runs `CardContent::Cover`: its dial paints
//! edge-to-edge, bounded only by the card's silhouette clip. A single
//! bottom band hosts `Minimize`/`Title`/`Close` - there is no top
//! arc. The cover content is display-only (non-targetable), so the
//! interaction rides the `+`/`-` satellites trimming the shared level.
//!
//! Run with `cargo run --example arc_chrome` (optionally
//! `--features adwaita` for themed chrome).

use floating_board::CardAnchor;
use floating_board::CardColor;
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
use floating_board::ChromeStyle;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::SatelliteConfig;
use floating_board::SatelliteLayout;
use gtk4::{Button, DrawingArea};
use gtk4::CssProvider;
use gtk4::Orientation;
use gtk4::glib::closure_local;
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
use std::cell::RefCell;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::TAU;
use std::rc::Rc;
use std::time::Duration;
use gtk4::cairo::{Context, LineCap};
use tracing_subscriber::EnvFilter;

/// A `DrawingArea` rendering an analog clock face, ticking once per
/// second. Two instances exist: the dial's content and its
/// `folded_content` (a widget has exactly one parent).
fn build_clock(size: Pixels<i32>) -> gtk4::DrawingArea {
    let clock = gtk4::DrawingArea::new();
    clock.set_content_width(size.as_raw());
    clock.set_content_height(size.as_raw());
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

// A compass dial: tick ring, cardinal letters and a north needle.
// Draws its own disc - mounted as `CardContent::Inscribed`, the
// surrounding ring stays card chrome and the arc band reads like a
// header strip.
palette_definition! {
    /// The compass dial's named colors.
    pub enum CompassColor {
        /// The dial's dark face, painted at 95% opacity.
        Face => "face",
        /// The bright lines on the dial: ring, ticks, hub and needle tails.
        Line => "line",
        /// The card-fixed needle - always points to the card's own top edge.
        CardNeedle => "card-needle",
        /// The world-fixed needle - always points to the board's top edge.
        WorldNeedle => "world-needle"
    }
}

/// The sRGB values of the [`CompassColor`] slots.
static COMPASS_PALETTE: StaticPalette<CompassColor, f64> = static_palette! {
    CompassColor::Face => Color::<f64>::new(0.16, 0.24, 0.34),
    CompassColor::Line => Color::<f64>::new(0.85, 0.88, 0.92),
    CompassColor::CardNeedle => Color::<f64>::new(0.18, 0.72, 0.36),
    CompassColor::WorldNeedle => Color::<f64>::new(0.87, 0.2, 0.25),
};

/// The color registered under `slot` in the compass palette - black
/// on a miss, unreachable while the definition stays complete.
fn compass_color(slot: CompassColor) -> Color<f64> {
    COMPASS_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

palette_definition! {
    /// The pie chart's named colors - each is both a slice fill and
    /// the matching satellite dot's color.
    pub enum PieColor {
        /// The blue slice and satellite.
        Blue => "blue",
        /// The green slice and satellite.
        Green => "green",
        /// The yellow slice and satellite.
        Yellow => "yellow",
        /// The red slice and satellite - the slice starts absent.
        Red => "red"
    }
}

/// The sRGB values of the [`PieColor`] slots.
static PIE_PALETTE: StaticPalette<PieColor, f64> = static_palette! {
    PieColor::Blue => Color::<f64>::new(0.32, 0.62, 0.84),
    PieColor::Green => Color::<f64>::new(0.24, 0.72, 0.48),
    PieColor::Yellow => Color::<f64>::new(0.92, 0.66, 0.20),
    PieColor::Red => Color::<f64>::new(0.85, 0.25, 0.22),
};

/// The color registered under `slot` in the pie palette - black on a
/// miss, unreachable while the definition stays complete.
fn pie_color(slot: PieColor) -> Color<f64> {
    PIE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

/// No slice may drop below this share.
const SLICE_FLOOR: f64 = 0.10;
/// The share a slice satellite adds per click.
const SLICE_STEP: f64 = 0.05;

/// The pie's slice shares, summing to 1 - `Red` starts at 0 (no
/// slice) until its satellite's first click seeds it. Shared between
/// the face's draw func and the satellite buttons.
#[derive(Clone)]
struct PieShares {
    /// `(slot, share)` pairs in draw order - a zero share is absent.
    shares: [(PieColor, f64); 4],
}

impl PieShares {
    /// The starting pie: blue 40%, green 35%, yellow 25%, no red.
    const INITIAL: Self = Self {
        shares: [(PieColor::Blue, 0.40), (PieColor::Green, 0.35), (PieColor::Yellow, 0.25), (PieColor::Red, 0.0)],
    };

    /// `slot`'s current share (`0` when absent).
    fn share(&self, slot: PieColor) -> f64 {
        self.shares.iter().find(|(s, _)| *s == slot).map(|(_, share)| *share).unwrap_or_default()
    }

    /// Grows `slot` by `amount`, taking the room proportionally from
    /// the other slices' headroom above `SLICE_FLOOR`: donors lose in
    /// proportion to what they can still give, so no slice drops below
    /// the floor and growth caps at the total headroom. A zero-share
    /// slot only seeds when the full `amount` fits - a partial seed
    /// would undercut the floor.
    fn grow(&mut self, slot: PieColor, amount: f64) {
        let headroom: f64 = self
            .shares
            .iter()
            .filter(|(s, _)| *s != slot)
            .map(|(_, share)| (share - SLICE_FLOOR).max(0.0))
            .sum();
        if headroom <= 0.0 || (self.share(slot) <= 0.0 && headroom < amount) {
            return;
        }
        let taken = headroom.min(amount);
        for (s, share) in self.shares.iter_mut() {
            if *s == slot {
                *share += taken;
            } else {
                *share -= taken * (*share - SLICE_FLOOR).max(0.0) / headroom;
            }
        }
    }
}

/// A compass needle: a `length`-long triangle pointing `angle`
/// clockwise off the face's "up", plus a shorter tail on the opposite
/// side. `RotationRad::ZERO` points to the widget's top edge.
fn paint_needle(context: &Context, center: Vec2<Pixels>, angle: RotationRad, length: Pixels, half_width: Pixels, color: Color<f64>) {
    let (sin, cos) = angle.sin_cos();
    let direction = Vec2::<Pixels>::new(sin, -cos);
    let perp = Vec2::<Pixels>::new(cos, sin);
    for (tip, color) in [(length, color), (length * -0.5, compass_color(CompassColor::Line))] {
        let tip = center + direction * tip.as_f64();
        let left = center - perp * half_width.as_f64();
        let right = center + perp * half_width.as_f64();
        context.move_to(left.x, left.y);
        context.line_to(tip.x, tip.y);
        context.line_to(right.x, right.y);
        context.close_path();
        color.apply(context);
        let _ = context.fill();
    }
}

/// Paints the compass face (dial + ticks), the green needle fixed to
/// the card's "up" and - when `world_up` is `Some` - the red needle,
/// pointing `world_up` so it keeps aiming at the board's top edge no
/// matter how the card is rotated.
fn paint_compass(context: &Context, size: Vec2<Pixels>, world_up: Option<RotationRad>) {
    let center = size * 0.5;
    let radius = Pixels::new(size.x.min(size.y) / 2.0 - 3.0);
    context.arc(center.x, center.y, radius.as_f64(), 0.0, TAU);
    compass_color(CompassColor::Face).apply_alpha(context, 0.95);
    let _ = context.fill_preserve();
    compass_color(CompassColor::Line).apply(context);
    context.set_line_width(2.5);
    let _ = context.stroke();
    for tick in 0..24 {
        let angle = f64::from(tick) * TAU / 24.0;
        let inner = radius.as_f64() - if tick % 6 == 0 { 14.0 } else { 7.0 };
        context.move_to(center.x + inner * angle.sin(), center.y - inner * angle.cos());
        context.line_to(center.x + (radius.as_f64() - 3.0) * angle.sin(), center.y - (radius.as_f64() - 3.0) * angle.cos());
        let _ = context.stroke();
    }
    paint_needle(context, center, RotationRad::ZERO, radius * 0.52, Pixels::new(5.0), compass_color(CompassColor::CardNeedle));
    if let Some(angle) = world_up {
        paint_needle(context, center, angle, radius * 0.62, Pixels::new(6.5), compass_color(CompassColor::WorldNeedle));
    }
    context.arc(center.x, center.y, 4.0, 0.0, TAU);
    compass_color(CompassColor::Line).apply(context);
    let _ = context.fill();
}

/// A compass disc - the face plus the green card-fixed needle. The
/// red world-fixed needle is installed by `main` once the card exists
/// (its draw func reads `card.rotation()`). `CardContent::Inscribed`
/// keeps the surrounding rim chrome.
fn build_compass(size: Pixels<i32>) -> gtk4::DrawingArea {
    let compass = gtk4::DrawingArea::new();
    compass.set_content_width(size.as_raw());
    compass.set_content_height(size.as_raw());
    compass.set_draw_func(|_area, context, width, height| {
        paint_compass(context, Vec2::<Pixels>::new(f64::from(width), f64::from(height)), None);
    });
    compass
}

/// A pie chart drawing `shares` - mounted as
/// `CardContent::Inscribed` so the surrounding ring stays chrome.
/// The draw func reads the shared state, so satellite clicks repaint
/// via `queue_draw`.
fn build_pie(size: Pixels<i32>, shares: Rc<RefCell<PieShares>>) -> gtk4::DrawingArea {
    let pie = gtk4::DrawingArea::new();
    pie.set_content_width(size.as_raw());
    pie.set_content_height(size.as_raw());
    pie.set_draw_func(move |_area, context, width, height| {
        let (center_x, center_y) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        let radius = f64::from(width.min(height)) / 2.0 - 3.0;
        let mut angle = -FRAC_PI_2;
        for (slot, share) in shares.borrow().shares {
            if share <= 0.0 {
                continue;
            }
            context.move_to(center_x, center_y);
            context.arc(center_x, center_y, radius, angle, angle + share * TAU);
            context.close_path();
            pie_color(slot).apply(context);
            let _ = context.fill();
            angle += share * TAU;
        }
    });
    pie
}

/// A slice satellite: a round dot in the slice's color that grows its
/// slice by `amount` on click - `Red`'s first click seeds a
/// `SLICE_FLOOR` share instead of stepping.
fn slice_satellite(class: &str, face: &DrawingArea, shares: &Rc<RefCell<PieShares>>, slot: PieColor) -> Button {
    let button = Button::new();
    button.add_css_class("pie-dot");
    button.add_css_class(class);
    let face = face.downgrade();
    let shares = shares.clone();
    button.connect_clicked(move |_| {
        let amount = if slot == PieColor::Red && shares.borrow().share(slot) <= 0.0 {
            // The first red click seeds a full floor share - a
            // `SLICE_STEP` seed would undercut it.
            SLICE_FLOOR
        } else {
            SLICE_STEP
        };
        shares.borrow_mut().grow(slot, amount);
        if let Some(face) = face.upgrade() {
            face.queue_draw();
        }
    });
    button
}

/// A gauge dial painted edge-to-edge for `CardContent::Cover`: the
/// level ring rides the very rim of the disc, so only the card's
/// silhouette clip bounds it - no ring stays card chrome. `level`
/// (0.0..=1.0) is shared state, so the satellites can repaint the
/// dial via `queue_draw`.
fn build_gauge(size: Pixels<i32>, level: Rc<Cell<f64>>) -> gtk4::DrawingArea {
    let gauge = DrawingArea::new();
    gauge.set_content_width(size.as_raw());
    gauge.set_content_height(size.as_raw());
    gauge.set_draw_func(move |_area, context, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        let (center_x, center_y) = (width / 2.0, height / 2.0);
        let radius = width.min(height) / 2.0;
        context.arc(center_x, center_y, radius, 0.0, TAU);
        context.set_source_rgb(0.12, 0.14, 0.18);
        let _ = context.fill();
        // Level ring on the rim: dim track + bright fill from the top.
        let track = radius - 9.0;
        context.arc(center_x, center_y, track, 0.0, TAU);
        context.set_source_rgba(0.42, 0.48, 0.60, 0.35);
        context.set_line_width(9.0);
        let _ = context.stroke();
        let value = level.get().clamp(0.0, 1.0);
        context.arc(center_x, center_y, track, -FRAC_PI_2, -FRAC_PI_2 + value * TAU);
        context.set_source_rgb(0.35, 0.78, 0.56);
        context.set_line_cap(LineCap::Round);
        let _ = context.stroke();
        // Tick ring inside the level ring.
        context.set_line_width(1.5);
        context.set_source_rgba(0.85, 0.88, 0.95, 0.55);
        for tick in 0..24 {
            let angle = f64::from(tick) * TAU / 24.0;
            let inner = track - if tick % 6 == 0 { 16.0 } else { 9.0 };
            context.move_to(center_x + inner * angle.sin(), center_y - inner * angle.cos());
            context.line_to(center_x + (track - 4.0) * angle.sin(), center_y - (track - 4.0) * angle.cos());
            let _ = context.stroke();
        }
        // Needle pointing at the level on the dial.
        let angle = -FRAC_PI_2 + value * TAU;
        let (sin, cos) = angle.sin_cos();
        context.move_to(center_x, center_y);
        context.line_to(center_x + (radius - 30.0) * -sin, center_y + (radius - 30.0) * cos);
        context.set_source_rgb(0.92, 0.93, 0.96);
        context.set_line_width(4.0);
        let _ = context.stroke();
        context.arc(center_x, center_y, 5.0, 0.0, TAU);
        let _ = context.fill();
    });
    gauge
}

/// A level satellite: a round `+`/`-` button stepping the shared
/// `level` by `delta` on click - the gauge face repaints via
/// `queue_draw`. The cover content itself is display-only
/// (non-targetable), so satellites carry the interaction.
fn level_satellite(label: &str, class: &str, delta: f64, face: &DrawingArea, level: &Rc<Cell<f64>>) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("gauge-btn");
    button.add_css_class(class);
    let face = face.downgrade();
    let level = level.clone();
    button.connect_clicked(move |_| {
        level.set((level.get() + delta).clamp(0.0, 1.0));
        if let Some(face) = face.upgrade() {
            face.queue_draw();
        }
    });
    button
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

    // Example-local styling: the pie's slice satellites are round
    // dots in their slice's color.
    let css = CssProvider::new();
    css.load_from_string(
        ".pie-dot { min-width: 34px; min-height: 34px; padding: 0; border-radius: 9999px; border-width: 2px; border-style: solid; }         .pie-blue { background: rgba(82, 158, 214, 0.55); border-color: rgba(82, 158, 214, 0.9); }         .pie-green { background: rgba(61, 184, 122, 0.55); border-color: rgba(61, 184, 122, 0.9); }         .pie-yellow { background: rgba(235, 168, 51, 0.55); border-color: rgba(235, 168, 51, 0.9); }         .pie-red { background: rgba(217, 64, 56, 0.55); border-color: rgba(217, 64, 56, 0.9); }         .gauge-btn { min-width: 34px; min-height: 34px; padding: 0; border-radius: 9999px; border-width: 2px; border-style: solid; font-weight: bold; }         .gauge-plus { background: rgba(89, 199, 143, 0.55); border-color: rgba(89, 199, 143, 0.9); }         .gauge-minus { background: rgba(150, 160, 190, 0.55); border-color: rgba(150, 160, 190, 0.9); }",
    );
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

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
            &build_clock(Pixels::new(280)),
            CardConfig {
                title: Some("Dial".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 400.0))),
                shape: CardShape::Circle,
                content: CardContent::Fill,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::RotateClockwise),
                    ChromeSlot::inline(ChromeElement::Close),
                ]),
                folded_content: Some(build_clock(Pixels::new(90)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                fold_animation: CardFoldAnimation::Shrink,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("dial"));

    // The Adwaita arc: raised circular theme buttons over the chrome
    // ring; the compass face gains its world-fixed red needle once the
    // card exists.
    let compass_face = build_compass(Pixels::new(200));
    board
        .add_card_with_id(
            "compass",
            &compass_face,
            CardConfig {
                title: Some("Compass".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(750.0, 380.0))),
                shape: CardShape::Circle,
                // A wider arc than the 90° default: the band wraps a
                // third of the disc's rim. The narrow bottom band
                // mirrors it as a second header strip.
                arc_span: Some(Degrees::new(135.0)),
                arc_bottom_span: Some(Degrees::new(30.0)),
                chrome_style: ChromeStyle::Adwaita,
                // `InlineStart`/`InlineEnd` pin to the band's ends like
                // a header bar's packed ends; the title rides the
                // center zone. `Minimize`/`Close` pack the narrow
                // bottom band's start/end.
                chrome: Some(vec![
                    ChromeSlot::inline_start(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline_end(ChromeElement::RotateClockwise),
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_compass(Pixels::new(80)).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("compass"));

    // The red needle counter-rotates against the card's rotation so it
    // always points to the board's top edge; the green one is drawn
    // card-fixed and rotates along. The card's `RotatedLayout` paints
    // the rotation without invalidating the face, so `card-rotated`
    // (emitted per `notify::rotation`) queues the redraw.
    if let Ok(card) = board.card(&CardId::new("compass")) {
        let weak_card = card.downgrade();
        compass_face.set_draw_func(move |_area, context, width, height| {
            let world_up = weak_card.upgrade().map(|card| -card.rotation().to_rotation_rad());
            paint_compass(context, Vec2::<Pixels>::new(f64::from(width), f64::from(height)), world_up);
        });
        let face = compass_face.downgrade();
        card.connect_closure(
            "card-rotated",
            false,
            closure_local!(move |_card: floating_board::Card| {
                if let Some(face) = face.upgrade() {
                    face.queue_draw();
                }
            }),
        );
    }

    // A small disc with a palette: the band takes the palette's
    // header color. The slot order mirrors a header bar - handle and
    // minimize lead, title centers, close packs the band's far end.
    let mut palette = CardPalette::new();
    palette.insert(CardColor::Background, Color::<f64>::new(0.30, 0.26, 0.36));
    palette.insert(CardColor::Header, Color::<f64>::new(0.45, 0.38, 0.55));
    palette.insert(CardColor::Foreground, Color::<f64>::new(0.92, 0.90, 0.95));
    // The face and its folded mini draw from the same shares - slice
    // satellites grow (or seed) their color's slice on click.
    let pie_shares = Rc::new(RefCell::new(PieShares::INITIAL));
    let pie_face = build_pie(Pixels::new(150), pie_shares.clone());
    board
        .add_card_with_id(
            "pie",
            &pie_face,
            CardConfig {
                title: Some("Pie".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(550.0, 600.0))),
                shape: CardShape::Circle,
                palette: Some(palette),
                // The bottom band mirrors the top one - the seat-pass
                // actions read correctly for a user facing the card's
                // bottom edge (start = the user's left).
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::arc_top_end(ChromeElement::Close),
                    ChromeSlot::bar_start(ChromeElement::PreviousSeat),
                    ChromeSlot::bar(ChromeElement::Minimize),
                    ChromeSlot::bar_end(ChromeElement::NextSeat),
                ]),
                folded_content: Some(build_pie(Pixels::new(60), pie_shares.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                // One dot per slice color, in draw order: blue, green
                // and yellow step their slice by +5%, red seeds a
                // 10% slice first and steps after.
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("blue", slice_satellite("pie-blue", &pie_face, &pie_shares, PieColor::Blue)),
                        SatelliteConfig::custom("green", slice_satellite("pie-green", &pie_face, &pie_shares, PieColor::Green)),
                        SatelliteConfig::custom("yellow", slice_satellite("pie-yellow", &pie_face, &pie_shares, PieColor::Yellow)),
                        SatelliteConfig::custom("red", slice_satellite("pie-red", &pie_face, &pie_shares, PieColor::Red)),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("pie"));

    // The gauge (`CardContent::Cover`): the dial reaches the
    // silhouette, no ring stays chrome. Its only band is the bottom
    // one - title centered between the packed Minimize/Close ends.
    // Content is display-only: the +/- satellites trim the level.
    let gauge_level = Rc::new(Cell::new(0.45));
    let gauge_face = build_gauge(Pixels::new(170), gauge_level.clone());
    board
        .add_card_with_id(
            "gauge",
            &gauge_face,
            CardConfig {
                title: Some("Gauge".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(920.0, 610.0))),
                shape: CardShape::Circle,
                content: CardContent::Cover,
                arc_bottom_span: Some(Degrees::new(80.0)),
                chrome: Some(vec![
                    ChromeSlot::bar_start(ChromeElement::Minimize),
                    ChromeSlot::bar(ChromeElement::Title),
                    ChromeSlot::bar_end(ChromeElement::Close),
                ]),
                folded_content: Some(build_gauge(Pixels::new(70), gauge_level.clone()).upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
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
}
