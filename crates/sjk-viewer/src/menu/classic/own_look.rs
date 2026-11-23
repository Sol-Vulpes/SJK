//! The classic menus in SJK's own look (`ui_classicLook sjk`, `docs/classic-plus.md`).
//!
//! The classic style follows the retail menus' pages, entries and flow. Its look was
//! retail's too: the player's own 640x480 artwork stretched over the window and
//! retail's blues and golds, which on a large screen is soft pictures in colours no
//! other SJK screen has. The own look keeps the pages and the flow and changes only
//! what is drawn:
//!
//! - retail's pictures are not used, but for the game's logo: every other piece is
//!   drawn ([`art`] leaves the pages only the logo, so each takes its drawn version,
//!   and [`super::own_art`] redraws the ground, the frames and the ring), sharp at
//!   any size;
//! - retail's colours become the theme's ([`crate::menu::sjk::theme`]): Solar, Sith,
//!   Jedi or a custom one, as in the SJK UI. One table ([`COLOURS`]) says which token
//!   each retail colour takes; the pages go on naming retail's constants, and the
//!   canvas swaps them when the page is finished ([`recolour`]).
//!
//! `ui_classicLook retail` (the default: the stock look is the default look, Sol's
//! rule of 30/10/2026) keeps the retail pictures and colours.
//!
//! Shelved for v1 since 23/11/2026 ([`SHELVED`], Sol's call: a first step, not finished):
//! the Settings row is gone and the classic menus keep the retail look whatever the
//! cvar says. `SJK_CLASSIC_LOOK=sjk` on `menu_snapshot` still draws it for work on it.
//! To bring it back: `SHELVED` to false, and the row back in `settings/catalog.rs`,
//! `settings/groups.rs` (Interface), `settings/sections.rs` (Menus) and its lines in
//! `settings/help.rs`, `settings/requires_rules.rs` and `ejk_mode.rs` (`retail`).

use crate::menu::art::{ArtPiece, ArtSet};
use crate::menu::sjk::theme::Token;
use sjk_ui::Color;
use std::sync::atomic::{AtomicBool, Ordering};

/// The classic menus' look: `retail` or `sjk`.
pub(crate) const CVAR: &str = "ui_classicLook";
/// Values the cvar takes.
pub(crate) const NAMES: [&str; 2] = ["retail", "sjk"];
/// The own look is shelved: the classic menus draw retail's, whatever [`CVAR`] says.
pub(crate) const SHELVED: bool = true;

/// Whether the own look is on, read by the canvases as they finish a page.
static OWN: AtomicBool = AtomicBool::new(false);

/// Whether `value` of [`CVAR`] asks for the own look (`sjk`, any case); anything
/// else, a missing or mistyped value included, is retail's.
fn from_cvar(value: Option<&str>) -> bool {
    value.is_some_and(|text| text.trim().eq_ignore_ascii_case(NAMES[1]))
}

/// Whether the classic menus draw the own look for `value` of [`CVAR`]: never while it
/// is [`SHELVED`].
pub(crate) fn chosen(value: Option<&str>) -> bool {
    !SHELVED && from_cvar(value)
}

/// Turn the own look on or off for the pages drawn from now on.
pub(crate) fn set(own: bool) {
    OWN.store(own, Ordering::Relaxed);
}

/// Whether the own look is on.
pub(crate) fn on() -> bool {
    OWN.load(Ordering::Relaxed)
}

/// The pictures the own look still draws: the game's logo, its letters alone
/// ([`ArtPiece::OwnLogo`], cut at start from the player's own game data: it is the
/// game's name, and no drawing of ours stands for it), and the columns of writing
/// the client makes ([`super::own_writing`]).
const KEPT: [ArtPiece; 2] = [ArtPiece::OwnLogo, ArtPiece::OwnWriting];

/// The art the classic pages may draw: `ready`, what the game data gave, or in the
/// own look only [`KEPT`] of it.
pub(crate) fn art(ready: ArtSet) -> ArtSet {
    if !on() {
        return ready;
    }
    KEPT.into_iter()
        .filter(|piece| ready.has(*piece))
        .fold(ArtSet::default(), ArtSet::with)
}

/// How a retail colour is taken from the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Take {
    /// The token as it is.
    Token(Token),
    /// The token, mixed this far towards the theme's ground: a frame or a rule,
    /// which retail drew in a darker blue than its text.
    Dimmed(Token, f32),
}

