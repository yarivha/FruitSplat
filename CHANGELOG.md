# Changelog

All notable changes to Fruit Splat are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Release notes are now the matching `## [x.y.z]` section of this file, lifted
  out at publish time, rather than GitHub's generated list of commit subjects.
  The changelog says what changed and why; a list of subjects says neither, and
  a link makes the reader go and find it. A tag whose version has no section
  here fails the release instead of publishing an empty body.

## [0.2.0] - 2026-08-09

The interface release: the shop moves off the bottom of the window and into a
column down the right, which is what made room for a sixth tower, and the route
picker grows to two rows to make room for a sixth route and a random pick.

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
- **The Durian**, a boss fruit that is the only thing in the game you shoot *at*
  rather than *through*. It breaks both of the ladder's rules at once: it
  carries **60 points of armour**, where every other fruit dies to the first
  thing that touches it, and it bursts into **four whole watermelons** rather
  than a pair of the tier below. Clearing one costs 60 hits plus 124 for the
  payload. At ×0.55 it is the slowest thing on the track, so the swarm it
  arrives with overtakes it and the player gets a long look at what is coming.
  - Hit points are new to the game, so pop resolution became damage
    resolution: hits accumulate on a fruit and only burst it at zero, several
    hits landing in one frame all count, and the killing blow takes the credit.
    Ordinary fruit have one point of armour, so nothing about them changed.
  - An armour bar appears above a boss the moment it takes its first hit,
    running green through amber to red. An untouched boss shows nothing, so the
    bar appearing is itself the signal that the fight has started.
  - Towers sort into roles against it. A knife spends one point of pierce per
    frame inside the husk, so a Lv3 throw is worth six hits — the Knife Thrower
    is the answer. A Spike Layer is not: a pile lands one hit per frame the
    boss stands on it and strips itself in a fraction of a second, leaving it an
    anti-swarm tower, which is what the boss leaves behind when it breaks.
- **Boss waves.** Durians are never produced by the tier unlock ladder — a boss
  is placed, not drifted into. They arrive on wave 15 and every fifth wave
  after, and on a route's final wave whatever its number, so no run can end
  without the fight it has been building toward. Every route runs at least 15
  waves, which a test enforces, so every route meets one.
  - They escalate by **count, not by stats**: one at wave 15, two at 20, three
    at 25. A tougher boss would need a second set of numbers to tune; a second
    boss needs none, and the speed ramp already makes a later one harder.
  - Each is slotted into the last third of the spawn order, so it lumbers in
    once most of the wave is walking rather than arriving alone at either end.
  - The armour number was picked against `balance_report`, not guessed. It puts
    wave 15 on the same 0.9 affordability ratio as wave 13 — the hardest point
    in a run before the boss existed — and leaves waves 20 and 25 at 1.1 and
    1.2 against 1.5 and 1.8 for their neighbours, so each boss wave is a real
    dip and the finale is the hardest thing in a run.
- **The shop is a column down the right of the window**, not a bar along the
  bottom. The bar ran out of window at five buttons; the column has room for
  several more than there are. The window grows to 1420 x 740 to pay for it, so
  the field keeps its full 1200 width and no route had to be redrawn.
  - Losing the bottom bar gave the field the whole window height, 650 to 740.
    Every route moved down 30px to sit centred in the taller field; shapes and
    lengths are untouched, confirmed by re-measuring all of them after.
  - The audio toggles, auto and quit moved out of the top strip and into a block
    at the foot of the column, which is what the strip was running out of room
    for. That block is anchored to the bottom of the window rather than flowing
    after the towers, so adding a tower grows the column downward into empty
    space instead of pushing the controls off screen.
