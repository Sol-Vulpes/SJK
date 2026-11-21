//! The ping the top bar shows: the server's own measure, the one on the scoreboard, which
//! is the number players compare.
//!
//! The server times each snapshot from its leaving to the arrival of the client's packet
//! that acknowledges it, and the scoreboard shows the average of the last 32
//! (`SV_CalcPings`). A `scores` reply carries it, but only comes while the scoreboard is
//! held (a request every 2 seconds) or at an intermission, and asking more often would
//! spend client commands that servers count (JoF's cap them). So the bar shows our own
//! row's ping from a reply for [`SCORES_FRESH`] after it came, and otherwise the average of
//! the stock ping ([`sjk_client::ClientSession::snapshot_ping`]) of the last [`AVERAGED`]
//! snapshots that show our own commands run, as many as the server averages. That ping has
//! the server measure's parts the other way round (a round trip, a wait of up to a packet
//! interval at one end and up to a frame at the other,
//! [`sjk_client::snapshot_ping`]), so their averages agree where single samples do not.
//!
//! A followed player's or the intermission's snapshots say nothing of our commands and
//! are not counted ([`crate::live_session`] passes only our own), and stock's 999 for "no
//! packet reached" is no time at all. Without a fresh reply or a new sample the bar keeps
//! the newer of the two it has: spectating a player shows the ping measured before, or
//! the last scoreboard's. A reply already held when the bar starts watching (each map's
//! new world, an intermission's reply in it) is of unknown age and not shown.

use std::time::{Duration, Instant};

/// Snapshots averaged: `PACKET_BACKUP`, the snapshots `SV_CalcPings` averages.
pub(super) const AVERAGED: usize = 32;
/// How long a `scores` reply's ping is shown after it came: two of the scoreboard's
/// requests, every 2 seconds while it is held, and their round trips.
pub(super) const SCORES_FRESH: Duration = Duration::from_secs(5);
/// The highest ping a scoreboard row shows (`SV_CalcPings` caps at 999).
const MAX_PING: i32 = 999;

/// The bar's ping, from the snapshots and the `scores` replies (module docs).
pub(crate) struct Ping {
    /// The last [`AVERAGED`] snapshot pings, oldest overwritten first.
    samples: [u16; AVERAGED],
    next: usize,
    len: usize,
    sum: u32,
    /// When the newest sample came.
    sampled: Option<Instant>,
    /// The `scores` revision last seen ([`sjk_client::ClientSession::scores_revision`]).
    revision: Option<u32>,
    /// Our own row's ping in the newest reply that had one, and when it came.
    scores: Option<(Instant, u16)>,
}

impl Default for Ping {
    fn default() -> Self {
        Self {
            samples: [0; AVERAGED],
            next: 0,
            len: 0,
            sum: 0,
            sampled: None,
            revision: None,
            scores: None,
        }
    }
}

impl Ping {
    /// A snapshot that shows our own commands run came at `now` with the stock ping
    /// `ping`; stock's [`sjk_client::NO_MATCH_PING`] and anything out of range are left
    /// out.
    pub(crate) fn snapshot(&mut self, ping: i32, now: Instant) {
        if !(0..sjk_client::NO_MATCH_PING).contains(&ping) {
            return;
        }
        let ping = ping as u16;
        if self.len == AVERAGED {
            self.sum -= u32::from(self.samples[self.next]);
        } else {
            self.len += 1;
        }
        self.samples[self.next] = ping;
        self.sum += u32::from(ping);
        self.next = (self.next + 1) % AVERAGED;
        self.sampled = Some(now);
    }

    /// The session's `scores` revision is `revision` at `now`; when it is a new one,
    /// `own` finds our row's ping in it (none for a reply without our row, `-1` while
    /// connecting). The first revision seen is only noted: a reply already there (the
    /// intermission's, when a new map's world takes over) is of unknown age.
    pub(crate) fn scores(
        &mut self,
        revision: u32,
        own: impl FnOnce() -> Option<i32>,
        now: Instant,
    ) {
        match self.revision.replace(revision) {
            Some(seen) if seen != revision => {}
            _ => return,
        }
        if let Some(ping) = own().filter(|ping| (0..=MAX_PING).contains(ping)) {
            self.scores = Some((now, ping as u16));
        }
    }

    /// What the bar shows at `now`: a fresh reply's ping, else the snapshots' average or
    /// the older reply's, whichever came last; nothing before either.
    pub(crate) fn shown(&self, now: Instant) -> Option<u16> {
        let average = (self.len > 0).then(|| {
            let len = self.len as u32;
            ((self.sum + len / 2) / len) as u16
        });
        match (self.scores, average) {
            (Some((at, ping)), _) if now.saturating_duration_since(at) <= SCORES_FRESH => {
                Some(ping)
            }
            (Some((at, ping)), Some(average)) => Some(if self.sampled > Some(at) {
                average
            } else {
                ping
            }),
            (scores, average) => average.or(scores.map(|(_, ping)| ping)),
        }
    }

