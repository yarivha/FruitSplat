// =============================================================================
// tower.rs — the defences the player buys and places
//
// Three roles: the Seed Shooter is cheap single-target DPS, the Blender throws
// splash that punishes clustered splits, and the Freezer deals no damage at all
// but chills fruit in range so the other two get more shots off.
// Targeting and firing are driven from main.rs, which owns the fruit list.
// =============================================================================

use macroquad::prelude::*;

/// Drawing and collision radius of a tower base.
pub const TOWER_RADIUS: f32 = 20.0;
/// How far a tower must sit from the track centreline to be placeable.
/// Set so a tower's body just meets the edge of the 44px-wide track rather than
/// overlapping it: half the track width, plus the tower radius.
pub const PATH_CLEARANCE: f32 = 44.0;
/// Seconds of slow the Freezer applies on each pulse.
pub const FREEZE_DURATION: f32 = 1.6;

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

    pub fn cost(&self) -> u32 {
        match self {
            TowerKind::SeedShooter => 90,
            TowerKind::Blender => 170,
            TowerKind::Freezer => 140,
        }
    }

    /// Radius within which the tower will engage fruit.
    pub fn range(&self) -> f32 {
        match self {
            TowerKind::SeedShooter => 135.0,
            TowerKind::Blender => 110.0,
            TowerKind::Freezer => 120.0,
        }
    }

    /// Seconds between shots (or pulses, for the Freezer).
    pub fn cooldown(&self) -> f32 {
        match self {
            TowerKind::SeedShooter => 0.45,
            TowerKind::Blender => 1.1,
            TowerKind::Freezer => 1.4,
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

/// A placed tower. `angle` is kept purely so the barrel can face its target.
pub struct Tower {
    pub kind: TowerKind,
    pub pos: Vec2,
    pub cooldown: f32,
    pub angle: f32,
}

impl Tower {
    pub fn new(kind: TowerKind, pos: Vec2) -> Self {
        Tower {
            kind,
            pos,
            cooldown: 0.0,
            angle: 0.0,
        }
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