- **Pause**, in that same block. Input keeps running while the game is held:
  towers can be bought, upgraded and sold with the wave standing still, which is
  the point of pausing rather than a side effect. Nothing else advances — not the
  spawn clock, not cooldowns, not the auto-send timer. The overlay dims the field
  only, leaving the column lit so its buttons still read as live.
  - Its label is plain ASCII, and now so is every other drawn string: the
    default font has no glyph past ASCII, so an em dash draws as a tofu box.
    That reached the screen twice — the send-wave prompt, then this overlay — so
    a test scans `render.rs` for non-ASCII inside string literals rather than
    trusting anyone to remember. Comment lines are skipped, and only `render.rs`
    is scanned, because only `render.rs` draws text.
- **Triple Seeder** ($260), a sixth tower that throws at three separate fruit a
  volley, four once maxed. The dearest thing in the shop on purpose: answering a
  crowd on its own is what the Blender and the Knife Thrower each do half of, and
  being able to buy that early would flatten the reason those two differ. Three
  barrels fanned around its aim, so what it does is legible from the board rather
  than only from its stats panel.
- **"Meander"**, a sixth route and the longest at 3710px, so also the gentlest —
  length does all the work, which is what makes it the one to learn on. Bright
  open meadow backdrop, the lightest of the six.
- **Route cards in two rows of four**, plus a seventh card, **Surprise Me**, that
  starts a run on one of the six picked at random. At seven cards a single row
  left each one 185px wide, too narrow to read its own name; two rows make them
  279px. The random card draws every route's outline layered faintly on itself.
  - It sits last so adding a route never shifts it out from under the player's
    finger, and it needed a seventh number key. That left more keys than towers,
    and the tower hotkey loop indexed `TowerKind::ALL` by key position, so it
    zips instead of indexing and can no longer run off the end.
  - The cards centre on the field rather than the window, because the audio
    toggles are drawn on every screen at their column position and are
    hit-tested before anything else — a card reaching under one lost its clicks
    to it.
- Five towers — Seed Shooter ($90, fast single target), Blender ($170, 58px
  splash), Knife Thrower ($130, knives pierce 3 fruit), Spike Layer ($150,
  spikes on the track), and Freezer ($140, no damage, chills fruit in range to
  45% speed).
- **Spike Layer** ($150), a fifth tower that doesn't shoot. It drops piles of
  spikes onto the track itself; each pile is worth one hit per spike, then wears
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
- "First" targeting: towers engage the fruit furthest along its lane.
- Free-form tower placement on open ground, with a live range preview that turns
  red when the spot is unaffordable, off-field, too close to the track, or
  overlapping another tower.
- Wave system — a new fruit tier unlocks every third wave, spawn intervals
  tighten from 0.85s to a 0.30s floor, and clearing a wave pays a bonus.
- Economy: $1 per fruit destroyed outright plus a wave clear bonus; leaked fruit
  cost lives equal to their tier (1 for a blueberry, 5 for a watermelon, 6 for a
  Durian). What a run opens with is set by its difficulty mode.
- Shop bar with 1/2/3 hotkeys, click-to-place, and right-click to cancel.
  - Placing a tower disarms the selection. Keeping the type armed let several of
    one kind go down in a row, but while anything is armed a field click always
    *places* — so a placed tower could not be clicked to open its panel, and a
    stray click bought another tower. Clearing that needed a right click, which
    on the web build the browser took for its own context menu. Re-arming is one
    click or one number key; being unable to touch the field until you cancelled
    cost more.
  - The web page suppresses the browser context menu over the canvas, so right
    click reaches the game as the cancel it is meant to be.
- The build's version in the bottom right corner of the title screen, dim enough
  to ignore and readable when looked for. Read from `CARGO_PKG_VERSION` rather
  than written out: a hand-kept string is exactly the kind that ends up
  disagreeing with the release it shipped in, which is the one question this
  answers.
- Procedural splat bursts and Freezer pulse rings.
- Fully procedural visuals — grass gradient, dirt track and all fruit drawn from
  macroquad primitives, so the game ships with no image assets.
