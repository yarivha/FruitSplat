// =============================================================================
// wave.rs — what each wave sends and how fast it sends it
//
// A wave is just a shuffled queue of fruit kinds drained on a timer. A new tier
// unlocks every third wave; the freshly unlocked tier arrives in small numbers
// while the tiers below it bulk the wave out, so difficulty climbs by both
// toughness and volume.
//
// That last part is load-bearing and was not always true. The debut count used
// to scale with the wave number, which meant the *later* a tier arrived the
// bigger its first appearance — and since the toughest tier arrives last, wave
// 13 opened with nine watermelons. It is near-flat now, and the tiers below
// carry the volume instead.
// =============================================================================

use macroquad::rand::gen_range;

use crate::fruit::FruitKind;
use crate::mode::Mode;

/// A new fruit tier unlocks every this many waves.
const WAVES_PER_TIER: i32 = 3;
/// Highest tier the unlock ladder ever reaches (Watermelon).
///
/// The Durian sits a tier above this and is deliberately out of reach: a boss
/// is not something the ladder drifts into, it is placed by `boss_count`.
const MAX_TIER: i32 = 4;

/// The earliest wave a Durian can arrive on. Late enough that the player has a
/// real field of towers to meet it with, and no route is shorter than this, so
/// every run meets the boss at least once.
pub const FIRST_BOSS_WAVE: u32 = 15;
/// Boss waves land on this interval once they have started.
const BOSS_EVERY: u32 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// Build the spawn queue for `wave` on a route of `total_waves`, shuffled so
// tiers arrive interleaved.
// ─────────────────────────────────────────────────────────────────────────────
pub fn build_wave(wave: u32, total_waves: u32) -> Vec<FruitKind> {
    let w = wave as i32;
    let top = ((w - 1) / WAVES_PER_TIER).clamp(0, MAX_TIER);

    let mut queue = Vec::new();
    for tier in 0..=top {
        // How many waves ago this tier unlocked: 0 means it's new this wave.
        let age = top - tier;
        let count = match age {
            // The tier that just unlocked. Deliberately near-flat in the wave
            // number, where this used to be 3 + w/2: a debut that scales with
            // the wave means the *last* tier to arrive gets the biggest one, so
            // wave 13 opened with nine watermelons at 31 hits apiece — 279 of
            // that wave's 443, and the sharpest cliff in the game. The tiers
            // below carry the volume instead, which is what the comment at the
            // top of this file always claimed happened.
            // Once the ladder has topped out there is no tougher tier coming,
            // so the top one carries the escalation alone. It keeps the gentle
            // debut — that is what took the wave-13 cliff out — and then grows
            // faster from wave 20 on. Without the second term a long run's
            // non-boss waves flatten into a stroll between bosses: wave 47 sat
            // at four times the firepower the player could afford.
            0 => 3 + w / 6 + (w - 20).max(0) / 3,
            // The tiers below stop growing, which is what keeps a long run from
            // going soft. A wave's pressure is hits per *second*, and once the
            // spawn interval is on its 0.30s floor that is just the average
            // toughness of a fruit — so padding a late wave with more
            // blueberries makes it longer without making it harder, while the
            // income from them keeps compounding. Capped, the chaff thins out
            // as the run goes on and the top of the ladder takes over.
            1 => (5 + w / 3).min(18),
            _ => (3 + w / 5).min(8),
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

    // Bosses go in after the shuffle, because where they land in the order is a
    // decision rather than something to leave to chance.
    add_bosses(&mut queue, boss_count(wave, total_waves));

    queue
}

// ─────────────────────────────────────────────────────────────────────────────
// How many Durians `wave` sends on a route that runs `total_waves` long.
//
// Bosses escalate by count rather than by stats, the same way everything else
// in this game escalates. A tougher Durian would need a second set of numbers
// to tune; a second Durian needs none, and the speed ramp already makes a later
// one harder to break before it arrives.
//
// A route's final wave is always a boss wave whatever its number, so no run can
// end without the fight it has spent twenty waves building toward.
// ─────────────────────────────────────────────────────────────────────────────
pub fn boss_count(wave: u32, total_waves: u32) -> u32 {
    if wave < FIRST_BOSS_WAVE {
        return 0;
    }
    let milestone = wave.is_multiple_of(BOSS_EVERY);
    let finale = wave == total_waves;
    if !(milestone || finale) {
        return 0;
    }

    // One more for every boss wave already survived.
    1 + (wave - FIRST_BOSS_WAVE) / BOSS_EVERY
}

// ─────────────────────────────────────────────────────────────────────────────
// Slot `count` Durians into an already-shuffled queue.
//
// The queue is drained from the back, so a *low* index spawns *late*. Bosses go
// into the first third of the vector, which puts them in the last third of the
// spawn order: they lumber in once most of the wave is already on the track,
// rather than walking in alone ahead of everything or arriving after the field
// has been swept clear.
//
// Positions are worked out against the length the queue will *finish* at, and
// inserted from the last one backwards with each offset by the inserts that
// will later land below it. Aiming at the current length instead lets every
// boss drift a slot or two to the right as the vector grows under it, which is
// enough to push the last one out of the third it was aimed at.
// ─────────────────────────────────────────────────────────────────────────────
fn add_bosses(queue: &mut Vec<FruitKind>, count: u32) {
    if count == 0 {
        return;
    }

    let count = count as usize;
    let final_len = queue.len() + count;

    for k in (0..count).rev() {
        let target = final_len * (k + 1) / (count * 3);
        queue.insert(target.saturating_sub(k).min(queue.len()), FruitKind::Durian);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Seconds between spawns during `wave`. Tightens as waves go on.
// ─────────────────────────────────────────────────────────────────────────────
pub fn spawn_interval(wave: u32) -> f32 {
    let w = wave as f32;
    // Two slopes, not one. The first tightens quickly to 0.30s by wave 28; the
    // second keeps going, gently, to a 0.16s floor around wave 56.
    //
    // The second slope is what makes a long run work at all. A wave's pressure
    // is hits per second, and once the interval stops falling that is fixed by
    // the fruit mix alone — so past wave 28 every extra fruit made a wave
    // longer rather than harder while its income compounded, and the run drifted
    // toward the player. Arriving faster costs the player nothing in cash and
    // everything in the time each tower has to work.
    (0.85 - w * 0.02).max((0.44 - w * 0.005).max(0.16))
}

// ─────────────────────────────────────────────────────────────────────────────
// Cash awarded for clearing `wave`, on top of the per-fruit income.
// ─────────────────────────────────────────────────────────────────────────────
pub fn clear_bonus(wave: u32) -> u32 {
    25 + wave * 4
}

/// How much faster fruit move each wave, and the ceiling on that, for the
/// difficulty the game is balanced around. Modes may soften these; Medium is
/// these numbers exactly.
pub const TUNED_SPEED_RAMP: f32 = 0.035;
pub const TUNED_MAX_SPEED: f32 = 2.80;

// ─────────────────────────────────────────────────────────────────────────────
// Speed multiplier applied to every fruit spawned during `wave`.
//
// Without this, waves past 13 escalate only by sending *more* fruit — the fruit
// themselves never get harder. Since a tower's output is bounded by how long a
// fruit stays in its range, speeding fruit up cuts the shots each tower lands,
// which is the escalation that count alone can't provide.
//
// The ramp comes from the mode, because it is the only difficulty dial that
// does not fade. Cash decides the opening and is swamped by income within ten
// waves; speed divides how much a tower gets done on every single wave, so a
// gentler ramp is still being felt on the last one.
// ─────────────────────────────────────────────────────────────────────────────
pub fn speed_multiplier(wave: u32, m: &Mode) -> f32 {
    (1.0 + (wave.saturating_sub(1)) as f32 * m.speed_ramp).min(m.max_speed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A route long enough that no wave under test is accidentally its finale.
    const LONG_ROUTE: u32 = 1000;

    fn top_tier(wave: u32, total_waves: u32) -> u8 {
        build_wave(wave, total_waves)
            .iter()
            .map(|f| f.tier())
            .max()
            .unwrap()
    }

    /// Total hits needed to fully clear one fruit and everything it splits
    /// into: 1, 3, 7, 15, 31 by tier, and the Durian's armour on top of four
    /// whole watermelons. Derived from the ladder rather than a closed form,
    /// because the Durian is neither binary nor unarmoured.
    fn subtree_hits(kind: FruitKind) -> u32 {
        match kind.child() {
            None => kind.armour(),
            Some(c) => kind.armour() + kind.split_count() as u32 * subtree_hits(c),
        }
    }

    /// Fruit at the bottom of the ladder in one fruit's subtree — the only ones
    /// that pay out: 1, 2, 4, 8, 16 by tier, and 64 for a Durian.
    fn subtree_payout(kind: FruitKind) -> u32 {
        match kind.child() {
            None => 1,
            Some(c) => kind.split_count() as u32 * subtree_payout(c),
        }
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
        // Modelled on the longest route, which is the one that sees every boss
        // wave the schedule produces.
        const TOTAL: u32 = 70;

        // One table per difficulty. Only the opening cash differs — a mode
        // changes what the player starts with, never what a wave sends — so the
        // three tables show the same curve shifted, and the shift is exactly
        // what fades out as cumulative income takes over.
        for m in &crate::mode::MODES {
            println!(
                "\n=== {} — ${} start, {} lives, ramp {:.3} capped {:.2} ===",
                m.name, m.start_cash, m.start_lives, m.speed_ramp, m.max_speed
            );
            balance_table(TOTAL, m);
        }
        println!();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // One difficulty's curve, printed as a table.
    // ─────────────────────────────────────────────────────────────────────────
    fn balance_table(total: u32, m: &Mode) {
        const SEED_SHOOTER_COST: f32 = 90.0;
        const SEED_SHOOTER_DPS: f32 = 1.0 / 0.45;

        let mut cash = m.start_cash as f32;
        println!(
            "{:>4} {:>6} {:>5} {:>6} {:>7} {:>8} {:>6} {:>7} {:>7} {:>7}",
            "wave",
            "fruit",
            "boss",
            "hits",
            "wave $",
            "cum $",
            "speed",
            "need/s",
            "afford",
            "ratio"
        );

        for wave in 1..=total {
            let queue = build_wave(wave, total);
            let hits: u32 = queue.iter().copied().map(subtree_hits).sum();
            let payout: u32 = queue.iter().copied().map(subtree_payout).sum();
            let seconds = queue.len() as f32 * spawn_interval(wave);
            let speed = speed_multiplier(wave, m);
            let bosses = boss_count(wave, total);

            let income = payout as f32 + clear_bonus(wave) as f32;
            cash += income;

            let need = hits as f32 / seconds;
            // Faster fruit spend proportionally less time inside a tower's
            // range, so the same cash buys proportionally less effective DPS.
            let afford = cash / SEED_SHOOTER_COST * SEED_SHOOTER_DPS / speed;

            println!(
                "{wave:>4} {:>6} {bosses:>5} {hits:>6} {:>7.0} {cash:>8.0} {speed:>6.2} {need:>7.2} {afford:>7.1} {:>7.1}",
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
            assert_eq!(
                top_tier(w, LONG_ROUTE),
                0,
                "wave {w} should be blueberries only"
            );
        }
    }

    #[test]
    fn a_new_tier_unlocks_every_third_wave() {
        assert_eq!(top_tier(4, LONG_ROUTE), 1);
        assert_eq!(top_tier(7, LONG_ROUTE), 2);
        assert_eq!(top_tier(10, LONG_ROUTE), 3);
        assert_eq!(top_tier(13, LONG_ROUTE), 4);
    }

    #[test]
    fn the_tier_ladder_tops_out_at_the_watermelon() {
        // The Durian is a tier above the watermelon but the ladder must never
        // reach it — bosses are placed by boss_count, never drifted into. Both
        // waves here are deliberately not boss waves.
        for w in [59u32, 501] {
            assert_eq!(
                boss_count(w, LONG_ROUTE),
                0,
                "picked a boss wave by accident"
            );
            assert_eq!(top_tier(w, LONG_ROUTE), 4);
        }
    }

    #[test]
    fn the_only_fruit_above_a_watermelon_are_the_bosses_the_wave_sends() {
        for w in 1..=40u32 {
            let queue = build_wave(w, 40);
            let durians = queue.iter().filter(|f| f.is_boss()).count() as u32;

            assert_eq!(durians, boss_count(w, 40), "wave {w} sent the wrong count");
            assert!(
                queue.iter().all(|f| f.tier() <= 4 || f.is_boss()),
                "wave {w} sent something above a watermelon that is not a boss"
            );
        }
    }

    #[test]
    fn every_wave_sends_something() {
        for w in 1..=40 {
            assert!(!build_wave(w, 40).is_empty(), "wave {w} was empty");
        }
    }

    #[test]
    fn no_boss_arrives_before_the_first_boss_wave() {
        for w in 1..FIRST_BOSS_WAVE {
            assert_eq!(boss_count(w, LONG_ROUTE), 0, "wave {w} sent a boss early");
            // Not even as a finale: a route that short has not earned one.
            assert_eq!(boss_count(w, w), 0, "a short route's finale sent a boss");
        }
    }

    #[test]
    fn the_final_wave_of_a_route_is_always_a_boss_wave() {
        // Whatever its number, so a run can't end without the fight.
        for total in FIRST_BOSS_WAVE..=40 {
            assert!(
                boss_count(total, total) >= 1,
                "a {total}-wave route ends without a boss"
            );
        }
    }

    #[test]
    fn bosses_escalate_by_count() {
        // One at the first boss wave, one more at every milestone after it.
        assert_eq!(boss_count(15, 25), 1);
        assert_eq!(boss_count(20, 25), 2);
        assert_eq!(boss_count(25, 25), 3);

        // And the count only ever climbs.
        let mut prev = 0;
        for w in FIRST_BOSS_WAVE..=60 {
            let n = boss_count(w, 60);
            if n > 0 {
                assert!(n >= prev, "boss count went backwards at wave {w}");
                prev = n;
            }
        }
    }

    #[test]
    fn the_waves_between_milestones_send_no_boss() {
        for w in [16u32, 17, 18, 19, 21, 22, 23, 24] {
            assert_eq!(boss_count(w, LONG_ROUTE), 0, "wave {w} should be quiet");
        }
    }

    #[test]
    fn a_boss_arrives_once_the_wave_is_already_under_way() {
        // The queue drains from the back, so a boss must sit in the front third
        // of the vector to spawn in the last third of the order. Walking in
        // alone at the head of the wave would waste what makes it frightening.
        let queue = build_wave(25, 25);
        let len = queue.len();

        for (i, f) in queue.iter().enumerate() {
            if f.is_boss() {
                assert!(
                    i * 3 <= len,
                    "boss at index {i} of {len} spawns too early in the order"
                );
            }
        }
    }

    #[test]
    fn a_boss_wave_is_a_real_step_up_in_work() {
        // The point of the boss is that it is felt. Wave 15 must cost markedly
        // more to clear than wave 14, which sends more fruit but no Durian.
        let quiet: u32 = build_wave(14, 25).iter().copied().map(subtree_hits).sum();
        let boss: u32 = build_wave(15, 25).iter().copied().map(subtree_hits).sum();

        assert!(
            boss as f32 > quiet as f32 * 1.25,
            "a boss wave should cost at least a quarter more: {quiet} -> {boss}"
        );
    }

    #[test]
    fn a_durian_is_worth_four_watermelons_at_the_till() {
        // Income is paid per fruit destroyed outright, so the payout follows the
        // payload rather than the armour.
        assert_eq!(
            subtree_payout(FruitKind::Durian),
            4 * subtree_payout(FruitKind::Watermelon)
        );
    }

    #[test]
    fn the_speed_ramp_starts_at_one_and_only_climbs() {
        for m in &crate::mode::MODES {
            assert_eq!(
                speed_multiplier(1, m),
                1.0,
                "{} must open at base speed",
                m.name
            );

            let mut prev = 0.0;
            for w in 1..=60 {
                let s = speed_multiplier(w, m);
                assert!(s >= prev, "{} went backwards at wave {w}", m.name);
                prev = s;
            }
        }
    }

    #[test]
    fn the_speed_ramp_is_capped() {
        // Uncapped, late waves would outrun the projectiles entirely.
        for m in &crate::mode::MODES {
            for w in [30u32, 100, 10_000, u32::MAX] {
                assert!(
                    speed_multiplier(w, m) <= m.max_speed,
                    "{} blew its cap",
                    m.name
                );
            }
        }
    }

    #[test]
    fn wave_zero_does_not_underflow_the_ramp() {
        // saturating_sub guards this; a plain subtraction would wrap.
        for m in &crate::mode::MODES {
            assert_eq!(speed_multiplier(0, m), 1.0, "{}", m.name);
        }
    }

    #[test]
    fn spawn_interval_never_drops_below_its_floor() {
        for w in 1..=500 {
            assert!(
                spawn_interval(w) >= 0.16,
                "wave {w} spawns faster than the floor"
            );
        }
    }
}
