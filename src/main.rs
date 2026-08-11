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
use projectile::{Blast, Projectile, ProjectileKind};
use tower::{Pulse, SpikePile, Tower, TowerKind, PATH_CLEARANCE, SPIKE_SPACING, TOWER_RADIUS};

/// Height of the playable field. The shop is a column down the right now, not a
/// bar along the bottom, so the field gets the whole window height.
pub const PLAYFIELD_H: f32 = 740.0;
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

/// Width of the shop column running down the right of the window.
///
/// A vertical column rather than a horizontal bar because a row of buttons runs
/// out of window after five or six, while a column has room to spare — and the
/// audio, auto, quit and pause controls can share it instead of competing with
/// the wave counter for space in the top strip.
pub const SHOP_PANEL_W: f32 = 220.0;

const WINDOW_W: i32 = (PLAYFIELD_W + SHOP_PANEL_W) as i32;
const WINDOW_H: i32 = PLAYFIELD_H as i32;
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
pub const NUMBER_KEYS: [KeyCode; 8] = [
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
];
/// Where the screenshot demo puts one of each tower. One entry per TowerKind,
/// held to that by a test — it used to be an assert inside the demo, which meant
/// adding a tower broke screenshot mode and nothing said so until someone ran it.
const DEMO_TOWER_SPOTS: [Vec2; TowerKind::ALL.len()] = [
    Vec2::new(185.0, 275.0),
    Vec2::new(395.0, 445.0),
    Vec2::new(640.0, 405.0),
    Vec2::new(880.0, 460.0),
    Vec2::new(330.0, 280.0),
    Vec2::new(1060.0, 290.0),
    Vec2::new(770.0, 235.0),
];

/// One landed hit: which fruit, which tower to credit, and whether it gets
/// through a shield. The flag rides along from whatever dealt the hit, because
/// by the time it lands the tower that fired may have been upgraded or sold.
type Hit = (usize, u32, bool);

/// World steps per frame while fast forward is on. Two ordinary steps rather
/// than one double-length one — see update_play for why that distinction is not
/// cosmetic.
const FAST_FORWARD_STEPS: u32 = 2;

