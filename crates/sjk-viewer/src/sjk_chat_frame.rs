//! The per-frame hook of the SJK chat (`docs/hub-chat.md`): when the identity
//! service's chat changed, bring the game's chat feed in line with it (new messages,
//! deleted ones, why a message was refused), and play the SJK chat's sound for a new
//! message from another player (`audio/sjk_chat_sound.rs`). Each frame costs one look
//! at the chat's revision and the mutes'; nothing is copied unless one changed.

use super::*;

impl GpuState {
    /// Follow the SJK chat in the chat feed while `cl_sjkChat` is on.
    pub(crate) fn update_sjk_chat(&mut self) {
        let Some(console) = self.console.as_ref() else {
            return;
        };
        if console.bool_cvar("cl_sjkChat") == Some(false) {
            return;
        }
        let sound = sound_on(console);
        let mutes = player_mutes::revision();
        let Some(mark) = player_identity::with_chat(|chat| {
            (
                chat.revision,
                chat.outcome.as_ref().map_or(0, |outcome| outcome.serial),
                mutes,
            )
        }) else {
            return;
        };
        if !self.chat.sjk_changed(mark) {
            return;
        }
        // The mutes live under the service's lock too: read them before it is taken.
        let muted = player_mutes::muted_keys();
        let is_muted = |key: &str| muted.iter().any(|m| m == key);
        // The player's own keys, read before the chat's lock as the mutes are.
        let own = if sound { own_keys() } else { Vec::new() };
        let now = Instant::now();
        let chat = &mut self.chat;
        let news = player_identity::with_chat(|state| {
            let after = chat.sync_sjk(state, is_muted, now);
            chat.sjk_outcome(state.outcome.as_ref(), now);
            after.is_some_and(|after| {
                chat::sjk::new_from_others(
                    &state.messages,
                    after,
                    |key| own.iter().any(|o| o == key),
                    is_muted,
                )
            })
        });
        if sound && news == Some(true) && self.chat.sjk_sound_due(now) {
            audio::ui_cues::post(audio::ui_cues::Cue::SjkChat);
        }
    }
}

/// Whether the SJK chat's sound plays: `cl_sjkChatSound`, under the master chat sound
/// switch `cg_chatSounds` (`cgame_options.rs`), which silences every chat sound.
fn sound_on(console: &console::ViewerConsole) -> bool {
    console.bool_cvar(SOUND_CVAR) != Some(false) && cgame_options::chat_sounds(Some(console))
}

/// `cl_sjkChatSound`: whether a new SJK chat message plays a sound.
pub(crate) const SOUND_CVAR: &str = "cl_sjkChatSound";

/// Every key id that is the player's: their id at the hub, this PC's key and the keys
/// linked to their person, so their own messages from any of them play no sound.
fn own_keys() -> Vec<String> {
    player_identity::with_snapshot(|snapshot| {
        let mut keys = vec![snapshot.key_id.clone(), snapshot.local_key_id.clone()];
        if let Some(me) = &snapshot.me {
            keys.extend(me.keys.iter().cloned());
        }
        keys
    })
    .unwrap_or_default()
}
