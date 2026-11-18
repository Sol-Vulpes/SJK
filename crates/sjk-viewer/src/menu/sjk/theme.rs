//! The SJK UI's colour theme (`docs/sjk-ui.md`, Theme): `ui_theme` picks the
//! palette its tokens ([`super::color`]) give, `warm` (the sun's colours, the
//! default), `cold` (the deep navy and holo blue the sun theme replaced) or
//! `custom`, the warm palette turned to the hue `ui_themeHue` at
//! `ui_themeSaturation`. Moving either of those picks `custom`
//! ([`follows_custom`], in the console's cvar writes).
//!
//! A custom palette keeps each token's lightness: every colour is turned round
//! the OKLab hue circle, its chroma scaled by the saturation, then set to the
//! warm token's relative luminance (and its chroma cut back to what sRGB can
//! show at that luminance). So the contrast between any two tokens, which WCAG
//! reckons from luminance alone, is the warm palette's whatever the hue.
//! Ember (leaving, errors) keeps the warm palette's red: it carries a meaning.
//! The team colours and the trusted green are not themed at all.
//!
//! The palette in use lives in atomics ([`Live`]): a token is three relaxed
//! loads, nothing locks or allocates, and [`sync`] (each frame, from the
//! runtime cvars) works a palette out only when one of the three cvars changed.

use crate::console::ViewerConsole;
use sjk_ui::Color;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// The theme: `warm`, `cold` or `custom`.
pub(crate) const CVAR: &str = "ui_theme";
/// The custom theme's hue, in degrees round the colour wheel (0 red, 120 green,
/// 240 blue): the hue its line-work takes.
pub(crate) const HUE_CVAR: &str = "ui_themeHue";
/// The custom theme's saturation, in percent of the warm palette's.
pub(crate) const SATURATION_CVAR: &str = "ui_themeSaturation";

/// The hue slider's ends and step.
pub(crate) const HUE_MAX: i64 = 360;
pub(crate) const HUE_STEP: i64 = 5;
/// The saturation slider's top and step.
pub(crate) const SATURATION_MAX: i64 = 100;
pub(crate) const SATURATION_STEP: i64 = 5;
/// `ui_themeHue`'s default: the warm line-work's own hue, so a custom theme
/// starts as the warm one.
pub(crate) const DEFAULT_HUE: i64 = 22;
/// `ui_themeSaturation`'s default: the warm palette's.
pub(crate) const DEFAULT_SATURATION: i64 = 100;

/// Which palette the SJK UI wears.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Theme {
    /// The sun's warm colours (17/11/2026): SJK's default.
    #[default]
    Warm,
    /// Deep navy and holo blue, the SJK UI's colours before the sun theme.
    Cold,
    /// The warm palette turned to `ui_themeHue` at `ui_themeSaturation`.
    Custom,
}

impl Theme {
    /// Values the settings offer, in [`Theme`] order.
    pub(crate) const NAMES: [&'static str; 3] = ["warm", "cold", "custom"];
    /// The `ui_theme` value of the default theme.
    pub(crate) const DEFAULT_NAME: &'static str = Self::NAMES[0];

    /// Read the cvar: `cold` or `custom` (any case, spaces round it allowed);
    /// anything else, a missing or mistyped value included, is the default
    /// warm theme.
    pub(crate) fn from_cvar(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(text) if text.eq_ignore_ascii_case("cold") => Self::Cold,
            Some(text) if text.eq_ignore_ascii_case("custom") => Self::Custom,
            _ => Self::Warm,
        }
    }
}

/// A palette: one colour for each themed token of [`super::color`], all opaque.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Palette {
    pub(crate) space: Color,
    pub(crate) glass: Color,
    pub(crate) glass_lit: Color,
    pub(crate) holo: Color,
    pub(crate) gold: Color,
    pub(crate) gold_bright: Color,
    pub(crate) text: Color,
    pub(crate) muted: Color,
    pub(crate) quiet: Color,
    pub(crate) ember: Color,
}

/// The themed tokens, in [`Palette::tokens`] order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Token {
    Space,
    Glass,
    GlassLit,
    Holo,
    Gold,
    GoldBright,
    Text,
    Muted,
    Quiet,
    Ember,
}

/// How many tokens a palette has.
const TOKENS: usize = 10;

