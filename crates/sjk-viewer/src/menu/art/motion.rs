//! Retail menu motion: what the original menus animate, as functions of a
//! shared clock, so the classic views and the renderer agree.
//!
//! None of it is scripted in the `.menu` files, whose page swaps are instant
//! (`close all ; open ...`) and whose fades step 0.1 per millisecond
//! (`fadeCycle 1`, `fadeAmount 0.1`). It comes from the artwork's shaders in
//! `shaders/ui.shader` and from the menu code (`ui_shared.c`):
//!
//! - `main_ring` turns (`tcMod rotate 5`), see [`rotation`];
//! - the side glyph columns climb (`tcMod scroll 0 0.025`) and the logo's
//!   `env_logo` reflection drifts (`tcMod scroll 0.03 0.05`), see [`scroll`];
//! - the focus glows and the in-game bar flicker with four layers of
//!   `gfx/hud/static_menu` multiplied over them (`blendFunc GL_DST_COLOR
//!   GL_ONE`, `tcMod scroll ±1`, `±1.3` or `±1.7`), see [`flicker`] and
//!   [`compose_flicker`];
//! - the focused item's text breathes between its colour and 80% of it
//!   (`Item_TextColor`, `PULSE_DIVISOR` 75), see [`pulse`].
//!
//! SJK adds its own: the menu emblem ([`crate::menu::emblem`]) that stands
//! where retail played its logo video, steel before a burning sun (16/11/2026).
//! The sun's layers move: its disc turns slowly and breathes ([`SUN_DISC_TURN`],
//! [`sun_breath`]), its two flame layers turn against each other and cross-fade
//! on an uneven beat so the tongues seem to lick ([`SUN_FLAMES_TURN`],
//! [`sun_flicker`]), its rays shimmer ([`sun_shimmer`]) and its embers drift out
//! ([`ember_drift`]).

use super::ArtPiece;
use image::RgbaImage;
use sjk_ui::Color;
use std::sync::OnceLock;
use std::time::Instant;

/// `main_ring`'s `tcMod rotate`, in degrees per second.
pub(crate) const RING_DEGREES_PER_SECOND: f32 = 5.0;
/// `menu_side_text`'s `tcMod scroll`, in textures per second.
pub(crate) const SIDE_SCROLL: [f32; 2] = [0.0, 0.025];
/// `jediacademy`'s `env_logo` stage: `tcMod scroll` and `alphaGen const`.
pub(crate) const LOGO_REFLECTION_SCROLL: [f32; 2] = [0.03, 0.05];
pub(crate) const LOGO_REFLECTION_ALPHA: f32 = 0.25;
/// `PULSE_DIVISOR` (`q_shared.h`): milliseconds per radian of the focus pulse.
const PULSE_DIVISOR: f64 = 75.0;
/// Seconds the sun's disc takes to turn once.
pub(crate) const SUN_DISC_TURN: f64 = 240.0;
/// Seconds each flame layer takes to turn once; opposite ways, at rates whose
/// ratio is no simple fraction, so the pattern does not visibly repeat.
pub(crate) const SUN_FLAMES_TURN: [f64; 2] = [170.0, -113.0];
/// Periods, in seconds, of the three sines the flames' cross-fade sums.
const SUN_FLICKER_PERIODS: [f64; 3] = [2.3, 0.97, 0.53];
/// Seconds one ember layer takes to drift from its start to its end.
pub(crate) const EMBER_DRIFT: f64 = 7.5;

/// Seconds on the menu clock, which starts the first time it is read.
pub(crate) fn seconds() -> f64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// `color` as retail draws a focused item at `seconds`: `Item_TextColor`
/// lerps from the focus colour towards 80% of it (alpha included) by
/// `0.5 + 0.5 sin(ms / 75)`.
pub(crate) fn pulse(color: Color, seconds: f64) -> Color {
    let toward_low = 0.5 + 0.5 * (seconds * 1_000.0 / PULSE_DIVISOR).sin();
    let factor = 1.0 - 0.2 * toward_low as f32;
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a * factor,
    )
}

