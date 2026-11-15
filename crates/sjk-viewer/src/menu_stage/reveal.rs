//! The Collection's first look at a New kyber on the stage (`collection_reveal.rs`): while
//! the saber on its own holds a New kyber not looked at yet its blade stays out, the hilt
//! alone; the first look lights it, the blade running hilt to tip as a match's does, with
//! the kyber's own ignition sound (the hilt's, else the stock one, as the Saber Lab's).

use super::*;
use crate::GameAudio;
use crate::console::collection_panel::{StageWish, reveal::IGNITE_MS};

/// Where the stage's first look stands.
#[derive(Debug, Default)]
pub(super) struct Reveal {
    /// The blade stays out: a New kyber waits for its first look.
    held_off: bool,
    /// The ignitions the page asked for, as last seen (`None` before the page was seen,
    /// so opening it lights nothing).
    seen: Option<u32>,
    /// Until when (the stage's milliseconds) the blade is being lit.
    lighting_until: Option<i64>,
    /// An ignition waits to be heard.
    sound: bool,
}

impl Reveal {
    /// Follow the page's `wish` this frame (`None`: the page is not showing).
    pub(super) fn follow(&mut self, wish: Option<&StageWish>) {
        let Some(wish) = wish.filter(|wish| wish.saber.is_some() && wish.lab.is_none()) else {
            self.held_off = false;
            self.lighting_until = None;
            return;
        };
        self.held_off = wish.held_off;
        if self.seen.is_some_and(|seen| seen != wish.ignite) {
            // Lit from the next frame the sabers are drawn ([`Self::lit`]).
            self.lighting_until = Some(i64::MIN);
            self.sound = true;
        }
        self.seen = Some(wish.ignite);
    }

    /// Whether the saber on its own is lit at `millis` (the stage's clock): `Some(false)`
    /// while held off, `Some(true)` while a first look lights it, `None` (out in full)
    /// otherwise.
    pub(super) fn lit(&mut self, millis: i64) -> Option<bool> {
        if self.held_off {
            return Some(false);
        }
        let until = self.lighting_until?;
        let until = if until == i64::MIN {
            let until = millis + IGNITE_MS as i64;
            self.lighting_until = Some(until);
            until
        } else {
            until
        };
        if millis < until {
            Some(true)
        } else {
            self.lighting_until = None;
            None
        }
    }

    /// Take the ignition waiting to be heard.
    fn take_sound(&mut self) -> bool {
        std::mem::take(&mut self.sound)
    }
}

impl GpuState {
    /// Play the first look's ignition on the stage, once: the kyber's own sound, else the
    /// hilt's, else the stock one.
    pub(crate) fn play_stage_reveal(&mut self, audio: &mut Option<GameAudio>) {
        if !self.menu_stage.reveal.take_sound() {
            return;
        }
        let (Some(audio), Some(vfs)) = (audio.as_mut(), self.vfs.clone()) else {
            return;
        };
        let skin = self.stage_skins()[0].map(|skin| skin.index);
        let skins = std::sync::Arc::clone(&self.blade_skins);
        let own = skin
            .and_then(|index| skins.sounds(index))
            .map(|sounds| sounds.on.clone());
        let hilt = self.menu_stage.request.hilt(0).and_then(|name| {
            self.menu_stage
                .saber_definitions
                .as_ref()?
                .get(&name.to_ascii_lowercase())?
                .sound_on
                .clone()
        });
        let found = [own, hilt, Some(lab::STOCK_ON.to_owned())]
            .into_iter()
            .flatten()
            .find(|path| !path.is_empty() && audio.register_sound(&vfs, path));
        if let Some(path) = found {
            audio.play_local(&path, 1.0, lab::SOURCE, lab::SWITCH_CHANNEL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::collection_panel::{PreviewSkin, SaberTurn};

    fn wish(held_off: bool, ignite: u32) -> StageWish {
        StageWish {
            skin: PreviewSkin::Skin("saber_sun"),
            saber: Some(SaberTurn::default()),
            on_second: false,
            on_both: false,
            holocron: false,
            lab: None,
            hall: false,
            held_off,
            ignite,
        }
    }

    /// A New kyber's blade stays out until its first look, which lights it once with a
    /// sound; opening the page on a count already there lights nothing.
    #[test]
    fn the_first_look_lights_the_held_blade_once() {
        let mut reveal = Reveal::default();
        reveal.follow(Some(&wish(true, 3)));
        assert_eq!(reveal.lit(0), Some(false), "held off");
        assert!(!reveal.take_sound(), "opening the page plays nothing");
        reveal.follow(Some(&wish(false, 4)));
        assert_eq!(reveal.lit(100), Some(true), "lighting");
        assert!(reveal.take_sound());
        assert!(!reveal.take_sound(), "heard once");
        assert_eq!(reveal.lit(100 + IGNITE_MS as i64 - 1), Some(true));
        assert_eq!(reveal.lit(100 + IGNITE_MS as i64), None, "out in full");
        reveal.follow(Some(&wish(false, 4)));
        assert_eq!(reveal.lit(5_000), None, "no new look, nothing lit");
        // Leaving the page drops a held blade.
        reveal.follow(Some(&wish(true, 4)));
        reveal.follow(None);
        assert_eq!(reveal.lit(6_000), None);
    }
}
