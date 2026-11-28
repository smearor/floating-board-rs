//! Zone-packed chrome placement: the arc bands and straight bars of
//! the card layouts share one placement→zone table (a header bar's
//! start/center/end packing, once per band) - [`ChromeSlots`] owns
//! the item list and exposes the three geometry passes.

use crate::domain::ChromeElement;
use crate::domain::ChromePlacement;
use crate::domain::ChromeSlot;
use gtk4::Widget;
use smearor_commons_animation::Interpolate;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::RotationRad;

/// Default arc-chrome band span - the disc's top quarter (±45°).
const ARC_HALF_SPAN: Radians = Radians::new(std::f64::consts::FRAC_PI_4);
/// Angular distance kept around a centered arc `Title` at the default
/// span (rad); `arc_angle` scales the gap with the configured span.
const ARC_TITLE_GAP: Radians = Radians::new(std::f64::consts::PI / 15.0);

/// A mounted chrome item list exposing the zone-packed placements the
/// card layouts share. Zones work like a header bar's: `*Start` items
/// pack from the band's start edge inward, `*End` items from its end
/// edge inward and the rest spread over the center - six zones in
/// slotted-band order `[top start, top center, top end, bottom start,
/// bottom center, bottom end]`, the bottom band mirrored so its
/// "start" edge sits screen-right (the user's left when facing the
/// card's bottom).
pub(crate) struct ChromeSlots {
    /// The mounted items' slots in slotted order.
    slots: Vec<ChromeSlot>,
}

impl From<&[(Widget, ChromeSlot)]> for ChromeSlots {
    /// The slot list of mounted chrome `(widget, slot)` pairs.
    fn from(items: &[(Widget, ChromeSlot)]) -> Self {
        Self {
            slots: items.iter().map(|(_, slot)| *slot).collect(),
        }
    }
}

impl From<&[ChromeSlot]> for ChromeSlots {
    /// The slot list itself.
    fn from(slots: &[ChromeSlot]) -> Self {
        Self { slots: slots.to_vec() }
    }
}

impl ChromeSlots {
    /// Item indices per zone in band order `[top start..end, bottom
    /// start..end]` - the single placement→zone table every geometry
    /// pass shares.
    fn zones(&self) -> [Vec<usize>; 6] {
        let mut zones: [Vec<usize>; 6] = Default::default();
        for (index, slot) in self.slots.iter().enumerate() {
            match slot.placement {
                ChromePlacement::InlineStart | ChromePlacement::ArcTopStart => zones[0].push(index),
                ChromePlacement::Inline | ChromePlacement::ArcTop => zones[1].push(index),
                ChromePlacement::InlineEnd | ChromePlacement::ArcTopEnd => zones[2].push(index),
                ChromePlacement::BarStart => zones[3].push(index),
                ChromePlacement::Bar => zones[4].push(index),
                ChromePlacement::BarEnd => zones[5].push(index),
                ChromePlacement::Satellite => {}
            }
        }
        zones
    }

