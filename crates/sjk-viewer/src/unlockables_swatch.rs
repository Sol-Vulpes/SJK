//! The Unlockables page's live swatches: each unlockable drawn small with the UI's
//! shapes, moving as it does in the game. A blade skin lies across its swatch from a
//! steel hilt, drawn from its loaded blade-skin file (`blade_skin_file.rs`): a hot core
//! in a corona graded from the file's inside colour to its rim, breathing as its light
//! flickers in a soft haze, flame loops rising off it, granules drifting along it and,
//! when the skin has flares, now and then a bright flare running from the hilt to the
//! tip (as `saber.wgsl` animates the real blade). A locked one is drawn grey and still,
//! under a padlock; an owned one whose pack has not come yet is drawn still in neutral
//! grey, and the page says its look downloads from the SJK hub.

use crate::blade_skin_file::{BladeSkinDef, Rgb};
use crate::menu::sjk::{Frame, color};
use crate::menu_widgets::MenuCanvas;
use crate::saber_skins::LoadedSkin;
use sjk_ui::{Color, DrawCommand, Gradient};

/// The share of each flare cycle its knot runs.
const FLARE_RUN: f32 = 0.62;
/// Layers of the soft haze and of the flare's light; flame loops and granules along
/// the corona.
const HAZE_LAYERS: usize = 9;
const FLARE_LAYERS: usize = 8;
const PROMINENCES: usize = 6;
const GRANULES: usize = 12;

fn push(canvas: &mut MenuCanvas, command: DrawCommand) {
    let _ = canvas.draw_list_mut().push(command);
}

/// A filled rounded rectangle, radius half its height (frame pixels).
fn capsule(canvas: &mut MenuCanvas, frame: &Frame, rect: [f32; 4], colour: Color) {
    let [x, y, width, height] = rect;
    let rect = frame.rect(x, y, width, height);
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect,
            radius: rect.height.min(rect.width) * 0.5,
            color: colour,
        },
    );
}

/// A filled disc of `radius` round (`x`, `y`) (frame pixels).
fn disc(canvas: &mut MenuCanvas, frame: &Frame, x: f32, y: f32, radius: f32, colour: Color) {
    capsule(
        canvas,
        frame,
        [x - radius, y - radius, radius * 2.0, radius * 2.0],
        colour,
    );
}

/// `value`'s fractional part.
fn fract(value: f32) -> f32 {
    value - value.floor()
}

/// What a swatch shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Shown {
    /// The skin, alive.
    Skin,
    /// Locked: grey and still, under a padlock.
    Locked,
    /// Owned, but its pack has not come from the hub yet: neutral and still.
    Waiting,
}

/// Draw a blade skin across the swatch `rect` (frame pixels) at `seconds`: `skin` its
/// loaded file, if its pack is loaded; `owned` lights it. Says what it showed.
pub(super) fn blade(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    skin: Option<&LoadedSkin>,
    owned: bool,
    seconds: f32,
) -> Shown {
    let [x, y, width, height] = rect;
    let s = frame.s;
    // The swatch's own dark glass, so the glow reads the same over any map.
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 14.0 * s,
            color: Color::new(0.016, 0.024, 0.05, 0.94),
        },
    );
    let centre = y + height * 0.5;
    let hilt = 84.0;
    let start = x + 26.0 + hilt;
    let length = width - (start - x) - 34.0;
    let shown = match (owned, skin) {
        (true, Some(skin)) => {
            lit_blade(canvas, frame, &skin.def, start, centre, length, seconds);
            Shown::Skin
        }
        (true, None) => {
            grey_blade(canvas, frame, start, centre, length, 1.6);
            Shown::Waiting
        }
        (false, _) => {
            grey_blade(canvas, frame, start, centre, length, 1.0);
            Shown::Locked
        }
    };
    hilt_at(canvas, frame, start, centre, hilt, owned);
    push(
        canvas,
        DrawCommand::Border {
            rect: frame.rect(x, y, width, height),
            radius: 14.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::HOLO, if owned { 0.22 } else { 0.14 }),
        },
    );
    if !owned {
        padlock(canvas, frame, x + width * 0.5, centre);
    }
    shown
}

/// `a` to `b` by `t`.
fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|channel| a[channel] + (b[channel] - a[channel]) * t)
}

