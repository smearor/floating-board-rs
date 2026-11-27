use crate::domain::ANCHOR_MARGIN;
use crate::domain::BoundsMode;
use crate::domain::CardAnchor;
use crate::domain::CardContent;
use crate::domain::CardFoldAnimation;
use crate::domain::CardFoldTiming;
use crate::domain::CardPalette;
use crate::domain::CardShape;
use crate::domain::ChromeElement;
use crate::domain::ChromePlacement;
use crate::domain::ChromeSlot;
use crate::domain::ChromeStyle;
use crate::domain::DragHandle;
use crate::domain::MinVisible;
use crate::domain::ScaleBounds;
use crate::widget::card_close_action::CardCloseAction;
use crate::widget::card_satellites::CardSatellites;
use crate::widget::satellite_config::SatelliteConfig;
use gtk4::Widget;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_layout::Seat;

/// Default space between the card border and the content widget (px) -
/// the value `.floating-card-content` previously carried as fixed CSS
/// padding.
pub const DEFAULT_CONTENT_PADDING: Pixels = Pixels::new(10.0);

/// Default fraction of an `Annulus` card's outer radius that is the
/// see-through, click-through hole.
pub const DEFAULT_HOLE_RATIO: f64 = 0.5;

/// Placement and interaction configuration of a card.
///
/// Passed to `FloatingBoard::add_card`; all fields have defaults so
/// `CardConfig::default()` yields a fully interactive, draggable card anchored
/// at the board center.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardConfig {
    /// Title shown in the header strip; falls back to a localized default when
    /// the header is visible and no title is set.
    pub title: Option<String>,
    /// Initial placement anchor, resolved on first allocation.
    pub position: CardAnchor,
    /// Whether the card can be moved by dragging.
    pub draggable: bool,
    /// Whether the card can be rotated via the two-finger rotate gesture.
    pub rotatable: bool,
    /// Whether the card can be scaled via the pinch gesture.
    pub scalable: bool,
    /// Whether the card shows a bottom-right resize grip that scales the
    /// card around its center when dragged (mouse and touch).
    pub resizable: bool,
    /// Whether the card shows a bottom-left rotate grip that steps the
    /// rotation to the next quarter turn in the swipe direction - one
    /// quarter turn per drag, no matter how far the swipe goes (mouse
    /// and touch). Works independently of `rotatable`, which gates the
    /// free two-finger rotate.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rotate_grip: bool,
    /// Whether the card can be folded down to its header.
    pub foldable: bool,
    /// Whether the header shows a close button.
    pub closable: bool,
    /// What the close button does - remove the card (default) or hide it
    /// so a menu can re-open it later.
    pub close_action: CardCloseAction,
    /// Collapses the header to a slim grip when no title/buttons are needed.
    pub headerless: bool,
    /// Which part of the card acts as the drag affordance.
    pub drag_handle: DragHandle,
    /// The card's outline shape - `Rectangle` (default) or `Circle`.
    ///
    /// A circle card has no header strip and no corner grips: inline
    /// chrome mounts along the disc's arc bands (`arc_span` on top,
    /// `arc_bottom_span` on the bottom), `Satellite`
    /// placements stay satellites, the edge grips are dropped with a
    /// warning at mount, and taps in the bounding-box corners pass
    /// through to whatever is underneath.
    #[cfg_attr(feature = "serde", serde(default))]
    pub shape: CardShape,
    /// How a shaped card (`Circle`/`Semicircle`/`Hexagon`/`Octagon`)
    /// allocates its content inside the bounding box: `Inscribed`
    /// fits the shape's inscribed square, `Fill` the rim-shrunk
    /// silhouette (the content draws its own shape), `Clipped` like
    /// `Fill` plus a silhouette clip so picks in the cut regions fall
    /// back onto the card. Ignored for `Rectangle`.
    #[cfg_attr(feature = "serde", serde(default, alias = "circle_content"))]
    pub content: CardContent,
    /// Angular span of the arc-chrome band on `CardShape::Circle` and
    /// `CardShape::Annulus` - the top arc the inline chrome items ride
    /// on, centered on the card's top. `None` defaults to
    /// `Degrees::QUARTER_TURN` (90°); values above a full turn are
    /// clamped. Ignored for `Rectangle` and for cards without inline
    /// chrome.
    #[cfg_attr(feature = "serde", serde(default))]
    pub arc_span: Option<Degrees>,
    /// Angular span of the bottom arc-chrome band on
    /// `CardShape::Circle` and `CardShape::Annulus`. `None` mirrors
    /// `arc_span`; set it when the bottom band should wrap a different
    /// share of the rim than the top band. Ignored for `Rectangle` and
    /// for cards without `Bar*` chrome.
    #[cfg_attr(feature = "serde", serde(default))]
    pub arc_bottom_span: Option<Degrees>,
    /// Fraction of the outer radius making the see-through,
    /// click-through hole on `CardShape::Annulus`. `None` defaults to
    /// `DEFAULT_HOLE_RATIO` (0.5); clamped to `0.05..=0.95`. Ignored
    /// for every other shape.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hole_ratio: Option<f64>,
    /// Margin in pixels an edge or corner anchor keeps between the card's
    /// footprint and the board edge; default `ANCHOR_MARGIN` (24 px).
    /// Ignored for `Center` and `Explicit` placements.
    #[cfg_attr(feature = "serde", serde(default = "default_anchor_margin"))]
    pub anchor_margin: Pixels,
    /// Inclusive scale limits for the pinch gesture; default `0.5 - 2.0`.
    pub scale_limits: ScaleBounds,
    /// Minimum overlap in pixels a card must keep inside the board bounds.
    pub min_visible: MinVisible,
    /// How strictly the card's footprint is kept inside the board bounds;
    /// default `Loose` (only `min_visible` is enforced).
    #[cfg_attr(feature = "serde", serde(default))]
    pub bounds_mode: BoundsMode,
    /// Per-card color scheme: a [`CardPalette`] mapped onto scoped CSS
    /// at build time. `None` (or a palette with unfilled slots) keeps
    /// the themed/default-card colors.
    #[cfg_attr(feature = "serde", serde(default))]
    pub palette: Option<CardPalette>,
    /// Space in pixels between the card border and the content widget;
    /// default `DEFAULT_CONTENT_PADDING` (10 px, the former fixed CSS
    /// value). `0` lets the content sit flush with the card edge.
    /// Applies to `CardContent::Inscribed` only - `Fill`/`Clipped`
    /// carry the ring band as their designed inset already, and
    /// `Cover` is edge-to-edge.
    #[cfg_attr(feature = "serde", serde(default = "default_content_padding"))]
    pub content_padding: Pixels,
    /// Optional initial z-ordering; `None` appends the card on top.
    pub z_index: Option<i32>,
    /// Hard upper bound on the card's measured width; `None` lets
    /// the card grow with its content. Caps growth caused by changing content
    /// (e.g. a number gaining a digit): the layout allocates at most this
    /// size and overflowing content is clipped by the default CSS.
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_width: Option<Pixels>,
    /// Hard upper bound on the card's measured height; see
    /// `max_width`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_height: Option<Pixels>,
    /// Duration of rotation animations (gesture snapping and
    /// `set_rotation_with_animation`) in milliseconds; default 500.
    #[cfg_attr(feature = "serde", serde(default = "default_animation_speed_ms"))]
    pub animation_speed_ms: u64,
    /// Optional seat assignment on the tabletop edge the card belongs
    /// to - decoupled from the card's visual rotation: rotating the
    /// board or freely spinning the card does not change which user
    /// owns it. `None` leaves the card unassigned; seat strategies and
    /// `rotate_to_next_user` fall back to the nearest side of its
    /// rotation.
    #[cfg_attr(feature = "serde", serde(default))]
    pub seat: Option<Seat>,
    /// Custom widget shown while the card is folded. When set, folding
    /// collapses the whole card - header included - to this widget and
    /// adds the `floating-card-collapsed` class, so a round button or
    /// a compact mini view can replace the full card. `None` keeps the
    /// default fold that leaves the header strip visible.
    ///
    /// The folded widget should offer an unfold affordance wired to
    /// `Card::set_folded(false)` or `Card::fold` - the card's own fold
    /// button hides with the header. `unfold_on_click` provides a
    /// ready-made affordance for cards without one.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub folded_content: Option<Widget>,
    /// A click on the folded card unfolds it (revealing the satellite
    /// set again). The click gesture only releases on sequences the
    /// card drag did not claim, so moving a folded card never unfolds
    /// it. While enabled the circle card's `Minimize` fold exception is
    /// unnecessary - the card is one tap away from unfolding - so the
    /// satellite closes with the rest of the set.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unfold_on_click: bool,
    /// How the card transitions between unfolded and folded
    /// presentation (`CardFoldAnimation`): an eased program over the
    /// axes width, height, rotation, zoom and content opacity,
    /// budgeted by `animation_speed_ms`. `Instant` (default) swaps
    /// atomically; reduced motion lands instantly too.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fold_animation: CardFoldAnimation,
    /// The morph program for the unfold direction - `None` replays
    /// `fold_animation` mirrored, so a single setting animates both
    /// directions.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unfold_animation: Option<CardFoldAnimation>,
    /// When an animated fold plays relative to the satellite close
    /// choreography: `Sequential` (default) morphs after the last
    /// satellite sank into the card, `Overlap(fraction)` starts the
    /// morph mid-close (`0.0` = parallel). Unfolds always morph while
    /// the set reveals.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fold_timing: CardFoldTiming,
    /// The unfold-direction choreography timing relative to the
    /// satellite reveal - `None` resolves to `Overlap(0.0)`, the
    /// morph and the reveal run in lockstep.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unfold_timing: Option<CardFoldTiming>,
    /// Pins the card to the screen instead of the workspace: a sticky
    /// card ignores the `View` camera entirely and keeps its on-screen
    /// position, rotation and zoom while the view pans, zooms or
    /// rotates. Its stored geometry is then expressed in widget
    /// pixels; toggling converts between the two spaces, so pinning
    /// never moves the card visually. Sticky cards are skipped by
    /// `apply_layout`, and passing one to a seat releases the pin.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sticky: bool,
    /// Explicit chrome composition as an ordered slot list
    /// (`CardConfig::chrome`).
    ///
    /// - `None` (default): derive slots from the legacy flags
    ///   (`title`/`title_label` -> `Title`, `foldable` -> `Minimize`,
    ///   `closable` -> `Close`, `resizable` -> `ResizeGrip`,
    ///   `rotate_grip` -> `RotateGrip`,
    ///   `drag_handle == Handle` -> `DragHandle`; all `Inline`).
    /// - `Some(slots)`: overrides the legacy-derived chrome entirely -
    ///   the list is the complete chrome description. `Satellite`
    ///   placements are validated at build time; until satellite
    ///   mounting lands they are skipped (no inline fallback).
    #[cfg_attr(feature = "serde", serde(default))]
    pub chrome: Option<Vec<ChromeSlot>>,
    /// Which renderer draws the chrome (`CardConfig::chrome_style`):
    /// `Builtin` (default) or `Adwaita` (requires the `adwaita`
    /// feature; falls back to `Builtin` without it).
    #[cfg_attr(feature = "serde", serde(default))]
    pub chrome_style: ChromeStyle,
    /// Satellite configuration: layout, rotation and collision
    /// semantics plus the explicit satellite list (`CardSatellites`).
    /// `None` (default) means no custom satellites; chrome elements
    /// configured `ChromeSlot::satellite(..)` still mount with the
    /// `CardSatellites` defaults (sidebar below the card).
    #[cfg_attr(feature = "serde", serde(default))]
    pub satellites: Option<CardSatellites>,
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            title: None,
            position: CardAnchor::Center,
            draggable: true,
            rotatable: true,
            scalable: true,
            resizable: false,
            rotate_grip: false,
            foldable: false,
            closable: false,
            close_action: CardCloseAction::Remove,
            headerless: false,
            drag_handle: DragHandle::Header,
            shape: CardShape::default(),
            content: CardContent::default(),
            arc_span: None,
            arc_bottom_span: None,
            hole_ratio: None,
            anchor_margin: ANCHOR_MARGIN,
            scale_limits: ScaleBounds::default(),
            min_visible: MinVisible::default(),
            bounds_mode: BoundsMode::default(),
            palette: None,
            content_padding: DEFAULT_CONTENT_PADDING,
            z_index: None,
            max_width: None,
            max_height: None,
            animation_speed_ms: 500,
            seat: None,
            folded_content: None,
            unfold_on_click: false,
            fold_animation: CardFoldAnimation::default(),
            unfold_animation: None,
            fold_timing: CardFoldTiming::default(),
            unfold_timing: None,
            sticky: false,
            chrome: None,
            chrome_style: ChromeStyle::default(),
            satellites: None,
        }
    }
}

