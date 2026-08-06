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

/// The three tower types available in the shop.
#[derive(Clone, Copy, PartialEq)]
pub enum TowerKind {
    SeedShooter,
    Blender,
    Freezer,
}

impl TowerKind {
    /// Shop order, also the order of the 1/2/3 hotkeys.
    pub const ALL: [TowerKind; 3] = [
        TowerKind::SeedShooter,
        TowerKind::Blender,
        TowerKind::Freezer,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            TowerKind::SeedShooter => "Seed Shooter",
            TowerKind::Blender => "Blender",
            TowerKind::Freezer => "Freezer",
        }
    }

    /// Purchase price — always the Lv1 cost.
    pub fn cost(&self) -> u32 {
        match self {
            TowerKind::SeedShooter => 90,
            TowerKind::Blender => 170,
            TowerKind::Freezer => 140,
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
            _ => "fully upgraded",
        }
    }

    /// Radius within which the tower will engage fruit.
    pub fn range(&self, level: u8) -> f32 {
        let table: [f32; 3] = match self {
            TowerKind::SeedShooter => [135.0, 158.0, 180.0],
            TowerKind::Blender => [110.0, 128.0, 148.0],
            TowerKind::Freezer => [120.0, 142.0, 165.0],
        };
        table[level_index(level)]
    }

    /// Seconds between shots (or pulses, for the Freezer).
    pub fn cooldown(&self, level: u8) -> f32 {
        let table: [f32; 3] = match self {
            TowerKind::SeedShooter => [0.45, 0.33, 0.24],
            TowerKind::Blender => [1.10, 0.88, 0.70],
            TowerKind::Freezer => [1.40, 1.15, 0.95],
        };
        table[level_index(level)]
    }

    /// Splash radius of this tower's shots. Zero means single-target.
    pub fn splash_radius(&self, level: u8) -> f32 {
        match self {
            TowerKind::Blender => [58.0, 72.0, 90.0][level_index(level)],
            _ => 0.0,
        }
    }

    /// How many fruit the tower engages per volley.
    pub fn shots(&self, level: u8) -> usize {
        match self {
            // The Lv3 Seed Shooter splits its fire across the two lead fruit.
            TowerKind::SeedShooter => [1, 1, 2][level_index(level)],
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
        }
    }

    /// One-line role summary, shown on the shop button.
    pub fn blurb(&self) -> &'static str {
        match self {
            TowerKind::SeedShooter => "fast single shot",
            TowerKind::Blender => "splash damage",
            TowerKind::Freezer => "slows, no damage",
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
    fn only_the_seed_shooter_ever_fires_twice() {
        assert_eq!(TowerKind::SeedShooter.shots(3), 2);
        assert_eq!(TowerKind::Blender.shots(3), 1);
        assert_eq!(TowerKind::Freezer.shots(3), 1);
    }
}
