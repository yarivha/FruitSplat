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
mod mode;
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
use tower::{Pulse, SpikePile, Tower, TowerKind, PATH_CLEARANCE, SPIKE_SPACING, TOWER_RADIUS};

/// Height of the playable field; the shop bar occupies the strip below it.
pub const PLAYFIELD_H: f32 = 650.0;
/// Width of the playable field. Routes and scenery are authored against this
/// fixed space rather than the live window size, so layout is reproducible
/// without a graphics context.
///
/// Widened from 1000 because every route ran off the right edge with its exit
/// still to come: the last stretch of track, and the exit marker itself, were
/// permanently out of sight. Routes still overhang the window at both ends so
/// fruit enter and leave off-screen, but the overhang is now margin rather than
/// the end of the route.
pub const PLAYFIELD_W: f32 = 1200.0;

const WINDOW_W: i32 = PLAYFIELD_W as i32;
const WINDOW_H: i32 = 740;
/// Cash earned for each fruit destroyed outright — that is, one that had no
/// children left to split into.
///
/// Paying per *pop* meant a watermelon was worth $31, so income scaled with the
/// threat while towers stayed a one-time cost, and the surplus compounded until
/// the player could afford six times the firepower a wave needed. Paying only
/// for the bottom of the split ladder roughly halves late-game income while
/// still rewarding bigger fruit, which are worth 16 blueberries apiece.
const CASH_PER_FRUIT_CLEARED: u32 = 1;
/// The number-row keys, shared by the two screens that use them: they arm tower
/// types in play, and pick a route on the selection screen. One array so a sixth
/// route or tower can't gain a card without gaining a key, which is exactly how
/// route five ended up unreachable from the keyboard.
const NUMBER_KEYS: [KeyCode; 5] = [
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
];
/// Seconds between a wave clearing and the next one going out on its own, once
/// auto-send is armed.
///
/// Deliberately not zero. Between waves is when towers get bought, placed and
/// upgraded, and that is most of the game's decision-making — sending instantly
/// would take that away rather than automate it. Three seconds is enough to
/// place a tower or buy an upgrade without turning the gap into dead time, and
/// Space still overrides it for anyone who wants the wave now.
const AUTO_WAVE_DELAY: f32 = 3.0;
/// How long the quit button stays armed waiting for its confirming click.
/// Long enough to move the mouse back deliberately, short enough that it can't
/// still be waiting by the time an unrelated click lands on it later.
const QUIT_ARM_SECONDS: f32 = 3.0;

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
    /// One path per lane of the current route. Most routes have a single lane;
    /// a two-entrance route has two, and every fruit and spike pile carries the
    /// index of the one it belongs to.
    paths: Vec<Path>,
    /// Which lane the next spawned fruit goes down. Lanes are dealt in turn, so
    /// a two-entrance route splits each wave evenly between its gates instead of
    /// leaving the split to chance.
    next_lane: usize,
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
    /// Which entry of mode::MODES the run is being played at. Chosen on the
    /// route screen and kept between runs, so picking Hard once doesn't have to
    /// be picked again every time a route is chosen.
    mode: usize,
    /// Backdrop for the current route: colours plus decorative props. Laid out
    /// once per run, since placement is rejection sampling and not cheap.
    palette: scenery::Palette,
    props: Vec<scenery::Prop>,
    /// Handed out to each placed tower so projectiles can credit kills back to
    /// a specific tower even after others are sold.
    next_tower_id: u32,
    /// Seconds left on the armed quit button. Above zero, the next click on it
    /// ends the run; it forgets it was asked once this runs out.
    quit_armed: f32,
    /// Whether waves send themselves. Kept across runs like the difficulty —
    /// someone who wants this on wants it on for the next route too.
    auto_wave: bool,
    /// Seconds until the next wave sends itself. Only counts down while
    /// `auto_wave` is on and no wave is walking.
    auto_timer: f32,
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
            paths: tracks::TRACKS[0].paths(),
            next_lane: 0,
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
            lives: mode::mode(mode::DEFAULT_MODE).start_lives,
            cash: mode::mode(mode::DEFAULT_MODE).start_cash,
            selected: None,
            selected_tower: None,
            track: 0,
            mode: mode::DEFAULT_MODE,
            palette: scenery::palette(0),
            props: Vec::new(),
            next_tower_id: 0,
            quit_armed: 0.0,
            auto_wave: false,
            auto_timer: AUTO_WAVE_DELAY,
            audio,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Clear the board and start from wave 1 on the chosen route.
    // ─────────────────────────────────────────────────────────────────────────
    fn start_run(&mut self, track: usize) {
        self.track = track.min(tracks::TRACKS.len() - 1);
        self.paths = tracks::TRACKS[self.track].paths();
        self.palette = scenery::palette(self.track);
        self.props = scenery::generate(self.track, &self.paths);

        self.clear_board();
        self.spawn_timer = 0.0;
        self.wave = 1;
        // The mode decides the opening hand. Nothing else about the run reads
        // it, so a mode can never quietly rewrite the wave table underneath the
        // balance report.
        let m = mode::mode(self.mode);
        self.lives = m.start_lives;
        self.cash = m.start_cash;
        self.next_tower_id = 0;
        self.state = State::Playing;
        self.audio.play_music(Track::Game);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sweep everything a run put on the board. Shared by starting a run and
    // walking away from one, so neither can forget a list the other clears.
    // ─────────────────────────────────────────────────────────────────────────
    fn clear_board(&mut self) {
        self.fruits.clear();
        self.towers.clear();
        self.projectiles.clear();
        self.splats.clear();
        self.pulses.clear();
        self.spikes.clear();
        self.queue.clear();
        self.wave_active = false;
        self.selected = None;
        self.selected_tower = None;
        self.quit_armed = 0.0;
        self.next_lane = 0;
        // The auto-send *setting* survives — it is a preference, not run state —
        // but its countdown restarts, so a fresh run always gets the full gap
        // before wave one rather than inheriting a part-spent timer.
        self.auto_timer = AUTO_WAVE_DELAY;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // The path a fruit or pile on `lane` walks. Falls back to the first lane
    // rather than panicking: nothing should ever hold a stale lane index, but a
    // wave is a bad moment to find out otherwise.
    // ─────────────────────────────────────────────────────────────────────────
    fn lane(&self, lane: usize) -> &Path {
        self.paths.get(lane).unwrap_or(&self.paths[0])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Give up on the run in progress and go back to the title screen.
    //
    // The board is swept rather than left standing, so the title screen looks
    // like a title screen instead of the wreckage of the run just walked away
    // from. The music dropping back to the menu loop is the confirmation that
    // the run is over — there is no sound effect for quitting, because none of
    // the ones here mean this.
    // ─────────────────────────────────────────────────────────────────────────
    fn abandon_run(&mut self) {
        self.clear_board();
        self.state = State::Menu;
        self.audio.play_music(Track::Menu);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Advance the whole game by `dt` seconds.
    // ─────────────────────────────────────────────────────────────────────────
    fn update(&mut self, dt: f32) {
        self.audio.begin_frame(dt);

        // The audio toggles are on every screen, so they get first refusal on a
        // click before anything screen-specific sees it. Without that, muting
        // from the title screen would also start a run, and muting mid-play
        // would try to place a tower behind the button.
        let audio_click =
            is_mouse_button_pressed(MouseButton::Left) && self.click_audio_buttons(mouse_vec());

        match self.state {
            // These all lead into route selection rather than straight into a
            // run, so a fresh route can be picked after finishing one.
            State::Menu | State::GameOver | State::Victory => {
                if !audio_click
                    && (is_mouse_button_pressed(MouseButton::Left)
                        || is_key_pressed(KeyCode::Space))
                {
                    self.state = State::TrackSelect;
                }
            }
            State::TrackSelect => {
                if !audio_click {
                    self.update_track_select();
                }
            }
            // The run keeps simulating either way — only the click is spent, so
            // muting never costs the player a frame of the wave.
            State::Playing => self.update_play(dt, audio_click),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Route a click at the audio toggles. Returns true when one took it, so the
    // click doesn't also land on whatever screen is underneath.
    // ─────────────────────────────────────────────────────────────────────────
    fn click_audio_buttons(&mut self, m: Vec2) -> bool {
        if render::audio_button_rect(0).contains(m) {
            self.audio.toggle_sfx();
            return true;
        }
        if render::audio_button_rect(1).contains(m) {
            self.audio.toggle_music();
            return true;
        }
        false
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Route selection: set a difficulty, then pick a route.
    //
    // The mode buttons only change the setting — it is choosing a route that
    // starts the run, so the difficulty can be changed as many times as the
    // player likes before committing to anything.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_track_select(&mut self) {
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let m = mouse_vec();

        // Difficulty is tested first, so a click on a mode button never also
        // falls through and starts a run.
        if clicked {
            for i in 0..mode::MODES.len() {
                if render::mode_button_rect(i).contains(m) {
                    self.mode = i;
                    self.audio.play_place();
                    return;
                }
            }
        }

        for i in 0..tracks::TRACKS.len() {
            let picked_by_key = NUMBER_KEYS.get(i).is_some_and(|&k| is_key_pressed(k));
            let picked_by_click = clicked && render::track_card_rect(i).contains(m);

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
    fn update_play(&mut self, dt: f32, audio_click: bool) {
        self.handle_input(audio_click);

        // Input can have just walked away from the run, in which case the board
        // is already swept and there is nothing left to step.
        if self.state != State::Playing {
            return;
        }

        if self.quit_armed > 0.0 {
            self.quit_armed -= dt;
        }

        if self.wave_active {
            self.spawn_from_queue(dt);
        }

        for f in &mut self.fruits {
            // Each fruit advances along its own lane. Borrowed by index rather
            // than through lane() because that borrows all of self.
            let path = self.paths.get(f.lane).unwrap_or(&self.paths[0]);
            f.update(dt, path);
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

        // After completion, so clearing the last wave of a route settles into
        // Victory rather than auto-sending a wave past the end of the run.
        if self.state == State::Playing {
            self.tick_auto_wave(dt);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Send the next wave on its own, once the gap has run down.
    //
    // The timer is held at full whenever a wave is walking, so it always starts
    // from the top the moment the field clears rather than carrying a part-spent
    // countdown across from somewhere else.
    // ─────────────────────────────────────────────────────────────────────────
    fn tick_auto_wave(&mut self, dt: f32) {
        if !self.auto_wave || self.wave_active {
            self.auto_timer = AUTO_WAVE_DELAY;
            return;
        }

        self.auto_timer -= dt;
        if self.auto_timer <= 0.0 {
            self.start_wave();
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Hotkeys, tower selection, placement, and sending the next wave.
    // ─────────────────────────────────────────────────────────────────────────
    fn handle_input(&mut self, audio_click: bool) {
        for (i, key) in NUMBER_KEYS.iter().enumerate() {
            if is_key_pressed(*key) {
                self.toggle_selection(TowerKind::ALL[i]);
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            self.selected = None;
            self.selected_tower = None;
            // Right click is already this game's "cancel", so it stands the
            // quit button down too.
            self.quit_armed = 0.0;
        }

        if is_key_pressed(KeyCode::Space) && !self.wave_active {
            self.start_wave();
        }

        if is_mouse_button_pressed(MouseButton::Left) && !audio_click {
            let m = mouse_vec();
            // Quit is tested first because it is drawn last: the tower panel can
            // float underneath it, and whatever is on top should take the click.
            if self.click_quit_button(m) {
                // Consumed.
            } else if render::auto_button_rect().contains(m) {
                self.auto_wave = !self.auto_wave;
                self.auto_timer = AUTO_WAVE_DELAY;
                self.audio.play_place();
            } else if m.y >= PLAYFIELD_H {
                self.click_shop(m);
            } else if !self.click_tower_panel(m) {
                // Only treat it as a field click if the panel didn't take it.
                self.click_field(m);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Route a click at the quit button. The first click arms it, a second
    // confirms, and the arming lapses on its own if neither happens.
    // ─────────────────────────────────────────────────────────────────────────
    fn click_quit_button(&mut self, m: Vec2) -> bool {
        if !render::quit_button_rect().contains(m) {
            return false;
        }

        if self.quit_armed > 0.0 {
            self.abandon_run();
        } else {
            self.quit_armed = QUIT_ARM_SECONDS;
        }
        true
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
    // Place the armed tower if the spot is legal, and disarm afterwards.
    //
    // Keeping the type armed let several of one kind go down in a row, but it
    // made the common case worse: while anything is armed a click on the field
    // *places*, so a placed tower could not be clicked to open its panel, and a
    // stray click bought another tower. Cancelling first meant a right click,
    // which on the web build the browser takes for its own context menu.
    //
    // Re-arming is one click on the shop button or one number key, so placing a
    // row of them costs a keypress each; not being able to touch anything on the
    // field until you disarm cost more.
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
        self.selected = None;
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
        // Clear of every lane, not just one — on a two-entrance route the two
        // gates are far apart and a tower is only legal if it crowds neither.
        if self
            .paths
            .iter()
            .any(|path| path.distance_to(p) < PATH_CLEARANCE)
        {
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
        self.queue = wave::build_wave(self.wave, self.total_waves());
        self.spawn_timer = 0.0;
        self.wave_active = true;
        self.audio.play_wave_start();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Release queued fruit onto the start of a lane on the spawn timer.
    //
    // Lanes are dealt in turn rather than at random, so a two-entrance route
    // splits every wave evenly between its gates. Leaving it to chance would let
    // a run of bad luck send most of a wave down one lane, and the player would
    // have no way to tell that from the route being unfair.
    // ─────────────────────────────────────────────────────────────────────────
    fn spawn_from_queue(&mut self, dt: f32) {
        if self.queue.is_empty() {
            return;
        }

        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            if let Some(kind) = self.queue.pop() {
                let lane = self.next_lane % self.paths.len();
                self.next_lane = (self.next_lane + 1) % self.paths.len();

                // Fruit carry the wave's speed ramp, and pass it to their
                // children when they split.
                self.fruits.push(Fruit::new(
                    kind,
                    lane,
                    0.0,
                    &self.paths[lane],
                    wave::speed_multiplier(self.wave, mode::mode(self.mode)),
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
            .filter(|(_, f)| f.reached_end(self.lane(f.lane)))
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

                if let Some((lane, dist, pos)) =
                    pick_spike_spot(&self.paths, t.pos, range, &self.spikes)
                {
                    self.spikes.push(SpikePile::new(
                        pos,
                        lane,
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
            //
            // Sorted on *fraction* of the lane walked, not raw distance. On a
            // two-entrance route the lanes are different lengths, so a fruit
            // 900px down the short lane is nearer its exit — and so more urgent
            // — than one 900px down the long lane. Comparing raw distances would
            // have a tower covering both gates quietly favour whichever lane
            // happened to be longer.
            let mut in_range: Vec<(f32, Vec2, f32, usize, f32)> = self
                .fruits
                .iter()
                .filter(|f| f.pos.distance(t.pos) <= range)
                .map(|f| {
                    let path = self.paths.get(f.lane).unwrap_or(&self.paths[0]);
                    (f.progress(path), f.pos, f.current_speed(), f.lane, f.dist)
                })
                .collect();

            if in_range.is_empty() {
                continue;
            }

            // Furthest along its lane first.
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
            for (_, pos, speed, lane, dist) in in_range.iter().take(t.shots()) {
                // Aim where the fruit will be, not where it is. Shots don't
                // home, so without leading, the late-wave speed ramp would make
                // towers miss almost everything through no fault of the player.
                // The lead runs along the target's own lane.
                let travel = pos.distance(t.pos) / projectile_kind.speed();
                let path = self.paths.get(*lane).unwrap_or(&self.paths[0]);
                let aim = path.point_at(dist + speed * travel);

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
        let mut hits: Vec<(usize, u32)> = Vec::new();
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
                        hits.push((i, p.owner));
                    }
                }
                self.audio.play_splash();
            } else {
                hits.push((fi, p.owner));
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

        self.apply_hits(hits);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Land a batch of hits, one point of damage each, and burst whatever runs
    // out of armour. `hits` is (fruit index, id of the tower to credit).
    //
    // Every ordinary fruit has a single point of armour, so for them a hit is
    // still a pop and a second one landing in the same frame is simply wasted.
    // The boss is why the distinction exists at all: it soaks dozens of hits,
    // and each one has to count, including several arriving in one frame from
    // splash, pierce and separate towers.
    // ─────────────────────────────────────────────────────────────────────────
    fn apply_hits(&mut self, hits: Vec<(usize, u32)>) {
        if hits.is_empty() {
            return;
        }

        let burst = land_hits(&mut self.fruits, hits);

        // Remove highest index first so the earlier indices stay valid.
        let mut children = Vec::new();
        for &(i, owner) in burst.iter().rev() {
            let f = self.fruits.remove(i);
            // Only the bottom of the ladder pays out; see CASH_PER_FRUIT_CLEARED.
            if f.kind.child().is_none() {
                self.cash += CASH_PER_FRUIT_CLEARED;
            }
            self.splats.push(Splat::burst(f.pos, f.kind));
            self.audio.play_pop(f.kind.tier(), f.kind.is_boss());
            children.extend(f.split(self.paths.get(f.lane).unwrap_or(&self.paths[0])));

            // The firing tower may already have been sold, in which case the
            // credit is simply dropped. Only the killing blow scores, so a boss
            // is one kill for whoever finally broke it, not sixty.
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
    // Run fruit over any spike piles they're standing on. A pile spends one
    // spike per hit it lands and vanishes once it's used up.
    //
    // Children of a burst fruit spawn at the same point on the track, so they
    // land on the same pile and get chewed through too — bounded by charges,
    // which is what makes a Spike Layer good against splits.
    //
    // Against the armoured boss that same rule makes a pile a burst of damage
    // rather than a wall: it lands one hit per frame the boss stands on it, so
    // the pile is stripped in a fraction of a second. A Spike Layer stays an
    // anti-swarm tower — which is exactly what the boss leaves behind when it
    // finally comes apart.
    // ─────────────────────────────────────────────────────────────────────────
    fn update_spikes(&mut self) {
        let hits = run_over_spikes(&mut self.spikes, &self.fruits);
        self.spikes.retain(|p| !p.spent());
        self.apply_hits(hits);
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
            render::draw_paths(&self.paths, &self.palette);
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
            State::TrackSelect => render::draw_track_select(self.mode),
            State::GameOver => render::draw_game_over(self.wave, self.total_waves()),
            State::Victory => render::draw_victory(
                tracks::TRACKS[self.track].name,
                self.total_waves(),
                self.lives,
                self.mode,
            ),
            State::Playing => {
                render::draw_hud(&render::HudState {
                    lives: self.lives,
                    cash: self.cash,
                    wave: self.wave,
                    total_waves: self.total_waves(),
                    wave_active: self.wave_active,
                    mode: self.mode,
                    auto: self.auto_wave,
                    auto_countdown: self.auto_timer,
                });
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

        // Above everything, on every screen — matching the fact that they take
        // a click before every screen does. Quit joins them during a run, and
        // is drawn last for the same reason: the tower panel can float under it.
        render::draw_audio_buttons(self.audio.sfx_muted(), self.audio.music_muted());
        if self.state == State::Playing {
            render::draw_quit_button(self.quit_armed > 0.0);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Land a batch of hits on the fruit list, one point of damage each, and report
// which fruit that burst as (index, id of the tower that landed the blow).
//
// Damage is dealt in place, so every index stays valid across the whole batch —
// nothing leaves the list here. `take_hit` reports a burst only once, so a fruit
// struck three more times after it broke still comes back exactly once. That is
// what keeps the caller's removal pass from taking out a bystander: it removes
// by index, and a repeated index would mean removing a fruit that was never hit.
//
// Indices come back in ascending order, ready to be removed from the back. They
// are unique, so the unstable sort has no ties to reorder.
// ─────────────────────────────────────────────────────────────────────────────
fn land_hits(fruits: &mut [Fruit], hits: Vec<(usize, u32)>) -> Vec<(usize, u32)> {
    let mut burst: Vec<(usize, u32)> = Vec::new();

    for (i, owner) in hits {
        // A stale index simply misses. Nothing produces one today, but the
        // alternative is a panic in the middle of a wave.
        if let Some(f) = fruits.get_mut(i) {
            if f.take_hit() {
                burst.push((i, owner));
            }
        }
    }

    burst.sort_unstable_by_key(|&(i, _)| i);
    burst
}

// ─────────────────────────────────────────────────────────────────────────────
// Walk the fruit over the spike piles, spending one spike per hit landed.
// Returns (fruit index, owning tower id) for every hit, for apply_hits.
//
// A fruit costs exactly one spike however many piles it is standing on. Piles
// are dropped SPIKE_SPACING apart and each reaches radius + SPIKE_RADIUS along
// the track, so overlap is the norm rather than the exception — a watermelon is
// wide enough to sit on three at once. Charging every pile that covered it spent
// three spikes on a single fruit, which quietly broke the rule the tower is sold
// on: a pile is worth one hit per spike.
// ─────────────────────────────────────────────────────────────────────────────
fn run_over_spikes(piles: &mut [SpikePile], fruits: &[Fruit]) -> Vec<(usize, u32)> {
    let mut hits = Vec::new();

    for (i, f) in fruits.iter().enumerate() {
        // Which of the covering piles pays is arbitrary — every spike is worth
        // the same — so it is simply the first one with any left.
        let pile = piles
            .iter_mut()
            .find(|p| !p.spent() && p.covers(f.lane, f.dist, f.radius()));

        if let Some(pile) = pile {
            pile.charges -= 1;
            hits.push((i, pile.owner));
        }
    }

    hits
}

// ─────────────────────────────────────────────────────────────────────────────
// Pick which lane and where along it a Spike Layer should drop its next pile.
// Returns (lane, distance along that lane, world position).
//
// Walks every lane looking for points inside the tower's range that aren't
// already too close to an existing pile on that same lane, and takes the one
// furthest along. Dropping at the far edge of coverage first, then working
// backwards as those spots fill, spreads a tower's piles across its whole
// stretch of track.
//
// Candidates are ranked by *fraction* of the lane covered so far, so a tower
// that reaches both gates of a two-entrance route treats them evenly instead of
// emptying its allowance into whichever lane happens to be longer. Spacing is
// only checked against piles on the same lane: two lanes running close together
// near the shared exit are still different track, and a pile on one does not
// block the other.
// ─────────────────────────────────────────────────────────────────────────────
fn pick_spike_spot(
    paths: &[Path],
    tower: Vec2,
    range: f32,
    existing: &[SpikePile],
) -> Option<(usize, f32, Vec2)> {
    const STEP: f32 = 12.0;

    let mut best: Option<(usize, f32, Vec2)> = None;
    let mut best_progress = f32::NEG_INFINITY;

    for (lane, path) in paths.iter().enumerate() {
        let total = path.total();
        let mut d = 0.0;

        while d <= total {
            let p = path.point_at(d);
            let clear = existing
                .iter()
                .all(|s| s.lane != lane || (s.dist - d).abs() >= SPIKE_SPACING);

            if p.distance(tower) <= range && clear {
                let progress = if total > 0.0 { d / total } else { 0.0 };
                if progress >= best_progress {
                    best_progress = progress;
                    best = Some((lane, d, p));
                }
            }
            d += STEP;
        }
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

        // Everything is staged on the first lane; a multi-lane route's second
        // lane is left clear so the two can be told apart in the capture.
        let lane = 0;

        // One fruit of every ordinary tier, spaced out along the route.
        for tier in 0..5u8 {
            let dist = 340.0 + tier as f32 * 300.0;
            self.fruits.push(Fruit::new(
                FruitKind::from_tier(tier),
                lane,
                dist,
                &self.paths[lane],
                1.0,
            ));
        }
        // Chill one so the frost treatment shows up too.
        self.fruits[2].chill(0.35, 5.0);

        // The boss is staged near the end of the route rather than next in the
        // spacing, to keep it out of the staged Freezer's reach. Screenshot mode
        // runs the real game loop for a dozen frames before it captures, so a
        // boss parked inside that range arrives frosted over and its artwork
        // can't be judged.
        let mut boss = Fruit::new(
            FruitKind::Durian,
            lane,
            self.paths[lane].total() - 240.0,
            &self.paths[lane],
            1.0,
        );
        // Rough it up so the armour bar is on screen; an untouched boss
        // deliberately shows nothing.
        boss.hp = boss.kind.armour() * 2 / 5;
        self.fruits.push(boss);

        // Spike piles at varying wear, since a tower would only have managed
        // one drop in the handful of frames before the capture.
        for (i, charges) in [9u32, 5, 2].iter().enumerate() {
            let dist = 520.0 + i as f32 * 240.0;
            self.spikes.push(SpikePile::new(
                self.paths[lane].point_at(dist),
                lane,
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
        game.audio.mute_all();
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

    /// Everything defaults to lane 0; the tests that care about lanes say so.
    fn fruit_at(kind: FruitKind, dist: f32, path: &Path) -> Fruit {
        Fruit::new(kind, 0, dist, path, 1.0)
    }

    fn fruit_on(kind: FruitKind, lane: usize, dist: f32, path: &Path) -> Fruit {
        Fruit::new(kind, lane, dist, path, 1.0)
    }

    fn pile_on(lane: usize, dist: f32, charges: u32, owner: u32) -> SpikePile {
        SpikePile::new(Vec2::ZERO, lane, dist, charges, owner, 0.0)
    }

    fn pile_at(dist: f32, charges: u32, owner: u32) -> SpikePile {
        pile_on(0, dist, charges, owner)
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
                .all(|p| p.covers(fruits[0].lane, fruits[0].dist, fruits[0].radius())),
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
        let paths = vec![straight_path()];
        let tower = vec2(600.0, 0.0);
        let existing = vec![pile_at(640.0, 4, 0)];

        let (lane, dist, pos) = pick_spike_spot(&paths, tower, 120.0, &existing).unwrap();

        assert_eq!(lane, 0, "only one lane to choose from");
        assert!(pos.distance(tower) <= 120.0, "spot fell outside the range");
        assert!(
            (dist - existing[0].dist).abs() >= SPIKE_SPACING,
            "spot crowded a pile already on the track"
        );
        // Coverage runs to the far edge of range, then works backwards as spots
        // fill, so a tower spreads its piles over its whole stretch.
        assert!(dist > tower.x, "should favour the far end of the coverage");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Two-lane routes. These are the cases a single-lane route can never reach,
    // and the ones where getting it wrong is invisible rather than obvious.
    // ─────────────────────────────────────────────────────────────────────────

    /// Two lanes running parallel a short distance apart, as the two gates of a
    /// two-entrance route do on their way to the shared exit.
    fn two_lanes() -> Vec<Path> {
        vec![
            Path::new(vec![vec2(0.0, 0.0), vec2(2000.0, 0.0)]),
            Path::new(vec![vec2(0.0, 60.0), vec2(2000.0, 60.0)]),
        ]
    }

    #[test]
    fn a_pile_never_pops_fruit_walking_the_other_lane() {
        // Same distance along, 60px apart in the world. Only the fruit actually
        // standing on the pile may be hit.
        let paths = two_lanes();
        let mut piles = vec![pile_on(0, 500.0, 4, 7)];
        let fruits = vec![
            fruit_on(FruitKind::Lime, 1, 500.0, &paths[1]),
            fruit_on(FruitKind::Lime, 0, 500.0, &paths[0]),
        ];

        let hits = run_over_spikes(&mut piles, &fruits);

        assert_eq!(hits, vec![(1, 7)], "the wrong lane's fruit was hit");
        assert_eq!(piles[0].charges, 3, "exactly one spike spent");
    }

    #[test]
    fn a_spike_layer_covering_both_lanes_can_drop_on_either() {
        // A tower between the two lanes reaches both. Which lane it picks is not
        // fixed, but it must be a lane it can actually reach.
        let paths = two_lanes();
        let tower = vec2(600.0, 30.0);

        let (lane, _, pos) = pick_spike_spot(&paths, tower, 120.0, &[]).unwrap();

        assert!(lane < paths.len(), "picked a lane that does not exist");
        assert!(pos.distance(tower) <= 120.0, "spot fell outside the range");
    }

    #[test]
    fn a_pile_on_one_lane_does_not_block_a_spot_on_the_other() {
        // Spacing is a per-lane rule. Two lanes that run close together near the
        // shared exit are still different track, so a pile on one must not stop
        // a Spike Layer dropping at the same point along the other.
        let paths = two_lanes();
        let tower = vec2(600.0, 30.0);

        // Saturate lane 0 across the tower's whole reach.
        let existing: Vec<SpikePile> = (0..40)
            .map(|i| pile_on(0, 480.0 + i as f32 * SPIKE_SPACING * 0.5, 4, 0))
            .collect();

        let (lane, _, _) =
            pick_spike_spot(&paths, tower, 120.0, &existing).expect("lane 1 was still wide open");
        assert_eq!(lane, 1, "a full lane 0 should push the drop onto lane 1");
    }

    #[test]
    fn targeting_ranks_fruit_by_how_far_along_their_own_lane_they_are() {
        // The lanes are deliberately different lengths. The fruit that is fewer
        // pixels along is nearer its own exit, and so the greater threat.
        let short = Path::new(vec![vec2(0.0, 0.0), vec2(1000.0, 0.0)]);
        let long = Path::new(vec![vec2(0.0, 60.0), vec2(3000.0, 60.0)]);

        let near_exit = fruit_on(FruitKind::Lime, 0, 900.0, &short);
        let barely_started = fruit_on(FruitKind::Lime, 1, 900.0, &long);

        assert!(
            near_exit.progress(&short) > barely_started.progress(&long),
            "raw distance would have called these two equally urgent"
        );
    }

    #[test]
    fn one_hit_bursts_an_ordinary_fruit_and_the_shooter_is_credited() {
        let path = straight_path();
        let mut fruits = vec![
            fruit_at(FruitKind::Lime, 100.0, &path),
            fruit_at(FruitKind::Orange, 200.0, &path),
        ];

        assert_eq!(land_hits(&mut fruits, vec![(1, 42)]), vec![(1, 42)]);
    }

    #[test]
    fn a_boss_only_bursts_once_however_many_hits_land_together() {
        // The dangerous case: splash, pierce and several towers can all connect
        // with one boss in a single frame. Reporting the burst twice would have
        // the caller remove a second fruit that was never hit.
        let path = straight_path();
        let mut fruits = vec![fruit_at(FruitKind::Durian, 100.0, &path)];

        let armour = FruitKind::Durian.armour() as usize;
        let overkill: Vec<(usize, u32)> = (0..armour + 20).map(|_| (0, 7)).collect();

        assert_eq!(land_hits(&mut fruits, overkill), vec![(0, 7)]);
        assert_eq!(fruits[0].hp, 0);
    }

    #[test]
    fn a_boss_survives_a_batch_that_falls_short_of_its_armour() {
        let path = straight_path();
        let mut fruits = vec![fruit_at(FruitKind::Durian, 100.0, &path)];

        let armour = FruitKind::Durian.armour();
        let batch: Vec<(usize, u32)> = (0..armour - 1).map(|_| (0, 7)).collect();

        assert!(land_hits(&mut fruits, batch).is_empty(), "burst too early");
        assert_eq!(fruits[0].hp, 1, "one point of armour should be left");
    }

    #[test]
    fn burst_indices_come_back_ascending_and_unique() {
        // The caller removes from the back, which only works on sorted indices,
        // and would remove a bystander on a repeated one.
        let path = straight_path();
        let mut fruits: Vec<Fruit> = (0..5)
            .map(|i| fruit_at(FruitKind::Lime, 100.0 + i as f32 * 50.0, &path))
            .collect();

        // Out of order, with a fruit struck twice and one index past the end.
        let burst = land_hits(&mut fruits, vec![(3, 1), (0, 2), (3, 3), (9, 4), (1, 5)]);

        let indices: Vec<usize> = burst.iter().map(|&(i, _)| i).collect();
        assert_eq!(indices, vec![0, 1, 3], "a stale index must simply miss");
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn a_pile_lands_one_hit_per_frame_on_a_boss_standing_over_it() {
        // Against an ordinary fruit a pile is a wall: the fruit is popped and
        // gone. Against armour it is a burst of damage instead — the boss keeps
        // standing there, so the pile strips itself in a handful of frames.
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 4, 7)];
        let mut fruits = vec![fruit_at(FruitKind::Durian, 500.0, &path)];

        for frame in 1..=4 {
            let hits = run_over_spikes(&mut piles, &fruits);
            assert_eq!(hits.len(), 1, "frame {frame} landed no hit");
            fruits[0].take_hit();
        }

        assert!(
            piles[0].spent(),
            "four spikes should be gone in four frames"
        );
        assert!(
            run_over_spikes(&mut piles, &fruits).is_empty(),
            "a spent pile kept hitting"
        );
        assert!(
            fruits[0].hp > 0,
            "four spikes must not break the boss outright"
        );
    }

    #[test]
    fn a_pile_and_a_fruit_that_only_just_touch_still_connect() {
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 4, 7)];
        let r = FruitKind::Lime.radius();
        let fruits = vec![fruit_at(
            FruitKind::Lime,
            500.0 + r + SPIKE_RADIUS - 0.1,
            &path,
        )];

        assert_eq!(run_over_spikes(&mut piles, &fruits).len(), 1);
    }
}
