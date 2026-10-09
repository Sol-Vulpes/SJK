//! Blade-skin files (`docs/unlockables.md`, "Blade-skin files"): the data a blade skin's
//! look and sounds are drawn from. The format is SJK's and open; the files themselves
//! are art, delivered in the SJK hub's packs (`sjk_packs.rs`), not in this repository.
//!
//! `skins/blades/<unlock id>.bladeskin` is a JSON object holding every parameter of the
//! generic blade-skin shading in `saber.wgsl` (colours, corona, granulation, flares,
//! shimmer, flame tongues), the trail and light colours, the light's flicker, the sounds'
//! game paths and, optionally, glow and core images (otherwise the engine generates the
//! grey glow/core pair from the two profiles). Parsing is strict: an unknown field, a
//! missing one or a value out of its range refuses the whole file, and the log names
//! the file and why.

use crate::saber_rgb::{CoreProfile, GlowProfile};
use serde::Deserialize;

/// Where blade-skin files are.
pub(crate) const FOLDER: &str = "skins/blades";
/// Their extension.
pub(crate) const EXTENSION: &str = ".bladeskin";
/// The one version of the format.
const VERSION: u32 = 1;
/// The longest game path (`MAX_QPATH` less its terminator).
const PATH_MAX: usize = 63;
/// Most flare tracks, shimmer waves and light-flicker waves a skin may have.
pub(crate) const MAX_FLARES: u32 = 8;
const MAX_WAVES: usize = 2;
/// Granulation octaves a skin may have.
const MAX_OCTAVES: usize = 2;

/// A linear RGB colour.
pub(crate) type Rgb = [f32; 3];

/// One blade skin, as read and checked.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct BladeSkinDef {
    /// The format's version, 1.
    pub(crate) version: u32,
    /// The generated glow image's profile; the neutral RGB pair's when absent.
    #[serde(default = "neutral_glow")]
    pub(crate) glow_profile: GlowProfile,
    /// The generated core image's profile; the neutral RGB pair's when absent.
    #[serde(default = "neutral_core")]
    pub(crate) core_profile: CoreProfile,
    /// A glow image in the same pack (PNG, TGA or JPEG, grey) instead of the profile's.
    #[serde(default)]
    pub(crate) glow_image: Option<String>,
    /// A core image in the same pack (red: hot core, green: fringe) instead of the
    /// profile's.
    #[serde(default)]
    pub(crate) core_image: Option<String>,
    pub(crate) core: Core,
    pub(crate) corona: Corona,
    pub(crate) granulation: Granulation,
    pub(crate) flares: Flares,
    /// Waves of the corona's breathing, at most two.
    #[serde(default)]
    pub(crate) shimmer: Vec<ShimmerWave>,
    pub(crate) tongues: Tongues,
    /// The blur trail's vertex colour.
    pub(crate) trail: Rgb,
    pub(crate) light: Light,
    pub(crate) sounds: Sounds,
}

fn neutral_glow() -> GlowProfile {
    GlowProfile::NEUTRAL
}

fn neutral_core() -> CoreProfile {
    CoreProfile::NEUTRAL
}

/// The core line: a hot centre (the core image's red) and a fringe (its green) heated
/// by the granulation and the flares.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Core {
    pub(crate) white: Rgb,
    /// How much a flare brightens the hot centre.
    pub(crate) white_flare: f32,
    pub(crate) fringe_cool: Rgb,
    pub(crate) fringe_hot: Rgb,
    /// Where the fringe sits between cool and hot: `base + grain × granulation`.
    pub(crate) fringe_heat: BaseGrain,
    /// The fringe's brightness: `base + grain × granulation + flare × flare`.
    pub(crate) fringe_brightness: BaseGrainFlare,
    /// The core's width breathing across.
    pub(crate) breathe: ShimmerWave,
}

/// The glow capsule: a gradient from the rim to the inside, widened by shimmer,
/// granulation and flares up to `reach` times the stock capsule.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Corona {
    /// The rim's colour, cool and heated.
    pub(crate) rim_cool: Rgb,
    pub(crate) rim_hot: Rgb,
    /// The colour inside, by the core.
    pub(crate) inner: Rgb,
    /// How far out (in capsule radii) the inside colour reaches.
    pub(crate) inner_width: f32,
    /// How the granulation and the flares heat the rim.
    pub(crate) rim_heat: GrainFlare,
    /// How much of the inside colour shows: `(base + grain × granulation) × inside² +
    /// flare × flare`.
    pub(crate) inner_mix: BaseGrainFlare,
    /// The glow's brightness: `base + grain × granulation + flare × flare`.
    pub(crate) brightness: BaseGrainFlare,
    /// Widest the glow may get, in stock capsule widths (1 to 2).
    pub(crate) reach: f32,
    /// How much the granulation and a flare widen the glow.
    pub(crate) swell: GrainFlare,
}

