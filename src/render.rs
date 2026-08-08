// =============================================================================
// render.rs — all drawing for Fruit Splat
//
// Every visual is generated procedurally from macroquad primitives, so the game
// ships with no image assets. Drawing is kept strictly separate from simulation:
// nothing in this file mutates game state.
// =============================================================================

use macroquad::prelude::*;

use crate::fruit::{Fruit, FruitKind, Splat};
use crate::mode::MODES;
use crate::path::Path;
use crate::projectile::{Projectile, ProjectileKind};
use crate::scenery::{Palette, Prop, PropKind};
use crate::tower::{Pulse, SpikePile, Tower, TowerKind, TOWER_RADIUS};
use crate::tracks::TRACKS;
use crate::{PLAYFIELD_H, PLAYFIELD_W};

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

/// Audio toggles: effects first, then music. They sit in the gap between the
/// cash readout and the wave counter, which is the only part of the top strip
/// nothing else claims.
const AUDIO_BTN: f32 = 30.0;
const AUDIO_BTN_X0: f32 = 352.0;
const AUDIO_BTN_Y: f32 = 11.0;
const AUDIO_BTN_GAP: f32 = 8.0;

/// Quit button, just right of the audio toggles. Only drawn during a run —
/// there is nothing to quit from the title screen.
const QUIT_BTN_X: f32 = 432.0;
const QUIT_BTN_W: f32 = 96.0;
const QUIT_LABEL: &str = "QUIT RUN";
const QUIT_LABEL_ARMED: &str = "SURE?";

/// Auto-send toggle, in the gap between the difficulty label and the wave
/// counter — the right half of the strip, where the wave state already lives.
const AUTO_BTN_X: f32 = 650.0;
const AUTO_BTN_W: f32 = 92.0;
const AUTO_LABEL: &str = "AUTO";

/// Route-selection card layout. The cards share a single row, so their width
/// falls out of how many routes there are rather than being fixed — at the old
/// fixed 220px a fifth route ran 164px off the side of the window, and two rows
/// don't fit under the title.
const CARD_H: f32 = 210.0;
const CARD_GAP: f32 = 12.0;
const CARD_Y: f32 = 296.0;
/// Space left either side of the row of cards.
const CARD_MARGIN: f32 = 14.0;