impl Palette {
    /// The tokens in [`Token`] order.
    pub(crate) const fn tokens(&self) -> [Color; TOKENS] {
        [
            self.space,
            self.glass,
            self.glass_lit,
            self.holo,
            self.gold,
            self.gold_bright,
            self.text,
            self.muted,
            self.quiet,
            self.ember,
        ]
    }

    fn from_tokens(t: [Color; TOKENS]) -> Self {
        Self {
            space: t[0],
            glass: t[1],
            glass_lit: t[2],
            holo: t[3],
            gold: t[4],
            gold_bright: t[5],
            text: t[6],
            muted: t[7],
            quiet: t[8],
            ember: t[9],
        }
    }
}

const fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::new(r, g, b, 1.0)
}

/// The sun theme (17/11/2026, `3c74e1adf`): an ember-brown ground, dark glass,
/// amber line-work, sun gold, warm white text, the sun's red rim for leaving.
pub(crate) const WARM: Palette = Palette {
    space: rgb(0.078, 0.043, 0.024),
    glass: rgb(0.122, 0.071, 0.039),
    glass_lit: rgb(0.18, 0.106, 0.059),
    holo: rgb(1.0, 0.663, 0.471),
    gold: rgb(1.0, 0.784, 0.239),
    gold_bright: rgb(1.0, 0.89, 0.561),
    text: rgb(0.988, 0.937, 0.886),
    muted: rgb(0.839, 0.749, 0.667),
    quiet: rgb(0.678, 0.584, 0.514),
    ember: rgb(1.0, 0.361, 0.239),
};

/// The SJK UI's colours before the sun theme (`3c74e1adf^`): deep space navy,
/// holo-blue line-work, gold for the choice, cool white text. Its glass, which
/// that palette left to literals, is the navy those literals had: the panels'
/// `(0.04, 0.06, 0.12)` and the lit tiles' `(0.07, 0.1, 0.17)`. Its muted text is
/// lifted 4 % from `(0.663, 0.71, 0.796)`, which the status bars' acrylic (younger
/// than this palette) left under 4.5:1 before snow.
pub(crate) const COLD: Palette = Palette {
    space: rgb(0.024, 0.039, 0.078),
    glass: rgb(0.04, 0.06, 0.12),
    glass_lit: rgb(0.07, 0.1, 0.17),
    holo: rgb(0.659, 0.812, 1.0),
    gold: rgb(0.91, 0.722, 0.29),
    gold_bright: rgb(1.0, 0.851, 0.478),
    text: rgb(0.886, 0.914, 0.965),
    muted: rgb(0.69, 0.738, 0.828),
    quiet: rgb(0.53, 0.573, 0.675),
    ember: rgb(1.0, 0.478, 0.239),
};

/// The palette of `theme`, a custom one at `hue` degrees and `saturation`
/// percent.
pub(crate) fn palette(theme: Theme, hue: f32, saturation: f32) -> Palette {
    match theme {
        Theme::Warm => WARM,
        Theme::Cold => COLD,
        Theme::Custom => custom(hue, saturation),
    }
}

/// The warm palette turned so its line-work takes `hue` (degrees round the
/// colour wheel), its chroma at `saturation` percent, each token at the warm
/// one's luminance (the module's notes). Ember stays the warm one's.
pub(crate) fn custom(hue: f32, saturation: f32) -> Palette {
    let turn = turn_to(hue);
    let chroma = (saturation / 100.0).clamp(0.0, 1.0);
    let mut tokens = WARM.tokens();
    for (index, token) in tokens.iter_mut().enumerate() {
        if index != Token::Ember as usize {
            *token = turned(*token, turn, chroma);
        }
    }
    Palette::from_tokens(tokens)
}

/// What a custom theme at `hue` and `saturation` shows its line-work in: the
/// Settings hue row's swatch.
pub(crate) fn swatch(hue: f32, saturation: f32) -> Color {
    turned(
        WARM.holo,
        turn_to(hue),
        (saturation / 100.0).clamp(0.0, 1.0),
    )
}

/// The turn (radians round OKLab's hue circle) that brings the warm line-work's
/// colour-wheel hue to `hue` degrees: the two wheels differ, so it is measured
/// between full colours of the two hues.
fn turn_to(hue: f32) -> f32 {
    let from = oklab_hue(full_colour(wheel_hue(WARM.holo)));
    oklab_hue(full_colour(hue.rem_euclid(360.0))) - from
}

