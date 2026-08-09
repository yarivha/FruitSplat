// =============================================================================
// tower.rs — the defences the player buys, places and upgrades
//
// Three roles: the Seed Shooter is cheap single-target DPS, the Blender throws
// splash that punishes clustered splits, and the Freezer deals no damage at all
// but chills fruit in range so the other two get more shots off.
//
// Every tower upgrades along one linear track, Lv1 → Lv3. Each kind's upgrades
// lean into what it already does — the Seed Shooter ends up firing at two fruit
// at once, the Blender's splash widens, the Freezer's chill deepens.
// Targeting and firing are driven from main.rs, which owns the fruit list.
// =============================================================================

use macroquad::prelude::*;

/// Drawing and collision radius of a tower base.
pub const TOWER_RADIUS: f32 = 20.0;
/// How far a tower must sit from the track centreline to be placeable.
/// Set so a tower's body just meets the edge of the 44px-wide track rather than
/// overlapping it: half the track width, plus the tower radius.
pub const PATH_CLEARANCE: f32 = 44.0;
/// Highest level a tower can reach.
pub const MAX_LEVEL: u8 = 3;
/// Fraction of everything invested that selling a tower gives back.
const SELL_REFUND: f32 = 0.6;

/// The tower types available in the shop.
#[derive(Clone, Copy, PartialEq)]
pub enum TowerKind {
    SeedShooter,
    Blender,
    Freezer,
    KnifeThrower,
    SpikeLayer,
    /// Three seeds at three separate fruit, every shot.
    TripleSeeder,
    /// Lobs a shell that clears everything standing near where it lands.
    BombLobber,
}

impl TowerKind {
    /// Shop order, also the order of the 1–7 hotkeys.
    pub const ALL: [TowerKind; 7] = [
        TowerKind::SeedShooter,
        TowerKind::Blender,
        TowerKind::Freezer,
        TowerKind::KnifeThrower,
        TowerKind::SpikeLayer,
        TowerKind::TripleSeeder,
        TowerKind::BombLobber,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            TowerKind::SeedShooter => "Seed Shooter",
            TowerKind::Blender => "Blender",
            TowerKind::Freezer => "Freezer",
            TowerKind::KnifeThrower => "Knife Thrower",
            TowerKind::SpikeLayer => "Spike Layer",
            TowerKind::TripleSeeder => "Triple Seeder",
            TowerKind::BombLobber => "Bomb Lobber",
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Abbreviated name for the shop buttons, which are too narrow for the full
    // ones now that there are five of them.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn short_name(&self) -> &'static str {
        match self {
            TowerKind::SeedShooter => "Seeds",
            TowerKind::Blender => "Blender",
            TowerKind::Freezer => "Freezer",
            TowerKind::KnifeThrower => "Knives",
            TowerKind::SpikeLayer => "Spikes",
            TowerKind::TripleSeeder => "Triple",
            TowerKind::BombLobber => "Bombs",
        }
    }

