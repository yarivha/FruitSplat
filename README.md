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
| `1` `2` `3` in play | Arm a tower type |
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

| Route | Difficulty | Length | Character |
|---|---|---|---|
| Market Run | Hard | 1500 px | Short and direct |
| Orchard Snake | Medium | 2370 px | A steady weave |
| Zigzag Grove | Medium | 2780 px | Tight lanes, one tower covers two |
| The Long Orchard | Gentle | 2870 px | Plenty of time to shoot |

## Towers

| Tower | Cost | Range | Rate | Role |
|---|---|---|---|---|
| Seed Shooter | $90 | 135 px | 0.45s | Single target, cheap sustained damage |
| Blender | $170 | 110 px | 1.10s | 58px splash — the answer to clustered splits |
| Freezer | $140 | 120 px | 1.40s | No damage; chills fruit to 45% speed for 1.6s |

Towers use "first" targeting — they shoot whichever fruit in range is furthest
along the track. Shots travel in a straight line and do not home, so fast fruit
can be missed.

### Stats and upgrades

Click a placed tower with nothing armed to open its panel. The panel floats
beside the tower — flipping to its other side near the window edge — and reports
what that specific tower has actually done:

| Tower | Stats shown |
|---|---|
| Seed Shooter | Range, rate, shots fired, kills |
| Blender | Range, splash, shots fired, kills |
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
| Freezer | $100 — chill 45% → 35% | $200 — chill 25%, lasts 2.3s |

Overlapping Freezers stack by keeping the strongest chill and the longest
remaining duration.

## Fruit

Popping a fruit splits it into **two** of the next tier down. Clearing a single
watermelon therefore takes 31 pops in total.

| Tier | Fruit | Radius | Speed | Splits into | Lives lost if leaked |
|---|---|---|---|---|---|
| 4 | Watermelon | 30 px | ×0.72 | 2 × Orange | 5 |
| 3 | Orange | 24 px | ×0.88 | 2 × Lime | 4 |
| 2 | Lime | 19 px | ×1.00 | 2 × Strawberry | 3 |
| 1 | Strawberry | 15 px | ×1.12 | 2 × Blueberry | 2 |
| 0 | Blueberry | 11 px | ×1.38 | — | 1 |

## Waves

A new tier unlocks every third wave. Spawn intervals tighten from 0.85s toward a
0.30s floor, and clearing a wave pays a bonus on top of the $1 earned per pop.
You start with 20 lives and $250.

## Audio

All music and sound effects are **generated procedurally** by pure-standard-library
Python scripts — no samples, no numpy, no external audio tools. The resulting WAVs
are embedded into the binary with `include_bytes!`, so the executable is
standalone.

```sh
python3 tools/gen_sounds.py   # 14 effects → assets/
python3 tools/gen_music.py    # 2 music loops → assets/
```

Both scripts use a fixed random seed, so regenerating produces byte-identical
files. The committed `assets/*.wav` are a build-time input only — nothing reads
them at runtime.

Fruit pop at five different pitches, highest for blueberries down to lowest for
watermelons. To keep a busy field readable, pops are capped at 3 per frame with
each successive one 22% quieter, and the shoot sound is limited to one per 55ms
across every tower.

## Layout

| File | Purpose |
|---|---|
| `src/main.rs` | Window config, game state, frame loop, economy |
| `src/path.rs` | Track polyline, distance lookup, placement clearance |
| `src/fruit.rs` | Fruit tiers, the split ladder, splat particles |
| `src/tower.rs` | Tower stats and the Freezer pulse effect |
| `src/projectile.rs` | Shots in flight and their splash radii |
| `src/tracks.rs` | The four selectable routes |
| `src/wave.rs` | Wave composition and pacing |
| `src/render.rs` | All drawing — procedural, no image assets |
| `src/audio.rs` | Clip loading, playback, throttling, mute |
| `tools/gen_sounds.py` | Sound effect generator |
| `tools/gen_music.py` | Music loop generator |

## Test

```sh
cargo test
```

## License

MIT — see [LICENSE](LICENSE).
