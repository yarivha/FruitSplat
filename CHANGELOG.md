# Changelog

All notable changes to Fruit Splat are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries say what changed, briefly. The reasoning behind a change lives in its
commit message and in the comments around the code it touched.

## [Unreleased]

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

- The Triple Seeder's 0.70s cooldown cancelled its own triple out: at Lv3 its
  sustained rate came to exactly a Seed Shooter's, for two and a half times the
  money, and $260 of Seed Shooters out-shot it at every level. It fires on the
  Seed Shooter's cadence now — three seeds where that tower puts one — and gains
  a seed per upgrade instead of losing rate.
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
