//! The first look (15/11/2026, idea 10 of the Collection's ideas round, Sol's approval):
//! the moment a New thing turns seen (it stayed chosen [`crate::collection_seen::LOOK_MS`],
//! [`crate::collection_seen::Watch`], [`crate::shader_marks::Look`]) is marked. A kyber the
//! stage holds ignites there with its own on-sound (the stage keeps its blade out until
//! then, [`crate::console::collection_panel::StageWish::held_off`]), a card turns over to
//! face the player, and every thing's tier frame flashes once in the tier's colour (none
//! with `ui_rarityEffects 0`). A quiet interface cue goes with it, a longer one for a
//! Legendary, Mythical or Unique thing ([`cue_for`]).
//!
//! Only a thing New when it is seen reveals itself, once: browsing things already seen,
//! Mark all seen (N) and leaving a page stay quiet.

use crate::audio::ui_cues::Cue;
use crate::collection_seen::{Kind, key_of};
use crate::unlockables::Rarity;

/// How long the tier frame's flash lasts, milliseconds.
pub(crate) const FLASH_MS: u64 = 600;
/// How long a card takes to turn over, milliseconds.
pub(crate) const FLIP_MS: u64 = 450;
/// How long the stage keeps lighting a revealed blade (it is out in full well before).
pub(crate) const IGNITE_MS: u64 = 1_200;

/// The thing last revealed: its kind, the ids (a page of finishers is seen at once), its
/// tier and when (milliseconds on the panel's look clock).
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    kind: Kind,
    keys: Vec<u64>,
    tier: Rarity,
    at: u64,
}

/// The reveals the Collection plays: the last one, and how many kyber ignitions the
/// stage was asked for.
#[derive(Debug, Default)]
pub(crate) struct Reveals {
    shown: Option<Shown>,
    ignitions: u32,
}

impl Reveals {
    /// Things `ids` of `kind`, the highest tier `tier`, have just been seen for the
    /// first time at `now`: their reveal starts, and the cue it plays is returned (the
    /// caller posts it). `ignite`: the stage holds this kyber, so it lights.
    pub(crate) fn start(
        &mut self,
        kind: Kind,
        ids: &[&str],
        tier: Rarity,
        now: u64,
        ignite: bool,
    ) -> Cue {
        self.shown = Some(Shown {
            kind,
            keys: ids.iter().map(|id| key_of(kind, id)).collect(),
            tier,
            at: now,
        });
        if ignite {
            self.ignitions = self.ignitions.wrapping_add(1);
        }
        cue_for(tier)
    }

    /// How many ignitions the stage was asked for: a new one lights its blade.
    pub(crate) fn ignitions(&self) -> u32 {
        self.ignitions
    }

    /// Milliseconds since thing `id` of `kind` was revealed, while it is the last one.
    fn since(&self, kind: Kind, id: &str, now: u64) -> Option<u64> {
        let shown = self.shown.as_ref()?;
        (shown.kind == kind && shown.keys.contains(&key_of(kind, id)))
            .then(|| now.saturating_sub(shown.at))
    }

    /// Whether thing `id` of `kind` was revealed (it is the last one): the stage lights
    /// it rather than keeping it out.
    pub(crate) fn revealed(&self, kind: Kind, id: &str) -> bool {
        self.shown
            .as_ref()
            .is_some_and(|shown| shown.kind == kind && shown.keys.contains(&key_of(kind, id)))
    }

    /// How bright the flash over thing `id`'s tier frame is at `now`, 0 to 1; 0 once it
    /// ended, for anything else and with `effects` off (`ui_rarityEffects 0`).
    pub(crate) fn flash(&self, kind: Kind, id: &str, now: u64, effects: bool) -> f32 {
        if !effects {
            return 0.0;
        }
        self.since(kind, id, now)
            .map_or(0.0, |since| flash_at(since as f32 / FLASH_MS as f32))
    }

    /// How far card `id` faces the player at `now` (the card look's `turn`, 0 to 1): it
    /// narrows to its edge and opens again over [`FLIP_MS`]; 1 otherwise. Nothing turns
    /// with `effects` off.
    pub(crate) fn turn(&self, id: &str, now: u64, effects: bool) -> f32 {
        if !effects {
            return 1.0;
        }
        self.since(Kind::Card, id, now)
            .filter(|since| *since < FLIP_MS)
            .map_or(1.0, |since| {
                let t = since as f32 / FLIP_MS as f32;
                (std::f32::consts::PI * t).cos().abs().max(0.02)
            })
    }
}

