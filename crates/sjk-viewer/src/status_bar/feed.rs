//! The top bar's feed of who joins and leaves (Sol's request, 16/11/2026): the last
//! [`SHOWN`] arrivals and departures, each a name in its own colours with "joined" or
//! "left", shown for [`HOLD`] then fading out over [`FADE`].
//!
//! It follows the players' config strings (`CS_PLAYERS`), as the scoreboard and the
//! nameplates know players, not the servers' print lines, whose words differ from mod to
//! mod: a slot that fills is a join, one that empties a leave, one whose name changes a
//! rename (not said). The slots are read with the bar's other inputs, four times a
//! second, whether or not the bar shows, so a join under the menus is still told. A new
//! match ([`Feed::observe`]'s key: a map change or another server, both a new game state)
//! reads the slots again without telling anyone, as everyone in it was there first.

use super::Line;
use std::time::{Duration, Instant};

/// The bytes of a name kept (a JA name is at most 36 visible characters; its colour
/// codes take room too).
pub(crate) const NAME_BYTES: usize = 64;
/// A player's name, colour codes kept.
pub(crate) type Name = Line<NAME_BYTES>;
/// Client slots followed.
pub(crate) const SLOTS: usize = sjk_protocol::PLAYER_CAPACITY;
/// How many joins and leaves show at once, the newest first.
pub(crate) const SHOWN: usize = 3;
/// How long an entry shows at full strength, then how long it takes to fade out.
pub(crate) const HOLD: Duration = Duration::from_secs(6);
pub(crate) const FADE: Duration = Duration::from_millis(1200);
/// The fade's steps: an entry's strength is a whole number of them, so the bar is laid
/// out again at most this many times while one fades rather than every frame.
pub(crate) const STEPS: u8 = 16;

/// One join or leave as the bar draws it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Shown {
    pub(crate) name: Name,
    pub(crate) joined: bool,
    /// Its strength in [`STEPS`]ths: [`STEPS`] while it holds, down to 1 as it fades.
    pub(crate) step: u8,
}

impl Shown {
    /// Its strength, 0 to 1.
    pub(crate) fn alpha(self) -> f32 {
        f32::from(self.step) / f32::from(STEPS)
    }
}

/// A join or leave and when it was seen.
#[derive(Clone, Copy, Debug)]
struct Entry {
    name: Name,
    joined: bool,
    at: Instant,
}

/// The slots as last read and the joins and leaves told.
pub(crate) struct Feed {
    /// The match the slots were read in; `None` before the first read.
    key: Option<u64>,
    /// Each slot's name as last read, empty for nobody.
    slots: Box<[Name; SLOTS]>,
    /// The newest first.
    entries: [Option<Entry>; SHOWN],
}

impl Default for Feed {
    fn default() -> Self {
        Self {
            key: None,
            slots: Box::new([Name::default(); SLOTS]),
            entries: [None; SHOWN],
        }
    }
}

impl Feed {
    /// Read the slots of match `key` at `now`: `name` gives slot `index`'s name, `None`
    /// for nobody. A slot filled since the last read is a join, one emptied a leave.
    pub(crate) fn observe(
        &mut self,
        key: u64,
        now: Instant,
        mut name: impl FnMut(usize) -> Option<Name>,
    ) {
        let fresh = self.key != Some(key);
        if fresh {
            self.key = Some(key);
            self.entries = [None; SHOWN];
        }
        for index in 0..SLOTS {
            let new = name(index).unwrap_or_default();
            let old = self.slots[index];
            if !fresh {
                match (old.is_empty(), new.is_empty()) {
                    (true, false) => self.tell(new, true, now),
                    (false, true) => self.tell(old, false, now),
                    _ => {}
                }
            }
            self.slots[index] = new;
        }
    }

    /// Out of a match: the next one is read afresh.
    pub(crate) fn forget(&mut self) {
        self.key = None;
        self.entries = [None; SHOWN];
    }

    fn tell(&mut self, name: Name, joined: bool, now: Instant) {
        self.entries.rotate_right(1);
        self.entries[0] = Some(Entry {
            name,
            joined,
            at: now,
        });
    }