impl CardConfig {
    /// The fold morph program for a transition direction:
    /// `unfold_animation` falls back to `fold_animation`, so a single
    /// setting animates both directions.
    pub fn effective_fold_animation(&self, to_folded: bool) -> CardFoldAnimation {
        match to_folded {
            true => self.fold_animation,
            false => self.unfold_animation.unwrap_or(self.fold_animation),
        }
    }

    /// The choreography timing for a transition direction: the unfold
    /// side defaults to `Overlap(0.0)` - morph and satellite reveal
    /// run in lockstep.
    pub fn effective_fold_timing(&self, to_folded: bool) -> CardFoldTiming {
        match to_folded {
            true => self.fold_timing,
            false => self.unfold_timing.unwrap_or(CardFoldTiming::Overlap(Fraction::ZERO)),
        }
    }

    /// The effective ordered chrome slots - either the explicit
    /// `CardConfig::chrome` list or the list derived from the legacy
    /// flags (default: `DragHandle`, `Title`, `Minimize`, `Close`,
    /// `ResizeGrip`, `RotateGrip`, all `Inline`; elements whose flag
    /// is off are absent).
    pub fn effective_chrome_slots(&self) -> Vec<ChromeSlot> {
        if let Some(chrome) = &self.chrome {
            return chrome.clone();
        }
        self.legacy_chrome_slots()
    }

