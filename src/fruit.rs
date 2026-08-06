// =============================================================================
// fruit.rs — the fruit entities and the splat particles they leave behind
//
// A Fruit rises from below the bottom edge like a balloon, swaying sideways on a
// sine path until it is clicked (splat) or drifts off the top (escaped).
// Popping one spawns a Splat: a short-lived burst of gravity-affected particles.
// =============================================================================

use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Downward pull applied to splat particles, in pixels per second squared.
const PARTICLE_GRAVITY: f32 = 620.0;

/// The five fruit varieties. Smaller fruit rise faster and score more, which is
/// the whole risk/reward dial for the game — big slow watermelons are freebies.
#[derive(Clone, Copy, PartialEq)]
pub enum FruitKind {
    Watermelon,
    Orange,
    Lime,
    Strawberry,
    Blueberry,
}

impl FruitKind {
    // ─────────────────────────────────────────────────────────────────────────
    // Pick a weighted-random kind.
    // Common fruit dominate; blueberries are the rare high-value target.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn random() -> Self {
        match gen_range(0, 100) {
            0..=27 => FruitKind::Watermelon,
            28..=54 => FruitKind::Orange,
            55..=76 => FruitKind::Lime,
            77..=92 => FruitKind::Strawberry,
            _ => FruitKind::Blueberry,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Collision + drawing radius in pixels.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn radius(&self) -> f32 {
        match self {
            FruitKind::Watermelon => 46.0,
            FruitKind::Orange => 34.0,
            FruitKind::Lime => 28.0,
            FruitKind::Strawberry => 26.0,
            FruitKind::Blueberry => 18.0,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Score awarded for splatting this kind.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn points(&self) -> u32 {
        match self {
            FruitKind::Watermelon => 1,
            FruitKind::Orange => 2,
            FruitKind::Lime => 3,
            FruitKind::Strawberry => 4,
            FruitKind::Blueberry => 6,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Per-kind multiplier on the base rise speed. Small fruit climb quicker.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn speed_scale(&self) -> f32 {
        match self {
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
            FruitKind::Watermelon => Color::new(0.93, 0.27, 0.34, 1.0),
            FruitKind::Orange => Color::new(1.0, 0.76, 0.36, 1.0),
            FruitKind::Lime => Color::new(0.80, 0.93, 0.48, 1.0),
            FruitKind::Strawberry => Color::new(0.99, 0.45, 0.48, 1.0),
            FruitKind::Blueberry => Color::new(0.55, 0.45, 0.78, 1.0),
        }
    }
}

/// A single rising fruit. `base_x` is the column it was launched in; the sway is
/// applied as an offset from that column so drift never accumulates.
pub struct Fruit {
    pub pos: Vec2,
    pub kind: FruitKind,
    pub base_x: f32,
    pub rise_speed: f32,
    pub sway_amp: f32,
    pub sway_freq: f32,
    pub sway_phase: f32,
    pub age: f32,
    pub rot: f32,
    pub spin: f32,
}

impl Fruit {
    // ─────────────────────────────────────────────────────────────────────────
    // Create a fruit just below the bottom edge, ready to rise into view.
    // `base_speed` comes from the spawner and grows as the round progresses.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn new(x: f32, base_speed: f32, kind: FruitKind) -> Self {
        let r = kind.radius();
        Fruit {
            pos: vec2(x, screen_height() + r),
            kind,
            base_x: x,
            rise_speed: base_speed * kind.speed_scale(),
            sway_amp: gen_range(12.0, 46.0),
            sway_freq: gen_range(0.9, 2.1),
            sway_phase: gen_range(0.0, std::f32::consts::TAU),
            age: 0.0,
            rot: gen_range(0.0, 360.0),
            spin: gen_range(-40.0, 40.0),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Advance one frame: rise, sway, and slowly spin the garnish.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn update(&mut self, dt: f32) {
        self.age += dt;
        self.pos.y -= self.rise_speed * dt;

        // Sway is a pure function of age, so the fruit always returns to base_x
        // rather than wandering off the side of the screen over time.
        let sway = (self.age * self.sway_freq + self.sway_phase).sin() * self.sway_amp;
        let r = self.radius();
        self.pos.x = (self.base_x + sway).clamp(r, screen_width() - r);

        self.rot += self.spin * dt;
    }

    pub fn radius(&self) -> f32 {
        self.kind.radius()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Did a click at `p` land on this fruit?
    // ─────────────────────────────────────────────────────────────────────────
    pub fn hit(&self, p: Vec2) -> bool {
        p.distance(self.pos) <= self.radius()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Has this fruit floated fully past the top edge (a miss)?
    // ─────────────────────────────────────────────────────────────────────────
    pub fn escaped(&self) -> bool {
        self.pos.y + self.radius() < 0.0
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
        let count = (kind.radius() * 0.45) as usize + 8;
        let mut particles = Vec::with_capacity(count);

        for i in 0..count {
            // Spread evenly around the circle, then jitter so it doesn't look
            // like a mechanical starburst.
            let base_angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let angle = base_angle + gen_range(-0.28, 0.28);
            let speed = gen_range(90.0, 300.0);
            let life = gen_range(0.42, 0.95);

            particles.push(Particle {
                pos,
                vel: vec2(angle.cos() * speed, angle.sin() * speed),
                radius: gen_range(3.0, 8.0) * (kind.radius() / 34.0).clamp(0.6, 1.5),
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
