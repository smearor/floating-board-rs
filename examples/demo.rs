use floating_board::BoardLayout;
use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::CardId;
use floating_board::CardPosition;
use floating_board::DragHandle;
use floating_board::FloatingBoard;
use floating_board::MinVisible;
use floating_board::ScaleBounds;
use floating_board::SmearorRotation;
use gtk4::glib::closure_local;
use gtk4::prelude::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::info;
use tracing_subscriber::EnvFilter;

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
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("floating-board demo"));
    window.set_default_size(1280, 800);

    // --- Bottom layer: ordinary application content -------------------------
    // Clicks on empty areas of the card layer fall through to these widgets.
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    toolbar.set_margin_top(8);
    toolbar.set_margin_bottom(8);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);

    let hint = gtk4::Label::new(Some("Drag card header | 2+ finger move/rotate/stretch | click raises"));
    hint.set_hexpand(true);
    hint.set_halign(gtk4::Align::Start);
    let content_button = gtk4::Button::with_label("Content button (click-through)");
    let add_button = gtk4::Button::with_label("Add card");
    let rotate_button = gtk4::Button::with_label("Rotate 90°");
    let snapshot_button = gtk4::Button::with_label("Snapshot");
    let restore_button = gtk4::Button::with_label("Restore");
    restore_button.set_sensitive(false);
    toolbar.append(&hint);
    toolbar.append(&content_button);
    toolbar.append(&add_button);
    toolbar.append(&rotate_button);
    toolbar.append(&snapshot_button);
    toolbar.append(&restore_button);

    let drawing_area = gtk4::DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_draw_func(|_, cr, width, height| {
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
    });

    let status = gtk4::Label::new(Some("Ready. Move a card."));
    status.set_halign(gtk4::Align::Start);
    status.set_margin_top(4);
    status.set_margin_bottom(4);
    status.set_margin_start(8);

    content.append(&toolbar);
    content.append(&drawing_area);
    content.append(&status);

    // --- The board ----------------------------------------------------------
    let board = FloatingBoard::new(&content);

    // Card 1: "controls" - interactive controls stay usable, dragged via the
    // header. Foldable and closable chrome enabled.
    let controls = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    controls.set_margin_top(4);
    controls.set_margin_bottom(4);
    controls.set_margin_start(4);
    controls.set_margin_end(4);
    controls.append(&gtk4::Label::new(Some("Volume")));
    controls.append(&gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0));
    let controls_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let play_button = gtk4::Button::with_label("Play");
    play_button.connect_clicked(|_| info!("play clicked"));
    controls_buttons.append(&play_button);
    controls_buttons.append(&gtk4::Button::with_label("Stop"));
    controls.append(&controls_buttons);
    let controls_id = board
        .add_card_with_id(
            "controls",
            &controls,
            CardConfig {
                title: Some("Controls".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(300.0, 420.0)),
                foldable: true,
                closable: true,
                scale_limits: ScaleBounds::new(0.75, 4.0),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("controls"));

    // Card 2: "info" - readable from the other side of the table (rotated
    // 90 degrees), dragged by its dedicated grip handle.
    let info = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    info.append(&gtk4::Label::new(Some("This card is readable\nfrom the other side\nof the table.")));
    let info_id = board
        .add_card_with_id(
            "info",
            &info,
            CardConfig {
                title: Some("Info".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(540.0, 420.0)),
                drag_handle: DragHandle::Handle,
                min_visible: MinVisible::new(60.0),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("info"));
    if let Ok(card) = board.card(&info_id) {
        card.set_rotation(SmearorRotation::Deg90);
    }

    // Card 3: "display" - pure view, headerless; the whole card is the drag
    // affordance.
    let display = gtk4::Label::new(Some("Display card\n(drag anywhere)"));
    let display_id = board
        .add_card_with_id(
            "display",
            &display,
            CardConfig {
                title: None,
                headerless: true,
                position: CardAnchor::Explicit(CardPosition::new(770.0, 420.0)),
                drag_handle: DragHandle::WholeCard,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("display"));

    // Card 4: "parameters" - second foldable card for multi-user testing.
    let parameters = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    parameters.append(&gtk4::Label::new(Some("Edge margin")));
    parameters.append(&gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 40.0, 1.0));
    parameters.append(&gtk4::CheckButton::with_label("Legacy mode"));
    let parameters_id = board
        .add_card_with_id(
            "parameters",
            &parameters,
            CardConfig {
                title: Some("Parameters".to_string()),
                position: CardAnchor::Explicit(CardPosition::new(1000.0, 420.0)),
                foldable: true,
                max_width: Some(300),
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| CardId::new("parameters"));

    // --- Dynamic cards ------------------------------------------------------
    let counter = Cell::new(0_u32);
    {
        let board = board.clone();
        add_button.connect_clicked(move |_| {
            let number = counter.get() + 1;
            counter.set(number);
            let body = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            body.append(&gtk4::Label::new(Some(&format!("Dynamic card {number}"))));
            body.append(&gtk4::Button::with_label("Click me"));
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
            info!(card = %id, "dynamic card added");
        });
    }

    // Rotates every card by +90 degrees with the zoom-snap animation
    // (duration: `CardConfig::animation_speed_ms`, default 500 ms).
    {
        let board = board.clone();
        rotate_button.connect_clicked(move |_| {
            for card in board.cards() {
                let next = card.rotation().to_degrees() + 90.0;
                card.set_rotation_with_animation(SmearorRotation::Deg(next));
            }
        });
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

    window.set_child(Some(&board));
    window.present();
}
