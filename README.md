# Fruit Splat

A balloon-pop arcade game, reskinned with fruit. Written in Rust with
[macroquad](https://github.com/not-fl3/macroquad).

Fruit float up from the bottom of the screen on a lazy sideways sway. Click to
splat them before they drift off the top. Small fruit climb faster and score
more, so the blueberries are where the points are — and where the misses come
from.

## Play

```sh
cargo run --release
```

- **Left click** — splat the fruit under the cursor
- **Space / click** — start or restart a round
- **Esc** — quit

A round lasts 60 seconds. Spawns tighten and fruit speed up as the clock runs
down.

## Scoring

| Fruit | Radius | Rise speed | Points |
|---|---|---|---|
| Watermelon | 46 px | ×0.72 | 1 |
| Orange | 34 px | ×0.88 | 2 |
| Lime | 28 px | ×1.00 | 3 |
| Strawberry | 26 px | ×1.12 | 4 |
| Blueberry | 18 px | ×1.38 | 6 |

## Layout

| File | Purpose |
|---|---|
| `src/main.rs` | Window config, game state machine, frame loop |
| `src/fruit.rs` | Fruit entities and splat particles |
| `src/spawn.rs` | Spawn pacing and the difficulty ramp |
| `src/render.rs` | All drawing — procedural, no image assets |

## Build

```sh
cargo build --release
```

## License

MIT — see [LICENSE](LICENSE).
