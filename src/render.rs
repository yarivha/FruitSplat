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
use crate::scenery::{Palette, Prop, PropKind};
use crate::tower::{Pulse, SpikePile, Tower, TowerKind, TOWER_RADIUS};
use crate::tracks::TRACKS;
use crate::PLAYFIELD_H;

/// Number of bands used to fake the grass gradient.
const FIELD_BANDS: i32 = 40;

/// Track widths — the outer band is the dirt border, the inner the worn middle.
const TRACK_OUTER: f32 = 44.0;
const TRACK_INNER: f32 = 34.0;

/// Shop bar layout. Sized so all five tower buttons plus the hint column fit
/// across the window without wrapping — which is why the buttons use each
/// tower's short name rather than its full one.
const BTN_W: f32 = 150.0;
const BTN_H: f32 = 62.0;
const BTN_GAP: f32 = 10.0;
const BTN_X0: f32 = 16.0;

/// Floating tower-panel size.
const PANEL_W: f32 = 250.0;
const PANEL_H: f32 = 228.0;

/// Route-selection card layout.
const CARD_W: f32 = 220.0;
const CARD_H: f32 = 210.0;
const CARD_GAP: f32 = 16.0;
const CARD_Y: f32 = 296.0;
/// The coordinate space routes are authored in, used to scale the previews.
const AUTHOR_W: f32 = 1000.0;
const AUTHOR_H: f32 = 650.0;

// ─────────────────────────────────────────────────────────────────────────────
// Shading helpers
//
// macroquad has no gradient primitive and no clipping, so every bit of shading
// here is faked by stacking shapes: a dark base, a mid body, then progressively
// smaller layers offset toward the light. Light is treated as coming from the
// upper left throughout, so shadows and highlights stay consistent.
// ─────────────────────────────────────────────────────────────────────────────

