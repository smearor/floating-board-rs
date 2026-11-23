//! Pure, GTK-free domain core: card identity/geometry, board layout and
//! bounds math. Everything in this module is serializable or pure
//! value/method code and free of `gtk4` imports.

pub(crate) mod board;
pub(crate) mod card;
pub(crate) mod satellite;

pub use board::BoardError;
pub use board::BoardLayout;
pub use board::GenerationCounter;
pub use board::GotoRotation;
pub use board::GotoTransition;
pub use board::SeatAnchor;
pub use board::SequenceNumber;
pub use card::ANCHOR_MARGIN;
pub use card::BoundsMode;
pub use card::CardAnchor;
pub use card::CardColor;
pub use card::CardGeometry;
pub use card::CardId;
pub use card::CardPalette;
pub use card::CardPlacement;
pub use card::CardPosition;
pub use card::CardShape;
pub use card::ChromeElement;
pub use card::ChromePlacement;
pub use card::ChromeSlot;
pub use card::ChromeStyle;
pub use card::CircleContent;
pub use card::DragHandle;
pub use card::MinVisible;
pub use card::ScaleBounds;
pub use card::ScaleFactor;
pub use card::TurnDirection;
pub use satellite::RadialConfig;
pub use satellite::SatelliteCollisionMode;
pub use satellite::SatelliteDistribution;
pub use satellite::SatelliteId;
pub use satellite::SatelliteItem;
pub use satellite::SatelliteKind;
pub use satellite::SatelliteLayout;
pub use satellite::SatellitePlacement;
pub use satellite::SatelliteRotationMode;
pub use satellite::resolve;
