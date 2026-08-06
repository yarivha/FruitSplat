// =============================================================================
// render.rs — all drawing for Fruit Splat
//
// Every visual is generated procedurally from macroquad primitives, so the game
// ships with no image assets. Drawing is kept strictly separate from simulation:
// nothing in this file mutates game state.
// =============================================================================

use macroquad::prelude::*;

use crate::fruit::{Fruit, FruitKind, Splat};
use crate::path::Path;
use crate::projectile::{Projectile, ProjectileKind};
use crate::tower::{Pulse, Tower, TowerKind, TOWER_RADIUS};
use crate::tracks::TRACKS;
use crate::PLAYFIELD_H;

/// Grass colours for the vertical gradient behind the track.
const FIELD_TOP: Color = Color::new(0.36, 0.51, 0.31, 1.0);
const FIELD_BOTTOM: Color = Color::new(0.24, 0.38, 0.24, 1.0);
/// Number of bands used to fake the gradient.
const FIELD_BANDS: i32 = 40;

/// Track widths — the outer band is the dirt border, the inner the worn middle.
const TRACK_OUTER: f32 = 44.0;
const TRACK_INNER: f32 = 34.0;

/// Shop bar layout. Sized so all four tower buttons plus the hint column fit
/// across the window without wrapping.
const BTN_W: f32 = 178.0;
const BTN_H: f32 = 62.0;
const BTN_GAP: f32 = 12.0;
const BTN_X0: f32 = 16.0;

/// Floating tower-panel size.
const PANEL_W: f32 = 250.0;
const PANEL_H: f32 = 206.0;

/// Route-selection card layout.
const CARD_W: f32 = 220.0;
const CARD_H: f32 = 210.0;
const CARD_GAP: f32 = 16.0;
const CARD_Y: f32 = 296.0;
/// The coordinate space routes are authored in, used to scale the previews.
const AUTHOR_W: f32 = 1000.0;
const AUTHOR_H: f32 = 650.0;