- 127 unit tests covering path maths (corner traversal, end clamping,
  perpendicular distance, zero-length segments), the split ladder, boss armour
  and the N-ary burst, the slow effect and Freezer stacking, wave composition
  and the boss schedule, tower upgrade monotonicity and sell values, spike-pile
  charge accounting and where a Spike Layer drops its next pile, difficulty
  modes ordering on both dials at once, lane isolation
  (a pile never reaching another lane, spacing staying per-lane, targeting
  ranking by lane fraction), validation that every authored lane enters and exits
  off-screen, leaves room for towers beside it, runs long enough to meet the boss
  and never shares a corridor with another lane, UI layout assertions that the
  shop buttons, the audio toggles, the quit button and both card text columns fit
  without overlapping, and scenery checks covering prop placement, determinism
  and the local RNG's range.
- Procedurally generated audio — 14 sound effects and 2 music loops, produced by
  pure-stdlib Python scripts in `tools/` and embedded with `include_bytes!` so
  the binary stays standalone.
  - Per-tier pop sounds: five pitches, highest for blueberries down to lowest
    for watermelons, plus a sixth for the Durian — a splintering crack over a
    long low body rather than another pop. It is exempt from the per-frame pop
    cap and the ducking, because it lands in exactly the crowded frame the cap
    exists to thin out, and it is the one pop the player is waiting to hear.
  - Effects for tower fire, Blender splash, Freezer pulse, tower placement,
    rejected placement, fruit leaking, wave start, wave cleared, and game over.
  - `music_game.wav` — 16 bars at 132 BPM, drums, bass, arpeggio and a lead that
    joins on the second pass. `music_menu.wav` — 8 bars at 96 BPM, pad and
    arpeggio, no drums. Both loop seamlessly.
- Audio throttling so a busy field stays readable: pops are capped at 3 per
  frame and each successive one ducks 22%, and the shoot sound is rate-limited
  to one per 55ms across all towers.
- `.cargo/config.toml` passing `--allow-undefined` to the linker for
  `wasm32-unknown-unknown`. quad-snd reaches the Web Audio API through
  `extern "C"` declarations that nothing in the Rust build defines — the JS
  bundle supplies them at runtime — so on wasm they have to link as imports.
  Whether wasm-ld does that unprompted turns out to depend on the toolchain: it
  linked without complaint locally and failed the CI runner with
  `undefined symbol: audio_init`. It lives in cargo config rather than the
  workflow so a local wasm build gets it too.
  - The workflow's global `RUSTFLAGS: -D warnings` had to go with it. Setting
    `RUSTFLAGS` in the environment *replaces* the rustflags in cargo config
    rather than adding to them, so it would have silently dropped the new flag
    and broken the web job again, with an error pointing nowhere near either
    file. `-D warnings` is passed to clippy directly instead.
- **GitHub Actions workflow** building Linux, macOS, Windows and web for a `v*`
  tag, and attaching all four to a release. Branch pushes run the checks alone.
  - There is no trigger on branch pushes, so a tagged release is exactly one
    run. GitHub starts a run per *ref* rather than per commit, so triggering on
    `main` as well meant two runs for every release — first building the same
    commit twice, then, once the builds were gated on the tag, still appearing
    as a second entry running the checks.
  - The cost is that a push to main runs nothing. The checks still gate the
    release from inside the tag run, so a broken tree fails the release rather
    than shipping a bad one, but it is caught at tag time rather than push time.
    `workflow_dispatch` runs the checks and builds on demand, without spending a
    version number.
  - macOS is a universal binary. The runners are Apple Silicon, so an arm64
    build alone would not start on an Intel Mac; both slices are built and
    joined with `lipo`.
  - Linux installs the X11, GL and ALSA headers macroquad links against — ALSA
    because the audio feature is enabled.
  - The web job takes macroquad's JS bundle from the crate source already
    unpacked in the cargo registry, pinned to the version in `Cargo.lock`, so
    the glue can never drift from the library it is gluing. It then checks all
    three files are present and non-empty before publishing, because a page
    missing any one of them is a black screen with no error in the console.
  - Builds are gated on `cargo fmt --check`, `cargo clippy` and `cargo test`.
    The tree did not pass rustfmt, so it was formatted — the gate is worth
    having only if it is true.
