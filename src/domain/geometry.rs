/// Rotates a vector by `degrees` (clockwise, screen coordinates).
pub fn rotate_vector(x: f64, y: f64, degrees: f64) -> (f64, f64) {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    (x * cos - y * sin, x * sin + y * cos)
}

/// Returns the axis-aligned bounding box of a `width`x`height` rectangle
/// rotated by `degrees` around its center.
pub fn rotated_bounds(width: f64, height: f64, degrees: f64) -> (f64, f64) {
    let radians = degrees.to_radians();
    let abs_cos = radians.cos().abs();
    let abs_sin = radians.sin().abs();
    (width * abs_cos + height * abs_sin, width * abs_sin + height * abs_cos)
}

/// Rasterizes a `width`x`height` rectangle rotated by `degrees` around its
/// center into horizontal scanline strips.
///
/// Returns `(y, x_start, x_end)` triples relative to the rectangle's center;
/// each strip is one pixel high. The widget layer maps these onto
/// `cairo::RectangleInt`s to build the rotated card footprint region.
pub fn rotated_region_scanlines(width: f64, height: f64, degrees: f64) -> Vec<(i32, i32, i32)> {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let corners = [
        (-half_width, -half_height),
        (half_width, -half_height),
        (half_width, half_height),
        (-half_width, half_height),
    ];
    let mut transformed = Vec::with_capacity(4);
    for (x, y) in corners {
        let (rx, ry) = rotate_vector(x, y, degrees);
        transformed.push((rx as f32, ry as f32));
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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    #[test]
    fn rotate_vector_quarter_turns() {
        let (x, y) = rotate_vector(1.0, 0.0, 90.0);
        assert!((x - 0.0).abs() < EPSILON && (y - 1.0).abs() < EPSILON);
        let (x, y) = rotate_vector(1.0, 0.0, 180.0);
        assert!((x + 1.0).abs() < EPSILON && y.abs() < EPSILON);
        let (x, y) = rotate_vector(0.0, 1.0, 270.0);
        assert!((x - 1.0).abs() < EPSILON && y.abs() < EPSILON);
    }

    #[test]
    fn rotated_bounds_axis_aligned() {
        let (width, height) = rotated_bounds(100.0, 50.0, 0.0);
        assert!((width - 100.0).abs() < EPSILON && (height - 50.0).abs() < EPSILON);
    }

    #[test]
    fn rotated_bounds_quarter_turn_swaps_axes() {
        let (width, height) = rotated_bounds(100.0, 50.0, 90.0);
        assert!((width - 50.0).abs() < EPSILON && (height - 100.0).abs() < EPSILON);
    }

    #[test]
    fn rotated_bounds_fortyfive_degrees() {
        let (width, height) = rotated_bounds(100.0, 100.0, 45.0);
        let expected = 100.0 * std::f64::consts::SQRT_2;
        assert!((width - expected).abs() < 0.01 && (height - expected).abs() < 0.01);
    }

    #[test]
    fn scanlines_cover_unrotated_rect() {
        let strips = rotated_region_scanlines(10.0, 6.0, 0.0);
        assert_eq!(strips.len(), 6);
        for (y, x_start, x_end) in &strips {
            assert_eq!(x_end - x_start, 10);
            assert!(*y >= -3 && *y < 3);
        }
    }

    #[test]
    fn scanlines_rotated_covers_diamond() {
        let strips = rotated_region_scanlines(10.0, 10.0, 45.0);
        assert!(!strips.is_empty());
        // The middle strip of a 45 degree square spans the full diagonal.
        let widest = strips.iter().map(|(_, a, b)| b - a).max().unwrap_or(0);
        assert!((13..=15).contains(&widest));
    }
}
