//! Satellite showcase (Phase 3): radial orbit layouts.
//!
//! Card "memo" runs `SatelliteDistribution::Even` on a full circle:
//! colored custom satellites plus chrome satellites orbit the card and
//! co-rotate with it. A button inside the card opens and closes the
//! set via `FloatingBoard::set_satellites_open`.
//!
//! Card "anchored" runs `SatelliteDistribution::Anchored` with
//! `CounterRotate`: pinned satellites keep their anchor angle on the
//! orbit - in screen space, since the mode leaves orbit angles
//! screen-fixed. Rotate the card and watch the ring stand still.
//!
//! Run with `cargo run --example satellites` (optionally
//! `--features adwaita` for themed chrome).

use floating_board::CardAnchor;
use floating_board::CardColor;
use floating_board::CardConfig;
use floating_board::CardId;
use floating_board::CardPalette;
use floating_board::CardPosition;
use floating_board::CardSatellites;
use floating_board::CardShape;
use floating_board::ChromeElement;
use floating_board::ChromeSlot;
use floating_board::ChromeStyle;
use floating_board::CircleContent;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::SatelliteConfig;
use floating_board::SatelliteDistribution;
use floating_board::SatelliteLayout;
use floating_board::SatelliteRotationMode;
use gtk4::Button;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::QuarterTurn;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::TAU;
use std::rc::Rc;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// A circular color-dot satellite button styled by `class`.
/// A `DrawingArea` rendering an analog clock face, ticking once per
/// second. Two instances exist: the orb's content and its
/// `folded_content` (a widget has exactly one parent). The unfolded
/// face is three times the folded one.
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
        let Ok(now) = glib::DateTime::now_local() else { return };
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
    glib::timeout_add_local(Duration::from_secs(1), {
        let clock = clock.clone();
        move || {
            clock.queue_draw();
            glib::ControlFlow::Continue
        }
    });
    clock
}

fn dot_button(class: &str) -> Button {
    let button = Button::new();
    button.add_css_class(class);
    button.add_css_class("memo-dot");
    button
}

/// Wires a color-dot satellite to re-color `card_id` on click via
/// `Card::set_palette`.
/// Starting point size of the memo card label (font satellites scale
/// it between 6 and 36 pt).
const MEMO_FONT_PT: f64 = 11.0;

fn wire_color_dot(board: &FloatingBoard, card_id: &CardId, button: &Button, background: DotColor, header: DotColor) {
    let board = board.clone();
    let id = card_id.clone();
    button.connect_clicked(move |_| {
        if let Ok(card) = board.card(&id) {
            let mut palette = CardPalette::new();
            palette.insert(CardColor::Background, Color::<f64>::new(background.0, background.1, background.2));
            palette.insert(CardColor::Header, Color::<f64>::new(header.0, header.1, header.2));
            palette.insert(CardColor::Foreground, Color::<f64>::new(0.20, 0.18, 0.10));
            card.set_palette(&palette);
        }
    });
}

/// An (r, g, b) palette color for the dot specs.
type DotColor = (f64, f64, f64);

/// Dot colors as (css class, background, header) - shared by both
/// cards so the same dot picks the same color.
const DOT_SPECS: [(&str, DotColor, DotColor); 3] = [
    ("memo-green", (0.20, 0.83, 0.48), (0.13, 0.60, 0.34)),
    ("memo-red", (0.94, 0.26, 0.21), (0.72, 0.14, 0.12)),
    ("memo-yellow", (0.96, 0.83, 0.18), (0.86, 0.68, 0.10)),
];

fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    #[cfg(feature = "adwaita")]
    if let Err(error) = floating_board::adw::init() {
        panic!("libadwaita init failed: {error}");
    }
    let application = gtk4::Application::new(Some("io.smearor.floating-board.satellites"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board satellites"));
    window.set_default_size(1100, 800);

    // Example-local styling: FAB-styled radial satellites (thumb
    // size, icon size untouched) plus a dark backdrop so the orbit
    // reads clearly. Border at 80% opacity stays visible; background
    // at 20% opacity reads as glass; the color dots keep their color
    // in the same translucency logic.
    let css = gtk4::CssProvider::new();
    css.load_from_string(
        ".floating-card-satellite-radial { min-width: 56px; min-height: 56px; padding: 0; border-radius: 9999px; border-width: 2px; border-style: solid; border-color: rgba(255, 255, 255, 0.8); background: rgba(255, 255, 255, 0.5); }\
         .memo-green { background: rgba(51, 209, 122, 0.5); border-color: rgba(51, 209, 122, 0.8); }\
         .memo-red { background: rgba(224, 27, 36, 0.5); border-color: rgba(224, 27, 36, 0.8); }\
         .memo-yellow { background: rgba(246, 211, 45, 0.5); border-color: rgba(246, 211, 45, 0.8); }\
         .backdrop { background: #241f31; }",
    );
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let board_content = gtk4::Box::new(Orientation::Vertical, 0);
    board_content.add_css_class("backdrop");
    board_content.append(&Label::new(Some(
        "\nSatellites example\n\nmemo: Even + CoRotate - anchored: Anchored + CounterRotate\norb: Circle card - corners pass through - star rotates, moon recolors, face folds\n",
    )));
    let board = FloatingBoard::new(&board_content);
    board.set_vexpand(true);

    // --- Card "memo": Even radial distribution, co-rotating ----------
    let memo = gtk4::Box::new(Orientation::Vertical, 6);
    let memo_label = Label::new(Some("Memo card\n- pick a color"));
    memo.append(&memo_label);
    let open_button = Button::with_label("Toggle satellites");
    memo.append(&open_button);
    // Font-size satellites: zoom icons, wired below the card mount.
    let font_larger = Button::from_icon_name("zoom-in-symbolic");
    let font_smaller = Button::from_icon_name("zoom-out-symbolic");
    let green = dot_button("memo-green");
    let red = dot_button("memo-red");
    let yellow = dot_button("memo-yellow");
    let memo_dots = [green.clone(), red.clone(), yellow.clone()];
    let mut memo_palette = CardPalette::new();
    memo_palette.insert(CardColor::Background, Color::<f64>::new(0.98, 0.92, 0.55));
    memo_palette.insert(CardColor::Foreground, Color::<f64>::new(0.20, 0.18, 0.10));
    memo_palette.insert(CardColor::Header, Color::<f64>::new(0.88, 0.80, 0.42));
    let memo_id = board
        .add_card_with_id(
            "memo",
            &memo,
            CardConfig {
                title: Some("Memo".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(330.0, 400.0))),
                palette: Some(memo_palette),
                chrome_style: ChromeStyle::Adwaita,
                // Unfolded drags only via the header (buttons stay
                // usable); the folded strip drags anywhere.
                drag_handle: DragHandle::WholeCardFolded,
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Close),
                    ChromeSlot::satellite(ChromeElement::Minimize),
                    ChromeSlot::satellite(ChromeElement::Close),
                    ChromeSlot::satellite(ChromeElement::RotateClockwise),
                    ChromeSlot::satellite(ChromeElement::RotateCounterClockwise),
                ]),
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("green", green.clone()),
                        SatelliteConfig::custom("red", red.clone()),
                        SatelliteConfig::custom("yellow", yellow.clone()),
                        SatelliteConfig::custom("font-larger", font_larger.clone()),
                        SatelliteConfig::custom("font-smaller", font_smaller.clone()),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("memo"));
    {
        let board = board.clone();
        let id = memo_id.clone();
        open_button.connect_clicked(move |_| {
            let open = !board.satellites_open(&id).unwrap_or(true);
            let _ = board.set_satellites_open(&id, open);
        });
    }
    // Consumer wiring for the custom satellites: each dot re-colors
    // the memo card via `Card::set_palette`.
    for (button, (_, background, header)) in memo_dots.iter().zip(DOT_SPECS) {
        wire_color_dot(&board, &memo_id, button, background, header);
    }
    // Custom wiring: the font satellites scale the card's label via
    // Pango markup (absolute sizes are 1024ths of a point).
    let font_size = Rc::new(Cell::new(MEMO_FONT_PT));
    for (button, factor) in [(&font_larger, 1.25), (&font_smaller, 0.8)] {
        let memo_label = memo_label.clone();
        let font_size = font_size.clone();
        button.connect_clicked(move |_| {
            let size = (font_size.get() * factor).clamp(6.0, 36.0);
            font_size.set(size);
            memo_label.set_markup(&format!("<span size=\"{}\">Memo card\n- pick a color</span>", (size * 1024.0) as i64));
        });
    }

    // --- Card "anchored": QuarterTurn anchors, CounterRotate ---------
    let anchored = gtk4::Box::new(Orientation::Vertical, 4);
    anchored.append(&Label::new(Some("Anchored card\n- rotate me, the ring stands still")));
    let anchored_dots: Vec<Button> = DOT_SPECS.iter().map(|(class, _, _)| dot_button(class)).collect();
    let anchored_id = board
        .add_card_with_id(
            "anchored",
            &anchored,
            CardConfig {
                title: Some("Anchored".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(760.0, 400.0))),
                chrome_style: ChromeStyle::Adwaita,
                // Explicit chrome: the inline `Minimize` stays in the
                // titlebar when folded - the unfold path back, since
                // the satellite set hides while folded.
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Close),
                    ChromeSlot::inline(ChromeElement::RotateGrip),
                ]),
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(floating_board::RadialConfig {
                        distribution: SatelliteDistribution::Anchored,
                        ..Default::default()
                    }),
                    rotation_mode: SatelliteRotationMode::CounterRotate,
                    // Anchored: pinned satellites keep their anchor
                    // angle on the screen - `Minimize` up, `Close`
                    // down, the rotate pair left/right; the colored
                    // dots stay flexible and fill the free arcs.
                    satellites: vec![
                        SatelliteConfig::chrome("min", ChromeElement::Minimize).with_anchor(Rotation::QuarterTurn(QuarterTurn::Deg270)),
                        SatelliteConfig::chrome("close", ChromeElement::Close).with_anchor(Rotation::QuarterTurn(QuarterTurn::Deg90)),
                        SatelliteConfig::chrome("cw", ChromeElement::RotateClockwise).with_anchor(Rotation::QuarterTurn(QuarterTurn::Deg0)),
                        SatelliteConfig::chrome("ccw", ChromeElement::RotateCounterClockwise).with_anchor(Rotation::QuarterTurn(QuarterTurn::Deg180)),
                        SatelliteConfig::custom("green", anchored_dots[0].clone()),
                        SatelliteConfig::custom("red", anchored_dots[1].clone()),
                        SatelliteConfig::custom("yellow", anchored_dots[2].clone()),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("anchored"));
    for (button, (_, background, header)) in anchored_dots.iter().zip(DOT_SPECS) {
        wire_color_dot(&board, &anchored_id, button, background, header);
    }

    // --- Card "orb": `CardShape::Circle` - no header, no grips; the
    // legacy `foldable`/`closable` flags are promoted to satellites
    // automatically. The orbit follows the contour exactly (radius =
    // card radius + gap). Its bounding-box corners overlap the memo
    // card: taps there fall through the circle's `contains` and pick
    // memo - the pass-through proof.
    // `CircleContent::Fill`: the widget draws its own circular
    // appearance - a clock face with hands, ticking once per second.
    // The face fills the inner square and leaves the palette ring
    // visible. Folded, the card swaps to a second clock instance via
    // `folded_content` - the minimized card keeps ticking.
    let orb = build_clock(288);
    let orb_folded = build_clock(96);
    let mut orb_palette = CardPalette::new();
    orb_palette.insert(CardColor::Background, Color::<f64>::new(0.30, 0.45, 0.62));
    orb_palette.insert(CardColor::Foreground, Color::<f64>::new(0.95, 0.95, 0.95));
    let orb_day = orb_palette.clone();
    let orb_sats: Vec<Button> = ["starred-symbolic", "night-light-symbolic", "face-smile-symbolic"]
        .iter()
        .map(|icon| Button::from_icon_name(icon))
        .collect();
    board
        .add_card_with_id(
            "orb",
            &orb,
            CardConfig {
                title: Some("Orb".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(465.0, 450.0))),
                palette: Some(orb_palette),
                shape: CardShape::Circle,
                circle_content: CircleContent::Fill,
                foldable: true,
                closable: true,
                // Minimized the card shows the folded clock; a tap
                // unfolds it again (the drag claims move sequences, so
                // dragging never unfolds) and drags move it in both
                // states.
                folded_content: Some(orb_folded.upcast()),
                unfold_on_click: true,
                drag_handle: DragHandle::WholeCard,
                satellites: Some(CardSatellites {
                    layout: SatelliteLayout::Radial(Default::default()),
                    satellites: vec![
                        SatelliteConfig::custom("star", orb_sats[0].clone()),
                        SatelliteConfig::custom("moon", orb_sats[1].clone()),
                        SatelliteConfig::custom("face", orb_sats[2].clone()),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("orb"));

    // orb satellites: star rotates the card a quarter turn clockwise
    // (the clock face turns with it), night-light toggles a dark
    // palette, face folds/unfolds - the folded clock unfolds again on
    // click (`unfold_on_click`), so no Minimize satellite stays.
    orb_sats[0].connect_clicked({
        let board = board.clone();
        move |_| {
            if let Ok(card) = board.card(&CardId::new("orb")) {
                card.set_rotation(Rotation::from_degrees(card.rotation().degrees() + 90.0));
            }
        }
    });
    let mut orb_night = CardPalette::new();
    orb_night.insert(CardColor::Background, Color::<f64>::new(0.10, 0.12, 0.20));
    orb_night.insert(CardColor::Foreground, Color::<f64>::new(0.90, 0.90, 0.95));
    orb_night.insert(CardColor::Header, Color::<f64>::new(0.16, 0.19, 0.30));
    let night = Cell::new(false);
    orb_sats[1].connect_clicked({
        let board = board.clone();
        move |_| {
            night.set(!night.get());
            if let Ok(card) = board.card(&CardId::new("orb")) {
                if night.get() {
                    card.set_palette(&orb_night);
                } else {
                    card.set_palette(&orb_day);
                }
            }
        }
    });
    orb_sats[2].connect_clicked({
        let board = board.clone();
        move |_| {
            if let Ok(card) = board.card(&CardId::new("orb")) {
                card.set_folded(!card.folded());
            }
        }
    });

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    content.append(&board);
    window.set_child(Some(&content));
    window.present();
}