    /// What shows at `now`, the newest first; an entry past its fade is gone.
    pub(crate) fn shown(&self, now: Instant) -> [Option<Shown>; SHOWN] {
        self.entries.map(|entry| {
            let entry = entry?;
            let step = step(now.saturating_duration_since(entry.at))?;
            Some(Shown {
                name: entry.name,
                joined: entry.joined,
                step,
            })
        })
    }
}

/// An entry's strength `age` after it came, in [`STEPS`]ths; `None` once faded.
fn step(age: Duration) -> Option<u8> {
    let Some(fading) = age.checked_sub(HOLD) else {
        return Some(STEPS);
    };
    if fading >= FADE {
        return None;
    }
    let left = 1.0 - fading.as_secs_f32() / FADE.as_secs_f32();
    // Rounded up, so it never reads 0 while still shown.
    Some(((left * f32::from(STEPS)).ceil() as u8).clamp(1, STEPS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names<'a>(list: &'a [(usize, &'a str)]) -> impl FnMut(usize) -> Option<Name> + 'a {
        move |slot| {
            list.iter()
                .find(|(at, _)| *at == slot)
                .map(|(_, name)| Name::from(*name))
        }
    }

    fn said(feed: &Feed, now: Instant) -> Vec<(String, bool)> {
        feed.shown(now)
            .iter()
            .flatten()
            .map(|shown| (shown.name.as_str().to_owned(), shown.joined))
            .collect()
    }

    /// The first read of a match tells no one; then a slot filling is a join, one
    /// emptying a leave, a rename nothing; the newest first, three at most.
    #[test]
    fn joins_and_leaves_are_told_from_the_slots() {
        let start = Instant::now();
        let mut feed = Feed::default();
        feed.observe(7, start, names(&[(0, "^1Sol"), (3, "Padawan")]));
        assert!(said(&feed, start).is_empty(), "everyone was there first");
        feed.observe(
            7,
            start,
            names(&[(0, "^1Sol"), (3, "Padawan"), (5, "Kyle")]),
        );
        assert_eq!(said(&feed, start), [("Kyle".to_owned(), true)]);
        feed.observe(7, start, names(&[(0, "^1Sol"), (5, "Kyle")]));
        assert_eq!(
            said(&feed, start),
            [("Padawan".to_owned(), false), ("Kyle".to_owned(), true)]
        );
        // A rename is no join.
        feed.observe(7, start, names(&[(0, "^1Sol^7!"), (5, "Kyle")]));
        assert_eq!(said(&feed, start).len(), 2);
        feed.observe(7, start, names(&[(0, "^1Sol^7!"), (5, "Kyle"), (9, "Jan")]));
        feed.observe(
            7,
            start,
            names(&[(0, "^1Sol^7!"), (5, "Kyle"), (9, "Jan"), (10, "Bot")]),
        );
        assert_eq!(
            said(&feed, start),
            [
                ("Bot".to_owned(), true),
                ("Jan".to_owned(), true),
                ("Padawan".to_owned(), false)
            ],
            "the oldest goes"
        );
        // A new match reads the slots again, telling no one, and clears the feed.
        feed.observe(8, start, names(&[(1, "Luke")]));
        assert!(said(&feed, start).is_empty());
        feed.forget();
        feed.observe(8, start, names(&[(1, "Luke"), (2, "Leia")]));
        assert!(said(&feed, start).is_empty(), "forgotten: read afresh");
    }

    /// An entry holds at full strength, then fades in steps and goes.
    #[test]
    fn entries_hold_then_fade_out() {
        assert_eq!(step(Duration::ZERO), Some(STEPS));
        assert_eq!(step(HOLD), Some(STEPS));
        let half = step(HOLD + FADE / 2).unwrap();
        assert!(half > 1 && half < STEPS, "{half}");
        assert_eq!(step(HOLD + FADE - Duration::from_millis(1)), Some(1));
        assert_eq!(step(HOLD + FADE), None);
        let start = Instant::now();
        let mut feed = Feed::default();
        feed.observe(1, start, names(&[]));
        feed.observe(1, start, names(&[(4, "Kyle")]));
        let later = start + HOLD + FADE / 2;
        let shown = feed.shown(later)[0].unwrap();
        assert!(shown.alpha() > 0.0 && shown.alpha() < 1.0);
        assert_eq!(feed.shown(start + HOLD + FADE), [None; SHOWN]);
    }
}
