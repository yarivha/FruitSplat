// =============================================================================
// fruit.rs — the fruit that walk the track, and the splats they leave behind
//
// Fruit are the "bloons": each one travels along the shared Path and, when
// popped, bursts into two of the next tier down. Watermelon → Orange → Lime →
// Strawberry → Blueberry, which is the bottom of the ladder and simply dies.
// Smaller tiers move faster, so a leaked watermelon becomes a fast swarm.
// =============================================================================

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::path::Path;

/// Downward pull applied to splat particles, in pixels per second squared.
const PARTICLE_GRAVITY: f32 = 620.0;
/// Base travel speed along the track before the per-tier multiplier.
pub const FRUIT_BASE_SPEED: f32 = 105.0;
/// How far behind its sibling the second child spawns, so splits fan out.
const SPLIT_TRAIL: f32 = 18.0;

/// Watermelons packed inside one Durian. Four is 124 pops of payload, which is
/// roughly a quarter of everything a late wave sends — enough that letting one
/// through decides the wave, without it being the whole wave on its own.
const DURIAN_PAYLOAD: usize = 4;
/// Hits a Durian takes before it bursts.
///
/// Tuned against `balance_report`, not guessed. At 60 it lands wave 15 — the
/// first boss wave — on the same 0.9 affordability ratio as wave 13, which was
/// the hardest point in a run before the boss existed. Each boss wave demands
/// 28% to 67% more work per second than the wave before it, the gap widening as
/// the count climbs, so waves 20 and 25 stay the two spikes of a run rather than
/// flattening out against a field that has had ten more waves to grow.
///
/// Lower and the boss dissolves in the crossfire meant for the swarm around it;
/// much higher and it simply walks a route the player is otherwise holding.
const DURIAN_ARMOUR: u32 = 60;

/// The six fruit tiers, ordered by the split ladder.
/// Tier 0 is Blueberry (weakest, fastest); tier 5 is the Durian, the boss.
#[derive(Clone, Copy, PartialEq)]
pub enum FruitKind {
    Durian,
    Watermelon,
    Orange,
    Lime,
    Strawberry,
    Blueberry,
}

