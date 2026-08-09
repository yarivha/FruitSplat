# Changelog

All notable changes to Fruit Splat are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries say what changed, briefly. The reasoning behind a change lives in its
commit message and in the comments around the code it touched.

## [Unreleased]

### Changed

- **Routes run 50 to 70 waves**, up from 15 to 25. Market Run 50, Twin Gates 54,
  Orchard Snake and Zigzag Grove 58, The Long Orchard 64, Meander 70.
  - Making the runs longer meant making them keep escalating. Past wave ~30 the
    game got steadily *easier*: the speed ramp topped out at wave 26 and the
    spawn interval at 28, after which a wave's demand per second was fixed by
    the fruit mix while cumulative income carried on compounding. Medium's
    affordability climbed from 1.3 at wave 30 to 4.9 by wave 70 — the last
    twenty waves were a victory lap.
  - Speed ceilings raised to 1.85 / 2.80 / 3.00 from 1.35 / 1.90 / 2.20, so the
    ramp is still working at the end of a long route rather than finished at
    wave 26.
  - Spawn intervals now fall on two slopes: as before to 0.30s by wave 28, then
    gently to a 0.16s floor by wave 56. Arriving faster raises the pressure
    without handing the player any more money, which no other dial does.
  - The lower tiers stop growing, so late waves concentrate on the top of the
    ladder instead of padding themselves with blueberries that add length and
    income but no difficulty. The top tier grows faster from wave 20 on to
    carry the escalation once nothing tougher is left to unlock.
  - Affordability now runs roughly flat from wave 13 to the finish: Easy 1.7–2.2,
    Medium 1.1–1.5, Hard 1.0–1.4.

- **The game is easier.** Four dials moved together, after the balance report
  showed Medium bottoming out at exactly 1.0 affordability on waves 13 and 15 —
  break-even in a model that spends every dollar perfectly, lands every shot and
  never leaves a tower idle, so a real player at 1.0 is behind. Worst-wave
  affordability now runs Easy 1.7, Medium 1.2, Hard 1.0, up from 1.3 / 1.0 / 0.8.
  - A freshly unlocked fruit tier no longer arrives in force. Its debut count
    scaled with the wave number, so the later a tier arrived the bigger its
    first appearance — and the toughest tier arrives last, which opened wave 13
    with nine watermelons and 279 of that wave's 443 hits. Wave 13 now asks for
    345 hits and sits at 1.4 instead of 1.0.
  - The wave-clear bonus goes from 15+2w to 25+4w, which compounds into every
    later wave. Income and wave composition are shared by all three modes, so
    this lifts the whole curve and leaves the modes as far apart as they were.
  - More lives: Easy 30 → 36, Medium 15 → 20, Hard 8 → 10. Changes nothing in
    the affordability maths, only what one mistake costs.
  - The Spike Layer lays at 3.60 / 2.80 / 2.20s, down from 4.50 / 3.40 / 2.60.
    Sweeping its piles at the end of a wave took away the head start it used to
    carry into the next one, and the slower rate compounded that; this gives
    back some of the opening without touching either rule.

## [0.3.0] - 2026-08-09

A seventh tower, a title made of fruit, and a game that finally fits a
phone screen. Hard also stops being Medium with fewer lives.

### Added

- **Bomb Lobber** ($320), a seventh tower. It lobs a slow shell that takes
  everything standing in the blast off the track at once — 110px at Lv1 against
  the Blender's 58, rising to 165. The dearest tower in the shop and the slowest
  to fire, so between shells whatever it missed keeps walking.
  - The only tower that does not use "first" targeting. It aims at whichever
    fruit has the most neighbours inside its blast, because a shell spent on the
    straggler that has outrun the pack clears exactly one fruit. Ties fall back
    on threat, and a field with nothing bunched up falls back on the leader.
  - Its blast is wider than half its own range, so it is placed by guessing
    where a crowd will be rather than by what it can see.
  - Two new sounds: a hollow lob on firing and a boom on landing, the loudest
    effect in the game. An expanding ring marks the blast, which is the only way
    to learn the radius the tower actually covers.
- The menu title is spelled out in fruit instead of set in the text font. Each
  letter is a 5x7 grid with a berry on every lit cell, drawn with the shading the
  fruit on the track use and coloured a tier per letter.
- The web build scales to the screen instead of overflowing it. The page sizes
  the canvas to the device — never with a CSS `transform`, which desynchronises
  rendering from hit-testing — and `render.rs` draws through a camera fitted to a
  fixed 1420x740 view, converting pointer positions back through the same fit, so
  the two scale together. A desktop window is that size exactly, so nothing about
  it changes.
- A **turn your phone sideways** prompt on screens under 820px in portrait, where
  the game would otherwise be a 200px strip. It overlays the canvas rather than
  hiding it, so rotating back restores a game that still fits.

### Changed

