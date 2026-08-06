// =============================================================================
// tracks.rs — the selectable routes the fruit walk
//
// Each track is a hand-authored polyline in playfield coordinates (0..1000 by
// 0..650). Routes start and end just outside the window so fruit enter and
// leave off-screen rather than blinking into existence at the border.
//
// Route length is the main difficulty dial: a longer route means more seconds
// under fire before a fruit reaches the exit, so towers get more shots off.
// Turn count matters too — tight switchbacks let one tower cover several lanes,
// which is why difficulty is labelled by hand rather than derived from length.
// =============================================================================

use macroquad::prelude::*;

use crate::path::Path;

/// One selectable route.
pub struct TrackDef {
    pub name: &'static str,
    /// Short note on how the route plays, shown on the selection card.
    pub blurb: &'static str,
    pub difficulty: &'static str,
    /// Waypoints as (x, y) pairs so the table can be a plain const.
    pub points: &'static [(f32, f32)],
}

impl TrackDef {
    // ─────────────────────────────────────────────────────────────────────────
    // Build the runtime Path for this route.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn path(&self) -> Path {
        Path::new(self.points.iter().map(|&(x, y)| vec2(x, y)).collect())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Total route length in pixels. Computed straight from the waypoints so the
    // selection screen can show it every frame without allocating a Path.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn length(&self) -> f32 {
        self.points
            .windows(2)
            .map(|w| {
                let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }
}

/// Every route offered at the start of a game.
pub const TRACKS: [TrackDef; 4] = [
    TrackDef {
        name: "Orchard Snake",
        blurb: "a steady weave",
        difficulty: "Medium",
        points: &[
            (-40.0, 150.0),
            (260.0, 150.0),
            (260.0, 330.0),
            (110.0, 330.0),
            (110.0, 520.0),
            (520.0, 520.0),
            (520.0, 230.0),
            (760.0, 230.0),
            (760.0, 560.0),
            (1040.0, 560.0),
        ],
    },
    TrackDef {
        name: "Market Run",
        blurb: "short and direct",
        difficulty: "Hard",
        points: &[
            (-40.0, 340.0),
            (300.0, 340.0),
            (300.0, 200.0),
            (700.0, 200.0),
            (700.0, 480.0),
            (1040.0, 480.0),
        ],
    },
    TrackDef {
        name: "The Long Orchard",
        blurb: "plenty of time to shoot",
        difficulty: "Gentle",
        points: &[
            (-40.0, 120.0),
            (180.0, 120.0),
            (180.0, 300.0),
            (60.0, 300.0),
            (60.0, 470.0),
            (300.0, 470.0),
            (300.0, 180.0),
            (500.0, 180.0),
            (500.0, 560.0),
            (700.0, 560.0),
            (700.0, 250.0),
            (860.0, 250.0),
            (860.0, 470.0),
            (1040.0, 470.0),
        ],
    },
    TrackDef {
        name: "Zigzag Grove",
        blurb: "tight lanes, one tower covers two",
        difficulty: "Medium",
        points: &[
            (-40.0, 180.0),
            (150.0, 180.0),
            (150.0, 520.0),
            (340.0, 520.0),
            (340.0, 180.0),
            (530.0, 180.0),
            (530.0, 520.0),
            (720.0, 520.0),
            (720.0, 180.0),
            (900.0, 180.0),
            (900.0, 520.0),
            (1040.0, 520.0),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tower::PATH_CLEARANCE;
    use crate::PLAYFIELD_H;

    /// The coordinate space the routes are authored in.
    const AUTHOR_W: f32 = 1000.0;

    #[test]
    fn every_track_has_a_usable_polyline() {
        for t in &TRACKS {
            assert!(t.points.len() >= 2, "{} has too few waypoints", t.name);
            assert!(t.length() > 0.0, "{} has zero length", t.name);
        }
    }

    #[test]
    fn every_track_enters_and_exits_off_screen() {
        for t in &TRACKS {
            let first = t.points[0];
            let last = t.points[t.points.len() - 1];
            assert!(
                first.0 < 0.0 || first.0 > AUTHOR_W,
                "{} starts on-screen",
                t.name
            );
            assert!(
                last.0 < 0.0 || last.0 > AUTHOR_W,
                "{} ends on-screen",
                t.name
            );
        }
    }

    #[test]
    fn every_track_stays_inside_the_playfield_vertically() {
        // Routes must leave room for a tower beside them, top and bottom.
        for t in &TRACKS {
            for &(_, y) in t.points {
                assert!(
                    y > PATH_CLEARANCE && y < PLAYFIELD_H - PATH_CLEARANCE,
                    "{} has a waypoint at y={y}, too close to the edge",
                    t.name
                );
            }
        }
    }

    #[test]
    fn no_track_has_a_zero_length_segment() {
        // Duplicate waypoints would make point_at fall back to a lerp of 0.
        for t in &TRACKS {
            for w in t.points.windows(2) {
                assert!(w[0] != w[1], "{} has a duplicated waypoint", t.name);
            }
        }
    }

    #[test]
    fn the_hard_route_is_the_shortest_and_the_gentle_one_the_longest() {
        let hard = TRACKS.iter().find(|t| t.difficulty == "Hard").unwrap();
        let gentle = TRACKS.iter().find(|t| t.difficulty == "Gentle").unwrap();

        for t in &TRACKS {
            assert!(hard.length() <= t.length(), "{} is shorter than Hard", t.name);
            assert!(
                gentle.length() >= t.length(),
                "{} is longer than Gentle",
                t.name
            );
        }
    }

    #[test]
    fn a_built_path_length_matches_the_computed_one() {
        for t in &TRACKS {
            assert!((t.path().total() - t.length()).abs() < 0.01, "{}", t.name);
        }
    }
}
