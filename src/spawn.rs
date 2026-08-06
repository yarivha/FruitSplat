// =============================================================================
// spawn.rs — spawn pacing and the round difficulty curve
//
// Owns the one dial that makes the round get harder: as elapsed time grows the
// gap between fruit shrinks and the base rise speed climbs, so late-round fruit
// arrive faster and leave the screen sooner.
// =============================================================================

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::fruit::{Fruit, FruitKind};

/// Seconds between spawns at the start of a round.
const INTERVAL_START: f32 = 0.95;
/// Seconds between spawns once the curve has fully ramped.
const INTERVAL_END: f32 = 0.30;
/// Base rise speed (px/s) at the start and end of the ramp.
const SPEED_START: f32 = 95.0;
const SPEED_END: f32 = 190.0;
/// How long, in seconds, the difficulty takes to reach its ceiling.
const RAMP_SECONDS: f32 = 45.0;
/// Keep fruit clear of the window edges when picking a spawn column.
const EDGE_MARGIN: f32 = 70.0;

/// Drives fruit creation. `timer` counts down to the next spawn.
pub struct Spawner {
    timer: f32,
}

impl Spawner {
    pub fn new() -> Self {
        // Small initial delay so the first fruit isn't already on screen when
        // the player's hand is still moving to the mouse.
        Spawner { timer: 0.6 }
    }

    pub fn reset(&mut self) {
        self.timer = 0.6;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tick the spawn clock. Returns Some(Fruit) on the frame a fruit is due.
    // `elapsed` is seconds since the round began and drives the difficulty ramp.
    // ─────────────────────────────────────────────────────────────────────────
    pub fn update(&mut self, dt: f32, elapsed: f32) -> Option<Fruit> {
        self.timer -= dt;
        if self.timer > 0.0 {
            return None;
        }

        // 0.0 at round start, 1.0 once the ramp is done.
        let t = (elapsed / RAMP_SECONDS).clamp(0.0, 1.0);
        let interval = INTERVAL_START + (INTERVAL_END - INTERVAL_START) * t;
        let speed = SPEED_START + (SPEED_END - SPEED_START) * t;

        // Jitter the interval so the rhythm never feels metronomic.
        self.timer = interval * gen_range(0.8, 1.2);

        let x = gen_range(EDGE_MARGIN, (screen_width() - EDGE_MARGIN).max(EDGE_MARGIN + 1.0));
        Some(Fruit::new(x, speed, FruitKind::random()))
    }
}