    /// Slots derived from the legacy boolean flags, in the historical
    /// header order (leading: handle + title, trailing: minimize +
    /// close; grips mount onto the content overlay).
    fn legacy_chrome_slots(&self) -> Vec<ChromeSlot> {
        let mut slots = Vec::with_capacity(6);
        if self.drag_handle == DragHandle::Handle {
            slots.push(ChromeSlot::inline(ChromeElement::DragHandle));
        }
        slots.push(ChromeSlot::inline(ChromeElement::Title));
        if self.foldable {
            slots.push(ChromeSlot::inline(ChromeElement::Minimize));
        }
        if self.closable {
            slots.push(ChromeSlot::inline(ChromeElement::Close));
        }
        if self.resizable {
            slots.push(ChromeSlot::inline(ChromeElement::ResizeGrip));
        }
        if self.rotate_grip {
            slots.push(ChromeSlot::inline(ChromeElement::RotateGrip));
        }
        slots
    }

    /// The effective satellite set: `ChromeSlot::satellite` entries in
    /// slot order (elements that cannot be satellites are warned about
    /// once at mount), then the explicit `CardSatellites::satellites`
    /// list. Layout, rotation and collision semantics come from
    /// `satellites`, defaulting to a `CoRotate` sidebar below the card.
    ///
    /// Non-satellite placements are never satellites: on `Rectangle`
    /// they mount in the header strip, on `Circle` in the top-arc
    /// chrome band (`Card::mount_chrome` owns both mounts).
    pub fn effective_satellites(&self) -> (CardSatellites, Vec<SatelliteConfig>) {
        let settings = self.satellites.clone().unwrap_or_default();
        let mut configs: Vec<SatelliteConfig> = self
            .effective_chrome_slots()
            .iter()
            .filter(|slot| slot.placement == ChromePlacement::Satellite && slot.element.supports_satellite())
            .map(|slot| SatelliteConfig::chrome(format!("chrome-{}", satellite_key(slot.element)), slot.element))
            .collect();
        configs.extend(settings.satellites.iter().cloned());
        (settings, configs)
    }
}

