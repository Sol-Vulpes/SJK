//! How a number setting reads and moves on its slider.
//!
//! The cvars keep their numbers; this is only what the rows show and how the track
//! is laid out, one table entry per setting, read by both views' generic code:
//!
//! - a [`Unit`] after the number, so a row says "85%", "100°" or "19:30" in place of
//!   a bare 0.85, 100.0 or 19.5 and its label needs no "(%)" or "(hour)";
//! - a [`Curve`] for the settings whose useful values crowd one end of their range;
//! - a name for a value that means something else than its number ([`NAMED`]): a
//!   volume of 0 is "Off", a weapon FOV of 0 "World".
//!
//! The loudness unit is the one that changes what the number is. Players set the
//! volumes to 0.05, 0.1, 0.01 and 0.001 (the hub's settings usage, 21/11/2026), where
//! a straight 0 to 1 track in steps of 0.05 has two positions. Hearing is not linear
//! in the gain, so the row shows the square root of the gain as a percentage: 100% is
//! the cvar's 1, 50% its 0.25, 22% its 0.05, 10% its 0.01, and a step of 5% is
//! as audible at the bottom of the track as at its top.

use super::catalog::{Setting, ValueKind};
use std::borrow::Cow;

/// What follows a number setting's value on its row, and what the number shown is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Unit {
    /// The cvar's number as it is.
    Plain,
    /// The cvar's number, then this ("°", " ms", "%" for a cvar already in percent).
    Suffix(&'static str),
    /// A share or a scale kept as a fraction: 0.85 reads 85%.
    Percent,
    /// A gain: its square root as a percentage (see the module's words).
    Loudness,
    /// An hour of the day as a decimal number: 19.5 reads 19:30.
    Clock,
}

/// How a setting's range lies along its track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Curve {
    Linear,
    /// The value grows as the square of the position: the lower half of the range's
    /// useful values gets most of the track.
    Square,
}

const UNITS: &[(&str, Unit)] = &[
    // Sound: gains.
    ("s_volume", Unit::Loudness),
    ("s_musicVolume", Unit::Loudness),
    (crate::audio::VOICE_VOLUME_CVAR, Unit::Loudness),
    (crate::emotes::VOLUME_CVAR, Unit::Loudness),
    (crate::toy_sounds::VOLUME_CVAR, Unit::Loudness),
    (crate::audio::spatial::REVERB_CVAR, Unit::Percent),
    // Angles.
    ("cg_fov", Unit::Suffix("\u{b0}")),
    ("cg_fovViewmodel", Unit::Suffix("\u{b0}")),
    ("cg_thirdPersonAngle", Unit::Suffix("\u{b0}")),
    ("cg_thirdPersonPitchOffset", Unit::Suffix("\u{b0}")),
    (crate::menu::sjk::theme::HUE_CVAR, Unit::Suffix("\u{b0}")),
    // Times.
    ("cg_playerCardDelay", Unit::Suffix(" s")),
    (crate::input::idrive::DELAY_CVAR, Unit::Suffix(" ms")),
    ("r_dayHour", Unit::Clock),
    // Cvars already in percent.
    (crate::chat::options::BACKDROP_CVAR, Unit::Suffix("%")),
    (crate::menu::sjk::theme::SATURATION_CVAR, Unit::Suffix("%")),
    (
        crate::frame_target::aa::color::SHARPEN_CVAR,
        Unit::Suffix("%"),
    ),
    (crate::frame_target::scale::CVAR, Unit::Suffix("%")),
    // Sizes and strengths kept as a scale, 1 being the stock size.
    ("cg_hudScale", Unit::Percent),
    ("cg_drawPlayerNamesScale", Unit::Percent),
    ("cg_nameplateScale", Unit::Percent),
    (crate::hud::nameplate::OPACITY_CVAR, Unit::Percent),
    ("cg_teamScoresScale", Unit::Percent),
    ("cg_lagometerScale", Unit::Percent),
    ("cg_chatBoxFontSize", Unit::Percent),
    (crate::chat::options::CARD_SCALE_CVAR, Unit::Percent),
    (crate::saber_clash_flare::SIZE_CVAR, Unit::Percent),
    (crate::holstered_sabers::SWAP_CVAR, Unit::Percent),
    (crate::text::style::SCALE_CVAR, Unit::Percent),
    ("con_scale", Unit::Percent),
    ("r_hdrExposure", Unit::Percent),
    ("r_parallaxStrength", Unit::Percent),
    ("r_emissionStrength", Unit::Percent),
    ("r_dayBrightness", Unit::Percent),
    ("r_ambientFillOcclusion", Unit::Percent),
    ("r_indirectBoost", Unit::Percent),
    ("r_emissiveLights", Unit::Percent),
    (
        crate::world_materials::emission::lamp_gain::CVAR,
        Unit::Percent,
    ),
    ("r_volumetricClarity", Unit::Percent),
    // Exposure steps.
    (
        crate::frame_target::aa::exposure::MIN_EV,
        Unit::Suffix(" EV"),
    ),
    (
        crate::frame_target::aa::exposure::MAX_EV,
        Unit::Suffix(" EV"),
    ),
];

