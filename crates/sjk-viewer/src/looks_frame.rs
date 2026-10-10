//! The per-frame hook of looks (`looks.rs`): other players' looks from the SJK hub's
//! roster and feed, the local player's own from its settings, and the own look handed
//! to the identity service when it changes. A frame costs the feed's queue and the
//! roster's revision (two uncontended locks each) and the own Illuminate; the names
//! are compared and the table rebuilt only when a roster or an event comes, the own
//! look changes, or twice a second.

use super::*;
use crate::looks::Worn;

impl GpuState {
    /// Bring the looks in line with the hub and the settings; `due` is the identity's
    /// twice-a-second turn, when the own blade skin is read again and given to the
    /// service.
    pub(crate) fn update_looks(&mut self, due: bool) {
        let session = self.live_session.as_ref();
        let server = session
            .filter(|session| !session.is_local())
            .map(|session| session.server());
        // Only the looks read for this server under the feed's current reading.
        let received = player_identity::take_looks(server);
        let mut changed = due;
        if self.looks.server != server {
            self.looks.clear(server);
            self.illuminate_others.clear();
            changed = true;
        }
        changed |= self.looks.follow_feed(received.generation);
        if server.is_some() {
            let revision = player_identity::revision();
            if self.looks.roster_revision != Some(revision) {
                self.looks.roster_revision = Some(revision);
                let looks = &mut self.looks;
                player_identity::with_roster(|players| {
                    looks.replace_roster(players.iter().map(|player| {
                        (
                            player.slot,
                            player.claimed_name.as_str(),
                            player.look.as_ref(),
                        )
                    }));
                });
                changed = true;
            }
            for event in &received.events {
                self.looks
                    .apply_event(event.slot, &event.claimed_name, &event.look());
                changed = true;
            }
        }
        let own_slot =
            session.and_then(|session| u8::try_from(session.game_state().client_num).ok());
        let lit = self.illuminate.lit();
        let saber_skin = if due {
            let setting = self
                .console
                .as_ref()
                .and_then(|console| console.text_cvar(crate::unlockables::SABER_SKIN_CVAR).ok())
                .unwrap_or_default();
            #[cfg(test)]
            let shot = self.looks.shot_owns_unlocks;
            #[cfg(not(test))]
            let shot = false;
            Worn::own(setting, |id| shot || player_identity::owns_unlock(id), lit).saber_skin
        } else {
            self.looks.own().saber_skin
        };
        let own = Worn {
            saber_skin,
            illuminate: lit,
        };
        if due {
            player_identity::set_look(&own.to_look());
            // Who sits where, and the own name, for the saber shaders drawn from them.
            let slots = session.map_or_else(
                || [crate::saber_persona::Persona::default(); crate::saber_skins::MAX_CLIENTS],
                |session| crate::saber_persona::Persona::of_slots(session.game_state()),
            );
            let name = self
                .console
                .as_ref()
                .and_then(|console| console.text_cvar("name").ok())
                .unwrap_or_default();
            self.saber_skins
                .set_personas(slots, crate::saber_persona::Persona::of(name.as_bytes(), 0));
        }
        if own != self.looks.own() || own_slot != self.looks.own_slot() {
            self.looks.set_own(own_slot, own);
        }
        if changed {
            let game_state = session.map(|session| session.game_state());
            self.looks.rebuild(|slot| {
                game_state.and_then(|state| player_identity::shown_name(state, usize::from(slot)))
            });
        }
    }
}
