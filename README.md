# Fruit Splat

A Bloons-style tower defence, reskinned with fruit. Written in Rust with
[macroquad](https://github.com/not-fl3/macroquad).

Fruit roll along a winding track toward the exit. You don't touch them directly —
you buy towers and place them along the route. Pop a fruit and it bursts into two
smaller, faster ones, so a watermelon you failed to handle early arrives as a
swarm of blueberries later.

![Fruit Splat on The Long Orchard](docs/screenshot.png)

*Every tower and every fruit tier at once, including the armoured Durian on the
right — staged with the [screenshot mode](#screenshots) rather than caught
mid-wave, so nothing here is hidden behind something else. All of it is drawn
from macroquad primitives; the game ships with no image assets.*

## Play

```sh
cargo run --release
```

| Input | Action |
|---|---|
| Click **Easy** / **Medium** / **Hard** | Set the difficulty |
| `1`–`7` on the route screen | Pick a route (7 is random) and start |
| `1`–`7` in play | Arm a tower type |
| Click shop button | Arm a tower type |
| Left click on field | Place the armed tower, then disarm |
| Left click a placed tower | Open its stats panel (nothing armed) |
| Click **Upgrade** / **Sell** | Upgrade or sell that tower |
| Right click | Cancel placement / close the panel |
| `Space` | Send the next wave now |
| Click **PAUSE** | Hold the wave; building stays live |
| Click **AUTO** | Waves send themselves, three seconds apart |
| Click the speaker button | Mute / unmute sound effects |
| Click the note button | Mute / unmute music |
| Click **QUIT RUN**, then **SURE?** | Abandon the run, back to the title screen |

Close the window to quit.

The title itself is spelled out in fruit: each letter is a 5x7 grid with a berry
on every lit cell, drawn with the same shading the fruit on the track use, and
coloured a tier per letter. The berries are nudged off their grid by a hash of
their position rather than by an RNG — the menu redraws every frame, so a live
random offset would leave the whole wordmark crawling.

The title screen carries the build's version in its bottom right corner. It comes
from `CARGO_PKG_VERSION`, so it is whatever the binary was actually compiled at —
which is the point, since the question it answers is "which build is this?" and a
hand-maintained string would eventually answer it wrongly.

## Difficulty

Before picking a route you pick a **mode**, from a row of three above the route
cards. It carries over between runs, and the HUD shows which one you're on.

| Mode | Starting cash | Starting lives | Fruit speed by the end |
|---|---|---|---|
| Easy | $550 | 36 | +73% |
| Medium | $300 | 20 | +172% |
| Hard | $200 | 10 | +200% |

A mode never changes what a wave **sends**. The fruit, their count, their order,
the boss schedule and every payout are identical on all three — the economy is
one curve, and the modes are pressure applied to it.

The three dials do different jobs at different times:

**Cash is the opening hand, and only the opening hand.** Cumulative income
swamps it within about ten waves, and it has to — the alternative is a mode that
scales income, which compounds and would tear the economy curve apart late.

**Lives are the margin for error.** Leaks cost by tier, so 36 lives absorbs six
leaked Durians while 10 dies to the second leaked watermelon.

**Speed is the only dial that never fades.** It divides how much every tower
gets done on every wave, so a gentler ramp is still being felt on the last one.
`balance_report` prints a table per mode, and the affordability ratio is what
the three dials add up to:

| Wave | 1 | 13 | 25 | 35 | 45 | 50 |
|---|---|---|---|---|---|---|
| Easy | 11.9 | 1.9 | 1.8 | 1.8 | 1.8 | 1.8 |
| Medium | 6.8 | 1.4 | 1.2 | 1.2 | 1.2 | 1.1 |
| Hard | 4.8 | 1.1 | 1.0 | 1.0 | 1.0 | 1.0 |

Below 1.0 means the wave outruns what you can afford, and nothing goes there.
Be careful reading 1.0 as "fair", though: the model spends every dollar on Seed
Shooters, lands every shot and never leaves a tower idle, so a real player at 1.0
is behind. Hard sits on that line for most of a run on purpose. Medium keeps 1.1
under it and Easy 1.8.

The point of those rows is that they stay **flat** from wave 13 to the end. They
used to climb away: past wave 30 the game got steadily easier, because a wave's
demand per second is bounded — the tier ladder tops out at the watermelon and the
spawn interval used to floor at 0.30s — while your bank is not.

Cash and lives alone could not hold that gap open: on those two dials the modes
converge on the same fight by wave 13, and Easy would be merely *rich early*. The
speed ramp is what keeps each mode distinct all the way through — which is also
how Medium and Hard came to feel like one game, since they shared it exactly
until Hard was given a ramp of its own.

## Routes

Each run starts by picking one of six routes. Length is the main difficulty
dial — a longer route means more seconds under fire before a fruit reaches the
exit. Turn count matters too: tight switchbacks let one tower cover several
lanes at once.

| Route | Difficulty | Waves | Length | Character | Backdrop |
|---|---|---|---|---|---|
| Market Run | Hard | 42 | 1700 px | Short and direct | Dusty market of crates and fences |
| Twin Gates | Tricky | 44 | 1775 px avg | **Two entrances**, one exit | Cold rocky highland |
| Orchard Snake | Medium | 46 | 2570 px | A steady weave | Temperate orchard |
| Zigzag Grove | Medium | 46 | 2980 px | Tight lanes, one tower covers two at once | Dark, dense forest |
| The Long Orchard | Gentle | 48 | 3070 px | Plenty of time to shoot | Lush farmland with ponds |
| Meander | Gentle | 50 | 3710 px | The long way round — the gentlest | Bright open meadow |

The cards sit in two rows of four. A seventh, **Surprise Me**, starts a run on
one of the six picked at random.

### Twin Gates, and lanes

Every other route is one lane. **Twin Gates is two**: fruit enter at two gates on
the left, one high and one low, and both streams leave by the same exit on the
right. Each wave is dealt to the gates in turn, so it arrives as two
half-strength streams rather than one you can meet head on.

Neither lane is long — the difficulty is that they are far apart. A single
cluster of towers cannot answer both, so the same money has to cover two
approaches. That makes it a different kind of hard from Market Run, which is why
it gets its own label rather than sitting on the length scale with the others.

Lanes are independent for their whole length, even where they converge on
screen. A fruit belongs to one lane and never changes; a spike pile belongs to
one lane and never touches the other's fruit, however close the two pass. The
two lanes meet at the exit *point* rather than sharing a stretch of track before
it, precisely so that rule never becomes visible as a Spike Layer that pops only
half the fruit crossing it.

Two things had to change to make lanes work at all:

- **Targeting ranks fruit by fraction of their lane walked, not raw distance.**
  The lanes are different lengths, so 900px along the short one is nearer its
  exit — and so more urgent — than 900px along the long one. Comparing raw
  distances would have every tower covering both gates quietly favour whichever
  lane happened to be longer.
- **Tower placement must clear every lane**, and so must scenery.

**PAUSE** holds the wave. Input keeps running while it does — towers can be
bought, upgraded and sold with everything standing still, which is the point of
pausing rather than a side effect. Nothing else advances: not the spawn clock,
not cooldowns, not the auto-send timer. The overlay dims the field only, leaving
the shop column lit so its buttons still read as live.

You can walk away from a run at any point with the **QUIT RUN** button at the
foot of the shop column. It asks before it acts: the first click turns it into
**SURE?** and the second confirms, and it stands itself down after three seconds
or on a right click. It shares that block with the buttons you press most, so one
stray click throwing away a twenty-five wave run would be unforgivable — the
button does the asking rather than a dialog, so nothing has to interrupt the wave
to ask it. Quitting sweeps the board and drops the music back to the menu loop.

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

### The field

The window is a fixed **1420 × 740**: 1200 × 740 of playfield, with a 220px shop
column down the right.

The shop used to be a bar along the bottom, which ran out of window at five
buttons. A column has room for a good many more, and the audio toggles, pause,
auto and quit moved into a block at its foot rather than competing with the wave
counter for space in the top strip. That block is anchored to the bottom of the
window rather than flowing after the towers, so adding a tower grows the column
downward into empty space instead of pushing the controls off screen.

Losing the bottom bar gave the field the full window height, 650 to 740, so every
route moved down 30px to sit centred in it — shapes and lengths untouched. Routes are authored in that space and deliberately overhang it at both
ends, by 40px, so fruit walk on and off the screen rather than blinking into
existence at the border.

The exit marker is drawn **clamped back inside the window** rather than at the
route's final waypoint. Because routes end off-screen, drawing it at the endpoint
put the one thing you are defending permanently out of sight — on every route.
Clamped, it lands where the track crosses out of the field, which is the honest
answer anyway: that is the last place a fruit can still be shot. On Twin Gates
the shared endpoint clamps to a point between the two converging lanes, right
where they meet.

## Towers

| Tower | Cost | Range | Rate | Role |
|---|---|---|---|---|
| Seed Shooter | $90 | 135 px | 0.45s | Single target, cheap sustained damage |
| Triple Seeder | $260 | 150 px | 0.45s | Three seeds every volley, spread over up to three fruit |
| Blender | $170 | 110 px | 1.10s | 58px splash — the answer to clustered splits |
| Knife Thrower | $130 | 145 px | 0.75s | Knives pierce 3 fruit and keep flying |
| Spike Layer | $150 | 120 px | 3.60s | Lays spike piles onto the track all wave long |
| Bomb Lobber | $320 | 125 px | 2.60s | A 110px blast that clears a whole cluster |
| Freezer | $140 | 120 px | 1.40s | No damage; chills fruit to 45% speed for 1.6s |

The Blender and the Knife Thrower both beat crowds, but differently: splash hits
a **blob** of fruit around one point, while pierce hits a **line** of them along
the shot's path. That makes the Knife Thrower strongest on the switchback
routes, where fruit queue up single file down a lane.

The **Bomb Lobber** is the third answer, and the most absolute: a slow shell that
takes everything standing in its blast off the track at once. It is the dearest
tower in the shop and the slowest to fire, which is what it pays for that with —
between shells, whatever it did not catch keeps walking, so it cannot hold a lane
by itself. Its blast is roughly double the Blender's and wider than half its own
range, so it is placed by guessing where the crowd will *be* rather than by what
it can see.

It is also the only tower that does not use "first" targeting. Everything else
duels whatever is nearest its exit, which is right when a shot pops one fruit; a
shell aimed that way lands on the straggler that has already outrun the pack —
the worst shot on the board. The Bomb Lobber aims at whichever fruit has the most
neighbours inside its blast instead, falling back on threat when two crowds tie
and on the leader when nothing is bunched up at all. Widening the blast can
therefore change its mind about where to aim, which is what its upgrades buy
beyond a bigger number.

The Spike Layer is the odd one out — it doesn't shoot. It lays piles of spikes
directly onto the track, and each pile is worth **one hit per spike** before it
wears away. Against everything except the boss a hit is a pop, so that reads as
one fruit per spike. It's especially good against splits: children spawn where
their parent died, right on top of the same pile.

It lays on a timer — one pile every 3.60s at Lv1, down to 2.20s at Lv3 — and it
keeps laying for the **whole wave**, from the moment the wave is sent until the
last fruit is off the track. Each pile lands at a random free spot on the track
it covers, so coverage builds up scattered rather than filing in from one end. On
Twin Gates, a tower reaching both lanes feeds each in proportion to how much of it
it actually covers.

Spikes are a **within-wave** resource. Nothing is laid on the build screen between
waves — the timer runs there, so the first pile goes down the instant the wave
starts — and whatever is still standing when the field clears is swept away rather
than carried over. Every wave begins on bare track, so what a Spike Layer is worth
is what it can build during the wave.

Spikes never miss, so what limits the tower is how fast it lays, how big its piles
are, and how much track it covers. A fruit costs exactly one spike however many
piles it happens to be standing on. Piles sit 34px apart and each reaches
`radius + 14px` along the track, so overlap is the norm — a watermelon is wide
enough to sit on three at once. That 34px spacing is also the ceiling: once the
track a tower covers is packed that densely it has nowhere left to lay, and waits
for the fruit to chew a gap open.

A pile is tested against fruit by **lane, and distance along that lane** — never
by how close it is in pixels. Two stretches of track can run within a few dozen
pixels of each other, either as a switchback on one lane or as the two lanes of
Twin Gates converging, and a pile must only touch the fruit actually walking
over it.

Placing a tower **disarms the shop selection**. While a type is armed a field
click always places, so a placed tower can't be clicked to open its panel and a
stray click buys another one. Re-arming is one click on the shop button or one
number key, which is cheaper than not being able to touch the field until you
cancel. Right click still cancels an armed tower you decide against.

Towers use "first" targeting — they shoot whichever fruit in range is furthest
along its lane, measured as a **fraction** of that lane rather than in pixels, so
that lanes of different lengths compare fairly. Shots don't home, but towers
**lead** their targets: they aim at where the fruit will be once the shot
arrives, which is what keeps them useful against the late-wave speed ramp.

The **Triple Seeder** is the dearest thing in the shop, and deliberately so. It
engages three separate fruit at once — four fully upgraded — which is answering a
crowd on its own, and the Blender and the Knife Thrower each do half of that.
Being able to buy it early would flatten the reason those two differ.

It fires on the **same cadence as a Seed Shooter**, so a volley is simply three
of that tower's shots where it puts one — 6.7 shots a second against 2.2, growing
to five seeds a volley fully upgraded.

A volley always fires every shot. With fewer fruit in range than shots, the extra
seeds come back round to the leading fruit rather than going unfired — so a
multi-shot tower is still worth its price against a lone target, which is exactly
the case a boss presents.

### Stats and upgrades

Click a placed tower with nothing armed to open its panel. The panel floats
beside the tower — flipping to its other side near the window edge — and reports
what that specific tower has actually done:

| Tower | Stats shown |
|---|---|
| Seed Shooter | Range, rate, shots fired, kills |
| Blender | Range, splash, shots fired, kills |
| Knife Thrower | Range, pierce, knives thrown, kills |
| Spike Layer | Spikes per pile, seconds between piles, piles laid, kills |
| Bomb Lobber | Blast radius, seconds between shells, shells fired, kills |
| Freezer | Range, chill strength, pulses, fruit chilled |

Kills are credited to the tower whose shot landed, so a Blender's splash banks
every fruit in the blast. A Freezer never gets kills — it deals no damage — so it
reports what it has slowed instead.

Upgrade and sell are buttons in that panel. Every tower runs Lv1 → Lv3 along one
track, and selling refunds **60%** of everything invested, upgrades included.

| Tower | Lv2 | Lv3 |
|---|---|---|
| Seed Shooter | $70 — faster, longer reach | $150 — twin shot |
| Triple Seeder | $160 — a fourth seed, faster | $280 — a fifth seed, faster |
| Blender | $120 — splash 58 → 72 px | $240 — splash 90 px, faster |
| Knife Thrower | $110 — pierce 3 → 4, faster | $220 — pierce 6 |
| Spike Layer | $130 — 6 spikes/pile, lays every 2.80s | $260 — 9 spikes/pile, every 2.20s |
| Bomb Lobber | $200 — blast 110 → 135 px, faster | $340 — blast 165 px, faster |
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
- **Bomb Lobbers** are no better against it than anything else — a shell is one
  hit, however wide the blast — but the boss travels inside a swarm, and one
  shell can take most of that escort off the track while the Knife Throwers work
  on the husk.
- **Freezers** still work on it, and buying the field a few extra seconds of
  fire is worth more here than anywhere else in the game.

## Waves

A route runs **42 to 50 waves** — about twenty minutes, which is as long as one
sitting should be. A new tier unlocks every third wave, so watermelons first
appear on wave 13. Fruit get **3.5% faster each wave** on Medium, reaching 2.72×
by wave 50; Easy ramps at 1.5% to 1.73×, Hard at 5% and hits its 3.0× ceiling at
wave 41. What you start with is set by the difficulty mode too.

Spawn intervals tighten on **two slopes**: quickly from 0.85s to 0.30s by wave 28,
then gently on toward a 0.16s floor. The second slope is what makes a
long run work. A wave's pressure is hits per second, and once the interval stops
falling that is fixed by the fruit mix alone — so every extra fruit past wave 28
made a wave *longer* rather than harder while its income compounded, and the run
drifted toward the player. Arriving faster costs you nothing in cash and
everything in the time each tower has to work.

For the same reason the lower tiers stop growing. Padding a late wave with more
blueberries adds length and income but no pressure, so the chaff thins out as the
run goes on and the top of the ladder takes over.

A freshly unlocked tier arrives in **small numbers** while the tiers below it
bulk the wave out. That is what keeps the ladder from being a cliff: the debut
count used to scale with the wave number, which meant the later a tier arrived
the bigger its first appearance — and since the toughest tier arrives last, wave
13 opened with nine watermelons at 31 hits each. It was comfortably the hardest
wave in the game and the point most runs ended.

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

### Sending waves

By default each wave waits for `Space`. The **AUTO** toggle in the top strip
hands that over: once it's lit, the next wave goes out three seconds after the
field clears, and the between-waves prompt counts down instead of asking. The
setting persists across runs, and `Space` still overrides it — pressing it skips
whatever is left of the gap.

The gap is deliberately not zero. Between waves is when towers get bought,
placed and upgraded, and that is most of the game's decision-making — sending
instantly would take it away rather than automate it. Three seconds is enough to
place a tower or buy an upgrade without turning the gap into dead time.

Auto never sends past the end of a run: clearing a route's last wave settles
into the victory screen rather than counting down to a wave that doesn't exist.

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

Prints one table per difficulty mode, each showing the economy curve per
wave: fruit sent, bosses among
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

### Muting

Two buttons sit in the top strip: a **speaker** for sound effects and a **note**
for music. They silence their halves independently, because someone playing with
the game in the background usually wants one or the other gone rather than both.
A muted button greys out and takes a red slash, and the speaker drops its sound
arcs — the conventional cue for silence, which a note has no equivalent of, so
the note keeps its shape and leans on the slash.

Both are drawn on every screen, not just during play: the menu and the end
screens have music too, and a mute you can only reach mid-run is a mute you
reach too late. They take a click before any screen sees it, so muting from the
title screen doesn't also start a run.

Unmuted music restarts from the top rather than resuming — macroquad gives no
way to seek a playing sound.

## Layout

| File | Purpose |
|---|---|
| `src/main.rs` | Window config, game state, frame loop, economy |
| `src/path.rs` | Track polyline, distance lookup, placement clearance |
| `src/fruit.rs` | Fruit tiers, the split ladder, splat particles |
| `src/tower.rs` | Tower stats, spike piles, the Freezer pulse effect |
| `src/projectile.rs` | Shots in flight, their splash radii and pierce |
| `src/tracks.rs` | The five selectable routes and their lanes |
| `src/mode.rs` | The three difficulty modes |
| `src/scenery.rs` | Per-route palettes and decorative prop layout |
| `src/wave.rs` | Wave composition and pacing |
| `src/render.rs` | All drawing — procedural, no image assets |
| `src/audio.rs` | Clip loading, playback, throttling, mute |
| `tools/gen_sounds.py` | Sound effect generator |
| `tools/gen_music.py` | Music loop generator |

## Builds

[`.github/workflows/build.yml`](.github/workflows/build.yml) builds Linux, macOS,
Windows and web **for a `v*` tag**, and attaches all four to a GitHub release.
The suite has to pass first — binaries from a tree that fails its own tests are
worse than no binaries.

**A branch push runs nothing.** GitHub starts a run per *ref* rather than per
commit, so a trigger on `main` meant every tagged release showed up as two runs —
one for `refs/heads/main`, one for `refs/tags/v*`. With the branch trigger gone,
pushing a commit and then tagging it produces exactly one run.

The trade-off is real: nothing is checked until you tag. The checks still gate
the release from inside the tag run, so a broken tree fails the release rather
than shipping a bad one — but you find out at tag time, not at push time. Run
`cargo test` locally, or use `workflow_dispatch`, which runs the checks and
builds without spending a version number.

A release's notes are the matching `## [x.y.z]` section lifted out of
[`CHANGELOG.md`](CHANGELOG.md) — not GitHub's generated list of commit subjects,
and not a link to the file. A tag whose version has no section there fails the
release rather than publishing an empty body.

| Target | Produced |
|---|---|
| Linux | `fruit-splat-linux-x86_64.tar.gz` |
| macOS | `fruit-splat-macos-universal.tar.gz` — Intel and Apple Silicon in one binary |
| Windows | `fruit-splat-windows-x86_64.zip` |
| Web | `fruit-splat-web.zip` |

Because the game embeds its assets with `include_bytes!`, each desktop archive is
just the executable — there's no asset directory to install beside it.

### Building for the web yourself

```sh
cargo build --release --target wasm32-unknown-unknown
```

A macroquad wasm build needs three files served together: the `.wasm`,
macroquad's JS bundle, and a page that calls `load()` on the module. The page is
[`web/index.html`](web/index.html); the bundle ships inside the macroquad crate.

```sh
mkdir -p dist
cp web/index.html dist/
cp target/wasm32-unknown-unknown/release/fruit-splat.wasm dist/
cp ~/.cargo/registry/src/*/macroquad-0.4.16/js/mq_js_bundle.js dist/
python3 -m http.server -d dist 8080
```

It has to be served over HTTP — opening `index.html` from the filesystem fails,
because browsers won't instantiate wasm from a `file://` origin.

[`.cargo/config.toml`](.cargo/config.toml) passes `--allow-undefined` to the
linker for this target. quad-snd reaches the Web Audio API through `extern "C"`
declarations that nothing in the Rust build defines — the JS bundle supplies them
at runtime — so on wasm they have to link as *imports*. Whether wasm-ld does that
unprompted depends on the toolchain, and a runner that doesn't fails the link with
`undefined symbol: audio_init`.

Careful with `RUSTFLAGS`: setting it in the environment **replaces** the flags in
that file rather than adding to them, which silently drops `--allow-undefined` and
breaks the web build. That is why the workflow sets no global `RUSTFLAGS` and
passes `-D warnings` to clippy directly.

The page sizes the canvas with CSS **width and height**, never with a
`transform`. macroquad's JS bundle is why the distinction matters — it uses two
different measurements:

```js
resize():                  canvas.clientWidth * dpi_scale()            // ignores CSS transforms
mouse_relative_position():  (clientX - getBoundingClientRect().left)   // does not
```

A `transform` makes those disagree by exactly the scale factor: the game renders
full size while every click arrives compressed toward the top-left, and below a
scale of 0.88 the shop is out of reach so no tower can be armed at all. Width and
height keep them in agreement, because both then read the same box.

What that costs is a drawing buffer that follows the element, so `screen_width()`
is whatever the device gave us rather than the 1420 the layout is authored
against. `render.rs` answers it with a **view**: everything is drawn in a fixed
1420 × 740 space through a camera fitted to the real surface, centred, with bars
where the aspect ratios differ, and every pointer position is converted back
through the same fit before anything is hit-tested. Rendering and input therefore
scale together by construction. On a desktop window, fixed at exactly that size,
the scale is 1 and the offset zero.

The two halves have to stay together: the page may resize the canvas freely, and
nothing may transform it. A test asserts the stylesheet contains no `transform:`,
and another that the page is still built around the view's dimensions.

Phones get the game scaled to fit their width. Held upright that is a strip about
200px tall — it renders, but it is not playable — so the page covers it with a
**turn your phone sideways** prompt below 820px in portrait. The prompt overlays
the canvas rather than hiding it: `display: none` would take `clientWidth` to
zero and macroquad would resize the buffer to nothing, so turning the phone back
would restore a canvas the game no longer fits.

The `.wasm` is about 5.7 MB, of which 5.1 MB is the embedded audio — the WAVs are
uncompressed PCM. Stripping barely dents it; shrinking it means shorter loops or
a lower sample rate.

## Test

```sh
cargo test
```

## Screenshots

Setting `FRUITSPLAT_SCREENSHOT` stages a scene, renders a few frames, writes a
PNG and exits. Handy for reviewing artwork without playing to the right moment —
the image at the top of this file was produced with it.

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