/// `colour` turned `turn` radians round OKLab's hue circle, its chroma times
/// `chroma`, at its own relative luminance; where sRGB cannot show that chroma
/// at that luminance, the most it can.
fn turned(colour: Color, turn: f32, chroma: f32) -> Color {
    let linear = linear(colour);
    let luminance = luminance_of(linear);
    let [_, a, b] = oklab(linear);
    let hue = b.atan2(a) + turn;
    let wanted = a.hypot(b) * chroma;
    let fit = |c: f32| {
        let rgb = at_luminance(luminance, c, hue);
        rgb.iter()
            .all(|channel| (-1.0e-4..=1.0 + 1.0e-4).contains(channel))
            .then_some(rgb)
    };
    let rgb = fit(wanted).unwrap_or_else(|| {
        // Grey always fits: halve the gap to the most chroma that does.
        let (mut low, mut high) = (0.0, wanted);
        let mut best = at_luminance(luminance, 0.0, hue);
        for _ in 0..24 {
            let middle = 0.5 * (low + high);
            match fit(middle) {
                Some(rgb) => {
                    best = rgb;
                    low = middle;
                }
                None => high = middle,
            }
        }
        best
    });
    let encode = |c: f32| crate::ui_target::linear_to_srgb(c.clamp(0.0, 1.0));
    Color::new(encode(rgb[0]), encode(rgb[1]), encode(rgb[2]), colour.a)
}

/// The linear sRGB colour of OKLab hue `hue` and chroma `chroma` whose relative
/// luminance is `luminance`, found by halving OKLab's lightness range.
fn at_luminance(luminance: f32, chroma: f32, hue: f32) -> [f32; 3] {
    let (a, b) = (chroma * hue.cos(), chroma * hue.sin());
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..32 {
        let middle = 0.5 * (low + high);
        if luminance_of(from_oklab([middle, a, b])) < luminance {
            low = middle;
        } else {
            high = middle;
        }
    }
    from_oklab([0.5 * (low + high), a, b])
}

/// A display colour's channels in linear light.
fn linear(colour: Color) -> [f32; 3] {
    let decode = |c: f32| crate::ui_target::srgb_to_linear(c.clamp(0.0, 1.0));
    [decode(colour.r), decode(colour.g), decode(colour.b)]
}