impl FruitKind {
    // ─────────────────────────────────────────────────────────────────────────
    // Tier index: 0 = Blueberry … 5 = Durian.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn tier(&self) -> u8 {
        match self {
            FruitKind::Blueberry => 0,
            FruitKind::Strawberry => 1,
            FruitKind::Lime => 2,
            FruitKind::Orange => 3,
            FruitKind::Watermelon => 4,
            FruitKind::Durian => 5,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Build a kind from its tier index. Anything above 5 clamps to the Durian.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn from_tier(tier: u8) -> Self {
        match tier {
            0 => FruitKind::Blueberry,
            1 => FruitKind::Strawberry,
            2 => FruitKind::Lime,
            3 => FruitKind::Orange,
            4 => FruitKind::Watermelon,
            _ => FruitKind::Durian,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // What this fruit bursts into. None means it dies outright.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn child(&self) -> Option<FruitKind> {
        match self {
            FruitKind::Durian => Some(FruitKind::Watermelon),
            FruitKind::Watermelon => Some(FruitKind::Orange),
            FruitKind::Orange => Some(FruitKind::Lime),
            FruitKind::Lime => Some(FruitKind::Strawberry),
            FruitKind::Strawberry => Some(FruitKind::Blueberry),
            FruitKind::Blueberry => None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How many children this fruit bursts into.
    //
    // The ladder is binary all the way up, which is what makes a watermelon
    // worth 31 pops. The Durian is the exception it exists for: it carries a
    // whole cluster of watermelons rather than a pair, so clearing one is worth
    // four times what the tier below it costs.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn split_count(&self) -> usize {
        match self {
            FruitKind::Durian => DURIAN_PAYLOAD,
            _ => 2,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Hits this fruit takes before it bursts.
    //
    // Every ordinary fruit is 1: the game expresses toughness through the size
    // of the subtree a fruit is hiding, not through durability, which is why a
    // watermelon is "hard" at 31 pops without ever surviving a seed. The Durian
    // is armoured on top of that, so it has to be worn down before it will give
    // up its payload — the one fruit you shoot at rather than through.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn armour(&self) -> u32 {
        match self {
            FruitKind::Durian => DURIAN_ARMOUR,
            _ => 1,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How far apart along the track this fruit's children are laid out.
    //
    // A pair can overlap and still read as one burst, but the Durian's cluster
    // has to fan out or four 30px watermelons land as a single unreadable blob.
    // ─────────────────────────────────────────────────────────────────────────
    fn split_spread(&self) -> f32 {
        match self {
            FruitKind::Durian => 34.0,
            _ => SPLIT_TRAIL,
        }
    }

    /// Collision and drawing radius, in pixels.
    pub fn radius(&self) -> f32 {
        match self {
            FruitKind::Durian => 42.0,
            FruitKind::Watermelon => 30.0,
            FruitKind::Orange => 24.0,
            FruitKind::Lime => 19.0,
            FruitKind::Strawberry => 15.0,
            FruitKind::Blueberry => 11.0,
        }
    }

    /// True for fruit that need a health bar and announce themselves. Reading
    /// "is this armoured" rather than "is this a Durian" keeps the rendering and
    /// wave code from caring which kind happens to be the boss.
    pub fn is_boss(&self) -> bool {
        self.armour() > 1
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Lives lost if this fruit reaches the exit. Higher tiers hurt more because
    // they represent a whole subtree of fruit that never got popped.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn leak_cost(&self) -> u32 {
        self.tier() as u32 + 1
    }

    /// Per-tier multiplier on the base speed. Small fruit move quicker.
    pub fn speed_scale(&self) -> f32 {
        match self {
            // The Durian lumbers. Being slow is most of what makes it readable
            // as a boss: the swarm it arrives with overtakes it, so the player
            // gets a good look at what is coming before it is on top of them.
            FruitKind::Durian => 0.55,
            FruitKind::Watermelon => 0.72,
            FruitKind::Orange => 0.88,
            FruitKind::Lime => 1.0,
            FruitKind::Strawberry => 1.12,
            FruitKind::Blueberry => 1.38,
        }
    }

    /// Main body colour.
    pub fn body(&self) -> Color {
        match self {
            // Khaki husk — deliberately unlike the watermelon's clean green, so
            // the two never get confused at speed.
            FruitKind::Durian => Color::new(0.52, 0.47, 0.20, 1.0),
            FruitKind::Watermelon => Color::new(0.22, 0.55, 0.24, 1.0),
            FruitKind::Orange => Color::new(0.97, 0.58, 0.11, 1.0),
            FruitKind::Lime => Color::new(0.55, 0.80, 0.20, 1.0),
            FruitKind::Strawberry => Color::new(0.88, 0.17, 0.26, 1.0),
            FruitKind::Blueberry => Color::new(0.29, 0.31, 0.62, 1.0),
        }
    }

    /// Inner flesh colour — what the splat particles are mostly made of.
    pub fn flesh(&self) -> Color {
        match self {
            FruitKind::Durian => Color::new(0.95, 0.84, 0.42, 1.0),
            FruitKind::Watermelon => Color::new(0.93, 0.27, 0.34, 1.0),
            FruitKind::Orange => Color::new(1.0, 0.76, 0.36, 1.0),
            FruitKind::Lime => Color::new(0.80, 0.93, 0.48, 1.0),
            FruitKind::Strawberry => Color::new(0.99, 0.45, 0.48, 1.0),
            FruitKind::Blueberry => Color::new(0.55, 0.45, 0.78, 1.0),
        }
    }
}

/// A fruit walking the track. `dist` is the single source of truth for where it
/// is; `pos` is just the cached world position for that distance.
pub struct Fruit {
    pub kind: FruitKind,
    pub dist: f32,
    pub pos: Vec2,
    /// Seconds of Freezer slow remaining.
    pub slow_timer: f32,
    /// Speed multiplier while chilled; 1.0 means unaffected. Set by whichever
    /// Freezer hit this fruit, so upgraded Freezers bite harder.
    pub slow_factor: f32,
    /// Wave speed ramp this fruit was spawned under. Children inherit it, so a
    /// late-wave watermelon's whole subtree stays fast.
    pub speed_mult: f32,
    pub rot: f32,
    pub spin: f32,
    /// Hits left before this fruit bursts. Every ordinary fruit starts at 1 and
    /// so dies to the first thing that touches it; only the armoured boss ever
    /// sits above that.
    pub hp: u32,
}

impl Fruit {
    // ─────────────────────────────────────────────────────────────────────────
    // Spawn a fruit at `dist` along the track.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn new(kind: FruitKind, dist: f32, path: &Path, speed_mult: f32) -> Self {
        Fruit {
            kind,
            dist,
            pos: path.point_at(dist),
            slow_timer: 0.0,
            slow_factor: 1.0,
            speed_mult,
            rot: gen_range(0.0, 360.0),
            spin: gen_range(-50.0, 50.0),
            hp: kind.armour(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Take one hit. Returns true only for the hit that actually burst it, so a
    // fruit can never be reported as bursting twice — several shots can land on
    // one boss in a single frame, and the caller turns each `true` into a
    // removal from the fruit list.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn take_hit(&mut self) -> bool {
        if self.hp == 0 {
            return false;
        }
        self.hp -= 1;
        self.hp == 0
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Armour left, as 1.0 down to 0.0, for the health bar. Ordinary fruit sit at
    // 1.0 for their whole life, which is why the bar is only drawn for bosses.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn health_fraction(&self) -> f32 {
        self.hp as f32 / self.kind.armour().max(1) as f32
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Advance along the track, honouring any active slow.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn update(&mut self, dt: f32, path: &Path) {
        self.dist += self.current_speed() * dt;
        self.pos = path.point_at(self.dist);

        if self.slow_timer > 0.0 {
            self.slow_timer -= dt;
            // Drop back to full speed the moment the chill runs out, so a stale
            // factor from an old Freezer can't linger.
            if self.slow_timer <= 0.0 {
                self.slow_factor = 1.0;
            }
        }
        self.rot += self.spin * dt;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How fast this fruit is travelling along the track right now, in pixels
    // per second. Towers use it to lead their shots.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn current_speed(&self) -> f32 {
        FRUIT_BASE_SPEED
            * self.kind.speed_scale()
            * self.speed_mult
            * if self.slow_timer > 0.0 {
                self.slow_factor
            } else {
                1.0
            }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Apply a Freezer's chill. Overlapping Freezers stack by taking the
    // strongest slow and the longest remaining duration, rather than the last
    // one to fire winning.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn chill(&mut self, factor: f32, duration: f32) {
        self.slow_factor = if self.slow_timer > 0.0 {
            self.slow_factor.min(factor)
        } else {
            factor
        };
        self.slow_timer = self.slow_timer.max(duration);
    }

    pub fn radius(&self) -> f32 {
        self.kind.radius()
    }

    pub fn chilled(&self) -> bool {
        self.slow_timer > 0.0
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Has this fruit made it to the exit?
    // ─────────────────────────────────────────────────────────────────────────
    pub fn reached_end(&self, path: &Path) -> bool {
        self.dist >= path.total()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // The children this fruit bursts into, if any. Each one trails a little
    // further back down the track, so a split visibly fans out rather than
    // stacking every child on the same spot.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn split(&self, path: &Path) -> Vec<Fruit> {
        let Some(child) = self.kind.child() else {
            return Vec::new();
        };

        let spread = self.kind.split_spread();
        (0..self.kind.split_count())
            .map(|i| {
                let dist = (self.dist - i as f32 * spread).max(0.0);
                // Children inherit the wave's speed ramp from their parent.
                Fruit::new(child, dist, path, self.speed_mult)
            })
            .collect()
    }
}

/// One fleck of fruit pulp thrown out by a splat.
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
}

/// The burst left behind when a fruit is popped. Purely cosmetic.
pub struct Splat {
    pub particles: Vec<Particle>,
}

impl Splat {
    // ─────────────────────────────────────────────────────────────────────────
    // Explode a fruit into a ring of pulp particles at `pos`.
    // Bigger fruit throw more, and slightly heavier, chunks.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn burst(pos: Vec2, kind: FruitKind) -> Self {
        let count = (kind.radius() * 0.5) as usize + 6;
        let mut particles = Vec::with_capacity(count);

        for i in 0..count {
            // Spread evenly around the circle, then jitter so it doesn't look
            // like a mechanical starburst.
            let base_angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let angle = base_angle + gen_range(-0.28, 0.28);
            let speed = gen_range(70.0, 240.0);
            let life = gen_range(0.35, 0.8);

            particles.push(Particle {
                pos,
                vel: vec2(angle.cos() * speed, angle.sin() * speed),
                radius: gen_range(2.5, 6.0) * (kind.radius() / 24.0).clamp(0.6, 1.5),
                // Mostly flesh, with some rind mixed in for contrast.
                color: if i % 3 == 0 { kind.body() } else { kind.flesh() },
                life,
                max_life: life,
            });
        }

        Splat { particles }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Advance the particles and drop the ones that have burned out.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.vel.y += PARTICLE_GRAVITY * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn finished(&self) -> bool {
        self.particles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hits needed to clear one fruit and everything below it in the ladder,
    /// armour included.
    fn subtree_hits(kind: FruitKind) -> u32 {
        match kind.child() {
            None => kind.armour(),
            Some(c) => kind.armour() + kind.split_count() as u32 * subtree_hits(c),
        }
    }

    #[test]
    fn tier_round_trips_through_from_tier() {
        for t in 0..=5u8 {
            assert_eq!(FruitKind::from_tier(t).tier(), t);
        }
    }

    #[test]
    fn the_split_ladder_descends_one_tier_and_terminates() {
        let mut kind = FruitKind::Durian;
        let mut steps = 0;
        while let Some(child) = kind.child() {
            assert_eq!(child.tier(), kind.tier() - 1);
            kind = child;
            steps += 1;
        }
        assert!(kind == FruitKind::Blueberry);
        assert_eq!(steps, 5);
    }

    #[test]
    fn clearing_one_watermelon_takes_31_pops() {
        // 1 + 2 + 4 + 8 + 16 — the whole subtree has to be popped.
        assert_eq!(subtree_hits(FruitKind::Watermelon), 31);
    }

    #[test]
    fn clearing_one_durian_takes_its_armour_plus_four_watermelons() {
        assert_eq!(
            subtree_hits(FruitKind::Durian),
            DURIAN_ARMOUR + DURIAN_PAYLOAD as u32 * 31
        );
    }

    #[test]
    fn smaller_tiers_move_faster() {
        for t in 1..=5u8 {
            let bigger = FruitKind::from_tier(t);
            let smaller = FruitKind::from_tier(t - 1);
            assert!(smaller.speed_scale() > bigger.speed_scale());
        }
    }

    #[test]
    fn bigger_tiers_are_bigger() {
        for t in 1..=5u8 {
            let bigger = FruitKind::from_tier(t);
            let smaller = FruitKind::from_tier(t - 1);
            assert!(bigger.radius() > smaller.radius());
        }
    }

    #[test]
    fn leak_cost_climbs_with_tier() {
        assert_eq!(FruitKind::Blueberry.leak_cost(), 1);
        assert_eq!(FruitKind::Watermelon.leak_cost(), 5);
        assert_eq!(FruitKind::Durian.leak_cost(), 6);
    }

    #[test]
    fn only_the_boss_is_armoured() {
        for t in 0..=4u8 {
            let kind = FruitKind::from_tier(t);
            assert_eq!(kind.armour(), 1, "an ordinary fruit must die to one hit");
            assert!(!kind.is_boss());
            assert_eq!(kind.split_count(), 2, "the ladder below the boss is binary");
        }
        assert!(FruitKind::Durian.armour() > 1);
        assert!(FruitKind::Durian.is_boss());
    }

    #[test]
    fn an_ordinary_fruit_bursts_on_its_first_hit() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let mut f = Fruit::new(FruitKind::Watermelon, 100.0, &path, 1.0);
        assert!(f.take_hit(), "a watermelon should not survive a hit");
    }

    #[test]
    fn a_durian_soaks_its_whole_armour_before_bursting() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let mut f = Fruit::new(FruitKind::Durian, 100.0, &path, 1.0);

        for hit in 1..DURIAN_ARMOUR {
            assert!(!f.take_hit(), "burst early, on hit {hit}");
        }
        assert!(f.take_hit(), "the last point of armour did not burst it");

        // Overkill lands on an already-burst fruit when several shots connect
        // in one frame. It must not report a second burst, or the fruit would
        // be removed from the field twice.
        assert!(!f.take_hit(), "reported bursting twice");
        assert_eq!(f.hp, 0);
    }

    #[test]
    fn the_health_bar_tracks_the_armour_down() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let mut f = Fruit::new(FruitKind::Durian, 0.0, &path, 1.0);
        assert_eq!(f.health_fraction(), 1.0);

        for _ in 0..DURIAN_ARMOUR / 2 {
            f.take_hit();
        }
        assert!((f.health_fraction() - 0.5).abs() < 0.01);

        // An unarmoured fruit is always full, which is why it gets no bar.
        let g = Fruit::new(FruitKind::Lime, 0.0, &path, 1.0);
        assert_eq!(g.health_fraction(), 1.0);
    }

    #[test]
    fn a_durian_bursts_into_a_fanned_out_cluster_of_watermelons() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(2000.0, 0.0)]);
        let f = Fruit::new(FruitKind::Durian, 900.0, &path, 1.0);
        let kids = f.split(&path);

        assert_eq!(kids.len(), DURIAN_PAYLOAD);
        assert!(kids.iter().all(|k| k.kind == FruitKind::Watermelon));

        // Each child trails the one before it, far enough apart that four 30px
        // watermelons don't land as a single blob.
        for pair in kids.windows(2) {
            let gap = pair[0].dist - pair[1].dist;
            assert!(gap > 0.0, "children stacked on the same spot");
            assert!(
                gap >= FruitKind::Watermelon.radius(),
                "cluster is too tightly packed to read: {gap}px"
            );
        }
    }

    #[test]
    fn a_split_yields_two_children_one_trailing() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let f = Fruit::new(FruitKind::Watermelon, 200.0, &path, 1.0);
        let kids = f.split(&path);

        assert_eq!(kids.len(), 2);
        assert!(kids.iter().all(|k| k.kind == FruitKind::Orange));
        assert_eq!(kids[0].dist, 200.0);
        assert_eq!(kids[1].dist, 200.0 - SPLIT_TRAIL);
    }

    #[test]
    fn a_blueberry_split_yields_nothing() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let f = Fruit::new(FruitKind::Blueberry, 10.0, &path, 1.0);
        assert!(f.split(&path).is_empty());
    }

    #[test]
    fn splitting_at_the_start_does_not_produce_negative_distance() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let f = Fruit::new(FruitKind::Watermelon, 0.0, &path, 1.0);
        assert!(f.split(&path).iter().all(|k| k.dist >= 0.0));
    }

