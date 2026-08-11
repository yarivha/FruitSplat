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
use crate::projectile::{Blast, Projectile, ProjectileKind};
use crate::scenery::{Palette, Prop, PropKind};
use crate::tower::{Pulse, SpikePile, Tower, TowerKind, TOWER_RADIUS};
use crate::tracks::TRACKS;
use crate::{PLAYFIELD_H, PLAYFIELD_W, SHOP_PANEL_W};

// ─────────────────────────────────────────────────────────────────────────────
// The view: a fixed space the game is drawn in, scaled onto whatever surface it
// actually has.
//
// Every layout number in this file is authored against VIEW_W x VIEW_H, and the
// game draws in those coordinates whatever size the real drawing surface is. A
// camera scales that space to fit, centred, with bars either side of it where
// the aspect ratios differ.
//
// The web build is why. Its canvas is sized by CSS to whatever the device has,
// and macroquad takes its drawing buffer from the element's clientWidth — so on
// a phone screen_width() is a few hundred pixels and a layout authored against
// 1420 would draw almost entirely off the side. Scaling the canvas with a CSS
// transform instead is the trap web/index.html documents: the buffer ignores
// transforms and hit-testing does not, so the two disagree and every tap lands
// somewhere other than where it looks. Doing the scaling here keeps rendering
// and input in one place, both derived from the same numbers.
//
// On a desktop window, which is fixed at exactly this size, the scale is 1 and
// the offset zero, so none of it does anything.
// ─────────────────────────────────────────────────────────────────────────────

/// Width of the space the game is drawn in: playfield plus the shop column.
pub const VIEW_W: f32 = PLAYFIELD_W + SHOP_PANEL_W;
/// Height of the space the game is drawn in.
pub const VIEW_H: f32 = PLAYFIELD_H;

/// Scale and top-left offset that fit the view onto a surface of `surface`
/// pixels, preserving the aspect ratio and centring what is left over.
///
/// Split out from the camera so it can be tested: the camera itself needs a live
/// graphics context, and this arithmetic is the part that can be wrong.
pub fn view_fit(surface: Vec2) -> (f32, Vec2) {
    let scale = (surface.x / VIEW_W)
        .min(surface.y / VIEW_H)
        .max(f32::MIN_POSITIVE);
    let size = vec2(VIEW_W, VIEW_H) * scale;
    (scale, (surface - size) * 0.5)
}

/// Turn a point in real surface pixels into one in view coordinates. Every
/// pointer position goes through this before anything is hit-tested.
pub fn to_view(surface: Vec2, point: Vec2) -> Vec2 {
    let (scale, offset) = view_fit(surface);
    (point - offset) / scale
}

/// The surface actually being drawn to.
pub fn surface() -> Vec2 {
    vec2(screen_width(), screen_height())
}

// ─────────────────────────────────────────────────────────────────────────────
// Point the camera at the view and start drawing in its coordinates. The bars
// beside a mismatched surface are painted first, so they don't smear the last
// frame down the sides.
// ─────────────────────────────────────────────────────────────────────────────
pub fn begin_view() {
    clear_background(Color::new(0.05, 0.06, 0.07, 1.0));

    let surface = surface();
    let (scale, offset) = view_fit(surface);
    let size = vec2(VIEW_W, VIEW_H) * scale;

    let cam = Camera2D {
        target: vec2(VIEW_W, VIEW_H) * 0.5,
        // Built by hand rather than with Camera2D::from_display_rect, whose
        // negative y zoom suits a render target. Against the screen's own
        // framebuffer it turns the world upside down — the menu came out
        // mirrored top to bottom, text and all.
        zoom: vec2(2.0 / VIEW_W, 2.0 / VIEW_H),
        offset: Vec2::ZERO,
        rotation: 0.0,
        render_target: None,
        // A viewport is given in GL coordinates, measured from the bottom left.
        // The letterbox is centred, so the flipped y comes to the same offset
        // either way round.
        viewport: Some((
            offset.x as i32,
            offset.y as i32,
            size.x as i32,
            size.y as i32,
        )),
    };
    set_camera(&cam);
}

pub fn end_view() {
    set_default_camera();
}

/// Number of bands used to fake the grass gradient.
const FIELD_BANDS: i32 = 40;

/// Track widths — the outer band is the dirt border, the inner the worn middle.
const TRACK_OUTER: f32 = 44.0;
const TRACK_INNER: f32 = 34.0;

/// Shop layout. The buttons stack down the right-hand column, so their width
/// comes from the panel and only their height and spacing are chosen here. A row
/// ran out of window at five buttons; a column has room for a good many more.
const BTN_H: f32 = 58.0;
const BTN_GAP: f32 = 7.0;
/// Margin between the panel's edges and everything inside it.
const PANEL_PAD: f32 = 12.0;
/// Where the tower buttons start, leaving room for the panel heading.
const BTN_Y0: f32 = 62.0;

/// The controls sit in a block at the foot of the column — the two audio
/// toggles side by side on one row, then pause, auto and quit. Laid out from the
/// bottom of the window upward, so adding another tower above can never push
/// them off the screen.
const CTRL_H: f32 = 32.0;
const CTRL_GAP: f32 = 7.0;
const CTRL_BOTTOM_PAD: f32 = 12.0;

/// Floating tower-panel size.
const PANEL_W: f32 = 250.0;
const PANEL_H: f32 = 228.0;

/// Which control sits on which row of the block at the foot of the shop column,
/// counted from the bottom of the window upward. Quit is furthest from the tower
/// buttons on purpose: it is the one click here that cannot be taken back.
const CTRL_ROW_QUIT: usize = 0;
const CTRL_ROW_AUTO: usize = 1;
const CTRL_ROW_PAUSE: usize = 2;
const CTRL_ROW_AUDIO: usize = 3;
/// How many rows that block occupies, for the layout tests and for working out
/// how much of the column is left for towers.
pub const CTRL_ROWS: usize = 4;

const QUIT_LABEL: &str = "QUIT RUN";
const QUIT_LABEL_ARMED: &str = "SURE?";
const AUTO_LABEL: &str = "AUTO";
const PAUSE_LABEL: &str = "PAUSE";
const PAUSE_LABEL_HELD: &str = "RESUME";

