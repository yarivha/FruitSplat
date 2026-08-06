// =============================================================================
// wave.rs — what each wave sends and how fast it sends it
//
// A wave is just a shuffled queue of fruit kinds drained on a timer. A new tier
// unlocks every third wave; the freshly unlocked tier arrives in small numbers
// while the tiers below it bulk the wave out, so difficulty climbs by both
// toughness and volume.
// =============================================================================

use macroquad::rand::gen_range;

use crate::fruit::FruitKind;

/// A new fruit tier unlocks every this many waves.
const WAVES_PER_TIER: i32 = 3;
/// Highest tier index that exists (Watermelon).
const MAX_TIER: i32 = 4;

// ─────────────────────────────────────────────────────────────────────────────
// Build the spawn queue for `wave`, shuffled so tiers arrive interleaved.
// ─────────────────────────────────────────────────────────────────────────────
pub fn build_wave(wave: u32) -> Vec<FruitKind> {
    let w = wave as i32;
    let top = ((w - 1) / WAVES_PER_TIER).clamp(0, MAX_TIER);

    let mut queue = Vec::new();
    for tier in 0..=top {
        // How many waves ago this tier unlocked: 0 means it's new this wave.
        let age = top - tier;
        let count = match age {
            0 => 3 + w / 2,
            1 => 4 + w / 3,
            _ => 2 + w / 5,
        };
        for _ in 0..count.max(1) {
            queue.push(FruitKind::from_tier(tier as u8));
        }
    }

    // Fisher-Yates, so the wave isn't sorted tier-by-tier.
    for i in (1..queue.len()).rev() {
        let j = gen_range(0, i + 1);
        queue.swap(i, j);
    }

    queue
}

// ─────────────────────────────────────────────────────────────────────────────
// Seconds between spawns during `wave`. Tightens as waves go on.
// ─────────────────────────────────────────────────────────────────────────────
pub fn spawn_interval(wave: u32) -> f32 {
    (0.85 - wave as f32 * 0.02).max(0.30)
}

// ─────────────────────────────────────────────────────────────────────────────
// Cash awarded for clearing `wave`, on top of the per-fruit income.
// ─────────────────────────────────────────────────────────────────────────────
pub fn clear_bonus(wave: u32) -> u32 {
    15 + wave * 2
}

/// How much faster fruit move each wave, and the ceiling on that.
const SPEED_RAMP_PER_WAVE: f32 = 0.035;
const MAX_SPEED_MULTIPLIER: f32 = 1.90;

// ─────────────────────────────────────────────────────────────────────────────
// Speed multiplier applied to every fruit spawned during `wave`.
//
// Without this, waves past 13 escalate only by sending *more* fruit — the fruit
// themselves never get harder. Since a tower's output is bounded by how long a
// fruit stays in its range, speeding fruit up cuts the shots each tower lands,
// which is the escalation that count alone can't provide.
// ─────────────────────────────────────────────────────────────────────────────
pub fn speed_multiplier(wave: u32) -> f32 {
    (1.0 + (wave.saturating_sub(1)) as f32 * SPEED_RAMP_PER_WAVE).min(MAX_SPEED_MULTIPLIER)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top_tier(wave: u32) -> u8 {
        build_wave(wave).iter().map(|f| f.tier()).max().unwrap()
    }

    /// Total pops needed to fully clear one fruit and everything it splits
    /// into: 1, 3, 7, 15, 31 by tier.
    fn subtree_pops(kind: FruitKind) -> u32 {
        (1u32 << (kind.tier() + 1)) - 1
    }

    /// Fruit at the bottom of the ladder in one fruit's subtree — the only ones
    /// that pay out: 1, 2, 4, 8, 16 by tier.
    fn subtree_payout(kind: FruitKind) -> u32 {
        1u32 << kind.tier()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Balance report, not an assertion. Prints the shape of the difficulty and
    // economy curves so tuning is done against numbers rather than a hunch.
    //
    //     cargo test balance_report -- --ignored --nocapture
    //
    // "Need" is the pops per second required to clear a wave as fast as it
    // arrives; "afford" is how many basic Seed Shooters the cumulative income
    // could have bought. If afford climbs faster than need, the game is
    // getting easier as it goes.
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    #[ignore]
    fn balance_report() {
        const SEED_SHOOTER_COST: f32 = 90.0;
        const SEED_SHOOTER_DPS: f32 = 1.0 / 0.45;

        let mut cash = 180.0_f32; // starting cash
        println!(
            "\n{:>4} {:>6} {:>6} {:>7} {:>8} {:>6} {:>7} {:>7} {:>7}",
            "wave", "fruit", "pops", "wave $", "cum $", "speed", "need/s", "afford", "ratio"
        );

        for wave in 1..=25u32 {
            let queue = build_wave(wave);
            let pops: u32 = queue.iter().copied().map(subtree_pops).sum();
            let payout: u32 = queue.iter().copied().map(subtree_payout).sum();
            let seconds = queue.len() as f32 * spawn_interval(wave);
            let speed = speed_multiplier(wave);

            let income = payout as f32 + clear_bonus(wave) as f32;
            cash += income;

            let need = pops as f32 / seconds;
            // Faster fruit spend proportionally less time inside a tower's
            // range, so the same cash buys proportionally less effective DPS.
            let afford = cash / SEED_SHOOTER_COST * SEED_SHOOTER_DPS / speed;

            println!(
                "{wave:>4} {:>6} {pops:>6} {:>7.0} {cash:>8.0} {speed:>6.2} {need:>7.2} {afford:>7.1} {:>7.1}",
                queue.len(),
                income,
                afford / need,
            );
        }
        println!();
    }

    #[test]
    fn the_opening_waves_are_bottom_tier_only() {
        for w in 1..=3 {
            assert_eq!(top_tier(w), 0, "wave {w} should be blueberries only");
        }
    }

    #[test]
    fn a_new_tier_unlocks_every_third_wave() {
        assert_eq!(top_tier(4), 1);
        assert_eq!(top_tier(7), 2);
        assert_eq!(top_tier(10), 3);
        assert_eq!(top_tier(13), 4);
    }

    #[test]
    fn nothing_ever_outranks_a_watermelon() {
        assert_eq!(top_tier(60), 4);
        assert_eq!(top_tier(500), 4);
    }

    #[test]
    fn every_wave_sends_something() {
        for w in 1..=40 {
            assert!(!build_wave(w).is_empty(), "wave {w} was empty");
        }
    }

    #[test]
    fn the_speed_ramp_starts_at_one_and_only_climbs() {
        assert_eq!(speed_multiplier(1), 1.0, "wave 1 must run at base speed");

        let mut prev = 0.0;
        for w in 1..=60 {
            let s = speed_multiplier(w);
            assert!(s >= prev, "speed went backwards at wave {w}");
            prev = s;
        }
    }

    #[test]
    fn the_speed_ramp_is_capped() {
        // Uncapped, late waves would outrun the projectiles entirely.
        for w in [30u32, 100, 10_000, u32::MAX] {
            assert!(speed_multiplier(w) <= MAX_SPEED_MULTIPLIER);
        }
    }

    #[test]
    fn wave_zero_does_not_underflow_the_ramp() {
        // saturating_sub guards this; a plain subtraction would wrap.
        assert_eq!(speed_multiplier(0), 1.0);
    }

    #[test]
    fn spawn_interval_never_drops_below_its_floor() {
        for w in 1..=500 {
            assert!(spawn_interval(w) >= 0.30);
        }
    }
}
