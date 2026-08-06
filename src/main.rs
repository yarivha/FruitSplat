// =============================================================================
// main.rs — Fruit Splat entry point, game state and frame loop
//
// A tower defence: fruit walk a fixed track and the player buys towers to pop
// them before they reach the exit. This file owns the world (track, fruit,
// towers, shots, economy) and the per-frame order of operations. Entity data
// lives in `fruit`/`tower`/`projectile`, track maths in `path`, wave
// composition in `wave`, and every pixel in `render`.
// =============================================================================

use macroquad::prelude::*;
use macroquad::rand::gen_range;

mod audio;
mod fruit;
mod path;
mod projectile;
mod render;
mod scenery;
mod tower;
mod tracks;
mod wave;

use audio::{Audio, Track};
use fruit::{Fruit, FruitKind, Splat};
use path::Path;
use projectile::{Projectile, ProjectileKind};
use tower::{
    Pulse, SpikePile, Tower, TowerKind, PATH_CLEARANCE, SPIKE_SPACING, TOWER_RADIUS,
};

/// Height of the playable field; the shop bar occupies the strip below it.
pub const PLAYFIELD_H: f32 = 650.0;
/// Width of the playable field. Routes and scenery are authored against this
/// fixed space rather than the live window size, so layout is reproducible
/// without a graphics context.
pub const PLAYFIELD_W: f32 = 1000.0;

const WINDOW_W: i32 = PLAYFIELD_W as i32;
const WINDOW_H: i32 = 740;
const START_LIVES: u32 = 15;
const START_CASH: u32 = 180;
/// Cash earned for each fruit destroyed outright — that is, one that had no
/// children left to split into.
///
/// Paying per *pop* meant a watermelon was worth $31, so income scaled with the
/// threat while towers stayed a one-time cost, and the surplus compounded until
/// the player could afford six times the firepower a wave needed. Paying only
/// for the bottom of the split ladder roughly halves late-game income while
/// still rewarding bigger fruit, which are worth 16 blueberries apiece.
const CASH_PER_FRUIT_CLEARED: u32 = 1;

// ─────────────────────────────────────────────────────────────────────────────
// Window configuration, consumed by the macroquad::main attribute.
// ─────────────────────────────────────────────────────────────────────────────
fn window_conf() -> Conf {
    Conf {
        window_title: "Fruit Splat".to_owned(),
        window_width: WINDOW_W,
        window_height: WINDOW_H,
        high_dpi: true,
        // Routes, scenery and the shop bar are authored against a fixed
        // PLAYFIELD_W x PLAYFIELD_H space, but the HUD and hit-testing read the
        // live window size. Resizing pulls those two apart: narrow the window
        // and the shop buttons keep taking clicks from where they are no longer
        // drawn, shorten it and the whole bar falls off the bottom, since
        // PLAYFIELD_H never gives the strip back.
        window_resizable: false,
        ..Default::default()
    }
}

/// Which screen the game is currently on.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Menu,
    /// Picking which route to defend, before a run starts.
    TrackSelect,
    Playing,
    GameOver,
    /// Every wave on the route survived.
    Victory,
}

/// The whole game world.
struct Game {
    state: State,
    path: Path,
    fruits: Vec<Fruit>,
    towers: Vec<Tower>,
    projectiles: Vec<Projectile>,
    splats: Vec<Splat>,
    pulses: Vec<Pulse>,
    /// Spike piles sitting on the track, dropped by Spike Layers.
    spikes: Vec<SpikePile>,
    /// Fruit still waiting to enter the track this wave.
    queue: Vec<FruitKind>,
    spawn_timer: f32,
    wave: u32,
    wave_active: bool,
    lives: u32,
    cash: u32,
    /// Tower type armed for placement, if any.
    selected: Option<TowerKind>,
    /// Index into `towers` of the placed tower being inspected, if any.
    selected_tower: Option<usize>,
    /// Which entry of tracks::TRACKS the current run is being played on.
    track: usize,
    /// Backdrop for the current route: colours plus decorative props. Laid out
    /// once per run, since placement is rejection sampling and not cheap.
    palette: scenery::Palette,
    props: Vec<scenery::Prop>,
    /// Handed out to each placed tower so projectiles can credit kills back to
    /// a specific tower even after others are sold.
    next_tower_id: u32,
    audio: Audio,
}