/// Route-selection card layout. The cards share a single row, so their width
/// falls out of how many routes there are rather than being fixed — at the old
/// fixed 220px a fifth route ran 164px off the side of the window, and two rows
/// don't fit under the title.
const CARD_H: f32 = 200.0;
const CARD_GAP: f32 = 14.0;
const CARD_Y: f32 = 248.0;
/// Space left either side of a row of cards.
const CARD_MARGIN: f32 = 20.0;
/// The picker's heading block, top to bottom: title, subtitle, then the caption
/// above the difficulty row. Named because the layout below them is computed and
/// has to be checked against them — two rows of cards leave far less slack above
/// them than one row did, and each of these overlapped something at least once
/// while that was being fitted.
const PICK_TITLE_Y: f32 = 88.0;
const PICK_SUBTITLE_Y: f32 = 126.0;
const PICK_CAPTION_Y: f32 = 156.0;
const _: () = assert!(PICK_SUBTITLE_Y - PICK_TITLE_Y >= 30.0);
const _: () = assert!(PICK_CAPTION_Y - PICK_SUBTITLE_Y >= 20.0);

/// Cards per row. Two rows rather than one: a single row of seven left each card
/// too narrow to read its own name, and the window is wide but not that wide.
const CARDS_PER_ROW: usize = 4;

/// Y of the bottom edge of the last row of cards.
fn card_rows_bottom() -> f32 {
    let rows = card_count().div_ceil(CARDS_PER_ROW) as f32;
    CARD_Y + rows * CARD_H + (rows - 1.0) * CARD_GAP
}

/// How many cards the picker shows: one per route, plus the random one.
pub fn card_count() -> usize {
    TRACKS.len() + 1
}

