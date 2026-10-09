//! The per-frame hook of player identity (`player_identity.rs`): twice a second,
//! hand the service the player's settings and where they are playing, the new
//! medal pop-up the medals of the player's own profile (`medal_popup.rs`), and the
//! achievements their counts (`achievements_frame.rs`). Each frame it also lets the
//! chat feed follow the SJK chat (`sjk_chat_frame.rs`), starts and ends emotes
//! (`emotes_frame.rs`), keeps the players' looks (`looks_frame.rs`) and follows the
//! players muted on this PC (`muted_players_frame.rs`).

use super::*;
use sjk_identity::Settings;

impl GpuState {
    /// Tell the identity service about the settings and the live session. Does
    /// nothing between its twice-a-second turns.
    pub(crate) fn update_identity(&mut self) {
        self.update_sjk_chat();
        self.update_emotes();
        self.update_muted_players();
        let due = player_identity::due();
        self.update_looks(due);
        if !due {
            return;
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let settings = Settings {
            enabled: console.bool_cvar("cl_identity") == Some(true),
            hub_url: console
                .text_cvar("cl_hubUrl")
                .unwrap_or_default()
                .trim()
                .to_owned(),
        };
        let name = console
            .text_cvar("name")
            .unwrap_or_default()
            .trim()
            .to_owned();
        let location = self.live_session.as_ref().and_then(|session| {
            player_identity::location(session.server(), session.is_local(), session.game_state())
        });
        let chat = console.bool_cvar("cl_sjkChat") != Some(false);
        player_identity::apply(console.config_directory(), settings, name, location, chat);
        let (packs_revision, assets_note) = player_identity::packs();
        crate::sjk_packs::follow_identity(packs_revision, assets_note);
        self.offer_medals();
        self.update_achievements();
    }
}