- Hard gets a speed ramp of its own — 0.050 to a x2.20 ceiling, against the
  tuned 0.035 and x1.90 it used to share with Medium. Speed is the only
  difficulty dial that does not fade, so sharing it left Medium and Hard the
  same game once income had swamped the opening hands around wave 10: identical
  fruit at identical speeds, separated by a life count. Medium now sits between
  its neighbours on every dial, and the mode buttons read +35% / +90% / +120%
  where two of them used to read the same. Medium itself is unchanged — it is
  still the baseline `balance_report` models.

### Fixed

- Towers on the right quarter of the map could not be upgraded or sold. Their
  floating panel was positioned against the window rather than the playfield, so
  it overhung the shop column — and clicks there go to the shop, which is tested
  first, leaving the overhanging part of the panel drawn but dead. For a tower at
  x 1126 that was all but 28px of its upgrade button. The panel now stays inside
  the playfield, and a test sweeps every tower position to keep it there.

## [0.2.4] - 2026-08-09

### Fixed

- The web build showed no shop. Its canvas is a fixed size written into
  `web/index.html` by hand, and it was never widened when the shop moved from a
  bottom bar into a right-hand column: the window went from 1000 to 1420 wide,
  the canvas stayed at 1200, and the 220px column sat off the edge of it. Native
  builds were unaffected, which is why nothing caught it. A test now fails if the
  canvas and `WINDOW_W`/`WINDOW_H` drift apart again.

## [0.2.3] - 2026-08-09

### Fixed

- The Spike Layer laid its spikes once and then went quiet: capped at a fixed
  allowance of piles (3 at Lv1) that it emptied in seconds, laying on the build
  screen as well as during a wave so the allowance was spent before the wave was
  sent, and always choosing the point furthest along its stretch. It now lays
  purely on its level's timer, for the full length of a wave and only during
  one, each pile at a random free spot across the track it covers. The pile
  allowance is gone; `SPIKE_SPACING` and how much track a tower reaches now
  bound it. Its info panel reports the lay interval in place of the allowance.

### Changed

- Spike piles are swept when a wave is cleared instead of being carried into the
  next one. With the tower only laying while a wave runs, every wave now starts
  on bare track.
- The Spike Layer lays at 4.50s / 3.40s / 2.60s per pile, up from
  2.20s / 1.70s / 1.30s. Its piles stay on the track until fruit wear them away,
  so what the tower is worth is the stock of spikes standing on its stretch, not
  the rate it lays them — at the old pace it saturated everything it covered
  inside the first third of a wave and then had nowhere left to lay. The stock
  now builds across the whole wave.

## [0.2.2] - 2026-08-09

### Changed

- The Triple Seeder's 0.70s cooldown cancelled its own triple out: at Lv3 its
  sustained rate came to exactly a Seed Shooter's, for two and a half times the
  money, and $260 of Seed Shooters out-shot it at every level. It fires on the
  Seed Shooter's cadence now — three seeds where that tower puts one — and gains
  a seed per upgrade instead of losing rate.

## [0.2.1] - 2026-08-09

### Changed

- Release notes are lifted from this file's matching version section instead of
  a generated commit list. A tag with no section here fails the release.
- Starting cash raised across all three modes — Easy $400 to $550, Medium $180
  to $300, Hard $120 to $200 — so the opening is not two Seed Shooters and a
  hope, and the Triple Seeder is buyable before the late game. Affects the start
  only, since income swamps the opening hand within ten waves, though it does
  take Medium's wave-13 dip below 1.0 out with it.

### Fixed

- Multi-shot towers fired one shot per fruit in range, so a Triple Seeder facing
  a lone fruit fired once — slower and dearer than the Seed Shooter for the same
  output, and worst against a boss. A volley now always fires every shot, cycling
  back to the leading fruit when there are fewer targets than shots. This also
  makes a Lv3 Seed Shooter fire both its seeds at a single target.

## [0.2.0] - 2026-08-09

The interface release: the shop moved from a bar along the bottom to a column
down the right, which made room for a sixth tower, and the route picker grew to
two rows for a sixth route and a random pick.

### Added

#### Core game

- Tower defence loop: fruit walk a fixed track, the player buys and places towers
  to pop them, and fruit reaching the exit drain lives.
- Split ladder — Watermelon → Orange → Lime → Strawberry → Blueberry. Popping a
  fruit bursts it into two of the next tier down, and smaller tiers move faster.
- Waves unlock a new tier every third wave, tighten spawn intervals from 0.85s to
  a 0.30s floor, and pay a bonus on clear. A run is finite and can be won.
- Free-form tower placement on open ground, with a live range preview that turns
  red when the spot is illegal. Placing a tower disarms the shop selection.
- Hit points, so a fruit can survive being hit. Several hits landing in one frame
  all count, and the killing blow takes the kill credit.

#### Towers

- Six towers: Seed Shooter ($90, single target), Knife Thrower ($130, pierces 3),
  Freezer ($140, no damage, chills to 45% speed), Spike Layer ($150, spikes on
  the track), Blender ($170, 58px splash), Triple Seeder ($260, three fruit per
  volley).
- Each upgrades Lv1 → Lv3 along one track; selling refunds 60% of everything
  invested, upgrades included.
