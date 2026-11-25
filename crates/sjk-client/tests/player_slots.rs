//! Players 32 to 63 of a server with more than 32 player slots, and their absence on a
//! classic one. These tests change the process-wide slot count, so they live in their
//! own test binary: the unit tests, which run beside each other, always see 32.

use sjk_client::{
    ChatDestination, ChatRoster, chat_command, obituary_name, parse_restore_client_ghoul,
};
use sjk_protocol::{CS_PLAYERS, CS_PLAYERS_HIGH, GameState, PlayerSlotsForTest};

/// Player 5 in the classic range and player 40 in the high one.
fn two_players() -> GameState {
    let mut game = GameState::empty_local(0);
    game.replace_config_string(CS_PLAYERS + 5, b"n\\Low\\t\\0".to_vec())
        .unwrap();
    game.replace_config_string(CS_PLAYERS_HIGH + 8, b"n\\High\\t\\0".to_vec())
        .unwrap();
    game
}

fn roster_name(game: &GameState, slot: u16) -> Option<String> {
    let mut roster = ChatRoster::default();
    roster.update(game);
    let target = roster.target(Some(slot))?;
    roster.display_name(target).map(str::to_owned)
}

#[test]
fn a_player_past_32_is_named_from_the_high_configstrings() {
    let _slots = PlayerSlotsForTest::new(64);
    let game = two_players();
    assert_eq!(obituary_name(&game, 5), "Low");
    assert_eq!(obituary_name(&game, 40), "High");
    assert_eq!(roster_name(&game, 5).as_deref(), Some("Low"));
    assert_eq!(roster_name(&game, 40).as_deref(), Some("High"));
    // Entity 32 is a player there, not the string that follows player 31's.
    assert_eq!(obituary_name(&game, 32), "noname");
}

#[test]
fn a_classic_server_has_no_player_past_32() {
    let _slots = PlayerSlotsForTest::new(32);
    let game = two_players();
    assert_eq!(obituary_name(&game, 5), "Low");
    assert_eq!(obituary_name(&game, 40), "noname");
    assert_eq!(roster_name(&game, 40), None);
}

#[test]
fn client_numbers_follow_the_servers_slots() {
    let restore = |client: &str| {
        parse_restore_client_ghoul(b"rcg", &[b"rcg".to_vec(), client.as_bytes().to_vec()])
            .map(|restore| restore.client_num)
    };
    {
        let _slots = PlayerSlotsForTest::new(64);
        assert_eq!(
            chat_command(ChatDestination::Player(40), "hi").as_deref(),
            Some("tell 40 \"hi\"")
        );
        assert_eq!(chat_command(ChatDestination::Player(64), "hi"), None);
        assert_eq!(restore("63"), Some(63));
        assert_eq!(restore("64"), None);
    }
    let _slots = PlayerSlotsForTest::new(32);
    assert_eq!(chat_command(ChatDestination::Player(40), "hi"), None);
    assert_eq!(restore("31"), Some(31));
    assert_eq!(restore("32"), None);
}
