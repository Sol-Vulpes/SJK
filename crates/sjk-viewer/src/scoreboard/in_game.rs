//! Who is in the game though the server's `scores` leave them out.
//!
//! A `scores` command lists at most 20 clients (`MAX_CLIENT_SCORE_SEND`,
//! `DeathmatchScoreboardMessage` in `codemp/game/g_cmds.c`), the best first. With more
//! players than that, the rest have no row, and SJK's board drew every connected client
//! without one as "Joining": on a full server the bottom players, and the viewer himself
//! when he was among them, stayed "Joining" for good. A client is known to be in the game,
//! and shown as a player whose score is not known, when it is
//!
//! * the viewer: this client receives snapshots, so the server has let it in;
//! * a bot (`skill` in its clientinfo), which a server lets in at once;
//! * seen since it connected: its player entity in a snapshot, or a `scores` row with a
//!   ping (a connecting client's row has a ping of -1, `CG_ParseScores`), until its slot
//!   empties, a row says it is connecting again or a new map starts.
//!
//! Anyone else without a row is still joining, as before.

use sjk_client::ScoreEntry;
use sjk_protocol::{GameState, Snapshot};

/// `ET_PLAYER` (`bg_public.h`).
const ET_PLAYER: u8 = 1;

/// The clients seen in the game since they connected, one bit each.
#[derive(Default)]
pub(super) struct InGame {
    seen: u64,
    time: i32,
}

impl InGame {
    /// Read a frame: the snapshot's players, the scores and the roster.
    pub(super) fn observe(&mut self, snapshot: &Snapshot, game: &GameState, scores: &[ScoreEntry]) {
        if snapshot.server_time < self.time {
            // A new map's clock, or a demo rewound: everyone connects again.
            self.seen = 0;
        }
        self.time = snapshot.server_time;
        for state in &snapshot.entities {
            if state.entity_type() == ET_PLAYER {
                self.seen |= bit(usize::from(state.number()));
            }
        }
        for score in scores {
            let bit = bit(usize::from(score.client_num));
            if score.ping < 0 {
                self.seen &= !bit;
            } else {
                self.seen |= bit;
            }
        }
        for client in 0..sjk_protocol::PLAYER_CAPACITY {
            let connected = crate::player_info::player_info(game, client)
                .is_some_and(|bytes| !bytes.is_empty());
            if !connected {
                self.seen &= !bit(client);
            }
        }
    }

    /// The clients seen in the game, one bit each.
    pub(super) fn seen(&self) -> u64 {
        self.seen
    }
}

/// Bit `client` of a 64-slot mask; none past the last slot (an entity that is no player).
fn bit(client: usize) -> u64 {
    if sjk_protocol::is_player_number(client) {
        1_u64.checked_shl(client as u32).unwrap_or(0)
    } else {
        0
    }
}

/// Whether `client`, connected but without a `scores` row, is in the game (shown as a
/// player whose score is not known) rather than still joining.
pub(super) fn in_game(client: u8, local: Option<u8>, bot: bool, seen: u64) -> bool {
    local == Some(client) || bot || seen & 1_u64.checked_shl(u32::from(client)).unwrap_or(0) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS, PlayerState};

    fn row(client_num: u8, ping: i32) -> ScoreEntry {
        ScoreEntry {
            client_num,
            score: 0,
            ping,
            time_minutes: 0,
            flags: 0,
            powerups: 0,
            defends: 0,
            assists: 0,
            captures: 0,
            deaths: None,
        }
    }

    fn game(clients: &[usize]) -> GameState {
        let mut game = GameState::empty_local(0);
        for &client in clients {
            game.replace_config_string(
                sjk_protocol::CS_PLAYERS + client,
                format!("n\\P{client}\\t\\0").into_bytes(),
            )
            .unwrap();
        }
        game
    }

    #[test]
    fn the_viewer_and_bots_are_in_the_game_without_a_row_and_others_join() {
        assert!(in_game(5, Some(5), false, 0), "the viewer");
        assert!(in_game(7, Some(5), true, 0), "a bot");
        assert!(!in_game(7, Some(5), false, 0), "a human never seen");
        assert!(in_game(7, Some(5), false, 1 << 7), "a human seen");
        assert!(in_game(63, None, false, 1 << 63), "the last of 64 slots");
    }

    #[test]
    fn a_player_seen_in_a_snapshot_or_a_row_stays_in_until_its_slot_empties() {
        let mut watch = InGame::default();
        let mut game = game(&[2, 3, 4]);
        let mut player = EntityState::zero(2, &LEGACY_ENTITY_FIELDS);
        player.set_raw_field(8, u32::from(ET_PLAYER));
        let mut snapshot = Snapshot {
            message_sequence: 1,
            reliable_acknowledge: 0,
            server_commands: Vec::new(),
            server_time: 1_000,
            delta_from: None,
            flags: 0,
            area_mask: Vec::new(),
            player: PlayerState::zero(),
            vehicle_player: None,
            entities: vec![player],
            consumed_bits: 0,
        };
        watch.observe(&snapshot, &game, &[row(3, 50), row(4, -1)]);
        assert_eq!(
            watch.seen(),
            1 << 2 | 1 << 3,
            "seen, listed; 4 still connecting"
        );
        // Out of view and off the list, it is still in the game.
        snapshot.entities.clear();
        snapshot.server_time = 1_050;
        watch.observe(&snapshot, &game, &[]);
        assert_eq!(watch.seen(), 1 << 2 | 1 << 3);
        // Its slot empties: whoever comes next is joining until seen.
        game.replace_config_string(sjk_protocol::CS_PLAYERS + 2, Vec::new())
            .unwrap();
        watch.observe(&snapshot, &game, &[row(3, -1)]);
        assert_eq!(watch.seen(), 0, "2 left, 3 connects again");
        watch.observe(&snapshot, &game, &[row(3, 40)]);
        snapshot.server_time = 10;
        watch.observe(&snapshot, &game, &[]);
        assert_eq!(watch.seen(), 0, "a new map: everyone connects again");
    }
}