/// Settings whose track is not straight, and why.
const CURVES: &[(&str, Curve)] = &[
    // 0.1 to 20, and nearly every player between 1 and 6.
    ("sensitivity", Curve::Square),
    // 25% to 400% around a stock 100%, 10% to 1000% around 100%.
    ("r_hdrExposure", Curve::Square),
    ("r_dayBrightness", Curve::Square),
];

/// A value that means something else than its number: `(cvar, value, name)`. The
/// name takes the number's place on the row, so it is a short word.
const NAMED: &[(&str, f64, &str)] = &[
    ("s_volume", 0.0, "Off"),
    ("s_musicVolume", 0.0, "Off"),
    (crate::audio::VOICE_VOLUME_CVAR, 0.0, "Off"),
    (crate::emotes::VOLUME_CVAR, 0.0, "Off"),
    (crate::toy_sounds::VOLUME_CVAR, 0.0, "Off"),
    (crate::audio::spatial::REVERB_CVAR, 0.0, "Off"),
    ("cg_fovViewmodel", 0.0, "World"),
    ("r_parallaxStrength", 0.0, "Flat"),
    ("r_emissionStrength", 0.0, "Off"),
    (crate::frame_target::aa::color::SHARPEN_CVAR, 0.0, "Off"),
    (crate::frame_target::scale::CVAR, 100.0, "Native"),
    (crate::chat::options::BACKDROP_CVAR, 0.0, "None"),
    (crate::input::idrive::DELAY_CVAR, 0.0, "None"),
    (crate::saber_clash_flare::SIZE_CVAR, 0.0, "Off"),
];

fn entry<T: Copy>(table: &[(&str, T)], cvar: &str) -> Option<T> {
    table
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))
        .map(|(_, value)| *value)
}

/// The unit of setting `cvar`'s number.
pub(super) fn unit(cvar: &str) -> Unit {
    entry(UNITS, cvar).unwrap_or(Unit::Plain)
}

/// How setting `cvar`'s range lies along its track.
pub(super) fn curve(cvar: &str) -> Curve {
    match unit(cvar) {
        Unit::Loudness => Curve::Square,
        _ => entry(CURVES, cvar).unwrap_or(Curve::Linear),
    }
}

/// A number to `places` decimals at most, without trailing zeros.
fn trimmed(value: f64, places: usize) -> String {
    let text = format!("{value:.places$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    // Rounding to nothing leaves no sign behind.
    if text == "-0" { "0" } else { text }.to_owned()
}

impl Unit {
    /// The number the row shows for the cvar's `value`.
    pub(super) fn display(self, value: f64) -> f64 {
        match self {
            Self::Percent => value * 100.0,
            Self::Loudness => value.max(0.0).sqrt() * 100.0,
            Self::Plain | Self::Suffix(_) | Self::Clock => value,
        }
    }

    /// The cvar's value for the number `shown` on the row, as a typed one is.
    pub(super) fn value(self, shown: f64) -> f64 {
        match self {
            Self::Percent => shown / 100.0,
            Self::Loudness => (shown.max(0.0) / 100.0).powi(2),
            Self::Plain | Self::Suffix(_) | Self::Clock => shown,
        }
    }

    /// Whether the number typed on the row is the cvar's own.
    pub(super) fn typed_as_stored(self) -> bool {
        matches!(self, Self::Plain | Self::Suffix(_) | Self::Clock)
    }

    /// The cvar's `value` as the row reads it, its unit after it.
    pub(super) fn write(self, value: f64) -> String {
        match self {
            Self::Plain => trimmed(value, 4),
            Self::Suffix(suffix) => format!("{}{suffix}", trimmed(value, 3)),
            Self::Percent => format!("{}%", trimmed(self.display(value), 1)),
            Self::Loudness => format!("{}%", trimmed(self.display(value), 0)),
            Self::Clock => {
                let minutes = (value.rem_euclid(24.0) * 60.0).round() as i64;
                // The day's last stop reads 24:00, not a second midnight.
                let (hours, minutes) = if value >= 24.0 {
                    (24, 0)
                } else {
                    (minutes / 60, minutes % 60)
                };
                format!("{hours:02}:{minutes:02}")
            }
        }
    }
}

/// What the row of number setting `cvar` shows for the value written `raw`: its
/// name if the value has one, else the number in its unit. A plain number is shown
/// as it is written (the rows keep `90.0` and `1.25` as they always read), and so is
/// text that is not a number.
pub(super) fn number<'a>(cvar: &str, raw: &'a str) -> Cow<'a, str> {
    let Ok(value) = raw.trim().parse::<f64>() else {
        return Cow::Borrowed(raw);
    };
    if let Some((_, _, name)) = NAMED
        .iter()
        .find(|(name, named, _)| name.eq_ignore_ascii_case(cvar) && (named - value).abs() < 1e-9)
    {
        return Cow::Borrowed(name);
    }
    match unit(cvar) {
        Unit::Plain => Cow::Borrowed(raw),
        unit => Cow::Owned(unit.write(value)),
    }
}

