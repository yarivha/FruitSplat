// =============================================================================
// scenery.rs — the decorative backdrop that gives each route its own look
//
// Every route gets a palette and a themed scatter of props: an orchard of neat
// trees, a dusty market of crates and fences, a lush farm with ponds, or a dense
// grove. All of it is generated procedurally, like the rest of the artwork, so
// nothing here ships as an image asset.
//
// Props are purely decorative — they never block tower placement. They are laid
// out once when a run starts, not per frame, and placement is deterministic per
// route so a given route always looks the same.
//
// Randomness here uses a local generator rather than macroquad's global one.
// Reseeding the global RNG to make scenery repeatable would also make the fruit
// order repeatable, which is not wanted.
// =============================================================================

use macroquad::prelude::*;

use crate::path::Path;
use crate::{PLAYFIELD_H, PLAYFIELD_W};

/// Keep props this far from the track centreline so the route stays readable.
const TRACK_CLEARANCE: f32 = 58.0;
/// Trees are big, so they stand further back than the smaller props.
const TREE_CLEARANCE: f32 = 72.0;
/// Ponds are bigger still.
const POND_CLEARANCE: f32 = 110.0;
/// Minimum gap between two props, so the scatter doesn't clump.
const PROP_SEPARATION: f32 = 34.0;
/// Bottom of the HUD strip. Prop artwork must not reach above this.
const HUD_BOTTOM: f32 = 56.0;
const EDGE_MARGIN: f32 = 10.0;
/// Attempts per prop before giving up on finding a free spot.
const PLACEMENT_TRIES: u32 = 40;

/// The colours that vary between routes.
pub struct Palette {
    pub grass_top: Color,
    pub grass_bottom: Color,
    pub track_border: Color,
    pub track_dirt: Color,
    /// Base foliage colour, tinted per prop.
    pub foliage: Color,
}

/// What a single piece of scenery is.
#[derive(Clone, Copy, PartialEq)]
pub enum PropKind {
    Tree,
    Bush,
    Rock,
    Flowers,
    Crate,
    Fence,
    Pond,
}

impl PropKind {
    /// How far this prop must sit from the track.
    fn clearance(&self) -> f32 {
        match self {
            PropKind::Tree => TREE_CLEARANCE,
            PropKind::Pond => POND_CLEARANCE,
            _ => TRACK_CLEARANCE,
        }
    }

