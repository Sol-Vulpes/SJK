//! The swatch's share of the later blade-skin sections (`blade_skin_effects.rs`), drawn
//! small with the UI's shapes as `saber.wgsl` animates them on the real blade: afterimages
//! trailing it, embers falling off it, veins along its core, a hologram's scan lines and
//! wireframe, glitched blocks with their colours split, and glyphs (the swatch spells
//! "SJK", having no wearer). The heartbeat, the sputtering cut and the team or
//! surroundings tint change the blade itself ([`Shaping`]).

use super::{BLADE_UNITS, capsule, disc, draw, fract, mix, stroke};
use crate::blade_skin_file::{BladeSkinDef, Rgb};
use crate::menu::sjk::Frame;
use sjk_ui::Color;
use sjk_ui::DrawList;

/// How the later sections change the swatch's blade at a moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Shaping {
    /// The heartbeat's brightening (0 at rest).
    pub(super) beat: f32,
    /// The share of the blade showing (1 unless it sputters).
    pub(super) shown: f32,
    /// A colour every colour is drawn toward, and how much.
    pub(super) tint: Option<(Rgb, f32)>,
    /// A chroma's turn to its wearer's colour (`saber_skins::Chroma`); 0 for none.
    pub(super) turn: f32,
}

impl Shaping {
    /// `def`'s shaping at `t` seconds.
    pub(super) fn of(def: &BladeSkinDef, t: f32) -> Self {
        let beat = def.pulse.map_or(0.0, |pulse| {
            crate::saber_skins::Beat {
                rate: pulse.rate,
                amount: pulse.amount,
                second: pulse.second,
                gap: pulse.gap,
                width: pulse.width,
            }
            .at(t)
        });
        let shown = def.sputter.map_or(1.0, |sputter| {
            let cycle = (t * sputter.rate).floor();
            if sputter.rate <= 0.0 || draw(cycle + 0.37) < sputter.threshold {
                1.0
            } else {
                1.0 - sputter.cut * draw(cycle * 1.7 + 4.1)
            }
        });
        // Outside a team (the swatch has no wearer) the team's `none`; the light of the
        // places a blade goes, slowly passing from warm to cool and back.
        let tint = def.team.map(|team| (team.none, team.amount)).or_else(|| {
            def.ambient.map(|ambient| {
                let light = crate::saber_skins::turn_hue([1.0, 0.55, 0.25], t * 0.05);
                let top = light.into_iter().fold(0.001, f32::max);
                (light.map(|c| (c / top).max(ambient.floor)), ambient.amount)
            })
        });
        Self {
            beat,
            shown,
            tint,
            turn: 0.0,
        }
    }

    /// `rgb` drawn toward the tint at its own strongest channel's level, as
    /// `saber.wgsl`'s `skin_apply_tint`.
    /// A chroma's colours are turned to its wearer's first.
    pub(super) fn tinted(self, rgb: Rgb) -> Rgb {
        let rgb = if self.turn == 0.0 {
            rgb
        } else {
            crate::saber_skins::turn_hue(rgb, self.turn)
        };
        match self.tint {
            Some((tint, amount)) => {
                let level = rgb.into_iter().fold(0.0, f32::max);
                mix(rgb, tint.map(|c| c * level), amount)
            }
            None => rgb,
        }
    }
}

/// The blade's afterimages, behind it: the corona drawn again above it, fainter each
/// time, as if it had just swung down.
pub(super) fn ghosts(
    canvas: &mut DrawList,
    frame: &Frame,
    def: &BladeSkinDef,
    at: [f32; 2],
    length: f32,
    corona: f32,
    ui: &dyn Fn(Rgb, f32) -> Color,
) {
    let Some(ghosts) = def.ghosts else {
        return;
    };
    let mut light = ghosts.fade;
    for index in 0..ghosts.count.min(3) {
        let lift = 13.0 * (index + 1) as f32;
        let half = corona * 0.8;
        capsule(
            canvas,
            frame,
            [at[0], at[1] - lift - half, length, half * 2.0],
            ui(def.corona.inner, 0.35 * light),
        );
        light *= ghosts.fade;
    }
}

/// Embers dripping off the blade and falling, accelerating, fading as they go.
pub(super) fn embers(
    canvas: &mut DrawList,
    frame: &Frame,
    def: &BladeSkinDef,
    at: [f32; 2],
    length: f32,
    t: f32,
    ui: &dyn Fn(Rgb, f32) -> Color,
) {
    let Some(embers) = def.embers else {
        return;
    };
    let count = (embers.density * 16.0).round() as usize;
    for index in 0..count {
        let i = index as f32;
        let phase = fract(t / embers.life + draw(i + 2.2));
        let x = at[0]
            + (0.06 + 0.88 * draw(i + 5.5)) * length
            + embers.spread * 10.0 * (draw(i + 8.1) - 0.5) * phase;
        let y = at[1] + 4.0 + 64.0 * phase * phase;
        let fade = (1.0 - phase).powf(1.5);
        disc(
            canvas,
            frame,
            x,
            y,
            1.2 + 1.8 * (1.0 - 0.5 * phase),
            ui(
                embers.color,
                (0.3 + 0.2 * embers.brightness).min(1.0) * fade,
            ),
        );
    }
}

