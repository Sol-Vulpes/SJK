//! Who a toy may act on right now: the rules the toys that strike or steer the player
//! share (the Ragdoll and the Monitor; the Prop gun until 20/11/2026). They came with Order 66 (17/10/2026),
//! removed on 20/11/2026, and live here since.

use super::*;

/// `PMF_FOLLOW`: the player state is that of the player being followed.
const PMF_FOLLOW: u16 = 4096;
/// `GT_DUEL` and `GT_POWERDUEL`.
const GT_DUEL: i32 = 3;
const GT_POWERDUEL: i32 = 4;
/// The flags' powerups: `PW_REDFLAG`, `PW_BLUEFLAG`, `PW_NEUTRALFLAG`.
const FLAGS: [usize; 3] = [4, 5, 6];

/// Why a toy leaves the player alone now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spared {
    /// Spectating, or at the intermission: nothing to say.
    Away,
    /// Already dead: nothing to say.
    Dead,
    Duel,
    Flag,
    /// The game server holds the player's view (a kiss, a grapple, a kick: `held_view`).
    /// Never from [`spared`]: the toys that steer the player add it, as their commands
    /// would fight the server's view.
    Held,
}

/// Whether `player` (the player's own state, or the followed player's) is in a duel: a
/// private one, or any game of a Duel or Power Duel server (`gametype`, `g_gametype`).
pub(crate) fn duelling(player: &sjk_protocol::PlayerState, gametype: i32) -> bool {
    matches!(gametype, GT_DUEL | GT_POWERDUEL) || player.duel_in_progress()
}

/// Why a toy leaves the player alone, if it does: `player` is the player's state,
/// `gametype` the server's `g_gametype`.
pub(crate) fn spared(player: &sjk_protocol::PlayerState, gametype: i32) -> Option<Spared> {
    // `pmtype_t`: PM_SPECTATOR 4, PM_DEAD 5, PM_INTERMISSION 7, PM_SPINTERMISSION 8.
    if player.movement_flags() & PMF_FOLLOW != 0
        || player.is_spectator()
        || matches!(player.movement_type(), 7 | 8)
    {
        return Some(Spared::Away);
    }
    if player.health() <= 0 || player.movement_type() == 5 {
        return Some(Spared::Dead);
    }
    if duelling(player, gametype) {
        return Some(Spared::Duel);
    }
    let time = player.command_time();
    if FLAGS.iter().any(|&flag| player.powerup_active(flag, time)) {
        return Some(Spared::Flag);
    }
    None
}

impl GpuState {
    /// Whether the player is away from the keyboard, as their own client knows it: idle
    /// or alt-tabbed (`away_emote`), or wearing an away tag in their name (`afk`).
    pub(crate) fn own_away(&self) -> bool {
        crate::away_emote::own_away()
            || self.live_session.as_ref().is_some_and(|session| {
                let game = session.game_state();
                let Ok(own) = u8::try_from(game.client_num) else {
                    return false;
                };
                let name = crate::player_identity::shown_name(game, usize::from(own))
                    .unwrap_or_else(|| "Someone".to_owned());
                self.scoreboard.seen_away(own, &name)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> sjk_protocol::PlayerState {
        let mut player = sjk_protocol::PlayerState::default();
        player.stats[0] = 100;
        player.persistent[3] = 1;
        player
    }

    /// A toy leaves a spectator, the dead and a duellist alone (in a private duel, or on
    /// a Duel or Power Duel server), and acts on a player in the game.
    #[test]
    fn a_toy_spares_spectators_the_dead_and_duellists() {
        assert_eq!(spared(&playing(), 0), None);
        let mut spectator = playing();
        spectator.persistent[3] = 3;
        assert_eq!(spared(&spectator, 0), Some(Spared::Away));
        let mut dead = playing();
        dead.stats[0] = 0;
        assert_eq!(spared(&dead, 0), Some(Spared::Dead));
        let mut duelling_player = playing();
        assert!(duelling_player.set_raw_field(119, 1));
        assert_eq!(spared(&duelling_player, 0), Some(Spared::Duel));
        assert!(duelling(&playing(), GT_DUEL) && duelling(&playing(), GT_POWERDUEL));
        assert!(!duelling(&playing(), 0));
    }
}
