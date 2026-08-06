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
- 19 unit tests covering path maths (corner traversal, end clamping,
  perpendicular distance, zero-length segments), the split ladder, slow effect,
  and wave composition.

### Changed

- Replaced the initial click-to-pop prototype. The first pass read "balloon-pop"
  as popping balloons by hand; the intent was Bloons TD, so fruit now follow a
  track and towers do the popping. The fruit rendering and splat particles from
  that pass were kept.