/// What row `setting` shows for the value written `raw`, whatever its control: a
/// stop's name, a slider's number in its unit, anything else as it is written (a
/// choice's name is [`super::classic_view::choice_label`]'s).
pub(super) fn value<'a>(setting: &Setting, raw: &'a str) -> Cow<'a, str> {
    match setting.kind() {
        ValueKind::Stops { stops, suffix, .. } => super::stops::shown(stops, suffix, raw),
        ValueKind::Integer { .. } | ValueKind::Float { .. } => number(setting.cvar, raw),
        _ => Cow::Borrowed(raw),
    }
}

/// The range of number row `setting` as the detail says it, in the row's unit
/// ("70 to 130" with its degree sign, "0% to 100%"); for a row of named stops, what
/// a typed value may be ("Any value, 0 to 2000 FPS"). `None` for a row that is not
/// a number.
pub(super) fn range(setting: &Setting) -> Option<String> {
    let unit = unit(setting.cvar);
    match setting.kind() {
        ValueKind::Integer { min, max, .. } => Some(format!(
            "{} to {}",
            unit.write(min as f64),
            unit.write(max as f64)
        )),
        ValueKind::Float { min, max, .. } => {
            Some(format!("{} to {}", unit.write(min), unit.write(max)))
        }
        // A special value below zero (the FPS cap's Auto) is a stop, not the range.
        ValueKind::Stops {
            suffix, min, max, ..
        } => Some(format!(
            "Any value, {} to {}{suffix}",
            trimmed(min.max(0.0), 3),
            trimmed(max, 3)
        )),
        _ => None,
    }
}

/// Where `value` sits along the track of setting `cvar`, whose range is `min` to
/// `max`: 0 at its left end, 1 at its right.
pub(super) fn ratio(cvar: &str, min: f64, max: f64, value: f64) -> f32 {
    if max <= min {
        return 0.0;
    }
    let linear = ((value - min) / (max - min)).clamp(0.0, 1.0);
    (match curve(cvar) {
        Curve::Linear => linear,
        Curve::Square => linear.sqrt(),
    }) as f32
}

/// The value at `ratio` along the track of setting `cvar` (see [`ratio`]), before
/// it is snapped to the setting's steps.
pub(super) fn at(cvar: &str, min: f64, max: f64, ratio: f64) -> f64 {
    let ratio = ratio.clamp(0.0, 1.0);
    let linear = match curve(cvar) {
        Curve::Linear => ratio,
        Curve::Square => ratio * ratio,
    };
    min + (max - min) * linear
}

/// The track position of slider row `setting` whose value is written `raw`; the
/// left end for a row that is no slider or text that is no number.
pub(super) fn track(setting: &Setting, raw: &str) -> f32 {
    let Ok(value) = raw.trim().parse::<f64>() else {
        return 0.0;
    };
    match setting.kind() {
        ValueKind::Integer { min, max, .. } => ratio(setting.cvar, min as f64, max as f64, value),
        ValueKind::Float { min, max, .. } => ratio(setting.cvar, min, max, value),
        _ => 0.0,
    }
}

/// Loudness rows move in shown percent, not in gain: the cvar text one step of 5%
/// `direction` from the gain `current`, within `min` to `max`.
pub(super) fn loudness_stepped(current: f64, direction: i32, min: f64, max: f64) -> String {
    let unit = Unit::Loudness;
    let shown = unit.display(current);
    // From between two steps, the first press lands on the step that way.
    let step = 5.0;
    let next = if direction > 0 {
        ((shown + 1e-6) / step).floor() * step + step
    } else {
        ((shown - 1e-6) / step).ceil() * step - step
    };
    gain_text(unit.value(next).clamp(min, max))
}

/// The gain at `ratio` along a loudness row's track, on a whole percent.
pub(super) fn loudness_snapped(cvar: &str, min: f64, max: f64, ratio: f64) -> String {
    let unit = Unit::Loudness;
    let shown = unit.display(at(cvar, min, max, ratio)).round();
    gain_text(unit.value(shown).clamp(min, max))
}

