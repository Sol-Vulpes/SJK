//! The per-frame hook of the SJK chat (`docs/hub-chat.md`): when the identity
//! service's chat changed, bring the game's chat feed in line with it (new messages,
//! deleted ones, why a message was refused). Each frame costs one look at the chat's
//! revision; nothing is copied unless it changed.

use super::*;

impl GpuState {
    /// Follow the SJK chat in the chat feed while `cl_sjkChat` is on.
    pub(crate) fn update_sjk_chat(&mut self) {
        let on = self
            .console
            .as_ref()
            .is_some_and(|console| console.bool_cvar("cl_sjkChat") != Some(false));
        if !on {
            return;
        }
        let Some(mark) = player_identity::with_chat(|chat| {
            (
                chat.revision,
                chat.outcome.as_ref().map_or(0, |outcome| outcome.serial),
            )
        }) else {
            return;
        };
        if !self.chat.sjk_changed(mark) {
            return;
        }
        // The mutes live under the service's lock too: read them before it is taken.
        let muted = player_identity::muted_keys();
        let now = Instant::now();
        let chat = &mut self.chat;
        player_identity::with_chat(|state| {
            chat.sync_sjk(&state.messages, |key| muted.iter().any(|m| m == key), now);
            chat.sjk_outcome(state.outcome.as_ref(), now);
        });
    }
}
