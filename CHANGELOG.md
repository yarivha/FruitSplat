# Changelog

All notable changes to Fruit Splat are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Rust + macroquad project scaffold (`Cargo.toml`, `src/` modules, release
  profile with LTO).
- Playable core loop: fruit rise from the bottom edge on a swaying path, a left
  click splats the topmost fruit under the cursor, and fruit that reach the top
  count as missed.
- Five fruit varieties (watermelon, orange, lime, strawberry, blueberry) with
  per-kind radius, score value and rise speed — smaller fruit are faster and
  worth more.
- Procedural splat bursts: gravity-affected pulp particles that fade and shrink
  as they expire.
- Difficulty ramp over a 60-second round — spawn interval tightens from 0.95s to
  0.30s and base rise speed climbs from 95 to 190 px/s across the first 45s.
- Menu → Playing → Game Over state machine with a HUD showing score, missed
  count, and a countdown that turns red in the final ten seconds.
- Fully procedural visuals — gradient sky and all fruit drawn from macroquad
  primitives, so the game ships with no image assets.
