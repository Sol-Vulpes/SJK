//! Bounded, snapshot-driven diagnostics for server-side command starvation.

use crate::gamestate_probe::MovePacketHead;
use std::fmt::{self, Write};
use std::time::{Duration, Instant};

const STALLED_SNAPSHOTS: u32 = 40;

/// Whether the server holds the player still for this `pm_type`: `PM_INTERMISSION` and
/// `PM_SPINTERMISSION` (`bg_public.h`), whose `ClientThink_real` returns before `Pmove`
/// (`g_active.c`), so `commandTime` does not advance and no freeze is reported.
pub(crate) fn held(movement_type: u8) -> bool {
    matches!(movement_type, crate::PM_INTERMISSION | 8)
}

/// The client's reliable commands at a snapshot: those sent and not yet acknowledged
/// (every packet carries them all again, `CL_WritePacket`), their bytes, and those still
/// waiting in the pacer. A backlog that grows while the freeze lasts means no packet
/// gets as far as its commands at the server (lost, or dropped whole for its serverId,
/// `SV_ExecuteClientMessage`); one that stays empty means the server reads them but
/// skips every usercmd (`SV_UserMove`: none newer than the last it ran). The bytes
/// matter because SJK never fragments a move packet as `Netchan_Transmit` does past
/// 1300 bytes.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Outbox {
    pub(crate) unacknowledged: usize,
    pub(crate) bytes: usize,
    pub(crate) waiting: usize,
}

#[derive(Debug)]
/// Fixed-storage history of sent stamps and snapshot command-time progress.
pub(crate) struct FreezeProbe {
    gamestate_id: i32,
    previous_map_stamp: i32,
    maximum_sent: i32,
    sent: Option<MovePacketHead>,
    previous: Option<(i32, i32)>,
    stalled: u32,
    next_report: Option<Instant>,
    command_sequence: i32,
    command_bytes: [u8; 256],
    command_len: usize,
    truncated: bool,
}

impl Default for FreezeProbe {
    fn default() -> Self {
        Self {
            gamestate_id: 0,
            previous_map_stamp: 0,
            maximum_sent: 0,
            sent: None,
            previous: None,
            stalled: 0,
            next_report: None,
            command_sequence: 0,
            command_bytes: [0; 256],
            command_len: 0,
            truncated: false,
        }
    }
}

impl FreezeProbe {
    /// Start a new map's watermark while retaining the previous map's stamp.
    pub(super) fn gamestate(&mut self, id: i32) {
        self.gamestate_id = id;
        self.previous_map_stamp = self.maximum_sent;
        self.maximum_sent = 0;
        self.sent = None;
        self.previous = None;
        self.stalled = 0;
        self.next_report = None;
    }

    /// Store the latest reliable command without allocating or logging unsafe bytes.
    pub(super) fn command(&mut self, sequence: i32, bytes: &[u8]) {
        if sequence < self.command_sequence {
            return;
        }
        self.command_sequence = sequence;
        self.command_len = bytes.len().min(self.command_bytes.len());
        self.command_bytes[..self.command_len].copy_from_slice(&bytes[..self.command_len]);
        self.truncated = bytes.len() > self.command_len;
    }

    /// Preserve the actual sent watermark even when later stamps go backwards.
    pub(super) fn sent(&mut self, head: MovePacketHead) {
        self.maximum_sent = self.maximum_sent.max(head.command_stamp);
        self.sent = Some(head);
    }

    /// Report sustained command starvation at most once per second.
    ///
    /// `held`: the server holds the player at the intermission, where
    /// `ClientThink_real` returns before `Pmove` (`g_active.c`), so `commandTime`
    /// stops by design. Before this every map's end logged 15 s of false freezes,
    /// starting right after "Timelimit hit." (Sol's session log, 25/11/2026).
    pub(super) fn snapshot(
        &mut self,
        now: Instant,
        head: MovePacketHead,
        outbox: Outbox,
        held: bool,
    ) {
        if held {
            self.previous = None;
            self.stalled = 0;
            self.next_report = None;
            return;
        }
        if !self.observe(now, head.snapshot_time, head.command_time) {
            return;
        }
        let sent = self.sent.unwrap_or(head);
        crate::diagnostics::line(format_args!(
            "clientthink freeze: snapshots={} snapTime={} cmdTime={} sentServerId={} \
             gamestateServerId={} currentServerId={} oldServerTime=unknown(server-only) \
             previousMapStamp={} stamp={} maxSentSinceGamestate={} msgAck={} \
             sentRelAck={} relAck={}/{} outbox={} unacked ({} bytes) {} waiting \
             lastServerCommand[{}]=\"{}{}\"",
            self.stalled,
            head.snapshot_time,
            head.command_time,
            sent.server_id,
            self.gamestate_id,
            head.server_id,
            self.previous_map_stamp,
            sent.command_stamp,
            self.maximum_sent,
            sent.message_acknowledge,
            sent.server_command_sequence,
            head.server_command_sequence,
            head.highest_server_command,
            outbox.unacknowledged,
            outbox.bytes,
            outbox.waiting,
            self.command_sequence,
            Escaped(&self.command_bytes[..self.command_len]),
            if self.truncated { "..." } else { "" },
        ));
    }

    fn observe(&mut self, now: Instant, snapshot_time: i32, command_time: i32) -> bool {
        self.stalled = match self.previous {
            Some((snap, cmd)) if snapshot_time > snap && command_time == cmd => {
                self.stalled.saturating_add(1)
            }
            _ => 0,
        };
        self.previous = Some((snapshot_time, command_time));
        if self.stalled == 0 {
            self.next_report = None;
        }
        if self.stalled < STALLED_SNAPSHOTS || self.next_report.is_some_and(|next| now < next) {
            return false;
        }
        self.next_report = Some(now + Duration::from_secs(1));
        true
    }
}

struct Escaped<'a>(&'a [u8]);

impl fmt::Display for Escaped<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &byte in self.0 {
            for escaped in std::ascii::escape_default(byte) {
                f.write_char(char::from(escaped))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(snapshot_time: i32) -> MovePacketHead {
        MovePacketHead {
            server_id: 1,
            message_acknowledge: 0,
            server_command_sequence: 0,
            command_stamp: 0,
            snapshot_time,
            command_time: 1_801_000,
            highest_server_command: 0,
        }
    }

    #[test]
    fn the_intermission_is_not_a_freeze() {
        let now = Instant::now();
        let mut probe = FreezeProbe::default();
        for frame in 0..60 {
            probe.snapshot(now, head(1_802_000 + frame * 33), Outbox::default(), true);
        }
        assert_eq!(probe.stalled, 0);
        for frame in 60..120 {
            probe.snapshot(now, head(1_802_000 + frame * 33), Outbox::default(), false);
        }
        assert_eq!(probe.stalled, 59);
        assert!(held(crate::PM_INTERMISSION) && held(8) && !held(6) && !held(0));
    }
}
