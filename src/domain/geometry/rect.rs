use crate::domain::BoardPoint;
use crate::domain::CardPosition;
use crate::domain::MinVisible;

/// Axis-aligned rectangle in board coordinates (pixels).
///
/// Used for card footprints and board bounds in the pure geometry math
/// (`anchor`, `clamp`) - deliberately independent of `graphene::Rect` so the
/// domain stays GTK-free.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    /// Left edge x-coordinate.
    pub x: f64,
    /// Top edge y-coordinate.
    pub y: f64,
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

impl Rect {
    /// Creates a rectangle from top-left corner and size.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    /// Creates the rectangle of the given size centered at `center`.
    pub fn centered(center: CardPosition, width: f64, height: f64) -> Self {
        Self {
            x: center.x() - width / 2.0,
            y: center.y() - height / 2.0,
            width,
            height,
        }
    }

    /// Right edge x-coordinate.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Bottom edge y-coordinate.
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Center point of the rectangle.
    pub fn center(&self) -> CardPosition {
        CardPosition::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Returns whether the rectangle has a positive area.
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Returns the rectangle translated by the given offset.
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }

    /// Returns the axis-aligned bounding box of this rectangle's size
    /// rotated by `degrees` around its center (position is irrelevant for
    /// the bounding box, so only width/height contribute).
    pub fn rotated_bounds(&self, degrees: f64) -> (f64, f64) {
        let radians = degrees.to_radians();
        let abs_cos = radians.cos().abs();
        let abs_sin = radians.sin().abs();
        (self.width * abs_cos + self.height * abs_sin, self.width * abs_sin + self.height * abs_cos)
    }

    /// Rasterizes this rectangle's size rotated by `degrees` around its
    /// center into horizontal scanline strips.
    ///
    /// Returns `(y, x_start, x_end)` triples relative to the rectangle's
    /// center; each strip is one pixel high. The widget layer maps these
    /// onto `cairo::RectangleInt`s to build the rotated card footprint
    /// region.
    pub fn rotated_region_scanlines(&self, degrees: f64) -> Vec<(i32, i32, i32)> {
        let half_width = self.width / 2.0;
        let half_height = self.height / 2.0;
        let corners = [
            (-half_width, -half_height),
            (half_width, -half_height),
            (half_width, half_height),
            (-half_width, half_height),
        ];
        let mut transformed = Vec::with_capacity(4);
        for (x, y) in corners {
            let rotated = BoardPoint::new(x, y).rotated(degrees);
            transformed.push((rotated.x() as f32, rotated.y() as f32));
        }
        let min_y = transformed.iter().map(|(_, y)| *y).fold(f32::INFINITY, f32::min).floor() as i32;
        let max_y = transformed.iter().map(|(_, y)| *y).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
        let mut strips = Vec::with_capacity((max_y - min_y).max(0) as usize);
        for y in min_y..max_y {
            let current_y = y as f32 + 0.5;
            let mut intersections = Vec::with_capacity(4);
            for i in 0..4 {
                let p1 = transformed[i];
                let p2 = transformed[(i + 1) % 4];
                if (p1.1 <= current_y && p2.1 > current_y) || (p2.1 <= current_y && p1.1 > current_y) {
                    intersections.push(p1.0 + (current_y - p1.1) * (p2.0 - p1.0) / (p2.1 - p1.1));
                }
            }
            if intersections.len() >= 2 {
                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                strips.push((y, intersections[0].round() as i32, intersections[intersections.len() - 1].round() as i32));
            }
        }
        strips
    }

    /// Computes the minimal translation `(dx, dy)` needed so that at least
    /// `min_visible` pixels of this rectangle overlap `bounds` on each
    /// axis.
    ///
    /// Applying the returned offset to the card's position moves the
    /// footprint back inside the board. The overlap requirement is capped
    /// at half the bounds size so it stays satisfiable on very small
    /// boards.
    pub fn clamp_offset(&self, bounds: &Rect, min_visible: MinVisible) -> (f64, f64) {
        (
            clamp_axis(self.x, self.width, bounds.x, bounds.width, min_visible),
            clamp_axis(self.y, self.height, bounds.y, bounds.height, min_visible),
        )
    }

    /// Computes the minimal translation `(dx, dy)` needed so that this
    /// rectangle lies entirely inside `bounds` - the clamp for
    /// `BoundsMode::Contained`.
    ///
    /// On axes where the rectangle is larger than the bounds it is
    /// centered, so the card stays reachable instead of snapping to one
    /// edge.
    pub fn contain_offset(&self, bounds: &Rect) -> (f64, f64) {
        (
            contain_axis(self.x, self.width, bounds.x, bounds.width),
            contain_axis(self.y, self.height, bounds.y, bounds.height),
        )
    }
}

/// Per-axis part of [`Rect::clamp_offset`]: clamps the rectangle's start
/// coordinate so it keeps `min_visible` pixels inside the bounds, capped at
/// half the bounds size.
fn clamp_axis(rect_min: f64, rect_size: f64, bounds_min: f64, bounds_size: f64, min_visible: MinVisible) -> f64 {
    let overlap = min_visible.value().min(bounds_size / 2.0);
    let allowed_min = bounds_min + overlap - rect_size;
    let allowed_max = bounds_min + bounds_size - overlap;
    rect_min.clamp(allowed_min, allowed_max) - rect_min
}