/// Multiply a colour toward black. `k` of 0.6 keeps 60% of the brightness.
fn shade(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

/// Lerp a colour toward white by `t`.
fn tint(c: Color, t: f32) -> Color {
    Color::new(
        c.r + (1.0 - c.r) * t,
        c.g + (1.0 - c.g) * t,
        c.b + (1.0 - c.b) * t,
        c.a,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// A soft contact shadow. Stacked ellipses of decreasing alpha fake the blur
// that a single hard-edged ellipse would miss.
// ─────────────────────────────────────────────────────────────────────────────
fn soft_shadow(center: Vec2, rx: f32, ry: f32) {
    for i in 0..3 {
        let spread = 1.0 + i as f32 * 0.20;
        let alpha = 0.17 - i as f32 * 0.05;
        draw_ellipse(
            center.x,
            center.y,
            rx * spread,
            ry * spread,
            0.0,
            Color::new(0.0, 0.0, 0.0, alpha),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// A sphere lit from the upper left: dark rim, body, then two lit layers pulled
// toward the light. This is what gives fruit and tower bases their volume.
// ─────────────────────────────────────────────────────────────────────────────
fn shaded_ball(center: Vec2, r: f32, base: Color) {
    draw_circle(center.x, center.y, r, shade(base, 0.55));
    draw_circle(center.x, center.y, r * 0.93, base);
    draw_circle(
        center.x - r * 0.09,
        center.y - r * 0.11,
        r * 0.76,
        tint(base, 0.10),
    );
    draw_circle(
        center.x - r * 0.15,
        center.y - r * 0.19,
        r * 0.52,
        tint(base, 0.22),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Two-stage specular: a soft bloom with a tight hot spot inside it, which reads
// as gloss far better than the single flat blob this replaced.
// ─────────────────────────────────────────────────────────────────────────────
fn specular(center: Vec2, r: f32) {
    draw_circle(
        center.x - r * 0.33,
        center.y - r * 0.37,
        r * 0.22,
        Color::new(1.0, 1.0, 1.0, 0.28),
    );
    draw_circle(
        center.x - r * 0.36,
        center.y - r * 0.40,
        r * 0.10,
        Color::new(1.0, 1.0, 1.0, 0.70),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Paint the grass gradient behind the whole playfield.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_background(p: &Palette) {
    let w = screen_width();
    let band_h = PLAYFIELD_H / FIELD_BANDS as f32;

    for i in 0..FIELD_BANDS {
        let t = i as f32 / (FIELD_BANDS - 1) as f32;
        let c = Color::new(
            p.grass_top.r + (p.grass_bottom.r - p.grass_top.r) * t,
            p.grass_top.g + (p.grass_bottom.g - p.grass_top.g) * t,
            p.grass_top.b + (p.grass_bottom.b - p.grass_top.b) * t,
            1.0,
        );
        // Overdraw each band by a pixel so seams never show at odd heights.
        draw_rectangle(0.0, i as f32 * band_h, w, band_h + 1.0, c);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw the route's decorative props. They arrive already sorted back to front,
// so this just walks the list.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_scenery(props: &[Prop], p: &Palette) {
    for prop in props {
        draw_prop(prop, p);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// One piece of scenery. Sizes are all relative to the prop's scale so the same
// code covers the small and large variants.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_prop(prop: &Prop, palette: &Palette) {
    let c = prop.pos;
    let s = prop.scale;
    // Per-prop jitter keeps a stand of trees from looking stamped out.
    let leaf = tint(shade(palette.foliage, 1.0 - prop.shade), prop.shade.max(0.0));

    match prop.kind {
        PropKind::Tree => {
            let h = 30.0 * s;
            soft_shadow(vec2(c.x + 4.0 * s, c.y + 9.0 * s), 15.0 * s, 6.5 * s);

            // Trunk, tapering slightly toward the canopy.
            let trunk = Color::new(0.36, 0.26, 0.17, 1.0);
            draw_line(c.x, c.y + 6.0 * s, c.x, c.y - h * 0.35, 5.5 * s, trunk);
            draw_line(
                c.x - 1.0 * s,
                c.y + 6.0 * s,
                c.x - 1.0 * s,
                c.y - h * 0.30,
                2.0 * s,
                tint(trunk, 0.18),
            );

            // Canopy: three overlapping blobs read as foliage more than one
            // circle does.
            let top = vec2(c.x, c.y - h * 0.62);
            draw_circle(top.x - 9.0 * s, top.y + 5.0 * s, 12.5 * s, shade(leaf, 0.82));
            draw_circle(top.x + 9.0 * s, top.y + 4.0 * s, 11.5 * s, shade(leaf, 0.88));
            draw_circle(top.x, top.y - 2.0 * s, 14.5 * s, leaf);
            draw_circle(top.x - 4.0 * s, top.y - 6.0 * s, 8.0 * s, tint(leaf, 0.16));
        }

        PropKind::Bush => {
            soft_shadow(vec2(c.x + 2.0 * s, c.y + 5.0 * s), 10.0 * s, 4.0 * s);
            draw_circle(c.x - 6.0 * s, c.y, 7.5 * s, shade(leaf, 0.84));
            draw_circle(c.x + 6.0 * s, c.y + 1.0 * s, 7.0 * s, shade(leaf, 0.90));
            draw_circle(c.x, c.y - 3.0 * s, 9.5 * s, leaf);
            draw_circle(c.x - 3.0 * s, c.y - 6.0 * s, 4.5 * s, tint(leaf, 0.18));
        }

        PropKind::Rock => {
            soft_shadow(vec2(c.x + 2.0 * s, c.y + 4.0 * s), 9.0 * s, 3.5 * s);
            let stone = Color::new(0.55, 0.54, 0.52, 1.0);
            draw_poly(c.x, c.y, 6, 9.0 * s, prop.angle.to_degrees(), shade(stone, 0.78));
            draw_poly(
                c.x - 1.5 * s,
                c.y - 2.0 * s,
                6,
                6.5 * s,
                prop.angle.to_degrees() + 12.0,
                stone,
            );
            draw_circle(c.x - 3.0 * s, c.y - 4.0 * s, 2.4 * s, tint(stone, 0.30));
        }

        PropKind::Flowers => {
            // A little clump of stems with coloured heads.
            let petal = [
                Color::new(0.95, 0.85, 0.35, 1.0),
                Color::new(0.92, 0.52, 0.66, 1.0),
                Color::new(0.80, 0.80, 0.95, 1.0),
            ][(prop.angle * 3.0) as usize % 3];

            for i in 0..3 {
                let dx = (i as f32 - 1.0) * 6.0 * s;
                let top = c.y - 8.0 * s - (i % 2) as f32 * 3.0 * s;
                draw_line(c.x + dx, c.y + 3.0 * s, c.x + dx, top, 1.6 * s, shade(leaf, 1.1));
                draw_circle(c.x + dx, top, 3.2 * s, petal);
                draw_circle(c.x + dx, top, 1.3 * s, tint(petal, 0.55));
            }
        }

        PropKind::Crate => {
            soft_shadow(vec2(c.x + 3.0 * s, c.y + 8.0 * s), 12.0 * s, 4.5 * s);
            let wood = Color::new(0.62, 0.45, 0.26, 1.0);
            let w = 20.0 * s;
            let h = 16.0 * s;

            draw_rectangle(c.x - w * 0.5, c.y - h * 0.5, w, h, shade(wood, 0.72));
            draw_rectangle(
                c.x - w * 0.5 + 1.5 * s,
                c.y - h * 0.5 + 1.5 * s,
                w - 3.0 * s,
                h - 3.0 * s,
                wood,
            );
            // Slats.
            draw_line(
                c.x - w * 0.5,
                c.y - h * 0.12,
                c.x + w * 0.5,
                c.y - h * 0.12,
                1.6 * s,
                shade(wood, 0.70),
            );
            draw_line(c.x, c.y - h * 0.5, c.x, c.y + h * 0.5, 1.6 * s, shade(wood, 0.70));
        }

        PropKind::Fence => {
            let wood = Color::new(0.58, 0.46, 0.32, 1.0);
            let span = 26.0 * s;
            soft_shadow(vec2(c.x + 2.0 * s, c.y + 7.0 * s), span * 0.55, 3.5 * s);

            // Two rails across three posts.
            for rail in 0..2 {
                let ry = c.y - 3.0 * s - rail as f32 * 6.0 * s;
                draw_line(c.x - span * 0.5, ry, c.x + span * 0.5, ry, 2.6 * s, wood);
            }
            for post in 0..3 {
                let px = c.x - span * 0.5 + post as f32 * span * 0.5;
                draw_line(px, c.y + 5.0 * s, px, c.y - 12.0 * s, 3.2 * s, shade(wood, 0.82));
            }
        }

        PropKind::Pond => {
            let w = 46.0 * s;
            let h = 26.0 * s;
            // Muddy bank, then water, then a lighter shallow edge.
            draw_ellipse(c.x, c.y, w, h, 0.0, Color::new(0.40, 0.36, 0.26, 1.0));
            draw_ellipse(
                c.x,
                c.y,
                w * 0.90,
                h * 0.88,
                0.0,
                Color::new(0.22, 0.42, 0.58, 1.0),
            );
            draw_ellipse(
                c.x - w * 0.08,
                c.y - h * 0.14,
                w * 0.62,
                h * 0.52,
                0.0,
                Color::new(0.32, 0.56, 0.72, 1.0),
            );
            // A couple of glints on the surface.
            draw_ellipse(
                c.x - w * 0.22,
                c.y - h * 0.24,
                w * 0.20,
                h * 0.10,
                0.0,
                Color::new(0.85, 0.94, 1.0, 0.55),
            );
            draw_ellipse(
                c.x + w * 0.16,
                c.y + h * 0.20,
                w * 0.13,
                h * 0.07,
                0.0,
                Color::new(0.85, 0.94, 1.0, 0.32),
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw the track as a thick dirt polyline. Circles at the joints round off the
// corners, which macroquad's square line caps would otherwise leave notched.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_path(path: &Path, palette: &Palette) {
    for (width, color) in [
        (TRACK_OUTER, palette.track_border),
        (TRACK_INNER, palette.track_dirt),
    ] {
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
    let c = f.pos;
    let r = f.radius();
    let rot = f.rot.to_radians();
    let body = f.kind.body();
    let flesh = f.kind.flesh();
    let tau = std::f32::consts::TAU;

    // Contact shadow on the track, cast away from the upper-left light.
    soft_shadow(vec2(c.x + r * 0.14, c.y + r * 0.34), r * 0.82, r * 0.40);

    match f.kind {
        FruitKind::Durian => {
            // A heavy, matte, spiked husk. The spikes carry the whole
            // silhouette — at boss size a plain ball would just read as an
            // oversized watermelon, and the two must never be confused at
            // speed. They are deliberately sparse and narrow: packed tighter
            // than this they overlap into a solid ring, and the fruit goes
            // straight back to reading as a plain disc with a fringe.
            let husk = shade(body, 0.80);
            let dark = shade(body, 0.40);
            let lit = shade(body, 0.60);

            // Spikes go down first, so the husk covers their roots and they
            // read as growing out of it rather than pinned onto it.
            for i in 0..9 {
                let a = rot + i as f32 * tau / 9.0;
                let dir = vec2(a.cos(), a.sin());
                let perp = vec2(-dir.y, dir.x);
                let base = c + dir * r * 0.62;
                let tip = base + dir * r * 0.48;
                // Split down the spine into a lit face and a shadowed one, so
                // each spike reads as a cone rather than a flat triangle.
                draw_triangle(tip, base + perp * r * 0.13, base, lit);
                draw_triangle(tip, base - perp * r * 0.13, base, dark);
            }

            // Husk, built by hand rather than with shaded_ball: that helper
            // lifts the centre toward white, which on this khaki washes the
            // whole fruit out to a pale grey.
            draw_circle(c.x, c.y, r * 0.70, shade(husk, 0.60));
            draw_circle(c.x, c.y, r * 0.65, husk);
            draw_circle(c.x - r * 0.08, c.y - r * 0.10, r * 0.46, tint(husk, 0.08));

            // Seams between the husk's lobes.
            for i in 0..5 {
                let a = rot * 0.4 + i as f32 * tau / 5.0;
                draw_line(
                    c.x,
                    c.y,
                    c.x + a.cos() * r * 0.68,
                    c.y + a.sin() * r * 0.68,
                    r * 0.035,
                    shade(body, 0.44),
                );
            }

            // Studs across the near face, so the husk reads as a studded
            // sphere rather than a flat disc behind a fringe.
            for i in 0..8 {
                let a = -rot * 0.7 + i as f32 * tau / 8.0;
                let dir = vec2(a.cos(), a.sin());
                let perp = vec2(-dir.y, dir.x);
                let base = c + dir * r * 0.34;
                draw_triangle(
                    base + dir * r * 0.24,
                    base + perp * r * 0.075,
                    base - perp * r * 0.075,
                    lit,
                );
            }

            draw_ellipse(
                c.x + r * 0.06,
                c.y - r * 0.84,
                r * 0.08,
                r * 0.18,
                14.0,
                Color::new(0.34, 0.26, 0.13, 1.0),
            );
        }

        FruitKind::Watermelon => {
            // Drawn as a cut cross-section: dark rind, pale pith, red flesh.
            shaded_ball(c, r, body);
            draw_circle(c.x, c.y, r * 0.84, tint(body, 0.50));
            draw_circle(c.x, c.y, r * 0.75, shade(flesh, 0.82));
            draw_circle(c.x - r * 0.04, c.y - r * 0.05, r * 0.70, flesh);
            draw_circle(c.x - r * 0.13, c.y - r * 0.16, r * 0.44, tint(flesh, 0.13));

            // Seeds sit in the flesh, each turned to face the centre.
            for i in 0..6 {
                let a = rot + i as f32 * tau / 6.0;
                let d = r * 0.45;
                draw_ellipse(
                    c.x + a.cos() * d,
                    c.y + a.sin() * d,
                    r * 0.11,
                    r * 0.065,
                    a.to_degrees(),
                    Color::new(0.13, 0.09, 0.08, 1.0),
                );
            }
        }

        FruitKind::Orange => {
            shaded_ball(c, r, body);

            // Peel pores, scattered at two radii so it doesn't look like a ring.
            for i in 0..12 {
                let a = rot * 0.3 + i as f32 * tau / 12.0;
                let d = r * if i % 2 == 0 { 0.38 } else { 0.66 };
                draw_circle(
                    c.x + a.cos() * d,
                    c.y + a.sin() * d,
                    r * 0.045,
                    shade(body, 0.84),
                );
            }

            draw_circle(c.x, c.y - r * 0.84, r * 0.13, Color::new(0.40, 0.28, 0.15, 1.0));
            draw_leaf(vec2(c.x + r * 0.30, c.y - r * 0.86), r * 0.52, rot);
        }

        FruitKind::Lime => {
            // A cut lime: dark rind, thin pale pith, then wedges split by
            // membranes. The rind is deliberately darker than the flesh —
            // without that contrast the whole fruit reads as one pale disc.
            shaded_ball(c, r, shade(body, 0.72));
            draw_circle(c.x, c.y, r * 0.82, tint(body, 0.70));
            draw_circle(c.x, c.y, r * 0.74, shade(flesh, 0.88));
            draw_circle(c.x - r * 0.05, c.y - r * 0.07, r * 0.68, flesh);

            let membrane = tint(flesh, 0.55);
            for i in 0..8 {
                let a = rot + i as f32 * tau / 8.0;
                draw_line(
                    c.x,
                    c.y,
                    c.x + a.cos() * r * 0.74,
                    c.y + a.sin() * r * 0.74,
                    r * 0.05,
                    membrane,
                );
            }
            draw_circle(c.x, c.y, r * 0.08, membrane);
        }

        FruitKind::Strawberry => {
            // Strawberries are conical, so this is shoulders plus a tip rather
            // than the plain circle the other fruit use.
            let dark = shade(body, 0.58);
            draw_triangle(
                vec2(c.x - r * 0.86, c.y + r * 0.02),
                vec2(c.x + r * 0.86, c.y + r * 0.02),
                vec2(c.x, c.y + r * 1.04),
                dark,
            );
            draw_ellipse(c.x, c.y - r * 0.06, r * 0.90, r * 0.80, 0.0, dark);

            draw_triangle(
                vec2(c.x - r * 0.76, c.y - r * 0.02),
                vec2(c.x + r * 0.76, c.y - r * 0.02),
                vec2(c.x, c.y + r * 0.94),
                body,
            );
            draw_ellipse(c.x, c.y - r * 0.10, r * 0.80, r * 0.72, 0.0, body);
            draw_ellipse(
                c.x - r * 0.12,
                c.y - r * 0.22,
                r * 0.52,
                r * 0.44,
                0.0,
                tint(body, 0.16),
            );

            // Seeds in offset rows, following the taper toward the tip.
            let seed = Color::new(1.0, 0.94, 0.62, 0.95);
            for row in 0..4 {
                let ry = c.y - r * 0.42 + row as f32 * r * 0.38;
                let spread = r * (0.62 - row as f32 * 0.13);
                let count = 4 - row.min(2);
                for i in 0..count {
                    let t = if count == 1 {
                        0.5
                    } else {
                        i as f32 / (count - 1) as f32
                    };
                    let sx = c.x - spread + t * spread * 2.0;
                    draw_ellipse(sx, ry, r * 0.075, r * 0.05, 20.0, seed);
                }
            }

            // Calyx: five pointed leaves fanned across the top.
            let green = Color::new(0.30, 0.63, 0.28, 1.0);
            let base = vec2(c.x, c.y - r * 0.66);
            for i in 0..5 {
                let a = -std::f32::consts::FRAC_PI_2 + (i as f32 - 2.0) * 0.55;
                let dir = vec2(a.cos(), a.sin());
                let perp = vec2(-dir.y, dir.x);
                draw_triangle(
                    base + dir * r * 0.80,
                    base + perp * r * 0.17,
                    base - perp * r * 0.17,
                    green,
                );
            }
            draw_circle(base.x, base.y, r * 0.15, shade(green, 0.85));
        }

        FruitKind::Blueberry => {
            shaded_ball(c, r, body);

            // The dusty bloom on a real blueberry's skin.
            draw_circle(
                c.x - r * 0.10,
                c.y - r * 0.10,
                r * 0.70,
                Color::new(0.78, 0.82, 0.96, 0.15),
            );

            // Crown: a sunken dimple ringed by five little points.
            let dimple = vec2(c.x - r * 0.05, c.y - r * 0.28);
            draw_circle(dimple.x, dimple.y, r * 0.27, shade(body, 0.58));
            for i in 0..5 {
                let a = rot + i as f32 * tau / 5.0;
                let dir = vec2(a.cos(), a.sin());
                let perp = vec2(-dir.y, dir.x);
                draw_triangle(
                    dimple + dir * r * 0.36,
                    dimple + perp * r * 0.10,
                    dimple - perp * r * 0.10,
                    shade(body, 0.48),
                );
            }
            draw_circle(dimple.x, dimple.y, r * 0.10, shade(body, 0.40));
        }
    }

    // Every fruit but the durian is glossy. A durian husk is dry and matte, and
    // a highlight on it only made it read as one more shiny ball — the opposite
    // of what has to happen when a boss comes on screen.
    if f.kind != FruitKind::Durian {
        specular(c, r);
    }

    if f.chilled() {
        // A light frost wash with a brighter rim. Kept subtle — a heavier wash
        // washed pale fruit like the lime out into an unreadable blob.
        draw_circle(c.x, c.y, r, Color::new(0.55, 0.80, 1.0, 0.17));
        draw_circle_lines(c.x, c.y, r * 0.97, 2.0, Color::new(0.82, 0.95, 1.0, 0.60));
    }

    // Armour readout, bosses only and only once one has actually been hit. An
    // untouched boss shows nothing, so the bar appearing is itself the signal
    // that the fight has started.
    if f.kind.is_boss() && f.hp < f.kind.armour() {
        draw_armour_bar(c, r, f.health_fraction());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The armour bar carried above a boss. It runs green through amber to red as
// the husk comes apart, so colour alone says how the fight is going without the
// player having to judge the length of a bar across a busy field.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_armour_bar(center: Vec2, r: f32, fraction: f32) {
    const BAR_W: f32 = 64.0;
    const BAR_H: f32 = 7.0;

    let left = fraction.clamp(0.0, 1.0);
    let x = center.x - BAR_W * 0.5;
    let y = center.y - r - 16.0;

    draw_rectangle(
        x - 1.5,
        y - 1.5,
        BAR_W + 3.0,
        BAR_H + 3.0,
        Color::new(0.0, 0.0, 0.0, 0.55),
    );
    draw_rectangle(x, y, BAR_W, BAR_H, Color::new(0.22, 0.08, 0.08, 0.92));

    let fill = if left > 0.5 {
        Color::new(0.46, 0.86, 0.36, 1.0)
    } else if left > 0.25 {
        Color::new(0.98, 0.78, 0.28, 1.0)
    } else {
        Color::new(1.0, 0.36, 0.30, 1.0)
    };
    draw_rectangle(x, y, BAR_W * left, BAR_H, fill);
}

// ─────────────────────────────────────────────────────────────────────────────
// A small leaf with a centre vein, used as the garnish on oranges.
// `rot` gives it a slight living wobble rather than a fixed pose.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_leaf(at: Vec2, size: f32, rot: f32) {
    let green = Color::new(0.30, 0.63, 0.28, 1.0);
    let sway = rot.sin() * 0.18;

    let dir = vec2(0.82 + sway, -0.57);
    let perp = vec2(-dir.y, dir.x);
    let tip = at + dir * size * 1.5;

    draw_triangle(at, tip, at + perp * size * 0.46, green);
    draw_triangle(at, tip, at - perp * size * 0.46, shade(green, 0.82));
    draw_line(at.x, at.y, tip.x, tip.y, size * 0.10, shade(green, 0.68));
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a placed tower: base, rim, then the kind-specific head.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_tower(t: &Tower) {
    soft_shadow(
        vec2(t.pos.x + TOWER_RADIUS * 0.16, t.pos.y + TOWER_RADIUS * 0.52),
        TOWER_RADIUS * 0.95,
        TOWER_RADIUS * 0.44,
    );
    draw_tower_body(t.kind, t.pos, t.angle, TOWER_RADIUS);
    draw_level_pips(t);
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a tower at an arbitrary size, so the shop buttons can show the real
// artwork as their icon instead of a flat colour swatch. Everything is
// expressed as a fraction of `r`, which is what makes it scale down cleanly.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_tower_body(kind: TowerKind, pos: Vec2, angle: f32, r: f32) {
    let c = kind.color();
    let (x, y) = (pos.x, pos.y);

    // Stone footing, so a tower reads as standing on the ground.
    draw_ellipse(x, y + r * 0.44, r * 1.04, r * 0.48, 0.0, Color::new(0.26, 0.25, 0.23, 1.0));
    draw_ellipse(x, y + r * 0.36, r * 1.00, r * 0.46, 0.0, Color::new(0.47, 0.45, 0.42, 1.0));
    draw_ellipse(x, y + r * 0.30, r * 0.92, r * 0.40, 0.0, Color::new(0.58, 0.56, 0.52, 1.0));

    shaded_ball(pos, r, c);

    let dir = vec2(angle.cos(), angle.sin());
    let perp = vec2(-dir.y, dir.x);

    match kind {
        TowerKind::SeedShooter => {
            // A wooden barrel aimed at whatever it last fired on.
            let wood = Color::new(0.36, 0.25, 0.15, 1.0);
            let tip = pos + dir * (r + r * 0.50);
            draw_line(x, y, tip.x, tip.y, r * 0.44, wood);
            // A lit strip along the top edge of the barrel.
            let lit = pos - perp * r * 0.10;
            draw_line(
                lit.x,
                lit.y,
                tip.x - perp.x * r * 0.10,
                tip.y - perp.y * r * 0.10,
                r * 0.14,
                tint(wood, 0.25),
            );
            draw_circle(tip.x, tip.y, r * 0.16, shade(wood, 0.6));
            draw_circle(x, y, r * 0.40, tint(c, 0.30));
            draw_circle(x, y, r * 0.22, shade(c, 0.70));
        }

        TowerKind::Blender => {
            let steel = Color::new(0.92, 0.94, 0.97, 1.0);
            for i in 0..3 {
                let a = angle + i as f32 * std::f32::consts::TAU / 3.0;
                let d = vec2(a.cos(), a.sin());
                let p = vec2(-d.y, d.x);
                // Tapered blades rather than plain lines.
                draw_triangle(
                    pos + d * r * 0.92,
                    pos + p * r * 0.20,
                    pos - p * r * 0.20,
                    steel,
                );
                draw_triangle(
                    pos + d * r * 0.92,
                    pos + p * r * 0.20,
                    pos,
                    shade(steel, 0.80),
                );
            }
            draw_circle(x, y, r * 0.30, Color::new(0.40, 0.44, 0.50, 1.0));
            draw_circle(x - r * 0.07, y - r * 0.07, r * 0.14, tint(steel, 0.4));
        }

        TowerKind::Freezer => {
            // A six-armed snowflake with barbed tips.
            let ice = Color::new(0.96, 0.99, 1.0, 1.0);
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                let d = vec2(a.cos(), a.sin()) * r * 0.82;
                draw_line(x - d.x, y - d.y, x + d.x, y + d.y, r * 0.16, ice);
            }
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                let arm = pos + vec2(a.cos(), a.sin()) * r * 0.82;
                draw_circle(arm.x, arm.y, r * 0.11, ice);
            }
            draw_circle(x, y, r * 0.30, Color::new(0.72, 0.90, 1.0, 1.0));
            draw_circle(x - r * 0.08, y - r * 0.08, r * 0.13, WHITE);
        }

        TowerKind::SpikeLayer => {
            // A hopper of spikes: a dark drum with points poking out the top.
            let iron = Color::new(0.42, 0.40, 0.42, 1.0);
            draw_circle(x, y, r * 0.62, shade(iron, 0.78));
            draw_circle(x - r * 0.06, y - r * 0.08, r * 0.48, iron);

            for i in 0..5 {
                let a = -std::f32::consts::FRAC_PI_2 + (i as f32 - 2.0) * 0.42;
                let d = vec2(a.cos(), a.sin());
                let p = vec2(-d.y, d.x);
                let base = pos + d * r * 0.44;
                draw_triangle(
                    base + d * r * 0.62,
                    base + p * r * 0.13,
                    base - p * r * 0.13,
                    Color::new(0.86, 0.86, 0.90, 1.0),
                );
            }
            draw_circle(x - r * 0.10, y - r * 0.14, r * 0.16, tint(iron, 0.45));
        }

        TowerKind::KnifeThrower => {
            // A blade cocked in the direction of the last target.
            let steel = Color::new(0.90, 0.92, 0.96, 1.0);
            let tip = pos + dir * (r + r * 0.58);
            let base = pos + dir * r * 0.15;

            draw_triangle(tip, base + perp * r * 0.28, base - perp * r * 0.28, steel);
            // Darker lower facet, so the blade has an edge rather than reading flat.
            draw_triangle(tip, base - perp * r * 0.28, base, shade(steel, 0.72));

            // Crossguard, so it reads as a knife and not an arrow.
            let guard = pos + dir * r * 0.34;
            draw_line(
                guard.x - perp.x * r * 0.46,
                guard.y - perp.y * r * 0.46,
                guard.x + perp.x * r * 0.46,
                guard.y + perp.y * r * 0.46,
                r * 0.16,
                Color::new(0.28, 0.31, 0.38, 1.0),
            );
            draw_circle(x, y, r * 0.32, Color::new(0.24, 0.27, 0.34, 1.0));
            draw_circle(x - r * 0.08, y - r * 0.08, r * 0.13, tint(c, 0.45));
        }
    }

    specular(pos, r * 0.9);
}

// ─────────────────────────────────────────────────────────────────────────────
// The shop-button icon: the same artwork as a placed tower, drawn small and at
// a fixed jaunty angle so every icon poses the same way.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_tower_icon(kind: TowerKind, center: Vec2, r: f32) {
    draw_tower_body(kind, center, -0.62, r);
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
// Draw the spike piles sitting on the track. The number of visible spikes
// tracks the remaining charges, so a pile is visibly wearing down.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_spikes(piles: &[SpikePile]) {
    for pile in piles {
        let c = pile.pos;
        let base = pile.rot.to_radians();

        // A scuffed patch of track under the caltrops.
        draw_ellipse(c.x, c.y, 15.0, 11.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.18));

        let steel = Color::new(0.84, 0.85, 0.89, 1.0);
        let spikes = pile.charges.min(9);
        for i in 0..spikes {
            let a = base + i as f32 * std::f32::consts::TAU / spikes.max(1) as f32;
            let d = vec2(a.cos(), a.sin());
            let p = vec2(-d.y, d.x);
            let root = c + d * 2.5;

            draw_triangle(
                root + d * 9.5,
                root + p * 2.8,
                root - p * 2.8,
                shade(steel, 0.75),
            );
            draw_triangle(root + d * 9.5, root + p * 2.8, root, steel);
        }
        draw_circle(c.x, c.y, 3.4, Color::new(0.36, 0.34, 0.36, 1.0));
    }
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
pub fn draw_hud(
    lives: u32,
    cash: u32,
    wave: u32,
    total_waves: u32,
    wave_active: bool,
    muted: bool,
) {
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

    // Progress through the route, not just the current wave number.
    let wave_txt = format!("WAVE {wave}/{total_waves}");
    let dims = measure_text(&wave_txt, None, 30, 1.0);
    let wave_color = if wave == total_waves {
        // The final wave is worth flagging.
        Color::new(1.0, 0.78, 0.35, 1.0)
    } else {
        WHITE
    };
    draw_text(
        &wave_txt,
        screen_width() - dims.width - 20.0,
        35.0,
        30.0,
        wave_color,
    );

    if !wave_active {
        let prompt = if wave == total_waves {
            // Plain ASCII only: the default font has no glyph for an em dash
            // and renders it as a tofu box.
            format!("SPACE  -  send the FINAL wave ({wave} of {total_waves})")
        } else {
            format!("SPACE  -  send wave {wave} of {total_waves}")
        };
        text_center(
            &prompt,
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
        // The real tower artwork, drawn small, rather than a flat colour dot.
        // Barrels and blades reach about 1.6x the radius, so the text column
        // starts clear of that rather than being overlapped by it.
        let icon = vec2(r.x + 20.0, r.y + r.h * 0.5 - 1.0);
        draw_tower_icon(*kind, icon, 12.0);
        if !affordable {
            // Fade the icon toward the button background instead of redrawing
            // every layer of it at a lower alpha.
            draw_circle(icon.x, icon.y, 19.0, Color::new(bg.r, bg.g, bg.b, 0.62));
        }

        let text_alpha = if affordable { 1.0 } else { 0.45 };
        draw_text(
            format!("{}. {}", i + 1, kind.short_name()),
            r.x + 44.0,
            r.y + 25.0,
            16.0,
            Color::new(1.0, 1.0, 1.0, text_alpha),
        );
        draw_text(
            format!("${}  {}", kind.cost(), kind.blurb()),
            r.x + 44.0,
            r.y + 46.0,
            13.0,
            Color::new(0.85, 0.85, 0.9, text_alpha),
        );
    }

    let dim = Color::new(0.70, 0.70, 0.78, 1.0);
    let hint_x = BTN_X0 + TowerKind::ALL.len() as f32 * (BTN_W + BTN_GAP);
    draw_text("click to place", hint_x, PLAYFIELD_H + 30.0, 14.0, dim);
    draw_text("right-click cancels", hint_x, PLAYFIELD_H + 52.0, 14.0, dim);
    draw_text("click a tower for stats", hint_x, PLAYFIELD_H + 74.0, 14.0, dim);
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
    Rect::new(panel.x + 12.0, panel.y + 150.0, panel.w - 24.0, 32.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// The sell button inside a panel.
// ─────────────────────────────────────────────────────────────────────────────
pub fn panel_sell_button(panel: Rect) -> Rect {
    Rect::new(panel.x + 12.0, panel.y + 190.0, panel.w - 24.0, 28.0)
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
        TowerKind::SpikeLayer => [
            ("Spikes/pile", t.spike_charges().to_string()),
            ("Max piles", t.max_piles().to_string()),
            ("Dropped", t.shots_fired.to_string()),
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

    // What the money buys, on its own line above the button. Sharing the
    // button's line meant the two strings collided on longer labels.
    draw_text(
        t.upgrade_label(),
        b.x + 2.0,
        b.y - 10.0,
        15.0,
        Color::new(0.78, 0.78, 0.84, 1.0),
    );

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

        // Preview in the route's own colours, so the backdrops are comparable
        // before committing to a run.
        draw_track_preview(track.points, r, &crate::scenery::palette(i));

        draw_text(
            track.difficulty,
            r.x + 12.0,
            r.y + 190.0,
            19.0,
            difficulty_color(track.difficulty),
        );
        // Wave count rather than route length: how long the run is is what the
        // player is actually choosing between.
        let waves_txt = format!("{} waves", track.waves);
        let dims = measure_text(&waves_txt, None, 17, 1.0);
        draw_text(
            &waves_txt,
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
fn draw_track_preview(points: &[(f32, f32)], card: Rect, palette: &Palette) {
    let inner_x = card.x + 10.0;
    let inner_y = card.y + 40.0;
    let inner_w = card.w - 20.0;
    let inner_h = 130.0;

    // Ramp the preview the same way the real field is shaded. Two flat halves
    // left an obvious seam across the middle of every card.
    let bands = 10;
    let band_h = inner_h / bands as f32;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let c = Color::new(
            palette.grass_top.r + (palette.grass_bottom.r - palette.grass_top.r) * t,
            palette.grass_top.g + (palette.grass_bottom.g - palette.grass_top.g) * t,
            palette.grass_top.b + (palette.grass_bottom.b - palette.grass_top.b) * t,
            1.0,
        );
        draw_rectangle(
            inner_x,
            inner_y + i as f32 * band_h,
            inner_w,
            band_h + 1.0,
            c,
        );
    }

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
pub fn draw_game_over(wave: u32, total_waves: u32) {
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
        &format!("You held out to wave {wave} of {total_waves}"),
        cy + 6.0,
        36.0,
        WHITE,
    );
    text_center(
        "Click to pick a route",
        cy + 100.0,
        30.0,
        Color::new(1.0, 0.85, 0.4, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Shown when every wave on a route has been survived.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_victory(route: &str, total_waves: u32, lives: u32) {
    let cy = screen_height() * 0.42;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.10, 0.03, 0.58),
    );

    text_center("ROUTE CLEARED", cy - 60.0, 72.0, Color::new(0.72, 1.0, 0.72, 1.0));
    text_center(
        &format!("{route} survived, all {total_waves} waves"),
        cy + 6.0,
        34.0,
        WHITE,
    );
    text_center(
        &format!("{lives} lives remaining"),
        cy + 48.0,
        28.0,
        Color::new(1.0, 1.0, 1.0, 0.75),
    );
    text_center(
        "Click to pick another route",
        cy + 118.0,
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
        // Leave enough width for the longest hint string at its font size.
        assert!(
            hint_x <= WINDOW_W - 176.0,
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
