//! The way between the hall and a kind (15/11/2026, idea 9 of the Collection's ideas
//! round, Sol's approval): Enter on a tile grows the tile into the page, the page fading
//! in over the last half; Escape (or the breadcrumb) shrinks the page back onto the same
//! tile while the hall's other tiles fade in; Ctrl+Tab between kinds slides the page in
//! sideways, to say "next kind" rather than "deeper". With `ui_rarityEffects 0` every
//! change is a short crossfade instead.
//!
//! It only draws: the change of page has already happened when it starts, so a key acts
//! at once, during a motion as at any other time. It follows what the panel shows each
//! frame ([`Transition::follow`]), so every way in and out (keys, clicks, commands) gets
//! the same motion without being told.

use crate::profile_hub::Tab;
use sjk_ui::Rect;

/// A tile growing into its page, or the page shrinking back onto it, milliseconds.
pub(crate) const OPEN_MS: u64 = 180;
/// The slide from one kind to the next.
pub(crate) const SLIDE_MS: u64 = 100;
/// The crossfade in place of either with `ui_rarityEffects 0`.
pub(crate) const FADE_MS: u64 = 120;
/// How far a slide starts from its place, frame pixels.
pub(crate) const SLIDE_FROM: f32 = 60.0;

/// What the Collection shows: the hall, or a kind (the Finishers pill a kind of its own).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Place {
    pub(crate) hall: bool,
    pub(crate) tab: Tab,
    pub(crate) finishers: bool,
}

/// The motion playing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Motion {
    /// From the hall's tile `from` (window pixels) into the page.
    Open { from: Rect },
    /// From the page back onto the hall's chosen tile.
    Close,
    /// To the next kind (`by` 1) or the one before (-1).
    Slide { by: f32 },
    /// A crossfade.
    Fade,
}

impl Motion {
    const fn length(self) -> u64 {
        match self {
            Self::Open { .. } | Self::Close => OPEN_MS,
            Self::Slide { .. } => SLIDE_MS,
            Self::Fade => FADE_MS,
        }
    }
}

/// The motion between the Collection's pages, and what it starts from.
#[derive(Debug, Default)]
pub(crate) struct Transition {
    last: Option<Place>,
    playing: Option<(Motion, u64)>,
    /// The hall's chosen tile on its last frame (window pixels): where an opening grows
    /// from and a closing lands.
    pub(crate) tile: Option<Rect>,
    /// This frame's motion and how far through it ([`Self::at`]), for the drawing.
    pub(crate) now: Option<(Motion, f32)>,
    /// Hold every motion this many milliseconds in, for a world shot.
    #[cfg(test)]
    pub(crate) hold: Option<u64>,
}

impl Transition {
    /// Forget the page shown, so that opening the Collection plays nothing.
    pub(crate) fn reset(&mut self) {
        self.last = None;
        self.playing = None;
        self.now = None;
    }

    /// The panel shows `place` at `now` (milliseconds): a change since the last frame
    /// starts its motion (a crossfade when `effects` are off).
    pub(crate) fn follow(&mut self, place: Place, now: u64, effects: bool) {
        let Some(last) = self.last.replace(place) else {
            return;
        };
        if last == place {
            return;
        }
        let motion = match (last.hall, place.hall) {
            _ if !effects => Motion::Fade,
            (true, false) => self.tile.map_or(Motion::Fade, |from| Motion::Open { from }),
            (false, true) => Motion::Close,
            (false, false) => Motion::Slide {
                by: if order(place) >= order(last) {
                    1.0
                } else {
                    -1.0
                },
            },
            (true, true) => return,
        };
        self.playing = Some((motion, now));
    }

    /// The motion playing at `now` and how far through it, eased out (0 to 1); `None`
    /// once it ended.
    pub(crate) fn at(&mut self, now: u64) -> Option<(Motion, f32)> {
        let (motion, start) = self.playing?;
        let since = now.saturating_sub(start);
        #[cfg(test)]
        let since = self.hold.unwrap_or(since);
        if since >= motion.length() {
            self.playing = None;
            return None;
        }
        Some((motion, ease_out(since as f32 / motion.length() as f32)))
    }
}