/// Deterministic id suffix for chrome satellites (`chrome-minimize`).
/// Only satellite-capable elements reach it (`supports_satellite`
/// filtered upstream); the fallback keeps the match total.
fn satellite_key(element: ChromeElement) -> &'static str {
    match element {
        ChromeElement::Minimize => "minimize",
        ChromeElement::Close => "close",
        ChromeElement::RotateClockwise => "rotate-clockwise",
        ChromeElement::RotateCounterClockwise => "rotate-counter-clockwise",
        ChromeElement::NextSeat => "next-seat",
        ChromeElement::PreviousSeat => "previous-seat",
        // Non-satellite-capable elements never reach this function
        // (`supports_satellite` filters upstream); distinct arms keep
        // id collisions impossible if the capability set grows.
        ChromeElement::DragHandle | ChromeElement::Title | ChromeElement::ResizeGrip | ChromeElement::RotateGrip => "element",
    }
}

#[cfg(feature = "serde")]
fn default_anchor_margin() -> Pixels {
    ANCHOR_MARGIN
}

#[cfg(feature = "serde")]
fn default_animation_speed_ms() -> u64 {
    500
}

#[cfg(feature = "serde")]
fn default_content_padding() -> Pixels {
    DEFAULT_CONTENT_PADDING
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ChromeElement;
    use crate::domain::ChromePlacement;

    fn elements(config: &CardConfig) -> Vec<ChromeElement> {
        config.effective_chrome_slots().iter().map(|slot| slot.element).collect()
    }

    #[test]
    fn legacy_default_chrome_is_title_only() {
        let config = CardConfig::default();
        assert_eq!(elements(&config), vec![ChromeElement::Title]);
        assert!(config.effective_chrome_slots().iter().all(|slot| slot.placement == ChromePlacement::Inline));
    }

    #[test]
    fn legacy_flags_derive_full_chrome() {
        let config = CardConfig {
            drag_handle: DragHandle::Handle,
            foldable: true,
            closable: true,
            resizable: true,
            rotate_grip: true,
            ..Default::default()
        };
        assert_eq!(
            elements(&config),
            vec![
                ChromeElement::DragHandle,
                ChromeElement::Title,
                ChromeElement::Minimize,
                ChromeElement::Close,
                ChromeElement::ResizeGrip,
                ChromeElement::RotateGrip,
            ]
        );
    }

    #[test]
    fn drag_handle_header_mode_adds_no_handle_slot() {
        let config = CardConfig {
            drag_handle: DragHandle::Header,
            shape: CardShape::default(),
            content: CardContent::default(),
            arc_span: None,
            arc_bottom_span: None,
            hole_ratio: None,
            ..Default::default()
        };
        assert!(!elements(&config).contains(&ChromeElement::DragHandle));
    }

    #[test]
    fn explicit_chrome_overrides_legacy_flags() {
        let config = CardConfig {
            // Legacy flags ask for everything; the explicit list wins.
            foldable: true,
            closable: true,
            resizable: true,
            chrome: Some(vec![ChromeSlot::inline(ChromeElement::Close)]),
            ..Default::default()
        };
        assert_eq!(elements(&config), vec![ChromeElement::Close]);
    }

    #[test]
    fn explicit_chrome_accepts_satellite_placement() {
        let config = CardConfig {
            chrome: Some(vec![ChromeSlot::satellite(ChromeElement::Minimize)]),
            ..Default::default()
        };
        assert_eq!(config.effective_chrome_slots()[0].placement, ChromePlacement::Satellite);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn old_config_without_chrome_fields_deserializes() {
        // A legacy serialization lacks `chrome`/`chrome_style` - emulate
        // it by serializing a current config and stripping the new keys.
        let source = CardConfig {
            title: Some("Hi".into()),
            foldable: true,
            closable: true,
            ..Default::default()
        };
        let mut value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
        if let Some(object) = value.as_object_mut() {
            object.remove("chrome");
            object.remove("chrome_style");
        }
        let config: CardConfig = serde_json::from_value(value).unwrap();
        assert!(config.chrome.is_none());
        assert_eq!(config.chrome_style, ChromeStyle::Builtin);
        assert_eq!(elements(&config), vec![ChromeElement::Title, ChromeElement::Minimize, ChromeElement::Close]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn chrome_slots_roundtrip() {
        let config = CardConfig {
            chrome: Some(vec![
                ChromeSlot::inline(ChromeElement::Title),
                ChromeSlot::satellite(ChromeElement::Minimize),
                ChromeSlot::inline(ChromeElement::Close),
            ]),
            chrome_style: ChromeStyle::Adwaita,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: CardConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.effective_chrome_slots(), config.effective_chrome_slots());
        assert_eq!(back.chrome_style, ChromeStyle::Adwaita);
    }

    #[test]
    fn satellite_slots_and_config_merge_in_order() {
        let config = CardConfig {
            chrome: Some(vec![
                ChromeSlot::inline(ChromeElement::Title),
                ChromeSlot::satellite(ChromeElement::Minimize),
                ChromeSlot::satellite(ChromeElement::Close),
            ]),
            satellites: Some(CardSatellites {
                satellites: vec![SatelliteConfig::chrome("extra", ChromeElement::Minimize)],
                ..Default::default()
            }),
            ..Default::default()
        };
        let (_, satellites) = config.effective_satellites();
        let ids: Vec<&str> = satellites.iter().map(|satellite| satellite.id.as_str()).collect();
        assert_eq!(ids, vec!["chrome-minimize", "chrome-close", "extra"]);
    }

    /// Every satellite-capable chrome element maps to a distinct
    /// generated id - a shared suffix would make placements collide
    /// at mount time.
    #[test]
    fn satellite_capable_elements_get_unique_ids() {
        let capable = [
            ChromeElement::Minimize,
            ChromeElement::Close,
            ChromeElement::RotateClockwise,
            ChromeElement::RotateCounterClockwise,
            ChromeElement::NextSeat,
            ChromeElement::PreviousSeat,
        ];
        let config = CardConfig {
            chrome: Some(capable.iter().map(|element| ChromeSlot::satellite(*element)).collect()),
            ..Default::default()
        };
        let (_, satellites) = config.effective_satellites();
        assert_eq!(satellites.len(), capable.len());
        let mut ids: Vec<String> = satellites.iter().map(|satellite| satellite.id.as_str().to_string()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), capable.len());
    }

    /// On `CardShape::Circle` every chrome element mounts as a
    /// satellite: the slot list promotes inline placements and
    /// `effective_satellites` keeps only the satellite-capable ones
    /// (header elements and grips are dropped).
    #[test]
    fn circle_card_keeps_inline_chrome_out_of_satellites() {
        // On `Circle`, inline slots mount in the disc's top-arc band
        // (`mount_arc_chrome`) - only explicit `Satellite` placements
        // orbit.
        let config = CardConfig {
            shape: CardShape::Circle,
            chrome: Some(vec![
                ChromeSlot::inline(ChromeElement::Title),
                ChromeSlot::inline(ChromeElement::Minimize),
                ChromeSlot::inline(ChromeElement::Close),
                ChromeSlot::satellite(ChromeElement::RotateClockwise),
            ]),
            ..Default::default()
        };
        let (_, satellites) = config.effective_satellites();
        let ids: Vec<&str> = satellites.iter().map(|satellite| satellite.id.as_str()).collect();
        assert_eq!(ids, vec!["chrome-rotate-clockwise"]);
    }

    #[test]
    fn non_satellite_capable_elements_are_filtered() {
        let config = CardConfig {
            chrome: Some(vec![ChromeSlot::satellite(ChromeElement::ResizeGrip)]),
            ..Default::default()
        };
        let (_, satellites) = config.effective_satellites();
        assert!(satellites.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn chrome_slot_serializes_kebab_case() {
        let slot = ChromeSlot::satellite(ChromeElement::RotateGrip);
        let json = serde_json::to_string(&slot).unwrap();
        assert_eq!(json, r#"{"element":"rotate-grip","placement":"satellite"}"#);
    }
}
