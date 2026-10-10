//! The JoF clan's mark (`docs/identity.md`, "JoF clan tag"): a player whose name carries the clan's
//! tag gets the clan's emblem (J, o, F; `assets/branding/jof-emblem.png`) on the left
//! of their name in the chat, on the scoreboard, on the player card and on the SJK
//! UI's Players page. The tag is read from the name alone, so it is what the player
//! says, not something the hub vouches for.
//!
//! The clan's rule: `jof` in any case, with no letter right before the J (it starts
//! the name or follows a separator: a space, a bracket, a dot, a digit...) and none
//! right after the F. Colour codes are dropped first, so `^1J^7oF` counts and
//! `Joffrey` or `MrJoF` do not.

use sjk_ui::{Color, DrawCommand, Rect, TextureId};

/// The emblem's tint: the clan's crimson.
pub(crate) const TINT: Color = Color::new(0.93, 0.24, 0.26, 1.0);

/// The emblem's atlas cell (`ui_renderer::JOF_TEXTURE`).
pub(crate) const TEXTURE: TextureId = crate::ui_renderer::JOF_TEXTURE;

/// The emblem, white on transparent, uploaded into [`TEXTURE`] at start.
pub(crate) const EMBLEM_PNG: &[u8] = include_bytes!("../../../assets/branding/jof-emblem.png");

/// Share of the emblem's square its ink spans across (the cell is square, the
/// emblem tall).
const INK_WIDTH: f32 = 0.65;
/// The gap after the emblem's ink, as a share of its side.
const GAP: f32 = 0.3;
/// The emblem's side beside a name in text `size` pixels tall.
pub(crate) fn side(size: f32) -> f32 {
    size * 0.95
}

/// The room the emblem `side` tall takes before a name, the gap after it included.
pub(crate) fn room(side: f32) -> f32 {
    side * (INK_WIDTH + GAP)
}

/// The emblem `side` tall with its ink from `x`, centred on `middle`, at `alpha`.
pub(crate) fn draw(x: f32, middle: f32, side: f32, alpha: f32, mut push: impl FnMut(DrawCommand)) {
    push(DrawCommand::TexturedQuad {
        rect: Rect::new(
            x - side * (1.0 - INK_WIDTH) * 0.5,
            middle - side * 0.5,
            side,
            side,
        ),
        texture: TEXTURE,
        color: Color {
            a: TINT.a * alpha,
            ..TINT
        },
    });
}

/// Whether `name` (with its colour codes) carries the JoF tag.
pub(crate) fn tagged(name: &str) -> bool {
    let plain = sjk_game_jka::client_view::strip_colours(name.as_bytes());
    let plain = String::from_utf8_lossy(&plain);
    let chars: Vec<char> = plain.chars().collect();
    chars.windows(3).enumerate().any(|(at, three)| {
        three[0].eq_ignore_ascii_case(&'j')
            && three[1].eq_ignore_ascii_case(&'o')
            && three[2].eq_ignore_ascii_case(&'f')
            && (at == 0 || !chars[at - 1].is_alphabetic())
            && chars.get(at + 3).is_none_or(|next| !next.is_alphabetic())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_counts_in_any_case_behind_any_separator() {
        for name in [
            "{JoF}^1emiah{I}",
            "JoF|Sol",
            "jof.Sol",
            "JOF-Sol",
            "[jof] Sol",
            "Sol JoF",
            "Sol-JoF",
            "^1J^7oF^3|Sol",
            "2jof3",
            "JoF",
            "*~JoF~*",
        ] {
            assert!(tagged(name), "{name}");
        }
    }

    #[test]
    fn a_letter_on_either_side_is_not_the_tag() {
        // A colour code is no separator: `^1JoF^7Sol` reads "JoFSol".
        for name in [
            "Joffrey",
            "MrJoF",
            "Jofa",
            "ajof",
            "éJoF",
            "JoFé",
            "J0F",
            "J oF",
            "^1JoF^7Sol",
            "",
        ] {
            assert!(!tagged(name), "{name}");
        }
    }
}
