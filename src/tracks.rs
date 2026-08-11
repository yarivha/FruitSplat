// =============================================================================
// tracks.rs — the selectable routes the fruit walk
//
// Each track is one or more hand-authored polylines — lanes — in playfield
// coordinates (0..1200 by 0..740). Lanes start and end just outside the window
// so fruit enter and leave off-screen rather than blinking into existence at the
// border.
//
// Most routes are a single lane. A route with two lanes has two entrances, and
// fruit are dealt to them in turn, so the wave arrives as two streams the player
// has to answer separately. Lanes stay logically independent for their whole
// length even where they converge on screen: a fruit belongs to one lane, and a
// spike pile only ever touches fruit on the lane it was dropped on.
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
    /// How many waves must be survived to clear this route. Harder routes run
    /// shorter, so they're a sharper challenge rather than simply a longer one.
    pub waves: u32,
    /// One polyline per lane, each a list of (x, y) pairs so the table can stay
    /// a plain const. Most routes have exactly one.
    pub lanes: &'static [&'static [(f32, f32)]],
}

impl TrackDef {
    // ─────────────────────────────────────────────────────────────────────────
    // Build the runtime Path for every lane of this route.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn paths(&self) -> Vec<Path> {
        self.lanes
            .iter()
            .map(|lane| Path::new(lane.iter().map(|&(x, y)| vec2(x, y)).collect()))
            .collect()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How far a single fruit walks on this route, averaged over its lanes.
    //
    // The average rather than the total, because a fruit walks one lane, not all
    // of them — summing would make a two-lane route look twice as forgiving as
    // it plays when it is really the same walk twice over.
    //
    // Test-only: the selection screen shows wave count instead, since that's
    // what the player is actually choosing between. This stays because the
    // route table's difficulty labels are asserted against it — a route
    // labelled Hard must really be the shortest.
    // ─────────────────────────────────────────────────────────────────────────
    #[cfg(test)]
    pub fn length(&self) -> f32 {
        let total: f32 = self
            .lanes
            .iter()
            .map(|lane| {
                lane.windows(2)
                    .map(|w| {
                        let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                        (dx * dx + dy * dy).sqrt()
                    })
                    .sum::<f32>()
            })
            .sum();
        total / self.lanes.len() as f32
    }
}

