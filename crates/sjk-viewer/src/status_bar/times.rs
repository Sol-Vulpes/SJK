//! The bars' times, written without allocating: the local time on the 24-hour clock
//! (`21:07`) and the date day first (`16/11/2026`, as SJK writes dates) on the bottom
//! bar, and the match's time left on the top bar (`12:04`, `1:02:09` past an hour),
//! worked out as the match timer does from the level's start and the server's
//! `timelimit` (`hud/family.rs`), none when the server sets no limit.

use sjk_protocol::GameState;
use std::fmt;

/// The local time, hours and minutes on the 24-hour clock.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Clock {
    pub(crate) hour: u8,
    pub(crate) minute: u8,
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour % 24, self.minute % 60)
    }
}

/// A calendar date, written day first.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Date {
    pub(crate) day: u8,
    pub(crate) month: u8,
    pub(crate) year: u16,
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:02}/{:04}", self.day, self.month, self.year)
    }
}

/// The local time and date `time` reads.
pub(crate) fn split(time: sjk_shell::local_time::LocalTime) -> (Clock, Date) {
    (
        Clock {
            hour: time.hour,
            minute: time.minute,
        },
        Date {
            day: time.day,
            month: time.month,
            year: u16::try_from(time.year.clamp(0, 9999)).unwrap_or(0),
        },
    )
}

/// Whole seconds of match time left, written as minutes and seconds (`12:04`), with
/// the hours before them past an hour (`1:02:09`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Left(pub(crate) u32);

impl Left {
    /// Under a minute left: the bar says it in ember.
    pub(crate) fn closing(self) -> bool {
        self.0 < 60
    }
}

impl fmt::Display for Left {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (hours, minutes, seconds) = (self.0 / 3600, self.0 / 60 % 60, self.0 % 60);
        if hours > 0 {
            write!(f, "{hours}:{minutes:02}:{seconds:02}")
        } else {
            write!(f, "{minutes}:{seconds:02}")
        }
    }
}

/// `CS_LEVEL_START_TIME`.
const LEVEL_START: usize = 21;

/// The match time left at server time `now`, in whole seconds rounded up (as a clock
/// counting down shows the second it is in): from the level's start and the server's
/// `timelimit` in minutes. `None` without a limit; held at zero once it has run out
/// (overtime, the end of the match on its way).
pub(crate) fn left(game: &GameState, now: i32) -> Option<Left> {
    let limit = info_number(game.config_string(0)?, b"timelimit").filter(|limit| *limit > 0)?;
    let start = game
        .config_string(LEVEL_START)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| text.trim().parse::<i64>().ok())
        .unwrap_or(0);
    Some(left_of(limit, start, i64::from(now)))
}

/// [`left`] from its numbers: `limit` minutes, the level started at `start` and the
/// server's time `now`, in milliseconds.
pub(crate) fn left_of(limit: i64, start: i64, now: i64) -> Left {
    let elapsed = (now - start).max(0);
    let remaining = (limit.saturating_mul(60_000) - elapsed).max(0);
    Left(u32::try_from((remaining + 999) / 1000).unwrap_or(u32::MAX))
}

/// The value of `key` in the info string `info` (`\key\value\...`), compared without
/// regard to case, without allocating.
pub(crate) fn info_value<'a>(info: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let body = info.strip_prefix(b"\\").unwrap_or(info);
    let mut fields = body.split(|byte| *byte == b'\\');
    while let (Some(name), Some(value)) = (fields.next(), fields.next()) {
        if name.eq_ignore_ascii_case(key) {
            return Some(value);
        }
    }
    None
}

/// [`info_value`] read as a whole number.
pub(crate) fn info_number(info: &[u8], key: &[u8]) -> Option<i64> {
    std::str::from_utf8(info_value(info, key)?)
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_runs_on_24_hours_and_the_date_day_first() {
        let (clock, date) = split(sjk_shell::local_time::LocalTime {
            year: 2026,
            month: 11,
            day: 6,
            weekday: 5,
            hour: 21,
            minute: 7,
            second: 59,
        });
        assert_eq!(clock.to_string(), "21:07");
        assert_eq!(date.to_string(), "06/11/2026");
        assert_eq!(Clock { hour: 0, minute: 0 }.to_string(), "00:00");
        assert_eq!(Clock { hour: 9, minute: 5 }.to_string(), "09:05");
        assert_eq!(
            Date {
                day: 31,
                month: 12,
                year: 999
            }
            .to_string(),
            "31/12/0999"
        );
    }

    #[test]
    fn time_left_counts_down_from_the_limit() {
        // 20 minutes, 7.4 s in: 19:53 left (the second it is in, rounded up).
        assert_eq!(left_of(20, 1_000, 8_400).to_string(), "19:53");
        assert_eq!(left_of(20, 1_000, 1_000).to_string(), "20:00");
        // Past an hour, with the hours.
        assert_eq!(left_of(90, 0, 1_000).to_string(), "1:29:59");
        assert_eq!(left_of(61, 0, 0).to_string(), "1:01:00");
        // Under a minute, closing; run out, held at zero.
        assert!(left_of(1, 0, 30_000).closing());
        assert!(!left_of(2, 0, 30_000).closing());
        assert_eq!(left_of(1, 0, 600_000), Left(0));
        // A start after now (a restart's clock) counts the whole limit.
        assert_eq!(left_of(5, 9_000, 1_000).to_string(), "5:00");
    }

    #[test]
    fn info_strings_are_read_by_key_whatever_the_case() {
        let info = b"\\sv_hostname\\^4JoF ^7duels\\TimeLimit\\20\\mapname\\mp/duel6";
        assert_eq!(info_value(info, b"mapname"), Some(&b"mp/duel6"[..]));
        assert_eq!(info_number(info, b"timelimit"), Some(20));
        assert_eq!(
            info_value(info, b"sv_hostname"),
            Some(&b"^4JoF ^7duels"[..])
        );
        assert_eq!(info_value(info, b"fraglimit"), None);
        assert_eq!(info_number(b"\\timelimit\\x", b"timelimit"), None);
        assert_eq!(info_value(b"", b"mapname"), None);
    }

    #[test]
    fn no_limit_no_time_left() {
        let mut game = GameState::empty_local(0);
        assert_eq!(left(&game, 5_000), None);
        game.replace_config_string(0, b"\\timelimit\\0\\mapname\\mp/ffa3".to_vec())
            .unwrap();
        assert_eq!(left(&game, 5_000), None);
        game.replace_config_string(0, b"\\timelimit\\10\\mapname\\mp/ffa3".to_vec())
            .unwrap();
        game.replace_config_string(LEVEL_START, b"2000".to_vec())
            .unwrap();
        assert_eq!(left(&game, 62_000), Some(Left(540)));
    }
}