    /// Forget everything: another session or time line.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    /// The stock pings of a 125-packet stream wander over a packet interval and a frame;
    /// their average over the last 32 is what the bar shows, rounded.
    #[test]
    fn the_bar_averages_the_last_32_snapshot_pings() {
        let start = Instant::now();
        let mut ping = Ping::default();
        assert_eq!(ping.shown(start), None);
        // 32 snapshots between 6 and 13 ms: an average of 9.5, shown as 10.
        for snapshot in 0..AVERAGED as u64 {
            ping.snapshot(6 + (snapshot % 8) as i32, ms(start, 33 * snapshot));
        }
        assert_eq!(ping.shown(ms(start, 1_100)), Some(10));
        // Only the last 32 count: 32 more at 40 replace them all.
        for snapshot in 0..AVERAGED as u64 {
            ping.snapshot(40, ms(start, 1_100 + 33 * snapshot));
        }
        assert_eq!(ping.shown(ms(start, 2_200)), Some(40));
        // Half replaced by 20s: (16 * 40 + 16 * 20) / 32.
        for snapshot in 0..16 {
            ping.snapshot(20, ms(start, 2_200 + 33 * snapshot));
        }
        assert_eq!(ping.shown(ms(start, 2_800)), Some(30));
    }

    /// Stock's 999 is "no packet reached", not a time, and negatives are no ping.
    #[test]
    fn stocks_999_and_nonsense_are_left_out() {
        let start = Instant::now();
        let mut ping = Ping::default();
        ping.snapshot(sjk_client::NO_MATCH_PING, start);
        ping.snapshot(-1, start);
        assert_eq!(ping.shown(start), None);
        ping.snapshot(12, start);
        ping.snapshot(sjk_client::NO_MATCH_PING, start);
        assert_eq!(ping.shown(start), Some(12));
    }

    /// A reply's ping, the server's own measure, is shown while fresh; then the newer of
    /// the two.
    #[test]
    fn a_fresh_scoreboard_ping_wins_then_the_newer_value() {
        let start = Instant::now();
        let mut ping = Ping::default();
        ping.scores(0, || panic!("the first revision is only noted"), start);
        ping.snapshot(8, start);
        ping.scores(1, || Some(11), ms(start, 100));
        assert_eq!(ping.shown(ms(start, 200)), Some(11));
        assert_eq!(ping.shown(ms(start, 100) + SCORES_FRESH), Some(11));
        // Stale, and no snapshot since it: still the reply's (a spectator following).
        let stale = ms(start, 200) + SCORES_FRESH;
        assert_eq!(ping.shown(stale), Some(11));
        // A snapshot after it: the average again.
        ping.snapshot(8, stale);
        assert_eq!(ping.shown(stale), Some(8));
    }

    /// The first revision seen is of unknown age (an intermission's reply when a new
    /// map's world takes over) and only noted; a revision already seen is not read again,
    /// and a reply without our row (or with the connecting `-1`) leaves the last one
    /// standing.
    #[test]
    fn only_a_new_reply_with_our_row_counts() {
        let start = Instant::now();
        let mut ping = Ping::default();
        ping.scores(3, || panic!("the first revision is only noted"), start);
        assert_eq!(ping.shown(start), None);
        ping.scores(4, || Some(30), start);
        ping.scores(4, || panic!("revision 4 was read"), ms(start, 1_000));
        ping.scores(5, || None, ms(start, 1_000));
        ping.scores(6, || Some(-1), ms(start, 1_000));
        assert_eq!(ping.shown(ms(start, 1_000)), Some(30));
        // Revision 4's, stale now, is still the only value.
        assert_eq!(
            ping.shown(start + SCORES_FRESH + Duration::from_millis(1)),
            Some(30)
        );
        ping.scores(7, || Some(25), ms(start, 2_000));
        assert_eq!(ping.shown(ms(start, 2_000)), Some(25));
    }

    /// A demo or a local game has no packets: no sample and no reply, so no ping.
    #[test]
    fn nothing_without_samples_or_replies() {
        let start = Instant::now();
        let mut ping = Ping::default();
        assert_eq!(ping.shown(start), None);
        ping.scores(1, || None, start);
        ping.snapshot(15, start);
        ping.scores(2, || Some(20), start);
        ping.clear();
        assert_eq!(ping.shown(ms(start, 10)), None);
        // After a clear the revision is noted afresh, not read.
        ping.scores(
            3,
            || panic!("the first revision is only noted"),
            ms(start, 10),
        );
        assert_eq!(ping.shown(ms(start, 10)), None);
    }
}