/// Index of the card that picks a route at random — always the last, so adding a
/// route never moves it out from under the player's finger.
pub fn random_card_index() -> usize {
    TRACKS.len()
}

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
const MODE_BTN_Y: f32 = 172.0;
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
    let w = VIEW_W;
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

    if f.shielded {
        draw_shield(c, r);
    }

    // Armour readout, bosses only and only once one has actually been hit. An
    // untouched boss shows nothing, so the bar appearing is itself the signal
    // that the fight has started.
    if f.kind.is_boss() && f.hp < f.kind.armour() {
        draw_armour_bar(c, r, f.health_fraction());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The bubble around a shielded fruit: a hard metallic ring with a faint fill and
// four rivets.
//
// It has to be legible instantly and at a glance, because it is the difference
// between a fruit a tower can kill and one it cannot — a player watching shots
// bounce with no visible reason would read it as the game being broken. So it
// sits *outside* the fruit's own silhouette rather than tinting it, which is
// what the frost wash above already does and what the two must not be confused
// for: frost is something a Freezer did, a shield is something the fruit came
// with.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_shield(center: Vec2, r: f32) {
    let ring = r * 1.28;
    let steel = Color::new(0.72, 0.80, 0.92, 1.0);

    // A cold fill, kept faint so the fruit's own colour still reads through it.
    draw_circle(center.x, center.y, ring, Color::new(0.62, 0.74, 0.95, 0.15));
    // Two rings: a dark backing under a bright edge, so it reads against both
    // the pale grass and the dark track.
    draw_circle_lines(
        center.x,
        center.y,
        ring,
        4.0,
        Color::new(0.16, 0.22, 0.34, 0.85),
    );
    draw_circle_lines(center.x, center.y, ring, 2.0, steel);

    // Rivets at the quarters, which is what makes it read as a shell rather
    // than another status wash.
    for i in 0..4 {
        let a = std::f32::consts::FRAC_PI_4 + i as f32 * std::f32::consts::FRAC_PI_2;
        let p = center + vec2(a.cos(), a.sin()) * ring;
        draw_circle(p.x, p.y, r * 0.13, shade(steel, 0.55));
        draw_circle(p.x - r * 0.02, p.y - r * 0.03, r * 0.07, tint(steel, 0.5));
    }

    // A highlight arc on the lit side, so the ring reads as curved glass rather
    // than a flat circle drawn on top.
    draw_arc(
        center.x - r * 0.10,
        center.y - r * 0.12,
        24,
        ring * 0.92,
        200.0,
        2.0,
        70.0,
        Color::new(1.0, 1.0, 1.0, 0.45),
    );
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

        TowerKind::TripleSeeder => {
            // Three barrels fanned around the aim, so what the tower does is
            // legible from the board rather than only from its panel.
            let wood = Color::new(0.30, 0.22, 0.13, 1.0);
            for spread in [-0.42_f32, 0.0, 0.42] {
                let a = angle + spread;
                let d = vec2(a.cos(), a.sin());
                let tip = pos + d * (r + r * 0.46);
                draw_line(x, y, tip.x, tip.y, r * 0.26, wood);
                draw_circle(tip.x, tip.y, r * 0.13, Color::new(0.52, 0.40, 0.24, 1.0));
            }
            // A collar covering the roots, so the three read as one machine.
            draw_circle(x, y, r * 0.46, Color::new(0.26, 0.38, 0.28, 1.0));
            draw_circle(
                x - r * 0.08,
                y - r * 0.10,
                r * 0.28,
                Color::new(0.44, 0.60, 0.46, 1.0),
            );
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

        TowerKind::BombLobber => {
            // A short, fat mortar tube tipped up at the sky, with a shell nosing
            // out of it. Stubby on purpose: every other barrel here is a long
            // thin line, and at tower size the silhouette is all the player has
            // to tell them apart with.
            let iron = Color::new(0.26, 0.27, 0.31, 1.0);
            let mouth = pos + dir * r * 0.86;

            // Tube, drawn as a thick line so the ends are round.
            draw_line(
                x - dir.x * r * 0.24,
                y - dir.y * r * 0.24,
                mouth.x,
                mouth.y,
                r * 0.78,
                iron,
            );
            // Lit along the upper edge, shadowed along the lower.
            draw_line(
                x - perp.x * r * 0.22,
                y - perp.y * r * 0.22,
                mouth.x - perp.x * r * 0.22,
                mouth.y - perp.y * r * 0.22,
                r * 0.20,
                tint(iron, 0.28),
            );
            draw_line(
                x + perp.x * r * 0.26,
                y + perp.y * r * 0.26,
                mouth.x + perp.x * r * 0.26,
                mouth.y + perp.y * r * 0.26,
                r * 0.16,
                shade(iron, 0.62),
            );

            // The bore, and a shell sitting in it ready to go.
            draw_circle(mouth.x, mouth.y, r * 0.34, shade(iron, 0.35));
            let shell = mouth + dir * r * 0.16;
            draw_circle(
                shell.x,
                shell.y,
                r * 0.26,
                Color::new(0.16, 0.16, 0.19, 1.0),
            );
            draw_circle(
                shell.x - dir.x * r * 0.04 - perp.x * r * 0.06,
                shell.y - dir.y * r * 0.04 - perp.y * r * 0.06,
                r * 0.10,
                Color::new(0.45, 0.46, 0.52, 1.0),
            );

            // Baseplate bolts, which read as heaviness at a glance.
            for i in 0..2 {
                let side = if i == 0 { 1.0 } else { -1.0 };
                let bolt = pos + perp * r * 0.62 * side - dir * r * 0.30;
                draw_circle(bolt.x, bolt.y, r * 0.13, shade(c, 0.55));
                draw_circle(bolt.x, bolt.y, r * 0.07, tint(c, 0.35));
            }
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

    // A shell is an iron ball with a lit fuse, so it reads as something about
    // to go off rather than as an oversized seed.
    if p.kind == ProjectileKind::Shell {
        shaded_ball(p.pos, r, c);
        let a = p.spin.to_radians();
        let fuse = p.pos + vec2(a.cos(), a.sin()) * r * 1.25;
        draw_line(p.pos.x, p.pos.y, fuse.x, fuse.y, r * 0.22, shade(c, 1.6));
        draw_circle(fuse.x, fuse.y, r * 0.30, Color::new(1.0, 0.78, 0.30, 0.95));
        draw_circle(fuse.x, fuse.y, r * 0.16, Color::new(1.0, 0.97, 0.80, 1.0));
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
// The blast a shell leaves: a filled flash that fades fast inside a ring that
// carries on out to the true blast radius.
//
// The ring is the useful half. It ends exactly on the radius that did the
// damage, so a player watching one land learns what a Bomb Lobber covers —
// which is otherwise unknowable, the tower's range being much shorter than the
// area it clears.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_blast(b: &Blast) {
    let t = b.progress();

    draw_circle(
        b.pos.x,
        b.pos.y,
        b.radius * (0.35 + t * 0.65),
        Color::new(1.0, 0.72, 0.30, (1.0 - t) * 0.28),
    );
    draw_circle_lines(
        b.pos.x,
        b.pos.y,
        b.radius * t,
        4.0,
        Color::new(1.0, 0.86, 0.55, 1.0 - t),
    );
    // A dark core that shrinks as the flash spreads, for the smoke.
    draw_circle(
        b.pos.x,
        b.pos.y,
        b.radius * 0.30 * (1.0 - t),
        Color::new(0.22, 0.18, 0.16, (1.0 - t) * 0.5),
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
    draw_rectangle(0.0, 0.0, PLAYFIELD_W, 52.0, Color::new(0.0, 0.0, 0.0, 0.35));

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
        360.0,
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
        PLAYFIELD_W - dims.width - 20.0,
        35.0,
        30.0,
        wave_color,
    );

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
        field_text_center(
            &prompt,
            PLAYFIELD_H - 26.0,
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
    let row = ctrl_row_rect(CTRL_ROW_AUDIO);
    let w = (row.w - CTRL_GAP) * 0.5;
    Rect::new(row.x + i as f32 * (w + CTRL_GAP), row.y, w, row.h)
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
    ctrl_row_rect(CTRL_ROW_QUIT)
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
    ctrl_row_rect(CTRL_ROW_AUTO)
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
// The usable width inside the shop column, once its margins are taken out.
// ─────────────────────────────────────────────────────────────────────────────
fn panel_inner_w() -> f32 {
    SHOP_PANEL_W - PANEL_PAD * 2.0
}

fn panel_inner_x() -> f32 {
    PLAYFIELD_W + PANEL_PAD
}

// ─────────────────────────────────────────────────────────────────────────────
// Full-width rect of control row `row`, counted from the bottom of the panel up.
//
// Anchored to the bottom of the window rather than flowing after the tower
// buttons, so adding a tower grows the column downward into empty space instead
// of shoving the controls off the screen.
// ─────────────────────────────────────────────────────────────────────────────
fn ctrl_row_rect(row: usize) -> Rect {
    Rect::new(
        panel_inner_x(),
        PLAYFIELD_H - CTRL_BOTTOM_PAD - CTRL_H - row as f32 * (CTRL_H + CTRL_GAP),
        panel_inner_w(),
        CTRL_H,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of the pause toggle.
// ─────────────────────────────────────────────────────────────────────────────
pub fn pause_button_rect() -> Rect {
    ctrl_row_rect(CTRL_ROW_PAUSE)
}

// ─────────────────────────────────────────────────────────────────────────────
// The pause toggle. Lit while the game is held, and it says RESUME then rather
// than PAUSE — a held game should tell you the way out, not repeat the way in.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_pause_button(paused: bool) {
    let r = pause_button_rect();
    let hovered = r.contains(mouse_vec());

    let (bg, edge, ink, label) = if paused {
        (
            Color::new(0.34, 0.28, 0.10, 0.96),
            Color::new(1.0, 0.85, 0.45, 1.0),
            Color::new(1.0, 0.92, 0.70, 1.0),
            PAUSE_LABEL_HELD,
        )
    } else if hovered {
        (
            Color::new(0.22, 0.24, 0.28, 0.95),
            Color::new(0.62, 0.64, 0.72, 0.95),
            Color::new(0.90, 0.90, 0.94, 1.0),
            PAUSE_LABEL,
        )
    } else {
        (
            Color::new(0.10, 0.11, 0.15, 0.78),
            Color::new(0.46, 0.48, 0.56, 0.9),
            Color::new(0.72, 0.72, 0.78, 1.0),
            PAUSE_LABEL,
        )
    };

    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, if paused { 2.5 } else { 1.5 }, edge);

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
// The held-game overlay. Dims the field only, leaving the shop column lit so its
// buttons still read as clickable — pause is for thinking about what to build.
//
// Plain ASCII in the label: the default font has no em dash glyph and draws one
// as a tofu box. See `every_drawn_string_is_ascii`.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_paused_overlay() {
    draw_rectangle(
        0.0,
        0.0,
        PLAYFIELD_W,
        PLAYFIELD_H,
        Color::new(0.0, 0.0, 0.0, 0.45),
    );
    field_text_center(
        "PAUSED",
        PLAYFIELD_H * 0.44,
        64.0,
        Color::new(1.0, 0.9, 0.5, 1.0),
    );
    field_text_center(
        "the wave is held  -  build, upgrade, then resume",
        PLAYFIELD_H * 0.44 + 44.0,
        24.0,
        Color::new(1.0, 1.0, 1.0, 0.75),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of shop button `i`, so main.rs can hit-test clicks against the same
// layout this file draws.
// ─────────────────────────────────────────────────────────────────────────────
pub fn shop_button_rect(i: usize) -> Rect {
    Rect::new(
        panel_inner_x(),
        BTN_Y0 + i as f32 * (BTN_H + BTN_GAP),
        panel_inner_w(),
        BTN_H,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// The shop column: a heading, one button per tower, the placement hints, and the
// control block at the foot.
//
// A button is dimmed when it can't be afforded and outlined when it's armed.
// ─────────────────────────────────────────────────────────────────────────────
pub fn draw_shop(selected: Option<TowerKind>, cash: u32) {
    draw_rectangle(
        PLAYFIELD_W,
        0.0,
        SHOP_PANEL_W,
        PLAYFIELD_H,
        Color::new(0.12, 0.12, 0.16, 1.0),
    );
    // A seam against the field, so the column reads as chrome rather than as
    // more playfield the player might try to build on.
    draw_line(
        PLAYFIELD_W,
        0.0,
        PLAYFIELD_W,
        PLAYFIELD_H,
        2.0,
        Color::new(0.30, 0.31, 0.38, 1.0),
    );

    draw_text(
        "TOWERS",
        panel_inner_x() + 2.0,
        36.0,
        24.0,
        Color::new(0.80, 0.80, 0.88, 1.0),
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

        // The real tower artwork drawn small, rather than a flat colour dot.
        // Barrels and blades reach about 1.6x the radius, so the text column
        // starts clear of that rather than being overlapped by it.
        let icon = vec2(r.x + 22.0, r.y + r.h * 0.5);
        draw_tower_icon(*kind, icon, 13.0);
        if !affordable {
            // Fade the icon toward the button background instead of redrawing
            // every layer of it at a lower alpha.
            draw_circle(icon.x, icon.y, 21.0, Color::new(bg.r, bg.g, bg.b, 0.62));
        }

        let text_alpha = if affordable { 1.0 } else { 0.45 };
        let text_x = r.x + 46.0;
        // Only the first nine get a number key, and the label should not claim
        // one that does not exist.
        let name = if i < crate::NUMBER_KEYS.len() {
            format!("{}. {}", i + 1, kind.short_name())
        } else {
            kind.short_name().to_string()
        };
        draw_text(
            &name,
            text_x,
            r.y + 24.0,
            17.0,
            Color::new(1.0, 1.0, 1.0, text_alpha),
        );
        draw_text(
            format!("${}", kind.cost()),
            text_x,
            r.y + 44.0,
            15.0,
            Color::new(1.0, 0.88, 0.45, text_alpha),
        );
        let dims = measure_text(kind.blurb(), None, 13, 1.0);
        draw_text(
            kind.blurb(),
            r.x + r.w - dims.width - 8.0,
            r.y + 44.0,
            13.0,
            Color::new(0.80, 0.80, 0.88, text_alpha),
        );
    }

    // Hints, tucked between the last tower and the control block.
    let dim = Color::new(0.62, 0.62, 0.70, 1.0);
    let hint_y = ctrl_row_rect(CTRL_ROWS - 1).y - 62.0;
    for (i, line) in [
        "click to place",
        "right-click cancels",
        "click a tower for stats",
    ]
    .iter()
    .enumerate()
    {
        draw_text(
            line,
            panel_inner_x() + 2.0,
            hint_y + i as f32 * 17.0,
            13.0,
            dim,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Where the floating tower panel sits for a tower at `tower_pos`.
//
// Prefers the right of the tower and flips to the left when that would run past
// the playfield, then clamps vertically so it always stays inside it.
// main.rs hit-tests clicks against this same rect.
//
// Every edge here is PLAYFIELD_W, not VIEW_W. The shop column starts at
// PLAYFIELD_W and main.rs gives it any click past that line before the panel is
// consulted at all, so a panel overhanging the column is drawn but dead: it took
// the towers between x 950 and 1126 — a quarter of the map — and left them
// impossible to upgrade or sell, since their buttons were the part hanging over.
// Fitting the panel inside the playfield settles it in one place rather than
// leaving two owners arguing over the same pixels.
// ─────────────────────────────────────────────────────────────────────────────
pub fn tower_panel_rect(tower_pos: Vec2) -> Rect {
    let gap = TOWER_RADIUS + 14.0;

    let mut x = tower_pos.x + gap;
    if x + PANEL_W > PLAYFIELD_W - 10.0 {
        x = tower_pos.x - gap - PANEL_W;
    }
    x = x.clamp(10.0, (PLAYFIELD_W - PANEL_W - 10.0).max(10.0));

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
            ("Every", format!("{:.2}s", t.fire_cooldown())),
            ("Laid", t.shots_fired.to_string()),
            ("Kills", t.kills.to_string()),
        ],
        // Blast before range: the area it clears is the number that decides
        // where this one goes, and it is nearly twice the reach.
        TowerKind::BombLobber => [
            ("Blast", format!("{:.0}", t.splash_radius())),
            ("Every", format!("{:.2}s", t.fire_cooldown())),
            ("Shells", t.shots_fired.to_string()),
            ("Kills", t.kills.to_string()),
        ],
        TowerKind::TripleSeeder => [
            ("Range", format!("{:.0}", t.range())),
            ("Targets", t.shots().to_string()),
            ("Volleys", t.shots_fired.to_string()),
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
// Current mouse position, in view coordinates — the same space every rect in
// this file is expressed in. Reading mouse_position() raw would hover-test
// against surface pixels, which only match on a window that happens to be the
// view's exact size.
// ─────────────────────────────────────────────────────────────────────────────
fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    to_view(surface(), vec2(x, y))
}

// ─────────────────────────────────────────────────────────────────────────────
// Title screen.
// ─────────────────────────────────────────────────────────────────────────────
// ─────────────────────────────────────────────────────────────────────────────
// The title, spelled out in fruit rather than set in the text font.
//
// Every letter is a 5x7 grid and every lit cell is a berry, drawn with the same
// shaded_ball and specular the fruit themselves use — so the wordmark is made of
// the game's own artwork instead of merely sitting above it. One fruit colour
// per letter, cycling the tiers the game actually sends, which lands the same
// five on each word.
//
// The berries are nudged off their grid by a hash of their position rather than
// by an RNG. It has to be deterministic: this is redrawn every frame, and a live
// random offset would leave the whole title crawling.
// ─────────────────────────────────────────────────────────────────────────────
const TITLE_TEXT: &str = "FRUIT SPLAT";
const TITLE_COLS: usize = 5;
const TITLE_ROWS: usize = 7;
/// Blank columns between two letters, and for the gap between the two words.
const TITLE_GAP_COLS: f32 = 1.0;
const TITLE_SPACE_COLS: f32 = 3.0;
/// How much of the window's width the finished wordmark spans.
const TITLE_SPAN: f32 = 0.80;
/// One colour per letter, in tier order.
const TITLE_FRUIT: [FruitKind; 5] = [
    FruitKind::Strawberry,
    FruitKind::Orange,
    FruitKind::Lime,
    FruitKind::Blueberry,
    FruitKind::Watermelon,
];

#[rustfmt::skip]
fn title_glyph(ch: char) -> Option<[&'static str; TITLE_ROWS]> {
    Some(match ch {
        'F' => ["#####",
                "#....",
                "#....",
                "####.",
                "#....",
                "#....",
                "#...."],
        'R' => ["####.",
                "#...#",
                "#...#",
                "####.",
                "#.#..",
                "#..#.",
                "#...#"],
        'U' => ["#...#",
                "#...#",
                "#...#",
                "#...#",
                "#...#",
                "#...#",
                ".###."],
        'I' => ["#####",
                "..#..",
                "..#..",
                "..#..",
                "..#..",
                "..#..",
                "#####"],
        'T' => ["#####",
                "..#..",
                "..#..",
                "..#..",
                "..#..",
                "..#..",
                "..#.."],
        'S' => [".####",
                "#....",
                "#....",
                ".###.",
                "....#",
                "....#",
                "####."],
        'P' => ["####.",
                "#...#",
                "#...#",
                "####.",
                "#....",
                "#....",
                "#...."],
        'L' => ["#....",
                "#....",
                "#....",
                "#....",
                "#....",
                "#....",
                "#####"],
        'A' => [".###.",
                "#...#",
                "#...#",
                "#####",
                "#...#",
                "#...#",
                "#...#"],
        _ => return None,
    })
}

/// Width of the wordmark in grid cells, gaps and the word break included.
fn title_width_cells(text: &str) -> f32 {
    let mut cells = 0.0;
    for (i, ch) in text.chars().enumerate() {
        if i > 0 {
            cells += TITLE_GAP_COLS;
        }
        cells += match title_glyph(ch) {
            Some(_) => TITLE_COLS as f32,
            None => TITLE_SPACE_COLS,
        };
    }
    cells
}

/// A repeatable -1..1 from an index, so the scatter is fixed for a given berry.
fn wobble(seed: u32) -> f32 {
    let n = seed.wrapping_mul(2_654_435_761);
    ((n >> 8) & 0xffff) as f32 / 32_768.0 - 1.0
}

fn draw_fruit_title(center: Vec2, cell: f32) {
    let mut x = center.x - title_width_cells(TITLE_TEXT) * cell * 0.5;
    let top = center.y - TITLE_ROWS as f32 * cell * 0.5;
    let mut letter = 0usize;

    for (i, ch) in TITLE_TEXT.chars().enumerate() {
        if i > 0 {
            x += TITLE_GAP_COLS * cell;
        }

        let Some(glyph) = title_glyph(ch) else {
            x += TITLE_SPACE_COLS * cell;
            continue;
        };

        let body = TITLE_FRUIT[letter % TITLE_FRUIT.len()].body();
        letter += 1;

        for (row, cells) in glyph.iter().enumerate() {
            for (col, lit) in cells.chars().enumerate() {
                if lit != '#' {
                    continue;
                }

                let seed = (i * 97 + row * 13 + col) as u32;
                let at = vec2(
                    x + (col as f32 + 0.5) * cell + wobble(seed) * cell * 0.06,
                    top + (row as f32 + 0.5) * cell + wobble(seed + 1) * cell * 0.06,
                );
                // Just over half a cell, so neighbouring berries touch and a
                // letter reads as one bunch rather than a row of loose dots.
                let r = cell * 0.52 * (1.0 + wobble(seed + 2) * 0.06);

                shaded_ball(at, r, body);
                specular(at, r);
            }
        }

        x += TITLE_COLS as f32 * cell;
    }
}

pub fn draw_menu() {
    let cy = VIEW_H * 0.42;
    draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, Color::new(0.0, 0.0, 0.0, 0.5));

    let cell = VIEW_W * TITLE_SPAN / title_width_cells(TITLE_TEXT);
    draw_fruit_title(vec2(VIEW_W * 0.5, cy - 96.0), cell);

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
        VIEW_W - dims.width - 14.0,
        VIEW_H - 14.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.38),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rect of route card `i`, so main.rs can hit-test clicks against the same
// layout this file draws.
// ─────────────────────────────────────────────────────────────────────────────
// Centred against PLAYFIELD_W, which is 220px narrower than the view — so the
// picker's cards sit left of centre by half that. The shop column is not drawn
// on this screen, so there is nothing beside them to justify it. Left alone
// here because it is a deliberate layout question rather than a scaling one,
// and moving the cards moves what main.rs hit-tests with them.
pub fn track_card_rect(i: usize) -> Rect {
    let w = card_width();
    let row = i / CARDS_PER_ROW;
    let col = i % CARDS_PER_ROW;

    // The last row is usually short, so it gets centred on its own count rather
    // than left-aligned under a full row above it.
    let in_row = (card_count() - row * CARDS_PER_ROW).min(CARDS_PER_ROW) as f32;
    let total = in_row * w + (in_row - 1.0) * CARD_GAP;
    let x0 = (PLAYFIELD_W - total) * 0.5;

    Rect::new(
        x0 + col as f32 * (w + CARD_GAP),
        CARD_Y + row as f32 * (CARD_H + CARD_GAP),
        w,
        CARD_H,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// How wide each route card is: whatever divides the window evenly once the
// margins and the gaps between cards are taken out.
// ─────────────────────────────────────────────────────────────────────────────
fn card_width() -> f32 {
    let n = CARDS_PER_ROW as f32;
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
    draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, Color::new(0.0, 0.0, 0.0, 0.62));

    // The screen's vertical rhythm, top to bottom: title, subtitle, difficulty
    // caption, the difficulty row at MODE_BTN_Y, then the card rows at CARD_Y.
    // Set explicitly rather than nudged, because two rows of cards leave far
    // less slack above them than one did.
    text_center("CHOOSE YOUR ROUTE", PICK_TITLE_Y, 56.0, WHITE);
    text_center(
        "Longer routes give your towers more time to shoot",
        PICK_SUBTITLE_Y,
        23.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    // Spells out that this applies to any route, because each card carries its
    // own difficulty word too, and "Hard" would otherwise mean two things on one
    // screen: how punishing the track is, versus how much you start with.
    text_center(
        "DIFFICULTY  -  applies to whichever route you pick",
        PICK_CAPTION_Y,
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

    draw_random_card();

    text_center(
        &format!("click a route, or press 1-{}", card_count()),
        card_rows_bottom() + 34.0,
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
// The card that starts a run on a route chosen at random.
//
// Deliberately looks like a route card rather than a button, because that is
// what it is: another way to answer the same question. It shows every route's
// outline layered faintly on top of itself, which is both a preview of "any of
// these" and the only honest picture of a choice not yet made.
// ─────────────────────────────────────────────────────────────────────────────
fn draw_random_card() {
    let r = track_card_rect(random_card_index());
    let hovered = r.contains(mouse_vec());

    let bg = if hovered {
        Color::new(0.26, 0.24, 0.30, 1.0)
    } else {
        Color::new(0.17, 0.16, 0.22, 1.0)
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(
        r.x,
        r.y,
        r.w,
        r.h,
        if hovered { 3.0 } else { 1.5 },
        if hovered {
            Color::new(0.85, 0.80, 1.0, 1.0)
        } else {
            Color::new(0.42, 0.40, 0.52, 1.0)
        },
    );

    draw_text(
        format!("{}. Surprise Me", random_card_index() + 1),
        r.x + 12.0,
        r.y + 26.0,
        19.0,
        WHITE,
    );

    // Every route's shape, stacked and faint.
    let inner_x = r.x + 10.0;
    let inner_y = r.y + 40.0;
    let inner_w = r.w - 20.0;
    let inner_h = 130.0;
    draw_rectangle(
        inner_x,
        inner_y,
        inner_w,
        inner_h,
        Color::new(0.12, 0.12, 0.17, 1.0),
    );

    let scale = (inner_w / PLAYFIELD_W).min(inner_h / PLAYFIELD_H);
    for track in TRACKS.iter() {
        for lane in track.lanes {
            for w in lane.windows(2) {
                let a = vec2(inner_x + w[0].0 * scale, inner_y + w[0].1 * scale);
                let b = vec2(inner_x + w[1].0 * scale, inner_y + w[1].1 * scale);
                draw_line(a.x, a.y, b.x, b.y, 3.0, Color::new(0.62, 0.55, 0.80, 0.30));
            }
        }
    }

    draw_text(
        "one of the six, picked for you",
        r.x + 12.0,
        r.y + 168.0,
        15.0,
        Color::new(0.72, 0.72, 0.80, 1.0),
    );
    draw_text(
        "Random",
        r.x + 12.0,
        r.y + 190.0,
        19.0,
        Color::new(0.80, 0.74, 1.0, 1.0),
    );
    let txt = format!("{} routes", TRACKS.len());
    let dims = measure_text(&txt, None, 17, 1.0);
    draw_text(
        &txt,
        r.x + r.w - dims.width - 12.0,
        r.y + 190.0,
        17.0,
        Color::new(0.75, 0.75, 0.82, 1.0),
    );
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
    let cy = VIEW_H * 0.42;
    draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, Color::new(0.0, 0.0, 0.0, 0.55));

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
    let cy = VIEW_H * 0.42;
    draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, Color::new(0.0, 0.10, 0.03, 0.58));

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
// Draw text horizontally centred on the window at baseline `y`. For the screens
// that cover everything — the title, the picker, the end screens — where the
// shop column isn't drawn and the whole window is the canvas.
// ─────────────────────────────────────────────────────────────────────────────
fn text_center(text: &str, y: f32, size: f32, color: Color) {
    let dims = measure_text(text, None, size as u16, 1.0);
    draw_text(
        text,
        (PLAYFIELD_W + SHOP_PANEL_W - dims.width) * 0.5,
        y,
        size,
        color,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Draw text centred on the *field* rather than the window, for anything shown
// during play. Centring on the window would push it under the shop column and
// leave it looking off to one side of the board it belongs to.
// ─────────────────────────────────────────────────────────────────────────────
fn field_text_center(text: &str, y: f32, size: f32, color: Color) {
    let dims = measure_text(text, None, size as u16, 1.0);
    draw_text(text, (PLAYFIELD_W - dims.width) * 0.5, y, size, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixed window width the layout constants are tuned against.
    const WINDOW_W: f32 = PLAYFIELD_W;

    /// Everything drawn in the shop column has to stay inside it.
    fn inside_panel(r: Rect) -> bool {
        r.x >= PLAYFIELD_W && r.x + r.w <= PLAYFIELD_W + SHOP_PANEL_W
    }

    #[test]
    fn a_surface_the_size_of_the_view_needs_no_scaling_at_all() {
        // The desktop window is fixed at exactly the view's size, so the whole
        // arrangement has to be an identity there — anything else would mean the
        // native build had quietly been rescaled to add phone support.
        let (scale, offset) = view_fit(vec2(VIEW_W, VIEW_H));

        assert_eq!(scale, 1.0);
        assert_eq!(offset, Vec2::ZERO);
        let p = vec2(613.0, 402.0);
        assert_eq!(to_view(vec2(VIEW_W, VIEW_H), p), p);
    }

    #[test]
    fn a_smaller_surface_fits_the_whole_view_inside_itself() {
        // A phone-sized canvas of the same shape: everything shrinks, nothing is
        // cropped, and the corners still map to the corners.
        let surface = vec2(VIEW_W / 4.0, VIEW_H / 4.0);
        let (scale, offset) = view_fit(surface);

        assert!((scale - 0.25).abs() < 1e-6);
        assert_eq!(offset, Vec2::ZERO);
        assert!(to_view(surface, Vec2::ZERO).abs_diff_eq(Vec2::ZERO, 1e-3));
        assert!(to_view(surface, surface).abs_diff_eq(vec2(VIEW_W, VIEW_H), 1e-3));
    }

    #[test]
    fn a_mismatched_surface_centres_the_view_and_bars_the_rest() {
        // Taller than the view's shape, as a phone held upright is: the fit is
        // decided by width, and what is left over is split evenly top and
        // bottom rather than piled at one end.
        let surface = vec2(VIEW_W, VIEW_H * 3.0);
        let (scale, offset) = view_fit(surface);

        assert_eq!(scale, 1.0, "the wider axis should not have driven the fit");
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, VIEW_H, "the bars are not even");

        // A tap in the middle of the visible game is the middle of the view,
        // whatever dead space surrounds it.
        let middle = vec2(VIEW_W * 0.5, VIEW_H * 3.0 * 0.5);
        assert!(to_view(surface, middle).abs_diff_eq(vec2(VIEW_W * 0.5, VIEW_H * 0.5), 1e-3));
    }

    #[test]
    fn every_corner_of_a_phone_sized_surface_round_trips() {
        // The whole point of doing the scaling in one place: a point converted
        // into the view and back has to land where it started, or rendering and
        // hit-testing have drifted apart again.
        for surface in [
            vec2(390.0, 203.0),  // a phone held upright, canvas full width
            vec2(844.0, 390.0),  // the same phone turned sideways
            vec2(1024.0, 768.0), // a tablet, taller in shape than the view
            vec2(VIEW_W, VIEW_H),
        ] {
            let (scale, offset) = view_fit(surface);
            for corner in [
                Vec2::ZERO,
                vec2(VIEW_W, 0.0),
                vec2(0.0, VIEW_H),
                vec2(VIEW_W, VIEW_H),
                vec2(VIEW_W * 0.5, VIEW_H * 0.5),
            ] {
                let on_surface = corner * scale + offset;
                assert!(
                    to_view(surface, on_surface).abs_diff_eq(corner, 1e-2),
                    "{corner:?} did not survive the round trip on a {surface:?} surface"
                );
            }
        }
    }

    #[test]
    fn a_tower_panel_and_its_buttons_stay_out_of_the_shop_column() {
        // main.rs hands every click at x >= PLAYFIELD_W to the shop before it
        // ever asks the panel, so any part of a panel past that line is drawn
        // but cannot be clicked. That is invisible to look at — the panel is on
        // top and appears perfectly normal — and it silently cost the towers on
        // the right quarter of the map their upgrade and sell buttons.
        let mut x = 0.0;
        while x <= PLAYFIELD_W {
            let mut y = 0.0;
            while y <= PLAYFIELD_H {
                let panel = tower_panel_rect(vec2(x, y));

                for (what, r) in [
                    ("panel", panel),
                    ("upgrade button", panel_upgrade_button(panel)),
                    ("sell button", panel_sell_button(panel)),
                ] {
                    assert!(
                        r.x >= 0.0 && r.x + r.w <= PLAYFIELD_W,
                        "{what} for a tower at ({x}, {y}) reaches into the shop \
                         column: {}..{}",
                        r.x,
                        r.x + r.w
                    );
                    assert!(
                        r.y >= 0.0 && r.y + r.h <= PLAYFIELD_H,
                        "{what} for a tower at ({x}, {y}) leaves the playfield"
                    );
                }

                y += 25.0;
            }
            x += 25.0;
        }
    }

    #[test]
    fn every_letter_of_the_title_has_a_glyph_the_right_shape() {
        // A missing glyph is silent — the letter is simply not drawn, and the
        // rest of the wordmark closes up over the hole — so it is worth pinning
        // rather than trusting an eyeball on the menu.
        for ch in TITLE_TEXT.chars().filter(|c| *c != ' ') {
            let glyph = title_glyph(ch).unwrap_or_else(|| panic!("no glyph for {ch:?}"));
            for row in glyph {
                assert_eq!(
                    row.chars().count(),
                    TITLE_COLS,
                    "{ch:?} has a row that is not {TITLE_COLS} cells wide"
                );
                assert!(
                    row.chars().all(|c| c == '#' || c == '.'),
                    "{ch:?} has a row using something other than # and ."
                );
            }
        }
        // The space is the one character with no glyph; anything else missing
        // one would be a typo in TITLE_TEXT.
        assert!(title_glyph(' ').is_none());
    }

    #[test]
    fn the_fruit_title_fits_across_the_window() {
        let win_w = PLAYFIELD_W + SHOP_PANEL_W;
        let cells = title_width_cells(TITLE_TEXT);
        let cell = win_w * TITLE_SPAN / cells;

        assert!(cells > 0.0, "the wordmark measured as empty");
        assert!(
            cells * cell < win_w,
            "the wordmark is wider than the window"
        );
        // Tall enough to read as fruit rather than confetti: a berry is about
        // half a cell across, and much under this they stop being legible.
        assert!(
            TITLE_ROWS as f32 * cell >= 90.0,
            "the wordmark is too short to read as fruit"
        );
    }

    #[test]
    fn every_shop_button_sits_inside_the_column() {
        for i in 0..TowerKind::ALL.len() {
            let r = shop_button_rect(i);
            assert!(inside_panel(r), "tower button {i} escapes the column");
            assert!(
                r.y >= 0.0 && r.y + r.h <= PLAYFIELD_H,
                "tower button {i} runs off the window"
            );
        }
    }

    #[test]
    fn shop_buttons_stack_without_overlapping() {
        for i in 1..TowerKind::ALL.len() {
            let prev = shop_button_rect(i - 1);
            let cur = shop_button_rect(i);
            assert!(
                cur.y >= prev.y + prev.h,
                "tower buttons {} and {i} overlap",
                i - 1
            );
        }
    }

    #[test]
    fn the_towers_clear_the_control_block_below_them() {
        // The controls are anchored to the bottom of the window and the towers
        // grow downward from the top, so this is the check that says how many
        // more towers the column can still take.
        let last = shop_button_rect(TowerKind::ALL.len() - 1);
        let top_control = ctrl_row_rect(CTRL_ROWS - 1);

        assert!(
            last.y + last.h < top_control.y,
            "the tower buttons have grown into the controls"
        );
        // Room left for the hint lines that sit between them.
        assert!(
            top_control.y - (last.y + last.h) >= 60.0,
            "no room left between the last tower and the controls"
        );
    }

    #[test]
    fn every_control_row_sits_inside_the_column_and_the_window() {
        for row in 0..CTRL_ROWS {
            let r = ctrl_row_rect(row);
            assert!(inside_panel(r), "control row {row} escapes the column");
            assert!(
                r.y >= 0.0 && r.y + r.h <= PLAYFIELD_H,
                "control row {row} runs off the window"
            );
        }
    }

    #[test]
    fn control_rows_do_not_overlap() {
        for row in 1..CTRL_ROWS {
            let lower = ctrl_row_rect(row - 1);
            let upper = ctrl_row_rect(row);
            assert!(
                upper.y + upper.h <= lower.y,
                "control rows {} and {row} overlap",
                row - 1
            );
        }
    }

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
    fn route_cards_never_overlap_and_stay_on_screen() {
        let win_w = PLAYFIELD_W + SHOP_PANEL_W;
        for i in 0..card_count() {
            let a = track_card_rect(i);
            assert!(
                a.x >= 0.0 && a.x + a.w <= win_w,
                "card {i} runs off the side"
            );
            assert!(
                a.y >= 0.0 && a.y + a.h <= PLAYFIELD_H,
                "card {i} runs off the bottom"
            );

            for j in (i + 1)..card_count() {
                let b = track_card_rect(j);
                let apart =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "cards {i} and {j} overlap");
            }
        }
    }

    #[test]
    fn no_route_card_reaches_under_the_audio_toggles() {
        // The audio toggles are drawn on every screen, including this one, and
        // the panel controls are hit-tested before anything else. A card sliding
        // under one would silently lose its clicks to it.
        let sfx = audio_button_rect(0);
        for i in 0..card_count() {
            let c = track_card_rect(i);
            let apart = c.x + c.w <= sfx.x || c.y + c.h <= sfx.y || sfx.y + sfx.h <= c.y;
            assert!(apart, "card {i} reaches under the audio toggles");
        }
    }

    #[test]
    fn the_cards_use_two_rows() {
        // The point of the change: one row of seven left each card too narrow to
        // read. If a route is ever removed this should fail rather than silently
        // going back to a single cramped row.
        assert!(
            card_count() > CARDS_PER_ROW,
            "the cards fit on one row again"
        );
        assert_eq!(card_count().div_ceil(CARDS_PER_ROW), 2);
    }

    #[test]
    fn the_heading_block_clears_the_difficulty_row() {
        // The heading baselines are fixed and the difficulty row is computed, so
        // this is the join where they can actually drift apart — and did, twice,
        // while two rows of cards were being fitted above the fold.
        assert!(
            mode_button_rect(0).y >= PICK_CAPTION_Y,
            "the difficulty row sits on its own caption"
        );
    }

    #[test]
    fn the_mode_row_clears_the_first_row_of_cards() {
        for i in 0..MODES.len() {
            let m = mode_button_rect(i);
            assert!(m.y + m.h <= CARD_Y, "the difficulty row overlaps the cards");
        }
    }

    #[test]
    fn the_hint_below_the_cards_still_fits_the_window() {
        assert!(
            card_rows_bottom() + 34.0 <= PLAYFIELD_H - 8.0,
            "the 'click a route' line falls off the bottom"
        );
    }

    #[test]
    fn the_random_card_is_last_and_is_not_a_route() {
        // It must sit past every route so adding one never shifts it, and it
        // must not be mistakable for an index into TRACKS.
        assert_eq!(random_card_index(), TRACKS.len());
        assert_eq!(card_count(), TRACKS.len() + 1);
    }

    #[test]
    fn every_card_can_be_reached_from_the_keyboard() {
        // Route five was once unreachable because the cards outgrew the keys.
        assert!(
            card_count() <= crate::NUMBER_KEYS.len(),
            "{} cards but only {} number keys",
            card_count(),
            crate::NUMBER_KEYS.len()
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
    fn the_audio_buttons_do_not_overlap_each_other() {
        let (sfx, music) = (audio_button_rect(0), audio_button_rect(1));
        assert!(music.x >= sfx.x + sfx.w, "the two toggles overlap");
    }

    #[test]
    fn every_drawn_string_is_ascii() {
        // macroquad's default font has no glyph beyond ASCII: an em dash, a
        // curly quote or an accent draws as a tofu box. That has reached the
        // screen twice now — once in the send-wave prompt and once in the pause
        // overlay — so this scans this file's own string literals rather than
        // trusting anyone to remember.
        //
        // Comment lines are skipped, which is why the box-drawing rules and the
        // em dashes in the prose above are fine. Only render.rs is scanned
        // because only render.rs draws text; the balance report prints to a
        // terminal, which has a real font.
        const QUOTE: u32 = 34;

        for (n, line) in include_str!("render.rs").lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }

            let mut in_string = false;
            let mut escaped = false;
            for ch in line.chars() {
                if escaped {
                    escaped = false;
                    continue;
                }
                if in_string && ch == '\\' {
                    escaped = true;
                } else if ch as u32 == QUOTE {
                    in_string = !in_string;
                } else if in_string && !ch.is_ascii() {
                    panic!(
                        "render.rs line {}: {:?} is not ASCII and will draw as a tofu box",
                        n + 1,
                        ch
                    );
                }
            }
        }
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
    fn no_two_panel_controls_overlap() {
        // Audio, pause, auto and quit all share the block at the foot of the
        // column, and all take a click before the field does. Two overlapping
        // would make which one fires depend on their order in the code rather
        // than on what was actually hit.
        let rects = [
            ("sfx", audio_button_rect(0)),
            ("music", audio_button_rect(1)),
            ("pause", pause_button_rect()),
            ("auto", auto_button_rect()),
            ("quit", quit_button_rect()),
        ];

        for (i, (an, a)) in rects.iter().enumerate() {
            for (bn, b) in rects.iter().skip(i + 1) {
                let apart =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "the {an} and {bn} buttons overlap");
            }
        }
    }

    #[test]
    fn the_panel_controls_never_reach_into_the_field() {
        // A click left of PLAYFIELD_W is a field click. Anything in this block
        // straying over that line would place a tower as well as press itself.
        for (name, r) in [
            ("sfx", audio_button_rect(0)),
            ("music", audio_button_rect(1)),
            ("pause", pause_button_rect()),
            ("auto", auto_button_rect()),
            ("quit", quit_button_rect()),
        ] {
            assert!(r.x >= PLAYFIELD_W, "the {name} button overhangs the field");
        }
    }

    #[test]
    fn the_auto_label_fits_its_button() {
        assert!(
            AUTO_LABEL.len() as f32 * 8.0 + 12.0 <= ctrl_row_rect(CTRL_ROW_AUTO).w,
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