/// Value noise drifting along the blade.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Granulation {
    /// One or two octaves, summed by weight.
    pub(crate) octaves: Vec<Octave>,
    /// The sum's contrast: smoothstep from `low` to `high`.
    pub(crate) low: f32,
    pub(crate) high: f32,
}

/// One octave of granulation: noise at `(along × scale − time × speed, time × evolve +
/// seed × offset)`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Octave {
    pub(crate) scale: f32,
    pub(crate) speed: f32,
    pub(crate) evolve: f32,
    pub(crate) offset: f32,
    pub(crate) weight: f32,
}

/// Bright knots running from the hilt to the tip, one track each.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Flares {
    /// Tracks, 0 to 8.
    pub(crate) count: u32,
    /// Cycles a second of track `n`: `rate + rate_step × n + rate_seed × seed`.
    pub(crate) rate: f32,
    pub(crate) rate_step: f32,
    pub(crate) rate_seed: f32,
    /// Phase offsets per blade seed and per track.
    pub(crate) phase_seed: f32,
    pub(crate) phase_step: f32,
    /// A cycle is lit when its random draw in [0, 1] is at least this.
    pub(crate) threshold: f32,
    /// How far (units) a flare starts before the hilt and ends past the tip.
    pub(crate) overshoot: f32,
    /// Its length along the blade (units): `size + size_jitter × random`.
    pub(crate) size: f32,
    pub(crate) size_jitter: f32,
}

/// `amount × sin(time × rate + along × along + seed × seed)`.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShimmerWave {
    pub(crate) amount: f32,
    pub(crate) rate: f32,
    pub(crate) along: f32,
    pub(crate) seed: f32,
}

/// Flame tongues licking out of the corona's edge.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tongues {
    /// Noise at `(along × along + seed × offset, out × out − time × speed)`.
    pub(crate) along: f32,
    pub(crate) offset: f32,
    pub(crate) out: f32,
    pub(crate) speed: f32,
    /// Where they start and are full, in capsule radii out.
    pub(crate) edge_low: f32,
    pub(crate) edge_high: f32,
    /// Their brightness: `low + range × noise`.
    pub(crate) low: f32,
    pub(crate) range: f32,
}

/// The blade's dynamic light.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Light {
    pub(crate) color: Rgb,
    pub(crate) flicker: Flicker,
}

/// The light's brightness: `1 − amount + amount × Σ weight × sin(t × rate + phase ×
/// the blade's phase)`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Flicker {
    pub(crate) amount: f32,
    #[serde(default)]
    pub(crate) waves: Vec<FlickerWave>,
}

/// One wave of the light's flicker.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FlickerWave {
    pub(crate) rate: f32,
    pub(crate) weight: f32,
    pub(crate) phase: f32,
}

/// The game paths of the skin's sounds, replacing the stock saber sounds.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sounds {
    pub(crate) on: String,
    pub(crate) off: String,
    pub(crate) hum: String,
    pub(crate) swings: [String; 3],
}

/// `base + grain × granulation`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaseGrain {
    pub(crate) base: f32,
    pub(crate) grain: f32,
}

/// `grain × granulation + flare × flare`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct GrainFlare {
    pub(crate) grain: f32,
    pub(crate) flare: f32,
}

/// `base + grain × granulation + flare × flare`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaseGrainFlare {
    pub(crate) base: f32,
    pub(crate) grain: f32,
    pub(crate) flare: f32,
}

/// Whether `id` may name an unlock: 1 to 32 of `a` to `z`, `0` to `9` and `_`.
pub(crate) fn valid_id(id: &str) -> bool {
    crate::emotes::valid_id(id)
}

/// Read one blade-skin file: `id` from its name, `text` its contents.
pub(crate) fn parse(id: &str, text: &str) -> Result<BladeSkinDef, String> {
    if !valid_id(id) {
        return Err(format!(
            "{id:?} is not an unlock id (1 to 32 of a to z, 0 to 9 and _)"
        ));
    }
    let def: BladeSkinDef = serde_json::from_str(text).map_err(|error| error.to_string())?;
    def.check()?;
    Ok(def)
}

/// `value`, named `field`, within `low..=high`.
fn within(field: &str, value: f32, low: f32, high: f32) -> Result<(), String> {
    if value.is_finite() && (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} must be {low} to {high}, not {value}"))
    }
}