/// The second flame layer's share of the flames at `seconds`, 0..=1 (the first
/// has the rest, so the light stays even while the tongues change): three sines
/// of no common period, eased so it lingers near either layer.
pub(crate) fn sun_flicker(seconds: f64) -> f32 {
    let wave: f64 = SUN_FLICKER_PERIODS
        .iter()
        .zip([0.55, 0.3, 0.15])
        .enumerate()
        .map(|(index, (period, weight))| {
            weight * (std::f64::consts::TAU * seconds / period + index as f64 * 1.7).sin()
        })
        .sum();
    (0.5 + 0.5 * wave).clamp(0.0, 1.0) as f32
}

/// The disc's strength at `seconds`: a slow, shallow breath, 0.9..=1.
pub(crate) fn sun_breath(seconds: f64) -> f32 {
    (0.95 + 0.05 * (std::f64::consts::TAU * seconds / 5.3).sin()) as f32
}

/// The rays' strength at `seconds`: a quick, shallow shimmer, 0.82..=1.
pub(crate) fn sun_shimmer(seconds: f64) -> f32 {
    let tau = std::f64::consts::TAU;
    let wave = 0.6 * (tau * seconds / 1.3).sin() + 0.4 * (tau * seconds / 0.47 + 1.1).sin();
    (0.91 + 0.09 * wave) as f32
}

/// Ember layer `index` of two at `seconds`: its scale (it grows, so its sparks
/// fly out) and its strength (it fades in and out, so the restart never shows).
/// The two run half a drift apart.
pub(crate) fn ember_drift(seconds: f64, index: usize) -> (f32, f32) {
    let phase = (seconds / EMBER_DRIFT + index as f64 * 0.5).rem_euclid(1.0);
    let scale = 0.82 + 0.5 * phase;
    let strength = (std::f64::consts::PI * phase).sin().powi(2);
    (scale as f32, strength as f32)
}

/// Corner texture coordinates (top-left, top-right, bottom-right,
/// bottom-left) of a quad whose texture turns at `degrees_per_second`
/// about its centre, as `RB_CalcRotateTexCoords` (`tr_shade_calc.cpp`):
/// the coordinates turn by `-degrees_per_second * seconds`.
pub(crate) fn rotation(degrees_per_second: f32, seconds: f64) -> [[f32; 2]; 4] {
    let degrees = (-f64::from(degrees_per_second) * seconds).rem_euclid(360.0);
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (sin, cos) = (sin as f32, cos as f32);
    let turn = |[s, t]: [f32; 2]| {
        [
            s * cos - t * sin + (0.5 - 0.5 * cos + 0.5 * sin),
            s * sin + t * cos + (0.5 - 0.5 * sin - 0.5 * cos),
        ]
    };
    [
        turn([0.0, 0.0]),
        turn([1.0, 0.0]),
        turn([1.0, 1.0]),
        turn([0.0, 1.0]),
    ]
}

/// Corner texture coordinates of a quad whose (wrapping) texture scrolls by
/// `speed` textures per second (`tcMod scroll`); the offset is kept in
/// `[0, 1)` so precision does not drift.
pub(crate) fn scroll(speed: [f32; 2], seconds: f64) -> [[f32; 2]; 4] {
    let offset = |speed: f32| (f64::from(speed) * seconds).rem_euclid(1.0) as f32;
    let [s, t] = [offset(speed[0]), offset(speed[1])];
    [[s, t], [s + 1.0, t], [s + 1.0, t + 1.0], [s, t + 1.0]]
}

/// Horizontal scroll speeds of the four `static_menu` layers a piece's
/// shader multiplies over it, for the pieces that flicker.
pub(crate) fn flicker(piece: ArtPiece) -> Option<[f32; 4]> {
    match piece {
        // `gfx/menus/menu_buttonback`.
        ArtPiece::ButtonBack => Some([-1.0, 1.0, -1.3, 1.3]),
        // `gfx/menus/menu_blendbox`, `menu_blendbox2` and `menu_top_mp`.
        ArtPiece::BlendBox | ArtPiece::BlendBox2 | ArtPiece::TopBar => Some([1.0, -1.0, 1.7, -1.7]),
        _ => None,
    }
}