/// A colour of the skin's own gradient, `u` from its rim (0) through its rim's heat and
/// its inside to its hot core (1).
fn gradient(def: &BladeSkinDef, u: f32) -> Rgb {
    let stops = [
        def.corona.rim_cool,
        def.corona.rim_hot,
        def.corona.inner,
        def.core.white,
    ];
    let at = u.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let index = (at.floor() as usize).min(stops.len() - 2);
    mix(stops[index], stops[index + 1], at - index as f32)
}

/// A UI colour of `rgb` (clamped) at `alpha`.
fn ui(rgb: Rgb, alpha: f32) -> Color {
    Color::new(
        rgb[0].clamp(0.0, 1.0),
        rgb[1].clamp(0.0, 1.0),
        rgb[2].clamp(0.0, 1.0),
        alpha,
    )
}

/// The skin `def` from `start` along `length`, centred on `centre`, at `t` seconds.
fn lit_blade(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    def: &BladeSkinDef,
    start: f32,
    centre: f32,
    length: f32,
    t: f32,
) {
    let s = frame.s;
    const WHITE: Rgb = [1.0; 3];
    // The corona breathes slowly and unevenly, as the blade's light flickers.
    let breath = 1.0
        + def
            .light
            .flicker
            .waves
            .iter()
            .enumerate()
            .map(|(index, wave)| 0.05 * (t * wave.rate + 1.7 * index as f32).sin() * wave.weight)
            .sum::<f32>();
    let end = start + length;
    // The light it casts on the swatch, and its haze: many faint layers, so the glow
    // falls off softly instead of in bands.
    for step in 0..HAZE_LAYERS {
        let k = step as f32 / (HAZE_LAYERS - 1) as f32;
        let half = (64.0 - 46.0 * k) * breath;
        let warm = mix(def.corona.rim_cool, def.corona.inner, 0.2 + 0.5 * k);
        capsule(
            canvas,
            frame,
            [
                start - 6.0 - half * 0.25,
                centre - half,
                length + 6.0 + half * 0.6,
                half * 2.0,
            ],
            ui(warm, 0.026 + 0.03 * k),
        );
    }
    // Flame loops rising off the corona and sinking back, drifting tipward, above and
    // below in turn, when the skin has tongues.
    let corona = 13.0 * breath;
    if def.tongues.range != 0.0 {
        for index in 0..PROMINENCES {
            let i = index as f32;
            let life = fract(t * (0.21 + 0.03 * i) + i * 0.37);
            let along = fract(i * 0.29 + 0.13 + t * 0.035);
            let rise = (life * std::f32::consts::PI).sin();
            let radius = 3.0 + 4.5 * rise;
            let px = start + 14.0 + along * (length - 28.0);
            let above = index % 2 == 0;
            let py = if above {
                centre - corona + 4.0
            } else {
                centre + corona - 4.0
            };
            push(
                canvas,
                DrawCommand::Arc {
                    center: frame.point(px, py),
                    radius: radius * s,
                    width: (1.4 + 1.2 * rise) * s,
                    start: if above { std::f32::consts::PI } else { 0.0 },
                    sweep: std::f32::consts::PI,
                    color: ui(
                        mix(def.corona.rim_hot, def.corona.inner, 0.45 * rise),
                        0.38 * rise,
                    ),
                    knockout: None,
                },
            );
        }
    }
    // The corona, rim into the inside and the glow by the core: stacked, so each band
    // is a step of a few pixels.
    for (scale, u, alpha) in [
        (1.0, 0.2, 0.55),
        (0.8, 0.4, 0.72),
        (0.62, 0.6, 0.86),
        (0.46, 0.8, 0.95),
    ] {
        let half = corona * scale;
        capsule(
            canvas,
            frame,
            [start, centre - half, length + half * 0.4, half * 2.0],
            ui(gradient(def, u), alpha),
        );
    }
    // Granulation: bright cells drifting along the glow.
    for index in 0..GRANULES {
        let i = index as f32;
        let along = fract(i * 0.0833 + 0.11 * (i * 1.7).sin() + t * 0.045);
        let wobble = (t * 1.9 + i * 2.3).sin() * corona * 0.5;
        let shine = 0.5 + 0.5 * (t * 3.3 + i * 1.3).sin();
        disc(
            canvas,
            frame,
            start + 6.0 + along * (length - 12.0),
            centre + wobble,
            1.4 + 1.1 * shine,
            ui(gradient(def, 0.9), 0.18 + 0.4 * shine),
        );
    }
    // The hot core, its fringe at its very edge.
    capsule(
        canvas,
        frame,
        [start, centre - 4.4, length, 8.8],
        ui(gradient(def, 0.9), 1.0),
    );
    capsule(
        canvas,
        frame,
        [start + 1.0, centre - 2.6, length - 2.0, 5.2],
        ui(mix(def.core.white, WHITE, 0.5), 1.0),
    );
    // A flare: a bright knot running from the hilt to the tip once a cycle of the skin's
    // first track, swelling and fading, its light soft round it.
    let phase = fract(t * def.flares.rate);
    if def.flares.count > 0 && def.flares.rate > 0.0 && phase < FLARE_RUN {
        let run = phase / FLARE_RUN;
        let eased = run * run * (3.0 - 2.0 * run);
        let fx = start + eased * length;
        let strength = (run * std::f32::consts::PI).sin();
        for step in 0..FLARE_LAYERS {
            let k = step as f32 / (FLARE_LAYERS - 1) as f32;
            let half = (5.0 + 25.0 * (1.0 - k) * (1.0 - k)) * (0.4 + 0.6 * strength);
            capsule(
                canvas,
                frame,
                [fx - half * 1.5, centre - half, half * 3.0, half * 2.0],
                ui(mix(gradient(def, 0.5), WHITE, 0.9 * k), 0.07 * strength),
            );
        }
        let streak = 10.0 + 30.0 * strength;
        capsule(
            canvas,
            frame,
            [fx - streak, centre - 4.0, streak * 2.0, 8.0],
            ui(mix(def.core.white, WHITE, 0.3), 0.85 * strength),
        );
        disc(
            canvas,
            frame,
            fx,
            centre,
            2.0 + 5.0 * strength,
            Color::new(1.0, 1.0, 1.0, strength),
        );
    }
    // A glint at the tip.
    disc(
        canvas,
        frame,
        end + corona * 0.2,
        centre,
        3.0 + 1.5 * breath,
        ui(gradient(def, 0.85), 0.55),
    );
}

