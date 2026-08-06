// =============================================================================
// render.rs — all drawing for Fruit Splat
//
// Every visual here is generated procedurally from macroquad primitives, so the
// game ships with no image assets at all. Keeps drawing separate from the
// simulation in `fruit`/`spawn` — nothing in this file mutates game state.
// =============================================================================

use macroquad::prelude::*;

use crate::fruit::{Fruit, FruitKind, Splat};

/// Sky colours for the vertical gradient, top and bottom.
const SKY_TOP: Color = Color::new(0.16, 0.20, 0.35, 1.0);
const SKY_BOTTOM: Color = Color::new(0.42, 0.30, 0.44, 1.0);
/// Number of bands used to fake the gradient.
const SKY_BANDS: i32 = 48;

// ─────────────────────────────────────────────────────────────────────────────
// Paint the dusk-sky gradient behind everything else.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_background() {
    let (w, h) = (screen_width(), screen_height());
    let band_h = h / SKY_BANDS as f32;

    for i in 0..SKY_BANDS {
        let t = i as f32 / (SKY_BANDS - 1) as f32;
        let c = Color::new(
            SKY_TOP.r + (SKY_BOTTOM.r - SKY_TOP.r) * t,
            SKY_TOP.g + (SKY_BOTTOM.g - SKY_TOP.g) * t,
            SKY_TOP.b + (SKY_BOTTOM.b - SKY_TOP.b) * t,
            1.0,
        );
        // Overdraw each band by a pixel so seams never show at odd heights.
        draw_rectangle(0.0, i as f32 * band_h, w, band_h + 1.0, c);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw one fruit: shared body + highlight, then per-kind garnish.
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
            // Dimpled peel suggested by a lighter inner ring, plus a leaf.
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
                    2.5,
                    Color::new(0.85, 0.96, 0.55, 0.7),
                );
            }
        }
        FruitKind::Strawberry => {
            // Pale seed speckle, then a leafy crown.
            for i in 0..9 {
                let a = rot + i as f32 * std::f32::consts::TAU / 9.0;
                let d = if i % 2 == 0 { 0.68 } else { 0.40 };
                draw_circle(
                    x + a.cos() * r * d,
                    y + a.sin() * r * d,
                    r * 0.06,
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

    // Specular highlight, drawn last so it sits over the garnish.
    draw_circle(
        x - r * 0.32,
        y - r * 0.36,
        r * 0.26,
        Color::new(1.0, 1.0, 1.0, 0.22),
    );
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
// In-round HUD: score and missed count on the left, countdown on the right.
// The clock turns red for the last ten seconds.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_hud(score: u32, missed: u32, time_left: f32) {
    draw_text(&format!("SCORE {score}"), 24.0, 46.0, 36.0, WHITE);
    draw_text(
        &format!("MISSED {missed}"),
        24.0,
        78.0,
        26.0,
        Color::new(1.0, 1.0, 1.0, 0.65),
    );

    let clock = format!("{:04.1}", time_left.max(0.0));
    let color = if time_left <= 10.0 {
        Color::new(1.0, 0.42, 0.38, 1.0)
    } else {
        WHITE
    };
    let dims = measure_text(&clock, None, 36, 1.0);
    draw_text(&clock, screen_width() - dims.width - 24.0, 46.0, 36.0, color);
}

// ─────────────────────────────────────────────────────────────────────────────
// Title screen.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_menu() {
    let cy = screen_height() * 0.5;
    text_center("FRUIT SPLAT", cy - 60.0, 78.0, WHITE);
    text_center(
        "Pop the fruit before it floats away",
        cy + 4.0,
        30.0,
        Color::new(1.0, 1.0, 1.0, 0.8),
    );
    text_center(
        "Smaller fruit rise faster and score more",
        cy + 42.0,
        24.0,
        Color::new(1.0, 1.0, 1.0, 0.55),
    );
    text_center(
        "Click to start",
        cy + 110.0,
        32.0,
        Color::new(1.0, 0.85, 0.4, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// End-of-round summary.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_game_over(score: u32, missed: u32) {
    let cy = screen_height() * 0.5;

    // Dim the playfield so the summary text stays readable over stray particles.
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.45),
    );

    text_center("TIME'S UP", cy - 70.0, 68.0, WHITE);
    text_center(&format!("Score {score}"), cy + 6.0, 44.0, WHITE);
    text_center(
        &format!("Missed {missed}"),
        cy + 50.0,
        28.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    text_center(
        "Click to play again",
        cy + 120.0,
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