    /// Roughly how much room this prop takes up, for spacing purposes.
    fn footprint(&self) -> f32 {
        match self {
            PropKind::Pond => 62.0,
            PropKind::Tree => 30.0,
            PropKind::Crate => 22.0,
            PropKind::Fence => 26.0,
            _ => 16.0,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How far this prop's artwork reaches from its position, as
    // (sideways, upward, downward) at scale 1.
    //
    // A prop is positioned by its base, but a tree draws its canopy well above
    // that and a pond spreads far to either side. Placing on the centre point
    // alone let ponds hang off the window edge and tree canopies push up into
    // the HUD strip.
    // ─────────────────────────────────────────────────────────────────────────
    fn extent(&self) -> (f32, f32, f32) {
        match self {
            PropKind::Tree => (17.0, 36.0, 11.0),
            PropKind::Bush => (16.0, 14.0, 8.0),
            PropKind::Rock => (10.0, 12.0, 7.0),
            PropKind::Flowers => (11.0, 14.0, 6.0),
            PropKind::Crate => (13.0, 11.0, 11.0),
            PropKind::Fence => (16.0, 14.0, 9.0),
            PropKind::Pond => (49.0, 28.0, 28.0),
        }
    }
}

/// One placed piece of scenery.
pub struct Prop {
    pub kind: PropKind,
    pub pos: Vec2,
    pub scale: f32,
    /// Per-prop colour jitter, so a stand of trees isn't uniformly flat.
    pub shade: f32,
    /// Free-form angle, used by props that aren't radially symmetric.
    pub angle: f32,
}

/// Which backdrop a route wears.
#[derive(Clone, Copy)]
enum Theme {
    Orchard,
    Market,
    Farm,
    Grove,
    /// The two-entrance route: a cold upland with two ways down into it.
    Highland,
    /// The long wandering route: open, bright, and easy on the eye, to match
    /// the gentlest walk in the game.
    Meadow,
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme for a route index. Anything beyond the known routes falls back to the
// grove rather than panicking.
// ─────────────────────────────────────────────────────────────────────────────
fn theme(track: usize) -> Theme {
    match track {
        0 => Theme::Orchard,
        1 => Theme::Market,
        2 => Theme::Farm,
        3 => Theme::Grove,
        4 => Theme::Highland,
        _ => Theme::Meadow,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The palette for a route. Kept public so the route-selection cards can preview
// each backdrop's colour without generating its props.
// ─────────────────────────────────────────────────────────────────────────────
pub fn palette(track: usize) -> Palette {
    match theme(track) {
        // Temperate orchard: the original mid green.
        Theme::Orchard => Palette {
            grass_top: Color::new(0.36, 0.51, 0.31, 1.0),
            grass_bottom: Color::new(0.24, 0.38, 0.24, 1.0),
            track_border: Color::new(0.38, 0.28, 0.18, 1.0),
            track_dirt: Color::new(0.55, 0.43, 0.29, 1.0),
            foliage: Color::new(0.26, 0.47, 0.26, 1.0),
        },
        // Sun-baked and dusty, to suit the short brutal route.
        Theme::Market => Palette {
            grass_top: Color::new(0.55, 0.52, 0.30, 1.0),
            grass_bottom: Color::new(0.41, 0.37, 0.22, 1.0),
            track_border: Color::new(0.42, 0.33, 0.22, 1.0),
            track_dirt: Color::new(0.66, 0.55, 0.38, 1.0),
            foliage: Color::new(0.38, 0.45, 0.24, 1.0),
        },
        // Lush and well watered.
        Theme::Farm => Palette {
            grass_top: Color::new(0.38, 0.58, 0.30, 1.0),
            grass_bottom: Color::new(0.23, 0.43, 0.24, 1.0),
            track_border: Color::new(0.35, 0.26, 0.17, 1.0),
            track_dirt: Color::new(0.58, 0.46, 0.31, 1.0),
            foliage: Color::new(0.24, 0.50, 0.27, 1.0),
        },
        // Dense and shaded.
        Theme::Grove => Palette {
            grass_top: Color::new(0.25, 0.40, 0.26, 1.0),
            grass_bottom: Color::new(0.15, 0.28, 0.19, 1.0),
            track_border: Color::new(0.31, 0.23, 0.15, 1.0),
            track_dirt: Color::new(0.50, 0.39, 0.26, 1.0),
            foliage: Color::new(0.19, 0.38, 0.22, 1.0),
        },
        // Bright open meadow. The lightest palette of the six, so the longest
        // and friendliest route also reads as the friendliest at a glance.
        Theme::Meadow => Palette {
            grass_top: Color::new(0.46, 0.60, 0.32, 1.0),
            grass_bottom: Color::new(0.33, 0.48, 0.26, 1.0),
            track_border: Color::new(0.44, 0.34, 0.21, 1.0),
            track_dirt: Color::new(0.68, 0.57, 0.38, 1.0),
            foliage: Color::new(0.34, 0.56, 0.28, 1.0),
        },
        // Cold, thin upland grass over pale stone. Deliberately the bluest of
        // the five: the two-lane route asks the player to watch two places at
        // once, and it helps if a glance at the field says which route this is.
        Theme::Highland => Palette {
            grass_top: Color::new(0.34, 0.46, 0.42, 1.0),
            grass_bottom: Color::new(0.22, 0.33, 0.33, 1.0),
            track_border: Color::new(0.33, 0.31, 0.28, 1.0),
            track_dirt: Color::new(0.56, 0.53, 0.47, 1.0),
            foliage: Color::new(0.24, 0.42, 0.34, 1.0),
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The prop mix for a theme, as a weighted bag drawn from at random, plus how
// many props to scatter.
// ─────────────────────────────────────────────────────────────────────────────
fn prop_mix(theme: Theme) -> (&'static [PropKind], usize) {
    match theme {
        Theme::Orchard => (
            &[
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Bush,
                PropKind::Flowers,
                PropKind::Rock,
            ],
            46,
        ),
        Theme::Market => (
            &[
                PropKind::Crate,
                PropKind::Crate,
                PropKind::Fence,
                PropKind::Rock,
                PropKind::Bush,
                PropKind::Tree,
            ],
            42,
        ),
        Theme::Farm => (
            &[
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Bush,
                PropKind::Flowers,
                PropKind::Flowers,
                PropKind::Pond,
                PropKind::Fence,
            ],
            52,
        ),
        Theme::Grove => (
            &[
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Bush,
                PropKind::Bush,
                PropKind::Rock,
            ],
            62,
        ),
        Theme::Meadow => (
            &[
                PropKind::Flowers,
                PropKind::Flowers,
                PropKind::Bush,
                PropKind::Tree,
                PropKind::Tree,
                PropKind::Pond,
                PropKind::Rock,
            ],
            48,
        ),
        // Rocky and sparse. Two lanes leave less open ground to build on, so
        // the scatter is thinner than anywhere else — scenery never blocks
        // placement, but a crowded field makes it harder to read where the
        // legal ground actually is.
        Theme::Highland => (
            &[
                PropKind::Rock,
                PropKind::Rock,
                PropKind::Rock,
                PropKind::Bush,
                PropKind::Bush,
                PropKind::Tree,
                PropKind::Flowers,
            ],
            34,
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lay out the scenery for a route.
//
// Placement is rejection sampling: pick a spot, reject it if it crowds the
// track or another prop, and give up on that prop after a bounded number of
// tries so a busy route can never spin forever.
// ─────────────────────────────────────────────────────────────────────────────
pub fn generate(track: usize, paths: &[Path]) -> Vec<Prop> {
    let (mix, count) = prop_mix(theme(track));
    // Seeded from the route index, so a route always looks the same.
    let mut rng = Rng::new(0x5EED_0000 + track as u64);
    let mut props: Vec<Prop> = Vec::with_capacity(count);

    for _ in 0..count {
        let kind = mix[rng.below(mix.len() as u32) as usize];
        // Scale is chosen before the position, because how much room the prop
        // needs at the window edges depends on it.
        let scale = rng.range(0.82, 1.24);
        let (ex, up, down) = kind.extent();
        let (ex, up, down) = (ex * scale, up * scale, down * scale);

        for _ in 0..PLACEMENT_TRIES {
            let pos = vec2(
                rng.range(EDGE_MARGIN + ex, PLAYFIELD_W - EDGE_MARGIN - ex),
                rng.range(HUD_BOTTOM + up, PLAYFIELD_H - EDGE_MARGIN - down),
            );

            // Clear of every lane. A prop crowding the second gate of a
            // two-entrance route is just as much in the way as one on the first.
            if paths
                .iter()
                .any(|path| path.distance_to(pos) < kind.clearance())
            {
                continue;
            }
            let spacing = PROP_SEPARATION + kind.footprint();
            if props
                .iter()
                .any(|p| p.pos.distance(pos) < spacing.max(p.kind.footprint()))
            {
                continue;
            }

            props.push(Prop {
                kind,
                pos,
                scale,
                shade: rng.range(-0.06, 0.08),
                angle: rng.range(0.0, std::f32::consts::TAU),
            });
            break;
        }
    }

    // Painter's order: props lower on screen are nearer, so they draw last.
    props.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));
    props
}

// ─────────────────────────────────────────────────────────────────────────────
// A small xorshift generator, kept local so scenery layout is repeatable
// without disturbing the global RNG that drives gameplay.
// ─────────────────────────────────────────────────────────────────────────────
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // A zero state would make xorshift emit zeroes forever.
        Rng(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    fn below(&mut self, n: u32) -> u32 {
        (self.next() >> 33) as u32 % n.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::TRACKS;

    fn test_paths(track: usize) -> Vec<Path> {
        TRACKS[track].paths()
    }

    #[test]
    fn the_local_rng_stays_in_range_and_does_not_stall() {
        let mut rng = Rng::new(1);
        let mut seen_low = false;
        let mut seen_high = false;

        for _ in 0..2000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u), "unit() escaped [0,1): {u}");
            if u < 0.25 {
                seen_low = true;
            }
            if u > 0.75 {
                seen_high = true;
            }
        }
        // A stuck generator would sit in one place forever.
        assert!(seen_low && seen_high, "rng never covered the range");
    }

    #[test]
    fn below_never_exceeds_its_bound() {
        let mut rng = Rng::new(7);
        for _ in 0..500 {
            assert!(rng.below(6) < 6);
        }
    }

    #[test]
    fn every_route_gets_scenery() {
        for track in 0..TRACKS.len() {
            let props = generate(track, &test_paths(track));
            assert!(!props.is_empty(), "route {track} generated no scenery");
        }
    }

    #[test]
    fn no_prop_sits_on_the_track() {
        for track in 0..TRACKS.len() {
            let paths = test_paths(track);
            for p in generate(track, &paths) {
                // Clear of every lane, not merely the first one.
                let nearest = paths
                    .iter()
                    .map(|path| path.distance_to(p.pos))
                    .fold(f32::MAX, f32::min);
                assert!(
                    nearest >= p.kind.clearance(),
                    "route {track} put a prop on the track"
                );
            }
        }
    }

    #[test]
    fn no_prop_artwork_escapes_the_playfield_or_hits_the_hud() {
        // Checks the drawn extent, not just the centre point: ponds used to
        // hang off the window edge and tree canopies reached into the HUD.
        for track in 0..TRACKS.len() {
            for p in generate(track, &test_paths(track)) {
                let (ex, up, down) = p.kind.extent();
                let (ex, up, down) = (ex * p.scale, up * p.scale, down * p.scale);

                assert!(p.pos.x - ex >= 0.0, "route {track}: prop off the left edge");
                assert!(
                    p.pos.x + ex <= PLAYFIELD_W,
                    "route {track}: prop off the right edge"
                );
                assert!(
                    p.pos.y - up >= HUD_BOTTOM,
                    "route {track}: prop reaches into the HUD strip"
                );
                assert!(
                    p.pos.y + down <= PLAYFIELD_H,
                    "route {track}: prop hangs below the playfield"
                );
            }
        }
    }

    #[test]
    fn layout_is_repeatable_for_a_route() {
        for track in 0..TRACKS.len() {
            let paths = test_paths(track);
            let a = generate(track, &paths);
            let b = generate(track, &paths);

            assert_eq!(a.len(), b.len());
            for (p, q) in a.iter().zip(b.iter()) {
                assert_eq!(p.pos, q.pos);
                assert!(p.kind == q.kind);
            }
        }
    }

    #[test]
    fn different_routes_get_different_layouts() {
        let a = generate(0, &test_paths(0));
        let b = generate(1, &test_paths(1));
        assert_ne!(a[0].pos, b[0].pos);
    }

    #[test]
    fn props_are_sorted_back_to_front() {
        let props = generate(0, &test_paths(0));
        for w in props.windows(2) {
            assert!(w[0].pos.y <= w[1].pos.y, "painter order is broken");
        }
    }

    #[test]
    fn every_route_has_a_distinct_palette() {
        for i in 0..TRACKS.len() {
            for j in (i + 1)..TRACKS.len() {
                let (a, b) = (palette(i), palette(j));
                let diff = (a.grass_top.r - b.grass_top.r).abs()
                    + (a.grass_top.g - b.grass_top.g).abs()
                    + (a.grass_top.b - b.grass_top.b).abs();
                assert!(diff > 0.02, "routes {i} and {j} look identical");
            }
        }
    }
}