- `web/index.html`, the page a wasm build is served from. The canvas is left at
  its authored 1200x740 and deliberately not scaled, because macroquad's JS
  bundle measures it two different ways: the drawing buffer comes from
  `clientWidth`, which ignores CSS transforms, while mouse input is mapped
  through `getBoundingClientRect()`, which does not. Scaling with a transform
  makes those disagree by exactly the scale factor — the game renders correctly
  but every click lands compressed toward the top-left, and below a scale of
  0.88 the shop bar is unreachable and no tower can be armed. Sizing with CSS
  width and height keeps them in agreement but lets the buffer follow the
  window, so `screen_width()` stops being the 1200 the HUD, shop bar and
  route-card row are laid out against. Browser zoom is the way to fit a smaller
  screen; it scales layout and hit-testing together.
- **Wider window: 1200 x 740, up from 1000 x 740.** Every route ran off the
  right edge with its last stretch still to come, so the approach to the exit
  could not be seen and fruit vanished mid-field while still shootable.
  - The exit marker is now drawn **clamped back inside the window** instead of
    at the route's terminal waypoint. Routes deliberately end off-screen so
    fruit leave rather than blink out at the border, which meant the marker was
    drawn every frame, entirely outside the window, on every route — the one
    thing the player is defending had never been visible. Clamped, it sits where
    the track crosses out of the field, which is also the last point a fruit can
    still be shot at.
  - Routes were reshaped rather than merely stretched: each one's rightmost
    vertical moved out by 200px and its final run shortened by the same amount,
    so the widened field is used instead of leaving a dead column of grass, and
    every route's length is unchanged to the pixel by the reshaping. Twin Gates'
    two lanes now hold 140px apart until the final dive, so they stay clearly
    two lanes right up to the point they merge.
  - Every route is nonetheless 200px longer than before the window changed,
    because the exits moved out with the edge. That is a small uniform easing:
    fruit spend about two extra seconds under fire on the shortest route.
- **AUTO toggle** in the top strip: waves send themselves three seconds after
  the field clears, so a run can be played without reaching for the keyboard
  between every wave. The between-waves prompt counts the gap down rather than
  asking for Space, and the setting persists across runs like the difficulty.
  - The gap is deliberately not zero. Between waves is when towers get bought,
    placed and upgraded, which is most of the game's decision-making — sending
    instantly would take that away rather than automate it. Space still
    overrides the timer for anyone who wants the wave now.
  - It cannot send past the end of a run: the countdown is ticked after wave
    completion is settled, so clearing a route's last wave lands on the victory
    screen instead of counting down to a wave that does not exist.
  - `draw_hud` now takes a `HudState` rather than eight positional arguments,
    which is where it was heading.
- **Three difficulty modes** — Easy, Medium and Hard — picked from a row above
  the route cards before choosing a route. The setting carries between runs, the
  HUD names it during play, and the victory screen records which one a route was
  cleared on.
  - A mode changes only **what you start with**: cash ($300 / $180 / $120) and
    lives (25 / 15 / 8). It never touches what a wave sends. The wave table, the
    speed ramp and the per-fruit payout are tuned together against
    `balance_report`, and a mode that quietly rewrote them would make that whole
    curve a fiction on two runs out of three.
  - The two dials do different jobs at different times, which `balance_report`
    now shows by printing a table per mode. Cash is the opening hand and only
    that: the affordability ratio spreads 2.9 to 6.6 at wave 1 and has converged
    to 0.9-1.0 by wave 15, because cumulative income dwarfs anything you started
    with. Lives carry the rest — 25 absorbs four leaked Durians, 8 does not
    survive two leaked watermelons — and that is what keeps the modes apart once
    the money has evened out. Cash alone would have left Easy and Hard genuinely
    indistinguishable from the midpoint on.
  - **Easy also ramps fruit speed up more gently** — 1.5% a wave to a 1.35x cap,
    against Medium's tuned 3.5% to 1.90x. Cash and lives alone could not keep
    the modes apart: both fade, and by wave 13 all three converged on the same
    fight, so Easy played as merely rich early rather than easy. Speed is the
    one dial that never fades, because it divides how much every tower gets done
    on every wave. Easy's affordability ratio now stays at or above 1.2 for a
    whole run where Medium and Hard both drop below 1.0 at wave 13 and again at
    the first boss wave.
  - Speed is the only thing a mode changes about the waves themselves, and the
    line still holds elsewhere: fruit, counts, order, boss schedule and payouts
    are identical on all three, so the economy remains one curve.
  - Hard deliberately keeps the tuned ramp rather than a steeper one — it is
    meant to be the balanced fight with no slack, not a faster one.
  - Medium is the tuned baseline, asserted against the cash, lives and ramp the
    balance work was done at, so it cannot drift off them without a test
    failing. The mode buttons print the speed cap alongside cash and lives,
    since the dial that decides how the late waves feel would otherwise be
    invisible at the point of choosing.