/// The running build's version, shown in the corner of the title screen.
///
/// Taken from CARGO_PKG_VERSION rather than written out by hand. A hand-kept
/// string is exactly the kind that ends up disagreeing with the release it
/// shipped in — which is the one question this exists to answer.
const VERSION_LABEL: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Difficulty selector, sitting above the route cards on the same screen: the
/// mode is a setting you carry into whichever route you then pick, so it reads
/// better as a row above them than as a screen of its own in between.
const MODE_BTN_W: f32 = 196.0;
const MODE_BTN_H: f32 = 58.0;
const MODE_BTN_GAP: f32 = 12.0;
const MODE_BTN_Y: f32 = 220.0;
/// The coordinate space routes are authored in, used to scale the previews.
const AUTHOR_W: f32 = PLAYFIELD_W;
const AUTHOR_H: f32 = PLAYFIELD_H;

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
    let leaf = tint(
        shade(palette.foliage, 1.0 - prop.shade),
        prop.shade.max(0.0),
    );

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
            draw_circle(
                top.x - 9.0 * s,
                top.y + 5.0 * s,
                12.5 * s,
                shade(leaf, 0.82),
            );
            draw_circle(
                top.x + 9.0 * s,
                top.y + 4.0 * s,
                11.5 * s,
                shade(leaf, 0.88),
            );
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
            draw_poly(
                c.x,
                c.y,
                6,
                9.0 * s,
                prop.angle.to_degrees(),
                shade(stone, 0.78),
            );
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
                draw_line(
                    c.x + dx,
                    c.y + 3.0 * s,
                    c.x + dx,
                    top,
                    1.6 * s,
                    shade(leaf, 1.1),
                );
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
            draw_line(
                c.x,
                c.y - h * 0.5,
                c.x,
                c.y + h * 0.5,
                1.6 * s,
                shade(wood, 0.70),
            );
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
                draw_line(
                    px,
                    c.y + 5.0 * s,
                    px,
                    c.y - 12.0 * s,
                    3.2 * s,
                    shade(wood, 0.82),
                );
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
// Draw every lane of the route as a thick dirt polyline. Circles at the joints
// round off the corners, which macroquad's square line caps would otherwise
// leave notched.
//
// Both passes run across all lanes before the next begins — every border, then
// every dirt fill — so that where two lanes converge the second lane's dark
// border can't be painted over the first lane's finished surface, which would
// leave a seam down the middle of what should look like one piece of track.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_paths(paths: &[Path], palette: &Palette) {
    for (width, color) in [
        (TRACK_OUTER, palette.track_border),
        (TRACK_INNER, palette.track_dirt),
    ] {
        for path in paths {
            for w in path.points().windows(2) {
                draw_line(w[0].x, w[0].y, w[1].x, w[1].y, width, color);
            }
            for p in path.points() {
                draw_circle(p.x, p.y, width * 0.5, color);
            }
        }
    }

    // Exit markers — the thing the player is defending. Lanes of a multi-lane
    // route share an exit, so identical endpoints are marked once; stacking two
    // translucent markers on one spot would just make it darker than the others.
    let mut drawn: Vec<Vec2> = Vec::new();
    for path in paths {
        let Some(&end) = path.points().last() else {
            continue;
        };
        if drawn.iter().any(|p| p.distance(end) < 1.0) {
            continue;
        }
        drawn.push(end);

        let at = exit_marker_pos(end);
        draw_circle(at.x, at.y, 26.0, Color::new(0.85, 0.25, 0.25, 0.55));
        draw_circle_lines(at.x, at.y, 26.0, 3.0, Color::new(1.0, 0.85, 0.85, 0.9));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Where a route's exit marker is drawn.
//
// Not at the terminal waypoint. Routes deliberately run off the edge of the
// window so fruit leave the screen rather than blinking out at the border, which
// put the one thing the player is defending permanently out of sight — the
// marker was drawn every frame, entirely outside the window, on every route.
//
// Clamping it back inside puts it where the track crosses out of the field,
// which is the honest answer anyway: that is the last place a fruit can still be
// shot. On a route whose lanes converge, the clamped shared endpoint lands
// between them, right where they meet.
// ─────────────────────────────────────────────────────────────────────────────
fn exit_marker_pos(end: Vec2) -> Vec2 {
    /// Far enough in that the whole 26px marker clears the window edge with a
    /// visible gap, rather than sitting flush against it.
    const INSET: f32 = 42.0;

    vec2(
        end.x.clamp(INSET, PLAYFIELD_W - INSET),
        end.y.clamp(INSET, PLAYFIELD_H - INSET),
    )
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

            draw_circle(
                c.x,
                c.y - r * 0.84,
                r * 0.13,
                Color::new(0.40, 0.28, 0.15, 1.0),
            );
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
    draw_ellipse(
        x,
        y + r * 0.44,
        r * 1.04,
        r * 0.48,
        0.0,
        Color::new(0.26, 0.25, 0.23, 1.0),
    );
    draw_ellipse(
        x,
        y + r * 0.36,
        r * 1.00,
        r * 0.46,
        0.0,
        Color::new(0.47, 0.45, 0.42, 1.0),
    );
    draw_ellipse(
        x,
        y + r * 0.30,
        r * 0.92,
        r * 0.40,
        0.0,
        Color::new(0.58, 0.56, 0.52, 1.0),
    );

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

    draw_circle(t.pos.x, t.pos.y, t.range(), Color::new(1.0, 1.0, 1.0, 0.08));
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
pub fn draw_hud(h: &HudState) {
    let HudState {
        lives,
        cash,
        wave,
        total_waves,
        wave_active,
        mode,
        auto,
        auto_countdown,
    } = *h;

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
    draw_text(
        format!("${cash}"),
        200.0,
        35.0,
        30.0,
        Color::new(1.0, 0.88, 0.45, 1.0),
    );

    // The difficulty a run is being played at, in the gap between the quit
    // button and the wave counter. Worth carrying: the mode is chosen once and
    // then decides how much slack every later decision has.
    draw_text(
        MODES[mode.min(MODES.len() - 1)].name,
        560.0,
        34.0,
        22.0,
        mode_color(mode.min(MODES.len() - 1)),
    );

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

    draw_auto_button(auto);

    if !wave_active {
        // Plain ASCII only throughout: the default font has no glyph for an em
        // dash and renders it as a tofu box.
        let final_wave = wave == total_waves;
        let prompt = if auto {
            // Counting down rather than just saying "auto": the player needs to
            // know how long they have left to spend before it goes.
            let secs = auto_countdown.max(0.0).ceil() as u32;
            if final_wave {
                format!("AUTO  -  FINAL wave in {secs}s")
            } else {
                format!("AUTO  -  wave {wave} of {total_waves} in {secs}s")
            }
        } else if final_wave {
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

/// Everything the HUD draws itself from, gathered into one value rather than
/// passed as eight positional arguments nobody could read at the call site.
pub struct HudState {
    pub lives: u32,
    pub cash: u32,
    pub wave: u32,
    pub total_waves: u32,
    pub wave_active: bool,
    pub mode: usize,
    /// Whether waves send themselves.
    pub auto: bool,
    /// Seconds until the next one does. Only meaningful while `auto` is on and
    /// no wave is walking.
    pub auto_countdown: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of audio toggle `i` — 0 is effects, 1 is music. main.rs hit-tests clicks
// against the same layout this file draws.
// ─────────────────────────────────────────────────────────────────────────────
pub fn audio_button_rect(i: usize) -> Rect {
    Rect::new(
        AUDIO_BTN_X0 + i as f32 * (AUDIO_BTN + AUDIO_BTN_GAP),
        AUDIO_BTN_Y,
        AUDIO_BTN,
        AUDIO_BTN,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// The two audio toggles. Drawn on every screen, not just during play: the menu
// and the end screens have music too, and a mute you can only reach mid-run is
// a mute you reach too late.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_audio_buttons(sfx_muted: bool, music_muted: bool) {
    let mouse = mouse_vec();

    for (i, muted) in [sfx_muted, music_muted].into_iter().enumerate() {
        let r = audio_button_rect(i);
        let hovered = r.contains(mouse);

        // Each button carries its own backing plate rather than leaning on the
        // HUD strip, since on the menu and the end screens there isn't one.
        let bg = if hovered {
            Color::new(0.26, 0.28, 0.34, 0.95)
        } else {
            Color::new(0.10, 0.11, 0.15, 0.78)
        };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, Color::new(0.46, 0.48, 0.56, 0.9));

        let ink = if muted {
            Color::new(0.58, 0.58, 0.64, 1.0)
        } else {
            Color::new(0.86, 0.93, 0.88, 1.0)
        };
        let c = vec2(r.x + r.w * 0.5, r.y + r.h * 0.5);

        if i == 0 {
            draw_speaker_icon(c, ink, !muted);
        } else {
            draw_note_icon(c, ink);
        }

        if muted {
            // A slash says "off" at a glance in a way a dimmed icon never does.
            draw_line(
                r.x + 6.0,
                r.y + r.h - 6.0,
                r.x + r.w - 6.0,
                r.y + 6.0,
                2.5,
                Color::new(1.0, 0.42, 0.38, 1.0),
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// A little speaker: box, cone, and two arcs of sound coming off it. The arcs
// are dropped when it's muted, so the icon still reads without the slash.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_speaker_icon(c: Vec2, ink: Color, sounding: bool) {
    draw_rectangle(c.x - 8.0, c.y - 3.0, 5.0, 6.0, ink);
    // The cone, as two triangles making a trapezoid that widens to the right.
    let (near_t, near_b) = (vec2(c.x - 3.0, c.y - 3.0), vec2(c.x - 3.0, c.y + 3.0));
    let (far_t, far_b) = (vec2(c.x + 1.0, c.y - 7.0), vec2(c.x + 1.0, c.y + 7.0));
    draw_triangle(near_t, near_b, far_b, ink);
    draw_triangle(near_t, far_t, far_b, ink);

    if sounding {
        for radius in [5.0, 8.5] {
            draw_arc(c.x + 1.0, c.y, 24, radius, -55.0, 1.6, 110.0, ink);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// A beamed pair of quavers. At 30px a pair reads as music far more reliably
// than a single note, whose flag merges into its own stem at this size.
//
// Unlike the speaker this keeps every part when muted: a speaker without its
// arcs is the conventional "silent" icon, but there is no such convention for a
// note, and a note missing pieces just looks broken. The slash carries it.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_note_icon(c: Vec2, ink: Color) {
    let (lx, rx) = (c.x - 6.5, c.x + 4.0);
    let head_y = c.y + 5.0;

    draw_ellipse(lx, head_y, 4.2, 3.2, -20.0, ink);
    draw_ellipse(rx, head_y - 2.0, 4.2, 3.2, -20.0, ink);

    draw_rectangle(lx + 2.6, c.y - 8.0, 2.0, 13.0, ink);
    draw_rectangle(rx + 2.6, c.y - 8.0, 2.0, 11.0, ink);
    draw_rectangle(lx + 2.6, c.y - 8.0, (rx - lx) + 2.0, 3.0, ink);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of the quit button, which abandons the run and returns to the title.
// ─────────────────────────────────────────────────────────────────────────────
pub fn quit_button_rect() -> Rect {
    Rect::new(QUIT_BTN_X, AUDIO_BTN_Y, QUIT_BTN_W, AUDIO_BTN)
}

// ─────────────────────────────────────────────────────────────────────────────
// The quit button. `armed` is the second half of a two-click confirm: it sits a
// dozen pixels from the audio toggles, and a single stray click there throwing
// away a twenty-five wave run would be unforgivable. The button asks rather than
// a dialog does, so nothing has to interrupt the wave to ask it.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_quit_button(armed: bool) {
    let r = quit_button_rect();
    let hovered = r.contains(mouse_vec());

    let (bg, edge, ink, label) = if armed {
        (
            Color::new(0.44, 0.14, 0.14, 0.96),
            Color::new(1.0, 0.50, 0.45, 1.0),
            Color::new(1.0, 0.86, 0.82, 1.0),
            QUIT_LABEL_ARMED,
        )
    } else if hovered {
        (
            Color::new(0.30, 0.22, 0.22, 0.95),
            Color::new(0.78, 0.62, 0.60, 0.95),
            Color::new(0.99, 0.93, 0.91, 1.0),
            QUIT_LABEL,
        )
    } else {
        (
            Color::new(0.10, 0.11, 0.15, 0.78),
            Color::new(0.46, 0.48, 0.56, 0.9),
            Color::new(0.82, 0.82, 0.88, 1.0),
            QUIT_LABEL,
        )
    };

    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, edge);

    let dims = measure_text(label, None, 15, 1.0);
    draw_text(
        label,
        r.x + (r.w - dims.width) * 0.5,
        r.y + r.h * 0.5 + 5.0,
        15.0,
        ink,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of the auto-send toggle.
// ─────────────────────────────────────────────────────────────────────────────
pub fn auto_button_rect() -> Rect {
    Rect::new(AUTO_BTN_X, AUDIO_BTN_Y, AUTO_BTN_W, AUDIO_BTN)
}

// ─────────────────────────────────────────────────────────────────────────────
// The auto-send toggle. Lit when waves are sending themselves.
//
// Unlike the audio toggles this has no "off" icon to fall back on, so being on
// has to be carried by the fill and the outline alone — which is why the lit
// state is a solid green plate rather than a tint.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_auto_button(on: bool) {
    let r = auto_button_rect();
    let hovered = r.contains(mouse_vec());

    let (bg, edge, ink) = if on {
        (
            Color::new(0.16, 0.34, 0.20, 0.96),
            Color::new(0.55, 1.0, 0.60, 1.0),
            Color::new(0.72, 1.0, 0.76, 1.0),
        )
    } else if hovered {
        (
            Color::new(0.22, 0.24, 0.28, 0.95),
            Color::new(0.62, 0.64, 0.72, 0.95),
            Color::new(0.90, 0.90, 0.94, 1.0),
        )
    } else {
        (
            Color::new(0.10, 0.11, 0.15, 0.78),
            Color::new(0.46, 0.48, 0.56, 0.9),
            Color::new(0.72, 0.72, 0.78, 1.0),
        )
    };

    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, if on { 2.5 } else { 1.5 }, edge);

    let dims = measure_text(AUTO_LABEL, None, 15, 1.0);
    draw_text(
        AUTO_LABEL,
        r.x + (r.w - dims.width) * 0.5,
        r.y + r.h * 0.5 + 5.0,
        15.0,
        ink,
    );
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
    draw_text(
        "click a tower for stats",
        hint_x,
        PLAYFIELD_H + 74.0,
        14.0,
        dim,
    );
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

    draw_text(
        format!("Upgrade ${cost}"),
        b.x + 10.0,
        b.y + 21.0,
        18.0,
        accent,
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

    // Tucked into the bottom right: out of the way of everything, but there
    // when you need to know which build you are actually running.
    let dims = measure_text(VERSION_LABEL, None, 16, 1.0);
    draw_text(
        VERSION_LABEL,
        screen_width() - dims.width - 14.0,
        screen_height() - 14.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.38),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of route card `i`, so main.rs can hit-test clicks against the same
// layout this file draws.
// ─────────────────────────────────────────────────────────────────────────────
// Centred against PLAYFIELD_W rather than screen_width(): the window is fixed
// size, so they are the same number, but the constant is one a test can reach
// and screen_width() needs a live graphics context.
pub fn track_card_rect(i: usize) -> Rect {
    let w = card_width();
    let n = TRACKS.len() as f32;
    let total = n * w + (n - 1.0) * CARD_GAP;
    let x0 = (PLAYFIELD_W - total) * 0.5;

    Rect::new(x0 + i as f32 * (w + CARD_GAP), CARD_Y, w, CARD_H)
}

// ─────────────────────────────────────────────────────────────────────────────
// How wide each route card is: whatever divides the window evenly once the
// margins and the gaps between cards are taken out.
// ─────────────────────────────────────────────────────────────────────────────
fn card_width() -> f32 {
    let n = TRACKS.len() as f32;
    (PLAYFIELD_W - 2.0 * CARD_MARGIN - (n - 1.0) * CARD_GAP) / n
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of difficulty button `i`, centred as a row above the route cards.
// main.rs hit-tests clicks against this same layout.
// ─────────────────────────────────────────────────────────────────────────────
pub fn mode_button_rect(i: usize) -> Rect {
    let n = MODES.len() as f32;
    let total = n * MODE_BTN_W + (n - 1.0) * MODE_BTN_GAP;
    let x0 = (PLAYFIELD_W - total) * 0.5;

    Rect::new(
        x0 + i as f32 * (MODE_BTN_W + MODE_BTN_GAP),
        MODE_BTN_Y,
        MODE_BTN_W,
        MODE_BTN_H,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// The difficulty row. `selected` is the mode a run would start at right now.
//
// The chosen one is filled and outlined rather than merely tinted: it has to be
// obvious at a glance, because it is the one setting on this screen that is
// still in force after the player has stopped looking at the screen.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_mode_buttons(selected: usize) {
    let mouse = mouse_vec();

    for (i, m) in MODES.iter().enumerate() {
        let r = mode_button_rect(i);
        let is_selected = i == selected;
        let hovered = r.contains(mouse);

        let accent = mode_color(i);
        let bg = if is_selected {
            Color::new(accent.r * 0.30, accent.g * 0.30, accent.b * 0.30, 1.0)
        } else if hovered {
            Color::new(0.22, 0.23, 0.28, 1.0)
        } else {
            Color::new(0.15, 0.16, 0.20, 1.0)
        };

        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            if is_selected { 3.0 } else { 1.5 },
            if is_selected {
                accent
            } else {
                Color::new(0.40, 0.42, 0.50, 1.0)
            },
        );

        let label_color = if is_selected {
            accent
        } else {
            Color::new(0.80, 0.80, 0.86, 1.0)
        };
        let dims = measure_text(m.name, None, 22, 1.0);
        draw_text(
            m.name,
            r.x + (r.w - dims.width) * 0.5,
            r.y + 24.0,
            22.0,
            label_color,
        );

        // What the mode actually changes, so the choice isn't three words with
        // nothing behind them. Both lines, because the speed cap is the dial
        // that decides how the late waves feel and it would otherwise be
        // invisible — the player would see only the opening hand.
        let detail = Color::new(0.70, 0.70, 0.78, 1.0);
        for (line, text) in [
            format!("${}   {} lives", m.start_cash, m.start_lives),
            format!("fruit up to +{:.0}% speed", (m.max_speed - 1.0) * 100.0),
        ]
        .into_iter()
        .enumerate()
        {
            let dims = measure_text(&text, None, 14, 1.0);
            draw_text(
                &text,
                r.x + (r.w - dims.width) * 0.5,
                r.y + 38.0 + line as f32 * 15.0,
                14.0,
                detail,
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Colour for a difficulty, by position in the row rather than by name, so the
// ramp stays green-to-red however the modes are eventually labelled.
// ─────────────────────────────────────────────────────────────────────────────
fn mode_color(i: usize) -> Color {
    match i {
        0 => Color::new(0.55, 0.95, 0.60, 1.0),
        1 => Color::new(1.0, 0.85, 0.45, 1.0),
        _ => Color::new(1.0, 0.50, 0.45, 1.0),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Route selection screen: one card per track, each showing a scaled preview of
// the actual polyline the fruit will walk.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_track_select(selected_mode: usize) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.62),
    );

    text_center("CHOOSE YOUR ROUTE", 138.0, 62.0, WHITE);
    text_center(
        "Longer routes give your towers more time to shoot",
        182.0,
        24.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    // Spells out that this applies to any route, because each card carries its
    // own difficulty word too, and "Hard" would otherwise mean two things on one
    // screen: how punishing the track is, versus how much you start with.
    text_center(
        "DIFFICULTY  -  applies to whichever route you pick",
        208.0,
        17.0,
        Color::new(1.0, 1.0, 1.0, 0.55),
    );

    draw_mode_buttons(selected_mode);

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

        // 19px, not 21: at five cards to a row the longest route name sat hard
        // against the card border.
        draw_text(
            format!("{}. {}", i + 1, track.name),
            r.x + 12.0,
            r.y + 26.0,
            19.0,
            WHITE,
        );

        // Preview in the route's own colours, so the backdrops are comparable
        // before committing to a run.
        draw_track_preview(track.lanes, r, &crate::scenery::palette(i));

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
        &format!("click a route, or press 1-{}", TRACKS.len()),
        CARD_Y + CARD_H + 48.0,
        26.0,
        Color::new(1.0, 0.85, 0.4, 1.0),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw a route's polyline scaled down to fit inside a selection card.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_track_preview(lanes: &[&[(f32, f32)]], card: Rect, palette: &Palette) {
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

    let dirt = Color::new(0.55, 0.43, 0.29, 1.0);
    for lane in lanes {
        for w in lane.windows(2) {
            let (a, b) = (to_card(w[0]), to_card(w[1]));
            draw_line(a.x, a.y, b.x, b.y, 5.0, dirt);
        }
        for &p in *lane {
            let c = to_card(p);
            draw_circle(c.x, c.y, 2.5, dirt);
        }
    }

    // Mark each entrance, so a two-lane route is recognisable as such from the
    // card rather than only once the first wave is already walking.
    for lane in lanes {
        if let Some(&first) = lane.first() {
            let s = to_card(first);
            draw_circle(s.x, s.y, 4.0, Color::new(0.55, 0.85, 1.0, 0.9));
        }
    }

    // Mark the exit the player is defending. Lanes share one, so this draws the
    // same point twice on a two-lane route — harmless, it is fully opaque.
    for lane in lanes {
        if let Some(&last) = lane.last() {
            let e = to_card(last);
            draw_circle(e.x, e.y, 5.0, Color::new(0.85, 0.25, 0.25, 0.9));
        }
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
pub fn draw_victory(route: &str, total_waves: u32, lives: u32, mode: usize) {
    let cy = screen_height() * 0.42;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.10, 0.03, 0.58),
    );

    text_center(
        "ROUTE CLEARED",
        cy - 60.0,
        72.0,
        Color::new(0.72, 1.0, 0.72, 1.0),
    );
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
    // Which difficulty it was cleared on — the same route on Easy and on Hard
    // are not the same achievement, and the screen should say which one this is.
    let m = mode.min(MODES.len() - 1);
    text_center(
        &format!("on {}", MODES[m].name),
        cy + 84.0,
        26.0,
        mode_color(m),
    );
    text_center(
        "Click to pick another route",
        cy + 130.0,
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
    const WINDOW_W: f32 = PLAYFIELD_W;

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

    /// The dark strip along the top of the playfield, which the audio toggles
    /// have to stay inside to read against a backing rather than the grass.
    const HUD_STRIP_H: f32 = 52.0;

    #[test]
    fn the_mode_row_fits_the_window_and_does_not_overlap() {
        let n = MODES.len();
        for i in 0..n {
            let r = mode_button_rect(i);
            assert!(r.x >= 0.0 && r.x + r.w <= WINDOW_W, "mode {i} runs off");
        }
        for i in 1..n {
            let (prev, cur) = (mode_button_rect(i - 1), mode_button_rect(i));
            assert!(
                cur.x >= prev.x + prev.w,
                "mode buttons {} and {i} overlap",
                i - 1
            );
        }
    }

    #[test]
    fn the_mode_row_clears_the_route_cards_below_it() {
        // The two are picked in sequence on one screen, so they must not share
        // any pixels — a click has to belong unambiguously to one of them.
        let row = mode_button_rect(0);
        assert!(
            row.y + row.h <= CARD_Y,
            "the difficulty row overlaps the route cards"
        );
    }

    #[test]
    fn every_mode_label_fits_its_button() {
        // Character budget rather than measure_text, which needs a graphics
        // context. The detail line is the longer of the two at 14px.
        for m in &MODES {
            for line in [
                format!("${}   {} lives", m.start_cash, m.start_lives),
                format!("fruit up to +{:.0}% speed", (m.max_speed - 1.0) * 100.0),
            ] {
                assert!(
                    line.len() as f32 * 7.5 + 12.0 <= MODE_BTN_W,
                    "\"{line}\" overflows the mode button"
                );
            }
            assert!(m.name.len() as f32 * 12.0 + 12.0 <= MODE_BTN_W);
        }
    }

    #[test]
    fn the_audio_buttons_sit_inside_the_hud_strip() {
        for i in 0..2 {
            let r = audio_button_rect(i);
            assert!(
                r.y >= 0.0 && r.y + r.h <= HUD_STRIP_H,
                "button {i} overhangs"
            );
            assert!(r.x >= 0.0 && r.x + r.w <= WINDOW_W, "button {i} runs off");
        }
    }

    #[test]
    fn the_audio_buttons_do_not_overlap_each_other() {
        let (sfx, music) = (audio_button_rect(0), audio_button_rect(1));
        assert!(music.x >= sfx.x + sfx.w, "the two toggles overlap");
    }

    #[test]
    fn the_audio_buttons_clear_the_cash_readout() {
        // The cash text starts at x=200 at size 30. Six digits of it is about
        // 100px, and the buttons must not land on top of a rich player's total.
        assert!(
            audio_button_rect(0).x >= 310.0,
            "audio buttons crowd the cash readout"
        );
    }

    #[test]
    fn the_quit_button_clears_the_audio_toggles() {
        // They sit side by side in the same strip, and the quit button ends a
        // run — it must not be reachable by a slip off the music toggle.
        let music = audio_button_rect(1);
        let quit = quit_button_rect();

        assert!(
            quit.x >= music.x + music.w,
            "quit overlaps the audio toggles"
        );
        assert!(quit.x - (music.x + music.w) >= 8.0, "quit crowds them");
    }

    #[test]
    fn the_quit_button_sits_inside_the_hud_strip() {
        let r = quit_button_rect();
        assert!(r.y >= 0.0 && r.y + r.h <= HUD_STRIP_H, "quit overhangs");
        // The wave counter is right-aligned; leave it its half of the strip.
        assert!(r.x + r.w <= WINDOW_W * 0.6, "quit crowds the wave counter");
    }

    #[test]
    fn the_version_label_looks_like_a_version() {
        // It is read from the package version, so this is really asserting that
        // the corner of the title screen says something a person can act on
        // rather than an empty string.
        assert!(VERSION_LABEL.starts_with('v'), "{VERSION_LABEL}");
        let parts: Vec<&str> = VERSION_LABEL[1..].split('.').collect();
        assert_eq!(parts.len(), 3, "expected v<major>.<minor>.<patch>");
        for p in parts {
            assert!(
                !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()),
                "\"{VERSION_LABEL}\" has a non-numeric component"
            );
        }
    }

    #[test]
    fn the_top_strip_buttons_never_overlap_each_other() {
        // Audio, quit and auto all live in the same 52px strip and all take a
        // click before the field does. Two overlapping would make which one
        // fires depend on their order in the code rather than on what was hit.
        let rects = [
            ("sfx", audio_button_rect(0)),
            ("music", audio_button_rect(1)),
            ("quit", quit_button_rect()),
            ("auto", auto_button_rect()),
        ];

        for (i, (an, a)) in rects.iter().enumerate() {
            for (bn, b) in rects.iter().skip(i + 1) {
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x,
                    "the {an} and {bn} buttons overlap"
                );
            }
        }
    }

    #[test]
    fn the_auto_button_sits_inside_the_strip_clear_of_the_wave_counter() {
        let r = auto_button_rect();
        assert!(r.y >= 0.0 && r.y + r.h <= HUD_STRIP_H, "auto overhangs");
        // "WAVE 25/25" is right-aligned at 30px, about 165px wide plus a 20px
        // margin, so the counter owns roughly the last 185px of the strip.
        assert!(
            r.x + r.w <= WINDOW_W - 185.0,
            "the auto button crowds the wave counter"
        );
    }

    #[test]
    fn the_auto_label_fits_its_button() {
        assert!(
            AUTO_LABEL.len() as f32 * 8.0 + 12.0 <= AUTO_BTN_W,
            "\"{AUTO_LABEL}\" overflows the auto button"
        );
    }

    #[test]
    fn both_quit_labels_fit_the_button() {
        // measure_text needs a graphics context that a unit test has no way to
        // get, so this is a character budget instead — the same trick the route
        // blurbs use. At 15px the default font runs about 8px per capital.
        const PX_PER_CHAR: f32 = 8.0;
        const PADDING: f32 = 12.0;

        for label in [QUIT_LABEL, QUIT_LABEL_ARMED] {
            assert!(
                label.len() as f32 * PX_PER_CHAR + PADDING <= quit_button_rect().w,
                "\"{label}\" is {} chars, too wide for the button",
                label.len()
            );
        }
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
