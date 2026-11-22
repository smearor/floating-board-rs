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
use floating_board::ChromeElement;
use floating_board::ChromeSlot;
use floating_board::ChromeStyle;
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
use tracing_subscriber::EnvFilter;

/// A circular color-dot satellite button styled by `class`.
fn dot_button(class: &str) -> Button {
    let button = Button::new();
    button.add_css_class(class);
    button.add_css_class("memo-dot");
    button
}

/// Wires a color-dot satellite to re-color `card_id` on click via
/// `Card::set_palette`.
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

    // Example-local styling: the colored dot satellites and a dark
    // backdrop so the orbit reads clearly.
    let css = gtk4::CssProvider::new();
    css.load_from_string(
        ".memo-dot { min-width: 20px; min-height: 20px; padding: 0; }\
         .memo-green { background: #33d17a; }\
         .memo-red { background: #e01b24; }\
         .memo-yellow { background: #f6d32d; }\
         .backdrop { background: #241f31; }",
    );
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    let board_content = gtk4::Box::new(Orientation::Vertical, 0);
    board_content.add_css_class("backdrop");
    board_content.append(&Label::new(Some(
        "\nSatellites example\n\nmemo: Even + CoRotate - anchored: Anchored + CounterRotate\nrotate both cards to compare\n",
    )));
    let board = FloatingBoard::new(&board_content);
    board.set_vexpand(true);

    // --- Card "memo": Even radial distribution, co-rotating ----------
    let memo = gtk4::Box::new(Orientation::Vertical, 6);
    memo.append(&Label::new(Some("Memo card\n- pick a color")));
    let open_button = Button::with_label("Toggle satellites");
    memo.append(&open_button);
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

    let content = gtk4::Box::new(Orientation::Vertical, 0);
    content.append(&board);
    window.set_child(Some(&content));
    window.present();
}
