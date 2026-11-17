use floating_board::BoardLayout;
use floating_board::CardAnchor;
use floating_board::CardColor;
use floating_board::CardConfig;
use floating_board::CardId;
use floating_board::CardPalette;
use floating_board::CardPosition;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::MinVisible;
use floating_board::ScaleBounds;
use floating_board::SeatAnchor;
use floating_board::TurnDirection;
use gtk4::Align;
use gtk4::Button;
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
use smearor_commons_geometry::Vec2;
use smearor_commons_layout::FlowLayout;
use smearor_commons_layout::Seat;
use smearor_commons_layout::TableSide;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// The demo's seat roster: user letter and marker color per seat.
/// Passing a card hops between these occupied seats, skipping the
/// empty west edge.
/// One demo user: its seat (mutable), marker letter and badge color.
type DemoUser = (Seat, &'static str, &'static str);

const USERS: [DemoUser; 4] = [
    (Seat::new(TableSide::Bottom), USER_LETTERS[0], USER_COLORS[0]),
    (Seat::new(TableSide::Right), USER_LETTERS[1], USER_COLORS[1]),
    (Seat::at(TableSide::Top, Fraction::new(0.25)), USER_LETTERS[2], USER_COLORS[2]),
    (Seat::at(TableSide::Top, Fraction::new(0.75)), USER_LETTERS[3], USER_COLORS[3]),
];

/// Marker letters available to demo users.
const USER_LETTERS: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
];

/// Marker colors, cycled by letter index.
const USER_COLORS: [&str; 10] = [
    "#3584e4", "#33d17a", "#f6d32d", "#9141ac", "#e66100", "#e01b24", "#62a0ea", "#57e389", "#c061cb", "#865e3c",
];

/// First unused marker letter and its palette color.
fn next_user(users: &[DemoUser]) -> Option<(&'static str, &'static str)> {
    USER_LETTERS
        .iter()
        .enumerate()
        .find(|(_, letter)| !users.iter().any(|user| user.1 == **letter))
        .map(|(index, letter)| (*letter, USER_COLORS[index % USER_COLORS.len()]))
}

/// Table side holding the fewest users (ties keep `TableSide::ALL` order).
fn least_crowded_side(users: &[DemoUser]) -> TableSide {
    TableSide::ALL
        .into_iter()
        .min_by_key(|side| users.iter().filter(|user| user.0.side == *side).count())
        .unwrap_or(TableSide::Bottom)
}

/// An edge offset on `side` not already taken by another user, so a new
/// marker never spawns on top of an existing one.
fn free_edge_offset(users: &[DemoUser], side: TableSide) -> Fraction {
    const SPREAD: [f64; 9] = [0.5, 0.25, 0.75, 0.125, 0.375, 0.625, 0.875, 0.0625, 0.9375];
    let used: Vec<f64> = users
        .iter()
        .filter(|user| user.0.side == side)
        .map(|user| user.0.offset.unwrap_or(Fraction::HALF).as_f64())
        .collect();
    let offset = SPREAD
        .iter()
        .find(|candidate| !used.iter().any(|taken| (taken - *candidate).abs() < 0.02))
        .copied()
        .unwrap_or(((used.len() * 37) % 90 + 5) as f64 / 100.0);
    Fraction::new(offset)
}

/// Markup badge naming the user a seat belongs to (`-` unassigned).
fn seat_badge_markup(seat: Option<Seat>, users: &[DemoUser]) -> String {
    match users.iter().find(|(user_seat, _, _)| Some(*user_seat) == seat) {
        Some((_, letter, color)) => format!("<span background='{color}' foreground='white' weight='bold'> {letter} </span>"),
        None => "<span foreground='#888'> - </span>".to_string(),
    }
}

/// Radius of the user markers, in pixels (also their hit target).
const MARKER_RADIUS: f64 = 13.0;

/// Distance of the marker center from the table edge, in pixels.
const MARKER_INSET: f64 = 24.0;

/// Canvas position of a seat's marker: `offset` along the edge
/// (0 = left/top end), `MARKER_INSET` inside the area.
fn marker_position(seat: Seat, width: f64, height: f64) -> (f64, f64) {
    let offset = seat.offset.unwrap_or(Fraction::HALF).as_f64();
    match seat.side {
        TableSide::Bottom => (width * offset, height - MARKER_INSET),
        TableSide::Left => (MARKER_INSET, height * offset),
        TableSide::Top => (width * offset, MARKER_INSET),
        TableSide::Right => (width - MARKER_INSET, height * offset),
    }
}

