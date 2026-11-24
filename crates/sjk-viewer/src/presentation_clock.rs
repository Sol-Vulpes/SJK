//! Wall-clock paced remote presentation with an interpolation bracket in hand.
//!
//! OpenJK codemp `cl_cgame.cpp:626-673,766-795` adjusts the realtime/server
//! delta per snapshot: reset above 500 ms, halve above 100 ms, otherwise drift
//! +1 ms or -2 ms if a frame approached the newest snapshot within 5 ms.
//! The feedback ignores time nudge; sampling subtracts it and never rewinds.
//! Unlike stock's first/reset frame, seed one interval behind the newest
//! snapshot: the runtime keeps a few samples per entity
//! ([`sjk_runtime::MOTION_HISTORY`]), not a snap queue. That drift settles the drawn
//! time a little more than one interval behind the newest snapshot, so the runtime
//! must still interpolate the pair before it.
//! Keep fractional milliseconds across arrivals; never clamp time to a packet.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

const DEFAULT_INTERVAL_MILLIS: i64 = 25;
const RESET_MILLIS: i64 = 500;
const FAST_ADJUST_MILLIS: i64 = 100;
const EXTRAPOLATION_MARGIN_MILLIS: i64 = 5;

/// Cached archived `cl_timenudge`, avoiding allocating cvar lookups each frame.
pub(crate) struct CvarSetting(Arc<AtomicI64>);

impl CvarSetting {
    /// Bind after registration, before cfg loading; changes include cfg/reset.
    pub(crate) fn bind(cvars: &mut sjk_shell::CvarRegistry) -> Result<Self, sjk_shell::CvarError> {
        let value = Arc::new(AtomicI64::new(0));
        let changed = Arc::clone(&value);
        cvars.on_change("cl_timenudge", move |change| {
            if let sjk_shell::CvarValue::Integer(value) = change.current {
                changed.store(value.clamp(-30, 30), Ordering::Relaxed);
            }
        })?;
        Ok(Self(value))
    }