/// WCAG's relative luminance of linear sRGB channels.
fn luminance_of([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Björn Ottosson's OKLab of linear sRGB.
fn oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// Linear sRGB of an OKLab colour (unclamped).
fn from_oklab([lightness, a, b]: [f32; 3]) -> [f32; 3] {
    let l = (lightness + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (lightness - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (lightness - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

/// OKLab hue (radians) of linear sRGB channels.
fn oklab_hue(linear: [f32; 3]) -> f32 {
    let [_, a, b] = oklab(linear);
    b.atan2(a)
}

/// The colour wheel's (HSV's) hue of a display colour, in degrees.
pub(crate) fn wheel_hue(c: Color) -> f32 {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let d = (max - min).max(1e-6);
    let h = if max == c.r {
        (c.g - c.b) / d
    } else if max == c.g {
        2.0 + (c.b - c.r) / d
    } else {
        4.0 + (c.r - c.g) / d
    };
    (h * 60.0).rem_euclid(360.0)
}

/// The full colour (HSV at full saturation and value) of wheel hue `hue`
/// degrees, in linear light (its channels are 0 or 1 at the corners, so the
/// same either way at them).
fn full_colour(hue: f32) -> [f32; 3] {
    let channel = |n: f32| {
        let k = (n + hue / 60.0).rem_euclid(6.0);
        1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
    };
    let display = [channel(5.0), channel(3.0), channel(1.0)];
    display.map(crate::ui_target::srgb_to_linear)
}

/// The palette in use, as the bits of each token's red, green and blue: lock-free,
/// read by every token ([`Live::get`]), written by [`sync`].
pub(crate) struct Live {
    channels: [[AtomicU32; 3]; TOKENS],
}

impl Live {
    /// A store holding `palette`.
    pub(crate) const fn new(palette: &Palette) -> Self {
        let t = palette.tokens();
        macro_rules! token {
            ($i:expr) => {
                [
                    AtomicU32::new(t[$i].r.to_bits()),
                    AtomicU32::new(t[$i].g.to_bits()),
                    AtomicU32::new(t[$i].b.to_bits()),
                ]
            };
        }
        Self {
            channels: [
                token!(0),
                token!(1),
                token!(2),
                token!(3),
                token!(4),
                token!(5),
                token!(6),
                token!(7),
                token!(8),
                token!(9),
            ],
        }
    }

    /// Token `token` of the palette held.
    #[inline]
    pub(crate) fn get(&self, token: Token) -> Color {
        let [r, g, b] = &self.channels[token as usize];
        let load = |channel: &AtomicU32| f32::from_bits(channel.load(Ordering::Relaxed));
        Color::new(load(r), load(g), load(b), 1.0)
    }

    /// Hold `palette` from now on.
    pub(crate) fn set(&self, palette: &Palette) {
        for (channels, colour) in self.channels.iter().zip(palette.tokens()) {
            for (channel, value) in channels.iter().zip([colour.r, colour.g, colour.b]) {
                channel.store(value.to_bits(), Ordering::Relaxed);
            }
        }
    }
}

/// The palette the SJK UI draws with.
pub(crate) static LIVE: Live = Live::new(&WARM);

/// The cvars [`LIVE`] was last worked out from ([`key`]); 0 before the first.
static APPLIED: AtomicU64 = AtomicU64::new(0);

/// The three cvars as one number, with a bit that no start value has.
fn key(theme: Theme, hue: i64, saturation: i64) -> u64 {
    1 << 40
        | (theme as u64) << 32
        | (hue.clamp(0, HUE_MAX) as u64) << 16
        | saturation.clamp(0, SATURATION_MAX) as u64
}

/// The theme the cvars ask for: the theme, the hue and the saturation, each
/// falling back to its default when missing or out of range.
pub(crate) fn read(console: &ViewerConsole) -> (Theme, i64, i64) {
    let theme = Theme::from_cvar(console.text_value(CVAR));
    let hue = console
        .integer_cvar(HUE_CVAR)
        .unwrap_or(DEFAULT_HUE)
        .clamp(0, HUE_MAX);
    let saturation = console
        .integer_cvar(SATURATION_CVAR)
        .unwrap_or(DEFAULT_SATURATION)
        .clamp(0, SATURATION_MAX);
    (theme, hue, saturation)
}

/// Apply the theme cvars to the palette in use, when they changed since the last
/// call: the next frame draws in it, no restart needed.
pub(crate) fn sync(console: &ViewerConsole) {
    let (theme, hue, saturation) = read(console);
    let key = key(theme, hue, saturation);
    if APPLIED.swap(key, Ordering::Relaxed) != key {
        LIVE.set(&palette(theme, hue as f32, saturation as f32));
    }
}

/// Whether a write to cvar `name` picks the custom theme: the hue's and the
/// saturation's, which mean nothing to the presets.
pub(crate) fn follows_custom(name: &str) -> bool {
    name.eq_ignore_ascii_case(HUE_CVAR) || name.eq_ignore_ascii_case(SATURATION_CVAR)
}

/// The hue and saturation [`sync`] last applied (their defaults before it ran).
fn applied() -> (i64, i64) {
    let key = APPLIED.load(Ordering::Relaxed);
    if key == 0 {
        return (DEFAULT_HUE, DEFAULT_SATURATION);
    }
    (((key >> 16) & 0xffff) as i64, (key & 0xffff) as i64)
}

/// The swatch a Settings row shows by its slider, for row `cvar` at `value`: the
/// line-work of the custom theme at that hue (or saturation), with the other as set.
/// Other rows have none.
pub(crate) fn row_swatch(cvar: &str, value: &str) -> Option<Color> {
    let (hue, saturation) = applied();
    let value = value.trim().parse::<f32>().ok();
    if cvar == HUE_CVAR {
        Some(swatch(value.unwrap_or(hue as f32), saturation as f32))
    } else if cvar == SATURATION_CVAR {
        Some(swatch(hue as f32, value.unwrap_or(saturation as f32)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG's relative luminance of a display colour.
    fn luminance(colour: Color) -> f32 {
        luminance_of(linear(colour))
    }

    /// WCAG's contrast ratio of two display colours.
    fn contrast(a: Color, b: Color) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    fn white() -> Color {
        rgb(1.0, 1.0, 1.0)
    }

    /// `top` at its alpha over `under`.
    fn over(top: Color, under: Color) -> Color {
        let a = top.a;
        rgb(
            top.r * a + under.r * (1.0 - a),
            top.g * a + under.g * (1.0 - a),
            top.b * a + under.b * (1.0 - a),
        )
    }

    /// Every palette a test walks: the presets and custom ones round the wheel,
    /// at full, half and no saturation.
    fn palettes() -> Vec<(String, Palette)> {
        let mut all = vec![("warm".to_owned(), WARM), ("cold".to_owned(), COLD)];
        for hue in (0..360).step_by(15) {
            for saturation in [100, 50, 0] {
                all.push((
                    format!("custom {hue}/{saturation}"),
                    custom(hue as f32, saturation as f32),
                ));
            }
        }
        all
    }

    #[test]
    fn the_presets_keep_their_colours() {
        assert_eq!(WARM.space, rgb(0.078, 0.043, 0.024));
        assert_eq!(WARM.holo, rgb(1.0, 0.663, 0.471));
        assert_eq!(WARM.gold, rgb(1.0, 0.784, 0.239));
        assert_eq!(COLD.space, rgb(0.024, 0.039, 0.078));
        assert_eq!(COLD.holo, rgb(0.659, 0.812, 1.0));
        assert_eq!(COLD.gold, rgb(0.91, 0.722, 0.29));
        assert_eq!(COLD.ember, rgb(1.0, 0.478, 0.239));
        assert_eq!(palette(Theme::Warm, 200.0, 0.0), WARM);
        assert_eq!(palette(Theme::Cold, 200.0, 0.0), COLD);
        // The ground and the glass are warm in one (red over green over blue), cold
        // in the other.
        for ground in [WARM.space, WARM.glass, WARM.glass_lit] {
            assert!(ground.r > ground.g && ground.g > ground.b);
        }
        for ground in [COLD.space, COLD.glass, COLD.glass_lit] {
            assert!(ground.b > ground.g && ground.g > ground.r);
        }
    }

    #[test]
    fn every_palette_keeps_its_contrast() {
        for (name, p) in palettes() {
            let body = [("text", p.text), ("muted", p.muted)];
            let accents = [
                ("gold", p.gold),
                ("gold bright", p.gold_bright),
                ("holo", p.holo),
            ];
            // On the ground itself, as the menus draw it.
            for (token, colour) in body.into_iter().chain(accents) {
                let ratio = contrast(colour, p.space);
                assert!(ratio >= 7.0, "{name}: {token} on the ground: {ratio:.2}");
            }
            for (token, colour) in [("quiet", p.quiet), ("ember", p.ember)] {
                let ratio = contrast(colour, p.space);
                assert!(ratio >= 4.5, "{name}: {token} on the ground: {ratio:.2}");
            }
            // Over each kind of acrylic glass in front of the brightest scene (snow,
            // a white sky), blurred and tinted, and over its plain fallback.
            for amount in [
                super::super::color::acrylic::BAR,
                super::super::color::acrylic::BANNER,
                super::super::color::acrylic::PANEL,
                super::super::color::acrylic::CARD,
                super::super::color::acrylic::TILE,
            ] {
                let tint = super::super::color::alpha(p.space, amount);
                let [r, g, b] = sjk_ui::acrylic::shade([1.0; 3], tint);
                let glass = rgb(r, g, b);
                let plain = over(sjk_ui::acrylic::fallback(tint), white());
                for (token, colour) in body.into_iter().chain(accents) {
                    let ratio = contrast(colour, glass);
                    assert!(
                        ratio >= 4.5,
                        "{name}: {token} on acrylic {amount}: {ratio:.2}"
                    );
                }
                for (token, colour) in body {
                    let ratio = contrast(colour, plain);
                    assert!(
                        ratio >= 4.5,
                        "{name}: {token} on the fallback {amount}: {ratio:.2}"
                    );
                }
            }
        }
    }

    #[test]
    fn gold_stands_apart_from_the_line_work_and_ember() {
        for p in [WARM, COLD] {
            let gap = (wheel_hue(p.gold) - wheel_hue(p.holo)).rem_euclid(360.0);
            assert!((15.0..=345.0).contains(&gap), "gold against the line-work");
        }
        assert!(wheel_hue(WARM.holo) - wheel_hue(WARM.ember) >= 8.0);
    }

    #[test]
    fn a_custom_hue_keeps_every_tokens_luminance() {
        for (name, p) in palettes().into_iter().skip(2) {
            for (index, (turned, warm)) in p.tokens().into_iter().zip(WARM.tokens()).enumerate() {
                let (a, b) = (luminance(turned), luminance(warm));
                assert!(
                    (a - b).abs() <= 0.002 + b * 0.01,
                    "{name}: token {index} luminance {a:.4} against {b:.4}"
                );
            }
        }
    }

    #[test]
    fn a_custom_hue_turns_the_line_work_and_keeps_ember() {
        for hue in [0.0_f32, 60.0, 120.0, 180.0, 240.0, 300.0] {
            let p = custom(hue, 100.0);
            let got = wheel_hue(p.holo);
            let gap = (got - hue)
                .rem_euclid(360.0)
                .min((hue - got).rem_euclid(360.0));
            assert!(gap <= 20.0, "hue {hue}: line-work at {got:.1}");
            assert_eq!(p.ember, WARM.ember);
            assert_eq!(swatch(hue, 100.0), p.holo);
        }
        // No saturation leaves greys.
        let grey = custom(200.0, 0.0);
        for colour in grey.tokens().into_iter().take(Token::Ember as usize) {
            assert!((colour.r - colour.g).abs() < 0.01 && (colour.g - colour.b).abs() < 0.01);
        }
    }

    #[test]
    fn the_default_hue_is_the_warm_palette() {
        let p = custom(DEFAULT_HUE as f32, DEFAULT_SATURATION as f32);
        for (index, (a, b)) in p.tokens().into_iter().zip(WARM.tokens()).enumerate() {
            let near = (a.r - b.r)
                .abs()
                .max((a.g - b.g).abs())
                .max((a.b - b.b).abs());
            assert!(near < 0.02, "token {index}: {a:?} against {b:?}");
        }
        assert!((wheel_hue(WARM.holo) - DEFAULT_HUE as f32).abs() < 0.5);
    }

    #[test]
    fn the_theme_cvar_falls_back_to_warm() {
        assert_eq!(Theme::from_cvar(Some("cold")), Theme::Cold);
        assert_eq!(Theme::from_cvar(Some(" Custom ")), Theme::Custom);
        assert_eq!(Theme::from_cvar(Some("warm")), Theme::Warm);
        assert_eq!(Theme::from_cvar(Some("blue")), Theme::Warm);
        assert_eq!(Theme::from_cvar(Some("")), Theme::Warm);
        assert_eq!(Theme::from_cvar(None), Theme::Warm);
        assert_eq!(Theme::NAMES[Theme::default() as usize], Theme::DEFAULT_NAME);
        assert_eq!(key(Theme::Custom, 999, -4), key(Theme::Custom, HUE_MAX, 0));
        assert_ne!(key(Theme::Warm, 0, 0), 0);
        assert!(follows_custom("ui_themehue") && follows_custom(SATURATION_CVAR));
        assert!(!follows_custom(CVAR));
        assert_eq!(row_swatch(CVAR, "warm"), None);
        assert_eq!(row_swatch(HUE_CVAR, "240").map(|c| c.a), Some(1.0));
    }

    #[test]
    fn the_cvars_read_with_their_defaults_and_the_hue_picks_custom() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(
            read(&console),
            (Theme::Warm, DEFAULT_HUE, DEFAULT_SATURATION)
        );
        console.set_cvar(CVAR, "cold");
        assert_eq!(read(&console).0, Theme::Cold);
        console.set_cvar(CVAR, "nonsense");
        assert_eq!(read(&console).0, Theme::Warm);
        // Moving the hue (or the saturation) picks the custom theme.
        console.set_cvar(HUE_CVAR, "240");
        assert_eq!(read(&console), (Theme::Custom, 240, DEFAULT_SATURATION));
        console.set_cvar(CVAR, "warm");
        console.set_cvar(SATURATION_CVAR, "40");
        assert_eq!(read(&console), (Theme::Custom, 240, 40));
        // Out of range falls back into it.
        console.set_cvar(HUE_CVAR, "900");
        console.set_cvar(SATURATION_CVAR, "-3");
        assert_eq!(read(&console), (Theme::Custom, HUE_MAX, 0));
    }

    #[test]
    fn the_live_palette_changes_in_place() {
        let live = Live::new(&WARM);
        assert_eq!(live.get(Token::Space), WARM.space);
        assert_eq!(live.get(Token::Ember), WARM.ember);
        live.set(&COLD);
        assert_eq!(live.get(Token::Space), COLD.space);
        assert_eq!(live.get(Token::GoldBright), COLD.gold_bright);
    }
}
