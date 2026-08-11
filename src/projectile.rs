// =============================================================================
// projectile.rs — the things towers fire
//
// Seeds are small and fast and pop exactly one fruit. Pulp blobs travel slower
// and pop everything inside their splash radius, which is what makes the Blender
// worth its price once fruit start splitting into clusters. Shells are the same
// idea taken to its end: slow, heavy, and a blast wide enough to take a whole
// cluster off the track at once.
// =============================================================================

use macroquad::prelude::*;

/// Seconds a projectile lives before expiring, so strays never leak memory.
const PROJECTILE_LIFE: f32 = 2.5;

/// What kind of shot this is, which decides collision size and splash.
#[derive(Clone, Copy, PartialEq)]
pub enum ProjectileKind {
    Seed,
    Pulp,
    Knife,
    Shell,
}

impl ProjectileKind {
    /// Collision radius of the projectile itself.
    pub fn radius(&self) -> f32 {
        match self {
            ProjectileKind::Seed => 4.0,
            ProjectileKind::Pulp => 9.0,
            ProjectileKind::Knife => 6.0,
            ProjectileKind::Shell => 11.0,
        }
    }

    /// How fast this shot travels, in pixels per second.
    pub fn speed(&self) -> f32 {
        match self {
            ProjectileKind::Seed => 430.0,
            ProjectileKind::Pulp => 270.0,
            ProjectileKind::Knife => 380.0,
            // The slowest thing in the air by a wide margin, and deliberately
            // so: a shell you can see coming is a shell whose blast the player
            // can read, and leading a slow shot into a moving crowd is the
            // skill the tower is sold on.
            ProjectileKind::Shell => 210.0,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ProjectileKind::Seed => Color::new(0.29, 0.20, 0.13, 1.0),
            ProjectileKind::Pulp => Color::new(0.85, 0.92, 0.55, 1.0),
            ProjectileKind::Knife => Color::new(0.88, 0.90, 0.95, 1.0),
            ProjectileKind::Shell => Color::new(0.20, 0.20, 0.23, 1.0),
        }
    }
}

/// A shot in flight. Travels in a straight line — no homing. `splash` is baked
/// in at fire time from the firing tower's level, so an upgraded Blender's shots
/// stay wide even if the tower is sold before they land.
pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub kind: ProjectileKind,
    pub life: f32,
    pub spin: f32,
    pub splash: f32,
    /// Whether this shot gets through a shielded fruit. Baked in at fire time
    /// from the tower's kind and level, so a shot already in the air keeps the
    /// ability of the tower that fired it.
    pub breaks_shield: bool,
    /// How many more fruit this shot can pop before it is used up. Seeds and
    /// pulp are 1; a knife carries several and keeps flying between hits.
    pub pierce: u32,
    /// Stable id of the tower that fired this, so kills can be credited back.
    /// The tower may be sold before the shot lands, in which case the credit is
    /// simply dropped.
    pub owner: u32,
}

impl Projectile {
    // ─────────────────────────────────────────────────────────────────────────
    // Fire from `origin` toward `target`. The shot does not track the fruit, so
    // fast fruit can be missed — that's the trade-off for cheap towers.
    // `splash` of zero makes it single-target.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn new(
        origin: Vec2,
        target: Vec2,
        kind: ProjectileKind,
        splash: f32,
        pierce: u32,
        owner: u32,
        breaks_shield: bool,
    ) -> Self {
        // Guard against a zero-length direction if the fruit is exactly on top
        // of the tower, which would produce a NaN velocity.
        let dir = (target - origin).normalize_or_zero();
        let dir = if dir == Vec2::ZERO {
            vec2(1.0, 0.0)
        } else {
            dir
        };

        Projectile {
            pos: origin,
            vel: dir * kind.speed(),
            kind,
            life: PROJECTILE_LIFE,
            spin: 0.0,
            splash,
            breaks_shield,
            // A shot that could pop nothing would hang around forever.
            pierce: pierce.max(1),
            owner,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Spend one hit's worth of pierce. Returns true once the shot is used up.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn consume_pierce(&mut self) -> bool {
        self.pierce = self.pierce.saturating_sub(1);
        self.pierce == 0
    }

    pub fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        self.life -= dt;
        self.spin += 420.0 * dt;
    }

    pub fn expired(&self) -> bool {
        self.life <= 0.0
    }
}

/// The expanding ring left where a shell lands. Purely cosmetic, but not
/// decoration: the blast is wide enough that without seeing it the player has no
/// way to learn what a Bomb Lobber actually covers, and placing one is entirely
/// a question of guessing that radius.
pub struct Blast {
    pub pos: Vec2,
    pub radius: f32,
    pub life: f32,
    pub max_life: f32,
}

impl Blast {
    pub fn new(pos: Vec2, radius: f32) -> Self {
        // Longer-lived than the Freezer's ring. A shell fires every couple of
        // seconds, so its mark can afford to linger and be read.
        Blast {
            pos,
            radius,
            life: 0.55,
            max_life: 0.55,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.life -= dt;
    }

    pub fn finished(&self) -> bool {
        self.life <= 0.0
    }

    /// 0.0 at the moment of the blast, 1.0 once it has fully expanded.
    pub fn progress(&self) -> f32 {
        1.0 - (self.life / self.max_life).clamp(0.0, 1.0)
    }
}
