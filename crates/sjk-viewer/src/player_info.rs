//! A player's info configstring, wherever the connected server keeps it.
//!
//! Players 0 to 31 have theirs at `CS_PLAYERS + client`; on servers with more than 32
//! player slots the others have theirs in a second range
//! ([`sjk_protocol::player_configstring`]). Code that reads a player's info goes through
//! [`player_info`] rather than adding a client number to `CS_PLAYERS`.

use sjk_protocol::GameState;

/// Player `client`'s info string, or `None` when the slot is empty or past the
/// connected server's player slots.
pub(crate) fn player_info(game: &GameState, client: usize) -> Option<&[u8]> {
    game.config_string(sjk_protocol::player_configstring(client)?)
}
