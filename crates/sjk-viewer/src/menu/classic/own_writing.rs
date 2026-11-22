//! The columns of Aurebesh writing at the sides of the classic menus' own look.
//!
//! Retail's side columns are pictures of small writing in Aurebesh, the Star Wars
//! alphabet. The own look writes its own, with Aurebesh AF by AurekFonts (public
//! domain, CC0: `assets/fonts/AurebeshAF-LICENSE.txt`), into one tall picture made
//! once at start: white letters on nothing, which the page tints with the theme's
//! line-work and tiles down the side ([`ArtPiece::OwnWriting`](crate::menu::art::ArtPiece)).
//! The words are SJK's own, in ragged lines as a readout has them.

use image::{Rgba, RgbaImage};

const FONT: &[u8] = include_bytes!("../../../assets/fonts/AurebeshAF-Canon.otf");

/// The picture's size: a column eight times as tall as it is wide.
const SIZE: [u32; 2] = [256, 2048];
/// The letters' size and the lines' step, in the picture's pixels.
const LETTER: f32 = 34.0;
const LINE: u32 = 46;

/// What the columns say, a line each; an empty one is a gap between blocks.
const LINES: &[&str] = &[
    "SUN JK",
    "JEDI ACADEMY",
    "MULTIPLAYER",
    "",
    "SABER",
    "FORCE",
    "HOLOCRON",
    "KYBER",
    "",
    "PROTOCOL 26",
    "CLIENT READY",
    "SERVERS",
    "",
    "DUEL",
    "POWER DUEL",
    "FREE FOR ALL",
    "CAPTURE",
    "SIEGE",
    "",
    "LIGHT",
    "DARK",
    "BALANCE",
    "",
    "YAVIN",
    "KORRIBAN",
    "TATOOINE",
    "HOTH",
    "",
    "PUSH PULL",
    "SPEED",
    "SENSE",
    "ABSORB",
    "PROTECT",
    "",
    "RENDERER",
    "SOLAR",
    "THEME",
    "",
    "MAY THE",
    "FORCE BE",
    "WITH YOU",
    "",
    "SJK",
];

/// The columns' picture, or `None` if the font cannot be read (the pages then draw
/// dashes in its place).
pub(crate) fn image() -> Option<RgbaImage> {
    let font = fontdue::Font::from_bytes(FONT, fontdue::FontSettings::default()).ok()?;
    let mut picture = RgbaImage::from_pixel(SIZE[0], SIZE[1], Rgba([255, 255, 255, 0]));
    let rows = SIZE[1] / LINE;
    for row in 0..rows {
        let text = LINES[row as usize % LINES.len()];
        let baseline = (row * LINE) as f32 + LETTER;
        let mut pen = 6.0_f32;
        for letter in text.chars() {
            let (metrics, coverage) = font.rasterize(letter, LETTER);
            if pen + metrics.advance_width > SIZE[0] as f32 - 4.0 {
                break;
            }
            let left = pen as i32 + metrics.xmin;
            let top = baseline as i32 - metrics.height as i32 - metrics.ymin;
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let (px, py) = (left + x as i32, top + y as i32);
                    if px < 0 || py < 0 || px >= SIZE[0] as i32 || py >= SIZE[1] as i32 {
                        continue;
                    }
                    let alpha = coverage[y * metrics.width + x];
                    let pixel = picture.get_pixel_mut(px as u32, py as u32);
                    pixel.0[3] = pixel.0[3].max(alpha);
                }
            }
            pen += metrics.advance_width;
        }
    }
    Some(picture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_columns_are_written() {
        let picture = image().expect("the font reads");
        assert_eq!(picture.dimensions(), (SIZE[0], SIZE[1]));
        // Letters were drawn, and they are white: only their alpha says where.
        let inked = picture.pixels().filter(|pixel| pixel.0[3] > 128).count();
        assert!(inked > 20_000, "{inked} pixels of writing");
        assert!(
            picture
                .pixels()
                .all(|pixel| pixel.0[..3] == [255, 255, 255])
        );
        // The picture tiles: its lines fill its height.
        assert_eq!(SIZE[1] % LINE, 2048 % 46);
    }

    #[test]
    fn every_line_is_capitals_the_font_has() {
        let font = fontdue::Font::from_bytes(FONT, fontdue::FontSettings::default()).unwrap();
        for line in LINES {
            for letter in line.chars().filter(|letter| *letter != ' ') {
                assert!(font.lookup_glyph_index(letter) != 0, "{line}: {letter}");
            }
        }
    }
}
