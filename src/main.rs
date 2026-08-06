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

mod audio;
mod fruit;
mod path;
mod projectile;
mod render;
mod tower;
mod wave;

use audio::{Audio, Track};
use fruit::{Fruit, FruitKind, Splat};
use path::Path;
use projectile::{Projectile, ProjectileKind};
use tower::{Pulse, Tower, TowerKind, FREEZE_DURATION, PATH_CLEARANCE, TOWER_RADIUS};

/// Height of the playable field; the shop bar occupies the strip below it.
pub const PLAYFIELD_H: f32 = 650.0;

const WINDOW_W: i32 = 1000;
const WINDOW_H: i32 = 740;
const START_LIVES: u32 = 20;
const START_CASH: u32 = 250;
/// Cash earned per fruit popped, regardless of tier.
const CASH_PER_POP: u32 = 1;

// ─────────────────────────────────────────────────────────────────────────────
// Window configuration, consumed by the macroquad::main attribute.
// ─────────────────────────────────────────────────────────────────────────────
fn window_conf() -> Conf {
    Conf {
        window_title: "Fruit Splat".to_owned(),
        window_width: WINDOW_W,
        window_height: WINDOW_H,
        high_dpi: true,
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The track the fruit follow: a snake from off the left edge to off the right.
// Starting and ending outside the window means fruit fade in and out of view
// rather than popping into existence at the border.
// ─────────────────────────────────────────────────────────────────────────────
fn build_path() -> Path {
    Path::new(vec![
        vec2(-40.0, 150.0),
        vec2(260.0, 150.0),
        vec2(260.0, 330.0),
        vec2(110.0, 330.0),
        vec2(110.0, 520.0),
        vec2(520.0, 520.0),
        vec2(520.0, 230.0),
        vec2(760.0, 230.0),
        vec2(760.0, 560.0),
        vec2(1040.0, 560.0),
    ])
}

/// Which screen the game is currently on.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Menu,
    Playing,
    GameOver,
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
    /// Fruit still waiting to enter the track this wave.
    queue: Vec<FruitKind>,
    spawn_timer: f32,
    wave: u32,
    wave_active: bool,
    lives: u32,
    cash: u32,
    /// Tower type armed for placement, if any.
    selected: Option<TowerKind>,
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
            path: build_path(),
            fruits: Vec::new(),
            towers: Vec::new(),
            projectiles: Vec::new(),
            splats: Vec::new(),
            pulses: Vec::new(),
            queue: Vec::new(),
            spawn_timer: 0.0,
            wave: 1,
            wave_active: false,
            lives: START_LIVES,
            cash: START_CASH,
            selected: None,
            audio,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clear the board and start from wave 1.
    // ─────────────────────────────────────────────────────────────────────────
    fn start_game(&mut self) {
        self.fruits.clear();
        self.towers.clear();
        self.projectiles.clear();
        self.splats.clear();
        self.pulses.clear();
        self.queue.clear();
        self.spawn_timer = 0.0;
        self.wave = 1;
        self.wave_active = false;
        self.lives = START_LIVES;
        self.cash = START_CASH;
        self.selected = None;
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
            State::Menu | State::GameOver => {
                if is_mouse_button_pressed(MouseButton::Left) || is_key_pressed(KeyCode::Space) {
                    self.start_game();
                }
            }
            State::Playing => self.update_play(dt),
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
        self.update_effects(dt);

        self.check_wave_complete();

        if self.lives == 0 {
            self.state = State::GameOver;
            self.audio.play_game_over();
            self.audio.play_music(Track::Menu);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Hotkeys, tower selection, placement, and sending the next wave.
    // ─────────────────────────────────────────────────────────────────────────
    fn handle_input(&mut self) {
        for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
            .iter()
            .enumerate()
        {
            if is_key_pressed(*key) {
                self.toggle_selection(TowerKind::ALL[i]);
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            self.selected = None;
        }

        if is_key_pressed(KeyCode::Space) && !self.wave_active {
            self.start_wave();
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let m = mouse_vec();
            if m.y >= PLAYFIELD_H {
                self.click_shop(m);
            } else {
                self.try_place(m);
            }
        }
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
        self.towers.push(Tower::new(kind, p));
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
                self.fruits.push(Fruit::new(kind, 0.0, &self.path));
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

            let range = t.kind.range();

            if t.kind == TowerKind::Freezer {
                let mut chilled_any = false;
                for f in &mut self.fruits {
                    if f.pos.distance(t.pos) <= range {
                        f.slow_timer = FREEZE_DURATION;
                        chilled_any = true;
                    }
                }
                // Only spend the cooldown when there was something to chill.
                if chilled_any {
                    t.cooldown = t.kind.cooldown();
                    self.pulses.push(Pulse::new(t.pos, range));
                    self.audio.play_freeze();
                }
                continue;
            }

            // "First" targeting: the fruit closest to the exit is the threat.
            let mut target: Option<Vec2> = None;
            let mut best_dist = f32::MIN;
            for f in &self.fruits {
                if f.pos.distance(t.pos) <= range && f.dist > best_dist {
                    best_dist = f.dist;
                    target = Some(f.pos);
                }
            }

            if let Some(target) = target {
                let delta = target - t.pos;
                t.angle = delta.y.atan2(delta.x);

                let kind = if t.kind == TowerKind::Blender {
                    ProjectileKind::Pulp
                } else {
                    ProjectileKind::Seed
                };
                self.projectiles.push(Projectile::new(t.pos, target, kind));
                t.cooldown = t.kind.cooldown();
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

        let mut pops: Vec<usize> = Vec::new();
        let mut spent: Vec<usize> = Vec::new();

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
            spent.push(pi);

            let splash = p.kind.splash_radius();
            if splash > 0.0 {
                // Splash catches every fruit around the one actually struck.
                let center = self.fruits[fi].pos;
                for (i, f) in self.fruits.iter().enumerate() {
                    if f.pos.distance(center) <= splash {
                        pops.push(i);
                    }
                }
                self.audio.play_splash();
            } else {
                pops.push(fi);
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
    fn apply_pops(&mut self, mut idx: Vec<usize>) {
        if idx.is_empty() {
            return;
        }
        idx.sort_unstable();
        idx.dedup();

        // Remove highest index first so the earlier indices stay valid.
        let mut children = Vec::new();
        for &i in idx.iter().rev() {
            let f = self.fruits.remove(i);
            self.cash += CASH_PER_POP;
            self.splats.push(Splat::burst(f.pos, f.kind));
            self.audio.play_pop(f.kind.tier());
            children.extend(f.split(&self.path));
        }
        self.fruits.extend(children);
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
        if self.wave_active && self.queue.is_empty() && self.fruits.is_empty() {
            self.cash += wave::clear_bonus(self.wave);
            self.wave += 1;
            self.wave_active = false;
            self.audio.play_wave_clear();
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Draw the current frame, back to front.
    // ─────────────────────────────────────────────────────────────────────────
    fn draw(&self) {
        render::draw_background();
        render::draw_path(&self.path);

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

        match self.state {
            State::Menu => render::draw_menu(),
            State::GameOver => render::draw_game_over(self.wave),
            State::Playing => {
                render::draw_hud(self.lives, self.cash, self.wave, self.wave_active);
                render::draw_shop(self.selected, self.cash, self.audio.muted());

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
// Entry point: seed the RNG, then pump update/draw until the window closes.
// ─────────────────────────────────────────────────────────────────────────────
#[macroquad::main(window_conf)]
async fn main() {
    // Without seeding, every run would deal the identical fruit sequence.
    macroquad::rand::srand(miniquad::date::now() as u64);

    let mut game = Game::new(Audio::load().await);
    game.audio.play_music(Track::Menu);

    loop {
        let dt = get_frame_time().min(0.05); // clamp so a stutter can't teleport fruit
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