/// Glowing veins along the core: short bright strokes wandering about it, running along.
pub(super) fn veins(
    canvas: &mut DrawList,
    frame: &Frame,
    def: &BladeSkinDef,
    at: [f32; 2],
    length: f32,
    t: f32,
    ui: &dyn Fn(Rgb, f32) -> Color,
) {
    let Some(veins) = def.veins else {
        return;
    };
    const RUNS: usize = 18;
    let shift = t * veins.speed / BLADE_UNITS;
    let point = |run: usize| {
        let k = run as f32 / RUNS as f32;
        let wander = draw((k * RUNS as f32 + (shift * RUNS as f32).floor()).floor() + 3.3) - 0.5;
        [at[0] + k * length, at[1] + wander * 7.0]
    };
    for run in 0..RUNS {
        stroke(
            canvas,
            frame,
            point(run),
            point(run + 1),
            1.6,
            ui(veins.color, (0.35 + 0.15 * veins.brightness).min(1.0)),
        );
    }
}

/// A hologram's scan lines running along the blade, its wireframe sides and rings.
#[allow(clippy::too_many_arguments)]
pub(super) fn scan(
    canvas: &mut DrawList,
    frame: &Frame,
    def: &BladeSkinDef,
    at: [f32; 2],
    length: f32,
    corona: f32,
    t: f32,
    ui: &dyn Fn(Rgb, f32) -> Color,
) {
    let Some(scan) = def.scan else {
        return;
    };
    let side = corona * scan.edge;
    let line = ui(scan.color, (0.3 + 0.12 * scan.brightness).min(0.9));
    for sign in [-1.0, 1.0] {
        stroke(
            canvas,
            frame,
            [at[0], at[1] + sign * side],
            [at[0] + length, at[1] + sign * side],
            1.2,
            line,
        );
    }
    // Rings across, sliding as the scan does.
    let rings = (scan.rings.max(0.15) * BLADE_UNITS * 0.5).round().max(2.0) as usize;
    let slide = fract(t * scan.speed / BLADE_UNITS * 0.5);
    for index in 0..rings {
        let k = (index as f32 + slide) / rings as f32;
        let x = at[0] + k * length;
        stroke(
            canvas,
            frame,
            [x, at[1] - side],
            [x, at[1] + side],
            1.0,
            line,
        );
    }
}

/// The glyphs of "SJK" running along the core toward the tip.
pub(super) fn glyphs(
    canvas: &mut DrawList,
    frame: &Frame,
    def: &BladeSkinDef,
    at: [f32; 2],
    length: f32,
    t: f32,
    ui: &dyn Fn(Rgb, f32) -> Color,
) {
    let Some(glyphs) = def.glyphs else {
        return;
    };
    const LETTERS: [u32; 3] = [19, 10, 11];
    let size = 16.0;
    let cell = size * 1.35;
    let scroll = (t * glyphs.speed / BLADE_UNITS * length).rem_euclid(cell * 4.0);
    let colour = ui(glyphs.color, (0.5 + 0.15 * glyphs.brightness).min(1.0));
    let mut slot = 0usize;
    let mut x = at[0] + 6.0 - cell * 4.0 + scroll;
    while x + size < at[0] + length - 4.0 {
        let letter = slot % 4;
        if x > at[0] + 4.0 && letter < 3 {
            for (a, b) in crate::saber_persona::glyph_strokes(LETTERS[letter]) {
                // Along the glyph is along the blade, across it is up the swatch.
                let point = |p: [f32; 2]| [x + p[1] * size, at[1] + (p[0] - 0.5) * size * 0.6];
                stroke(canvas, frame, point(a), point(b), 1.3, colour);
            }
        }
        x += cell;
        slot += 1;
    }
}

/// Glitched blocks over the blade: on a draw, a block shown again aside, red one way and
/// blue the other.
pub(super) fn glitch(
    canvas: &mut DrawList,
    frame: &Frame,
    def: &BladeSkinDef,
    at: [f32; 2],
    length: f32,
    t: f32,
) {
    let Some(glitch) = def.glitch else {
        return;
    };
    let frame_draw = (t * glitch.rate).floor();
    let blocks = (BLADE_UNITS / glitch.blocks).round().clamp(2.0, 12.0) as usize;
    let width = length / blocks as f32;
    for block in 0..blocks {
        if draw(frame_draw * 1.3 + block as f32 * 7.7) < glitch.threshold {
            continue;
        }
        let jump = (draw(frame_draw + block as f32) - 0.5) * 18.0 * glitch.shift;
        let x = at[0] + block as f32 * width;
        for (offset, colour) in [
            (-2.0 - 6.0 * glitch.split, Color::new(1.0, 0.2, 0.3, 0.55)),
            (2.0 + 6.0 * glitch.split, Color::new(0.2, 0.5, 1.0, 0.55)),
        ] {
            capsule(
                canvas,
                frame,
                [x, at[1] + jump + offset - 3.0, width, 6.0],
                colour,
            );
        }
    }
}