    /// Disc-direction per arc-chrome item (radians, 0 = the disc's
    /// top, positive clockwise): the top band packs items with
    /// `Start` placements edge-inward on its left end, spreads
    /// `ArcTop`/`Inline` items over the center zone and packs `End`
    /// items edge-inward on its right end - header-bar zones on an
    /// arc. `Bar*` items land on the mirrored bottom band (offset π:
    /// its "start" edge sits on the screen's right, the user's left
    /// when facing the card's bottom). Each band spans its configured
    /// `arc_span` / `arc_bottom_span`. A top band of pure `Inline`
    /// slots keeps the legacy title-centered spread over the whole
    /// band.
    pub(crate) fn arc_angles(&self, top_half_span: Radians, bottom_half_span: Radians) -> Vec<RotationRad> {
        let top_gap = top_half_span * (ARC_TITLE_GAP / ARC_HALF_SPAN);
        let bottom_gap = bottom_half_span * (ARC_TITLE_GAP / ARC_HALF_SPAN);
        let [top_start, top_center, top_end, bar_start, bar_center, bar_end] = self.zones();
        let mut angles = vec![RotationRad::ZERO; self.slots.len()];
        // The legacy path: an unpinned top band of `Inline` items
        // packs around a centered title over the whole band.
        let legacy = top_start.is_empty() && top_end.is_empty() && top_center.iter().all(|&index| self.slots[index].placement == ChromePlacement::Inline);
        if legacy && !top_center.is_empty() {
            let title_index = top_center.iter().position(|&index| self.slots[index].element == ChromeElement::Title);
            for (position, &item) in top_center.iter().enumerate() {
                angles[item] = arc_angle(position, top_center.len(), title_index, top_half_span);
            }
        } else {
            for (position, &item) in top_start.iter().enumerate() {
                angles[item] = spread(RotationRad::from(-top_half_span), RotationRad::from(-top_gap), position, top_start.len());
            }
            for (position, &item) in top_center.iter().enumerate() {
                angles[item] = if top_center.len() == 1 {
                    RotationRad::ZERO
                } else {
                    spread(RotationRad::from(-top_gap), RotationRad::from(top_gap), position, top_center.len())
                };
            }
            for (position, &item) in top_end.iter().enumerate() {
                angles[item] = spread(RotationRad::from(top_half_span), RotationRad::from(top_gap), position, top_end.len());
            }
        }
        let bottom = RotationRad::HALF_TURN;
        for (position, &item) in bar_start.iter().enumerate() {
            angles[item] = bottom + spread(RotationRad::from(-bottom_half_span), RotationRad::from(-bottom_gap), position, bar_start.len());
        }
        for (position, &item) in bar_center.iter().enumerate() {
            angles[item] = bottom
                + if bar_center.len() == 1 {
                    RotationRad::ZERO
                } else {
                    spread(RotationRad::from(-bottom_gap), RotationRad::from(bottom_gap), position, bar_center.len())
                };
        }
        for (position, &item) in bar_end.iter().enumerate() {
            angles[item] = bottom + spread(RotationRad::from(bottom_half_span), RotationRad::from(bottom_gap), position, bar_end.len());
        }
        angles
    }

    /// X-offsets (px, relative to the bar's middle) per item in one
    /// straight-bar band: `zone` selects the band's three zones,
    /// `start_sign` the edge `*Start` packs from - `+1` on the
    /// mirrored bottom bar (its start edge sits screen-right), `-1`
    /// on the unmirrored top bar (start = screen-left). `half_width`
    /// is the safe half-extent of the bar at its mid line.
    fn straight_offsets(&self, zone: usize, half_width: f64, start_sign: f64) -> Vec<f64> {
        let gap = half_width * (ARC_TITLE_GAP / ARC_HALF_SPAN);
        let zones = self.zones();
        let [start, center, end] = [&zones[zone], &zones[zone + 1], &zones[zone + 2]];
        let mut offsets = vec![0.0; self.slots.len()];
        for (position, &item) in start.iter().enumerate() {
            offsets[item] = spread_line(start_sign * half_width, start_sign * gap, position, start.len());
        }
        for (position, &item) in center.iter().enumerate() {
            offsets[item] = if center.len() == 1 {
                0.0
            } else {
                spread_line(-gap, gap, position, center.len())
            };
        }
        for (position, &item) in end.iter().enumerate() {
            offsets[item] = spread_line(-start_sign * half_width, -start_sign * gap, position, end.len());
        }
        offsets
    }

    /// X-offsets (px, relative to the dome's bottom middle) per item
    /// on a `Semicircle` card's straight bottom bar: `BarStart` packs
    /// from the bar's "start" edge inward - screen-right, the user's
    /// left when facing the card's bottom edge, the same convention
    /// the bottom arc uses - `Bar` spreads over the center span and
    /// `BarEnd` packs from the left. `half_width` is the safe
    /// half-extent of the bar (the dome's chord at the bar's top
    /// edge).
    pub(crate) fn bar_offsets(&self, half_width: f64) -> Vec<f64> {
        self.straight_offsets(3, half_width, 1.0)
    }

