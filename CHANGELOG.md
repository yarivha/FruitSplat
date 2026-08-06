# Changelog

All notable changes to Fruit Splat are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Rust + macroquad project scaffold (`Cargo.toml`, `src/` modules, release
  profile with LTO).
- Tower defence core loop: fruit walk a fixed track, the player buys and places
  towers to pop them, and fruit reaching the exit drain lives.
- `Path` — polyline track with cached segment lengths, distance-along-track
  lookup, and a point-to-segment test used to keep towers off the dirt.
- Split ladder: popping a fruit bursts it into two of the next tier down.
  Watermelon → Orange → Lime → Strawberry → Blueberry, which does not split.
  Smaller tiers move faster, so an unhandled watermelon becomes a fast swarm.
- Three towers — Seed Shooter ($90, fast single target), Blender ($170, 58px
  splash), and Freezer ($140, no damage, chills fruit in range to 45% speed).
- "First" targeting: towers engage the fruit furthest along the track.
- Free-form tower placement on open ground, with a live range preview that turns
  red when the spot is unaffordable, off-field, too close to the track, or
  overlapping another tower.
- Wave system — a new fruit tier unlocks every third wave, spawn intervals
  tighten from 0.85s to a 0.30s floor, and clearing a wave pays a bonus.
- Economy: 250 starting cash, $1 per pop, 20 starting lives; leaked fruit cost
  lives equal to their tier (1 for a blueberry, 5 for a watermelon).
- Shop bar with 1/2/3 hotkeys, click-to-place, and right-click to cancel.
- Procedural splat bursts and Freezer pulse rings.
- Fully procedural visuals — grass gradient, dirt track and all fruit drawn from
  macroquad primitives, so the game ships with no image assets.
- 34 unit tests covering path maths (corner traversal, end clamping,
  perpendicular distance, zero-length segments), the split ladder, the slow
  effect and Freezer stacking, wave composition, tower upgrade monotonicity and
  sell values, and validation that every authored route enters and exits
  off-screen and leaves room for towers beside it.
- Procedurally generated audio — 14 sound effects and 2 music loops, produced by
  pure-stdlib Python scripts in `tools/` and embedded with `include_bytes!` so
  the binary stays standalone.
  - Per-tier pop sounds: five pitches, highest for blueberries down to lowest
    for watermelons.
  - Effects for tower fire, Blender splash, Freezer pulse, tower placement,
    rejected placement, fruit leaking, wave start, wave cleared, and game over.
  - `music_game.wav` — 16 bars at 132 BPM, drums, bass, arpeggio and a lead that
    joins on the second pass. `music_menu.wav` — 8 bars at 96 BPM, pad and
    arpeggio, no drums. Both loop seamlessly.
- Audio throttling so a busy field stays readable: pops are capped at 3 per
  frame and each successive one ducks 22%, and the shoot sound is rate-limited
  to one per 55ms across all towers.
- `M` toggles mute, with the current state shown in the HUD strip.
- Tower upgrades — every tower runs Lv1 → Lv3 along one linear track, each kind
  leaning into its existing role:
  - Seed Shooter ($70, $150): faster and longer-reaching, then splits its fire
    across the two lead fruit at Lv3.
  - Blender ($120, $240): splash widens from 58px to 90px and the rate climbs.
  - Freezer ($100, $200): chill deepens from 45% to 25% speed and lasts longer.
- Tower selling — refunds 60% of everything invested in a tower, upgrades
  included.
- Per-tower statistics panel: clicking a placed tower opens a floating panel
  showing its level, current range and rate, shots fired and kills. A Freezer
  reports pulses and fruit chilled instead, since it deals no damage. Upgrade
  and Sell are buttons in that panel — there are no keyboard shortcuts for them.
- Kill attribution: projectiles carry the id of the tower that fired them, so
  kills are credited to the right tower even after other towers are sold. A
  Blender's splash banks every fruit caught in the blast.
- The inspected tower's range ring is drawn on the field, and gold pips under
  each tower show its level without needing a click.
- Overlapping Freezers now stack by keeping the strongest chill and the longest
  remaining duration, instead of the last one to fire winning.
- Route selection — four hand-authored tracks to choose from at the start of a
  run, each shown as a scaled preview of the actual polyline:
  - **Market Run** (Hard, 1500px) — short and direct.
  - **Orchard Snake** (Medium, 2370px) — a steady weave.
  - **Zigzag Grove** (Medium, 2780px) — tight lanes, one tower covers two.
  - **The Long Orchard** (Gentle, 2870px) — plenty of time to shoot.
- Being overrun now returns to route selection rather than restarting the same
  track, so a different route can be picked after a loss.

### Changed

- Replaced the initial click-to-pop prototype. The first pass read "balloon-pop"
  as popping balloons by hand; the intent was Bloons TD, so fruit now follow a
  track and towers do the popping. The fruit rendering and splat particles from
  that pass were kept.
