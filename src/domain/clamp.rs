use crate::domain::MinVisible;
use crate::domain::Rect;

/// Computes the minimal translation `(dx, dy)` needed so that at least
/// `min_visible` pixels of `footprint` overlap `bounds` on each axis.
///
/// Applying the returned offset to the card's position moves the footprint
/// back inside the board. The overlap requirement is capped at half the
/// bounds size so it stays satisfiable on very small boards.
///
/// Pure function - unit-testable without GTK.
pub fn clamp_offset(footprint: &Rect, bounds: &Rect, min_visible: MinVisible) -> (f64, f64) {
    (
        clamp_axis(footprint.x, footprint.width, bounds.x, bounds.width, min_visible),
        clamp_axis(footprint.y, footprint.height, bounds.y, bounds.height, min_visible),
    )
}

fn clamp_axis(footprint_min: f64, footprint_size: f64, bounds_min: f64, bounds_size: f64, min_visible: MinVisible) -> f64 {
    let overlap = min_visible.value().min(bounds_size / 2.0);
    // footprint_min must keep `overlap` pixels inside the bounds on this axis.
    let allowed_min = bounds_min + overlap - footprint_size;
    let allowed_max = bounds_min + bounds_size - overlap;
    footprint_min.clamp(allowed_min, allowed_max) - footprint_min
}

/// Computes the minimal translation `(dx, dy)` needed so that `footprint`
/// lies entirely inside `bounds` - the clamp for `BoundsMode::Contained`.
///
/// On axes where the footprint is larger than the bounds it is centered, so
/// the card stays reachable instead of snapping to one edge.
///
/// Pure function - unit-testable without GTK.
pub fn contain_offset(footprint: &Rect, bounds: &Rect) -> (f64, f64) {
    (
        contain_axis(footprint.x, footprint.width, bounds.x, bounds.width),
        contain_axis(footprint.y, footprint.height, bounds.y, bounds.height),
    )
}

fn contain_axis(footprint_min: f64, footprint_size: f64, bounds_min: f64, bounds_size: f64) -> f64 {
    if footprint_size >= bounds_size {
        return bounds_min + (bounds_size - footprint_size) / 2.0 - footprint_min;
    }
    footprint_min.clamp(bounds_min, bounds_min + bounds_size - footprint_size) - footprint_min
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1000.0,
        height: 800.0,
    };
    const MIN_VISIBLE: MinVisible = MinVisible(40.0);

    #[test]
    fn fully_inside_footprint_is_untouched() {
        let footprint = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(clamp_offset(&footprint, &BOUNDS, MIN_VISIBLE), (0.0, 0.0));
    }

    #[test]
    fn footprint_pushed_back_from_right_edge() {
        // Only 10 px of the card remain inside on the right edge.
        let footprint = Rect::new(990.0, 100.0, 200.0, 100.0);
        let (dx, dy) = clamp_offset(&footprint, &BOUNDS, MIN_VISIBLE);
        assert_eq!(dy, 0.0);
        assert_eq!(dx, -30.0); // 40 px of the card stay inside the bounds
    }

    #[test]
    fn footprint_pushed_back_from_left_edge() {
        let footprint = Rect::new(-190.0, 100.0, 200.0, 100.0);
        let (dx, _) = clamp_offset(&footprint, &BOUNDS, MIN_VISIBLE);
        assert_eq!(dx, 30.0); // 40 px of the card stay inside the bounds
    }

    #[test]
    fn footprint_pushed_back_from_bottom_edge() {
        let footprint = Rect::new(100.0, 785.0, 200.0, 100.0);
        let (_, dy) = clamp_offset(&footprint, &BOUNDS, MIN_VISIBLE);
        assert_eq!(dy, -25.0); // 40 px of the card stay inside the bounds
    }

    #[test]
    fn oversized_footprint_is_still_clamped() {
        // Card wider than the board: only 30 px overlap on the right edge.
        let footprint = Rect::new(970.0, 100.0, 2000.0, 100.0);
        let (dx, _) = clamp_offset(&footprint, &BOUNDS, MIN_VISIBLE);
        assert_eq!(dx, -10.0); // right edge pulled back to 1000 - 40
    }

    #[test]
    fn oversized_centered_footprint_is_untouched() {
        let footprint = Rect::new(-500.0, 100.0, 2000.0, 100.0);
        assert_eq!(clamp_offset(&footprint, &BOUNDS, MIN_VISIBLE).0, 0.0);
    }

    #[test]
    fn contained_inside_footprint_is_untouched() {
        let footprint = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(contain_offset(&footprint, &BOUNDS), (0.0, 0.0));
    }

    #[test]
    fn contained_right_edge_is_pulled_fully_inside() {
        let footprint = Rect::new(900.0, 100.0, 200.0, 100.0);
        assert_eq!(contain_offset(&footprint, &BOUNDS), (-100.0, 0.0));
    }

    #[test]
    fn contained_left_edge_is_pushed_fully_inside() {
        let footprint = Rect::new(-50.0, 100.0, 200.0, 100.0);
        assert_eq!(contain_offset(&footprint, &BOUNDS), (50.0, 0.0));
    }

    #[test]
    fn contained_oversized_footprint_is_centered() {
        let footprint = Rect::new(0.0, 100.0, 1200.0, 100.0);
        assert_eq!(contain_offset(&footprint, &BOUNDS), (-100.0, 0.0));
    }
}