// ─────────────────────────────────────────────────────────────────────────────
// Paint the grass gradient behind the whole playfield.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_background() {
    let w = screen_width();
    let band_h = PLAYFIELD_H / FIELD_BANDS as f32;

    for i in 0..FIELD_BANDS {
        let t = i as f32 / (FIELD_BANDS - 1) as f32;
        let c = Color::new(
            FIELD_TOP.r + (FIELD_BOTTOM.r - FIELD_TOP.r) * t,
            FIELD_TOP.g + (FIELD_BOTTOM.g - FIELD_TOP.g) * t,
            FIELD_TOP.b + (FIELD_BOTTOM.b - FIELD_TOP.b) * t,
            1.0,
        );
        // Overdraw each band by a pixel so seams never show at odd heights.
        draw_rectangle(0.0, i as f32 * band_h, w, band_h + 1.0, c);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw the track as a thick dirt polyline. Circles at the joints round off the
// corners, which macroquad's square line caps would otherwise leave notched.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_path(path: &Path) {
    let border = Color::new(0.38, 0.28, 0.18, 1.0);
    let dirt = Color::new(0.55, 0.43, 0.29, 1.0);

    for (width, color) in [(TRACK_OUTER, border), (TRACK_INNER, dirt)] {
        for w in path.points().windows(2) {
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, width, color);
        }
        for p in path.points() {
            draw_circle(p.x, p.y, width * 0.5, color);
        }
    }

    // Exit marker — the thing the player is defending.
    if let Some(end) = path.points().last() {
        draw_circle(end.x, end.y, 26.0, Color::new(0.85, 0.25, 0.25, 0.55));
        draw_circle_lines(end.x, end.y, 26.0, 3.0, Color::new(1.0, 0.85, 0.85, 0.9));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw one fruit: shared body + highlight, then per-kind garnish.
// Chilled fruit get a pale blue wash so the Freezer's effect is visible.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_fruit(f: &Fruit) {
    let (x, y) = (f.pos.x, f.pos.y);
    let r = f.radius();
    let rot = f.rot.to_radians();
    let body = f.kind.body();

    // Body, with a darkened rim to lift it off the background.
    draw_circle(x, y, r, body);
    draw_circle_lines(
        x,
        y,
        r,
        2.5,
        Color::new(body.r * 0.6, body.g * 0.6, body.b * 0.6, 1.0),
    );

    match f.kind {
        FruitKind::Watermelon => {
            // Green rind with a red core and a ring of seeds.
            draw_circle(x, y, r * 0.68, f.kind.flesh());
            for i in 0..5 {
                let a = rot + i as f32 * std::f32::consts::TAU / 5.0;
                draw_circle(
                    x + a.cos() * r * 0.42,
                    y + a.sin() * r * 0.42,
                    r * 0.075,
                    Color::new(0.15, 0.11, 0.10, 1.0),
                );
            }
        }
        FruitKind::Orange => {
            draw_circle_lines(x, y, r * 0.62, 2.0, Color::new(1.0, 0.78, 0.42, 0.55));
            draw_leaf(x, y - r * 0.9, r * 0.42, rot);
        }
        FruitKind::Lime => {
            // Cut-face wedge lines radiating from the centre.
            for i in 0..6 {
                let a = rot + i as f32 * std::f32::consts::TAU / 6.0;
                draw_line(
                    x,
                    y,
                    x + a.cos() * r * 0.74,
                    y + a.sin() * r * 0.74,
                    2.0,
                    Color::new(0.85, 0.96, 0.55, 0.7),
                );
            }
        }
        FruitKind::Strawberry => {
            for i in 0..9 {
                let a = rot + i as f32 * std::f32::consts::TAU / 9.0;
                let d = if i % 2 == 0 { 0.68 } else { 0.40 };
                draw_circle(
                    x + a.cos() * r * d,
                    y + a.sin() * r * d,
                    r * 0.08,
                    Color::new(1.0, 0.92, 0.55, 0.9),
                );
            }
            draw_leaf(x, y - r * 0.85, r * 0.5, rot);
        }
        FruitKind::Blueberry => {
            // The little five-pointed calyx on top of a real blueberry.
            draw_poly(
                x,
                y - r * 0.45,
                5,
                r * 0.34,
                f.rot,
                Color::new(0.18, 0.19, 0.42, 1.0),
            );
        }
    }

    // Specular highlight, drawn over the garnish.
    draw_circle(
        x - r * 0.32,
        y - r * 0.36,
        r * 0.26,
        Color::new(1.0, 1.0, 1.0, 0.22),
    );

    if f.chilled() {
        draw_circle(x, y, r, Color::new(0.55, 0.80, 1.0, 0.35));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// A small two-lobed leaf, used as the garnish on oranges and strawberries.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_leaf(x: f32, y: f32, size: f32, rot: f32) {
    let green = Color::new(0.29, 0.62, 0.28, 1.0);
    let (c, s) = (rot.cos() * 0.35, rot.sin() * 0.35);

    draw_triangle(
        vec2(x, y + size * 0.4),
        vec2(x - size + c * size, y - size * 0.3),
        vec2(x - size * 0.15, y - size * 0.55 + s * size),
        green,
    );
    draw_triangle(
        vec2(x, y + size * 0.4),
        vec2(x + size + c * size, y - size * 0.3),
        vec2(x + size * 0.15, y - size * 0.55 + s * size),
        green,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a placed tower: base, rim, then the kind-specific head.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_tower(t: &Tower) {
    let (x, y) = (t.pos.x, t.pos.y);
    let c = t.kind.color();

    // Ground shadow, so towers read as sitting on the grass.
    draw_circle(x + 2.0, y + 3.0, TOWER_RADIUS, Color::new(0.0, 0.0, 0.0, 0.22));
    draw_circle(x, y, TOWER_RADIUS, c);
    draw_circle_lines(
        x,
        y,
        TOWER_RADIUS,
        2.5,
        Color::new(c.r * 0.55, c.g * 0.55, c.b * 0.55, 1.0),
    );

    match t.kind {
        TowerKind::SeedShooter => {
            // A barrel pointing at whatever it last fired on.
            let dir = vec2(t.angle.cos(), t.angle.sin());
            let tip = t.pos + dir * (TOWER_RADIUS + 9.0);
            draw_line(x, y, tip.x, tip.y, 8.0, Color::new(0.35, 0.25, 0.15, 1.0));
            draw_circle(x, y, TOWER_RADIUS * 0.45, Color::new(0.78, 0.62, 0.40, 1.0));
        }
        TowerKind::Blender => {
            // Spinning blades, angled off the tower's facing.
            for i in 0..3 {
                let a = t.angle + i as f32 * std::f32::consts::TAU / 3.0;
                draw_line(
                    x,
                    y,
                    x + a.cos() * TOWER_RADIUS * 0.85,
                    y + a.sin() * TOWER_RADIUS * 0.85,
                    5.0,
                    Color::new(0.90, 0.93, 0.96, 1.0),
                );
            }
            draw_circle(x, y, TOWER_RADIUS * 0.3, Color::new(0.45, 0.49, 0.55, 1.0));
        }
        TowerKind::Freezer => {
            // A six-armed snowflake.
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                let d = vec2(a.cos(), a.sin()) * TOWER_RADIUS * 0.8;
                draw_line(x - d.x, y - d.y, x + d.x, y + d.y, 3.5, WHITE);
            }
            draw_circle(x, y, TOWER_RADIUS * 0.28, Color::new(0.85, 0.95, 1.0, 1.0));
        }
        TowerKind::KnifeThrower => {
            // A blade cocked in the direction of the last target.
            let dir = vec2(t.angle.cos(), t.angle.sin());
            let perp = vec2(-dir.y, dir.x);
            let tip = t.pos + dir * (TOWER_RADIUS + 11.0);
            let base = t.pos + dir * 3.0;

            draw_triangle(
                tip,
                base + perp * 5.5,
                base - perp * 5.5,
                Color::new(0.90, 0.92, 0.96, 1.0),
            );
            // Crossguard, so it reads as a knife rather than an arrow.
            let guard = t.pos + dir * 6.0;
            draw_line(
                guard.x - perp.x * 9.0,
                guard.y - perp.y * 9.0,
                guard.x + perp.x * 9.0,
                guard.y + perp.y * 9.0,
                3.0,
                Color::new(0.30, 0.33, 0.40, 1.0),
            );
            draw_circle(x, y, TOWER_RADIUS * 0.32, Color::new(0.26, 0.29, 0.36, 1.0));
        }
    }

    draw_level_pips(t);
}

// ─────────────────────────────────────────────────────────────────────────────
// Gold pips under a tower showing its level, so upgrades are visible on the
// field without having to click each tower.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_level_pips(t: &Tower) {
    if t.level <= 1 {
        return;
    }

    let gold = Color::new(1.0, 0.82, 0.30, 1.0);
    let pips = (t.level - 1) as i32;
    let spacing = 9.0;
    let start_x = t.pos.x - (pips - 1) as f32 * spacing * 0.5;
    let y = t.pos.y + TOWER_RADIUS + 7.0;

    for i in 0..pips {
        let x = start_x + i as f32 * spacing;
        draw_circle(x, y, 3.5, gold);
        draw_circle_lines(x, y, 3.5, 1.0, Color::new(0.35, 0.25, 0.05, 0.8));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Highlight the tower being inspected: its range footprint plus a ring.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_tower_selection(t: &Tower) {
    let accent = Color::new(1.0, 0.92, 0.55, 1.0);

    draw_circle(
        t.pos.x,
        t.pos.y,
        t.range(),
        Color::new(1.0, 1.0, 1.0, 0.08),
    );
    draw_circle_lines(t.pos.x, t.pos.y, t.range(), 2.0, accent);
    draw_circle_lines(t.pos.x, t.pos.y, TOWER_RADIUS + 5.0, 2.5, accent);
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw the placement preview under the cursor, tinted by whether it's legal.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_ghost(pos: Vec2, kind: TowerKind, valid: bool) {
    let tint = if valid {
        Color::new(0.4, 1.0, 0.5, 0.9)
    } else {
        Color::new(1.0, 0.35, 0.35, 0.9)
    };

    // Range footprint, so the player can judge coverage before committing.
    // Always the Lv1 range — that's what actually gets placed.
    let range = kind.range(1);
    draw_circle(
        pos.x,
        pos.y,
        range,
        Color::new(tint.r, tint.g, tint.b, 0.10),
    );
    draw_circle_lines(pos.x, pos.y, range, 2.0, tint);

    let c = kind.color();
    draw_circle(pos.x, pos.y, TOWER_RADIUS, Color::new(c.r, c.g, c.b, 0.55));
    draw_circle_lines(pos.x, pos.y, TOWER_RADIUS, 2.0, tint);
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a Freezer pulse as an expanding, fading ring.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_pulse(p: &Pulse) {
    let t = p.progress();
    let alpha = (1.0 - t) * 0.55;
    draw_circle_lines(
        p.pos.x,
        p.pos.y,
        p.max_radius * t,
        3.0,
        Color::new(0.75, 0.92, 1.0, alpha),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a shot in flight.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_projectile(p: &Projectile) {
    let c = p.kind.color();
    let r = p.kind.radius();

    // A knife tumbles end over end; the others are just round.
    if p.kind == ProjectileKind::Knife {
        let a = p.spin.to_radians();
        let dir = vec2(a.cos(), a.sin());
        let perp = vec2(-dir.y, dir.x);

        draw_triangle(
            p.pos + dir * r * 1.9,
            p.pos + perp * r * 0.6,
            p.pos - perp * r * 0.6,
            c,
        );
        draw_triangle(
            p.pos - dir * r * 1.9,
            p.pos + perp * r * 0.6,
            p.pos - perp * r * 0.6,
            Color::new(0.42, 0.46, 0.56, 1.0),
        );
        return;
    }

    draw_circle(p.pos.x, p.pos.y, r, c);
    draw_circle(
        p.pos.x - r * 0.3,
        p.pos.y - r * 0.3,
        r * 0.35,
        Color::new(1.0, 1.0, 1.0, 0.35),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a splat burst, fading each particle out over its remaining life.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_splat(s: &Splat) {
    for p in &s.particles {
        let fade = (p.life / p.max_life).clamp(0.0, 1.0);
        let c = Color::new(p.color.r, p.color.g, p.color.b, fade);
        // Shrink as well as fade so the pulp reads as drying up, not blinking out.
        draw_circle(p.pos.x, p.pos.y, p.radius * (0.4 + fade * 0.6), c);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-of-screen HUD: lives, cash, wave number, plus the send-wave prompt while
// the player is between waves.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_hud(lives: u32, cash: u32, wave: u32, wave_active: bool, muted: bool) {
    // Dark strip so white text stays readable over the grass.
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        52.0,
        Color::new(0.0, 0.0, 0.0, 0.35),
    );

    let lives_color = if lives <= 5 {
        Color::new(1.0, 0.42, 0.38, 1.0)
    } else {
        WHITE
    };
    draw_text(format!("LIVES {lives}"), 20.0, 35.0, 30.0, lives_color);
    draw_text(format!("${cash}"), 200.0, 35.0, 30.0, Color::new(1.0, 0.88, 0.45, 1.0));

    // Mute state doubles as the hint for the key that toggles it.
    let (mute_label, mute_color) = if muted {
        ("M  muted", Color::new(1.0, 0.55, 0.5, 1.0))
    } else {
        ("M  sound on", Color::new(0.62, 0.85, 0.68, 1.0))
    };
    draw_text(mute_label, 360.0, 34.0, 22.0, mute_color);

    let wave_txt = format!("WAVE {wave}");
    let dims = measure_text(&wave_txt, None, 30, 1.0);
    draw_text(&wave_txt, screen_width() - dims.width - 20.0, 35.0, 30.0, WHITE);

    if !wave_active {
        text_center(
            &format!("SPACE — send wave {wave}"),
            PLAYFIELD_H - 22.0,
            30.0,
            Color::new(1.0, 0.9, 0.5, 1.0),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of shop button `i`, so main.rs can hit-test clicks against the same
// layout this file draws.
// ─────────────────────────────────────────────────────────────────────────────
pub fn shop_button_rect(i: usize) -> Rect {
    Rect::new(
        BTN_X0 + i as f32 * (BTN_W + BTN_GAP),
        PLAYFIELD_H + 14.0,
        BTN_W,
        BTN_H,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Bottom shop bar: one button per tower, dimmed when it can't be afforded and
// outlined when it's the current selection.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_shop(selected: Option<TowerKind>, cash: u32) {
    draw_rectangle(
        0.0,
        PLAYFIELD_H,
        screen_width(),
        screen_height() - PLAYFIELD_H,
        Color::new(0.12, 0.12, 0.16, 1.0),
    );

    for (i, kind) in TowerKind::ALL.iter().enumerate() {
        let r = shop_button_rect(i);
        let affordable = cash >= kind.cost();
        let is_selected = selected == Some(*kind);

        let bg = if is_selected {
            Color::new(0.26, 0.30, 0.24, 1.0)
        } else if affordable {
            Color::new(0.20, 0.21, 0.26, 1.0)
        } else {
            Color::new(0.15, 0.15, 0.18, 1.0)
        };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);

        if is_selected {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, Color::new(0.6, 1.0, 0.6, 1.0));
        }

        // Colour swatch identifying the tower.
        let c = kind.color();
        let swatch = Color::new(c.r, c.g, c.b, if affordable { 1.0 } else { 0.4 });
        draw_circle(r.x + 20.0, r.y + r.h * 0.5, 12.0, swatch);

        let text_alpha = if affordable { 1.0 } else { 0.45 };
        draw_text(
            format!("{}. {}", i + 1, kind.name()),
            r.x + 38.0,
            r.y + 25.0,
            17.0,
            Color::new(1.0, 1.0, 1.0, text_alpha),
        );
        draw_text(
            format!("${}  {}", kind.cost(), kind.blurb()),
            r.x + 38.0,
            r.y + 46.0,
            14.0,
            Color::new(0.85, 0.85, 0.9, text_alpha),
        );
    }

    let dim = Color::new(0.70, 0.70, 0.78, 1.0);
    let hint_x = BTN_X0 + TowerKind::ALL.len() as f32 * (BTN_W + BTN_GAP);
    draw_text("click to place", hint_x, PLAYFIELD_H + 30.0, 16.0, dim);
    draw_text("right-click cancels", hint_x, PLAYFIELD_H + 52.0, 16.0, dim);
    draw_text("click a tower for stats", hint_x, PLAYFIELD_H + 74.0, 16.0, dim);
}

// ─────────────────────────────────────────────────────────────────────────────
// Where the floating tower panel sits for a tower at `tower_pos`.
//
// Prefers the right of the tower and flips to the left when that would run off
// the window, then clamps vertically so it always stays inside the playfield.
// main.rs hit-tests clicks against this same rect.
// ─────────────────────────────────────────────────────────────────────────────
pub fn tower_panel_rect(tower_pos: Vec2) -> Rect {
    let gap = TOWER_RADIUS + 14.0;

    let mut x = tower_pos.x + gap;
    if x + PANEL_W > screen_width() - 10.0 {
        x = tower_pos.x - gap - PANEL_W;
    }
    x = x.clamp(10.0, (screen_width() - PANEL_W - 10.0).max(10.0));

    let y = (tower_pos.y - PANEL_H * 0.5).clamp(10.0, PLAYFIELD_H - PANEL_H - 10.0);

    Rect::new(x, y, PANEL_W, PANEL_H)
}

// ─────────────────────────────────────────────────────────────────────────────
// The upgrade button inside a panel.
// ─────────────────────────────────────────────────────────────────────────────
pub fn panel_upgrade_button(panel: Rect) -> Rect {
    Rect::new(panel.x + 12.0, panel.y + 130.0, panel.w - 24.0, 32.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// The sell button inside a panel.
// ─────────────────────────────────────────────────────────────────────────────
pub fn panel_sell_button(panel: Rect) -> Rect {
    Rect::new(panel.x + 12.0, panel.y + 168.0, panel.w - 24.0, 28.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// The floating panel for the selected tower: what it is, how it's performing,
// and buttons to upgrade or sell it.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_tower_panel(t: &Tower, cash: u32) {
    let r = tower_panel_rect(t.pos);
    let mouse = mouse_vec();

    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.10, 0.11, 0.14, 0.95));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(1.0, 0.92, 0.55, 1.0));

    draw_text(
        format!("{}  Lv{}", t.kind.name(), t.level),
        r.x + 12.0,
        r.y + 26.0,
        21.0,
        WHITE,
    );
    draw_line(
        r.x + 12.0,
        r.y + 36.0,
        r.x + r.w - 12.0,
        r.y + 36.0,
        1.0,
        Color::new(0.4, 0.4, 0.48, 1.0),
    );

    draw_tower_stats(t, r);
    draw_upgrade_button(t, r, cash, mouse);
    draw_sell_button(t, r, mouse);
}

// ─────────────────────────────────────────────────────────────────────────────
// The stat rows: current numbers first, then this tower's running tally.
// A Freezer deals no damage, so it reports what it has chilled instead.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_tower_stats(t: &Tower, panel: Rect) {
    let rows: [(&str, String); 4] = match t.kind {
        TowerKind::Freezer => [
            ("Range", format!("{:.0}", t.range())),
            ("Chill", format!("{:.0}%", t.slow_factor() * 100.0)),
            ("Pulses", t.shots_fired.to_string()),
            ("Chilled", t.chills.to_string()),
        ],
        TowerKind::Blender => [
            ("Range", format!("{:.0}", t.range())),
            ("Splash", format!("{:.0}", t.splash_radius())),
            ("Shots", t.shots_fired.to_string()),
            ("Kills", t.kills.to_string()),
        ],
        TowerKind::KnifeThrower => [
            ("Range", format!("{:.0}", t.range())),
            ("Pierce", t.pierce().to_string()),
            ("Knives", t.shots_fired.to_string()),
            ("Kills", t.kills.to_string()),
        ],
        TowerKind::SeedShooter => [
            ("Range", format!("{:.0}", t.range())),
            ("Rate", format!("{:.2}s", t.fire_cooldown())),
            ("Shots", t.shots_fired.to_string()),
            ("Kills", t.kills.to_string()),
        ],
    };

    let label = Color::new(0.70, 0.70, 0.78, 1.0);
    for (i, (name, value)) in rows.iter().enumerate() {
        let y = panel.y + 58.0 + i as f32 * 21.0;
        draw_text(*name, panel.x + 14.0, y, 17.0, label);

        // Values are right-aligned so the column reads as a table.
        let dims = measure_text(value, None, 17, 1.0);
        draw_text(value, panel.x + panel.w - dims.width - 14.0, y, 17.0, WHITE);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Upgrade button: green when affordable, red when not, inert once maxed.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_upgrade_button(t: &Tower, panel: Rect, cash: u32, mouse: Vec2) {
    let b = panel_upgrade_button(panel);

    let Some(cost) = t.upgrade_cost() else {
        draw_rectangle(b.x, b.y, b.w, b.h, Color::new(0.16, 0.16, 0.19, 1.0));
        draw_text(
            "fully upgraded",
            b.x + 10.0,
            b.y + 21.0,
            18.0,
            Color::new(1.0, 0.85, 0.45, 1.0),
        );
        return;
    };

    let affordable = cash >= cost;
    let hovered = b.contains(mouse) && affordable;

    let bg = if !affordable {
        Color::new(0.20, 0.14, 0.14, 1.0)
    } else if hovered {
        Color::new(0.24, 0.40, 0.26, 1.0)
    } else {
        Color::new(0.18, 0.30, 0.20, 1.0)
    };
    let accent = if affordable {
        Color::new(0.60, 1.0, 0.65, 1.0)
    } else {
        Color::new(1.0, 0.50, 0.45, 1.0)
    };

    draw_rectangle(b.x, b.y, b.w, b.h, bg);
    draw_rectangle_lines(b.x, b.y, b.w, b.h, 2.0, accent);

    draw_text(format!("Upgrade ${cost}"), b.x + 10.0, b.y + 21.0, 18.0, accent);

    // The label for what the money buys, right-aligned against the button edge.
    let label = t.upgrade_label();
    let dims = measure_text(label, None, 14, 1.0);
    draw_text(
        label,
        b.x + b.w - dims.width - 10.0,
        b.y + 21.0,
        14.0,
        Color::new(0.78, 0.78, 0.84, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Sell button, showing what the refund is worth right now.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_sell_button(t: &Tower, panel: Rect, mouse: Vec2) {
    let b = panel_sell_button(panel);
    let hovered = b.contains(mouse);

    let bg = if hovered {
        Color::new(0.32, 0.22, 0.22, 1.0)
    } else {
        Color::new(0.22, 0.17, 0.17, 1.0)
    };
    draw_rectangle(b.x, b.y, b.w, b.h, bg);
    draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.5, Color::new(0.75, 0.55, 0.55, 1.0));

    draw_text(
        format!("Sell ${}", t.sell_value()),
        b.x + 10.0,
        b.y + 19.0,
        17.0,
        Color::new(0.95, 0.80, 0.78, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Current mouse position as a Vec2.
// ─────────────────────────────────────────────────────────────────────────────
fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}

// ─────────────────────────────────────────────────────────────────────────────
// Title screen.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_menu() {
    let cy = screen_height() * 0.42;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.5),
    );

    text_center("FRUIT SPLAT", cy - 60.0, 78.0, WHITE);
    text_center(
        "Build towers. Splat the fruit before it reaches the end.",
        cy + 6.0,
        28.0,
        Color::new(1.0, 1.0, 1.0, 0.85),
    );
    text_center(
        "Popped fruit bursts into two smaller, faster ones.",
        cy + 44.0,
        24.0,
        Color::new(1.0, 1.0, 1.0, 0.6),
    );
    text_center(
        "Click to start",
        cy + 120.0,
        32.0,
        Color::new(1.0, 0.85, 0.4, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of route card `i`, so main.rs can hit-test clicks against the same
// layout this file draws.
// ─────────────────────────────────────────────────────────────────────────────
pub fn track_card_rect(i: usize) -> Rect {
    let n = TRACKS.len() as f32;
    let total = n * CARD_W + (n - 1.0) * CARD_GAP;
    let x0 = (screen_width() - total) * 0.5;

    Rect::new(x0 + i as f32 * (CARD_W + CARD_GAP), CARD_Y, CARD_W, CARD_H)
}

// ─────────────────────────────────────────────────────────────────────────────
// Route selection screen: one card per track, each showing a scaled preview of
// the actual polyline the fruit will walk.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_track_select() {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.62),
    );

    text_center("CHOOSE YOUR ROUTE", 150.0, 62.0, WHITE);
    text_center(
        "Longer routes give your towers more time to shoot",
        200.0,
        26.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );

    let mouse = {
        let (x, y) = mouse_position();
        vec2(x, y)
    };

    for (i, track) in TRACKS.iter().enumerate() {
        let r = track_card_rect(i);
        let hovered = r.contains(mouse);

        let bg = if hovered {
            Color::new(0.24, 0.28, 0.24, 1.0)
        } else {
            Color::new(0.16, 0.17, 0.21, 1.0)
        };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            if hovered { 3.0 } else { 1.5 },
            if hovered {
                Color::new(0.7, 1.0, 0.7, 1.0)
            } else {
                Color::new(0.4, 0.4, 0.48, 1.0)
            },
        );

        draw_text(
            format!("{}. {}", i + 1, track.name),
            r.x + 12.0,
            r.y + 26.0,
            21.0,
            WHITE,
        );

        draw_track_preview(track.points, r);

        draw_text(
            track.difficulty,
            r.x + 12.0,
            r.y + 190.0,
            19.0,
            difficulty_color(track.difficulty),
        );
        let len_txt = format!("{} px", track.length() as i32);
        let dims = measure_text(&len_txt, None, 17, 1.0);
        draw_text(
            &len_txt,
            r.x + r.w - dims.width - 12.0,
            r.y + 190.0,
            17.0,
            Color::new(0.75, 0.75, 0.82, 1.0),
        );

        draw_text(
            track.blurb,
            r.x + 12.0,
            r.y + 168.0,
            15.0,
            Color::new(0.72, 0.72, 0.80, 1.0),
        );
    }

    text_center(
        "click a route, or press 1-4",
        CARD_Y + CARD_H + 48.0,
        26.0,
        Color::new(1.0, 0.85, 0.4, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a route's polyline scaled down to fit inside a selection card.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_track_preview(points: &[(f32, f32)], card: Rect) {
    let inner_x = card.x + 10.0;
    let inner_y = card.y + 40.0;
    let inner_w = card.w - 20.0;
    let inner_h = 130.0;

    draw_rectangle(
        inner_x,
        inner_y,
        inner_w,
        inner_h,
        Color::new(0.30, 0.42, 0.28, 1.0),
    );

    // Uniform scale keeps the route's shape honest rather than stretching it.
    let scale = (inner_w / AUTHOR_W).min(inner_h / AUTHOR_H);
    let to_card = |p: (f32, f32)| vec2(inner_x + p.0 * scale, inner_y + p.1 * scale);

    for w in points.windows(2) {
        let (a, b) = (to_card(w[0]), to_card(w[1]));
        draw_line(a.x, a.y, b.x, b.y, 5.0, Color::new(0.55, 0.43, 0.29, 1.0));
    }
    for &p in points {
        let c = to_card(p);
        draw_circle(c.x, c.y, 2.5, Color::new(0.55, 0.43, 0.29, 1.0));
    }

    // Mark the exit the player is defending.
    if let Some(&last) = points.last() {
        let e = to_card(last);
        draw_circle(e.x, e.y, 5.0, Color::new(0.85, 0.25, 0.25, 0.9));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Colour for a difficulty label.
// ─────────────────────────────────────────────────────────────────────────────
fn difficulty_color(difficulty: &str) -> Color {
    match difficulty {
        "Gentle" => Color::new(0.55, 0.95, 0.60, 1.0),
        "Hard" => Color::new(1.0, 0.50, 0.45, 1.0),
        _ => Color::new(1.0, 0.85, 0.45, 1.0),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// End screen, shown once the fruit have drained every life.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_game_over(wave: u32) {
    let cy = screen_height() * 0.42;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.55),
    );

    text_center("OVERRUN", cy - 60.0, 72.0, WHITE);
    text_center(
        &format!("You held out to wave {wave}"),
        cy + 6.0,
        36.0,
        WHITE,
    );
    text_center(
        "Click to try again",
        cy + 100.0,
        30.0,
        Color::new(1.0, 0.85, 0.4, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw text horizontally centred on the window at baseline `y`.
// ─────────────────────────────────────────────────────────────────────────────
fn text_center(text: &str, y: f32, size: f32, color: Color) {
    let dims = measure_text(text, None, size as u16, 1.0);
    draw_text(text, (screen_width() - dims.width) * 0.5, y, size, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixed window width the layout constants are tuned against.
    const WINDOW_W: f32 = 1000.0;

    #[test]
    fn every_shop_button_fits_inside_the_window() {
        for i in 0..TowerKind::ALL.len() {
            let r = shop_button_rect(i);
            assert!(
                r.x >= 0.0 && r.x + r.w <= WINDOW_W,
                "shop button {i} runs off the window"
            );
        }
    }

    #[test]
    fn shop_buttons_do_not_overlap() {
        for i in 1..TowerKind::ALL.len() {
            let prev = shop_button_rect(i - 1);
            let cur = shop_button_rect(i);
            assert!(
                cur.x >= prev.x + prev.w,
                "shop buttons {} and {i} overlap",
                i - 1
            );
        }
    }

    #[test]
    fn the_hint_column_clears_the_last_shop_button() {
        let last = shop_button_rect(TowerKind::ALL.len() - 1);
        let hint_x = BTN_X0 + TowerKind::ALL.len() as f32 * (BTN_W + BTN_GAP);

        assert!(
            hint_x >= last.x + last.w,
            "hint text overlaps the last shop button"
        );
        // Leave enough width for the longest hint string.
        assert!(
            hint_x <= WINDOW_W - 190.0,
            "no room left for the hint column"
        );
    }

    #[test]
    fn panel_buttons_stay_inside_the_panel() {
        let panel = Rect::new(0.0, 0.0, PANEL_W, PANEL_H);
        for b in [panel_upgrade_button(panel), panel_sell_button(panel)] {
            assert!(b.x >= panel.x && b.x + b.w <= panel.x + panel.w);
            assert!(b.y >= panel.y && b.y + b.h <= panel.y + panel.h);
        }
    }

    #[test]
    fn the_sell_button_sits_below_the_upgrade_button() {
        let panel = Rect::new(0.0, 0.0, PANEL_W, PANEL_H);
        let upgrade = panel_upgrade_button(panel);
        let sell = panel_sell_button(panel);
        assert!(sell.y >= upgrade.y + upgrade.h, "panel buttons overlap");
    }
}
