//! Acrylic glass ([`crate::DrawCommand::Backdrop`]): what is behind a panel, blurred, its
//! brightness held under a ceiling, a tint over it, a faint grain and a light top edge, as
//! Windows' acrylic and iOS's materials draw a panel. The renderer blurs and adds the
//! grain and edge; [`shade`] is the reference for the colour it then gives a pixel
//! (`ui_shapes.wgsl` evaluates the same expression), and [`fallback`] the plain tint a
//! backdrop draws as where nothing behind it can be blurred.
//!
//! The ceiling is what keeps text legible however clear the glass is: a bright sky behind a
//! panel is held to [`LUMA_CEILING`] before the tint goes over it, so text over the
//! panel keeps its contrast over snow as over a dark corridor, while darker scenes show
//! through untouched.

use crate::Color;

/// The brightest a blurred backdrop shows under the tint, as a display value's Rec. 709
/// luma; brighter pixels are scaled down to it, keeping their hue.
pub const LUMA_CEILING: f32 = 0.42;

/// How much of the way from a backdrop's tint alpha to opaque its plain [`fallback`] goes:
/// without the blur and the ceiling, the sharp scene behind needs more cover for text to
/// read.
pub const FALLBACK_GAIN: f32 = 0.6;

/// The colour of acrylic with `tint` over `blurred`, both display values: the blurred
/// backdrop held under [`LUMA_CEILING`], then `tint` over it at its alpha. The grain and
/// the lit top edge the renderer adds are left out.
pub fn shade(blurred: [f32; 3], tint: Color) -> [f32; 3] {
    let luma = 0.2126 * blurred[0] + 0.7152 * blurred[1] + 0.0722 * blurred[2];
    let held = (LUMA_CEILING / luma.max(1.0e-4)).min(1.0);
    let tint_rgb = [tint.r, tint.g, tint.b];
    std::array::from_fn(|i| blurred[i] * held * (1.0 - tint.a) + tint_rgb[i] * tint.a)
}

/// What a backdrop with `tint` draws where the renderer cannot blur what is behind it
/// (a swapchain it may not sample, the CPU menu snapshots): the tint alone, opaquer.
pub fn fallback(tint: Color) -> Color {
    let alpha = tint.a.clamp(0.0, 1.0);
    Color::new(
        tint.r,
        tint.g,
        tint.b,
        alpha + (1.0 - alpha) * FALLBACK_GAIN,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_scenes_show_through_and_bright_ones_are_held() {
        let clear = Color::new(0.0, 0.0, 0.0, 0.0);
        // A dark corridor passes untouched.
        let dark = shade([0.2, 0.15, 0.1], clear);
        assert!((dark[0] - 0.2).abs() < 1e-6 && (dark[2] - 0.1).abs() < 1e-6);
        // Snow is held to the ceiling, keeping its hue.
        let snow = shade([1.0, 1.0, 1.0], clear);
        for channel in snow {
            assert!((channel - LUMA_CEILING).abs() < 1e-5);
        }
        let sky = shade([0.4, 0.7, 1.0], clear);
        assert!(sky[2] > sky[1] && sky[1] > sky[0]);
    }

    #[test]
    fn the_tint_goes_over_at_its_alpha() {
        let tint = Color::new(0.1, 0.05, 0.0, 0.5);
        let out = shade([0.3, 0.3, 0.3], tint);
        assert!((out[0] - 0.2).abs() < 1e-5);
        assert!((out[1] - 0.175).abs() < 1e-5);
        assert!((out[2] - 0.15).abs() < 1e-5);
    }

    #[test]
    fn the_fallback_is_the_tint_opaquer() {
        let tint = Color::new(0.1, 0.05, 0.0, 0.4);
        let plain = fallback(tint);
        assert_eq!((plain.r, plain.g, plain.b), (0.1, 0.05, 0.0));
        assert!(plain.a > tint.a && plain.a < 1.0);
        assert_eq!(fallback(Color::new(0.0, 0.0, 0.0, 1.0)).a, 1.0);
    }
}
