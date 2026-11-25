//! Why a player holding a saber shows none, written to the session log.
//!
//! Hub report #248 and Sol (24/11/2026): now and then another player holds a saber
//! with no hilt and no blade at all, and stays so. Every path read so far draws one,
//! so the frame itself says which check took it away: the weapon gone with a cut
//! limb on a living player, no hilt name, no hand bolt from the pose, no hilt
//! catalog. A reason that lasts [`STUCK_MS`] is written once, and again when the
//! saber comes back, so a stray frame (a pose not yet evaluated) writes nothing.

use std::sync::Mutex;

/// How long a reason must hold before it is written.
const STUCK_MS: i64 = 2_000;
/// Client slots (`MAX_CLIENTS`); NPCs are not followed.
const CLIENTS: usize = sjk_protocol::PLAYER_CAPACITY;

/// The check that left a held saber undrawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The actor's surfaces still say the weapon went with a cut limb.
    WeaponLost,
    /// No hilt catalog is loaded.
    NoCatalog,
    /// The actor mesh carries no first saber name.
    NoHiltName,
    /// The pose gave no usable right-hand bolt.
    NoHandBolt,
}

impl Reason {
    fn describe(self) -> &'static str {
        match self {
            Self::WeaponLost => "weapon marked lost with a cut limb while alive",
            Self::NoCatalog => "no saber hilt catalog",
            Self::NoHiltName => "no saber name on the player's model",
            Self::NoHandBolt => "no right-hand bolt from the pose",
        }
    }
}

/// What the frame knows about a player whose saber should be in hand.
pub(crate) struct Held<'a> {
    pub(crate) weapon_lost: bool,
    pub(crate) catalog: bool,
    pub(crate) hilt_name: Option<&'a str>,
    pub(crate) hand_bolt: bool,
}

/// The first check that leaves the saber undrawn, if any.
pub(crate) fn reason(held: &Held<'_>) -> Option<Reason> {
    if held.weapon_lost {
        Some(Reason::WeaponLost)
    } else if !held.catalog {
        Some(Reason::NoCatalog)
    } else if held.hilt_name.is_none() {
        Some(Reason::NoHiltName)
    } else if !held.hand_bolt {
        Some(Reason::NoHandBolt)
    } else {
        None
    }
}

#[derive(Clone, Copy, Default)]
struct Track {
    reason: Option<Reason>,
    since: i64,
    logged: bool,
}

impl Track {
    /// Follow one frame; the line to write, if this frame writes one.
    fn observe(&mut self, reason: Option<Reason>, now: i64) -> Option<Line> {
        if reason != self.reason {
            let back = self.logged && reason.is_none();
            let was = self.since;
            *self = Track {
                reason,
                since: now,
                logged: false,
            };
            return back.then_some(Line::Back {
                after_ms: now - was,
            });
        }
        let reason = reason?;
        if self.logged || now - self.since < STUCK_MS {
            return None;
        }
        self.logged = true;
        Some(Line::Missing {
            reason,
            for_ms: now - self.since,
        })
    }
}

#[derive(Debug, PartialEq)]
enum Line {
    Missing { reason: Reason, for_ms: i64 },
    Back { after_ms: i64 },
}

static TRACKS: Mutex<[Track; CLIENTS]> = Mutex::new(
    [Track {
        reason: None,
        since: 0,
        logged: false,
    }; CLIENTS],
);

/// Follow `client`'s held saber this frame (`None`: it is drawn, or not meant to be);
/// `model` names the player's model in the line.
pub(crate) fn observe(client: usize, reason: Option<Reason>, now: i64, model: &str) {
    let Ok(mut tracks) = TRACKS.lock() else {
        return;
    };
    let Some(track) = tracks
        .get_mut(client)
        .filter(|_| sjk_protocol::is_player_number(client))
    else {
        return;
    };
    match track.observe(reason, now) {
        Some(Line::Missing { reason, for_ms }) => crate::log::progress(format_args!(
            "saber probe: client {client} ({model}) holds a saber but none is drawn \
             ({:.1} s): {}",
            for_ms as f64 / 1_000.0,
            reason.describe()
        )),
        Some(Line::Back { after_ms }) => crate::log::progress(format_args!(
            "saber probe: client {client}'s saber is drawn again after {:.1} s",
            after_ms as f64 / 1_000.0
        )),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(name: Option<&str>, bolt: bool) -> Held<'_> {
        Held {
            weapon_lost: false,
            catalog: true,
            hilt_name: name,
            hand_bolt: bolt,
        }
    }

    #[test]
    fn a_drawn_saber_has_no_reason() {
        assert_eq!(reason(&held(Some("single_1"), true)), None);
        assert_eq!(reason(&held(None, true)), Some(Reason::NoHiltName));
        assert_eq!(
            reason(&held(Some("single_1"), false)),
            Some(Reason::NoHandBolt)
        );
        let lost = Held {
            weapon_lost: true,
            ..held(None, false)
        };
        assert_eq!(reason(&lost), Some(Reason::WeaponLost));
    }

    #[test]
    fn only_a_lasting_reason_is_written_once_then_its_end() {
        let mut track = Track::default();
        let bolt = Some(Reason::NoHandBolt);
        assert_eq!(track.observe(bolt, 1_000), None);
        assert_eq!(track.observe(bolt, 2_500), None);
        assert_eq!(
            track.observe(bolt, 3_000),
            Some(Line::Missing {
                reason: Reason::NoHandBolt,
                for_ms: 2_000
            })
        );
        assert_eq!(track.observe(bolt, 9_000), None);
        assert_eq!(
            track.observe(None, 10_000),
            Some(Line::Back { after_ms: 9_000 })
        );
    }

    #[test]
    fn a_stray_frame_writes_nothing() {
        let mut track = Track::default();
        assert_eq!(track.observe(Some(Reason::NoHandBolt), 0), None);
        assert_eq!(track.observe(None, 16), None);
        assert_eq!(track.observe(None, 5_000), None);
    }
}
