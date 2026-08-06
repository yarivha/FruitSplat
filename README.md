# Fruit Splat

A Bloons-style tower defence, reskinned with fruit. Written in Rust with
[macroquad](https://github.com/not-fl3/macroquad).

Fruit roll along a winding track toward the exit. You don't touch them directly —
you buy towers and place them along the route. Pop a fruit and it bursts into two
smaller, faster ones, so a watermelon you failed to handle early arrives as a
swarm of blueberries later.

## Play

```sh
cargo run --release
```

| Input | Action |
|---|---|
| `1`–`4` on the route screen | Pick a route |
| `1`–`5` in play | Arm a tower type |
| Click shop button | Arm a tower type |
| Left click on field | Place the armed tower |
| Left click a placed tower | Open its stats panel (nothing armed) |
| Click **Upgrade** / **Sell** | Upgrade or sell that tower |
| Right click | Cancel placement / close the panel |
| `Space` | Send the next wave |
| `M` | Mute / unmute |

Close the window to quit.

## Routes

Each run starts by picking one of four routes. Length is the main difficulty
dial — a longer route means more seconds under fire before a fruit reaches the
exit. Turn count matters too: tight switchbacks let one tower cover several
lanes at once.

| Route | Difficulty | Waves | Length | Character | Backdrop |
|---|---|---|---|---|---|
| Market Run | Hard | 15 | 1500 px | Short and direct | Dusty market of crates and fences |
| Orchard Snake | Medium | 20 | 2370 px | A steady weave | Temperate orchard |
| Zigzag Grove | Medium | 20 | 2780 px | Tight lanes, one tower covers two at once | Dark, dense forest |
| The Long Orchard | Gentle | 25 | 2870 px | Plenty of time to shoot | Lush farmland with ponds |

A run is finite: survive every wave on the route and it's cleared. Harder routes
run **shorter**, so they're a sharper challenge rather than simply a longer one.
The HUD shows progress as `WAVE 7/20`, and the counter turns amber on the final
wave.

Each route has its own palette and a themed scatter of scenery — trees, bushes,
rocks, flowers, crates, fences, ponds — laid out once when the run starts.
Placement is seeded from the route index, so a given route always looks the
same, and it uses a local random generator rather than the global one so making
the scenery repeatable doesn't also make the fruit order repeatable.

Scenery is purely decorative: it never blocks tower placement, and it is drawn
underneath the track so foliage can't obscure the route.

## Towers

| Tower | Cost | Range | Rate | Role |
|---|---|---|---|---|
| Seed Shooter | $90 | 135 px | 0.45s | Single target, cheap sustained damage |
| Blender | $170 | 110 px | 1.10s | 58px splash — the answer to clustered splits |
| Knife Thrower | $130 | 145 px | 0.75s | Knives pierce 3 fruit and keep flying |
| Spike Layer | $150 | 120 px | 2.20s | Drops spike piles onto the track itself |
| Freezer | $140 | 120 px | 1.40s | No damage; chills fruit to 45% speed for 1.6s |

The Blender and the Knife Thrower both beat crowds, but differently: splash hits
a **blob** of fruit around one point, while pierce hits a **line** of them along
the shot's path. That makes the Knife Thrower strongest on the switchback
routes, where fruit queue up single file down a lane.

The Spike Layer is the odd one out — it doesn't shoot. It drops piles of spikes
directly onto the track, and each pile is worth **one hit per spike** before it
wears away. Against everything except the boss a hit is a pop, so that reads as
one fruit per spike. Spikes never miss, so the limits are the pile size and how
many piles a tower may have out at once (3 at Lv1). It's especially good against
splits: children spawn where their parent died, right on top of the same pile.

A fruit costs exactly one spike however many piles it happens to be standing on.
Piles sit 34px apart and each reaches `radius + 14px` along the track, so overlap
is the norm — a watermelon is wide enough to sit on three at once.

A pile is tested against fruit by **distance along the route**, not by how close
it is in pixels. On the switchback routes two stretches of track run within a
few dozen pixels of each other, and a pile on one lane must not pop fruit
walking the other.

Towers use "first" targeting — they shoot whichever fruit in range is furthest
along the track. Shots don't home, but towers **lead** their targets: they aim
at where the fruit will be once the shot arrives, which is what keeps them
useful against the late-wave speed ramp.

### Stats and upgrades

Click a placed tower with nothing armed to open its panel. The panel floats
beside the tower — flipping to its other side near the window edge — and reports
what that specific tower has actually done:

| Tower | Stats shown |
|---|---|
| Seed Shooter | Range, rate, shots fired, kills |
| Blender | Range, splash, shots fired, kills |
| Knife Thrower | Range, pierce, knives thrown, kills |
| Spike Layer | Spikes per pile, max piles, piles dropped, kills |
| Freezer | Range, chill strength, pulses, fruit chilled |

Kills are credited to the tower whose shot landed, so a Blender's splash banks
every fruit in the blast. A Freezer never gets kills — it deals no damage — so it
reports what it has slowed instead.

Upgrade and sell are buttons in that panel. Every tower runs Lv1 → Lv3 along one
track, and selling refunds **60%** of everything invested, upgrades included.

| Tower | Lv2 | Lv3 |
|---|---|---|
| Seed Shooter | $70 — faster, longer reach | $150 — twin shot, fires at the two lead fruit |
| Blender | $120 — splash 58 → 72 px | $240 — splash 90 px, faster |
| Knife Thrower | $110 — pierce 3 → 4, faster | $220 — pierce 6 |
| Spike Layer | $130 — 6 spikes/pile, 4 piles, faster | $260 — 9 spikes/pile, 5 piles |
| Freezer | $100 — chill 45% → 35% | $200 — chill 25%, lasts 2.3s |

Overlapping Freezers stack by keeping the strongest chill and the longest
remaining duration.

## Fruit

Popping a fruit splits it into **two** of the next tier down. Clearing a single
watermelon therefore takes 31 pops in total.

| Tier | Fruit | Radius | Speed | Armour | Splits into | Lives lost if leaked |
|---|---|---|---|---|---|---|
| 5 | **Durian** | 42 px | ×0.55 | **60** | 4 × Watermelon | 6 |
| 4 | Watermelon | 30 px | ×0.72 | 1 | 2 × Orange | 5 |
| 3 | Orange | 24 px | ×0.88 | 1 | 2 × Lime | 4 |
| 2 | Lime | 19 px | ×1.00 | 1 | 2 × Strawberry | 3 |
| 1 | Strawberry | 15 px | ×1.12 | 1 | 2 × Blueberry | 2 |
| 0 | Blueberry | 11 px | ×1.38 | 1 | — | 1 |

### The Durian

The boss, and the only fruit you shoot *at* rather than *through*.

Everything below it expresses toughness through the size of the subtree it is
hiding — a watermelon is hard because it is 31 pops, not because it is durable,
and it dies to the first seed that touches it like everything else. The Durian
breaks both rules at once. It carries **60 points of armour**, so it has to be
worn down before it will give anything up, and it is packed with **four whole
watermelons** rather than a pair of the tier below. Clearing one outright costs
60 hits plus 124 more for the payload.

It also lumbers, at ×0.55 — slower than anything else on the track. That is most
of what makes it readable: the swarm it arrives with overtakes it, so you get a
long look at what is coming before it reaches your guns. An armour bar appears
above it the moment it takes its first hit, running green through amber to red.

Because it is armoured, the towers sort themselves into roles against it:

- **Knife Throwers** are the answer. A knife spends one point of pierce per
  frame it spends inside the husk, so a single Lv3 throw is worth six hits.
- **Blenders** hit it and the swarm around it with one shot, but only for one
  point each.
- **Spike Layers** are not boss weapons. A pile lands one hit per frame the
  Durian stands on it, so it strips itself in a fraction of a second — they stay
  an anti-swarm tower, which is exactly what the boss leaves behind when it
  finally comes apart.
- **Freezers** still work on it, and buying the field a few extra seconds of
  fire is worth more here than anywhere else in the game.

## Waves

A new tier unlocks every third wave, so watermelons first appear on wave 13 —
every route runs long enough to see them. Spawn intervals tighten from 0.85s
toward a 0.30s floor, and fruit get **3.5% faster each wave**, capped at 1.9×.
You start with 15 lives and $180.

### Boss waves

Durians are not part of that unlock ladder — a boss is placed, never drifted
into. They arrive on **wave 15 and every fifth wave after it**, and on a route's
**final wave** whatever its number, so no run can end without the fight it has
been building toward. Every route is at least 15 waves long, so every route
meets one.

They escalate by **count, not by stats**, the same way everything else in this
game escalates: wave 15 sends one Durian, wave 20 sends two, wave 25 sends three.
A tougher boss would need a second set of numbers to tune; a second boss needs
none, and the speed ramp already makes a later one harder to break.

Each one is slotted into the last third of the spawn order, so it lumbers in
once most of the wave is already walking rather than arriving alone at either
end.

Clearing the route's last wave wins the run. Losing all your lives ends it, and
either way you return to the route picker.

### Economy

Cash is earned for each fruit **destroyed outright** — one with no children left
to split into — plus a bonus for clearing a wave. Paying per *pop* instead meant
a watermelon was worth $31, so income scaled with the threat while towers stayed
a one-time cost. The surplus compounded until you could afford six times the
firepower a wave actually needed.

Because towers accumulate and income doesn't have to, income must grow *slower*
than the threat for difficulty to hold steady.

### Checking the balance

```sh
cargo test balance_report -- --ignored --nocapture
```

Prints the difficulty and economy curves per wave: fruit sent, bosses among
them, hits required, income, the speed ramp, the hits per second needed to keep
up, and how much firepower the cumulative income could buy. The last column is
the ratio — 1.0 means you can afford exactly enough, below 1.0 means the wave
outpaces you.

The curve is deliberately generous early (about 4× at wave 1) and tightest at
wave 13, where watermelons first arrive, drifting back toward 1.8× by wave 24.
**Boss waves cut across that drift**, which is the point of them:

| Wave | 13 | 14 | 15 | 19 | 20 | 24 | 25 |
|---|---|---|---|---|---|---|---|
| Durians | — | — | 1 | — | 2 | — | 3 |
| Ratio | 0.9 | 1.0 | **0.9** | 1.5 | **1.1** | 1.8 | **1.2** |

Each boss wave demands 28% to 67% more work per second than the wave before it,
the gap widening as the count climbs, so waves 20 and 25 stay the two spikes of
a run rather than flattening out against a field that has had ten more waves to
grow. The Durian's armour was picked against this table, not guessed.

It's a model, not a simulation: it ignores upgrades, splash and pierce
multi-kills, and spikes, all of which make the real game more forgiving than the
numbers suggest — and it charges a knife one hit per point of pierce spent on a
boss, where the real thing lands all six.

## Audio

All music and sound effects are **generated procedurally** by pure-standard-library
Python scripts — no samples, no numpy, no external audio tools. The resulting WAVs
are embedded into the binary with `include_bytes!`, so the executable is
standalone.

```sh
python3 tools/gen_sounds.py   # 20 effects → assets/
python3 tools/gen_music.py    # 2 music loops → assets/
```

Both scripts use a fixed random seed, so regenerating reproduces the existing
files byte for byte. The committed `assets/*.wav` are a build-time input only —
nothing reads them at runtime.

> **Adding a sound:** put its `write_wav` call at the **end** of `main()` in
> `gen_sounds.py`. Every generator draws from one shared seeded stream, so
> inserting a call earlier shifts the random numbers each later sound receives
> and silently rewrites unrelated `.wav` files.

Fruit pop at five different pitches, highest for blueberries down to lowest for
watermelons, and the Durian gets a sixth clip of its own — a splintering crack
over a long low body rather than another pop. To keep a busy field readable,
pops are capped at 3 per frame with each successive one 22% quieter, and the
firing sounds are rate-limited — seeds and knives on separate gates, so a row of
Seed Shooters can't silence every Knife Thrower.

The boss burst is exempt from that cap and from the ducking. It is the payoff
for a fight that ran most of a route, and it lands in exactly the crowded frame
the cap exists to thin out — so the cap would swallow the one pop the player is
waiting to hear.

## Layout

| File | Purpose |
|---|---|
| `src/main.rs` | Window config, game state, frame loop, economy |
| `src/path.rs` | Track polyline, distance lookup, placement clearance |
| `src/fruit.rs` | Fruit tiers, the split ladder, splat particles |
| `src/tower.rs` | Tower stats, spike piles, the Freezer pulse effect |
| `src/projectile.rs` | Shots in flight, their splash radii and pierce |
| `src/tracks.rs` | The four selectable routes |
| `src/scenery.rs` | Per-route palettes and decorative prop layout |
| `src/wave.rs` | Wave composition and pacing |
| `src/render.rs` | All drawing — procedural, no image assets |
| `src/audio.rs` | Clip loading, playback, throttling, mute |
| `tools/gen_sounds.py` | Sound effect generator |
| `tools/gen_music.py` | Music loop generator |

## Test

```sh
cargo test
```

## Screenshots

Setting `FRUITSPLAT_SCREENSHOT` stages a scene, renders a few frames, writes a
PNG and exits. Handy for reviewing artwork without playing to the right moment.

```sh
FRUITSPLAT_SCREENSHOT=/tmp/shot.png cargo run --release
```

`FRUITSPLAT_SCREEN` picks what to capture — `panel` opens a tower's stats panel,
`select` the route picker, `menu` the title screen, `victory` and `over` the two
end screens. `FRUITSPLAT_TRACK` picks which route's backdrop to stage. The
default stages a board with all five towers at different levels, one fruit of
every tier including a chilled one, a half-broken Durian showing its armour bar,
and spike piles at varying wear.

## License

MIT — see [LICENSE](LICENSE).
