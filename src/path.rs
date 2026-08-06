// =============================================================================
// path.rs — the track the fruit walk along
//
// A polyline from the left edge to the right edge. Fruit store only how far they
// have travelled along it, so movement is a single scalar and `point_at` turns
// that back into a world position. Also answers "how close is this point to the
// track?", which is what gates tower placement.
// =============================================================================

use macroquad::prelude::*;

/// A polyline track with cached cumulative segment lengths.
pub struct Path {
    points: Vec<Vec2>,
    /// cum[i] is the distance from the start to points[i]; cum[0] is always 0.
    cum: Vec<f32>,
    total: f32,
}

impl Path {
    // ─────────────────────────────────────────────────────────────────────────
    // Build a path and precompute the cumulative length at every waypoint.
    // Needs at least two points.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn new(points: Vec<Vec2>) -> Self {
        assert!(points.len() >= 2, "a path needs at least two waypoints");

        let mut cum = Vec::with_capacity(points.len());
        let mut total = 0.0;
        cum.push(0.0);
        for w in points.windows(2) {
            total += w[0].distance(w[1]);
            cum.push(total);
        }

        Path { points, cum, total }
    }

    pub fn total(&self) -> f32 {
        self.total
    }

    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    // ─────────────────────────────────────────────────────────────────────────
    // World position at distance `d` along the track, clamped at both ends.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn point_at(&self, d: f32) -> Vec2 {
        if d <= 0.0 {
            return self.points[0];
        }
        if d >= self.total {
            return *self.points.last().unwrap();
        }

        // Find the segment containing d, then lerp across it.
        for i in 0..self.points.len() - 1 {
            if d <= self.cum[i + 1] {
                let seg_len = self.cum[i + 1] - self.cum[i];
                // Guard against zero-length segments from duplicate waypoints.
                let t = if seg_len > 0.0 {
                    (d - self.cum[i]) / seg_len
                } else {
                    0.0
                };
                return self.points[i].lerp(self.points[i + 1], t);
            }
        }

        *self.points.last().unwrap()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Shortest distance from `p` to the track. Used to keep towers off the path.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn distance_to(&self, p: Vec2) -> f32 {
        let mut best = f32::MAX;
        for w in self.points.windows(2) {
            best = best.min(point_segment_distance(p, w[0], w[1]));
        }
        best
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Distance from point `p` to the line segment a–b.
// ─────────────────────────────────────────────────────────────────────────────
fn point_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq == 0.0 {
        return p.distance(a);
    }
    // Project p onto the segment, clamped to the segment's extent.
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 100px run east, then 50px south.
    fn l_path() -> Path {
        Path::new(vec![vec2(0.0, 0.0), vec2(100.0, 0.0), vec2(100.0, 50.0)])
    }

    #[test]
    fn total_is_the_sum_of_segments() {
        assert_eq!(l_path().total(), 150.0);
    }

    #[test]
    fn point_at_walks_across_the_corner() {
        let p = l_path();
        assert_eq!(p.point_at(0.0), vec2(0.0, 0.0));
        assert_eq!(p.point_at(50.0), vec2(50.0, 0.0));
        assert_eq!(p.point_at(100.0), vec2(100.0, 0.0));
        // Past the corner, travel continues down the second segment.
        assert_eq!(p.point_at(125.0), vec2(100.0, 25.0));
    }

    #[test]
    fn point_at_clamps_beyond_both_ends() {
        let p = l_path();
        assert_eq!(p.point_at(-10.0), vec2(0.0, 0.0));
        assert_eq!(p.point_at(9999.0), vec2(100.0, 50.0));
    }

    #[test]
    fn distance_to_is_perpendicular_not_to_the_nearest_waypoint() {
        let p = l_path();
        // Sitting 20px above the middle of the first segment.
        assert!((p.distance_to(vec2(50.0, 20.0)) - 20.0).abs() < 0.001);
    }

    #[test]
    fn zero_length_segments_do_not_divide_by_zero() {
        let p = Path::new(vec![vec2(0.0, 0.0), vec2(0.0, 0.0), vec2(10.0, 0.0)]);
        assert!(p.point_at(5.0).is_finite());
    }
}
