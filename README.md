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
| `1` `2` `3` | Arm a tower type |
| Click shop button | Arm a tower type |
| Left click on field | Place the armed tower |
| Right click | Cancel placement |
| `Space` | Send the next wave |
| `M` | Mute / unmute |

Close the window to quit.

## Towers

| Tower | Cost | Range | Rate | Role |
|---|---|---|---|---|
| Seed Shooter | $90 | 135 px | 0.45s | Single target, cheap sustained damage |
| Blender | $170 | 110 px | 1.10s | 58px splash — the answer to clustered splits |
| Freezer | $140 | 120 px | 1.40s | No damage; chills fruit to 45% speed for 1.6s |

Towers use "first" targeting — they shoot whichever fruit in range is furthest
along the track. Shots travel in a straight line and do not home, so fast fruit
can be missed.

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