/// Seconds between a wave clearing and the next one going out on its own, once
/// auto-send is armed.
///
/// Deliberately not zero. Between waves is when towers get bought, placed and
/// upgraded, and that is most of the game's decision-making — sending instantly
/// would take that away rather than automate it. Three seconds is enough to
/// place a tower or buy an upgrade without turning the gap into dead time, and
/// Space still overrides it for anyone who wants the wave now.
const AUTO_WAVE_DELAY: f32 = 3.0;
/// How far apart a multi-shot volley's projectiles leave the tower, measured
/// across its face. Only so three seeds at one fruit are visibly three.
const VOLLEY_SPREAD: f32 = 7.0;
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
        // Routes, scenery, the shop column and every hit-test are authored
        // against a fixed PLAYFIELD_W x PLAYFIELD_H space plus the panel beside
        // it. Resizing would pull the drawing and the hit-testing apart, so the
        // window is pinned to exactly what the layout was built for.
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
    /// Fast forward: the world takes two steps a frame instead of one.
    fast: bool,
    /// Expanding rings where shells have landed. Cosmetic.
    blasts: Vec<Blast>,
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
    /// Whether the run is held. Input keeps working while it is — the point of
    /// pausing is to look at the board and spend money, not to stop doing so.
    paused: bool,
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
            fast: false,
            blasts: Vec::new(),
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
            paused: false,
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
        self.blasts.clear();
        self.spikes.clear();
        self.queue.clear();
        self.wave_active = false;
        self.selected = None;
        self.selected_tower = None;
        self.quit_armed = 0.0;
        self.next_lane = 0;
        self.paused = false;
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

        for i in 0..render::card_count() {
            let picked_by_key = NUMBER_KEYS.get(i).is_some_and(|&k| is_key_pressed(k));
            let picked_by_click = clicked && render::track_card_rect(i).contains(m);
            if !(picked_by_key || picked_by_click) {
                continue;
            }

            // The last card isn't a route, it's a way of not choosing one.
            let track = if i == render::random_card_index() {
                gen_range(0, tracks::TRACKS.len())
            } else {
                i
            };
            self.start_run(track);
            return;
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

        // Held. Input above this line already ran, so towers can still be
        // bought, upgraded and sold while the wave stands still — that is what
        // pausing is for. Nothing below advances: not the spawn clock, not
        // cooldowns, not the auto-send timer.
        if self.paused {
            return;
        }

        // Real seconds, not world ones. Arming the quit button is a promise to
        // the hand holding the mouse, and fast-forwarding the game should not
        // make it lapse sooner.
        if self.quit_armed > 0.0 {
            self.quit_armed -= dt;
        }

        // Fast forward runs the world twice at the ordinary step rather than
        // once at double it, which sounds equivalent and is not. Collision here
        // is a test of where things are *this frame*, not of the line between
        // frames: a spike pile covers the fruit's radius plus 14px along the
        // track, so a blueberry at the top of the speed ramp already crosses
        // 22px of that 25px window in a single clamped frame. Double the step
        // and it crosses 43px, clean over the pile, and spikes quietly stop
        // working at speed. Two ordinary steps keep every window the size it
        // was.
        for _ in 0..self.world_steps() {
            self.step_world(dt);
            // A step can end the run or the route; the second one must not run
            // on into a board that has already been settled.
            if self.state != State::Playing {
                return;
            }
        }
    }

    /// How many world steps a frame runs: two while fast forward is on.
    fn world_steps(&self) -> u32 {
        if self.fast {
            FAST_FORWARD_STEPS
        } else {
            1
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // One step of the world. Everything here advances by `dt`; nothing in it
    // reads input, so it is safe to run more than once in a frame.
    // ─────────────────────────────────────────────────────────────────────────
    fn step_world(&mut self, dt: f32) {
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
        // Zipped rather than indexed: there are more number keys than towers,
        // because the route picker needs one for its random card, and indexing
        // ALL by key position would panic the moment that key was pressed here.
        for (key, kind) in NUMBER_KEYS.iter().zip(TowerKind::ALL.iter()) {
            if is_key_pressed(*key) {
                self.toggle_selection(*kind);
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
            } else if render::pause_button_rect().contains(m) {
                self.paused = !self.paused;
                self.audio.play_place();
            } else if render::fast_button_rect().contains(m) {
                self.fast = !self.fast;
                self.audio.play_place();
            } else if m.x >= PLAYFIELD_W {
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
            || p.x > PLAYFIELD_W - TOWER_RADIUS
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
                let fruit = Fruit::new(
                    kind,
                    lane,
                    0.0,
                    &self.paths[lane],
                    wave::speed_multiplier(self.wave, mode::mode(self.mode)),
                );

                // Shields are rolled per fruit rather than allotted per wave, so
                // a share of 0.3 is roughly three in ten rather than exactly
                // three — the player reads the field, not a quota. A boss is
                // never shielded: it already has 60 points of armour, and the
                // two together would need a specific board rather than a good
                // one.
                let shielded = kind != FruitKind::Durian
                    && gen_range(0.0, 1.0) < wave::shield_share(self.wave);

                self.fruits
                    .push(if shielded { fruit.with_shield() } else { fruit });
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
                // A Spike Layer lays for as long as the wave runs, and only
                // then. Its timer still ticks on the build screen, so the first
                // pile goes down the instant the wave starts, but nothing is
                // laid before that: a tower left alone between waves would
                // otherwise carpet its stretch before a fruit ever walked it.
                if !self.wave_active {
                    continue;
                }

                let spots = spike_spots(&self.paths, t.pos, range, &self.spikes);
                if spots.is_empty() {
                    // Every lane it reaches is already packed as densely as
                    // SPIKE_SPACING allows. The cooldown isn't spent, so it lays
                    // again the moment the fruit chew a gap open.
                    continue;
                }

                let (lane, dist, pos) = spots[gen_range(0, spots.len() as u32) as usize];
                self.spikes.push(SpikePile::new(
                    pos,
                    lane,
                    dist,
                    t.spike_charges(),
                    t.id,
                    gen_range(0.0, 360.0),
                    t.breaks_shield(),
                ));
                t.cooldown = t.fire_cooldown();
                t.shots_fired += 1;
                self.audio.play_spikes();
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
                TowerKind::BombLobber => ProjectileKind::Shell,
                _ => ProjectileKind::Seed,
            };
            let splash = t.splash_radius();
            let breaks_shield = t.breaks_shield();

            if t.kind == TowerKind::BombLobber {
                sort_by_crowd(&mut in_range, splash);
            }
            let pierce = t.pierce();

            // The whole volley goes out every time. A Triple Seeder throws
            // three seeds whether there are three fruit in range or one.
            let volley = t.shots();
            for k in volley_targets(volley, in_range.len()) {
                let (_, pos, speed, lane, dist) = in_range[k];

                // Aim where the fruit will be, not where it is. Shots don't
                // home, so without leading, the late-wave speed ramp would make
                // towers miss almost everything through no fault of the player.
                // The lead runs along the target's own lane.
                let travel = pos.distance(t.pos) / projectile_kind.speed();
                let path = self.paths.get(lane).unwrap_or(&self.paths[0]);
                let aim = path.point_at(dist + speed * travel);

                // Fan the volley across the tower's face so three seeds at one
                // target read as three, instead of stacking into one sprite.
                let dir = (aim - t.pos).normalize_or_zero();
                let perp = vec2(-dir.y, dir.x);
                let offset = k as f32 - (volley as f32 - 1.0) * 0.5;

                self.projectiles.push(Projectile::new(
                    t.pos + perp * offset * VOLLEY_SPREAD,
                    aim,
                    projectile_kind,
                    splash,
                    pierce,
                    t.id,
                    breaks_shield,
                ));
                t.shots_fired += 1;
            }

            let lead = in_range[0].1 - t.pos;
            t.angle = lead.y.atan2(lead.x);
            t.cooldown = t.fire_cooldown();
            match t.kind {
                TowerKind::KnifeThrower => self.audio.play_knife(),
                // The lob is the soft part; the boom belongs to the landing,
                // and plays from the impact path with the rest of the blast.
                TowerKind::BombLobber => self.audio.play_lob(),
                _ => self.audio.play_shoot(),
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

        // (fruit index, id of the tower that gets the credit, whether the hit
        // gets through a shield)
        let mut hits: Vec<Hit> = Vec::new();
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
                        hits.push((i, p.owner, p.breaks_shield));
                    }
                }
                // A shell landing is the loudest thing on the field; pulp is a
                // wet slap. Same code path, different weight of event.
                if p.kind == ProjectileKind::Shell {
                    self.audio.play_boom();
                    self.blasts.push(Blast::new(center, splash));
                } else {
                    self.audio.play_splash();
                }
            } else {
                hits.push((fi, p.owner, p.breaks_shield));
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
    fn apply_hits(&mut self, hits: Vec<Hit>) {
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

        for b in &mut self.blasts {
            b.update(dt);
        }
        self.blasts.retain(|b| !b.finished());
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

        // Spikes are a within-wave resource: whatever a Spike Layer still has
        // standing when the field clears is swept, rather than banked toward the
        // next wave. Paired with the tower only laying while a wave runs, that
        // makes every wave start on bare track, so what a Spike Layer is worth
        // is what it can build during the wave — not how long it was left alone
        // beforehand.
        self.spikes.clear();

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
        // Everything below is drawn in the view's coordinates, whatever size the
        // real surface is; begin_view scales them onto it.
        render::begin_view();
        self.draw_view();
        render::end_view();
    }

    fn draw_view(&self) {
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
            // Over the shots, under the splats: the blast is the event, and the
            // fruit bursting inside it should still read on top of it.
            for b in &self.blasts {
                render::draw_blast(b);
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
                    if m.x < PLAYFIELD_W {
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
            render::draw_pause_button(self.paused);
            render::draw_fast_button(self.fast);
            render::draw_auto_button(self.auto_wave);
            render::draw_quit_button(self.quit_armed > 0.0);
            if self.paused {
                render::draw_paused_overlay();
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Which fruit each shot of a volley goes to, as indices into the in-range list
// ordered by threat.
//
// Cycles back to the front when there are fewer fruit in range than shots, so
// the whole volley is always fired. Taking one fruit per shot instead meant a
// multi-shot tower facing a lone fruit fired once — which made the Triple Seeder
// slower, dearer and strictly worse than the Seed Shooter it costs nearly three
// times as much as, and worst exactly where it should have shone: against a
// single armoured boss.
// ─────────────────────────────────────────────────────────────────────────────
fn volley_targets(shots: usize, available: usize) -> Vec<usize> {
    if available == 0 {
        return Vec::new();
    }
    (0..shots).map(|k| k % available).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Reorder a tower's targets so the fruit standing in the thickest crowd comes
// first, scoring each by how many others sit within `blast` of it.
//
// Only the Bomb Lobber uses this. Every other tower duels whatever is nearest
// its exit, which is right when a shot pops one fruit — but a shell is worth
// firing for what it takes off the track at once, and "first" targeting aims it
// at the fruit that has already outrun the crowd behind it, which is the worst
// shot on the board.
//
// The sort is stable and the list arrives in threat order, so between two
// equally crowded spots the one nearer its exit still wins, and a scattering of
// stragglers with no crowd at all leaves the leading fruit in front — the same
// choice every other tower would have made.
// ─────────────────────────────────────────────────────────────────────────────
fn sort_by_crowd<T: Copy>(targets: &mut Vec<(f32, Vec2, T, usize, f32)>, blast: f32) {
    let mut scored: Vec<(usize, _)> = targets
        .iter()
        .map(|a| {
            let caught = targets
                .iter()
                .filter(|b| b.1.distance(a.1) <= blast)
                .count();
            (caught, *a)
        })
        .collect();

    scored.sort_by_key(|x| std::cmp::Reverse(x.0));
    *targets = scored.into_iter().map(|(_, a)| a).collect();
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
fn land_hits(fruits: &mut [Fruit], hits: Vec<Hit>) -> Vec<(usize, u32)> {
    let mut burst: Vec<(usize, u32)> = Vec::new();

    for (i, owner, breaks_shield) in hits {
        // A stale index simply misses. Nothing produces one today, but the
        // alternative is a panic in the middle of a wave.
        if let Some(f) = fruits.get_mut(i) {
            // A shielded fruit turns away anything that cannot get through,
            // and the hit is simply lost — no damage, and no burst to report.
            if f.take_hit(breaks_shield) {
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
fn run_over_spikes(piles: &mut [SpikePile], fruits: &[Fruit]) -> Vec<Hit> {
    let mut hits = Vec::new();

    for (i, f) in fruits.iter().enumerate() {
        // Which of the covering piles pays is arbitrary — every spike is worth
        // the same — so it is simply the first one with any left.
        //
        // A shielded fruit only spends a spike from a pile that can actually
        // get through it. Charging one that cannot would have the shield
        // absorbing spikes rather than turning them away, and a single shielded
        // blueberry could then walk a Lv1 tower's whole stretch clean.
        let pile = piles.iter_mut().find(|p| {
            !p.spent() && p.covers(f.lane, f.dist, f.radius()) && (!f.shielded || p.breaks_shield)
        });

        if let Some(pile) = pile {
            pile.charges -= 1;
            hits.push((i, pile.owner, pile.breaks_shield));
        }
    }

    hits
}

// ─────────────────────────────────────────────────────────────────────────────
// Every spot where a Spike Layer at `tower` could lay its next pile, as
// (lane, distance along that lane, world position).
//
// Walks every lane collecting the points inside the tower's range that aren't
// already too close to an existing pile on that same lane. The caller picks one
// at random, so piles land scattered across everything the tower covers rather
// than filing in from one end — which is what taking the spot furthest along,
// as this used to, produced: the same tower laid the same handful of piles in
// the same places every wave.
//
// Pooling every lane's spots into one list also keeps a tower that reaches both
// gates of a two-entrance route even-handed. Each lane is represented in
// proportion to how much of it the tower actually covers, instead of one lane
// swallowing the drops because it happens to be longer.
//
// Spacing is only checked against piles on the same lane: two lanes running
// close together near the shared exit are still different track, and a pile on
// one does not block the other.
// ─────────────────────────────────────────────────────────────────────────────
fn spike_spots(
    paths: &[Path],
    tower: Vec2,
    range: f32,
    existing: &[SpikePile],
) -> Vec<(usize, f32, Vec2)> {
    const STEP: f32 = 12.0;

    let mut spots = Vec::new();

    for (lane, path) in paths.iter().enumerate() {
        let total = path.total();
        let mut d = 0.0;

        while d <= total {
            let p = path.point_at(d);
            let clear = existing
                .iter()
                .all(|s| s.lane != lane || (s.dist - d).abs() >= SPIKE_SPACING);

            if p.distance(tower) <= range && clear {
                spots.push((lane, d, p));
            }
            d += STEP;
        }
    }

    spots
}

// ─────────────────────────────────────────────────────────────────────────────
// Current pointer position, in view coordinates.
//
// Everything this file hit-tests — shop buttons, towers, the panel — is laid out
// in the view's space, so the pointer has to be converted out of surface pixels
// before any of it means anything. On the web the surface is whatever size the
// page gave the canvas, which is rarely the view's size and never is on a phone.
// ─────────────────────────────────────────────────────────────────────────────
fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    render::to_view(render::surface(), vec2(x, y))
}

// ─────────────────────────────────────────────────────────────────────────────
// True if a point has strayed well outside the playfield, used to retire shots
// that never hit anything.
// ─────────────────────────────────────────────────────────────────────────────
fn off_field(p: Vec2) -> bool {
    p.x < -50.0 || p.x > PLAYFIELD_W + 50.0 || p.y < -50.0 || p.y > PLAYFIELD_H + 50.0
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

        for (i, kind) in TowerKind::ALL.iter().enumerate() {
            let mut t = Tower::new(*kind, DEMO_TOWER_SPOTS[i], self.next_tower_id);
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
        // And shield a different one. These are the two overlays most easily
        // confused for each other, so the capture has to show them side by side
        // rather than stacked on one fruit.
        self.fruits[3].shielded = true;

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
                false,
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

    #[test]
    fn fast_forward_takes_two_ordinary_steps_rather_than_one_long_one() {
        // The distinction the implementation turns on, pinned as arithmetic
        // because nothing else can see it: a step is only safe if it is shorter
        // than the narrowest collision window it has to notice.
        //
        // A spike pile covers the fruit's radius plus SPIKE_RADIUS along the
        // track. The blueberry is the fastest and the smallest, so it has the
        // narrowest window and the longest stride — if any fruit can cross a
        // pile without being seen, it is that one.
        let window = fruit::FruitKind::Blueberry.radius() + SPIKE_RADIUS;
        let top_speed = fruit::FRUIT_BASE_SPEED
            * FruitKind::Blueberry.speed_scale()
            * mode::MODES.iter().map(|m| m.max_speed).fold(0.0, f32::max);

        // The frame clamp in main().
        const MAX_FRAME: f32 = 0.05;
        let ordinary = top_speed * MAX_FRAME;
        let doubled = ordinary * FAST_FORWARD_STEPS as f32;

        assert!(
            ordinary < window,
            "even an ordinary step ({ordinary:.0}px) outruns a {window:.0}px pile"
        );
        assert!(
            doubled > window,
            "a doubled step is {doubled:.0}px against a {window:.0}px window — if this \
             ever stops being true, scaling dt directly would be safe and this \
             whole arrangement could go"
        );
    }

    #[test]
    fn the_web_page_is_built_around_the_view_the_game_draws() {
        // The page decides the canvas's shape and nothing in Rust makes it agree
        // with the view. It stopped agreeing once already: when the shop moved
        // out of a bottom bar and into the right-hand column the window went
        // from 1000 to 1420 wide, the canvas stayed at 1200, and the column sat
        // off the edge of it — missing and unclickable in the browser while
        // every native build was correct. Nothing else can see this, because
        // every other test measures the layout against the constants rather
        // than against what the page actually hands the game.
        let html = include_str!("../web/index.html");
        let (w, h) = (render::VIEW_W, render::VIEW_H);

        for needle in [
            format!("width=\"{w}\""),
            format!("height=\"{h}\""),
            // Sized against the viewport, capped at the view's own width, and
            // held to the view's shape.
            format!("{w}px"),
            format!("aspect-ratio: {w} / {h}"),
            format!("calc(100vh * {w} / {h})"),
        ] {
            assert!(
                html.contains(&needle),
                "web/index.html has no `{needle}`: the page must be built \
                 around the {w}x{h} view render.rs draws"
            );
        }
    }

    #[test]
    fn the_web_page_never_transforms_the_canvas() {
        // The one thing the page must not do. macroquad sizes its drawing buffer
        // from clientWidth, which ignores CSS transforms, and maps input through
        // getBoundingClientRect(), which honours them — so a transformed canvas
        // renders at one scale and takes clicks at another. That shipped once
        // and made the shop unreachable in the browser.
        let html = include_str!("../web/index.html");
        let css = &html[..html.find("</style>").expect("the page has no stylesheet")];

        assert!(
            !css.contains("transform:"),
            "web/index.html transforms the canvas; size it with width/height"
        );
    }

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

    /// A pile from a Lv1 tower: it cannot get through a shield. The tests that
    /// care about shields build their own.
    fn pile_on(lane: usize, dist: f32, charges: u32, owner: u32) -> SpikePile {
        SpikePile::new(Vec2::ZERO, lane, dist, charges, owner, 0.0, false)
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
    fn a_shielded_fruit_walks_over_spikes_it_can_shrug_off_without_spending_one() {
        // The subtle half of the rule. A shield turns a hit away rather than
        // absorbing it, so a pile that cannot get through must not be worn down
        // by the fruit walking over it — otherwise one shielded blueberry
        // strips a Lv1 tower's whole stretch on its way past, and the shield
        // costs the player far more than the fruit is worth.
        let path = straight_path();
        let mut piles = vec![pile_at(500.0, 4, 7)];
        let fruits = vec![fruit_at(FruitKind::Lime, 500.0, &path).with_shield()];

        assert!(
            run_over_spikes(&mut piles, &fruits).is_empty(),
            "the shield was pierced"
        );
        assert_eq!(
            piles[0].charges, 4,
            "a spike was spent on a fruit it could not hurt"
        );
    }

    #[test]
    fn spikes_from_an_upgraded_layer_do_get_through_a_shield() {
        let path = straight_path();
        let mut piles = vec![SpikePile::new(Vec2::ZERO, 0, 500.0, 4, 7, 0.0, true)];
        let fruits = vec![fruit_at(FruitKind::Lime, 500.0, &path).with_shield()];

        assert_eq!(run_over_spikes(&mut piles, &fruits), vec![(0, 7, true)]);
        assert_eq!(piles[0].charges, 3, "the pile should have spent a spike");
    }

    #[test]
    fn a_shielded_fruit_ignores_a_hit_that_cannot_reach_it() {
        // End to end through land_hits: the hit is simply lost, and nothing is
        // reported as bursting.
        let path = straight_path();
        let mut fruits = vec![fruit_at(FruitKind::Lime, 100.0, &path).with_shield()];

        assert!(land_hits(&mut fruits, vec![(0, 1, false)]).is_empty());
        assert_eq!(land_hits(&mut fruits, vec![(0, 1, true)]), vec![(0, 1)]);
    }

    #[test]
    fn a_pop_is_credited_to_the_tower_that_dropped_the_pile() {
        let path = straight_path();
        let mut piles = vec![pile_at(200.0, 4, 11), pile_at(900.0, 4, 22)];
        let fruits = vec![fruit_at(FruitKind::Lime, 900.0, &path)];

        assert_eq!(run_over_spikes(&mut piles, &fruits), vec![(0, 22, false)]);
    }

    #[test]
    fn every_spike_spot_is_inside_range_and_clear_of_the_piles_already_down() {
        let paths = vec![straight_path()];
        let tower = vec2(600.0, 0.0);
        let existing = vec![pile_at(640.0, 4, 0)];

        let spots = spike_spots(&paths, tower, 120.0, &existing);
        assert!(
            !spots.is_empty(),
            "a tower over open track has somewhere to lay"
        );

        for (lane, dist, pos) in &spots {
            assert_eq!(*lane, 0, "only one lane to choose from");
            assert!(pos.distance(tower) <= 120.0, "spot fell outside the range");
            assert!(
                (dist - existing[0].dist).abs() >= SPIKE_SPACING,
                "spot crowded a pile already on the track"
            );
        }

        // Both sides of the tower are offered, so the random pick can scatter
        // piles across the whole stretch instead of filing in from one end.
        assert!(spots.iter().any(|(_, d, _)| *d < tower.x));
        assert!(spots.iter().any(|(_, d, _)| *d > tower.x));
    }

    #[test]
    fn a_spike_layer_whose_stretch_is_packed_has_nowhere_left_to_lay() {
        // The natural ceiling now that the pile allowance is gone: spots run out
        // once the covered track is full at SPIKE_SPACING, and the tower simply
        // waits for the fruit to chew a gap open.
        let paths = vec![straight_path()];
        let tower = vec2(600.0, 0.0);

        let existing: Vec<SpikePile> = (0..40)
            .map(|i| pile_at(600.0 - 130.0 + i as f32 * SPIKE_SPACING * 0.5, 4, 0))
            .collect();

        assert!(spike_spots(&paths, tower, 120.0, &existing).is_empty());
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

        assert_eq!(hits, vec![(1, 7, false)], "the wrong lane's fruit was hit");
        assert_eq!(piles[0].charges, 3, "exactly one spike spent");
    }

    #[test]
    fn a_spike_layer_covering_both_lanes_can_lay_on_either() {
        // A tower between the two lanes reaches both, so both must be on offer —
        // otherwise the random pick could only ever feed one of them.
        let paths = two_lanes();
        let tower = vec2(600.0, 30.0);

        let spots = spike_spots(&paths, tower, 120.0, &[]);

        for (lane, _, pos) in &spots {
            assert!(*lane < paths.len(), "offered a lane that does not exist");
            assert!(pos.distance(tower) <= 120.0, "spot fell outside the range");
        }
        assert!(spots.iter().any(|(lane, _, _)| *lane == 0));
        assert!(spots.iter().any(|(lane, _, _)| *lane == 1));
    }

    #[test]
    fn a_pile_on_one_lane_does_not_block_a_spot_on_the_other() {
        // Spacing is a per-lane rule. Two lanes that run close together near the
        // shared exit are still different track, so a pile on one must not stop
        // a Spike Layer laying at the same point along the other.
        let paths = two_lanes();
        let tower = vec2(600.0, 30.0);

        // Saturate lane 0 across the tower's whole reach.
        let existing: Vec<SpikePile> = (0..40)
            .map(|i| pile_on(0, 480.0 + i as f32 * SPIKE_SPACING * 0.5, 4, 0))
            .collect();

        let spots = spike_spots(&paths, tower, 120.0, &existing);

        assert!(!spots.is_empty(), "lane 1 was still wide open");
        assert!(
            spots.iter().all(|(lane, _, _)| *lane == 1),
            "a full lane 0 should leave only lane 1 on offer"
        );
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
    fn the_screenshot_demo_has_a_spot_for_every_tower() {
        assert_eq!(DEMO_TOWER_SPOTS.len(), TowerKind::ALL.len());
        for (i, p) in DEMO_TOWER_SPOTS.iter().enumerate() {
            assert!(
                p.x > 0.0 && p.x < PLAYFIELD_W && p.y > 0.0 && p.y < PLAYFIELD_H,
                "demo tower {i} is staged outside the field"
            );
        }
    }

    #[test]
    fn a_volley_always_fires_every_shot() {
        // Three fruit in range, three shots, three different targets.
        assert_eq!(volley_targets(3, 5), vec![0, 1, 2]);
        // Two fruit: the third shot comes back round to the leader.
        assert_eq!(volley_targets(3, 2), vec![0, 1, 0]);
        // One fruit — a boss, say — still takes all three.
        assert_eq!(volley_targets(3, 1), vec![0, 0, 0]);
        // Single-shot towers are unaffected.
        assert_eq!(volley_targets(1, 4), vec![0]);
        // Nothing in range fires nothing, rather than indexing an empty list.
        assert!(volley_targets(3, 0).is_empty());
    }

    /// A target as update_towers builds them: (progress, position, speed, lane,
    /// distance). Only progress and position matter to the crowd sort.
    fn target(progress: f32, at: Vec2) -> (f32, Vec2, f32, usize, f32) {
        (progress, at, 0.0, 0, 0.0)
    }

    #[test]
    fn a_shell_is_aimed_at_the_crowd_and_not_at_the_leader() {
        // The whole reason the Bomb Lobber targets differently. A straggler has
        // outrun the pack and is nearest the exit, so every other tower would
        // shoot it — and a shell spent on it clears exactly one fruit.
        let mut targets = vec![
            target(0.90, vec2(900.0, 100.0)), // the leader, alone
            target(0.40, vec2(300.0, 100.0)), // a cluster of three
            target(0.38, vec2(330.0, 110.0)),
            target(0.36, vec2(320.0, 130.0)),
        ];

        sort_by_crowd(&mut targets, 60.0);

        assert!(
            targets[0].1.x < 400.0,
            "the shell was aimed at the lone leader instead of the crowd"
        );
    }

    #[test]
    fn a_tie_between_equal_crowds_goes_to_whichever_is_nearer_its_exit() {
        // Two clusters of two, far apart. Neither is thicker, so the tiebreak
        // has to fall back on threat — which is the order the list arrives in,
        // and only survives because the sort is stable.
        let mut targets = vec![
            target(0.80, vec2(900.0, 100.0)),
            target(0.78, vec2(920.0, 110.0)),
            target(0.30, vec2(200.0, 100.0)),
            target(0.28, vec2(220.0, 110.0)),
        ];

        sort_by_crowd(&mut targets, 60.0);

        assert_eq!(
            targets[0].0, 0.80,
            "a tie on crowd size ignored which fruit was nearer its exit"
        );
    }

    #[test]
    fn with_nothing_bunched_up_a_shell_goes_at_the_leader_like_anything_else() {
        // Every fruit alone in its own blast: the crowd score is 1 across the
        // board, so the sort must leave the threat order it was given intact.
        let mut targets = vec![
            target(0.90, vec2(900.0, 100.0)),
            target(0.50, vec2(500.0, 100.0)),
            target(0.10, vec2(100.0, 100.0)),
        ];
        let before: Vec<f32> = targets.iter().map(|t| t.0).collect();

        sort_by_crowd(&mut targets, 60.0);

        let after: Vec<f32> = targets.iter().map(|t| t.0).collect();
        assert_eq!(before, after, "a scattered field was reordered anyway");
    }

    #[test]
    fn a_wider_blast_can_change_which_crowd_is_worth_hitting() {
        // Two pairs and one loose trio spread wider than a small blast can
        // cover. The upgrade that widens the blast is supposed to change the
        // tower's mind about where to aim, which is what makes it an upgrade
        // rather than a bigger number.
        let spread = vec![
            target(0.90, vec2(900.0, 100.0)),
            target(0.88, vec2(940.0, 100.0)),
            target(0.30, vec2(100.0, 100.0)),
            target(0.28, vec2(180.0, 100.0)),
            target(0.26, vec2(260.0, 100.0)),
        ];

        let mut tight = spread.clone();
        sort_by_crowd(&mut tight, 50.0);
        assert!(
            tight[0].1.x > 800.0,
            "a narrow blast should take the pair it can actually cover"
        );

        let mut wide = spread;
        sort_by_crowd(&mut wide, 100.0);
        assert!(
            wide[0].1.x < 400.0,
            "a wide blast should reach the loose trio and prefer it"
        );
    }

    #[test]
    fn a_multi_shot_tower_beats_a_single_shot_one_against_a_lone_target() {
        // The property that was broken: facing one fruit, a Triple Seeder must
        // put out more than a Seed Shooter, or its price buys nothing at all.
        let triple = volley_targets(TowerKind::TripleSeeder.shots(1), 1).len();
        let single = volley_targets(TowerKind::SeedShooter.shots(1), 1).len();
        assert!(
            triple > single,
            "Triple Seeder fires {triple} at a lone fruit, Seed Shooter {single}"
        );
    }

    #[test]
    fn one_hit_bursts_an_ordinary_fruit_and_the_shooter_is_credited() {
        let path = straight_path();
        let mut fruits = vec![
            fruit_at(FruitKind::Lime, 100.0, &path),
            fruit_at(FruitKind::Orange, 200.0, &path),
        ];

        assert_eq!(land_hits(&mut fruits, vec![(1, 42, false)]), vec![(1, 42)]);
    }

    #[test]
    fn a_boss_only_bursts_once_however_many_hits_land_together() {
        // The dangerous case: splash, pierce and several towers can all connect
        // with one boss in a single frame. Reporting the burst twice would have
        // the caller remove a second fruit that was never hit.
        let path = straight_path();
        let mut fruits = vec![fruit_at(FruitKind::Durian, 100.0, &path)];

        let armour = FruitKind::Durian.armour() as usize;
        let overkill: Vec<Hit> = (0..armour + 20).map(|_| (0, 7, false)).collect();

        assert_eq!(land_hits(&mut fruits, overkill), vec![(0, 7)]);
        assert_eq!(fruits[0].hp, 0);
    }

    #[test]
    fn a_boss_survives_a_batch_that_falls_short_of_its_armour() {
        let path = straight_path();
        let mut fruits = vec![fruit_at(FruitKind::Durian, 100.0, &path)];

        let armour = FruitKind::Durian.armour();
        let batch: Vec<Hit> = (0..armour - 1).map(|_| (0, 7, false)).collect();

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
        let burst = land_hits(
            &mut fruits,
            vec![
                (3, 1, false),
                (0, 2, false),
                (3, 3, false),
                (9, 4, false),
                (1, 5, false),
            ],
        );

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
            fruits[0].take_hit(true);
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
