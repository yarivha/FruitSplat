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
- Five towers — Seed Shooter ($90, fast single target), Blender ($170, 58px
  splash), Knife Thrower ($130, knives pierce 3 fruit), Spike Layer ($150,
  spikes on the track), and Freezer ($140, no damage, chills fruit in range to
  45% speed).
- **Spike Layer** ($150), a fifth tower that doesn't shoot. It drops piles of
  spikes onto the track itself; each pile pops one fruit per spike, then wears
  away. Spikes never miss, so the limits are pile size and how many piles a
  tower may keep on the track at once (3 at Lv1). Strong against splits, since
  children spawn where their parent died — on top of the same pile.
  - Piles are tested against fruit by **distance along the route**, not
    euclidean distance. On the switchback routes two stretches of track run
    within a few dozen pixels of each other, and a pile on one lane must not pop
    fruit walking the other.
  - Upgrades ($130, $260): 4 → 6 → 9 spikes per pile, 3 → 4 → 5 piles allowed,
    and a faster drop rate.
  - Selling a Spike Layer removes its piles, which keeps the total bounded —
    otherwise repeatedly building and selling would litter the track with
    orphans nothing cleans up.
  - New piles go at the furthest covered point on the track and work backwards
    as spots fill, so a tower spreads its spikes across its whole stretch.
- Shop buttons now use abbreviated tower names, so five of them plus the hint
  column still fit the window.
- Pierce as a mechanic: a projectile carries a pierce budget and survives each
  hit until it runs out, instead of being consumed by the first fruit it
  touches. Only the Knife Thrower exceeds 1. Splash hits a blob of fruit around
  a point; pierce hits a line of them along the shot's path, which makes the
  Knife Thrower strongest on the switchback routes.
- Knife Thrower upgrades ($110, $220): pierce 3 → 4 → 6, with a faster throw at
  Lv2. Its knives tumble end over end in flight and it has its own metallic
  whoosh, gated separately from the seed thwip so a field of Seed Shooters can't
  silence it.
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
- 54 unit tests covering path maths (corner traversal, end clamping,
  perpendicular distance, zero-length segments), the split ladder, the slow
  effect and Freezer stacking, wave composition, tower upgrade monotonicity and
  sell values, validation that every authored route enters and exits off-screen
  and leaves room for towers beside it, UI layout assertions that the four shop
  buttons and the hint column fit the window without overlapping, and scenery
  checks covering prop placement, determinism and the local RNG's range.
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

- Screenshot mode (`FRUITSPLAT_SCREENSHOT`, `FRUITSPLAT_SCREEN`): stages a
  scene, renders a few frames, writes a PNG and exits. Added to make the
  artwork reviewable without playing to the right moment.

- Per-route backdrops. Each route gets its own palette and a themed scatter of
  decorative props, generated procedurally like the rest of the artwork:
  - **Orchard Snake** — temperate orchard of trees, bushes and flowers.
  - **Market Run** — sun-baked and dusty, with crates and fences.
  - **The Long Orchard** — lush farmland with ponds.
  - **Zigzag Grove** — dark, dense forest.
  - Props are laid out once per run by rejection sampling, kept clear of the
    track, sorted back to front, and placed by their drawn extent so ponds
    can't hang off the window edge and tree canopies can't reach into the HUD.
  - Layout is seeded from the route index so a route always looks the same. It
    uses a local xorshift generator rather than macroquad's global RNG —
    reseeding that to make scenery repeatable would also have made the fruit
    order repeatable.
  - Scenery never blocks tower placement and draws under the track, so it can
    never obscure the route.
  - Route selection cards preview each route in its own palette.

### Changed

- Reworked all the artwork. macroquad has no gradient primitive or clipping, so
  shading is faked by stacking shapes: a dark base, a mid body, then smaller
  layers offset toward a light treated as coming from the upper left.
  - Fruit and towers gained soft contact shadows, two-stage speculars (a soft
    bloom with a tight hot spot) and volume shading, replacing flat discs with a
    single highlight blob.
  - Strawberries are drawn conical — shoulders plus a tip with a fanned calyx —
    rather than as a circle. Watermelons read as a cut cross-section with pith
    and oriented seeds, oranges gained peel pores and a veined leaf, limes got a
    darker rind for contrast against the flesh, blueberries a dusty bloom and a
    sunken crown.
  - Towers stand on a stone footing instead of floating on the grass.
  - Shop buttons show the real tower artwork scaled down, instead of a flat
    colour swatch.
- Replaced the initial click-to-pop prototype. The first pass read "balloon-pop"
  as popping balloons by hand; the intent was Bloons TD, so fruit now follow a
  track and towers do the popping. The fruit rendering and splat particles from
  that pass were kept.

### Fixed

- The send-wave prompt drew an em dash, which the default font has no glyph for
  and rendered as a tofu box.
- The upgrade button's cost and effect labels overlapped on longer labels; the
  effect now sits on its own line above the button.
- Shop button icons overlapped the button text.
- The frost overlay on chilled fruit was heavy enough to wash pale fruit like
  the lime into an unreadable blob.
- "Zigzag Grove"'s blurb overflowed its selection card, now guarded by a test.