- **Lanes.** A route is now one or more polylines rather than exactly one, and
  every fruit and spike pile carries the index of the lane it belongs to. Lanes
  stay independent for their whole length even where they converge on screen: a
  fruit never changes lane, and a pile never touches another lane's fruit.
  - **"Twin Gates"**, a fifth route (Tricky, 18 waves) and the first with two
    lanes. Fruit enter at two gates on the left, one high and one low, and both
    streams leave by the same exit on the right. Each wave is dealt to the gates
    in turn, so it arrives as two half-strength streams instead of one the player
    can meet head on. Neither lane is long — the difficulty is that they are far
    apart, so one cluster of towers cannot answer both and the same money has to
    cover two approaches.
  - The lanes converge on the exit *point* rather than sharing a stretch of track
    before it. A shared corridor would look right and play wrong: piles belong to
    one lane, so a Spike Layer covering the shared run would pop only half the
    fruit walking over it, for no reason the player could see. A test walks both
    lanes and fails if they ever come within 60px outside the exit.
  - **Targeting now ranks fruit by fraction of their lane walked, not by raw
    distance.** The lanes are different lengths, so 900px along the short one is
    nearer its exit — and so more urgent — than 900px along the long one.
    Comparing raw distances would have had every tower covering both gates
    quietly favour whichever lane happened to be longer.
  - Tower placement clearance, scenery rejection and Spike Layer drop spots all
    consider every lane. Pile spacing stays a per-lane rule, so a pile on one
    lane cannot block a drop on the other where the two run close together.
  - Route cards size themselves to however many routes exist — at the old fixed
    220px a fifth card ran 164px off the window — and their previews draw every
    lane with each entrance marked, so a two-lane route is recognisable from the
    selection screen rather than only once the first wave is walking.
  - New "Highland" backdrop for it: cold, thin upland grass over pale stone, the
    bluest of the five, and a sparser scatter of props since two lanes leave less
    open ground to read as buildable.
- **QUIT RUN** button in the HUD strip, to abandon a run in progress and go back
  to the title screen. Drawn only during play — there is nothing to quit from
  the title screen.
  - It asks before it acts: the first click turns it into **SURE?** and the
    second confirms. It sits a dozen pixels from the audio toggles, and one
    stray click there silently throwing away a twenty-five wave run would be
    unforgivable. The button does the asking rather than a dialog, so nothing
    has to interrupt the wave to ask it.
  - The arming lapses after three seconds, and a right click stands it down —
    right click is already this game's cancel.
  - Quitting sweeps the board, so the title screen looks like a title screen
    rather than the wreckage of the run just walked away from. Board clearing
    is now shared with starting a run, so neither can forget a list the other
    clears.