/// The flash's brightness `t` of the way through it: up quickly, then fading out.
pub(crate) fn flash_at(t: f32) -> f32 {
    if !(0.0..1.0).contains(&t) {
        0.0
    } else if t < 0.15 {
        t / 0.15
    } else {
        let left = (1.0 - t) / 0.85;
        left * left
    }
}

/// The cue a first look at a thing of `tier` plays: a quiet blip, a longer chime for
/// Legendary and above.
pub(crate) fn cue_for(tier: Rarity) -> Cue {
    match tier {
        Rarity::Legendary | Rarity::Mythical | Rarity::Unique => Cue::RevealRare,
        _ => Cue::Reveal,
    }
}

/// The tier of thing `id` of `kind`, for its reveal (Common when unknown).
pub(crate) fn tier_of(kind: Kind, id: &str) -> Rarity {
    match kind {
        Kind::Kyber => crate::unlockables::find(id).map(|skin| skin.tier),
        Kind::Finisher => crate::finishers::get(id).map(|finisher| finisher.tier),
        Kind::Tag => crate::tags::get(id).map(|tag| tag.tier),
        Kind::Card => crate::cards::faces::current()
            .find(id)
            .map(|(_, face)| crate::menu::sjk::kit::card_tier(face.rarity.index)),
    }
    .unwrap_or(Rarity::Common)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reveal belongs to the things it was started for: their flash rises and fades
    /// within its time, a card turns over and faces the player again, and nothing else
    /// flashes; `ui_rarityEffects 0` keeps it all still.
    #[test]
    fn a_reveal_flashes_and_turns_only_its_things_once() {
        let mut reveals = Reveals::default();
        assert_eq!(reveals.flash(Kind::Card, "a", 0, true), 0.0, "nothing yet");
        let cue = reveals.start(Kind::Card, &["a"], Rarity::Rare, 1_000, false);
        assert_eq!(cue, Cue::Reveal);
        assert_eq!(reveals.ignitions(), 0, "a card lights no blade");
        assert!(reveals.flash(Kind::Card, "a", 1_080, true) > 0.8);
        assert!(reveals.flash(Kind::Card, "a", 1_400, true) < 0.3);
        assert_eq!(reveals.flash(Kind::Card, "a", 1_000 + FLASH_MS, true), 0.0);
        assert_eq!(
            reveals.flash(Kind::Card, "b", 1_080, true),
            0.0,
            "another card"
        );
        assert_eq!(
            reveals.flash(Kind::Tag, "a", 1_080, true),
            0.0,
            "another kind"
        );
        assert_eq!(
            reveals.flash(Kind::Card, "a", 1_080, false),
            0.0,
            "effects off"
        );
        // It turns to its edge half way and faces the player again at the end.
        assert!(reveals.turn("a", 1_000 + FLIP_MS / 2, true) < 0.1);
        assert_eq!(reveals.turn("a", 1_000 + FLIP_MS, true), 1.0);
        assert_eq!(reveals.turn("a", 1_000 + FLIP_MS / 2, false), 1.0);
        assert_eq!(reveals.turn("b", 1_000 + FLIP_MS / 2, true), 1.0);
    }

    /// Legendary and above play the longer cue; a kyber on the stage asks for one
    /// ignition each time, and is the one the stage lights.
    #[test]
    fn rare_things_chime_and_a_staged_kyber_ignites() {
        assert_eq!(cue_for(Rarity::Common), Cue::Reveal);
        assert_eq!(cue_for(Rarity::Rare), Cue::Reveal);
        assert_eq!(cue_for(Rarity::Legendary), Cue::RevealRare);
        assert_eq!(cue_for(Rarity::Mythical), Cue::RevealRare);
        assert_eq!(cue_for(Rarity::Unique), Cue::RevealRare);
        let mut reveals = Reveals::default();
        reveals.start(Kind::Kyber, &["saber_sun"], Rarity::Mythical, 0, true);
        assert_eq!(reveals.ignitions(), 1);
        assert!(reveals.revealed(Kind::Kyber, "saber_sun"));
        assert!(!reveals.revealed(Kind::Kyber, "saber_void"));
        reveals.start(Kind::Kyber, &["saber_void"], Rarity::Rare, 10, false);
        assert_eq!(reveals.ignitions(), 1, "not on the stage: no ignition");
        assert!(
            !reveals.revealed(Kind::Kyber, "saber_sun"),
            "the last one only"
        );
    }

    #[test]
    fn the_flash_rises_then_fades() {
        assert_eq!(flash_at(-0.1), 0.0);
        assert_eq!(flash_at(0.0), 0.0);
        assert_eq!(flash_at(0.15), 1.0);
        assert!(flash_at(0.5) > flash_at(0.8));
        assert_eq!(flash_at(1.0), 0.0);
    }
}