- Splash hits a blob of fruit around a point; pierce hits a line of them along
  the shot's path, which makes the Knife Thrower strongest on switchbacks.
- Spike piles are matched to fruit by lane and distance along it, never by pixel
  distance, and one fruit costs exactly one spike however many piles it is on.
- "First" targeting: towers engage the fruit furthest along its lane, and lead
  their shots to where it will be when the shot lands.
- A per-tower stats panel showing range, rate and its own running tally, with
  Upgrade and Sell buttons.

#### The Durian

- A boss fruit with **60 points of armour** that bursts into **four watermelons**
  — 184 hits to clear one. Slowest thing on the track, with an armour bar that
  appears once it has been hit.
- Boss waves arrive on wave 15, every fifth wave after, and every route's final
  wave. They escalate by count, not by stats: one at wave 15, three at wave 25.
- Knife Throwers are the answer to it; Spike Layers are not, since a pile lands
  one hit per frame and strips itself in a fraction of a second.

#### Routes

- Six routes: Market Run (Hard, 15 waves), Twin Gates (Tricky, 18), Orchard Snake
  and Zigzag Grove (Medium, 20), The Long Orchard and Meander (Gentle, 25).
- **Twin Gates** has two entrances feeding one exit, with each wave dealt to the
  gates in turn. Lanes stay independent, so targeting ranks fruit by fraction of
  their lane walked rather than raw distance.
- Route cards in two rows of four, plus a **Surprise Me** card that starts a run
  on one of the six at random.
- Each route has its own palette and a themed, deterministic scatter of scenery.
  Scenery never blocks placement and is drawn under the track.

#### Difficulty modes

- Easy, Medium and Hard, picked on the route screen and kept between runs.
- A mode sets starting cash ($400/$180/$120), lives (30/15/8) and the speed ramp
  fruit accelerate along (+35%/+90%/+90% by the end). It never changes what a
  wave sends.
- Speed is the dial that carries: cash is swamped by income within ten waves, so
  without it the three modes converged on the same fight by wave 13.

#### Interface

- Shop is a column down the right of a 1420x740 window, with the audio toggles,
  pause, auto and quit in a block at its foot.
- **Pause** holds the wave while leaving building live.
- **AUTO** sends each wave three seconds after the field clears.
- **QUIT RUN** abandons a run, asking once before it acts.
- Separate mute toggles for sound effects and music, available on every screen.
- The build's version in the corner of the title screen, read from
  `CARGO_PKG_VERSION` so it cannot drift from the release it shipped in.

#### Under the hood

- Fully procedural visuals and audio — no image assets, and 20 sound effects plus
  2 music loops generated by pure-stdlib Python scripts and embedded with
  `include_bytes!`, so the binary is standalone.
- 127 unit tests covering path maths, the split ladder, boss armour, wave and
  boss composition, tower upgrades, spike accounting, lane isolation, route
  validation, difficulty ordering and UI layout.
- `balance_report`, an ignored test printing the difficulty and economy curves
  per mode, so tuning is done against numbers rather than guesswork.
- GitHub Actions builds Linux, macOS (universal), Windows and web for a `v*` tag
  and publishes them as a release.
- Screenshot mode (`FRUITSPLAT_SCREENSHOT`) stages a scene, writes a PNG, exits.

### Changed

- **Difficulty pass.** The player could afford roughly six times the firepower a
  wave needed, and the gap widened after wave 13. Income now pays only for fruit
  destroyed outright, fruit gain 3.5% speed per wave, and starting lives and cash
  drop to 15 and $180.
- Towers lead their shots, without which the speed ramp would have made them miss
  through no fault of the player.
- Reworked all the artwork: stacked-shape shading, contact shadows, two-stage
  speculars, and per-fruit silhouettes instead of flat discs.
- Replaced the initial click-to-pop prototype with a proper Bloons-style track,
  keeping the fruit rendering and splat particles from it.

### Fixed

- Losing to the last fruit of a wave credited the wave as cleared, banking the
  bonus and overstating the wave reached — and on a final wave played the victory
  and game-over stings together. Death is settled before completion now.
- A fruit standing on several spike piles spent a spike from each while only ever
  being popped once.
- The window was resizable while the layout is fixed size, so dragging it left
  buttons taking clicks from where they were no longer drawn.
- Every route's exit marker was drawn off-screen, so the thing being defended was
  never visible. The window is wider and the marker is clamped into view.
- The web build failed to link: quad-snd's Web Audio imports need
  `--allow-undefined`, now set in `.cargo/config.toml`.
- Web clicks landed in the wrong place, and the shop was unreachable, because the
  canvas was scaled with a CSS transform — macroquad sizes its buffer from
  `clientWidth` but maps input through `getBoundingClientRect`.
- Right click opened the browser's context menu instead of cancelling.
- The send-wave prompt and the pause overlay drew em dashes as tofu boxes; the
  default font is ASCII-only, now checked by a test.
- The upgrade button's labels overlapped, shop icons overlapped their text, the
  frost overlay washed out pale fruit, and a route blurb overflowed its card.