/// Recompose a flickering piece at `seconds` into `out` (RGBA, `base`'s
/// size), ready to upload. Retail adds the piece to the screen, then each
/// noise layer multiplies the screen by `1 + noise`; over SJK's
/// alpha-blended UI the product is taken over the piece alone and turned
/// into colour and coverage as the static pieces are
/// ([`super::additive_texel`]). Noise is sampled at the nearest texel.
pub(crate) fn compose_flicker(
    base: &RgbaImage,
    noise: &RgbaImage,
    speeds: [f32; 4],
    seconds: f64,
    out: &mut [u8],
) {
    let (width, height) = base.dimensions();
    let (noise_width, noise_height) = noise.dimensions();
    if out.len() != (width * height * 4) as usize || noise_width == 0 || noise_height == 0 {
        return;
    }
    let offsets = speeds.map(|speed| (f64::from(speed) * seconds).rem_euclid(1.0) as f32);
    for y in 0..height {
        let noise_y = (((y as f32 + 0.5) / height as f32) * noise_height as f32) as u32;
        let noise_y = noise_y.min(noise_height - 1);
        for x in 0..width {
            let s = (x as f32 + 0.5) / width as f32;
            let mut gain = [1.0_f32; 3];
            for offset in offsets {
                let noise_x =
                    (((s + offset).fract() * noise_width as f32) as u32).min(noise_width - 1);
                let texel = noise.get_pixel(noise_x, noise_y).0;
                for channel in 0..3 {
                    gain[channel] *= 1.0 + f32::from(texel[channel]) / 255.0;
                }
            }
            let [r, g, b, a] = base.get_pixel(x, y).0;
            let lit = |channel: u8, gain: f32| (f32::from(channel) * gain).min(255.0) as u8;
            let texel =
                super::additive_texel([lit(r, gain[0]), lit(g, gain[1]), lit(b, gain[2]), a]);
            let at = ((y * width + x) * 4) as usize;
            out[at..at + 4].copy_from_slice(&texel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
    }

    #[test]
    fn rotation_starts_square_and_turns_about_the_centre() {
        let start = rotation(RING_DEGREES_PER_SECOND, 0.0);
        for (corner, expected) in start
            .iter()
            .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
        {
            assert!(close(*corner, expected));
        }
        // 18 s at 5 degrees per second: a quarter turn, the coordinates
        // turned by -90 degrees, so the top-left corner samples the top-right.
        let quarter = rotation(RING_DEGREES_PER_SECOND, 18.0);
        assert!(close(quarter[0], [0.0, 1.0]), "{:?}", quarter[0]);
        assert!(close(quarter[1], [0.0, 0.0]), "{:?}", quarter[1]);
        // The centre never moves.
        let mid = |uv: [[f32; 2]; 4]| [(uv[0][0] + uv[2][0]) * 0.5, (uv[0][1] + uv[2][1]) * 0.5];
        assert!(close(mid(rotation(5.0, 7.3)), [0.5, 0.5]));
        // A full turn comes back.
        let full = rotation(RING_DEGREES_PER_SECOND, 72.0);
        assert!(close(full[2], [1.0, 1.0]));
    }

    #[test]
    fn scrolling_wraps_its_offset() {
        let uv = scroll(SIDE_SCROLL, 10.0);
        assert!(close(uv[0], [0.0, 0.25]));
        assert!(close(uv[2], [1.0, 1.25]));
        // 40 s is one full texture: back to the start.
        assert!(close(scroll(SIDE_SCROLL, 40.0)[0], [0.0, 0.0]));
        assert!(close(scroll([-1.0, 0.0], 0.25)[0], [0.75, 0.0]));
    }

    #[test]
    fn pulse_breathes_between_full_and_eighty_percent() {
        let white = Color::new(1.0, 1.0, 1.0, 1.0);
        // sin(0) = 0: halfway, 90%.
        assert!((pulse(white, 0.0).r - 0.9).abs() < 1e-6);
        let mut low = f32::MAX;
        let mut high = f32::MIN;
        for step in 0..1_000 {
            let color = pulse(white, f64::from(step) * 0.001);
            low = low.min(color.r);
            high = high.max(color.a);
        }
        assert!((low - 0.8).abs() < 1e-3 && (high - 1.0).abs() < 1e-3);
        // One cycle is 2 pi * 75 ms.
        let period = std::f64::consts::TAU * 0.075;
        assert!((pulse(white, 0.123).r - pulse(white, 0.123 + period).r).abs() < 1e-5);
    }

    #[test]
    fn the_sun_moves_gently_within_its_ranges() {
        let (mut least, mut most) = (f32::MAX, f32::MIN);
        for step in 0..20_000 {
            let at = f64::from(step) * 0.003;
            let share = sun_flicker(at);
            least = least.min(share);
            most = most.max(share);
            assert!((0.9..=1.0).contains(&sun_breath(at)));
            assert!((0.81..=1.0).contains(&sun_shimmer(at)));
            // No jump between two 60 Hz frames.
            assert!((sun_flicker(at + 1.0 / 60.0) - share).abs() < 0.08);
            let (scale, strength) = ember_drift(at, step as usize % 2);
            assert!((0.82..=1.32).contains(&scale) && (0.0..=1.0).contains(&strength));
        }
        // The flames use most of their cross-fade.
        assert!(least < 0.15 && most > 0.85, "{least}..{most}");
    }

    #[test]
    fn embers_restart_unseen_and_half_a_drift_apart() {
        let (_, at_start) = ember_drift(0.0, 0);
        let (_, at_end) = ember_drift(EMBER_DRIFT - 1e-6, 0);
        assert!(at_start < 1e-6 && at_end < 1e-6);
        let (_, other) = ember_drift(0.0, 1);
        assert!((other - 1.0).abs() < 1e-6);
        // They grow as they go.
        assert!(ember_drift(1.0, 0).0 < ember_drift(5.0, 0).0);
    }

    #[test]
    fn only_the_static_shaders_flicker() {
        assert_eq!(flicker(ArtPiece::ButtonBack), Some([-1.0, 1.0, -1.3, 1.3]));
        assert!(flicker(ArtPiece::TopBar).is_some());
        assert!(flicker(ArtPiece::Ring).is_none());
        assert!(ArtPiece::ButtonBack.dynamic());
        assert!(!ArtPiece::Logo.dynamic());
    }

    #[test]
    fn flicker_brightens_the_glow_by_its_noise() {
        let base = RgbaImage::from_pixel(4, 2, image::Rgba([40, 80, 120, 255]));
        let mut out = vec![0_u8; 4 * 2 * 4];
        // No noise: the glow as the static piece converts it.
        let quiet = RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]));
        compose_flicker(&base, &quiet, [1.0; 4], 3.7, &mut out);
        assert_eq!(&out[..4], &super::super::additive_texel([40, 80, 120, 255]));
        // Full noise doubles four times, clamped at white.
        let loud = RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
        compose_flicker(&base, &loud, [1.0; 4], 0.0, &mut out);
        assert_eq!(&out[..4], &[255, 255, 255, 255]);
        // A wrong-sized output is left alone.
        let mut short = vec![7_u8; 3];
        compose_flicker(&base, &loud, [1.0; 4], 0.0, &mut short);
        assert_eq!(short, [7, 7, 7]);
    }

    #[test]
    fn flicker_noise_moves_with_time() {
        let base = RgbaImage::from_pixel(8, 1, image::Rgba([60, 60, 60, 255]));
        // Noise lit in its left half only.
        let mut noise = RgbaImage::from_pixel(8, 1, image::Rgba([0, 0, 0, 255]));
        for x in 0..4 {
            noise.put_pixel(x, 0, image::Rgba([255, 255, 255, 255]));
        }
        let speeds = [0.5; 4];
        let mut early = vec![0_u8; 8 * 4];
        let mut later = vec![0_u8; 8 * 4];
        compose_flicker(&base, &noise, speeds, 0.0, &mut early);
        compose_flicker(&base, &noise, speeds, 1.0, &mut later);
        // Half a texture later the lit half has moved to the other side.
        assert_eq!(&early[..4], &later[16..20]);
        assert_ne!(early, later);
    }
}
