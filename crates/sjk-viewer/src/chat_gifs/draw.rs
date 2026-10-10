//! How a GIF shows under its chat line, on the SJK chat page and the docks: the GIF at
//! a fixed height ([`Look::height`], smaller on the docks), as wide as its shape makes
//! it within the column (narrower and lower when the column is too narrow); while it
//! loads, a quiet box of that height saying so; when it could not be had, one quiet
//! line, "GIF unavailable".

use super::Shown;
use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The GIF block's measures, in frame pixels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Look {
    /// The room above the GIF, under the line's text.
    pub(crate) gap: f32,
    /// The GIF's height (its width follows its shape).
    pub(crate) height: f32,
    /// The height of the "GIF unavailable" line.
    pub(crate) note: f32,
    /// The size of the block's words.
    pub(crate) text_size: f32,
}

/// The SJK chat page's GIFs.
pub(crate) const PAGE: Look = Look {
    gap: 6.0,
    height: 160.0,
    note: 24.0,
    text_size: 15.0,
};

/// The docks' GIFs (main page, in-game menu), smaller.
pub(crate) const DOCK: Look = Look {
    gap: 4.0,
    height: 48.0,
    note: 20.0,
    text_size: 13.0,
};

/// The shape a loading GIF's box takes, its size not known yet.
const PLACEHOLDER_SHAPE: f32 = 1.5;

/// The room the block takes under its line (frame pixels).
pub(crate) fn room(look: Look, shown: Shown) -> f32 {
    match shown {
        Shown::Unavailable => look.note,
        Shown::Loading | Shown::Ready { .. } => look.gap + look.height,
    }
}

/// The size (frame pixels) a GIF of `size` (or a placeholder, `None`) is drawn at: the
/// look's height, its width from the GIF's shape, both made smaller to fit `width`.
pub(crate) fn fitted(look: Look, size: Option<[u32; 2]>, width: f32) -> [f32; 2] {
    let shape = size
        .filter(|[w, h]| *w > 0 && *h > 0)
        .map_or(PLACEHOLDER_SHAPE, |[w, h]| w as f32 / h as f32);
    let wide = look.height * shape;
    if wide <= width {
        [wide, look.height]
    } else {
        let width = width.max(1.0);
        [width, width / shape]
    }
}

/// Draw the block for `shown` with its top at `y` and its left at `x` (frame pixels)
/// in a column `width` wide.
pub(crate) fn draw(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    look: Look,
    shown: Shown,
    [x, y]: [f32; 2],
    width: f32,
) {
    let s = frame.s;
    match shown {
        Shown::Ready { texture, size } => {
            let [w, h] = fitted(look, Some(size), width);
            let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(x, y + look.gap, w, h),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        Shown::Loading => {
            let [w, h] = fitted(look, None, width);
            let rect = frame.rect(x, y + look.gap, w, h);
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 6.0 * s,
                color: color::alpha(color::HOLO, 0.07),
            });
            text(
                canvas,
                TextFamily::Body,
                format_args!("Loading GIF"),
                rect,
                look.text_size * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        Shown::Unavailable => text(
            canvas,
            TextFamily::Body,
            format_args!("GIF unavailable"),
            frame.rect(x, y, width, look.note - 2.0),
            look.text_size * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gif_keeps_its_shape_within_the_column() {
        assert_eq!(fitted(PAGE, Some([200, 200]), 1_180.0), [160.0, 160.0]);
        assert_eq!(fitted(PAGE, Some([400, 200]), 1_180.0), [320.0, 160.0]);
        // Too wide for the column: as wide as it, lower.
        assert_eq!(fitted(DOCK, Some([480, 60]), 300.0), [300.0, 37.5]);
        assert_eq!(fitted(DOCK, None, 600.0), [72.0, 48.0]);
        assert_eq!(fitted(DOCK, Some([0, 0]), 600.0), [72.0, 48.0]);
    }

    #[test]
    fn the_block_takes_its_height_while_loading_and_a_line_when_unavailable() {
        let ready = Shown::Ready {
            texture: super::super::slot_texture(0),
            size: [200, 200],
        };
        assert_eq!(room(PAGE, Shown::Loading), room(PAGE, ready));
        assert_eq!(room(PAGE, ready), 166.0);
        // On the docks, a one-row message with its GIF and two more fill the box.
        assert_eq!(room(DOCK, ready), 52.0);
        assert_eq!(room(DOCK, Shown::Unavailable), 20.0);
    }
}