/// Per-axis part of [`Rect::contain_offset`]: clamps the rectangle's start
/// coordinate fully inside the bounds, or centers it when larger.
fn contain_axis(rect_min: f64, rect_size: f64, bounds_min: f64, bounds_size: f64) -> f64 {
    if rect_size >= bounds_size {
        return bounds_min + (bounds_size - rect_size) / 2.0 - rect_min;
    }
    rect_min.clamp(bounds_min, bounds_min + bounds_size - rect_size) - rect_min
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    #[test]
    fn rect_edges_and_center() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.bottom(), 70.0);
        assert_eq!(rect.center(), CardPosition::new(60.0, 45.0));
    }

    #[test]
    fn rect_centered_roundtrip() {
        let center = CardPosition::new(200.0, 100.0);
        let rect = Rect::centered(center, 80.0, 40.0);
        assert_eq!(rect.center(), center);
        assert_eq!(rect, Rect::new(160.0, 80.0, 80.0, 40.0));
    }

    #[test]
    fn rotated_bounds_axis_aligned() {
        let (width, height) = Rect::new(0.0, 0.0, 100.0, 50.0).rotated_bounds(0.0);
        assert!((width - 100.0).abs() < EPSILON && (height - 50.0).abs() < EPSILON);
    }

    #[test]
    fn rotated_bounds_quarter_turn_swaps_axes() {
        let (width, height) = Rect::new(0.0, 0.0, 100.0, 50.0).rotated_bounds(90.0);
        assert!((width - 50.0).abs() < EPSILON && (height - 100.0).abs() < EPSILON);
    }

    #[test]
    fn rotated_bounds_fortyfive_degrees() {
        let (width, height) = Rect::new(0.0, 0.0, 100.0, 100.0).rotated_bounds(45.0);
        let expected = 100.0 * std::f64::consts::SQRT_2;
        assert!((width - expected).abs() < 0.01 && (height - expected).abs() < 0.01);
    }

    #[test]
    fn scanlines_cover_unrotated_rect() {
        let strips = Rect::new(0.0, 0.0, 10.0, 6.0).rotated_region_scanlines(0.0);
        assert_eq!(strips.len(), 6);
        for (y, x_start, x_end) in &strips {
            assert_eq!(x_end - x_start, 10);
            assert!(*y >= -3 && *y < 3);
        }
    }

    #[test]
    fn scanlines_rotated_covers_diamond() {
        let strips = Rect::new(0.0, 0.0, 10.0, 10.0).rotated_region_scanlines(45.0);
        assert!(!strips.is_empty());
        // The middle strip of a 45 degree square spans the full diagonal.
        let widest = strips.iter().map(|(_, a, b)| b - a).max().unwrap_or(0);
        assert!((13..=15).contains(&widest));
    }

    const BOUNDS: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1000.0,
        height: 800.0,
    };
    const MIN_VISIBLE_TEST: MinVisible = MinVisible(40.0);

    #[test]
    fn fully_inside_footprint_is_untouched() {
        let footprint = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(footprint.clamp_offset(&BOUNDS, MIN_VISIBLE_TEST), (0.0, 0.0));
    }

    #[test]
    fn footprint_pushed_back_from_right_edge() {
        // Only 10 px of the card remain inside on the right edge.
        let footprint = Rect::new(990.0, 100.0, 200.0, 100.0);
        let (dx, dy) = footprint.clamp_offset(&BOUNDS, MIN_VISIBLE_TEST);
        assert_eq!(dy, 0.0);
        assert_eq!(dx, -30.0); // 40 px of the card stay inside the bounds
    }

    #[test]
    fn footprint_pushed_back_from_left_edge() {
        let footprint = Rect::new(-190.0, 100.0, 200.0, 100.0);
        let (dx, _) = footprint.clamp_offset(&BOUNDS, MIN_VISIBLE_TEST);
        assert_eq!(dx, 30.0); // 40 px of the card stay inside the bounds
    }

    #[test]
    fn footprint_pushed_back_from_bottom_edge() {
        let footprint = Rect::new(100.0, 785.0, 200.0, 100.0);
        let (_, dy) = footprint.clamp_offset(&BOUNDS, MIN_VISIBLE_TEST);
        assert_eq!(dy, -25.0); // 40 px of the card stay inside the bounds
    }

    #[test]
    fn oversized_footprint_is_still_clamped() {
        // Card wider than the board: only 30 px overlap on the right edge.
        let footprint = Rect::new(970.0, 100.0, 2000.0, 100.0);
        let (dx, _) = footprint.clamp_offset(&BOUNDS, MIN_VISIBLE_TEST);
        assert_eq!(dx, -10.0); // right edge pulled back to 1000 - 40
    }

    #[test]
    fn oversized_centered_footprint_is_untouched() {
        let footprint = Rect::new(-500.0, 100.0, 2000.0, 100.0);
        assert_eq!(footprint.clamp_offset(&BOUNDS, MIN_VISIBLE_TEST).0, 0.0);
    }

    #[test]
    fn contained_inside_footprint_is_untouched() {
        let footprint = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(footprint.contain_offset(&BOUNDS), (0.0, 0.0));
    }

    #[test]
    fn contained_right_edge_is_pulled_fully_inside() {
        let footprint = Rect::new(900.0, 100.0, 200.0, 100.0);
        assert_eq!(footprint.contain_offset(&BOUNDS), (-100.0, 0.0));
    }

    #[test]
    fn contained_left_edge_is_pushed_fully_inside() {
        let footprint = Rect::new(-50.0, 100.0, 200.0, 100.0);
        assert_eq!(footprint.contain_offset(&BOUNDS), (50.0, 0.0));
    }

    #[test]
    fn contained_oversized_footprint_is_centered() {
        let footprint = Rect::new(0.0, 100.0, 1200.0, 100.0);
        assert_eq!(footprint.contain_offset(&BOUNDS), (-100.0, 0.0));
    }
}
