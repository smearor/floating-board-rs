mod table;

use floating_board::BoardLayout;
use floating_board::Card;
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
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::GotoRotation;
use floating_board::GotoTransition;
use floating_board::MinVisible;
use floating_board::SatelliteConfig;
use floating_board::ScaleBounds;
use floating_board::SeatAnchor;
use floating_board::TurnDirection;
use gtk4::Align;
use gtk4::Button;
use gtk4::CheckButton;
use gtk4::EventSequenceState;
use gtk4::GestureDrag;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Scale;
use gtk4::Switch;
use gtk4::glib::closure_local;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::ScaleFactor;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use smearor_commons_layout::FlowLayout;
use smearor_commons_layout::Seat;
use smearor_commons_layout::TableSide;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use table::DemoUser;
use table::MARKER_RADIUS;
use table::USERS;
use table::build_table_face;
use table::free_edge_offset;
use table::least_crowded_side;
use table::marker_position;
use table::next_user;
use table::seat_badge_markup;
use table::seat_for_point;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Interactive test application for the floating board.
///
/// Three pre-configured cards plus dynamic cards added at runtime. The toolbar
/// exercises snapshot/restore; a status line shows live signal feedback.
/// Run with `cargo run --example demo`.
fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    // Optional Adwaita chrome (`ChromeStyle::Adwaita` on the "chrome"
    // card): `cargo run --example demo --features adwaita`.
    #[cfg(feature = "adwaita")]
    if let Err(error) = floating_board::adw::init() {
        panic!("libadwaita init failed: {error}");
    }
    let application = gtk4::Application::new(Some("io.smearor.floating-board.demo"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &gtk4::Application) {
    // Mutable user roster: seat markers are draggable in seat mode, so
    // badges and the pass roster read this live state, not USERS.
    let users: Rc<RefCell<Vec<DemoUser>>> = Rc::new(RefCell::new(USERS.to_vec()));
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board demo"));
    window.set_default_size(1280, 800);

    // --- Bottom layer: ordinary application content -------------------------
    // Clicks on empty areas of the card layer fall through to these widgets.
    let content = gtk4::Box::new(Orientation::Vertical, 0);

    // Demo-local styling: the round button the Settings card folds into.
    let css = gtk4::CssProvider::new();
    css.load_from_string(".round-button { border-radius: 50%; min-width: 46px; min-height: 46px; padding: 0; }");
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    // The demo has no toolbar: every control lives on a sticky card
    // pinned to the glass, so they stay reachable however the camera
    // moves.

    let content_button = Button::with_label("Content button (click-through)");
    let add_button = Button::with_label("Add card");
    let rotate_button = Button::with_label("Rotate 90°");
    let arrange_button = Button::with_label("Arrange");
    let pass_button = Button::with_label("Pass card →");
    let goto_button = Button::with_label("Goto card");
    let snapshot_button = Button::with_label("Snapshot");
    let restore_button = Button::with_label("Restore");
    restore_button.set_sensitive(false);
    let mode_switch = Switch::new();
    mode_switch.set_active(true);
    mode_switch.set_valign(Align::Center);
    let mode_label = Label::new(Some("Seats"));
    mode_label.set_valign(Align::Center);
    let add_user_button = Button::with_label("+ user");
    let remove_user_button = Button::with_label("− user");
    let gestures_check = gtk4::CheckButton::with_label("View gestures");
    gestures_check.set_valign(Align::Center);
    let buttons_check = gtk4::CheckButton::with_label("View buttons");
    buttons_check.set_valign(Align::Center);
    buttons_check.set_active(true);
    buttons_check.set_tooltip_text(Some("Show the view-control card (sticky, bottom-right)"));
    let seat_anchor_check = gtk4::CheckButton::with_label("Seats @ view");
    seat_anchor_check.set_valign(Align::Center);
    seat_anchor_check.set_active(true);
    seat_anchor_check.set_tooltip_text(Some("On: passes land at the visible edge - Off: seats anchor at the workspace extent"));
    let prezi_check = gtk4::CheckButton::with_label("Prezi goto");
    prezi_check.set_valign(Align::Center);
    prezi_check.set_tooltip_text(Some("On: goto pulls back to an overview first - Off: direct flight"));
    let upright_check = gtk4::CheckButton::with_label("Goto upright");
    upright_check.set_valign(Align::Center);
    upright_check.set_active(true);
    upright_check.set_tooltip_text(Some("On: the camera turns the card upright - Off: keep the view rotation"));

    let face = build_table_face(users.clone());
    face.set_vexpand(true);

    let status = Label::new(Some("Ready. Move a card."));
    status.set_halign(Align::Start);
    status.set_margin_top(4);
    status.set_margin_bottom(4);
    status.set_margin_start(8);

    // Only the drawing area sits inside the board - the card layer
    // covers it alone, so the toolbar and status line always stay
    // clickable and unobscured. The click-through row at the bottom
    // keeps demonstrating that empty card-layer space falls through
    // to the content beneath.
    let board_content = gtk4::Box::new(Orientation::Vertical, 0);
    board_content.append(&face);
    let click_row = gtk4::Box::new(Orientation::Horizontal, 0);
    click_row.set_halign(Align::Center);
    click_row.set_margin_bottom(6);
    click_row.append(&content_button);
    board_content.append(&click_row);

    // --- The board ----------------------------------------------------------
    // The board wraps only its own content area, so cards can never
    // cover the toolbar or the status line.
    let board = FloatingBoard::new(&board_content);
    board.set_vexpand(true);

    // Camera chrome as a sticky card: pinned to the bottom-right
    // corner of the glass - it stays put while the view pans, zooms
    // or rotates. The "View buttons" check toggles its visibility.
    let view_controls = gtk4::Box::new(Orientation::Horizontal, 4);
    let zoom_out_button = Button::with_label("−");
    zoom_out_button.set_tooltip_text(Some("Zoom out"));
    let zoom_in_button = Button::with_label("+");
    zoom_in_button.set_tooltip_text(Some("Zoom in"));
    let rotate_left_button = Button::with_label("⟲");
    rotate_left_button.set_tooltip_text(Some("Rotate the table left"));
    let rotate_right_button = Button::with_label("⟳");
    rotate_right_button.set_tooltip_text(Some("Rotate the table right"));
    let reset_button = Button::with_label("⌂");
    reset_button.set_tooltip_text(Some("Reset view"));
    for button in [&zoom_out_button, &zoom_in_button, &rotate_left_button, &rotate_right_button, &reset_button] {
        view_controls.append(button);
    }
    let view_card_id = board
        .add_card_with_id(
            "view-controls",
            &view_controls,
            CardConfig {
                title: Some("View".to_string()),
                position: CardAnchor::BottomRight,
                sticky: true,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("view-controls"));
    {
        let board = board.clone();
        zoom_in_button.connect_clicked(move |_| {
            let anchor = board.view().region.center();
            board.zoom_view(anchor, ScaleFactor::new(1.25));
        });
    }
    {
        let board = board.clone();
        zoom_out_button.connect_clicked(move |_| {
            let anchor = board.view().region.center();
            board.zoom_view(anchor, ScaleFactor::new(0.8));
        });
    }
    {
        let board = board.clone();
        rotate_left_button.connect_clicked(move |_| {
            let pivot = board.view().region.center();
            board.rotate_view(pivot, RotationRad::from_degrees(15.0));
        });
    }
    {
        let board = board.clone();
        rotate_right_button.connect_clicked(move |_| {
            let pivot = board.view().region.center();
            board.rotate_view(pivot, RotationRad::from_degrees(-15.0));
        });
    }
    {
        let board = board.clone();
        reset_button.connect_clicked(move |_| board.reset_view());
    }

    // Sticky control cards: all demo chrome lives on the glass, grouped
    // by topic. Each card explains what its controls do - the cards
    // stay reachable under every camera position.
    let explain = |text: &str| {
        let label = Label::new(Some(text));
        label.add_css_class("dim-label");
        label.set_wrap(false);
        label.set_halign(Align::Start);
        label
    };

    // User: the seat roster. Add/remove users, drag their markers
    // along the table edge - passes target the next occupied seat.
    // Folds into a compact icon row instead of the header strip.
    let user_card = gtk4::Box::new(Orientation::Vertical, 4);
    let user_row = gtk4::Box::new(Orientation::Horizontal, 4);
    user_row.append(&add_user_button);
    user_row.append(&remove_user_button);
    user_card.append(&user_row);
    user_card.append(&explain("Adds/removes a seat marker on the table\nedge. Passes land at the next occupied seat."));
    let user_folded = gtk4::Box::new(Orientation::Horizontal, 0);
    let folded_add = Button::from_icon_name("contact-new-symbolic");
    folded_add.set_tooltip_text(Some("Add user"));
    let folded_remove = Button::from_icon_name("edit-delete-symbolic");
    folded_remove.set_tooltip_text(Some("Remove user"));
    let folded_open = Button::from_icon_name("view-restore-symbolic");
    folded_open.set_tooltip_text(Some("Unfold"));
    user_folded.append(&folded_add);
    user_folded.append(&folded_remove);
    user_folded.append(&folded_open);
    let user_id = board.add_card(
        &user_card,
        CardConfig {
            title: Some("User".to_string()),
            position: CardAnchor::TopLeft,
            sticky: true,
            foldable: true,
            folded_content: Some(user_folded.clone().upcast()),
            ..Default::default()
        },
    );
    let add_button_clone = add_user_button.clone();
    folded_add.connect_clicked(move |_| add_button_clone.emit_clicked());
    let remove_button_clone = remove_user_button.clone();
    folded_remove.connect_clicked(move |_| remove_button_clone.emit_clicked());
    if let Ok(card) = board.card(&user_id) {
        folded_open.connect_clicked(move |_| card.set_folded(false));
    }

    // Layout: how cards travel and align. The Seats switch picks the
    // pass semantics, Seats @ view anchors seats at the screen edge.
    let layout_card = gtk4::Box::new(Orientation::Vertical, 4);
    let mode_row = gtk4::Box::new(Orientation::Horizontal, 4);
    mode_row.append(&Label::new(Some("Table")));
    mode_row.append(&mode_switch);
    mode_row.append(&mode_label);
    layout_card.append(&mode_row);
    let layout_row = gtk4::Box::new(Orientation::Horizontal, 4);
    layout_row.append(&rotate_button);
    layout_row.append(&arrange_button);
    layout_row.append(&pass_button);
    layout_row.append(&goto_button);
    layout_card.append(&layout_row);
    layout_card.append(&explain(
        "Seats: passes hop to the next occupied seat; off: one side.\nSeats @ view: seats at the screen edge, off: workspace.\nRotate/Arrange all cards, Pass/Goto the controls card.",
    ));
    let _layout_id = board.add_card(
        &layout_card,
        CardConfig {
            title: Some("Layout".to_string()),
            position: CardAnchor::CenterLeft,
            sticky: true,
            foldable: true,
            ..Default::default()
        },
    );

    // Settings: camera and goto behavior.
    let settings_card = gtk4::Box::new(Orientation::Vertical, 4);
    let settings_row = gtk4::Box::new(Orientation::Horizontal, 4);
    settings_row.append(&gestures_check);
    settings_row.append(&buttons_check);
    settings_card.append(&settings_row);
    let goto_row = gtk4::Box::new(Orientation::Horizontal, 4);
    goto_row.append(&seat_anchor_check);
    goto_row.append(&prezi_check);
    goto_row.append(&upright_check);
    settings_card.append(&goto_row);
    settings_card.append(&explain(
        "View gestures: pan/zoom/rotate on empty space.\nView buttons: the camera card bottom-right.\nPrezi goto: pull back to an overview first.\nGoto upright: turn the target card readable.",
    ));
    // Folds into a single round burger-menu button.
    let settings_folded = Button::from_icon_name("open-menu-symbolic");
    settings_folded.add_css_class("round-button");
    settings_folded.set_tooltip_text(Some("Settings"));
    let settings_id = board.add_card(
        &settings_card,
        CardConfig {
            title: Some("Settings".to_string()),
            position: CardAnchor::TopRight,
            sticky: true,
            foldable: true,
            folded_content: Some(settings_folded.clone().upcast()),
            ..Default::default()
        },
    );
    if let Ok(card) = board.card(&settings_id) {
        settings_folded.connect_clicked(move |_| card.set_folded(false));
    }

    // Session: board content and persistence.
    let session_card = gtk4::Box::new(Orientation::Vertical, 4);
    let session_row = gtk4::Box::new(Orientation::Horizontal, 4);
    session_row.append(&add_button);
    session_row.append(&snapshot_button);
    session_row.append(&restore_button);
    session_card.append(&session_row);
    session_card.append(&explain(
        "Add card: spawn a dynamic card (pass/goto/sticky\nbuttons inside). Snapshot/Restore round-trips\ngeometry, seats, workspace and view.",
    ));
    let _session_id = board.add_card(
        &session_card,
        CardConfig {
            title: Some("Session".to_string()),
            position: CardAnchor::BottomLeft,
            sticky: true,
            foldable: true,
            ..Default::default()
        },
    );

    // Opt-in view control: pan/zoom/rotate on empty space, corner
    // buttons - both off by default, toggled live by the checks.
    {
        let board = board.clone();
        gestures_check.connect_toggled(move |check| board.set_view_gestures(check.is_active()));
    }
    {
        let board = board.clone();
        buttons_check.connect_toggled(move |check| {
            let _ = board.set_card_visible(&view_card_id, check.is_active());
        });
    }
    {
        let board = board.clone();
        seat_anchor_check.connect_toggled(move |check| {
            board.set_seat_anchor(if check.is_active() { SeatAnchor::View } else { SeatAnchor::Workspace });
        });
    }

    content.append(&board);
    content.append(&status);

    // Card 1: "controls" - interactive controls stay usable, dragged via the
    // header. Foldable and closable chrome enabled.
    let controls = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    controls.set_margin_top(4);
    controls.set_margin_bottom(4);
    controls.set_margin_start(4);
    controls.set_margin_end(4);
    controls.append(&Label::new(Some("Volume")));
    controls.append(&Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0));
    let controls_badge = Label::new(None);
    controls_badge.set_markup(&seat_badge_markup(Some(USERS[0].0), &users.borrow()));
    controls_badge.set_halign(Align::End);
    controls.append(&controls_badge);
    let controls_buttons = gtk4::Box::new(Orientation::Horizontal, 8);
    let play_button = Button::with_label("Play");
    play_button.connect_clicked(|_| info!("play clicked"));
    controls_buttons.append(&play_button);
    controls_buttons.append(&Button::with_label("Stop"));
    controls.append(&controls_buttons);
    let controls_id = board
        .add_card_with_id(
            "controls",
            &controls,
            CardConfig {
                title: Some("Controls".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(300.0, 420.0))),
                content_padding: Pixels::new(16.0),
                scale_limits: ScaleBounds::new(0.75, 4.0),
                seat: Some(Seat::new(TableSide::Bottom)),
                chrome_style: ChromeStyle::Adwaita,
                // The explicit list reproduces the legacy flags
                // (foldable/closable/resizable/rotate_grip). The
                // rotate/pass actions flank the titlebar ends via
                // `InlineStart`/`InlineEnd`, directionally consistent:
                // the left pair counter-rotates/passes, the right pair
                // rotates/passes the other way.
                chrome: Some(vec![
                    ChromeSlot::inline_start(ChromeElement::RotateCounterClockwise),
                    ChromeSlot::inline_start(ChromeElement::NextSeat),
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Close),
                    ChromeSlot::inline_end(ChromeElement::PreviousSeat),
                    ChromeSlot::inline_end(ChromeElement::RotateClockwise),
                    ChromeSlot::inline(ChromeElement::ResizeGrip),
                    ChromeSlot::inline(ChromeElement::RotateGrip),
                ]),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("controls"));

    // Card 2: "info" - readable from the other side of the table (rotated
    // 90 degrees), dragged by its dedicated grip handle, light-themed via
    // a per-card palette.
    let info = gtk4::Box::new(Orientation::Vertical, 4);
    info.append(&Label::new(Some("This card is readable\nfrom the other side\nof the table.")));
    let info_badge = Label::new(None);
    info_badge.set_markup(&seat_badge_markup(Some(USERS[1].0), &users.borrow()));
    info_badge.set_halign(Align::End);
    info.append(&info_badge);
    let mut info_palette = CardPalette::new();
    info_palette.insert(CardColor::Background, Color::<f64>::new(0.93, 0.94, 0.96));
    info_palette.insert(CardColor::Foreground, Color::<f64>::new(0.18, 0.20, 0.24));
    info_palette.insert(CardColor::Header, Color::<f64>::new(0.80, 0.83, 0.88));
    let info_id = board
        .add_card_with_id(
            "info",
            &info,
            CardConfig {
                title: Some("Info".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(540.0, 420.0))),
                drag_handle: DragHandle::Handle,
                min_visible: MinVisible::new(60.0),
                palette: Some(info_palette),
                seat: Some(Seat::new(TableSide::Right)),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("info"));
    if let Ok(card) = board.card(&info_id) {
        card.set_rotation(Rotation::from_degrees(90.0));
    }

    // Card 3: "display" - pure view, headerless; the whole card is the drag
    // affordance.
    let display = gtk4::Box::new(Orientation::Vertical, 4);
    display.append(&Label::new(Some("Display card\n(drag anywhere)")));
    let display_badge = Label::new(None);
    display_badge.set_markup(&seat_badge_markup(Some(USERS[2].0), &users.borrow()));
    display_badge.set_halign(Align::End);
    display.append(&display_badge);
    let display_id = board
        .add_card_with_id(
            "display",
            &display,
            CardConfig {
                title: None,
                headerless: true,
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(770.0, 420.0))),
                drag_handle: DragHandle::WholeCard,
                seat: Some(Seat::at(TableSide::Top, Fraction::new(0.25))),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("display"));

    // Card 4: "parameters" - second foldable card for multi-user testing.
    let parameters = gtk4::Box::new(Orientation::Vertical, 8);
    parameters.append(&Label::new(Some("Edge margin")));
    parameters.append(&Scale::with_range(Orientation::Horizontal, 0.0, 40.0, 1.0));
    parameters.append(&gtk4::CheckButton::with_label("Legacy mode"));
    let parameters_badge = Label::new(None);
    parameters_badge.set_markup(&seat_badge_markup(Some(USERS[3].0), &users.borrow()));
    parameters_badge.set_halign(Align::End);
    parameters.append(&parameters_badge);
    let parameters_id = board
        .add_card_with_id(
            "parameters",
            &parameters,
            CardConfig {
                title: Some("Parameters".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(1000.0, 420.0))),
                foldable: true,
                max_width: Some(Pixels::new(300.0)),
                seat: Some(Seat::at(TableSide::Top, Fraction::new(0.75))),
                // Pinned to the glass: keeps its spot while the camera
                // pans, zooms or rotates (passing it releases the pin).
                sticky: true,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("parameters"));

    // Card 5: "chrome" - Phase-1 showcase for `CardConfig::chrome`: an
    // explicit slot list overrides the legacy flags entirely. `closable`
    // keeps its default `false`, yet the explicit `Close` slot renders a
    // working close button; the explicit `Minimize` slot folds the card.
    let chrome = gtk4::Box::new(Orientation::Vertical, 8);
    chrome.append(&Label::new(Some(
        "Explicit chrome + satellites:
minimize docked below, star = custom
(closable flag is ignored)",
    )));
    // Interactive content exercises hit-testing inside the card: widget
    // input (clicks, slider drags, text selection) must not be eaten by
    // the card's own gestures.
    chrome.append(&Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0));
    let entry = gtk4::Entry::new();
    entry.set_placeholder_text(Some("Type here"));
    chrome.append(&entry);
    let switch_row = gtk4::Box::new(Orientation::Horizontal, 8);
    switch_row.append(&Label::new(Some("Toggle:")));
    let chrome_switch = Switch::new();
    switch_row.append(&chrome_switch);
    chrome.append(&switch_row);
    let counter = Label::new(Some("Clicked: 0"));
    let click_button = Button::with_label("Click me");
    chrome.append(&click_button);
    chrome.append(&counter);
    let clicks = Rc::new(Cell::new(0u32));
    click_button.connect_clicked(move |_| {
        let count = clicks.get() + 1;
        clicks.set(count);
        counter.set_label(&format!("Clicked: {count}"));
    });
    // A custom satellite: the consumer owns the widget and its wiring,
    // the board owns placement and lifetime.
    let star = Button::with_label("\u{2605}");
    // `0°` stays a custom satellite (board-zero is dev semantics); the
    // quarter-step buttons are standard chrome below.
    let rotate_zero = Button::with_label("0\u{b0}");
    let chrome_id = board
        .add_card_with_id(
            "chrome",
            &chrome,
            CardConfig {
                title: Some("Chrome".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(1230.0, 420.0))),
                foldable: true,
                max_width: Some(Pixels::new(300.0)),
                chrome: Some(vec![
                    ChromeSlot::inline(ChromeElement::Title),
                    ChromeSlot::inline(ChromeElement::Minimize),
                    ChromeSlot::inline(ChromeElement::Close),
                    ChromeSlot::inline(ChromeElement::RotateGrip),
                    ChromeSlot::inline(ChromeElement::ResizeGrip),
                    // Phase-2 showcase: `Minimize` docked as a satellite
                    // on the default sidebar below the card, plus the
                    // standard quarter-step rotate actions.
                    ChromeSlot::satellite(ChromeElement::Minimize),
                    ChromeSlot::satellite(ChromeElement::RotateClockwise),
                    ChromeSlot::satellite(ChromeElement::RotateCounterClockwise),
                ]),
                chrome_style: ChromeStyle::Adwaita,
                satellites: Some(CardSatellites {
                    satellites: vec![
                        SatelliteConfig::custom("star", star.clone()),
                        SatelliteConfig::custom("rotate-zero", rotate_zero.clone()),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("chrome"));
    if let Ok(card) = board.card(&chrome_id) {
        star.connect_clicked({
            let card = card.clone();
            move |_| card.fold()
        });
        rotate_zero.connect_clicked(move |_| card.rotate_to_zero());
    }

    // Shared pass helper used by the toolbar button and the Prev/Next
    // buttons on dynamic cards: in "Seats" mode the card hops to the
    // next occupied roster seat, otherwise it steps one table side.
    let pass_card = {
        let board = board.clone();
        let users = users.clone();
        let mode_switch = mode_switch.clone();
        let status = status.clone();
        Rc::new(move |id: &CardId, direction: TurnDirection| {
            let result = if mode_switch.is_active() {
                let roster: Vec<Seat> = users.borrow().iter().map(|(seat, _, _)| *seat).collect();
                board.rotate_to_next_occupied_seat(id, &roster, direction, true)
            } else {
                board.rotate_to_next_user(id, direction, true)
            };
            match result {
                Ok(()) => status.set_text("Passed on."),
                Err(error) => status.set_text(&format!("Pass failed: {error}")),
            }
        }) as Rc<dyn Fn(&CardId, TurnDirection)>
    };

    // Shared goto helper used by the toolbar button and the Goto
    // buttons on dynamic cards: the camera flies onto the card,
    // optionally Prezi-style and with the card turned upright.
    let goto_card = {
        let board = board.clone();
        let prezi_check = prezi_check.clone();
        let upright_check = upright_check.clone();
        let status = status.clone();
        Rc::new(move |id: &CardId| {
            let transition = if prezi_check.is_active() {
                GotoTransition::Transit
            } else {
                GotoTransition::Glide
            };
            let rotation = if upright_check.is_active() {
                GotoRotation::Upright
            } else {
                GotoRotation::Keep
            };
            match board.goto_card(id, transition, rotation) {
                Ok(()) => status.set_text("Flying to card."),
                Err(error) => status.set_text(&format!("Goto failed: {error}")),
            }
        }) as Rc<dyn Fn(&CardId)>
    };

    // --- Dynamic cards ------------------------------------------------------
    let counter = Cell::new(0_u32);
    {
        let board = board.clone();
        let pass_card = pass_card.clone();
        let goto_card = goto_card.clone();
        let users = users.clone();
        add_button.connect_clicked(move |_| {
            let number = counter.get() + 1;
            counter.set(number);
            let body = gtk4::Box::new(Orientation::Vertical, 4);
            body.append(&Label::new(Some(&format!("Dynamic card {number}"))));
            let row = gtk4::Box::new(Orientation::Horizontal, 4);
            row.set_halign(Align::Center);
            let prev_button = Button::with_label("Prev →");
            let next_button = Button::with_label("← Next");
            let goto_button = Button::with_label("Goto");
            let sticky_button = CheckButton::with_label("Sticky");
            sticky_button.set_tooltip_text(Some("Pin to the screen - stays put while the view moves"));
            let badge = Label::new(None);
            row.append(&next_button);
            row.append(&prev_button);
            row.append(&goto_button);
            row.append(&sticky_button);
            row.append(&badge);
            body.append(&row);
            let id = board.add_card(
                &body,
                CardConfig {
                    title: Some(format!("Card {number}")),
                    position: CardAnchor::Center,
                    chrome_style: ChromeStyle::Adwaita,
                    // Explicit chrome replaces the foldable/closable
                    // flags: the quarter-step rotate actions flank the
                    // titlebar ends (CCW left, CW right).
                    chrome: Some(vec![
                        ChromeSlot::inline_start(ChromeElement::RotateCounterClockwise),
                        ChromeSlot::inline(ChromeElement::Title),
                        ChromeSlot::inline(ChromeElement::Minimize),
                        ChromeSlot::inline(ChromeElement::Close),
                        ChromeSlot::inline_end(ChromeElement::RotateClockwise),
                    ]),
                    ..Default::default()
                },
            );
            badge.set_markup(&seat_badge_markup(None, &users.borrow()));
            if let Ok(card) = board.card(&id) {
                let badge = badge.clone();
                let users = users.clone();
                card.connect_closure(
                    "card-seat-changed",
                    false,
                    closure_local!(move |card: Card| {
                        badge.set_markup(&seat_badge_markup(card.seat(), &users.borrow()));
                    }),
                );
            }
            {
                let pass_card = pass_card.clone();
                let id = id.clone();
                prev_button.connect_clicked(move |_| pass_card(&id, TurnDirection::CounterClockwise));
            }
            {
                let pass_card = pass_card.clone();
                let id = id.clone();
                next_button.connect_clicked(move |_| pass_card(&id, TurnDirection::Clockwise));
            }
            {
                let goto_card = goto_card.clone();
                let id = id.clone();
                goto_button.connect_clicked(move |_| goto_card(&id));
            }
            {
                let board = board.clone();
                let id = id.clone();
                sticky_button.connect_toggled(move |button| {
                    let _ = board.set_card_sticky(&id, button.is_active());
                });
            }
            info!(card = %id, "dynamic card added");
        });
    }

    // Rotates every card by +90 degrees with the zoom-snap animation
    // (duration: `CardConfig::animation_speed_ms`, default 500 ms).
    {
        let board = board.clone();
        rotate_button.connect_clicked(move |_| {
            for card in board.cards() {
                let next = card.rotation().degrees() + 90.0;
                card.set_rotation_with_animation(Rotation::Free(RotationRad::from_degrees(next)));
            }
        });
    }

    // Flows all visible cards into a non-overlapping row layout, gliding
    // each card on the frame clock (`animation_speed_ms` per card).
    {
        let board = board.clone();
        let status = status.clone();
        arrange_button.connect_clicked(move |_| match board.apply_layout(&FlowLayout::default(), None, true) {
            Ok(()) => status.set_text("Arranged."),
            Err(error) => status.set_text(&format!("Arrange failed: {error}")),
        });
    }

    // Passes the controls card to the next user clockwise: it glides in
    // front of their seat edge and rotates to their reading
    // orientation; its seat assignment advances with it.
    {
        let controls_id = controls_id.clone();
        let pass_card = pass_card.clone();
        pass_button.connect_clicked(move |_| pass_card(&controls_id, TurnDirection::Clockwise));
    }

    // Flies the camera onto the controls card - centered, slightly
    // inset and upright unless the toggles pick the transit or the
    // keep-rotation mode.
    {
        let controls_id = controls_id.clone();
        goto_button.connect_clicked(move |_| goto_card(&controls_id));
    }

    // --- Snapshot / restore (in-memory; persistence is consumer-side) -------
    let snapshot: Rc<RefCell<Option<BoardLayout>>> = Rc::new(RefCell::new(None));
    {
        let board = board.clone();
        let snapshot = snapshot.clone();
        let restore_button = restore_button.clone();
        let status = status.clone();
        snapshot_button.connect_clicked(move |_| {
            let layout = board.layout_snapshot();
            match serde_json::to_string_pretty(&layout) {
                Ok(json) => {
                    info!("layout snapshot:\n{json}");
                    status.set_text(&format!("Snapshot saved ({} cards).", layout.len()));
                }
                Err(error) => status.set_text(&format!("Snapshot failed: {error}")),
            }
            *snapshot.borrow_mut() = Some(layout);
            restore_button.set_sensitive(true);
        });
    }
    {
        let board = board.clone();
        let status = status.clone();
        restore_button.connect_clicked(move |_| {
            let layout = snapshot.borrow().clone();
            if let Some(layout) = layout {
                board.restore_layout(&layout);
                status.set_text(&format!("Layout restored ({} cards).", layout.len()));
            } else {
                status.set_text("No snapshot yet.");
            }
        });
    }

    content_button.connect_clicked(|_| info!("content button clicked - the gesture fell through the card layer"));

    // --- Live signal feedback ------------------------------------------------
    {
        let status = status.clone();
        board.connect_closure(
            "geometry-changed",
            false,
            closure_local!(move |board: FloatingBoard| {
                let layout = board.layout_snapshot();
                status.set_text(&format!("geometry-changed: {} cards", layout.len()));
            }),
        );
    }
    {
        let status = status.clone();
        board.connect_closure(
            "card-added",
            false,
            closure_local!(move |_: FloatingBoard, card: floating_board::Card| {
                status.set_text(&format!("card-added: {}", card.id()));
            }),
        );
    }
    {
        let status = status.clone();
        board.connect_closure(
            "card-removed",
            false,
            closure_local!(move |_: FloatingBoard, card: floating_board::Card| {
                status.set_text(&format!("card-removed: {}", card.id()));
            }),
        );
    }
    // Refresh every badge from the live roster - shared by
    // `card-seat-changed` (a card travelled) and marker drags (a user
    // moved to another seat).
    let badges: Rc<Vec<(CardId, Label)>> = Rc::new(vec![
        (controls_id.clone(), controls_badge),
        (info_id.clone(), info_badge),
        (display_id.clone(), display_badge),
        (parameters_id.clone(), parameters_badge),
    ]);
    let refresh_badges = {
        let board = board.clone();
        let users = users.clone();
        let badges = badges.clone();
        Rc::new(move || {
            for (id, badge) in badges.iter() {
                if let Ok(card) = board.card(id) {
                    badge.set_markup(&seat_badge_markup(card.seat(), &users.borrow()));
                }
            }
        }) as Rc<dyn Fn()>
    };
    for (id, _) in badges.iter() {
        if let Ok(card) = board.card(id) {
            let refresh_badges = refresh_badges.clone();
            card.connect_closure(
                "card-seat-changed",
                false,
                closure_local!(move |_: floating_board::Card| {
                    refresh_badges();
                }),
            );
        }
    }

    // Drag user markers along the table edge (seat mode only): grab a
    // marker, the pointer snaps to the nearest edge and rewrites that
    // user's side+offset; badges and the pass roster follow live.
    {
        let dragged: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let drag = GestureDrag::new();
        {
            let users = users.clone();
            let mode_switch = mode_switch.clone();
            let dragged = dragged.clone();
            let face = face.clone();
            drag.connect_drag_begin(move |gesture, x, y| {
                if !mode_switch.is_active() {
                    gesture.set_state(EventSequenceState::Denied);
                    return;
                }
                let face_size = face.allocated_size().to_f64().with_unit::<Pixels>();
                for (index, (seat, _, _)) in users.borrow().iter().enumerate() {
                    let at = marker_position(*seat, face_size);
                    if Vec2::<Pixels>::new(x, y).distance(&at) <= MARKER_RADIUS + Pixels::new(8.0) {
                        dragged.set(Some(index));
                        return;
                    }
                }
                gesture.set_state(EventSequenceState::Denied);
            });
        }
        {
            let users = users.clone();
            let dragged = dragged.clone();
            let face = face.clone();
            let refresh_badges = refresh_badges.clone();
            drag.connect_drag_update(move |gesture, offset_x, offset_y| {
                let Some(index) = dragged.get() else { return };
                let Some((start_x, start_y)) = gesture.start_point() else { return };
                let seat = seat_for_point(Vec2::new(start_x + offset_x, start_y + offset_y), face.allocated_size().to_f64().with_unit::<Pixels>());
                if let Some(user) = users.borrow_mut().get_mut(index) {
                    user.0 = seat;
                }
                face.queue_draw();
                refresh_badges();
            });
        }
        {
            let dragged = dragged.clone();
            drag.connect_drag_end(move |_, _, _| dragged.set(None));
        }
        face.add_controller(drag);
    }

    // Add/remove demo users. New users take the next free letter on the
    // least crowded edge at a free offset; removing pops the last added
    // (LIFO, like "Add card"). An empty roster exercises
    // BoardError::NoNextSeat on "Pass card".
    {
        let users = users.clone();
        let face = face.clone();
        let refresh_badges = refresh_badges.clone();
        let status = status.clone();
        add_user_button.connect_clicked(move |_| {
            let mut roster = users.borrow_mut();
            match next_user(&roster) {
                Some((letter, color)) => {
                    let side = least_crowded_side(&roster);
                    let seat = Seat::at(side, free_edge_offset(&roster, side));
                    roster.push((seat, letter, color));
                    status.set_text(&format!("User {letter} joined the {side:?} edge."));
                }
                None => status.set_text("All 26 user letters are taken."),
            }
            drop(roster);
            face.queue_draw();
            refresh_badges();
        });
    }
    {
        let users = users.clone();
        let face = face.clone();
        let refresh_badges = refresh_badges.clone();
        let status = status.clone();
        remove_user_button.connect_clicked(move |_| {
            let message = match users.borrow_mut().pop() {
                Some((_, letter, _)) => format!("User {letter} left."),
                None => "No users left to remove.".to_string(),
            };
            status.set_text(&message);
            face.queue_draw();
            refresh_badges();
        });
    }
    for id in [controls_id, info_id, display_id, parameters_id] {
        if let Ok(card) = board.card(&id) {
            card.connect_closure("card-moved", false, closure_local!(move |card: floating_board::Card| info!(card = %card.id(), "card-moved")));
            card.connect_closure(
                "card-rotated",
                false,
                closure_local!(move |card: floating_board::Card| info!(card = %card.id(), "card-rotated")),
            );
            card.connect_closure(
                "card-folded",
                false,
                closure_local!(move |card: floating_board::Card| info!(card = %card.id(), folded = card.folded(), "card-folded")),
            );
        }
    }

    window.set_child(Some(&content));
    window.present();
}
