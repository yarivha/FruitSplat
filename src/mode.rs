// =============================================================================
// mode.rs — the difficulty a run is played at
//
// A mode is picked on the route screen, alongside the route, and sets what the
// player starts with. It deliberately does not touch what a wave *sends*: the
// wave table, the speed ramp and the per-fruit payout are tuned together against
// `wave::balance_report`, and a mode that quietly rewrote them would make that
// whole curve a fiction on two runs out of three.
//
// So a mode moves two numbers, and they do different jobs at different times.
//
//   Cash  is the opening hand, and only the opening hand. `balance_report`
//         prints all three modes, and the affordability ratios say plainly how
//         fast it stops mattering:
//
//             wave     1     5    10    15    25
//             Easy   6.6   3.6   1.3   1.0   1.2
//             Medium 4.1   2.6   1.1   0.9   1.2
//             Hard   2.9   2.2   1.0   0.9   1.1
//
//         A 2.3x spread at wave 1 is gone by wave 15. Cumulative income dwarfs
//         anything you started with, and it must — the alternative is a mode
//         that scales income, which compounds and would tear the economy curve
//         apart by the late waves.
//
//   Lives is what carries the rest of the run, and it is the reason the modes
//         still differ once the cash has evened out. Leaks cost by tier — five
//         for a watermelon, six for a Durian — so 25 lives absorbs four leaked
//         bosses and 8 does not survive two leaked watermelons. That question is
//         as sharp on the last wave as on the first, and the report above cannot
//         see it at all.
//
// Cash alone would have left Easy and Hard genuinely indistinguishable from the
// midpoint on, which is why lives moves with it.
// =============================================================================

/// One difficulty setting.
///
/// Deliberately just a name and two numbers. The button prints the numbers
/// themselves rather than a flavour line, because "$300, 25 lives" tells the
/// player what they are choosing and "room to learn" does not.
pub struct Mode {
    pub name: &'static str,
    pub start_cash: u32,
    pub start_lives: u32,
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
        // Three towers up before the first wave instead of two, and enough
        // lives to leak four Durians and still be standing.
        start_cash: 300,
        start_lives: 25,
    },
    Mode {
        name: "Medium",
        start_cash: TUNED_CASH,
        start_lives: TUNED_LIVES,
    },
    Mode {
        name: "Hard",
        // One tower and change to open with, and a life count where a single
        // leaked Durian takes most of what you have.
        start_cash: 120,
        start_lives: 8,
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
        // The row is authored easiest first, and both dials must agree with
        // that order — a mode with more cash but fewer lives than its neighbour
        // would not be "easier" in any way a player could act on.
        for w in MODES.windows(2) {
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
    fn medium_is_the_balanced_baseline() {
        // balance_report models these numbers. If Medium drifts off them the
        // report stops describing any mode the player can actually pick.
        let medium = mode(DEFAULT_MODE);
        assert_eq!(medium.name, "Medium");
        assert_eq!(medium.start_cash, TUNED_CASH);
        assert_eq!(medium.start_lives, TUNED_LIVES);
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