fn colour(field: &str, rgb: Rgb, high: f32) -> Result<(), String> {
    for (channel, value) in ["red", "green", "blue"].into_iter().zip(rgb) {
        within(&format!("{field} {channel}"), value, 0.0, high)?;
    }
    Ok(())
}

/// A game path in the pack: relative, `/`-separated, no `..`, at most 63 characters.
fn game_path(field: &str, path: &str) -> Result<(), String> {
    let fine = !path.is_empty()
        && path.len() <= PATH_MAX
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| !part.is_empty() && part != "..");
    if fine {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a game path of 1 to {PATH_MAX} characters, not {path:?}"
        ))
    }
}

/// A coefficient of the shading: any reasonable number.
fn factor(field: &str, value: f32) -> Result<(), String> {
    within(field, value, -1_000.0, 1_000.0)
}

fn wave(field: &str, wave: &ShimmerWave) -> Result<(), String> {
    within(&format!("{field}.amount"), wave.amount, 0.0, 0.5)?;
    factor(&format!("{field}.rate"), wave.rate)?;
    factor(&format!("{field}.along"), wave.along)?;
    factor(&format!("{field}.seed"), wave.seed)
}

impl BladeSkinDef {
    /// Every value within its range.
    fn check(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err(format!("version must be {VERSION}, not {}", self.version));
        }
        let glow = &self.glow_profile;
        within("glow_profile.width", glow.width, 0.05, 1.0)?;
        within("glow_profile.peak", glow.peak, 0.0, 1.0)?;
        within("glow_profile.tail_width", glow.tail_width, 0.05, 2.0)?;
        within("glow_profile.tail_peak", glow.tail_peak, 0.0, 1.0)?;
        let core = &self.core_profile;
        within("core_profile.width", core.width, 0.05, 1.0)?;
        within("core_profile.fringe_width", core.fringe_width, 0.05, 1.5)?;
        for (field, path) in [
            ("glow_image", &self.glow_image),
            ("core_image", &self.core_image),
        ] {
            if let Some(path) = path {
                game_path(field, path)?;
            }
        }
        let c = &self.core;
        colour("core.white", c.white, 4.0)?;
        within("core.white_flare", c.white_flare, 0.0, 10.0)?;
        colour("core.fringe_cool", c.fringe_cool, 4.0)?;
        colour("core.fringe_hot", c.fringe_hot, 4.0)?;
        factor("core.fringe_heat.base", c.fringe_heat.base)?;
        factor("core.fringe_heat.grain", c.fringe_heat.grain)?;
        within(
            "core.fringe_brightness.base",
            c.fringe_brightness.base,
            0.0,
            10.0,
        )?;
        factor("core.fringe_brightness.grain", c.fringe_brightness.grain)?;
        factor("core.fringe_brightness.flare", c.fringe_brightness.flare)?;
        wave("core.breathe", &c.breathe)?;
        let r = &self.corona;
        colour("corona.rim_cool", r.rim_cool, 4.0)?;
        colour("corona.rim_hot", r.rim_hot, 4.0)?;
        colour("corona.inner", r.inner, 4.0)?;
        within("corona.inner_width", r.inner_width, 0.01, 1.0)?;
        factor("corona.rim_heat.grain", r.rim_heat.grain)?;
        factor("corona.rim_heat.flare", r.rim_heat.flare)?;
        factor("corona.inner_mix.base", r.inner_mix.base)?;
        factor("corona.inner_mix.grain", r.inner_mix.grain)?;
        factor("corona.inner_mix.flare", r.inner_mix.flare)?;
        within("corona.brightness.base", r.brightness.base, 0.0, 10.0)?;
        factor("corona.brightness.grain", r.brightness.grain)?;
        factor("corona.brightness.flare", r.brightness.flare)?;
        within(
            "corona.reach",
            r.reach,
            1.0,
            crate::saber_skins::MAX_GLOW_REACH,
        )?;
        within("corona.swell.grain", r.swell.grain, -2.0, 2.0)?;
        within("corona.swell.flare", r.swell.flare, -2.0, 2.0)?;
        let g = &self.granulation;
        if !(1..=MAX_OCTAVES).contains(&g.octaves.len()) {
            return Err(format!(
                "granulation.octaves must hold 1 to {MAX_OCTAVES} octaves, not {}",
                g.octaves.len()
            ));
        }
        for (index, octave) in g.octaves.iter().enumerate() {
            let field = |name: &str| format!("granulation.octaves[{index}].{name}");
            within(&field("scale"), octave.scale, 0.0, 100.0)?;
            factor(&field("speed"), octave.speed)?;
            factor(&field("evolve"), octave.evolve)?;
            factor(&field("offset"), octave.offset)?;
            within(&field("weight"), octave.weight, 0.0, 1.0)?;
        }
        within("granulation.low", g.low, 0.0, 1.0)?;
        within("granulation.high", g.high, 0.0, 1.0)?;
        if g.low >= g.high {
            return Err("granulation.low must be below granulation.high".to_owned());
        }
        let f = &self.flares;
        if f.count > MAX_FLARES {
            return Err(format!(
                "flares.count must be 0 to {MAX_FLARES}, not {}",
                f.count
            ));
        }
        within("flares.rate", f.rate, 0.0, 100.0)?;
        factor("flares.rate_step", f.rate_step)?;
        factor("flares.rate_seed", f.rate_seed)?;
        factor("flares.phase_seed", f.phase_seed)?;
        factor("flares.phase_step", f.phase_step)?;
        within("flares.threshold", f.threshold, 0.0, 1.0)?;
        within("flares.overshoot", f.overshoot, 0.0, 100.0)?;
        within("flares.size", f.size, 0.1, 100.0)?;
        within("flares.size_jitter", f.size_jitter, 0.0, 100.0)?;
        if self.shimmer.len() > MAX_WAVES {
            return Err(format!(
                "shimmer must hold at most {MAX_WAVES} waves, not {}",
                self.shimmer.len()
            ));
        }
        for (index, shimmer) in self.shimmer.iter().enumerate() {
            wave(&format!("shimmer[{index}]"), shimmer)?;
        }
        let t = &self.tongues;
        factor("tongues.along", t.along)?;
        factor("tongues.offset", t.offset)?;
        factor("tongues.out", t.out)?;
        factor("tongues.speed", t.speed)?;
        within("tongues.edge_low", t.edge_low, 0.0, 2.0)?;
        within("tongues.edge_high", t.edge_high, 0.0, 2.0)?;
        if t.edge_low >= t.edge_high {
            return Err("tongues.edge_low must be below tongues.edge_high".to_owned());
        }
        within("tongues.low", t.low, 0.0, 10.0)?;
        factor("tongues.range", t.range)?;
        colour("trail", self.trail, 1.0)?;
        colour("light.color", self.light.color, 4.0)?;
        within("light.flicker.amount", self.light.flicker.amount, 0.0, 1.0)?;
        if self.light.flicker.waves.len() > MAX_WAVES {
            return Err(format!(
                "light.flicker.waves must hold at most {MAX_WAVES} waves, not {}",
                self.light.flicker.waves.len()
            ));
        }
        for (index, wave) in self.light.flicker.waves.iter().enumerate() {
            let field = |name: &str| format!("light.flicker.waves[{index}].{name}");
            within(&field("rate"), wave.rate, 0.0, 100.0)?;
            within(&field("weight"), wave.weight, 0.0, 1.0)?;
            factor(&field("phase"), wave.phase)?;
        }
        let s = &self.sounds;
        game_path("sounds.on", &s.on)?;
        game_path("sounds.off", &s.off)?;
        game_path("sounds.hum", &s.hum)?;
        for (index, swing) in s.swings.iter().enumerate() {
            game_path(&format!("sounds.swings[{index}]"), swing)?;
        }
        Ok(())
    }
}

/// Every `skins/blades/<id>.bladeskin` in `vfs` that reads, by id; the ones that do not
/// are reported to the log by name.
pub(crate) fn load(vfs: &sjk_vfs::VirtualFileSystem) -> Vec<(String, BladeSkinDef)> {
    let mut skins = Vec::new();
    for listed in vfs.list_files(FOLDER, EXTENSION) {
        let path = format!("{FOLDER}/{listed}");
        let id = listed.trim_end_matches(EXTENSION);
        let read = vfs
            .read(&path)
            .map_err(|error| error.to_string())
            .and_then(|asset| asset.ok_or_else(|| "missing".to_owned()))
            .and_then(|asset| String::from_utf8(asset.bytes).map_err(|_| "not UTF-8".to_owned()))
            .and_then(|text| parse(id, &text));
        match read {
            Ok(def) => skins.push((id.to_owned(), def)),
            Err(why) => crate::log::progress(format_args!("blade skins: {path}: {why}")),
        }
    }
    skins.sort_by(|a, b| a.0.cmp(&b.0));
    skins.dedup_by(|a, b| a.0 == b.0);
    skins
}

#[cfg(test)]
#[path = "blade_skin_file_tests.rs"]
pub(crate) mod tests;