/// A gain as its cvar takes it: four decimals hold every whole percent shown.
fn gain_text(gain: f64) -> String {
    let text = trimmed(gain, 4);
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_in_their_units() {
        assert_eq!(number("cg_fov", "90.0"), "90\u{b0}");
        assert_eq!(number("cg_hudScale", "0.85"), "85%");
        assert_eq!(number("r_volumetricClarity", "0.025"), "2.5%");
        assert_eq!(number("r_dayHour", "19.5"), "19:30");
        assert_eq!(number("r_dayHour", "0.0"), "00:00");
        assert_eq!(number("r_dayHour", "24.0"), "24:00");
        assert_eq!(number("cg_playerCardDelay", "1.5"), "1.5 s");
        // A plain number stays as it is written.
        assert_eq!(number("r_gamma", "1.0"), "1.0");
        assert_eq!(number("cg_fov", "?"), "?");
    }

    #[test]
    fn values_with_a_meaning_read_as_their_name() {
        assert_eq!(number("s_musicVolume", "0.0"), "Off");
        assert_eq!(number("cg_fovViewmodel", "0.0"), "World");
        assert_eq!(number("cg_fovViewmodel", "80.0"), "80\u{b0}");
    }

    #[test]
    fn loudness_is_the_square_root_of_the_gain() {
        assert_eq!(number("s_volume", "1.0"), "100%");
        assert_eq!(number("s_volume", "0.25"), "50%");
        assert_eq!(number("s_volume", "0.05"), "22%");
        assert_eq!(number("s_volume", "0.01"), "10%");
        // The track follows what is shown: a quarter gain sits in the middle.
        assert!((ratio("s_volume", 0.0, 1.0, 0.25) - 0.5).abs() < 1e-6);
        assert!((at("s_volume", 0.0, 1.0, 0.5) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn loudness_steps_by_five_percent_and_joins_the_grid_from_between() {
        assert_eq!(loudness_stepped(0.25, 1, 0.0, 1.0), "0.3025");
        assert_eq!(number("s_volume", "0.3025"), "55%");
        assert_eq!(loudness_stepped(0.25, -1, 0.0, 1.0), "0.2025");
        // 0.05 shows 22%: up is 25%, down 20%.
        assert_eq!(loudness_stepped(0.05, 1, 0.0, 1.0), "0.0625");
        assert_eq!(loudness_stepped(0.05, -1, 0.0, 1.0), "0.04");
        assert_eq!(loudness_stepped(0.0, -1, 0.0, 1.0), "0.0");
        assert_eq!(loudness_stepped(1.0, 1, 0.0, 1.0), "1.0");
        // Every step shown reads back as itself.
        let mut gain = 0.0;
        for step in 1..=20 {
            let next = loudness_stepped(gain, 1, 0.0, 1.0);
            gain = next.parse().unwrap();
            assert_eq!(number("s_volume", &next), format!("{}%", step * 5));
        }
    }

    #[test]
    fn a_dragged_loudness_lands_on_a_whole_percent() {
        let text = loudness_snapped("s_volume", 0.0, 1.0, 0.223);
        assert_eq!(text, "0.0484");
        assert_eq!(number("s_volume", &text), "22%");
        assert_eq!(loudness_snapped("s_volume", 0.0, 1.0, 0.0), "0.0");
        assert_eq!(loudness_snapped("s_volume", 0.0, 1.0, 1.0), "1.0");
    }

    #[test]
    fn a_curved_track_gives_the_low_values_room_and_reads_back() {
        // Sensitivity 5 of 0.1 to 20 sits at half the track, not a quarter.
        let middle = ratio("sensitivity", 0.1, 20.0, 5.0);
        assert!((0.45..0.55).contains(&middle), "{middle}");
        for value in [0.1, 1.35, 5.0, 20.0] {
            let back = at(
                "sensitivity",
                0.1,
                20.0,
                f64::from(ratio("sensitivity", 0.1, 20.0, value)),
            );
            assert!((back - value).abs() < 1e-4, "{value} read back {back}");
        }
        // A straight track is unchanged.
        assert!((ratio("cg_fov", 70.0, 130.0, 100.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn typed_numbers_convert_both_ways() {
        for (unit, stored) in [
            (Unit::Percent, 0.85),
            (Unit::Loudness, 0.25),
            (Unit::Suffix(" ms"), 250.0),
            (Unit::Clock, 19.5),
        ] {
            assert!((unit.value(unit.display(stored)) - stored).abs() < 1e-9);
        }
        assert!(!Unit::Percent.typed_as_stored() && Unit::Clock.typed_as_stored());
    }
}