    /// X-offsets (px, relative to the hexagon's top middle) per item
    /// on a `Hexagon` card's straight top bar: `InlineStart`/
    /// `ArcTopStart` pack from the bar's left edge inward -
    /// screen-left, the user's left when facing the card's top
    /// edge - `Inline`/`ArcTop` spread over the center span and
    /// `InlineEnd`/`ArcTopEnd` pack from the right. The unmirrored
    /// counterpart of [`ChromeSlots::bar_offsets`]: the top edge
    /// reads left-to-right. `half_width` is the safe half-extent of
    /// the bar (the hexagon's cross-section at the bar's mid line).
    pub(crate) fn top_offsets(&self, half_width: f64) -> Vec<f64> {
        self.straight_offsets(0, half_width, -1.0)
    }
}

/// Evenly spreads `count` directions over `[from, to]`; a lone item
/// sits at `from` - groups pack from the band's outer edge inward,
/// like a header bar's packed ends. The shortest-arc `interpolate`
/// never sweeps a span the long way around.
fn spread(from: RotationRad, to: RotationRad, index: usize, count: usize) -> RotationRad {
    let t = Fraction::new(if count <= 1 { 0.0 } else { index as f64 / (count - 1) as f64 });
    from.interpolate(to, t)
}

/// Angular position of arc-chrome item `index` of `count` in the
/// legacy all-`Inline` packing (the band spanning `±half_span`):
/// items before `title_index` pack the left half of the band from its
/// outer edge inward, a `Title` item sits centered like a header
/// bar's title, and items after pack the right half from its outer
/// edge inward - the header bar's end-pack semantics. Without a title
/// the whole set spreads evenly over the band.
fn arc_angle(index: usize, count: usize, title_index: Option<usize>, half_span: Radians) -> RotationRad {
    let title_gap = half_span * (ARC_TITLE_GAP / ARC_HALF_SPAN);
    let Some(title) = title_index else {
        return if count <= 1 {
            RotationRad::ZERO
        } else {
            spread(RotationRad::from(-half_span), RotationRad::from(half_span), index, count)
        };
    };
    if index == title {
        RotationRad::ZERO
    } else if index < title {
        spread(RotationRad::from(-half_span), RotationRad::from(-title_gap), index, title)
    } else {
        spread(RotationRad::from(half_span), RotationRad::from(title_gap), index - title - 1, count - title - 1)
    }
}

