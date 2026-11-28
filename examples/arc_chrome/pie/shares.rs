//! The `pie` face's shared slice state.

use super::color::PieColor;
use smearor_commons_geometry::Fraction;

/// No slice may drop below this share.
pub(super) const SLICE_FLOOR: Fraction = Fraction::new(0.10);
/// The share a slice satellite adds per click.
pub(super) const SLICE_STEP: Fraction = Fraction::new(0.05);

/// The pie's slice shares, summing to 1 - `Red` starts at 0 (no
/// slice) until its satellite's first click seeds it. Shared between
/// the face's draw func and the satellite buttons.
#[derive(Clone)]
pub(crate) struct PieShares {
    /// `(slot, share)` pairs in draw order - a zero share is absent.
    pub(crate) shares: [(PieColor, Fraction); 4],
}

impl PieShares {
    /// The starting pie: blue 40%, green 35%, yellow 25%, no red.
    pub(crate) const INITIAL: Self = Self {
        shares: [
            (PieColor::Blue, Fraction::new(0.40)),
            (PieColor::Green, Fraction::new(0.35)),
            (PieColor::Yellow, Fraction::new(0.25)),
            (PieColor::Red, Fraction::ZERO),
        ],
    };

    /// `slot`'s current share (`0` when absent).
    pub(crate) fn share(&self, slot: PieColor) -> Fraction {
        self.shares.iter().find(|(s, _)| *s == slot).map(|(_, share)| *share).unwrap_or(Fraction::ZERO)
    }

    /// Grows `slot` by `amount`, taking the room proportionally from
    /// the other slices' headroom above `SLICE_FLOOR`: donors lose in
    /// proportion to what they can still give, so no slice drops below
    /// the floor and growth caps at the total headroom. A zero-share
    /// slot only seeds when the full `amount` fits - a partial seed
    /// would undercut the floor.
    pub(crate) fn grow(&mut self, slot: PieColor, amount: Fraction) {
        let headroom = self
            .shares
            .iter()
            .filter(|(s, _)| *s != slot)
            .map(|(_, share)| (*share - SLICE_FLOOR).max(Fraction::ZERO))
            .fold(Fraction::ZERO, |sum, share| sum + share);
        if headroom <= Fraction::ZERO || (self.share(slot) <= Fraction::ZERO && headroom < amount) {
            return;
        }
        let taken = headroom.min(amount);
        for (s, share) in self.shares.iter_mut() {
            if *s == slot {
                *share = *share + taken;
            } else {
                *share = *share - taken * ((*share - SLICE_FLOOR).max(Fraction::ZERO) / headroom);
            }
        }
    }
}