/// A grey, still blade: locked (`light` 1), or owned with its look still to come
/// (brighter).
fn grey_blade(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    start: f32,
    centre: f32,
    length: f32,
    light: f32,
) {
    capsule(
        canvas,
        frame,
        [start - 4.0, centre - 15.0, length + 12.0, 30.0],
        color::alpha(color::QUIET, 0.08 * light),
    );
    capsule(
        canvas,
        frame,
        [start, centre - 6.0, length, 12.0],
        color::alpha(color::QUIET, 0.3 * light),
    );
    capsule(
        canvas,
        frame,
        [start + 1.0, centre - 2.5, length - 2.0, 5.0],
        color::alpha(color::MUTED, (0.45 * light).min(1.0)),
    );
}

/// The steel hilt ending at `start`, `length` long.
fn hilt_at(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    start: f32,
    centre: f32,
    length: f32,
    lit: bool,
) {
    let s = frame.s;
    let shade = if lit { 1.0 } else { 0.55 };
    let steel = |level: f32| {
        Color::new(
            level * shade,
            (level + 0.02) * shade,
            (level + 0.06) * shade,
            1.0,
        )
    };
    let x = start - length;
    // The pommel, the grip and the emitter's shroud.
    let rect = frame.rect(x, centre - 8.0, length - 12.0, 16.0);
    push(
        canvas,
        DrawCommand::GradientRect {
            rect,
            radius: 4.0 * s,
            gradient: Gradient {
                start: steel(0.78),
                end: steel(0.3),
                vertical: true,
            },
        },
    );
    for ridge in 0..5 {
        let rx = x + 22.0 + ridge as f32 * 8.0;
        push(
            canvas,
            DrawCommand::SolidRect {
                rect: frame.rect(rx, centre - 8.0, 3.0, 16.0),
                color: Color::new(0.06, 0.07, 0.1, 0.85),
            },
        );
    }
    let rect = frame.rect(start - 14.0, centre - 11.0, 14.0, 22.0);
    push(
        canvas,
        DrawCommand::GradientRect {
            rect,
            radius: 3.0 * s,
            gradient: Gradient {
                start: steel(0.86),
                end: steel(0.36),
                vertical: true,
            },
        },
    );
    // The activator, gold.
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x + 66.0, centre - 10.5, 7.0, 4.0),
            radius: 1.5 * s,
            color: if lit {
                color::GOLD
            } else {
                color::alpha(color::QUIET, 0.8)
            },
        },
    );
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x - 4.0, centre - 9.0, 6.0, 18.0),
            radius: 2.0 * s,
            color: steel(0.5),
        },
    );
}