/// Each retail colour the classic pages name (red, green, blue) and what it becomes.
/// The alpha a page gave it is kept. A colour not listed (white focus, greys, the
/// team colours, a rarity's own) is left as it is.
const COLOURS: &[([f32; 3], Take)] = &[
    // Titles, headings, captions: `forecolor` of retail's titles.
    ([0.549, 0.854, 1.0], Take::Token(Token::Holo)),
    // Values and list text: retail's field `forecolor`.
    ([0.615, 0.615, 0.956], Take::Token(Token::Muted)),
    // The option panels' brighter values.
    ([0.93, 0.93, 1.0], Take::Token(Token::Text)),
    // Option labels: `setup.menu` items.
    ([0.65, 0.65, 1.0], Take::Token(Token::Muted)),
    // Buttons, key facts and the description line.
    ([1.0, 0.682, 0.0], Take::Token(Token::Gold)),
    // Frames and rules: `bordercolor`.
    ([0.298, 0.305, 0.690], Take::Dimmed(Token::Holo, 0.45)),
    // List boxes and their borders: dimmed, as a quarter of the bright line-work
    // over a dark ground reads as a salmon slab, not as a box.
    ([0.66, 0.66, 1.0], Take::Dimmed(Token::Holo, 0.55)),
    // The option panel: `setup_background`.
    ([0.0, 0.0, 0.6], Take::Token(Token::GlassLit)),
    // The page's ground: the ink over the map, the browser's window.
    ([0.004, 0.008, 0.02], Take::Token(Token::Space)),
    ([0.0, 0.0, 0.12], Take::Token(Token::Space)),
    // Boxes and windows, darker to lighter: the browser's rows and prompts, an open
    // list, the Force page's and the creation pages' panels.
    ([0.0, 0.0, 0.2], Take::Token(Token::Glass)),
    ([0.0, 0.0, 0.3], Take::Token(Token::Glass)),
    ([0.02, 0.02, 0.12], Take::Token(Token::Glass)),
    ([0.03, 0.04, 0.16], Take::Token(Token::Glass)),
    ([0.0, 0.0, 0.5], Take::Token(Token::GlassLit)),
    ([0.1, 0.1, 0.3], Take::Token(Token::GlassLit)),
    ([0.1, 0.1, 0.5], Take::Token(Token::GlassLit)),
    // Rules, column edges and list fills in retail's deeper blues.
    ([0.0, 0.0, 0.8], Take::Dimmed(Token::Holo, 0.5)),
    ([0.2, 0.2, 0.5], Take::Dimmed(Token::Holo, 0.6)),
    ([0.25, 0.25, 0.8], Take::Dimmed(Token::Holo, 0.45)),
    // Highlights and bands in retail's lighter blues.
    ([0.35, 0.55, 1.0], Take::Token(Token::Holo)),
    ([0.5, 0.5, 1.0], Take::Token(Token::Holo)),
    ([0.55, 0.62, 1.0], Take::Token(Token::Holo)),
    // Quiet text: the main page's "MULTIPLAYER", notes, disabled-looking captions.
    ([0.695, 0.76, 0.861], Take::Token(Token::Muted)),
    ([0.72, 0.73, 0.78], Take::Token(Token::Muted)),
    ([0.75, 0.75, 1.0], Take::Token(Token::Muted)),
    ([0.33, 0.33, 0.5], Take::Token(Token::Quiet)),
    // The browser's hovered column and refresh line, retail's duller gold.
    ([0.79, 0.64, 0.22], Take::Token(Token::Gold)),
];

/// `colour` in the own look: the theme's colour for a retail one, itself otherwise.
fn own(colour: Color) -> Color {
    let near = |a: f32, b: f32| (a - b).abs() < 0.004;
    let Some((_, take)) = COLOURS
        .iter()
        .find(|([r, g, b], _)| near(colour.r, *r) && near(colour.g, *g) && near(colour.b, *b))
    else {
        return colour;
    };
    let live = |token| crate::menu::sjk::theme::LIVE.get(token);
    let themed = match *take {
        Take::Token(token) => live(token),
        Take::Dimmed(token, amount) => {
            let (top, ground) = (live(token), live(Token::Space));
            let mix = |a: f32, b: f32| a + (b - a) * amount;
            Color::new(
                mix(top.r, ground.r),
                mix(top.g, ground.g),
                mix(top.b, ground.b),
                1.0,
            )
        }
    };
    Color::new(themed.r, themed.g, themed.b, colour.a)
}

/// Swap the retail colours of a finished page for the theme's, when the own look is
/// on. The SJK UI's pages name no retail colour, so the pass leaves them as they are.
pub(crate) fn recolour(list: &mut sjk_ui::DrawList) {
    if on() {
        list.recolour(own);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::classic::view::{FOCUS, GOLD};
    use crate::menu::sjk::theme;

    #[test]
    fn the_cvar_reads_retail_unless_it_says_sjk() {
        assert!(from_cvar(Some("sjk")) && from_cvar(Some(" SJK ")));
        assert!(!from_cvar(Some("retail")) && !from_cvar(Some("1")) && !from_cvar(None));
        // Shelved, `sjk` draws retail's look too.
        assert_eq!(chosen(Some("sjk")), !SHELVED);
        assert!(!chosen(Some("retail")) && !chosen(None));
    }

    #[test]
    fn retail_colours_take_the_themes_and_keep_their_alpha() {
        let gold = own(GOLD);
        let live = theme::LIVE.get(Token::Gold);
        assert_eq!((gold.r, gold.g, gold.b), (live.r, live.g, live.b));
        // The description line is retail's gold at 80 %.
        let hint = own(Color::new(1.0, 0.682, 0.0, 0.8));
        assert_eq!((hint.r, hint.a), (live.r, 0.8));
        // What is not retail's is left alone: the white focus, a team's red.
        assert_eq!(own(FOCUS), FOCUS);
        let red = Color::new(0.9, 0.1, 0.1, 1.0);
        assert_eq!(own(red), red);
    }

    #[test]
    fn a_frame_is_dimmer_than_the_line_work_it_comes_from() {
        let frame = own(Color::new(0.298, 0.305, 0.690, 1.0));
        let holo = theme::LIVE.get(Token::Holo);
        assert!(frame.r + frame.g + frame.b < holo.r + holo.g + holo.b);
    }

    #[test]
    fn every_retail_colour_is_listed_once() {
        for (index, (colour, _)) in COLOURS.iter().enumerate() {
            assert!(
                COLOURS[..index].iter().all(|(other, _)| other != colour),
                "{colour:?} twice"
            );
        }
    }
}