    /// Effective stock-range nudge in milliseconds, without allocation or locks.
    pub(crate) fn millis(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The adjustment branch taken on receipt; observing it never changes the clock.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TimingEvent {
    /// No new server time (a duplicate timestamp).
    #[default]
    None,
    /// A backwards server timestamp started a different timeline.
    TimelineRestart,
    /// Absolute delta error exceeded 500 ms; restore the interval buffer.
    Reset,
    /// Absolute delta error exceeded 100 ms; halve the difference.
    FastAdjust,
    /// Stock per-snapshot delta drift, in milliseconds (+1 or -2).
    Drift(i8),
}

/// Microsecond realtime/server delta with a monotonic presentation floor.
#[derive(Debug)]
pub(crate) struct SnapshotPresentationClock {
    anchor_server_micros: i64,
    upper_server_time: i64,
    anchor_instant: Instant,
    interval: i64,
    time_nudge_millis: i64,
    last_presented_micros: i64,
    extrapolated_snapshot: bool,
    local: bool,
}

impl SnapshotPresentationClock {
    /// Follow an in-process authority without network interpolation or drift.
    /// Its command clock already advances at wall-clock speed.
    pub(crate) fn follow_local(&mut self, server_time: i32, now: Instant) {
        self.upper_server_time = i64::from(server_time);
        if self.local {
            return;
        }
        self.anchor_server_micros = i64::from(server_time) * 1_000;
        self.anchor_instant = now;
        self.last_presented_micros = self.anchor_server_micros;
        self.local = true;
        self.extrapolated_snapshot = false;
    }

    /// Start one default snapshot interval behind the initial authoritative time.
    pub(crate) fn new(server_time: i32, now: Instant) -> Self {
        let server_time = i64::from(server_time);
        Self {
            anchor_server_micros: (server_time - DEFAULT_INTERVAL_MILLIS) * 1_000,
            upper_server_time: server_time,
            anchor_instant: now,
            interval: DEFAULT_INTERVAL_MILLIS,
            time_nudge_millis: 0,
            last_presented_micros: i64::MIN,
            extrapolated_snapshot: false,
            local: false,
        }
    }

    /// Apply live `cl_timenudge`; positive values add presentation latency.
    pub(crate) fn set_time_nudge_millis(&mut self, millis: i64) {
        self.time_nudge_millis = millis.clamp(-30, 30);
    }

    /// Adjust the delta once per accepted snapshot, independent of frame rate.
    pub(crate) fn receive_snapshot(&mut self, server_time: i32, now: Instant) -> TimingEvent {
        let server_time = i64::from(server_time);
        if server_time < self.upper_server_time {
            // A new map/time domain is the sole exception to monotonic time.
            let nudge = self.time_nudge_millis;
            *self = Self::new(server_time as i32, now);
            self.time_nudge_millis = nudge;
            return TimingEvent::TimelineRestart;
        }
        if server_time == self.upper_server_time {
            return TimingEvent::None;
        }
        self.interval = (server_time - self.upper_server_time).clamp(1, 200);
        self.upper_server_time = server_time;
        let mut micros = self.raw_micros(now);
        // CL_SetCGameTime checks the newly installed snap before adjusting.
        self.observe_extrapolation(micros);
        let error = server_time * 1_000 - micros;
        let event = if error.abs() > RESET_MILLIS * 1_000 {
            micros = (server_time - self.interval) * 1_000;
            TimingEvent::Reset
        } else if error.abs() > FAST_ADJUST_MILLIS * 1_000 {
            micros += error.div_euclid(2);
            TimingEvent::FastAdjust
        } else {
            let drift = if self.extrapolated_snapshot { -2 } else { 1 };
            self.extrapolated_snapshot = false;
            micros += i64::from(drift) * 1_000;
            TimingEvent::Drift(drift)
        };
        self.anchor_server_micros = micros;
        self.anchor_instant = now;
        event
    }

    /// Newest received server timestamp; sampling is allowed to pass it.
    pub(crate) fn upper_server_time(&self) -> i64 {
        self.upper_server_time
    }

    /// Monotonic integer-millisecond presentation time, including time nudge.
    pub(crate) fn sample(&mut self, now: Instant) -> i64 {
        self.sample_micros(now) / 1_000
    }

    fn sample_micros(&mut self, now: Instant) -> i64 {
        let raw = self.raw_micros(now);
        self.observe_extrapolation(raw);
        let nudge = if self.local {
            0
        } else {
            self.time_nudge_millis
        };
        self.last_presented_micros = raw
            .saturating_sub(nudge * 1_000)
            .max(self.last_presented_micros);
        self.last_presented_micros
    }

    fn observe_extrapolation(&mut self, raw_micros: i64) {
        self.extrapolated_snapshot |=
            raw_micros >= (self.upper_server_time - EXTRAPOLATION_MARGIN_MILLIS) * 1_000;
    }

    fn raw_micros(&self, now: Instant) -> i64 {
        let elapsed = now.saturating_duration_since(self.anchor_instant);
        self.anchor_server_micros
            .saturating_add(i64::try_from(elapsed.as_micros()).unwrap_or(i64::MAX))
    }
}

#[cfg(test)]
mod remote_motion_tests {
    use super::*;
    use sjk_runtime::{EntityId, EntityKind, MotionSample, Transform, World, WorldId};
    use std::time::Duration;

    /// Run 20 s of a remote player running at 300 units/s, its snapshots every
    /// `interval` ms after 40 ms plus up to `jitter` ms, through this clock and a
    /// presentation world, drawing every 4 ms. Returns the frames drawn, how many were
    /// drawn before the second newest snapshot, and the largest distance from where
    /// cgame, interpolating its snapshot queue, would draw the player.
    fn run(interval: i64, jitter: i64) -> (u32, u32, f32) {
        let start = Instant::now();
        let mut world = World::new(WorldId::new(1));
        let id = EntityId::new(1);
        let mut clock: Option<SnapshotPresentationClock> = None;
        let mut next_snapshot = 1_000_i64;
        let mut in_flight: Vec<(i64, i64)> = Vec::new();
        let (mut frames, mut behind, mut worst) = (0, 0, 0.0_f32);
        let mut seed = 1_u32;
        for millis in 0..20_000_i64 {
            let now = start + Duration::from_millis(millis as u64);
            while next_snapshot - 1_000 <= millis {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let delay = 40 + i64::from(seed >> 16) % (jitter + 1);
                in_flight.push((next_snapshot, next_snapshot - 1_000 + delay));
                next_snapshot += interval;
            }
            in_flight.sort_by_key(|&(_, arrival)| arrival);
            while in_flight
                .first()
                .is_some_and(|&(_, arrival)| arrival <= millis)
            {
                let (server_time, _) = in_flight.remove(0);
                let transform = Transform {
                    translation: [server_time as f32 * 0.3, 0.0, 0.0],
                    ..Transform::IDENTITY
                };
                world.upsert(
                    id,
                    EntityKind::Actor,
                    MotionSample {
                        time_millis: server_time,
                        transform,
                    },
                );
                match &mut clock {
                    None => clock = Some(SnapshotPresentationClock::new(server_time as i32, now)),
                    Some(clock) => {
                        clock.receive_snapshot(server_time as i32, now);
                    }
                }
            }
            // Past the clock's settling, on the frames drawn.
            let Some(clock) = clock
                .as_mut()
                .filter(|_| millis >= 5_000 && millis % 4 == 0)
            else {
                continue;
            };
            let time = clock.sample(now);
            let entity = world.entity(id).expect("player");
            let newest = entity.current().time_millis;
            let drawn = entity.sample(time).translation[0];
            // cgame holds the newest snapshot once past it, and interpolates otherwise.
            let stock = time.min(newest) as f32 * 0.3;
            frames += 1;
            behind += u32::from(time < newest - interval);
            worst = worst.max((drawn - stock).abs());
        }
        (frames, behind, worst)
    }

    #[test]
    fn remote_players_move_between_the_snapshots_the_stock_clock_draws() {
        // JoF's sv_fps 30, base 20 and 40, with and without arrival jitter.
        for (interval, jitter) in [(33, 0), (33, 6), (33, 15), (50, 0), (25, 0)] {
            let (frames, behind, worst) = run(interval, jitter);
            // The stock clock draws a good share of frames before the second newest
            // snapshot; those held it with only two kept (up to 4.5 units off).
            assert!(
                behind * 20 > frames,
                "{interval}/{jitter}: {behind} of {frames}"
            );
            assert!(worst < 0.01, "{interval}/{jitter}: {worst} units off");
        }
    }
}