/// A padlock centred on (`x`, `y`).
fn padlock(canvas: &mut MenuCanvas, frame: &Frame, x: f32, y: f32) {
    let s = frame.s;
    disc(canvas, frame, x, y, 30.0, color::alpha(color::SPACE, 0.82));
    push(
        canvas,
        DrawCommand::Arc {
            center: frame.point(x, y - 5.0),
            radius: 7.0 * s,
            width: 3.0 * s,
            start: std::f32::consts::PI,
            sweep: std::f32::consts::PI,
            color: color::MUTED,
            knockout: None,
        },
    );
    for side in [-7.0, 7.0] {
        push(
            canvas,
            DrawCommand::SolidRect {
                rect: frame.rect(x + side - 1.5, y - 5.0, 3.0, 5.0),
                color: color::MUTED,
            },
        );
    }
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x - 11.0, y - 1.0, 22.0, 17.0),
            radius: 3.0 * s,
            color: color::MUTED,
        },
    );
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x - 2.0, y + 4.0, 4.0, 7.0),
            radius: 2.0 * s,
            color: color::SPACE,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(skin: Option<&LoadedSkin>, owned: bool, seconds: f32) -> (Shown, Vec<DrawCommand>) {
        let mut canvas = MenuCanvas::with_capacities(8, 32, 400);
        canvas.begin_transparent([1920.0, 1080.0]);
        let frame = Frame::new([1920.0, 1080.0]);
        let shown = blade(
            &mut canvas,
            &frame,
            [100.0, 100.0, 380.0, 200.0],
            skin,
            owned,
            seconds,
        );
        (shown, canvas.draw_list().commands().to_vec())
    }

    #[test]
    fn a_loaded_skin_moves_and_a_locked_or_waiting_one_is_still() {
        let skins = crate::saber_skins::tests::loaded_sample(1);
        let skin = skins.get("saber_sun");
        assert!(skin.is_some());
        let (shown, a) = drawn(skin, true, 0.3);
        assert_eq!(shown, Shown::Skin);
        assert_ne!(a, drawn(skin, true, 1.1).1, "the owned swatch is alive");
        assert_eq!(
            drawn(skin, false, 0.3),
            drawn(skin, false, 1.1),
            "locked is still"
        );
        assert_eq!(drawn(skin, false, 0.3).0, Shown::Locked);
        let (waiting, still) = drawn(None, true, 0.3);
        assert_eq!(waiting, Shown::Waiting);
        assert_eq!(
            still,
            drawn(None, true, 1.1).1,
            "waiting for its pack is still"
        );
        // The flare runs part of each cycle: more shapes then.
        let period = 1.0 / skin.unwrap().def.flares.rate;
        let resting = drawn(skin, true, period * 0.9).1.len();
        assert!(drawn(skin, true, period * 0.3).1.len() > resting);
        // Its colours are the skin's: the sample is teal, never red.
        let teal = drawn(skin, true, 0.3).1.iter().any(|command| {
            matches!(command, DrawCommand::RoundedRect { color, .. }
                if color.g > color.r && color.b > color.r && color.a > 0.9)
        });
        assert!(teal);
        // Every shape stays inside the swatch.
        let frame = Frame::new([1920.0, 1080.0]);
        let swatch = frame.rect(100.0, 100.0, 380.0, 200.0);
        for seconds in [0.0, 0.7, 1.4, 2.2, 9.9] {
            for command in drawn(skin, true, seconds).1 {
                let rect = match command {
                    DrawCommand::RoundedRect { rect, .. }
                    | DrawCommand::GradientRect { rect, .. }
                    | DrawCommand::SolidRect { rect, .. }
                    | DrawCommand::Border { rect, .. } => rect,
                    _ => continue,
                };
                assert!(
                    rect.x >= swatch.x - 0.5
                        && rect.y >= swatch.y - 0.5
                        && rect.right() <= swatch.right() + 0.5
                        && rect.y + rect.height <= swatch.y + swatch.height + 0.5,
                    "{rect:?} at {seconds}"
                );
            }
        }
    }
}