- Two audio toggles in the HUD strip — a speaker for sound effects and a note
  for music — each silencing its half independently. Someone playing with the
  game in the background usually wants one or the other gone, not both, which a
  single switch can't express.
  - They are drawn on every screen, not just during play. The menu and the end
    screens have music too, and a mute you can only reach mid-run is a mute you
    reach too late.
  - They take a click before any screen sees it, so muting from the title screen
    doesn't also start a run, and muting mid-wave doesn't try to place a tower
    behind the button. The run keeps simulating either way: only the click is
    spent, never a frame.
  - Muted music resumes from the top rather than where it stopped — macroquad
    gives no way to seek a playing sound.
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
- Runs are finite: every route has a wave count, and surviving them all clears
  it. Harder routes run shorter — Market Run is 15 waves, Orchard Snake and
  Zigzag Grove 20, The Long Orchard 25 — so difficulty is a sharper challenge
  rather than simply a longer one. Every route runs past wave 13, where
  watermelons first appear, which is asserted by a test.
- The HUD shows progress as `WAVE 7/20` rather than a bare wave number, and the
  counter turns amber on the final wave. The send-wave prompt names the total
  too, and calls out the final wave explicitly.
- New victory screen when a route is cleared, showing the route name, its wave
  count and the lives left, with its own fanfare.
- Route cards show wave count instead of route length in pixels — how long a run
  is is what the player is actually choosing between.

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

- **Difficulty pass — the game was far too easy.** A balance report over the
  real wave tables showed the player could afford roughly six times the
  firepower a wave needed, and that the gap *widened* after wave 13: the game
  got easier the longer it ran. Two causes, both now addressed.
  - Income was proportional to pops, so a watermelon paid $31. Towers are a
    one-time cost, so the surplus compounded. Cash is now earned only for fruit
    **destroyed outright** — the bottom of the split ladder — which roughly
    halves late income while still paying more for bigger fruit. The wave clear
    bonus drops from `25 + wave*4` to `15 + wave*2`.
  - Past wave 13 the only escalation was sending *more* fruit; the fruit
    themselves never got harder. Fruit now gain 3.5% speed per wave, capped at
    1.9×, and children inherit their parent's ramp. Since a tower's output is
    bounded by how long a fruit stays in range, this is the escalation that
    count alone couldn't provide.
  - Starting lives 20 → 15 and starting cash $250 → $180.
  - Affordable-versus-needed firepower now runs about 4× at wave 1, dips to
    0.9× at wave 13 where watermelons arrive, and drifts to 1.7× by wave 25 —
    a real wall instead of a slide into surplus.
- Towers now **lead their shots**, aiming where a fruit will be when the shot
  lands rather than where it is. Without this the speed ramp would have made
  towers miss constantly through no fault of the player.
- `balance_report` — an ignored test that prints the difficulty and economy
  curves, so tuning is done against numbers instead of guesswork. Run with
  `cargo test balance_report -- --ignored --nocapture`.
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
- Losing to the last fruit of a wave credited the wave as cleared on the way to
  the game over screen. One leak can both drain the last life and empty the
  field, and completion was tested before death, so the run banked the clear
  bonus and advanced the wave counter — leaving the game over screen reporting a
  wave the player never reached. On a route's final wave it was louder still:
  victory was declared, its jingle and the menu music started, and the game over
  state overwrote it in the same frame, so both stings played together. Death is
  now settled first.
- The window was resizable while the world is a fixed 1000x650. Routes, scenery
  and the shop bar are authored against that space, but the HUD and hit-testing
  read the live window size, so dragging the window pulled the two apart —
  narrowing it left the shop buttons taking clicks from where they were no
  longer drawn, and shortening it dropped the whole bar off the bottom, since
  PLAYFIELD_H never yields the strip back. The window is now fixed size.
- A fruit standing on several spike piles at once spent a spike from every one
  of them while only ever being popped once. Piles sit `SPIKE_SPACING` apart and
  each reaches `radius + SPIKE_RADIUS` along the track, so overlap is the norm
  rather than the exception — a watermelon is wide enough to sit on three at
  once, burning three spikes for one pop against the rule the tower is sold on.
  Fruit are now walked over the piles rather than piles over the fruit, so one
  fruit costs exactly one spike. Guarded by tests.