/// The seat under a canvas point: the nearest edge becomes the side,
/// the position along it the pinned offset.
fn seat_for_point(x: f64, y: f64, width: f64, height: f64) -> Seat {
    let mut side = TableSide::Top;
    let mut nearest = y;
    for (candidate, distance) in [(TableSide::Bottom, height - y), (TableSide::Left, x), (TableSide::Right, width - x)] {
        if distance < nearest {
            side = candidate;
            nearest = distance;
        }
    }
    let offset = match side {
        TableSide::Top | TableSide::Bottom => x / width,
        TableSide::Left | TableSide::Right => y / height,
    };
    Seat::at(side, Fraction::new(offset.clamp(0.0, 1.0)))
}

/// Interactive test application for the floating board.
///
/// Three pre-configured cards plus dynamic cards added at runtime. The toolbar
/// exercises snapshot/restore; a status line shows live signal feedback.
/// Run with `cargo run --example demo`.
fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
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

    let toolbar = gtk4::Box::new(Orientation::Horizontal, 8);
    toolbar.set_margin_top(8);
    toolbar.set_margin_bottom(8);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);

    let hint = Label::new(Some("Drag card header | 2+ finger move/rotate/stretch | click raises"));
    hint.set_hexpand(true);
    hint.set_halign(Align::Start);
    let content_button = Button::with_label("Content button (click-through)");
    let add_button = Button::with_label("Add card");
    let rotate_button = Button::with_label("Rotate 90°");
    let arrange_button = Button::with_label("Arrange");
    let pass_button = Button::with_label("Pass card →");
    let snapshot_button = Button::with_label("Snapshot");
    let restore_button = Button::with_label("Restore");
    restore_button.set_sensitive(false);
    let mode_switch = Switch::new();
    mode_switch.set_active(true);
    mode_switch.set_valign(Align::Center);
    let mode_label = Label::new(Some("Seats"));
    mode_label.set_valign(Align::Center);
    toolbar.append(&hint);
    let add_user_button = Button::with_label("+ user");
    let remove_user_button = Button::with_label("− user");
    toolbar.append(&mode_label);
    toolbar.append(&mode_switch);
    toolbar.append(&add_user_button);
    toolbar.append(&remove_user_button);
    toolbar.append(&add_button);
    toolbar.append(&rotate_button);
    toolbar.append(&arrange_button);
    toolbar.append(&pass_button);
    toolbar.append(&snapshot_button);
    toolbar.append(&restore_button);
    let gestures_check = gtk4::CheckButton::with_label("View gestures");
    gestures_check.set_valign(Align::Center);
    let buttons_check = gtk4::CheckButton::with_label("View buttons");
    buttons_check.set_valign(Align::Center);
    let seat_anchor_check = gtk4::CheckButton::with_label("Seats @ view");
    seat_anchor_check.set_valign(Align::Center);
    seat_anchor_check.set_active(true);
    seat_anchor_check.set_tooltip_text(Some("On: passes land at the visible edge - Off: seats anchor at the workspace extent"));
    toolbar.append(&gestures_check);
    toolbar.append(&buttons_check);
    toolbar.append(&seat_anchor_check);

    let drawing_area = gtk4::DrawingArea::new();
    drawing_area.set_vexpand(true);
    let draw_users = users.clone();
    drawing_area.set_draw_func(move |_, cr, width, height| {
        cr.set_source_rgb(0.16, 0.18, 0.22);
        let _ = cr.paint();
        cr.set_source_rgb(0.25, 0.28, 0.33);
        let mut x = 0.0;
        while x < width as f64 {
            cr.move_to(x, 0.0);
            cr.line_to(x, height as f64);
            x += 80.0;
        }
        let mut y = 0.0;
        while y < height as f64 {
            cr.move_to(0.0, y);
            cr.line_to(width as f64, y);
            y += 80.0;
        }
        let _ = cr.stroke();

        // Table users at their current seats (the roster drives both
        // the badges and "Pass card"). In seat mode the markers are
        // draggable along the table edge.
        cr.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Bold);
        cr.set_font_size(12.0);
        for (seat, letter, color) in draw_users.borrow().iter() {
            let (x, y) = marker_position(*seat, f64::from(width), f64::from(height));
            cr.arc(x, y, MARKER_RADIUS, 0.0, std::f64::consts::TAU);
            let color = color.parse::<Color<f64>>().unwrap_or_else(|_| Color::<f64>::new(0.29, 0.56, 0.85));
            cr.set_source_rgb(color.r, color.g, color.b);
            let _ = cr.fill();
            if let Ok(extents) = cr.text_extents(letter) {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.move_to(x - extents.width() / 2.0 - extents.x_bearing(), y - extents.height() / 2.0 - extents.y_bearing());
                let _ = cr.show_text(letter);
            }
        }
    });

    let status = gtk4::Label::new(Some("Ready. Move a card."));
    status.set_halign(gtk4::Align::Start);
    status.set_margin_top(4);
    status.set_margin_bottom(4);
    status.set_margin_start(8);

    // Only the drawing area sits inside the board - the card layer
    // covers it alone, so the toolbar and status line always stay
    // clickable and unobscured. The click-through row at the bottom
    // keeps demonstrating that empty card-layer space falls through
    // to the content beneath.
    let board_content = gtk4::Box::new(Orientation::Vertical, 0);
    board_content.append(&drawing_area);
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

    // Opt-in view control: pan/zoom/rotate on empty space, corner
    // buttons - both off by default, toggled live by the checks.
    {
        let board = board.clone();
        gestures_check.connect_toggled(move |check| board.set_view_gestures(check.is_active()));
    }
    {
        let board = board.clone();
        buttons_check.connect_toggled(move |check| board.set_view_buttons(check.is_active()));
    }
    {
        let board = board.clone();
        seat_anchor_check.connect_toggled(move |check| {
            board.set_seat_anchor(if check.is_active() { SeatAnchor::View } else { SeatAnchor::Workspace });
        });
    }

    content.append(&toolbar);
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
                foldable: true,
                closable: true,
                resizable: true,
                rotate_grip: true,
                content_padding: Pixels::new(16.0),
                scale_limits: ScaleBounds::new(0.75, 4.0),
                seat: Some(Seat::new(TableSide::Bottom)),
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
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("parameters"));

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

    // --- Dynamic cards ------------------------------------------------------
    let counter = Cell::new(0_u32);
    {
        let board = board.clone();
        let pass_card = pass_card.clone();
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
            let badge = Label::new(None);
            row.append(&next_button);
            row.append(&prev_button);
            row.append(&badge);
            body.append(&row);
            let id = board.add_card(
                &body,
                CardConfig {
                    title: Some(format!("Card {number}")),
                    position: CardAnchor::Center,
                    closable: true,
                    foldable: true,
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
                    closure_local!(move |card: floating_board::Card| {
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
            let drawing_area = drawing_area.clone();
            drag.connect_drag_begin(move |gesture, x, y| {
                if !mode_switch.is_active() {
                    gesture.set_state(EventSequenceState::Denied);
                    return;
                }
                let width = f64::from(drawing_area.width());
                let height = f64::from(drawing_area.height());
                for (index, (seat, _, _)) in users.borrow().iter().enumerate() {
                    let (mx, my) = marker_position(*seat, width, height);
                    if (x - mx).hypot(y - my) <= MARKER_RADIUS + 8.0 {
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
            let drawing_area = drawing_area.clone();
            let refresh_badges = refresh_badges.clone();
            drag.connect_drag_update(move |gesture, offset_x, offset_y| {
                let Some(index) = dragged.get() else { return };
                let Some((start_x, start_y)) = gesture.start_point() else { return };
                let seat = seat_for_point(start_x + offset_x, start_y + offset_y, f64::from(drawing_area.width()), f64::from(drawing_area.height()));
                if let Some(user) = users.borrow_mut().get_mut(index) {
                    user.0 = seat;
                }
                drawing_area.queue_draw();
                refresh_badges();
            });
        }
        {
            let dragged = dragged.clone();
            drag.connect_drag_end(move |_, _, _| dragged.set(None));
        }
        drawing_area.add_controller(drag);
    }

    // Add/remove demo users. New users take the next free letter on the
    // least crowded edge at a free offset; removing pops the last added
    // (LIFO, like "Add card"). An empty roster exercises
    // BoardError::NoNextSeat on "Pass card".
    {
        let users = users.clone();
        let drawing_area = drawing_area.clone();
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
            drawing_area.queue_draw();
            refresh_badges();
        });
    }
    {
        let users = users.clone();
        let drawing_area = drawing_area.clone();
        let refresh_badges = refresh_badges.clone();
        let status = status.clone();
        remove_user_button.connect_clicked(move |_| {
            let message = match users.borrow_mut().pop() {
                Some((_, letter, _)) => format!("User {letter} left."),
                None => "No users left to remove.".to_string(),
            };
            status.set_text(&message);
            drawing_area.queue_draw();
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