/// Every route offered at the start of a game.
pub const TRACKS: [TrackDef; 7] = [
    TrackDef {
        name: "Orchard Snake",
        blurb: "a steady weave",
        difficulty: "Medium",
        waves: 46,
        lanes: &[&[
            (-40.0, 180.0),
            (260.0, 180.0),
            (260.0, 360.0),
            (110.0, 360.0),
            (110.0, 550.0),
            (520.0, 550.0),
            (520.0, 260.0),
            (960.0, 260.0),
            (960.0, 590.0),
            (1240.0, 590.0),
        ]],
    },
    TrackDef {
        name: "Market Run",
        blurb: "short and direct",
        difficulty: "Hard",
        waves: 42,
        lanes: &[&[
            (-40.0, 370.0),
            (300.0, 370.0),
            (300.0, 230.0),
            (900.0, 230.0),
            (900.0, 510.0),
            (1240.0, 510.0),
        ]],
    },
    TrackDef {
        name: "The Long Orchard",
        blurb: "plenty of time to shoot",
        difficulty: "Gentle",
        waves: 48,
        lanes: &[&[
            (-40.0, 150.0),
            (180.0, 150.0),
            (180.0, 330.0),
            (60.0, 330.0),
            (60.0, 500.0),
            (300.0, 500.0),
            (300.0, 210.0),
            (500.0, 210.0),
            (500.0, 590.0),
            (700.0, 590.0),
            (700.0, 280.0),
            (1060.0, 280.0),
            (1060.0, 500.0),
            (1240.0, 500.0),
        ]],
    },
    TrackDef {
        name: "Zigzag Grove",
        blurb: "tight lanes, wide cover",
        difficulty: "Medium",
        waves: 46,
        lanes: &[&[
            (-40.0, 210.0),
            (150.0, 210.0),
            (150.0, 550.0),
            (340.0, 550.0),
            (340.0, 210.0),
            (530.0, 210.0),
            (530.0, 550.0),
            (720.0, 550.0),
            (720.0, 210.0),
            (1100.0, 210.0),
            (1100.0, 550.0),
            (1240.0, 550.0),
        ]],
    },
    // ─────────────────────────────────────────────────────────────────────────
    // The two-lane route. Fruit enter at two gates on the left, one high and one
    // low, and both streams leave by the same exit on the right.
    //
    // The lanes converge on the exit *point* rather than sharing a stretch of
    // track before it. A shared corridor would look right and play wrong: piles
    // belong to one lane, so a Spike Layer covering the shared run would pop
    // only half the fruit walking over it, for no reason the player could see.
    //
    // Neither lane is long — the difficulty is that they are far apart. A single
    // cluster of towers cannot answer both, so the same money has to cover two
    // approaches, and the wave arrives as two half-strength streams instead of
    // one you can meet head on.
    // ─────────────────────────────────────────────────────────────────────────
    TrackDef {
        name: "Twin Gates",
        blurb: "two ways in, one out",
        difficulty: "Tricky",
        waves: 44,
        lanes: &[
            // The high road.
            &[
                (-40.0, 150.0),
                (240.0, 150.0),
                (240.0, 330.0),
                (450.0, 330.0),
                (450.0, 160.0),
                (700.0, 160.0),
                (700.0, 360.0),
                (1000.0, 360.0),
                (1240.0, 430.0),
            ],
            // The low road.
            &[
                (-40.0, 560.0),
                (200.0, 560.0),
                (200.0, 410.0),
                (380.0, 410.0),
                (380.0, 590.0),
                (620.0, 590.0),
                (620.0, 500.0),
                (1000.0, 500.0),
                (1240.0, 430.0),
            ],
        ],
    },
    // ─────────────────────────────────────────────────────────────────────────
    // A long route where length is doing all the work: every extra pixel of
    // track is another moment a fruit spends inside somebody's range, and
    // nothing else about it is unusual. The one to learn on, until the Spiral
    // Garden took the "longest" title off it.
    // ─────────────────────────────────────────────────────────────────────────
    TrackDef {
        name: "Meander",
        blurb: "the long way round",
        difficulty: "Gentle",
        waves: 50,
        lanes: &[&[
            (-40.0, 130.0),
            (150.0, 130.0),
            (150.0, 300.0),
            (60.0, 300.0),
            (60.0, 470.0),
            (240.0, 470.0),
            (240.0, 620.0),
            (420.0, 620.0),
            (420.0, 180.0),
            (600.0, 180.0),
            (600.0, 540.0),
            (760.0, 540.0),
            (760.0, 240.0),
            (900.0, 240.0),
            (900.0, 620.0),
            (1060.0, 620.0),
            (1060.0, 340.0),
            (1240.0, 340.0),
        ]],
    },
    // ─────────────────────────────────────────────────────────────────────────
    // The spiral. Fruit wind all the way into the middle, turn, and wind back
    // out through the channels between the arms they came in along.
    //
    // A spiral cannot simply end in the middle — a lane has to leave the screen
    // — and it cannot cut straight out either, because the way out would cross
    // every arm it had just wound past. So this is a double spiral: the inbound
    // and outbound arms interleave the whole way, which is why the exit sits
    // just below the entrance rather than on the far side.
    //
    // Every parallel pair of arms is 110px apart. That is deliberate and it is
    // the number the whole shape is built on: a tower must be PATH_CLEARANCE
    // (44px) from a centreline, so 88px is the least that leaves a legal strip
    // between two arms, and 110 leaves one comfortably. Tighten the spiral and
    // the channels stop being buildable, at which point the middle of the
    // board is scenery rather than a place to defend.
    //
    // At 6510px it is by a wide margin the longest route in the game, and a
    // tower parked in an inner channel covers three or four arms at once. Both
    // of those pull the same way, which is why it is labelled Gentle.
    // ─────────────────────────────────────────────────────────────────────────
    TrackDef {
        name: "Spiral Garden",
        blurb: "wind in, then wind out",
        difficulty: "Gentle",
        waves: 50,
        lanes: &[&[
            (-40.0, 90.0),
            (1070.0, 90.0),
            (1070.0, 640.0),
            (80.0, 640.0),
            (80.0, 310.0),
            (850.0, 310.0),
            (850.0, 420.0),
            // The turn, at the centre.
            (300.0, 420.0),
            (300.0, 530.0),
            (960.0, 530.0),
            (960.0, 200.0),
            (-40.0, 200.0),
        ]],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tower::PATH_CLEARANCE;
    use crate::PLAYFIELD_H;

    /// The coordinate space the routes are authored in.
    const AUTHOR_W: f32 = crate::PLAYFIELD_W;

    /// Longest blurb that fits inside a selection card at its font size.
    /// Measured against the card, not guessed. Was 26 when four cards shared the
    /// row at 220px each; a fifth route narrowed them to about 185px, and this
    /// came down with it.
    const MAX_BLURB_CHARS: usize = 23;
    /// Longest route name, counting the "5. " the card prefixes it with. Also
    /// measured: at 19px the name column runs out at roughly this many.
    const MAX_NAME_CHARS: usize = 19;

    #[test]
    fn blurbs_fit_inside_a_selection_card() {
        for t in &TRACKS {
            assert!(
                t.blurb.len() <= MAX_BLURB_CHARS,
                "{} blurb is {} chars, over the {MAX_BLURB_CHARS} that fit",
                t.name,
                t.blurb.len()
            );
        }
    }

    #[test]
    fn route_names_fit_inside_a_selection_card() {
        for (i, t) in TRACKS.iter().enumerate() {
            let shown = format!("{}. {}", i + 1, t.name);
            assert!(
                shown.len() <= MAX_NAME_CHARS,
                "\"{shown}\" is {} chars, over the {MAX_NAME_CHARS} that fit",
                shown.len()
            );
        }
    }

    #[test]
    fn every_lane_has_a_usable_polyline() {
        for t in &TRACKS {
            assert!(!t.lanes.is_empty(), "{} has no lanes", t.name);
            for lane in t.lanes {
                assert!(
                    lane.len() >= 2,
                    "{} has a lane with too few waypoints",
                    t.name
                );
            }
            assert!(t.length() > 0.0, "{} has zero length", t.name);
        }
    }

    #[test]
    fn every_lane_enters_and_exits_off_screen() {
        for t in &TRACKS {
            for lane in t.lanes {
                let first = lane[0];
                let last = lane[lane.len() - 1];
                assert!(
                    first.0 < 0.0 || first.0 > AUTHOR_W,
                    "a lane of {} starts on-screen",
                    t.name
                );
                assert!(
                    last.0 < 0.0 || last.0 > AUTHOR_W,
                    "a lane of {} ends on-screen",
                    t.name
                );
            }
        }
    }

    #[test]
    fn every_lane_stays_inside_the_playfield_vertically() {
        // Routes must leave room for a tower beside them, top and bottom.
        for t in &TRACKS {
            for lane in t.lanes {
                for &(_, y) in *lane {
                    assert!(
                        y > PATH_CLEARANCE && y < PLAYFIELD_H - PATH_CLEARANCE,
                        "{} has a waypoint at y={y}, too close to the edge",
                        t.name
                    );
                }
            }
        }
    }

    #[test]
    fn a_multi_lane_route_shares_one_exit() {
        // Two entrances, one exit: the streams converge on a single point, so
        // there is still only one thing to defend.
        for t in TRACKS.iter().filter(|t| t.lanes.len() > 1) {
            let exit = *t.lanes[0].last().unwrap();
            for lane in t.lanes {
                assert_eq!(
                    *lane.last().unwrap(),
                    exit,
                    "{} has lanes leaving by different exits",
                    t.name
                );
            }
        }
    }

    #[test]
    fn a_multi_lane_route_starts_its_lanes_apart() {
        // Two entrances that arrive at the same place would not read as two.
        for t in TRACKS.iter().filter(|t| t.lanes.len() > 1) {
            for (i, a) in t.lanes.iter().enumerate() {
                for b in t.lanes.iter().skip(i + 1) {
                    let (ax, ay) = a[0];
                    let (bx, by) = b[0];
                    let gap = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
                    assert!(
                        gap > 200.0,
                        "{} has two gates only {gap:.0}px apart",
                        t.name
                    );
                }
            }
        }
    }

    #[test]
    fn lanes_never_share_a_stretch_of_track_before_the_exit() {
        // Piles belong to one lane. Two lanes running the same corridor would
        // let a Spike Layer sit on both and visibly pop only half the fruit
        // crossing it, so lanes may only meet at the exit point itself.
        const STEP: f32 = 10.0;
        // Two track bodies touch at 44px apart; keep a clear margin past that.
        const MIN_GAP: f32 = 60.0;

        for t in TRACKS.iter().filter(|t| t.lanes.len() > 1) {
            let paths = t.paths();
            let exit = *paths[0].points().last().unwrap();

            for (i, a) in paths.iter().enumerate() {
                for b in paths.iter().skip(i + 1) {
                    let mut d = 0.0;
                    while d <= a.total() {
                        let p = a.point_at(d);
                        // Near the shared exit they are meant to converge.
                        if p.distance(exit) > 180.0 {
                            assert!(
                                b.distance_to(p) >= MIN_GAP,
                                "{} has lanes {:.0}px apart at {p}, sharing a corridor",
                                t.name,
                                b.distance_to(p)
                            );
                        }
                        d += STEP;
                    }
                }
            }
        }
    }

    #[test]
    fn no_lane_has_a_zero_length_segment() {
        // Duplicate waypoints would make point_at fall back to a lerp of 0.
        for t in &TRACKS {
            for lane in t.lanes {
                for w in lane.windows(2) {
                    assert!(w[0] != w[1], "{} has a duplicated waypoint", t.name);
                }
            }
        }
    }

    /// The route with the shortest and the longest walk, by name.
    fn extremes() -> (&'static TrackDef, &'static TrackDef) {
        let by_len = |a: &&TrackDef, b: &&TrackDef| a.length().total_cmp(&b.length());
        (
            TRACKS.iter().min_by(by_len).unwrap(),
            TRACKS.iter().max_by(by_len).unwrap(),
        )
    }

    #[test]
    fn the_shortest_route_is_labelled_hard_and_the_longest_gentle() {
        // Asked this way round rather than "the Gentle route is the longest",
        // because more than one route can share a label — there are two Gentle
        // ones now. What has to hold is that the extremes are labelled honestly.
        let (shortest, longest) = extremes();
        assert_eq!(shortest.difficulty, "Hard", "{} is shortest", shortest.name);
        assert_eq!(longest.difficulty, "Gentle", "{} is longest", longest.name);
    }

    #[test]
    fn no_route_labelled_gentle_is_shorter_than_one_labelled_hard() {
        // The weaker claim that survives duplicate labels: whatever carries a
        // label has to be on the right side of everything carrying the other.
        for gentle in TRACKS.iter().filter(|t| t.difficulty == "Gentle") {
            for hard in TRACKS.iter().filter(|t| t.difficulty == "Hard") {
                assert!(
                    gentle.length() > hard.length(),
                    "{} (Gentle) is shorter than {} (Hard)",
                    gentle.name,
                    hard.name
                );
            }
        }
    }

    #[test]
    fn every_route_is_a_long_run_that_still_reaches_the_top_tier() {
        // A new tier unlocks every third wave, so watermelons first appear on
        // wave 13 — the floor below is far above that anyway.
        //
        // Routes used to run 15 to 25 waves, and this test capped them at 40 on
        // the grounds that more would outstay their welcome. That was true of
        // the game as it escalated then: the speed ramp topped out at wave 26
        // and the spawn interval at 28, after which waves grew longer rather
        // than harder while income compounded, so a fiftieth wave really would
        // have been a victory lap. Both dials keep working now, which is what
        // makes a run this length worth playing to the end.
        for t in &TRACKS {
            assert!(
                t.waves >= 40,
                "{} ends at wave {}, short of a full run",
                t.name,
                t.waves
            );
            // A played-and-measured ceiling rather than a guess: 70 waves came
            // to about fifteen minutes of fruit walking plus build time, which
            // is longer than anyone wants to sit for one run.
            assert!(t.waves <= 50, "{} runs longer than a sitting", t.name);
        }
    }

    #[test]
    fn every_route_runs_long_enough_to_meet_the_boss() {
        // The Durian's whole design assumes the player will actually face it. A
        // route ending before the first boss wave would never show them one.
        for t in &TRACKS {
            assert!(
                t.waves >= crate::wave::FIRST_BOSS_WAVE,
                "{} ends at wave {}, before the first boss wave",
                t.name,
                t.waves
            );
            assert!(
                crate::wave::boss_count(t.waves, t.waves) >= 1,
                "{} does not end on a boss wave",
                t.name
            );
        }
    }

    #[test]
    fn harder_routes_are_shorter_runs() {
        // Same shape of claim as the length one: a Hard route must not outlast a
        // Gentle one, however many routes carry either label.
        for gentle in TRACKS.iter().filter(|t| t.difficulty == "Gentle") {
            for hard in TRACKS.iter().filter(|t| t.difficulty == "Hard") {
                assert!(
                    hard.waves <= gentle.waves,
                    "{} (Hard) runs longer than {} (Gentle)",
                    hard.name,
                    gentle.name
                );
            }
        }
    }

    #[test]
    fn built_paths_match_the_computed_average_length() {
        for t in &TRACKS {
            let built: f32 =
                t.paths().iter().map(|p| p.total()).sum::<f32>() / t.lanes.len() as f32;
            assert!((built - t.length()).abs() < 0.01, "{}", t.name);
        }
    }

    #[test]
    fn the_two_lane_route_is_the_only_one_with_more_than_one_lane() {
        // Everything downstream branches on lane count, so it is worth knowing
        // exactly which routes exercise that path.
        let multi: Vec<&str> = TRACKS
            .iter()
            .filter(|t| t.lanes.len() > 1)
            .map(|t| t.name)
            .collect();
        assert_eq!(multi, vec!["Twin Gates"]);
        assert_eq!(
            TRACKS
                .iter()
                .find(|t| t.name == "Twin Gates")
                .unwrap()
                .lanes
                .len(),
            2
        );
    }
}
