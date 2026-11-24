//! The sounds a local mute silences (`docs/hub-chat.md`, "Muting a player"): every
//! sound whose source is a muted player's entity (footsteps, jumps, pain, taunts, weapon
//! fire, saber swings and hum, Force), every sound the event names them as the cause of
//! ([`sjk_client::LegacySoundDecision::cause`]: a saber hit or block they made, a voice
//! command they gave, the chat beep of their message) and the hum of a saber they
//! threw. The test is a bit test over entity numbers, rebuilt once a snapshot and only
//! while someone is muted, so it costs nothing otherwise.

use sjk_audio::SourceId;
use sjk_protocol::EntityState;

/// Entity numbers the legacy protocol uses.
const ENTITIES: usize = 1_024;
/// Game slots the muted mask holds, one bit each.
const CLIENTS: usize = u64::BITS as usize;

/// Which sound sources are silenced.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AudioMute {
    /// Muted client slots, as bits.
    clients: u64,
    /// Silenced entity numbers, as bits: the muted clients' own and their thrown sabers.
    entities: [u32; ENTITIES / 32],
}

impl Default for AudioMute {
    fn default() -> Self {
        Self {
            clients: 0,
            entities: [0; ENTITIES / 32],
        }
    }
}

impl AudioMute {
    /// The muted slots changed (or not: this is called every frame).
    pub(crate) fn set_clients(&mut self, clients: u64) {
        if self.clients == clients {
            return;
        }
        self.clients = clients;
        self.entities = self.own_entities();
    }

    /// Note the entities the muted players own among a snapshot's `entities` (their
    /// thrown sabers).
    pub(crate) fn observe(&mut self, entities: &[EntityState]) {
        if self.clients == 0 {
            return;
        }
        self.entities = self.own_entities();
        for state in entities {
            let number = usize::from(state.number());
            if (sjk_protocol::player_slots()..ENTITIES).contains(&number)
                && sjk_client::legacy_thrown_saber_owner(state)
                    .is_some_and(|owner| self.client(owner))
            {
                self.entities[number / 32] |= 1 << (number % 32);
            }
        }
    }

    /// The muted clients' own entities (a client's entity number is its slot).
    fn own_entities(&self) -> [u32; ENTITIES / 32] {
        let mut entities = [0; ENTITIES / 32];
        entities[0] = self.clients as u32;
        entities[1] = (self.clients >> 32) as u32;
        entities
    }

    fn client(&self, client: u16) -> bool {
        usize::from(client) < CLIENTS && self.clients & (1 << client) != 0
    }

    /// Whether a sound from `source`, caused by `cause`, is silenced.
    pub(crate) fn silences(&self, source: SourceId, cause: Option<u16>) -> bool {
        if self.clients == 0 {
            return false;
        }
        let number = source.0 as usize;
        (number < ENTITIES && self.entities[number / 32] & (1 << (number % 32)) != 0)
            || cause.is_some_and(|cause| self.client(cause))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS};

    /// A saber thrown by `owner`, as `w_saber.c` marks it (`genericenemyindex`).
    fn thrown_saber(number: u16, owner: u16) -> EntityState {
        let mut state = EntityState::zero(number, &LEGACY_ENTITY_FIELDS);
        state.set_raw_field(14, 3);
        state.set_raw_field(18, 1_024 + u32::from(owner));
        state
    }

    #[test]
    fn a_muted_players_sounds_are_silenced_and_no_one_elses() {
        let mut mute = AudioMute::default();
        assert!(!mute.silences(SourceId(4), Some(4)), "nobody muted");
        mute.set_clients(1 << 4);
        // Their entity: footsteps, voice, weapon fire, swings, hum.
        assert!(mute.silences(SourceId(4), Some(4)));
        assert!(mute.silences(SourceId(4), None));
        // A saber hit or block they made, a voice command they gave, the beep of their
        // chat: another source, but they are the cause.
        assert!(mute.silences(SourceId(300), Some(4)));
        assert!(mute.silences(SourceId(0), Some(4)));
        // Everyone else, and what nobody is named as the cause of.
        assert!(!mute.silences(SourceId(5), Some(5)));
        assert!(!mute.silences(SourceId(300), None));
        assert!(!mute.silences(SourceId(300), Some(7)));
        // A source outside the table.
        assert!(!mute.silences(SourceId(5_000), None));
    }

    /// A server with more than 32 player slots: a player past slot 31 is silenced, and
    /// nobody else is.
    #[test]
    fn a_muted_player_past_slot_31_is_silenced() {
        let mut mute = AudioMute::default();
        mute.set_clients(1 << 40);
        assert!(mute.silences(SourceId(40), None));
        assert!(mute.silences(SourceId(300), Some(40)));
        assert!(!mute.silences(SourceId(8), Some(8)), "slot 40, not 8");
        assert!(!mute.silences(SourceId(41), Some(41)));
    }

    #[test]
    fn a_saber_they_throw_is_silent_too() {
        let mut mute = AudioMute::default();
        mute.set_clients(1 << 2);
        mute.observe(&[thrown_saber(200, 2), thrown_saber(201, 3)]);
        assert!(mute.silences(SourceId(200), None));
        assert!(
            !mute.silences(SourceId(201), None),
            "another player's saber"
        );
        assert!(mute.silences(SourceId(2), None), "the player still");
        mute.set_clients(0);
        assert!(!mute.silences(SourceId(200), None));
    }
}
