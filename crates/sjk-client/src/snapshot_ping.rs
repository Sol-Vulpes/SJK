//! The ping stock gives every snapshot, `cl.snap.ping` (`CL_ParseSnapshot`,
//! `codemp/client/cl_parse.cpp`), which the lagometer graphs and EJK's chat writes.
//!
//! For every move packet `CL_WritePacket` (`codemp/client/cl_input.cpp`) keeps when it
//! left (`p_realtime`) and the `serverTime` of the newest command in it (`p_serverTime`).
//! A snapshot's ping is the time from the newest packet whose newest command the
//! snapshot's `ps.commandTime` has reached (the last packet the server ran before it built
//! the snapshot) to the snapshot's reading, and 999 when no packet kept was reached.
//!
//! The server runs a packet's commands as it arrives, so this is a round trip, plus the
//! time from that packet's arrival to the snapshot built after it (under one packet
//! interval: a later packet would have been the one found) and the client's wait before
//! reading the snapshot (under one frame). The scoreboard's ping, the server's measure
//! (`SV_CalcPings`: from a snapshot's leaving to the arrival of the packet that
//! acknowledges it), has the same parts the other way round: a round trip, the client's
//! wait to read the snapshot and its wait for the next packet.
//!
//! [`SentPackets`] keeps the last [`SENT_PACKETS`] move packets in storage allocated once
//! with the session: more than stock's `PACKET_BACKUP` (32), which at 125 packets a
//! second covers only 256 ms of round trip before it gives 999.

use std::time::Instant;

/// Move packets kept: two seconds at 125 a second, a quarter of a second at the 1000 a
/// second `cl_maxpackets` allows.
pub const SENT_PACKETS: usize = 256;

/// Stock's ping when no packet kept has been reached (`cl.snap.ping = 999`).
pub const NO_MATCH_PING: i32 = 999;

#[derive(Clone, Copy, Debug)]
struct Sent {
    /// The `serverTime` of the packet's newest command.
    stamp: i32,
    /// When the packet left.
    at: Instant,
}

/// The last move packets sent, newest last, for [`SentPackets::ping`].
#[derive(Debug)]
pub struct SentPackets {
    ring: Box<[Sent]>,
    /// Packets recorded since the last [`SentPackets::clear`].
    count: u64,
}

impl Default for SentPackets {
    fn default() -> Self {
        Self::new()
    }
}

impl SentPackets {
    pub fn new() -> Self {
        let placeholder = Sent {
            stamp: 0,
            at: Instant::now(),
        };
        Self {
            ring: vec![placeholder; SENT_PACKETS].into_boxed_slice(),
            count: 0,
        }
    }

    /// A move packet left at `at`, its newest command stamped `stamp`. A stamp of 0 or
    /// less is not kept: the commands sent before the clock stands on a time line (a map
    /// load, `CL_ClearState`) are stamped 0, which every `commandTime` has reached.
    pub fn sent(&mut self, stamp: i32, at: Instant) {
        if stamp <= 0 {
            return;
        }
        let index = (self.count % SENT_PACKETS as u64) as usize;
        self.ring[index] = Sent { stamp, at };
        self.count += 1;
    }

    /// The ping, in milliseconds, of a snapshot read at `at` whose player state's
    /// `commandTime` is `command_time`: from the newest packet kept whose newest command
    /// it has reached, [`NO_MATCH_PING`] when it has reached none. `None` while no packet
    /// is kept: nothing stamped has been sent since the game state (the first snapshots
    /// of a map, a demo, a local game).
    pub fn ping(&self, command_time: i32, at: Instant) -> Option<i32> {
        if self.count == 0 {
            return None;
        }
        let kept = self.count.min(SENT_PACKETS as u64);
        for back in 0..kept {
            let index = ((self.count - 1 - back) % SENT_PACKETS as u64) as usize;
            let sent = self.ring[index];
            if command_time >= sent.stamp {
                let millis = at.saturating_duration_since(sent.at).as_millis();
                return Some(i32::try_from(millis).unwrap_or(i32::MAX));
            }
        }
        Some(NO_MATCH_PING)
    }

    /// Forget every packet: a new game state starts a new time line (`CL_ClearState`
    /// clears `cl.outPackets`).
    pub fn clear(&mut self) {
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    /// `CL_ParseSnapshot` walks back from the newest packet to the first whose newest
    /// command the snapshot shows run: packets sent after it are not counted yet.
    #[test]
    fn the_newest_packet_the_server_ran_gives_the_ping() {
        let start = Instant::now();
        let mut packets = SentPackets::new();
        // A packet every 8 ms, its newest command 8 ms later each time.
        for packet in 0..10_u64 {
            packets.sent(1_000 + 8 * packet as i32, ms(start, 8 * packet));
        }
        // Read at 80 ms: the server had run up to the command stamped 1_040 (the sixth
        // packet, sent at 40 ms), not the four after it.
        assert_eq!(packets.ping(1_040, ms(start, 80)), Some(40));
        // A `commandTime` between two packets' stamps reached the older one only.
        assert_eq!(packets.ping(1_043, ms(start, 80)), Some(40));
        // Everything run: the newest packet, sent at 72 ms.
        assert_eq!(packets.ping(2_000, ms(start, 80)), Some(8));
    }

    #[test]
    fn a_command_time_before_every_packet_kept_is_stocks_999() {
        let start = Instant::now();
        let mut packets = SentPackets::new();
        packets.sent(5_000, start);
        packets.sent(5_008, ms(start, 8));
        assert_eq!(packets.ping(4_990, ms(start, 50)), Some(NO_MATCH_PING));
    }

    /// A demo or a local game sends no packet, and the map load's are stamped 0: no
    /// ping rather than one made up from them.
    #[test]
    fn no_ping_without_a_stamped_packet() {
        let start = Instant::now();
        let mut packets = SentPackets::new();
        assert_eq!(packets.ping(1_000, start), None);
        packets.sent(0, start);
        packets.sent(-5, start);
        assert_eq!(packets.ping(1_000, ms(start, 30)), None);
        packets.sent(900, start);
        packets.clear();
        assert_eq!(packets.ping(1_000, ms(start, 30)), None);
    }

    /// Past [`SENT_PACKETS`] the oldest are overwritten: the walk reads only the newest
    /// that many, in order, across the ring's end.
    #[test]
    fn the_ring_wraps_and_keeps_the_newest() {
        let start = Instant::now();
        let mut packets = SentPackets::new();
        let total = SENT_PACKETS as u64 + 40;
        for packet in 0..total {
            packets.sent(1 + packet as i32, ms(start, packet));
        }
        let now = ms(start, total);
        // The newest packet (stamp `total`, sent at `total - 1` ms).
        assert_eq!(packets.ping(total as i32, now), Some(1));
        // One just inside the ring, after the wrap.
        let oldest_kept = total - SENT_PACKETS as u64;
        assert_eq!(
            packets.ping(1 + oldest_kept as i32, now),
            Some(SENT_PACKETS as i32)
        );
        // One overwritten: no packet kept is that old.
        assert_eq!(packets.ping(oldest_kept as i32, now), Some(NO_MATCH_PING));
    }
}
