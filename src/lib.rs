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

pub use domain::BoardError;
pub use domain::BoardLayout;
pub use domain::BoundsMode;
pub use domain::CardAnchor;
pub use domain::CardGeometry;
pub use domain::CardId;
pub use domain::CardPlacement;
pub use domain::CardPosition;
pub use domain::DragHandle;
pub use domain::MinVisible;
pub use domain::Rect;
pub use domain::ScaleBounds;
pub use domain::ScaleFactor;
pub use smearor_wrot_rotation::SmearorRotation;
pub use widget::Card;
pub use widget::CardConfig;
pub use widget::FloatingBoard;
pub use widget::ScalingWidget;
