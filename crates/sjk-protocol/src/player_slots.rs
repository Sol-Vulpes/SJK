//! How many player slots the connected server has, and where each player's data sits.
//!
//! Protocol 26 was built for `MAX_CLIENTS` = 32: a player's client number is its entity
//! number, and its `CS_PLAYERS` configstring follows at `CS_PLAYERS + client`. A server
//! with more than 32 player slots keeps all of that for its first 32 players and adds
//! the rest above: their entity numbers go on from 32, and their configstrings sit in
//! the range no protocol-26 configstring uses, from [`CS_PLAYERS_HIGH`].
//!
//! Three things replace a bare `32` in the client:
//!
//! * [`PLAYER_CAPACITY`], the most players any server can have: the size of every
//!   per-player table.
//! * [`player_slots`], how many slots *this* server has: the bound of every "is this
//!   entity a player" and "is this a client number" test. It is 32 unless
//!   [`set_player_slots`] said otherwise, so on a classic server nothing changes.
//! * [`player_configstring`] and [`configstring_player`], the two directions between a
//!   client number and its configstring index.

use std::sync::atomic::{AtomicUsize, Ordering};

/// `MAX_CLIENTS` of a classic server: the slots every protocol-26 client knows.
pub const CLASSIC_PLAYER_SLOTS: usize = 32;

/// The most player slots a server can have: the size of a per-player table.
pub const PLAYER_CAPACITY: usize = 64;

/// `CS_PLAYERS`: the configstring of player 0; players 1 to 31 follow.
pub const CS_PLAYERS: usize = 1_131;

/// The configstring of player 32; players 33 to 63 follow. Protocol 26's last
/// configstring in use is 1643 (`CS_MAX` is 1644) and `MAX_CONFIGSTRINGS` is 1700.
pub const CS_PLAYERS_HIGH: usize = 1_644;

static PLAYER_SLOTS: AtomicUsize = AtomicUsize::new(CLASSIC_PLAYER_SLOTS);

/// How many player slots the connected server has: 32 on a classic server.
pub fn player_slots() -> usize {
    PLAYER_SLOTS.load(Ordering::Relaxed)
}

/// Sets the connected server's player slots, held between 32 and [`PLAYER_CAPACITY`].
pub fn set_player_slots(slots: usize) {
    PLAYER_SLOTS.store(
        slots.clamp(CLASSIC_PLAYER_SLOTS, PLAYER_CAPACITY),
        Ordering::Relaxed,
    );
}

/// Back to a classic server's 32 slots (on disconnect).
pub fn reset_player_slots() {
    set_player_slots(CLASSIC_PLAYER_SLOTS);
}

/// Whether `number` (a client number, which is also that player's entity number) names
/// a player slot of the connected server.
pub fn is_player_number(number: usize) -> bool {
    number < player_slots()
}

/// The configstring index of player `client`, or `None` past the server's slots.
pub fn player_configstring(client: usize) -> Option<usize> {
    if client >= player_slots() {
        None
    } else if client < CLASSIC_PLAYER_SLOTS {
        Some(CS_PLAYERS + client)
    } else {
        Some(CS_PLAYERS_HIGH + (client - CLASSIC_PLAYER_SLOTS))
    }
}

/// The player whose configstring is `index`, or `None` if it is not a player's (on a
/// classic server the high range is never one).
pub fn configstring_player(index: usize) -> Option<usize> {
    let client = if (CS_PLAYERS..CS_PLAYERS + CLASSIC_PLAYER_SLOTS).contains(&index) {
        index - CS_PLAYERS
    } else if (CS_PLAYERS_HIGH..CS_PLAYERS_HIGH + (PLAYER_CAPACITY - CLASSIC_PLAYER_SLOTS))
        .contains(&index)
    {
        CLASSIC_PLAYER_SLOTS + (index - CS_PLAYERS_HIGH)
    } else {
        return None;
    };
    (client < player_slots()).then_some(client)
}

/// Holds the slot count at `slots` for a test and restores 32 when dropped. Tests that
/// change the count run one at a time behind this guard; the rest of the suite sees 32.
pub struct PlayerSlotsForTest(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl PlayerSlotsForTest {
    pub fn new(slots: usize) -> Self {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_player_slots(slots);
        Self(guard)
    }
}

impl Drop for PlayerSlotsForTest {
    fn drop(&mut self) {
        reset_player_slots();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_ranges_map_each_way() {
        {
            let _slots = PlayerSlotsForTest::new(64);
            assert_eq!(player_configstring(0), Some(1_131));
            assert_eq!(player_configstring(31), Some(1_162));
            assert_eq!(player_configstring(32), Some(1_644));
            assert_eq!(player_configstring(63), Some(1_675));
            assert_eq!(player_configstring(64), None);
            assert_eq!(configstring_player(1_675), Some(63));
            assert_eq!(configstring_player(1_163), None);
            assert_eq!(configstring_player(1_676), None);
            assert!(is_player_number(63));
        }
        // A classic server: 32 slots, and the high range is nobody's.
        let _slots = PlayerSlotsForTest::new(32);
        assert_eq!(player_configstring(32), None);
        assert_eq!(configstring_player(1_644), None);
        assert_eq!(configstring_player(1_162), Some(31));
        assert!(!is_player_number(32));
    }
}
