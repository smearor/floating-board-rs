//! # floating-board
//!
//! A GTK4 widget that renders freely movable, rotatable and scalable cards on
//! top of an application's content - designed for table-top multitouch surfaces
//! where there is no "up" or "down".
//!
//! ## Layers
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Consumer application                                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ widget/   - thin GTK glue (gestures, layout, transforms)         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ domain/   - pure Rust: newtypes, geometry math, layout serde     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use floating_board::CardAnchor;
//! use floating_board::CardConfig;
//! use floating_board::FloatingBoard;
//!
//! # fn build(content: &gtk4::Widget, controls: &gtk4::Widget) {
//! let board = FloatingBoard::new(content);
//! let id = board.add_card(
//!     controls,
//!     CardConfig {
//!         title: Some("Controls".into()),
//!         position: CardAnchor::BottomCenter,
//!         ..Default::default()
//!     },
//! );
//! let layout = board.layout_snapshot();
//! board.restore_layout(&layout);
//! # let _ = id;
//! # }
//! ```

pub mod domain;
pub mod widget;

mod l10n;

/// libadwaita, re-exported for consumers of `ChromeStyle::Adwaita` so
/// they can call `adw::init()` without depending on the exact
/// libadwaita version this crate builds against.
#[cfg(feature = "adwaita")]
pub use libadwaita as adw;

pub use domain::BoardError;
pub use domain::BoardLayout;
pub use domain::BoundsMode;
pub use domain::CardAnchor;
pub use domain::CardColor;
pub use domain::CardGeometry;
pub use domain::CardId;
pub use domain::CardPalette;
pub use domain::CardPlacement;
pub use domain::CardPosition;
pub use domain::ChromeElement;
pub use domain::ChromePlacement;
pub use domain::ChromeSlot;
pub use domain::ChromeStyle;
pub use domain::DragHandle;
pub use domain::GotoRotation;
pub use domain::GotoTransition;
pub use domain::MinVisible;
pub use domain::ScaleBounds;
pub use domain::ScaleFactor;
pub use domain::SeatAnchor;
pub use domain::SequenceNumber;
pub use domain::TurnDirection;
pub use widget::Card;
pub use widget::CardAlignment;
pub use widget::CardCloseAction;
pub use widget::CardConfig;
pub use widget::FloatingBoard;
pub use widget::ScalingWidget;
