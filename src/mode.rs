// =============================================================================
// mode.rs — the difficulty a run is played at
//
// A mode is picked on the route screen, alongside the route. It sets what the
// player starts with, and how hard the waves lean on them once they have it.
//
// So a mode moves three numbers, and they do different jobs at different times.
//
//   Cash  is the opening hand, and only the opening hand. Cumulative income
//         dwarfs it within about ten waves, and it must — the alternative is a
//         mode that scales income, which compounds and would tear the economy
//         curve apart by the late waves. On cash alone the three modes opened
//         2.3x apart and were indistinguishable by wave 15.
//
//   Lives is what carries the rest of the run, and it is the reason the modes
//         still differ once the cash has evened out. Leaks cost by tier — five
//         for a watermelon, six for a Durian — so 25 lives absorbs four leaked
//         bosses and 8 does not survive two leaked watermelons. That question is
//         as sharp on the last wave as on the first, and the report above cannot
//         see it at all.
//
//   Speed is the ramp fruit accelerate along as the waves go by, and it is the
//         only dial that never fades. Speed divides how much every tower gets
//         done on every wave, so a gentler ramp is still being felt on the last
//         one. `balance_report` prints all three modes; the affordability ratio
//         is what the dials add up to:
//
//             wave     1     5    10    13    15    20    25
//             Easy   8.6   4.8   1.7   1.2   1.3   1.5   1.6
//             Medium 4.1   2.6   1.1   0.9   0.9   1.1   1.2
//             Hard   2.9   2.2   1.0   0.8   0.9   1.1   1.1
//
//         Below 1.0 means the wave outruns what the player can afford. Easy
//         never goes there; Medium and Hard both do, at wave 13 and again at
//         the first boss wave. That gap is the whole point, and it was cash and
//         lives alone that could not hold it open — on those two the modes had
//         converged on the same fight by wave 13, which is exactly how it
//         played.
//
// Speed is the one thing here that changes what a wave *does* rather than what
// the player brings to it, and that is a line worth drawing carefully: a mode
// still never changes what a wave **sends**. The fruit, their count, their
// order, the boss schedule and every payout are identical on all three, so the
// economy curve is one curve and the modes are pressure applied to it.
// =============================================================================

/// One difficulty setting.
///
/// Deliberately just a name and numbers. The button prints them rather than a
/// flavour line, because "$400, 30 lives" tells the player what they are
/// choosing and "room to learn" does not.
pub struct Mode {
    pub name: &'static str,
    pub start_cash: u32,
    pub start_lives: u32,
    /// How much faster fruit get with each wave.
    pub speed_ramp: f32,
    /// The ceiling that ramp climbs to.
    pub max_speed: f32,
}

/// Cash and lives the game's balance was actually tuned against. Medium is
/// these numbers exactly, and `wave::balance_report` models this mode — the
/// other two are that curve shifted, not a curve of their own.
pub const TUNED_CASH: u32 = 180;
pub const TUNED_LIVES: u32 = 15;

/// Easiest first, so the row reads left to right the way the labels do.
pub const MODES: [Mode; 3] = [
    Mode {
        name: "Easy",
        // Four towers up before the first wave, and enough lives to leak five
        // Durians and still be standing.
        start_cash: 400,
        start_lives: 30,
        // The dial that carries. Fruit still speed up, but reach x1.35 by the
        // end of a long route where Medium reaches x1.90 — so a tower lands
        // roughly 40% more shots on each fruit at wave 25, every wave, rather
        // than only while the opening cash lasts.
        speed_ramp: 0.015,
        max_speed: 1.35,
    },
    Mode {
        name: "Medium",
        start_cash: TUNED_CASH,
        start_lives: TUNED_LIVES,
        speed_ramp: crate::wave::TUNED_SPEED_RAMP,
        max_speed: crate::wave::TUNED_MAX_SPEED,
    },
    Mode {
        name: "Hard",
        // One tower and change to open with, and a life count where a single
        // leaked Durian takes most of what you have.
        start_cash: 120,
        start_lives: 8,
        // Deliberately the tuned ramp, not a steeper one. Hard is meant to be
        // the balanced fight with no slack, and nobody asked for it to get
        // faster as well.
        speed_ramp: crate::wave::TUNED_SPEED_RAMP,
        max_speed: crate::wave::TUNED_MAX_SPEED,
    },
];