    /// Purchase price — always the Lv1 cost.
    pub fn cost(&self) -> u32 {
        match self {
            TowerKind::SeedShooter => 90,
            TowerKind::Blender => 170,
            TowerKind::Freezer => 140,
            TowerKind::KnifeThrower => 130,
            TowerKind::SpikeLayer => 150,
            // Dearer than anything else. Three shots a volley is the answer to
            // a crowd, and being able to buy that answer early would flatten
            // the whole reason the Blender and the Knife Thrower differ.
            TowerKind::TripleSeeder => 260,
            // The most expensive thing in the shop. It answers a crowd outright
            // rather than working through one, and a price that let it arrive
            // before the split ladder starts producing crowds would leave
            // nothing for the Blender or the Knife Thrower to be better at.
            TowerKind::BombLobber => 320,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Price of moving from `level` to the next one. None means already maxed.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn upgrade_cost(&self, level: u8) -> Option<u32> {
        let costs: [u32; 2] = match self {
            TowerKind::SeedShooter => [70, 150],
            TowerKind::Blender => [120, 240],
            TowerKind::Freezer => [100, 200],
            TowerKind::KnifeThrower => [110, 220],
            TowerKind::SpikeLayer => [130, 260],
            TowerKind::TripleSeeder => [160, 280],
            TowerKind::BombLobber => [200, 340],
        };
        match level {
            1 => Some(costs[0]),
            2 => Some(costs[1]),
            _ => None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // What the next upgrade actually buys, for the tower info panel.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn upgrade_label(&self, level: u8) -> &'static str {
        match (self, level) {
            (TowerKind::SeedShooter, 1) => "faster, longer reach",
            (TowerKind::SeedShooter, 2) => "twin shot",
            (TowerKind::Blender, 1) => "wider splash",
            (TowerKind::Blender, 2) => "widest splash, faster",
            (TowerKind::Freezer, 1) => "deeper chill",
            (TowerKind::Freezer, 2) => "deepest chill, longer",
            (TowerKind::KnifeThrower, 1) => "more pierce, faster",
            (TowerKind::KnifeThrower, 2) => "deepest pierce",
            (TowerKind::SpikeLayer, 1) => "bigger piles, faster",
            (TowerKind::SpikeLayer, 2) => "biggest piles",
            (TowerKind::TripleSeeder, 1) => "a fourth seed, faster",
            (TowerKind::TripleSeeder, 2) => "a fifth seed, faster",
            (TowerKind::BombLobber, 1) => "wider blast, faster",
            (TowerKind::BombLobber, 2) => "widest blast, faster",
            _ => "fully upgraded",
        }
    }

    /// Radius within which the tower will engage fruit.
    pub fn range(&self, level: u8) -> f32 {
        let table: [f32; 3] = match self {
            TowerKind::SeedShooter => [135.0, 158.0, 180.0],
            TowerKind::Blender => [110.0, 128.0, 148.0],
            TowerKind::Freezer => [120.0, 142.0, 165.0],
            TowerKind::KnifeThrower => [145.0, 165.0, 185.0],
            TowerKind::SpikeLayer => [120.0, 138.0, 158.0],
            TowerKind::TripleSeeder => [150.0, 172.0, 195.0],
            // Short. The blast is what makes it worth its price, so reach is
            // where it pays for that — a Bomb Lobber has to be put where the
            // crowd will be rather than parked somewhere it can see everything.
            TowerKind::BombLobber => [125.0, 142.0, 160.0],
        };
        table[level_index(level)]
    }

    /// Seconds between shots (or pulses, for the Freezer).
    pub fn cooldown(&self, level: u8) -> f32 {
        let table: [f32; 3] = match self {
            TowerKind::SeedShooter => [0.45, 0.33, 0.24],
            TowerKind::Blender => [1.10, 0.88, 0.70],
            TowerKind::Freezer => [1.40, 1.15, 0.95],
            TowerKind::KnifeThrower => [0.75, 0.60, 0.46],
            // For the Spike Layer this is the gap between laying piles: it lays
            // one on this timer for the whole length of a wave.
            //
            // Slower than every other tower's cadence by design. A pile is not
            // a shot — it stays on the track until fruit wear it away, so the
            // tower's output is the *stock* of spikes standing on its stretch,
            // not the rate it lays them. At 2.20s it saturated everything it
            // covered inside the first third of a wave and then had nowhere
            // left to lay, which made the timer meaningless and the upgrades
            // along with it. At this pace the stock builds over the whole wave.
            TowerKind::SpikeLayer => [4.50, 3.40, 2.60],
            // The same cadence as a Seed Shooter, so the volley is simply
            // three of its shots where it fires one. A longer cooldown was
            // cancelling the triple out: at Lv3 the sustained rate came to
            // exactly a Seed Shooter's, for two and a half times the money.
            TowerKind::TripleSeeder => [0.45, 0.38, 0.32],
            // By a distance the slowest thing in the shop. One shell clears a
            // whole cluster, so the cost of that is the wait: between lobs the
            // fruit it did not catch keep walking, and something else has to
            // answer them.
            TowerKind::BombLobber => [2.60, 2.10, 1.70],
        };
        table[level_index(level)]
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How many fruit one dropped pile of spikes can pop before it's used up.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn spike_charges(&self, level: u8) -> u32 {
        match self {
            TowerKind::SpikeLayer => [4, 6, 9][level_index(level)],
            _ => 0,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How many fruit one shot can pop before it is used up. Only the Knife
    // Thrower exceeds 1 — its knives pass straight through and keep travelling,
    // which is what makes it worth building on the tight zigzag routes where
    // fruit line up single file.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn pierce(&self, level: u8) -> u32 {
        match self {
            TowerKind::KnifeThrower => [3, 4, 6][level_index(level)],
            _ => 1,
        }
    }

    /// Splash radius of this tower's shots. Zero means single-target.
    pub fn splash_radius(&self, level: u8) -> f32 {
        match self {
            TowerKind::Blender => [58.0, 72.0, 90.0][level_index(level)],
            // Roughly double the Blender's, which is the whole difference
            // between the two: the Blender catches the fruit around the one it
            // hit, the Bomb Lobber clears the ground the cluster is standing on.
            TowerKind::BombLobber => [110.0, 135.0, 165.0][level_index(level)],
            _ => 0.0,
        }
    }

    /// How many fruit the tower engages per volley.
    pub fn shots(&self, level: u8) -> usize {
        match self {
            // The Lv3 Seed Shooter splits its fire across the two lead fruit.
            TowerKind::SeedShooter => [1, 1, 2][level_index(level)],
            // The Triple Seeder's whole point: three separate fruit engaged at
            // once from the first level, and a seed more at each upgrade.
            TowerKind::TripleSeeder => [3, 4, 5][level_index(level)],
            _ => 1,
        }
    }

    /// Speed multiplier applied to chilled fruit. Lower is a stronger slow.
    pub fn slow_factor(&self, level: u8) -> f32 {
        match self {
            TowerKind::Freezer => [0.45, 0.35, 0.25][level_index(level)],
            _ => 1.0,
        }
    }

    /// Seconds of slow applied by one Freezer pulse.
    pub fn freeze_duration(&self, level: u8) -> f32 {
        match self {
            TowerKind::Freezer => [1.6, 1.9, 2.3][level_index(level)],
            _ => 0.0,
        }
    }

    /// Body colour of the tower base.
    pub fn color(&self) -> Color {
        match self {
            TowerKind::SeedShooter => Color::new(0.58, 0.42, 0.26, 1.0),
            TowerKind::Blender => Color::new(0.62, 0.66, 0.72, 1.0),
            TowerKind::Freezer => Color::new(0.55, 0.80, 0.90, 1.0),
            TowerKind::KnifeThrower => Color::new(0.42, 0.47, 0.60, 1.0),
            TowerKind::SpikeLayer => Color::new(0.60, 0.34, 0.28, 1.0),
            TowerKind::TripleSeeder => Color::new(0.36, 0.52, 0.38, 1.0),
            // Gunmetal with a little violet in it, so it is not mistaken for
            // the Knife Thrower's blue-grey at a glance across the field.
            TowerKind::BombLobber => Color::new(0.34, 0.29, 0.38, 1.0),
        }
    }

    /// Very short role summary for the shop button, which is narrow. Kept to a
    /// single word so the cost and blurb fit on one line at five buttons wide.
    pub fn blurb(&self) -> &'static str {
        match self {
            TowerKind::SeedShooter => "single",
            TowerKind::Blender => "splash",
            TowerKind::Freezer => "slows",
            TowerKind::KnifeThrower => "pierces",
            TowerKind::SpikeLayer => "spikes",
            TowerKind::TripleSeeder => "3 at once",
            TowerKind::BombLobber => "clears",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Turn a 1-based level into a stat-table index, clamped so a bad level can
// never index out of bounds.
// ─────────────────────────────────────────────────────────────────────────────
fn level_index(level: u8) -> usize {
    (level.clamp(1, MAX_LEVEL) - 1) as usize
}

/// A placed tower. `angle` is kept purely so the barrel can face its target;
/// `invested` accumulates the purchase and every upgrade, for the sell refund.
///
/// `id` is a stable handle that survives other towers being sold, unlike a
/// position in the tower list. Projectiles carry it so kills can be credited
/// back to whichever tower actually fired the shot.
pub struct Tower {
    pub kind: TowerKind,
    pub pos: Vec2,
    pub cooldown: f32,
    pub angle: f32,
    pub level: u8,
    pub invested: u32,
    pub id: u32,

    /// Projectiles fired, or pulses emitted for a Freezer.
    pub shots_fired: u32,
    /// Fruit popped by this tower's shots. Always 0 for a Freezer.
    pub kills: u32,
    /// Fruit chilled across every pulse. Freezer only.
    pub chills: u32,
}

impl Tower {
    pub fn new(kind: TowerKind, pos: Vec2, id: u32) -> Self {
        Tower {
            kind,
            pos,
            cooldown: 0.0,
            angle: 0.0,
            level: 1,
            invested: kind.cost(),
            id,
            shots_fired: 0,
            kills: 0,
            chills: 0,
        }
    }

    pub fn range(&self) -> f32 {
        self.kind.range(self.level)
    }

    pub fn fire_cooldown(&self) -> f32 {
        self.kind.cooldown(self.level)
    }

    pub fn splash_radius(&self) -> f32 {
        self.kind.splash_radius(self.level)
    }

    pub fn shots(&self) -> usize {
        self.kind.shots(self.level)
    }

    pub fn pierce(&self) -> u32 {
        self.kind.pierce(self.level)
    }

    pub fn spike_charges(&self) -> u32 {
        self.kind.spike_charges(self.level)
    }

    pub fn slow_factor(&self) -> f32 {
        self.kind.slow_factor(self.level)
    }

    pub fn freeze_duration(&self) -> f32 {
        self.kind.freeze_duration(self.level)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Cost of the next upgrade, or None when the tower is maxed out.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn upgrade_cost(&self) -> Option<u32> {
        self.kind.upgrade_cost(self.level)
    }

    pub fn upgrade_label(&self) -> &'static str {
        self.kind.upgrade_label(self.level)
    }

    pub fn maxed(&self) -> bool {
        self.level >= MAX_LEVEL
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Apply a purchased upgrade. `cost` is folded into the sell value so the
    // refund tracks everything the player actually spent here.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn upgrade(&mut self, cost: u32) {
        if self.maxed() {
            return;
        }
        self.level += 1;
        self.invested += cost;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // What selling this tower pays back.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn sell_value(&self) -> u32 {
        (self.invested as f32 * SELL_REFUND) as u32
    }
}

/// Collision half-width of a spike pile, measured along the track.
pub const SPIKE_RADIUS: f32 = 14.0;
/// Minimum gap along the track between two piles, so a tower spreads its
/// spikes out instead of stacking them all on one spot.
///
/// This is also what bounds a Spike Layer, now that it lays on a timer for the
/// whole wave rather than up to a fixed allowance: a tower can only fill the
/// lanes it reaches this densely, and then has nowhere left to lay until the
/// fruit chew a gap open.
pub const SPIKE_SPACING: f32 = 34.0;

/// A pile of spikes sitting on the track.
///
/// A pile belongs to exactly one lane, and `dist` is its position *along that
/// lane*, not a world coordinate. Fruit are matched on both. Two things would go
/// wrong with euclidean distance instead: on the switchback routes two stretches
/// of the same lane run within a few dozen pixels of each other, and on a
/// two-entrance route the two lanes pass close together on their way to the
/// shared exit. In both cases a pile must only touch fruit actually walking over
/// it, not fruit that merely happens to be nearby.
pub struct SpikePile {
    pub pos: Vec2,
    /// Which lane this pile sits on. A pile never touches another lane's fruit.
    pub lane: usize,
    pub dist: f32,
    /// Remaining pops. The number of spikes drawn tracks this, so a worn pile
    /// is visibly running down without needing to store its original size.
    pub charges: u32,
    /// Stable id of the Spike Layer that dropped this, for kill credit.
    pub owner: u32,
    pub rot: f32,
}

impl SpikePile {
    pub fn new(pos: Vec2, lane: usize, dist: f32, charges: u32, owner: u32, rot: f32) -> Self {
        SpikePile {
            pos,
            lane,
            dist,
            charges,
            owner,
            rot,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Is a fruit on `lane` at `fruit_dist` with radius `radius` standing on this
    // pile? A fruit on any other lane never is, however close it looks.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn covers(&self, lane: usize, fruit_dist: f32, radius: f32) -> bool {
        self.lane == lane && (fruit_dist - self.dist).abs() <= radius + SPIKE_RADIUS
    }

    pub fn spent(&self) -> bool {
        self.charges == 0
    }
}

/// The expanding ring drawn when a Freezer pulses. Purely cosmetic.
pub struct Pulse {
    pub pos: Vec2,
    pub max_radius: f32,
    pub life: f32,
    pub max_life: f32,
}

impl Pulse {
    pub fn new(pos: Vec2, max_radius: f32) -> Self {
        Pulse {
            pos,
            max_radius,
            life: 0.45,
            max_life: 0.45,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.life -= dt;
    }

    pub fn finished(&self) -> bool {
        self.life <= 0.0
    }

    /// 0.0 at the moment of firing, 1.0 when the ring has fully expanded.
    pub fn progress(&self) -> f32 {
        1.0 - (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tower_maxes_out_at_level_three() {
        for kind in TowerKind::ALL {
            assert!(kind.upgrade_cost(1).is_some());
            assert!(kind.upgrade_cost(2).is_some());
            assert!(kind.upgrade_cost(MAX_LEVEL).is_none());
        }
    }

    #[test]
    fn upgrading_never_makes_a_tower_worse() {
        for kind in TowerKind::ALL {
            for level in 1..MAX_LEVEL {
                let (lo, hi) = (level, level + 1);
                assert!(kind.range(hi) >= kind.range(lo), "range regressed");
                assert!(kind.cooldown(hi) <= kind.cooldown(lo), "rate regressed");
                assert!(
                    kind.splash_radius(hi) >= kind.splash_radius(lo),
                    "splash regressed"
                );
                // Lower slow_factor is a stronger slow, so it must not rise.
                assert!(kind.slow_factor(hi) <= kind.slow_factor(lo));
                assert!(kind.freeze_duration(hi) >= kind.freeze_duration(lo));
                assert!(kind.pierce(hi) >= kind.pierce(lo), "pierce regressed");
                assert!(
                    kind.spike_charges(hi) >= kind.spike_charges(lo),
                    "spike charges regressed"
                );
            }
        }
    }

    #[test]
    fn only_the_spike_layer_lays_spikes() {
        // Driven off ALL rather than a hand-written list, so a tower added
        // later is covered without anyone remembering to add it here.
        for level in 1..=MAX_LEVEL {
            for kind in TowerKind::ALL {
                let lays = kind == TowerKind::SpikeLayer;
                assert_eq!(
                    kind.spike_charges(level) > 0,
                    lays,
                    "{} spike charges at Lv{level}",
                    kind.name()
                );
            }
        }
    }

    #[test]
    fn a_pile_only_covers_fruit_near_it_along_the_track() {
        let pile = SpikePile::new(Vec2::ZERO, 0, 500.0, 4, 0, 0.0);

        assert!(pile.covers(0, 500.0, 10.0), "fruit on the pile missed it");
        assert!(
            pile.covers(0, 500.0 + SPIKE_RADIUS + 9.0, 10.0),
            "edge case missed"
        );
        // Far along the route, even though a switchback could put this fruit
        // physically close to the pile.
        assert!(!pile.covers(0, 700.0, 10.0), "pile reached down the track");
        assert!(!pile.covers(0, 300.0, 10.0));
    }

    #[test]
    fn a_pile_never_touches_another_lane() {
        // On a two-entrance route both lanes converge on the same exit, so a
        // pile near the end of one sits close to fruit walking the other. Same
        // distance, different lane: it must not connect.
        let pile = SpikePile::new(Vec2::ZERO, 0, 500.0, 4, 0, 0.0);

        assert!(pile.covers(0, 500.0, 10.0), "own lane missed");
        assert!(!pile.covers(1, 500.0, 10.0), "pile reached across lanes");
    }

    #[test]
    fn a_pile_is_spent_only_once_its_charges_run_out() {
        let mut pile = SpikePile::new(Vec2::ZERO, 0, 0.0, 2, 0, 0.0);
        assert!(!pile.spent());
        pile.charges -= 1;
        assert!(!pile.spent());
        pile.charges -= 1;
        assert!(pile.spent());
    }

    #[test]
    fn only_the_knife_thrower_pierces() {
        for level in 1..=MAX_LEVEL {
            assert!(TowerKind::KnifeThrower.pierce(level) > 1);
            for kind in [
                TowerKind::SeedShooter,
                TowerKind::Blender,
                TowerKind::Freezer,
            ] {
                assert_eq!(kind.pierce(level), 1, "{} should not pierce", kind.name());
            }
        }
    }

    #[test]
    fn every_tower_kind_has_a_distinct_name_and_colour() {
        for (i, a) in TowerKind::ALL.iter().enumerate() {
            for b in TowerKind::ALL.iter().skip(i + 1) {
                assert_ne!(a.name(), b.name());
                let (ca, cb) = (a.color(), b.color());
                assert!(
                    (ca.r - cb.r).abs() + (ca.g - cb.g).abs() + (ca.b - cb.b).abs() > 0.05,
                    "{} and {} look too alike",
                    a.name(),
                    b.name()
                );
            }
        }
    }

    #[test]
    fn level_index_clamps_instead_of_panicking() {
        // A level of 0 or one past the max must still index a real stat row.
        for kind in TowerKind::ALL {
            assert_eq!(kind.range(0), kind.range(1));
            assert_eq!(kind.range(99), kind.range(MAX_LEVEL));
        }
    }

    #[test]
    fn selling_refunds_a_fraction_of_everything_invested() {
        let mut t = Tower::new(TowerKind::SeedShooter, Vec2::ZERO, 0);
        assert_eq!(t.invested, 90);
        assert_eq!(t.sell_value(), 54); // 60% of 90

        let cost = t.upgrade_cost().unwrap();
        t.upgrade(cost);
        assert_eq!(t.level, 2);
        assert_eq!(t.invested, 160); // 90 + 70
        assert_eq!(t.sell_value(), 96); // 60% of 160
    }

    #[test]
    fn selling_never_pays_more_than_was_spent() {
        for kind in TowerKind::ALL {
            let mut t = Tower::new(kind, Vec2::ZERO, 0);
            while let Some(cost) = t.upgrade_cost() {
                t.upgrade(cost);
            }
            assert!(t.sell_value() < t.invested);
        }
    }

    #[test]
    fn a_maxed_tower_cannot_be_upgraded_further() {
        let mut t = Tower::new(TowerKind::Blender, Vec2::ZERO, 0);
        while let Some(cost) = t.upgrade_cost() {
            t.upgrade(cost);
        }
        assert_eq!(t.level, MAX_LEVEL);

        // Upgrading a maxed tower must be inert, not push it past the table.
        let before = t.invested;
        t.upgrade(999);
        assert_eq!(t.level, MAX_LEVEL);
        assert_eq!(t.invested, before);
    }

    #[test]
    fn only_the_multi_target_towers_engage_more_than_one_fruit() {
        // The Seed Shooter earns a second target at Lv3; the Triple Seeder is
        // built around engaging several from the start and is priced for it.
        // Everything else is single-target at every level, and driving this off
        // ALL means a new tower cannot quietly join them.
        assert_eq!(TowerKind::SeedShooter.shots(1), 1);
        assert_eq!(TowerKind::SeedShooter.shots(3), 2);
        assert_eq!(TowerKind::TripleSeeder.shots(1), 3);
        assert_eq!(TowerKind::TripleSeeder.shots(3), 5);

        let multi = [TowerKind::SeedShooter, TowerKind::TripleSeeder];
        for kind in TowerKind::ALL {
            if multi.contains(&kind) {
                continue;
            }
            for level in 1..=MAX_LEVEL {
                assert_eq!(kind.shots(level), 1, "{} at Lv{level}", kind.name());
            }
        }
    }

    #[test]
    fn the_triple_seeder_puts_out_far_more_than_a_seed_shooter() {
        // The whole reason to buy it, and it did not hold. A cooldown long
        // enough to cancel the triple out left its sustained rate at Lv3
        // *identical* to a Seed Shooter's, for two and a half times the money.
        for level in 1..=MAX_LEVEL {
            let rate = |k: TowerKind| k.shots(level) as f32 / k.cooldown(level);
            let (triple, seed) = (rate(TowerKind::TripleSeeder), rate(TowerKind::SeedShooter));
            assert!(
                triple >= seed * 1.8,
                "Lv{level}: Triple Seeder {triple:.2}/s against Seed Shooter {seed:.2}/s"
            );
        }
    }

    #[test]
    fn a_triple_seeder_is_worth_more_than_its_price_in_seed_shooters() {
        // Being dearest is fine; being worse value than the cheapest tower is
        // not, because then buying it is simply a mistake. One Triple Seeder
        // once put out 4.3 shots a second where the $270 of Seed Shooters it
        // costs put out 6.7.
        let per_dollar = |k: TowerKind| k.shots(1) as f32 / k.cooldown(1) / k.cost() as f32;
        let (triple, seed) = (
            per_dollar(TowerKind::TripleSeeder),
            per_dollar(TowerKind::SeedShooter),
        );
        assert!(
            triple >= seed,
            "the Triple Seeder buys {triple:.4} shots/s per dollar, the Seed Shooter {seed:.4}"
        );
    }

    #[test]
    fn answering_a_crowd_outright_costs_more_than_working_through_one() {
        // The Triple Seeder and the Bomb Lobber both answer a crowd on their
        // own rather than chipping at it. Being able to buy either early would
        // flatten the reason the Blender and the Knife Thrower differ from each
        // other, so both have to sit above everything that does not.
        let crowd = [TowerKind::TripleSeeder, TowerKind::BombLobber];

        for answer in crowd {
            for kind in TowerKind::ALL {
                if crowd.contains(&kind) {
                    continue;
                }
                assert!(
                    answer.cost() > kind.cost(),
                    "the {} is not dearer than the {}",
                    answer.name(),
                    kind.name()
                );
            }
        }
    }

    #[test]
    fn the_bomb_lobber_is_the_dearest_and_the_slowest_of_the_shooters() {
        // It is the most absolute answer in the game — everything standing in
        // the blast comes off the track at once — so it pays for that twice, in
        // price and in how long it waits between shells. Either one alone would
        // leave it strictly better than the Triple Seeder rather than a
        // different trade.
        //
        // The Spike Layer is left out of the rate half deliberately. Its
        // cooldown is the gap between laying piles that then sit on the track
        // until fruit wear them away, not the gap between shots, and at 4.50s
        // it is the slower number without being the slower tower.
        let bomb = TowerKind::BombLobber;

        for kind in TowerKind::ALL {
            if kind == bomb {
                continue;
            }
            assert!(
                bomb.cost() > kind.cost(),
                "the Bomb Lobber is not dearer than the {}",
                kind.name()
            );
            if kind == TowerKind::SpikeLayer {
                continue;
            }
            for level in 1..=MAX_LEVEL {
                assert!(
                    bomb.cooldown(level) > kind.cooldown(level),
                    "at Lv{level} the Bomb Lobber fires no slower than the {}",
                    kind.name()
                );
            }
        }
    }

    #[test]
    fn the_bomb_lobber_clears_far_more_ground_than_the_blender() {
        // The two are the only splash in the game and the difference between
        // them has to be plain, or the dearer one is just a worse Blender.
        for level in 1..=MAX_LEVEL {
            let (bomb, blender) = (
                TowerKind::BombLobber.splash_radius(level),
                TowerKind::Blender.splash_radius(level),
            );
            assert!(
                bomb >= blender * 1.7,
                "at Lv{level} the blast is {bomb} against the Blender's {blender}"
            );
        }
    }

    #[test]
    fn the_bomb_lobber_reaches_less_far_than_it_clears() {
        // What makes it a placement puzzle rather than a turret: the ground it
        // covers is wider than the ground it can see, so it has to be put where
        // the crowd will be rather than somewhere with a good view.
        for level in 1..=MAX_LEVEL {
            let kind = TowerKind::BombLobber;
            assert!(
                kind.splash_radius(level) * 2.0 > kind.range(level),
                "at Lv{level} the blast no longer spans a useful part of the range"
            );
        }
    }

    #[test]
    fn every_tower_can_be_armed_from_the_keyboard() {
        // The shop zips tower kinds against NUMBER_KEYS, so a seventh tower
        // added without a seventh key would simply be unreachable from the
        // keyboard — silently, since zip stops at the shorter side. That is
        // exactly how a route once ended up with a card and no key.
        assert!(
            TowerKind::ALL.len() <= crate::NUMBER_KEYS.len(),
            "{} towers but only {} number keys",
            TowerKind::ALL.len(),
            crate::NUMBER_KEYS.len()
        );
    }
}