/// A kind's place in the order Ctrl+Tab goes through (Finishers after Kybers).
fn order(place: Place) -> f32 {
    place.tab.index() as f32 + if place.finishers { 0.5 } else { 0.0 }
}

/// `t` eased out (cubic): fast at first, settling at the end.
pub(crate) fn ease_out(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - left * left * left
}

/// The rectangle `t` of the way from `from` to `to`.
pub(crate) fn between(from: Rect, to: Rect, t: f32) -> Rect {
    let mix = |a: f32, b: f32| a + (b - a) * t;
    Rect::new(
        mix(from.x, to.x),
        mix(from.y, to.y),
        mix(from.width, to.width),
        mix(from.height, to.height),
    )
}

/// How opaque the page arriving is `t` (eased) into `motion`: an opening's page comes in
/// over the last half, the rest at once from faint.
pub(crate) fn arriving(motion: Motion, t: f32) -> f32 {
    match motion {
        Motion::Open { .. } => ((t - 0.5) * 2.0).clamp(0.0, 1.0),
        Motion::Close => t,
        Motion::Slide { .. } => 0.4 + 0.6 * t,
        Motion::Fade => t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALL: Place = Place {
        hall: true,
        tab: Tab::Shaders,
        finishers: false,
    };
    const fn kind(tab: Tab) -> Place {
        Place {
            hall: false,
            tab,
            finishers: false,
        }
    }

    /// Entering a tile opens from it over 180 ms, eased out; Escape plays it backward;
    /// Ctrl+Tab slides over 100 ms in its direction; the first page shown plays nothing.
    #[test]
    fn each_change_of_page_plays_its_motion_for_its_time() {
        let tile = Rect::new(100.0, 200.0, 300.0, 250.0);
        let mut transition = Transition {
            tile: Some(tile),
            ..Transition::default()
        };
        transition.follow(HALL, 0, true);
        assert_eq!(
            transition.at(0),
            None,
            "opening the Collection plays nothing"
        );
        transition.follow(kind(Tab::Cards), 1_000, true);
        let (motion, t) = transition.at(1_090).expect("half way");
        assert_eq!(motion, Motion::Open { from: tile });
        assert!(
            t > 0.5 && t < 1.0,
            "eased out: past half at half time ({t})"
        );
        assert!(arriving(motion, ease_out(0.2)) < 0.01, "the page waits");
        assert_eq!(transition.at(1_000 + OPEN_MS), None, "done at 180 ms");
        transition.follow(kind(Tab::Inventory), 2_000, true);
        assert_eq!(
            transition.at(2_050).map(|(motion, _)| motion),
            Some(Motion::Slide { by: 1.0 })
        );
        assert_eq!(transition.at(2_000 + SLIDE_MS), None, "a slide is 100 ms");
        transition.follow(kind(Tab::Cards), 3_000, true);
        assert_eq!(
            transition.at(3_010).map(|(motion, _)| motion),
            Some(Motion::Slide { by: -1.0 })
        );
        transition.follow(HALL, 4_000, true);
        assert_eq!(transition.at(4_010).map(|(m, _)| m), Some(Motion::Close));
        // Effects off: a crossfade, whatever the change.
        transition.follow(kind(Tab::Medals), 5_000, false);
        assert_eq!(transition.at(5_010).map(|(m, _)| m), Some(Motion::Fade));
        assert_eq!(transition.at(5_000 + FADE_MS), None);
        transition.reset();
        transition.follow(kind(Tab::Cards), 6_000, true);
        assert_eq!(transition.at(6_010), None, "reopened: nothing");
    }

    #[test]
    fn a_rect_eases_from_one_to_the_other() {
        let from = Rect::new(0.0, 0.0, 10.0, 10.0);
        let to = Rect::new(100.0, 50.0, 200.0, 100.0);
        assert_eq!(between(from, to, 0.0), from);
        assert_eq!(between(from, to, 1.0), to);
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.5) > 0.5);
    }
}
