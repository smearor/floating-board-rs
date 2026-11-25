use smearor_commons_geometry::Fraction;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

/// One animated axis between a start and an end value, with a
/// sinusoidal `excursion` on top that peaks at the eased half-way
/// point (`sin(pi * t)` envelope: zero at both ends). An axis whose
/// `start` and `end` agree still sweeps through `excursion` - the
/// card's width can dip mid-flip although both states have the same
/// width.
///
/// The axis is generic over its value type `T`: footprint axes
/// animate `f64` px components, the rotation axis animates
/// [`smearor_commons_geometry::Radians`]. `T` must support the
/// lerp-plus-excursion arithmetic (`Add`/`Sub`/`Mul<f64>`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldAxis<T> {
    /// Axis value at transition start (`t == 0`).
    pub start: T,
    /// Axis value at transition end (`t == 1`).
    pub end: T,
    /// Signed deviation added at the half-way point. Positive values
    /// overshoot, negative values dip.
    pub excursion: T,
}

impl<T> FoldAxis<T> {
    /// An axis from `start` to `end` with `excursion` added mid-way
    /// (`zero` in the axis' unit = a plain linear lerp).
    pub const fn new(start: T, end: T, excursion: T) -> Self {
        Self { start, end, excursion }
    }
}

impl<T> FoldAxis<T>
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f64, Output = T>,
{
    /// The axis value at eased progress `t` (`Fraction::ZERO` =
    /// start, `Fraction::ONE` = end). The ends snap to `start`/`end`
    /// exactly - `sin(pi)` is only approximately zero, and a handoff
    /// frame must land precisely.
    pub fn value(&self, t: Fraction) -> T {
        if t <= Fraction::ZERO {
            return self.start;
        }
        if t >= Fraction::ONE {
            return self.end;
        }
        let t = t.as_f64();
        self.start + (self.end - self.start) * t + self.excursion * (std::f64::consts::PI * t).sin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_without_excursion_interpolates_linearly() {
        let axis = FoldAxis::new(10.0, 20.0, 0.0);
        assert_eq!(axis.value(Fraction::ZERO), 10.0);
        assert_eq!(axis.value(Fraction::HALF), 15.0);
        assert_eq!(axis.value(Fraction::ONE), 20.0);
    }

    #[test]
    fn axis_with_equal_ends_still_sweeps_excursion() {
        let axis = FoldAxis::new(100.0, 100.0, -40.0);
        assert_eq!(axis.value(Fraction::ZERO), 100.0);
        assert_eq!(axis.value(Fraction::HALF), 60.0);
        assert_eq!(axis.value(Fraction::ONE), 100.0);
        for index in 0..=20 {
            let t = Fraction::new(index as f64 / 20.0);
            assert!(axis.value(t) <= 100.0, "excursion must dip below the base at t={t:?}");
        }
    }
}
