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
// Cash awarded for clearing `wave`, on top of the per-pop income.
// ─────────────────────────────────────────────────────────────────────────────
pub fn clear_bonus(wave: u32) -> u32 {
    25 + wave * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top_tier(wave: u32) -> u8 {
        build_wave(wave).iter().map(|f| f.tier()).max().unwrap()
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
    fn spawn_interval_never_drops_below_its_floor() {
        for w in 1..=500 {
            assert!(spawn_interval(w) >= 0.30);
        }
    }
}
