// =============================================================================
// main.rs — Fruit Splat entry point, game state machine and frame loop
//
// Owns the window config and the Menu → Playing → GameOver cycle. Each frame it
// ticks the spawner, advances the fruit and splats, resolves mouse clicks, then
// hands everything to `render`. Entity behaviour lives in `fruit`, pacing in
// `spawn`; this file holds only the glue and the round rules.
// =============================================================================

use macroquad::prelude::*;

mod fruit;
mod render;
mod spawn;

use fruit::{Fruit, Splat};
use spawn::Spawner;

/// Length of a single round, in seconds.
const ROUND_SECONDS: f32 = 60.0;

// ─────────────────────────────────────────────────────────────────────────────
// Window configuration, consumed by the macroquad::main attribute.
// ─────────────────────────────────────────────────────────────────────────────
fn window_conf() -> Conf {
    Conf {
        window_title: "Fruit Splat".to_owned(),
        window_width: 900,
        window_height: 700,
        high_dpi: true,
        ..Default::default()
    }
}

/// Which screen the game is currently on.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Menu,
    Playing,
    GameOver,
}

/// The whole game world. Small enough to keep in one struct at this stage.
struct Game {
    state: State,
    fruits: Vec<Fruit>,
    splats: Vec<Splat>,
    spawner: Spawner,
    score: u32,
    missed: u32,
    time_left: f32,
    elapsed: f32,
}

impl Game {
    // ─────────────────────────────────────────────────────────────────────────
    // Build the game sitting on the title screen.
    // ─────────────────────────────────────────────────────────────────────────
    fn new() -> Self {
        Game {
            state: State::Menu,
            fruits: Vec::new(),
            splats: Vec::new(),
            spawner: Spawner::new(),
            score: 0,
            missed: 0,
            time_left: ROUND_SECONDS,
            elapsed: 0.0,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clear the board and begin a fresh round.
    // ─────────────────────────────────────────────────────────────────────────
    fn start_round(&mut self) {
        self.fruits.clear();
        self.splats.clear();
        self.spawner.reset();
        self.score = 0;
        self.missed = 0;
        self.time_left = ROUND_SECONDS;
        self.elapsed = 0.0;
        self.state = State::Playing;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Advance the whole game by `dt` seconds.
    // ─────────────────────────────────────────────────────────────────────────
    fn update(&mut self, dt: f32) {
        match self.state {
            State::Menu | State::GameOver => {
                if start_pressed() {
                    self.start_round();
                }
                // Let any leftover pulp finish falling behind the menu text.
                self.update_splats(dt);
            }
            State::Playing => self.update_round(dt),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // One frame of active play: spawn, move, resolve clicks, retire fruit.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_round(&mut self, dt: f32) {
        self.elapsed += dt;
        self.time_left -= dt;

        if let Some(f) = self.spawner.update(dt, self.elapsed) {
            self.fruits.push(f);
        }

        for f in &mut self.fruits {
            f.update(dt);
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            self.try_splat(vec2(mx, my));
        }

        // Anything that reached the top got away.
        let before = self.fruits.len();
        self.fruits.retain(|f| !f.escaped());
        self.missed += (before - self.fruits.len()) as u32;

        self.update_splats(dt);

        if self.time_left <= 0.0 {
            self.time_left = 0.0;
            self.state = State::GameOver;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Pop the topmost fruit under `p`, if any. One click splats one fruit.
    // ─────────────────────────────────────────────────────────────────────────
    fn try_splat(&mut self, p: Vec2) {
        // Search back to front: later fruit are drawn on top, so they should be
        // the ones the click lands on when two overlap.
        if let Some(i) = self.fruits.iter().rposition(|f| f.hit(p)) {
            let f = self.fruits.remove(i);
            self.score += f.kind.points();
            self.splats.push(Splat::burst(f.pos, f.kind));
        }
    }

    fn update_splats(&mut self, dt: f32) {
        for s in &mut self.splats {
            s.update(dt);
        }
        self.splats.retain(|s| !s.finished());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Draw the current frame, back to front.
    // ─────────────────────────────────────────────────────────────────────────
    fn draw(&self) {
        render::draw_background();

        for f in &self.fruits {
            render::draw_fruit(f);
        }
        for s in &self.splats {
            render::draw_splat(s);
        }

        match self.state {
            State::Menu => render::draw_menu(),
            State::Playing => render::draw_hud(self.score, self.missed, self.time_left),
            State::GameOver => render::draw_game_over(self.score, self.missed),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// True when the player asks to start/restart a round.
// ─────────────────────────────────────────────────────────────────────────────
fn start_pressed() -> bool {
    is_mouse_button_pressed(MouseButton::Left) || is_key_pressed(KeyCode::Space)
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point: seed the RNG, then pump update/draw until the window closes.
// ─────────────────────────────────────────────────────────────────────────────
#[macroquad::main(window_conf)]
async fn main() {
    // Without seeding, every run would deal the identical fruit sequence.
    macroquad::rand::srand(miniquad::date::now() as u64);

    let mut game = Game::new();

    loop {
        let dt = get_frame_time().min(0.05); // clamp so a stutter can't teleport fruit
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