/// Evenly spreads `count` positions over `[from, to]`; a lone item
/// sits at `from` - groups pack from the bar's outer edge inward,
/// matching the arc bands' header-bar semantics.
fn spread_line(from: f64, to: f64, index: usize, count: usize) -> f64 {
    from.interpolate(to, Fraction::new(if count <= 1 { 0.0 } else { index as f64 / (count - 1) as f64 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direction comparisons go through `sin_cos` - the normalized
    /// scalar wraps across the turn boundary.
    fn assert_angle(actual: RotationRad, expected_degrees: f64) {
        let (actual_sin, actual_cos) = actual.sin_cos();
        let expected = RotationRad::new(expected_degrees.to_radians());
        let (expected_sin, expected_cos) = expected.sin_cos();
        assert!(
            (actual_sin - expected_sin).abs() < 1e-9 && (actual_cos - expected_cos).abs() < 1e-9,
            "{actual:?} != {expected_degrees}deg"
        );
    }

    fn slots(placements: &[ChromePlacement]) -> Vec<ChromeSlot> {
        placements
            .iter()
            .map(|placement| ChromeSlot {
                element: ChromeElement::Minimize,
                placement: *placement,
            })
            .collect()
    }

    #[test]
    fn arc_angle_single_item_centers() {
        assert_angle(arc_angle(0, 1, None, ARC_HALF_SPAN), 0.0);
    }

    #[test]
    fn arc_angle_spreads_over_band_without_title() {
        assert_angle(arc_angle(0, 3, None, ARC_HALF_SPAN), -45.0);
        assert_angle(arc_angle(1, 3, None, ARC_HALF_SPAN), 0.0);
        assert_angle(arc_angle(2, 3, None, ARC_HALF_SPAN), 45.0);
    }

    #[test]
    fn arc_angle_title_pins_center() {
        assert_angle(arc_angle(1, 3, Some(1), ARC_HALF_SPAN), 0.0);
        assert_angle(arc_angle(0, 3, Some(1), ARC_HALF_SPAN), -45.0);
        assert_angle(arc_angle(2, 3, Some(1), ARC_HALF_SPAN), 45.0);
    }

    #[test]
    fn arc_angle_groups_around_title() {
        // [A, B, Title, C, D] - each side packs outer-edge inward, so
        // the first slot of each group sits at the band's end.
        assert_angle(arc_angle(0, 5, Some(2), ARC_HALF_SPAN), -45.0);
        assert_angle(arc_angle(1, 5, Some(2), ARC_HALF_SPAN), -12.0);
        assert_angle(arc_angle(3, 5, Some(2), ARC_HALF_SPAN), 45.0);
        assert_angle(arc_angle(4, 5, Some(2), ARC_HALF_SPAN), 12.0);
    }

    #[test]
    fn arc_angles_zones_pack_edges() {
        let angles = ChromeSlots::from(slots(&[ChromePlacement::ArcTopStart, ChromePlacement::ArcTop, ChromePlacement::ArcTopEnd]).as_slice())
            .arc_angles(ARC_HALF_SPAN, ARC_HALF_SPAN);
        assert_angle(angles[0], -45.0);
        assert_angle(angles[1], 0.0);
        assert_angle(angles[2], 45.0);
    }

    #[test]
    fn arc_angles_inline_ends_map_to_zones() {
        let angles = ChromeSlots::from(slots(&[ChromePlacement::InlineStart, ChromePlacement::InlineEnd]).as_slice()).arc_angles(ARC_HALF_SPAN, ARC_HALF_SPAN);
        assert_angle(angles[0], -45.0);
        assert_angle(angles[1], 45.0);
    }

    #[test]
    fn arc_angles_bottom_band_is_mirrored() {
        // Bottom band: the "start" edge sits on the screen's right -
        // the user's left when facing the card's bottom.
        let angles = ChromeSlots::from(slots(&[ChromePlacement::BarStart, ChromePlacement::Bar, ChromePlacement::BarEnd]).as_slice())
            .arc_angles(ARC_HALF_SPAN, ARC_HALF_SPAN);
        assert_angle(angles[0], 135.0);
        assert_angle(angles[1], 180.0);
        assert_angle(angles[2], 225.0);
    }

    #[test]
    fn arc_angles_bottom_band_uses_own_span() {
        // A narrow bottom band: start/end pack its 15deg half-span.
        let angles = ChromeSlots::from(slots(&[ChromePlacement::BarStart, ChromePlacement::BarEnd]).as_slice())
            .arc_angles(ARC_HALF_SPAN, Radians::new(std::f64::consts::PI / 12.0));
        assert_angle(angles[0], 165.0);
        assert_angle(angles[1], 195.0);
    }

    #[test]
    fn arc_angles_mixed_inline_plus_zone_uses_zones() {
        // One pinned slot switches the whole top band to zone packing:
        // remaining `Inline` items share the center zone.
        let angles = ChromeSlots::from(slots(&[ChromePlacement::Inline, ChromePlacement::Inline, ChromePlacement::InlineEnd]).as_slice())
            .arc_angles(ARC_HALF_SPAN, ARC_HALF_SPAN);
        assert_angle(angles[0], -12.0);
        assert_angle(angles[1], 12.0);
        assert_angle(angles[2], 45.0);
    }
}