/// Where the selector starts: the mode the game is balanced around.
pub const DEFAULT_MODE: usize = 1;

// ─────────────────────────────────────────────────────────────────────────────
// The mode at `i`, clamped rather than panicking on a stale index.
// ─────────────────────────────────────────────────────────────────────────────
pub fn mode(i: usize) -> &'static Mode {
    &MODES[i.min(MODES.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_get_steadily_harder() {
        // The row is authored easiest first, and every dial must agree with that
        // order — a mode with more cash but fewer lives than its neighbour would
        // not be "easier" in any way a player could act on. Speed is compared
        // non-strictly because Medium and Hard deliberately share the tuned ramp.
        for w in MODES.windows(2) {
            assert!(
                w[0].speed_ramp <= w[1].speed_ramp,
                "{} ramps up faster than {}",
                w[0].name,
                w[1].name
            );
            assert!(
                w[0].max_speed <= w[1].max_speed,
                "{} tops out faster than {}",
                w[0].name,
                w[1].name
            );
            assert!(
                w[0].start_cash > w[1].start_cash,
                "{} does not start richer than {}",
                w[0].name,
                w[1].name
            );
            assert!(
                w[0].start_lives > w[1].start_lives,
                "{} does not start with more lives than {}",
                w[0].name,
                w[1].name
            );
        }
    }

    #[test]
    fn no_mode_is_harsher_than_the_tuned_baseline() {
        // Medium is the balanced fight. Nothing should out-accelerate it — Hard
        // is meant to be that fight with no slack, not a faster one.
        for m in &MODES {
            assert!(m.speed_ramp <= crate::wave::TUNED_SPEED_RAMP, "{}", m.name);
            assert!(m.max_speed <= crate::wave::TUNED_MAX_SPEED, "{}", m.name);
        }
    }

    #[test]
    fn easy_stays_slower_than_the_baseline_for_the_whole_run() {
        // The point of the speed dial: it has to still be doing something on the
        // last wave, which is where cash has long since stopped mattering.
        let easy = mode(0);
        let medium = mode(DEFAULT_MODE);

        for w in 2..=25u32 {
            let (e, m) = (
                crate::wave::speed_multiplier(w, easy),
                crate::wave::speed_multiplier(w, medium),
            );
            assert!(e < m, "wave {w}: Easy runs at {e}, Medium at {m}");
        }
    }

    #[test]
    fn medium_is_the_balanced_baseline() {
        // balance_report models these numbers. If Medium drifts off them the
        // report stops describing any mode the player can actually pick.
        let medium = mode(DEFAULT_MODE);
        assert_eq!(medium.name, "Medium");
        assert_eq!(medium.start_cash, TUNED_CASH);
        assert_eq!(medium.start_lives, TUNED_LIVES);
        assert_eq!(medium.speed_ramp, crate::wave::TUNED_SPEED_RAMP);
        assert_eq!(medium.max_speed, crate::wave::TUNED_MAX_SPEED);
    }

    #[test]
    fn every_mode_can_afford_a_tower_to_open_with() {
        // The cheapest tower is $90. A mode that cannot buy one before the
        // first wave would start the player behind with nothing to do about it.
        for m in &MODES {
            assert!(
                m.start_cash >= 90,
                "{} cannot afford a Seed Shooter to open with",
                m.name
            );
        }
    }

    #[test]
    fn every_mode_survives_a_leaked_durian() {
        // Six lives. A mode where one early leak ends the run outright would
        // read as a bug rather than as difficulty.
        for m in &MODES {
            assert!(m.start_lives > 6, "{} dies to a single Durian", m.name);
        }
    }

    #[test]
    fn mode_names_are_distinct_and_the_index_is_clamped() {
        for (i, a) in MODES.iter().enumerate() {
            for b in MODES.iter().skip(i + 1) {
                assert_ne!(a.name, b.name);
            }
        }
        assert_eq!(mode(999).name, MODES[MODES.len() - 1].name);
    }
}