impl Game {
    // ─────────────────────────────────────────────────────────────────────────
    // Build the game sitting on the title screen. Audio is loaded by the caller
    // because decoding the embedded clips is async.
    // ─────────────────────────────────────────────────────────────────────────
    fn new(audio: Audio) -> Self {
        Game {
            state: State::Menu,
            // Placeholder until a route is chosen; the menu draws it as a
            // backdrop and start_run replaces it.
            path: tracks::TRACKS[0].path(),
            fruits: Vec::new(),
            towers: Vec::new(),
            projectiles: Vec::new(),
            splats: Vec::new(),
            pulses: Vec::new(),
            spikes: Vec::new(),
            queue: Vec::new(),
            spawn_timer: 0.0,
            wave: 1,
            wave_active: false,
            lives: START_LIVES,
            cash: START_CASH,
            selected: None,
            selected_tower: None,
            track: 0,
            palette: scenery::palette(0),
            props: Vec::new(),
            next_tower_id: 0,
            audio,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clear the board and start from wave 1 on the chosen route.
    // ─────────────────────────────────────────────────────────────────────────
    fn start_run(&mut self, track: usize) {
        self.track = track.min(tracks::TRACKS.len() - 1);
        self.path = tracks::TRACKS[self.track].path();
        self.palette = scenery::palette(self.track);
        self.props = scenery::generate(self.track, &self.path);

        self.fruits.clear();
        self.towers.clear();
        self.projectiles.clear();
        self.splats.clear();
        self.pulses.clear();
        self.spikes.clear();
        self.queue.clear();
        self.spawn_timer = 0.0;
        self.wave = 1;
        self.wave_active = false;
        self.lives = START_LIVES;
        self.cash = START_CASH;
        self.selected = None;
        self.selected_tower = None;
        self.next_tower_id = 0;
        self.state = State::Playing;
        self.audio.play_music(Track::Game);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Advance the whole game by `dt` seconds.
    // ─────────────────────────────────────────────────────────────────────────
    fn update(&mut self, dt: f32) {
        self.audio.begin_frame(dt);

        // Mute works on every screen, not just during play.
        if is_key_pressed(KeyCode::M) {
            self.audio.toggle_mute();
        }

        match self.state {
            // These all lead into route selection rather than straight into a
            // run, so a fresh route can be picked after finishing one.
            State::Menu | State::GameOver | State::Victory => {
                if is_mouse_button_pressed(MouseButton::Left) || is_key_pressed(KeyCode::Space) {
                    self.state = State::TrackSelect;
                }
            }
            State::TrackSelect => self.update_track_select(),
            State::Playing => self.update_play(dt),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Route selection: click a card or press its number to start that run.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_track_select(&mut self) {
        for i in 0..tracks::TRACKS.len() {
            let picked_by_key = match i {
                0 => is_key_pressed(KeyCode::Key1),
                1 => is_key_pressed(KeyCode::Key2),
                2 => is_key_pressed(KeyCode::Key3),
                3 => is_key_pressed(KeyCode::Key4),
                _ => false,
            };
            let picked_by_click = is_mouse_button_pressed(MouseButton::Left)
                && render::track_card_rect(i).contains(mouse_vec());

            if picked_by_key || picked_by_click {
                self.start_run(i);
                return;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // One frame of active play. Order matters: fruit move before towers aim, so
    // towers always fire at where fruit actually are this frame.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_play(&mut self, dt: f32) {
        self.handle_input();

        if self.wave_active {
            self.spawn_from_queue(dt);
        }

        for f in &mut self.fruits {
            f.update(dt, &self.path);
        }
        self.handle_leaks();

        self.update_towers(dt);
        self.update_projectiles(dt);
        self.update_spikes();
        self.update_effects(dt);

        // Death is settled before wave completion, because one leak can trigger
        // both: the fruit that drains the last life is often also the last one
        // on the field. Checked the other way round, the run would bank the
        // clear bonus and tick the wave counter on its way to the game over
        // screen, which then reports a wave the player never actually reached —
        // and on a route's final wave it would fire the victory jingle a frame
        // before losing.
        if self.lives == 0 {
            self.state = State::GameOver;
            self.audio.play_game_over();
            self.audio.play_music(Track::Menu);
            return;
        }

        self.check_wave_complete();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Hotkeys, tower selection, placement, and sending the next wave.
    // ─────────────────────────────────────────────────────────────────────────
    fn handle_input(&mut self) {
        for (i, key) in [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
        ]
        .iter()
        .enumerate()
        {
            if is_key_pressed(*key) {
                self.toggle_selection(TowerKind::ALL[i]);
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            self.selected = None;
            self.selected_tower = None;
        }

        if is_key_pressed(KeyCode::Space) && !self.wave_active {
            self.start_wave();
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let m = mouse_vec();
            if m.y >= PLAYFIELD_H {
                self.click_shop(m);
            } else if !self.click_tower_panel(m) {
                // Only treat it as a field click if the panel didn't take it.
                self.click_field(m);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Route a click at the open tower panel. Returns true when the panel
    // consumed it, so a click on the panel never also places or deselects.
    // ─────────────────────────────────────────────────────────────────────────
    fn click_tower_panel(&mut self, m: Vec2) -> bool {
        let Some(t) = self.inspected_tower() else {
            return false;
        };

        let panel = render::tower_panel_rect(t.pos);
        if !panel.contains(m) {
            return false;
        }

        if render::panel_upgrade_button(panel).contains(m) {
            self.upgrade_selected();
        } else if render::panel_sell_button(panel).contains(m) {
            self.sell_selected();
        }
        true
    }

    // ─────────────────────────────────────────────────────────────────────────
    // A click on the playfield either places the armed tower, or — when nothing
    // is armed — picks a placed tower to inspect, upgrade or sell.
    // ─────────────────────────────────────────────────────────────────────────
    fn click_field(&mut self, p: Vec2) {
        if self.selected.is_some() {
            self.try_place(p);
            return;
        }

        // Clicking empty ground clears the selection, which position() gives
        // for free by returning None.
        self.selected_tower = self
            .towers
            .iter()
            .position(|t| t.pos.distance(p) <= TOWER_RADIUS + 4.0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Buy the next level for the inspected tower.
    // ─────────────────────────────────────────────────────────────────────────
    fn upgrade_selected(&mut self) {
        let Some(i) = self.selected_tower else { return };
        let Some(cost) = self.towers[i].upgrade_cost() else {
            // Already maxed out.
            self.audio.play_deny();
            return;
        };

        if self.cash < cost {
            self.audio.play_deny();
            return;
        }

        self.cash -= cost;
        self.towers[i].upgrade(cost);
        self.audio.play_upgrade();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sell the inspected tower back for a fraction of what went into it.
    // ─────────────────────────────────────────────────────────────────────────
    fn sell_selected(&mut self) {
        let Some(i) = self.selected_tower else { return };

        // A sold tower takes its spikes with it, which keeps the pile count
        // bounded — otherwise repeatedly building and selling Spike Layers
        // would litter the track with orphans nothing ever cleans up.
        let id = self.towers[i].id;
        self.spikes.retain(|s| s.owner != id);

        self.cash += self.towers[i].sell_value();
        self.towers.remove(i);
        // Every index past the removed one has shifted, so drop the selection
        // rather than try to fix it up.
        self.selected_tower = None;
        self.audio.play_sell();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Arm a tower type, or disarm it if it was already armed.
    // ─────────────────────────────────────────────────────────────────────────
    fn toggle_selection(&mut self, kind: TowerKind) {
        self.selected = if self.selected == Some(kind) {
            None
        } else {
            Some(kind)
        };
        // Arming a tower type takes over the panel, so drop any inspected tower.
        self.selected_tower = None;
    }

    fn click_shop(&mut self, m: Vec2) {
        for (i, kind) in TowerKind::ALL.iter().enumerate() {
            if render::shop_button_rect(i).contains(m) {
                self.toggle_selection(*kind);
                return;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Place the armed tower if the spot is legal. Selection is kept so several
    // towers of one type can be placed in a row.
    // ─────────────────────────────────────────────────────────────────────────
    fn try_place(&mut self, p: Vec2) {
        let Some(kind) = self.selected else { return };
        if !self.placement_valid(p, kind) {
            self.audio.play_deny();
            return;
        }

        self.cash -= kind.cost();
        self.towers.push(Tower::new(kind, p, self.next_tower_id));
        self.next_tower_id += 1;
        self.audio.play_place();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Can a tower of `kind` go at `p`? Must be affordable, inside the field,
    // clear of the track, and not overlapping an existing tower.
    // ─────────────────────────────────────────────────────────────────────────
    fn placement_valid(&self, p: Vec2, kind: TowerKind) -> bool {
        if self.cash < kind.cost() {
            return false;
        }
        if p.x < TOWER_RADIUS
            || p.x > screen_width() - TOWER_RADIUS
            || p.y < TOWER_RADIUS
            || p.y > PLAYFIELD_H - TOWER_RADIUS
        {
            return false;
        }
        if self.path.distance_to(p) < PATH_CLEARANCE {
            return false;
        }
        if self
            .towers
            .iter()
            .any(|t| t.pos.distance(p) < TOWER_RADIUS * 2.0)
        {
            return false;
        }
        true
    }

    fn start_wave(&mut self) {
        self.queue = wave::build_wave(self.wave);
        self.spawn_timer = 0.0;
        self.wave_active = true;
        self.audio.play_wave_start();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Release queued fruit onto the start of the track on the spawn timer.
    // ─────────────────────────────────────────────────────────────────────────
    fn spawn_from_queue(&mut self, dt: f32) {
        if self.queue.is_empty() {
            return;
        }

        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            if let Some(kind) = self.queue.pop() {
                // Fruit carry the wave's speed ramp, and pass it to their
                // children when they split.
                self.fruits.push(Fruit::new(
                    kind,
                    0.0,
                    &self.path,
                    wave::speed_multiplier(self.wave),
                ));
            }
            self.spawn_timer = wave::spawn_interval(self.wave);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Remove fruit that reached the exit and charge their leak cost in lives.
    // ─────────────────────────────────────────────────────────────────────────
    fn handle_leaks(&mut self) {
        let leaked: Vec<usize> = self
            .fruits
            .iter()
            .enumerate()
            .filter(|(_, f)| f.reached_end(&self.path))
            .map(|(i, _)| i)
            .collect();

        for &i in leaked.iter().rev() {
            let f = self.fruits.remove(i);
            self.lives = self.lives.saturating_sub(f.kind.leak_cost());
        }

        // One warning per frame, however many got through together.
        if !leaked.is_empty() {
            self.audio.play_leak();
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tick every tower: Freezers chill everything in range, the others pick the
    // fruit furthest along the track and shoot at it.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_towers(&mut self, dt: f32) {
        for t in &mut self.towers {
            t.cooldown -= dt;
            if t.cooldown > 0.0 {
                continue;
            }

            let range = t.range();

            if t.kind == TowerKind::SpikeLayer {
                // Hold off once this tower's allowance of piles is already out
                // on the track; the cooldown isn't spent, so it drops again as
                // soon as one is chewed through.
                let live = self.spikes.iter().filter(|s| s.owner == t.id).count() as u32;
                if live >= t.max_piles() {
                    continue;
                }

                if let Some((dist, pos)) =
                    pick_spike_spot(&self.path, t.pos, range, &self.spikes)
                {
                    self.spikes.push(SpikePile::new(
                        pos,
                        dist,
                        t.spike_charges(),
                        t.id,
                        gen_range(0.0, 360.0),
                    ));
                    t.cooldown = t.fire_cooldown();
                    t.shots_fired += 1;
                    self.audio.play_spikes();
                }
                continue;
            }

            if t.kind == TowerKind::Freezer {
                let (factor, duration) = (t.slow_factor(), t.freeze_duration());
                let mut chilled = 0;
                for f in &mut self.fruits {
                    if f.pos.distance(t.pos) <= range {
                        f.chill(factor, duration);
                        chilled += 1;
                    }
                }
                // Only spend the cooldown when there was something to chill.
                if chilled > 0 {
                    t.cooldown = t.fire_cooldown();
                    t.shots_fired += 1;
                    t.chills += chilled;
                    self.pulses.push(Pulse::new(t.pos, range));
                    self.audio.play_freeze();
                }
                continue;
            }

            // "First" targeting: the fruit closest to the exit is the threat.
            // Collect owned copies so the fruit list isn't borrowed while firing.
            let mut in_range: Vec<(f32, Vec2, f32)> = self
                .fruits
                .iter()
                .filter(|f| f.pos.distance(t.pos) <= range)
                .map(|f| (f.dist, f.pos, f.current_speed()))
                .collect();

            if in_range.is_empty() {
                continue;
            }

            // Furthest along the track first.
            in_range.sort_by(|a, b| b.0.total_cmp(&a.0));

            let projectile_kind = match t.kind {
                TowerKind::Blender => ProjectileKind::Pulp,
                TowerKind::KnifeThrower => ProjectileKind::Knife,
                _ => ProjectileKind::Seed,
            };
            let splash = t.splash_radius();
            let pierce = t.pierce();

            // A Lv3 Seed Shooter engages the two lead fruit; everything else
            // fires a single shot.
            for (dist, pos, speed) in in_range.iter().take(t.shots()) {
                // Aim where the fruit will be, not where it is. Shots don't
                // home, so without leading, the late-wave speed ramp would make
                // towers miss almost everything through no fault of the player.
                let travel = pos.distance(t.pos) / projectile_kind.speed();
                let aim = self.path.point_at(dist + speed * travel);

                self.projectiles.push(Projectile::new(
                    t.pos,
                    aim,
                    projectile_kind,
                    splash,
                    pierce,
                    t.id,
                ));
                t.shots_fired += 1;
            }

            let lead = in_range[0].1 - t.pos;
            t.angle = lead.y.atan2(lead.x);
            t.cooldown = t.fire_cooldown();
            if t.kind == TowerKind::KnifeThrower {
                self.audio.play_knife();
            } else {
                self.audio.play_shoot();
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Move shots, resolve the first fruit each one touches, and retire spent or
    // stray projectiles.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_projectiles(&mut self, dt: f32) {
        for p in &mut self.projectiles {
            p.update(dt);
        }

        // (fruit index, id of the tower that gets the credit)
        let mut pops: Vec<(usize, u32)> = Vec::new();
        let mut spent: Vec<usize> = Vec::new();
        // Projectiles that connected this frame and owe a point of pierce.
        let mut connected: Vec<usize> = Vec::new();

        for (pi, p) in self.projectiles.iter().enumerate() {
            if p.expired() || off_field(p.pos) {
                spent.push(pi);
                continue;
            }

            let hit = self
                .fruits
                .iter()
                .position(|f| f.pos.distance(p.pos) <= f.radius() + p.kind.radius());

            let Some(fi) = hit else { continue };

            // A piercing knife survives the hit and keeps flying; whether this
            // one is used up is settled after the loop.
            connected.push(pi);

            let splash = p.splash;
            if splash > 0.0 {
                // Splash catches every fruit around the one actually struck.
                let center = self.fruits[fi].pos;
                for (i, f) in self.fruits.iter().enumerate() {
                    if f.pos.distance(center) <= splash {
                        pops.push((i, p.owner));
                    }
                }
                self.audio.play_splash();
            } else {
                pops.push((fi, p.owner));
            }
        }

        // Charge each connecting shot one point of pierce, and retire the ones
        // that ran out. Done here rather than in the loop above so the
        // projectile list isn't borrowed mutably while it's being scanned.
        for &pi in &connected {
            if self.projectiles[pi].consume_pierce() {
                spent.push(pi);
            }
        }

        spent.sort_unstable();
        spent.dedup();
        for &i in spent.iter().rev() {
            self.projectiles.remove(i);
        }

        self.apply_pops(pops);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Pop the listed fruit, pay out, and add their children to the track.
    // Indices are deduped because two shots can land on one fruit in a frame.
    // ─────────────────────────────────────────────────────────────────────────
    fn apply_pops(&mut self, mut pops: Vec<(usize, u32)>) {
        if pops.is_empty() {
            return;
        }
        // Two shots can land on the same fruit in one frame; the first one to
        // be recorded takes the credit.
        pops.sort_unstable_by_key(|&(i, _)| i);
        pops.dedup_by_key(|&mut (i, _)| i);

        // Remove highest index first so the earlier indices stay valid.
        let mut children = Vec::new();
        for &(i, owner) in pops.iter().rev() {
            let f = self.fruits.remove(i);
            // Only the bottom of the ladder pays out; see CASH_PER_FRUIT_CLEARED.
            if f.kind.child().is_none() {
                self.cash += CASH_PER_FRUIT_CLEARED;
            }
            self.splats.push(Splat::burst(f.pos, f.kind));
            self.audio.play_pop(f.kind.tier());
            children.extend(f.split(&self.path));

            // The firing tower may already have been sold, in which case the
            // credit is simply dropped.
            if let Some(t) = self.towers.iter_mut().find(|t| t.id == owner) {
                t.kills += 1;
            }
        }
        self.fruits.extend(children);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // The placed tower currently being inspected, if the selection is still
    // valid. Returns None once a selected tower has been sold.
    // ─────────────────────────────────────────────────────────────────────────
    fn inspected_tower(&self) -> Option<&Tower> {
        self.selected_tower.and_then(|i| self.towers.get(i))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Run fruit over any spike piles they're standing on. A pile pops one fruit
    // per charge and vanishes once it's used up.
    //
    // Children of a popped fruit spawn at the same point on the track, so they
    // land on the same pile and get chewed through too — bounded by charges,
    // which is what makes a Spike Layer good against splits.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_spikes(&mut self) {
        let pops = run_over_spikes(&mut self.spikes, &self.fruits);
        self.spikes.retain(|p| !p.spent());
        self.apply_pops(pops);
    }

    fn update_effects(&mut self, dt: f32) {
        for s in &mut self.splats {
            s.update(dt);
        }
        self.splats.retain(|s| !s.finished());

        for p in &mut self.pulses {
            p.update(dt);
        }
        self.pulses.retain(|p| !p.finished());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // A wave ends once the queue is drained and the field is clear of fruit.
    // ─────────────────────────────────────────────────────────────────────────
    fn check_wave_complete(&mut self) {
        if !(self.wave_active && self.queue.is_empty() && self.fruits.is_empty()) {
            return;
        }

        self.cash += wave::clear_bonus(self.wave);
        self.wave_active = false;

        // That was the final wave, so the route is cleared.
        if self.wave >= self.total_waves() {
            self.state = State::Victory;
            self.audio.play_victory();
            self.audio.play_music(Track::Menu);
            return;
        }

        self.wave += 1;
        self.audio.play_wave_clear();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // How many waves the current route runs for.
    // ─────────────────────────────────────────────────────────────────────────
    fn total_waves(&self) -> u32 {
        tracks::TRACKS[self.track].waves
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Draw the current frame, back to front.
    // ─────────────────────────────────────────────────────────────────────────
    fn draw(&self) {
        render::draw_background(&self.palette);

        // The selection screen shows its own route previews, so the live board
        // is hidden behind it rather than drawn as a stale backdrop.
        if self.state != State::TrackSelect {
            // Scenery sits under the track, so foliage can never obscure the
            // route the fruit are walking.
            render::draw_scenery(&self.props, &self.palette);
            render::draw_path(&self.path, &self.palette);
            // Spikes lie on the track, under everything that moves over them.
            render::draw_spikes(&self.spikes);

            // Range footprint of the inspected tower sits under the towers.
            if let Some(t) = self.inspected_tower() {
                render::draw_tower_selection(t);
            }

            for t in &self.towers {
                render::draw_tower(t);
            }
            for p in &self.pulses {
                render::draw_pulse(p);
            }
            for f in &self.fruits {
                render::draw_fruit(f);
            }
            for p in &self.projectiles {
                render::draw_projectile(p);
            }
            for s in &self.splats {
                render::draw_splat(s);
            }
        }

        match self.state {
            State::Menu => render::draw_menu(),
            State::TrackSelect => render::draw_track_select(),
            State::GameOver => render::draw_game_over(self.wave, self.total_waves()),
            State::Victory => render::draw_victory(
                tracks::TRACKS[self.track].name,
                self.total_waves(),
                self.lives,
            ),
            State::Playing => {
                render::draw_hud(
                    self.lives,
                    self.cash,
                    self.wave,
                    self.total_waves(),
                    self.wave_active,
                    self.audio.muted(),
                );
                render::draw_shop(self.selected, self.cash);

                // The tower panel floats over the field, above everything else.
                if let Some(t) = self.inspected_tower() {
                    render::draw_tower_panel(t, self.cash);
                }

                // Placement preview follows the cursor while a tower is armed.
                if let Some(kind) = self.selected {
                    let m = mouse_vec();
                    if m.y < PLAYFIELD_H {
                        render::draw_ghost(m, kind, self.placement_valid(m, kind));
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Walk the fruit over the spike piles, spending one spike per fruit popped.
// Returns (fruit index, owning tower id) for every fruit a pile got.
//
// A fruit costs exactly one spike however many piles it is standing on. Piles
// are dropped SPIKE_SPACING apart and each reaches radius + SPIKE_RADIUS along
// the track, so overlap is the norm rather than the exception — a watermelon is
// wide enough to sit on three at once. Charging every pile that covered it spent
// three spikes to pop a single fruit, which quietly broke the rule the tower is
// sold on: a pile pops one fruit per spike.
// ─────────────────────────────────────────────────────────────────────────────
fn run_over_spikes(piles: &mut [SpikePile], fruits: &[Fruit]) -> Vec<(usize, u32)> {
    let mut pops = Vec::new();

    for (i, f) in fruits.iter().enumerate() {
        // Which of the covering piles pays is arbitrary — every spike is worth
        // the same — so it is simply the first one with any left.
        let hit = piles
            .iter_mut()
            .find(|p| !p.spent() && p.covers(f.dist, f.radius()));

        if let Some(pile) = hit {
            pile.charges -= 1;
            pops.push((i, pile.owner));
        }
    }

    pops
}

// ─────────────────────────────────────────────────────────────────────────────
// Pick where a Spike Layer should drop its next pile.
//
// Walks the route looking for points inside the tower's range that aren't
// already too close to an existing pile, and takes the one furthest along.
// Dropping at the far edge of coverage first, then working backwards as those
// spots fill, spreads a tower's piles across its whole stretch of track.
// ─────────────────────────────────────────────────────────────────────────────
fn pick_spike_spot(
    path: &Path,
    tower: Vec2,
    range: f32,
    existing: &[SpikePile],
) -> Option<(f32, Vec2)> {
    const STEP: f32 = 12.0;

    let mut best: Option<(f32, Vec2)> = None;
    let mut d = 0.0;

    while d <= path.total() {
        let p = path.point_at(d);
        if p.distance(tower) <= range
            && existing
                .iter()
                .all(|s| (s.dist - d).abs() >= SPIKE_SPACING)
        {
            best = Some((d, p));
        }
        d += STEP;
    }

    best
}

// ─────────────────────────────────────────────────────────────────────────────
// Current mouse position as a Vec2.
// ─────────────────────────────────────────────────────────────────────────────
fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}

// ─────────────────────────────────────────────────────────────────────────────
// True if a point has strayed well outside the playfield, used to retire shots
// that never hit anything.
// ─────────────────────────────────────────────────────────────────────────────
fn off_field(p: Vec2) -> bool {
    p.x < -50.0 || p.x > screen_width() + 50.0 || p.y < -50.0 || p.y > PLAYFIELD_H + 50.0
}

// ─────────────────────────────────────────────────────────────────────────────
// Development helper: stage a board showing every tower and every fruit tier at
// once, so the artwork can be reviewed in a single screenshot. Only reachable
// via the FRUITSPLAT_SCREENSHOT environment variable.
// ─────────────────────────────────────────────────────────────────────────────
impl Game {
    fn stage_art_demo(&mut self) {
        // FRUITSPLAT_TRACK picks which route's backdrop to stage.
        let track = std::env::var("FRUITSPLAT_TRACK")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        self.start_run(track);
        self.cash = 500;

        // One position per tower kind — must stay the same length as ALL.
        let spots = [
            vec2(185.0, 245.0),
            vec2(395.0, 415.0),
            vec2(640.0, 375.0),
            vec2(880.0, 430.0),
            vec2(330.0, 250.0),
        ];
        assert_eq!(spots.len(), TowerKind::ALL.len());
        for (i, kind) in TowerKind::ALL.iter().enumerate() {
            let mut t = Tower::new(*kind, spots[i], self.next_tower_id);
            // Spread the levels so the pips and upgraded stats are visible.
            t.level = (i as u8 % 3) + 1;
            t.angle = -0.62;
            t.shots_fired = 120 + i as u32 * 37;
            t.kills = 45 + i as u32 * 11;
            t.chills = 88;
            self.next_tower_id += 1;
            self.towers.push(t);
        }

        // One fruit of every tier, spaced out along the route.
        for tier in 0..5u8 {
            let dist = 380.0 + tier as f32 * 270.0;
            self.fruits
                .push(Fruit::new(FruitKind::from_tier(tier), dist, &self.path, 1.0));
        }
        // Chill one so the frost treatment shows up too.
        self.fruits[2].chill(0.35, 5.0);

        // Spike piles at varying wear, since a tower would only have managed
        // one drop in the handful of frames before the capture.
        for (i, charges) in [9u32, 5, 2].iter().enumerate() {
            let dist = 520.0 + i as f32 * 240.0;
            self.spikes.push(SpikePile::new(
                self.path.point_at(dist),
                dist,
                *charges,
                0,
                i as f32 * 37.0,
            ));
        }

        // The panel covers part of the board, so only open it when it's the
        // thing being reviewed.
        // Selects the last tower placed, which is the most recently added kind
        // and so usually the one being reviewed.
        if std::env::var("FRUITSPLAT_SCREEN").as_deref() == Ok("panel") {
            self.selected_tower = Some(self.towers.len() - 1);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point: seed the RNG, then pump update/draw until the window closes.
// ─────────────────────────────────────────────────────────────────────────────
#[macroquad::main(window_conf)]
async fn main() {
    // Without seeding, every run would deal the identical fruit sequence.
    macroquad::rand::srand(miniquad::date::now() as u64);

    let mut game = Game::new(Audio::load().await);
    game.audio.play_music(Track::Menu);

    // Screenshot mode: stage a scene, let it settle, write a PNG and exit.
    let shot_path = std::env::var("FRUITSPLAT_SCREENSHOT").ok();
    if shot_path.is_some() {
        game.audio.toggle_mute();
        match std::env::var("FRUITSPLAT_SCREEN").as_deref() {
            Ok("select") => game.state = State::TrackSelect,
            Ok("menu") => game.state = State::Menu,
            Ok("victory") => {
                game.stage_art_demo();
                game.wave = game.total_waves();
                game.lives = 14;
                game.state = State::Victory;
            }
            Ok("over") => {
                game.stage_art_demo();
                game.wave = 7;
                game.state = State::GameOver;
            }
            _ => game.stage_art_demo(),
        }
    }
    let mut frames = 0u32;

    loop {
        let dt = get_frame_time().min(0.05); // clamp so a stutter can't teleport fruit
        game.update(dt);
        game.draw();

        if let Some(path) = &shot_path {
            frames += 1;
            // Give the window a few frames to settle before capturing.
            if frames >= 12 {
                get_screen_data().export_png(path);
                return;
            }
        }

        next_frame().await;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Only the free functions are covered here. The Game methods can't be reached
// from a test: they read macroquad's input globals and hold an Audio, neither
// of which exists without a window.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use tower::SPIKE_RADIUS;

    /// A long straight run, so a fruit's distance along it is easy to reason
    /// about. Spike collision is measured along the track, never in pixels.
    fn straight_path() -> Path {
        Path::new(vec![vec2(0.0, 0.0), vec2(2000.0, 0.0)])
    }

    fn fruit_at(kind: FruitKind, dist: f32, path: &Path) -> Fruit {
        Fruit::new(kind, dist, path, 1.0)
    }

    fn pile_at(dist: f32, charges: u32, owner: u32) -> SpikePile {
        SpikePile::new(Vec2::ZERO, dist, charges, owner, 0.0)
    }

    #[test]
    fn a_fruit_standing_on_two_piles_only_costs_one_spike() {
        let path = straight_path();
        // The tightest two piles can ever sit, and a watermelon parked between
        // them — the arrangement a saturated stretch of track produces.
        let mut piles = vec![pile_at(500.0, 4, 7), pile_at(500.0 + SPIKE_SPACING, 4, 7)];
        let fruits = vec![fruit_at(
            FruitKind::Watermelon,
            500.0 + SPIKE_SPACING * 0.5,
            &path,
        )];
        assert!(
            piles
                .iter()
                .all(|p| p.covers(fruits[0].dist, fruits[0].radius())),
            "setup is wrong: both piles must cover the fruit"
        );

        let pops = run_over_spikes(&mut piles, &fruits);

        assert_eq!(pops.len(), 1, "one fruit can only be popped once");
        assert_eq!(
            piles.iter().map(|p| p.charges).sum::<u32>(),
            7,
            "one spike spent, not one per overlapping pile"
        );
    }

    #[test]
    fn each_fruit_on_a_pile_costs_its_own_spike() {
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 4, 7)];
        let fruits = vec![
            fruit_at(FruitKind::Blueberry, 495.0, &path),
            fruit_at(FruitKind::Blueberry, 505.0, &path),
        ];

        assert_eq!(run_over_spikes(&mut piles, &fruits).len(), 2);
        assert_eq!(piles[0].charges, 2);
    }

    #[test]
    fn a_pile_stops_popping_once_its_spikes_run_out() {
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 1, 7)];
        let fruits = vec![
            fruit_at(FruitKind::Blueberry, 498.0, &path),
            fruit_at(FruitKind::Blueberry, 502.0, &path),
        ];

        assert_eq!(run_over_spikes(&mut piles, &fruits).len(), 1);
        assert!(piles[0].spent());
    }

    #[test]
    fn a_pile_ignores_fruit_walking_a_neighbouring_lane() {
        // On the switchback routes two stretches of track run within a few
        // dozen pixels of each other. Far apart along the route is far apart.
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 4, 7)];
        let fruits = vec![fruit_at(FruitKind::Watermelon, 900.0, &path)];

        assert!(run_over_spikes(&mut piles, &fruits).is_empty());
        assert_eq!(piles[0].charges, 4, "an untouched pile keeps its spikes");
    }

    #[test]
    fn a_pop_is_credited_to_the_tower_that_dropped_the_pile() {
        let path = straight_path();
        let mut piles = vec![pile_at(200.0, 4, 11), pile_at(900.0, 4, 22)];
        let fruits = vec![fruit_at(FruitKind::Lime, 900.0, &path)];

        assert_eq!(run_over_spikes(&mut piles, &fruits), vec![(0, 22)]);
    }

    #[test]
    fn a_spike_spot_is_inside_range_and_clear_of_the_piles_already_down() {
        let path = straight_path();
        let tower = vec2(600.0, 0.0);
        let existing = vec![pile_at(640.0, 4, 0)];

        let (dist, pos) = pick_spike_spot(&path, tower, 120.0, &existing).unwrap();

        assert!(pos.distance(tower) <= 120.0, "spot fell outside the range");
        assert!(
            (dist - existing[0].dist).abs() >= SPIKE_SPACING,
            "spot crowded a pile already on the track"
        );
        // Coverage runs to the far edge of range, then works backwards as spots
        // fill, so a tower spreads its piles over its whole stretch.
        assert!(dist > tower.x, "should favour the far end of the coverage");
    }

    #[test]
    fn a_pile_and_a_fruit_that_only_just_touch_still_connect() {
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 4, 7)];
        let r = FruitKind::Lime.radius();
        let fruits = vec![fruit_at(FruitKind::Lime, 500.0 + r + SPIKE_RADIUS - 0.1, &path)];

        assert_eq!(run_over_spikes(&mut piles, &fruits).len(), 1);
    }
}