    #[test]
    fn a_chilled_fruit_travels_slower() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);

        let mut normal = Fruit::new(FruitKind::Lime, 0.0, &path, 1.0);
        let mut chilled = Fruit::new(FruitKind::Lime, 0.0, &path, 1.0);
        chilled.chill(0.45, 1.0);

        normal.update(0.5, &path);
        chilled.update(0.5, &path);

        assert!(chilled.dist < normal.dist);
        assert!((chilled.dist - normal.dist * 0.45).abs() < 0.001);
    }

    #[test]
    fn overlapping_freezers_keep_the_strongest_chill() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(500.0, 0.0)]);
        let mut f = Fruit::new(FruitKind::Lime, 0.0, &path, 1.0);

        f.chill(0.45, 1.6);
        f.chill(0.25, 1.0);

        // Strongest slow wins, and the longer duration is kept.
        assert_eq!(f.slow_factor, 0.25);
        assert_eq!(f.slow_timer, 1.6);

        // A weaker Freezer must not undo a stronger one already in effect.
        f.chill(0.45, 1.0);
        assert_eq!(f.slow_factor, 0.25);
    }

    #[test]
    fn a_chill_wearing_off_restores_full_speed() {
        let path = Path::new(vec![vec2(0.0, 0.0), vec2(9000.0, 0.0)]);
        let mut f = Fruit::new(FruitKind::Lime, 0.0, &path, 1.0);
        f.chill(0.25, 0.1);

        f.update(0.2, &path);

        assert_eq!(f.slow_factor, 1.0);
        assert!(f.slow_timer <= 0.0);
    }
}
