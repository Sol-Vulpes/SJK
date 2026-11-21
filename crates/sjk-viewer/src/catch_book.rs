//! The Fishing and Mining toys' log books (Slash, 15/11/2026, after adwè's #ideas post),
//! read only since the toys were removed (20/11/2026): the Collection's Log book page
//! (`collection_log_book.rs`) shows them for a player whose logs hold a catch.
//!
//! - **The books.** Every fish and junk, or every ore and rubble, one to a cell, the
//!   ones never caught as "???", the others with their count and the day the first came
//!   up (a `first <id> <yyyymmdd>` line of the toy's log; catches from before it have no
//!   day), the rare ones ([`crate::fishing::is_rare`], [`crate::mining::is_rare`]) in
//!   gold. [`load`] reads both logs from the profile folder as the Collection opens.
//! - **Sharing.** Each book's Share button says its rarest catch in the SJK chat as the
//!   player's own message ([`crate::player_identity::chat_sjk_only`]: the hub's chat
//!   rules and quota hold, and nothing goes to the game server's chat), at most once in
//!   [`BRAG_EVERY`]. Nothing is ever said for the player unasked.
//!
//! Everything here is the player's own log and words: the hub neither reads nor pays
//! for any of it.

use std::time::{Duration, Instant};

/// The least time between two shares of a toy's catch.
pub(crate) const BRAG_EVERY: Duration = Duration::from_secs(60);

/// The log's line of the day a catch first came up: `first <id> <yyyymmdd>`.
pub(crate) const FIRST: &str = "first";

/// A kept day as the card writes it: `dd/mm/yyyy`; `None` for one that is no day.
pub(crate) fn day_text(day: u32) -> Option<String> {
    let (year, month, date) = (day / 10_000, day / 100 % 100, day % 100);
    ((1..=12).contains(&month) && (1..=31).contains(&date) && year >= 2_000)
        .then(|| format!("{date:02}/{month:02}/{year}"))
}

/// Read a `first <id> <yyyymmdd>` line's day; `None` for one that is no day.
pub(crate) fn parse_day(text: &str) -> Option<u32> {
    text.parse::<u32>()
        .ok()
        .filter(|day| day_text(*day).is_some())
}

/// `n` as an ordinal: 1st, 2nd, 3rd, 4th, 11th, 21st...
pub(crate) fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// A catch's name without its article ("a Kyber minnow" is Kyber minnow, "the Heart of
/// the Mountain" Heart of the Mountain); what follows a comma stays ("Old Whiskers, the
/// great catfish").
pub(crate) fn bare_name(name: &str) -> &str {
    ["a ", "an ", "the "]
        .iter()
        .find_map(|article| name.strip_prefix(article))
        .unwrap_or(name)
}

/// The SJK chat line sharing a catch: `verb` (caught, dug out) the catch `name`, the
/// `count`th of it, with the log's kinds caught of all there are.
pub(crate) fn brag_line(verb: &str, name: &str, count: u32, kinds: (usize, usize)) -> String {
    let (caught, all) = kinds;
    let which = if count <= 1 {
        "my first!".to_owned()
    } else {
        format!("my {}", ordinal(count))
    };
    format!(
        "{verb} {} ({which}) - {caught} of {all} kinds in my log",
        bare_name(name)
    )
}

/// One cell of the log book.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Entry {
    /// Its name, without the article; "???" when never caught.
    pub(crate) name: String,
    /// How many were caught (0: never).
    pub(crate) count: u32,
    /// The day the first came up (`dd/mm/yyyy`), when the log knows it.
    pub(crate) first: Option<String>,
    /// A rare catch's cell, drawn in gold.
    pub(crate) rare: bool,
}

impl Entry {
    /// The cell of a catch named `name`, caught `count` times, the first on `first`.
    pub(crate) fn of(name: &str, count: u32, first: Option<u32>, rare: bool) -> Self {
        if count == 0 {
            return Self {
                name: "???".to_owned(),
                count,
                first: None,
                rare,
            };
        }
        Self {
            name: bare_name(name).split(',').next().unwrap_or(name).to_owned(),
            count,
            first: first.and_then(day_text),
            rare,
        }
    }

    /// The cell's second words: its count and first day ("x3 · 12/11/2026"), nothing
    /// for one never caught.
    pub(crate) fn note(&self) -> String {
        match (&self.first, self.count) {
            (_, 0) => String::new(),
            (Some(day), count) => format!("x{count}  {day}"),
            (None, count) => format!("x{count}"),
        }
    }
}

/// The log's first days, kept beside its counts: one `(id, yyyymmdd)` a catch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Firsts(Vec<(String, u32)>);

impl Firsts {
    /// Read a `first <id> <day>` line's words, `id` and `day`.
    pub(crate) fn read(&mut self, id: &str, day: &str) {
        if let Some(day) = parse_day(day) {
            self.0.retain(|(known, _)| known != id);
            self.0.push((id.to_owned(), day));
        }
    }

    /// The first day of `id`, when known.
    pub(crate) fn of(&self, id: &str) -> Option<u32> {
        self.0
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, day)| *day)
    }
}

/// When a toy's catch was last shared, and the last rare catch of the session.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Brag {
    pub(crate) at: Option<Instant>,
}

impl Brag {
    /// Whether a share may go now (none in [`BRAG_EVERY`]); notes it when it may.
    pub(crate) fn take(&mut self, now: Instant) -> bool {
        if self
            .at
            .is_some_and(|at| now.duration_since(at) < BRAG_EVERY)
        {
            return false;
        }
        self.at = Some(now);
        true
    }
}

/// Send `line` to the SJK chat as the player's own message, and say what came of it on
/// the centre of the screen: `None` when it went, else why not.
pub(crate) fn share(line: String) -> Option<&'static str> {
    if crate::player_identity::chat_sjk_only(line) {
        None
    } else {
        Some("SJK online is off: the SJK chat shares your catch.")
    }
}

/// One toy's log book as the Collection's Log book page shows it (since the Fishing and
/// Mining toys were removed, 20/11/2026: their logs stay the players' own record).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Book {
    /// Its head: "Fishing", "Mining".
    pub(crate) title: &'static str,
    /// What its second part holds: "junk", "rubble".
    pub(crate) goods: &'static str,
    /// The fish (or ores) in the table's order, then the junk (or rubble).
    pub(crate) book: Vec<Entry>,
    /// The kinds of the first part: the rest is junk or rubble.
    pub(crate) book_kinds: usize,
    /// What its Share button says in the SJK chat (`None`: nothing worth sharing).
    pub(crate) brag: Option<String>,
}

impl Book {
    /// Whether anything of it was ever caught.
    pub(crate) fn any(&self) -> bool {
        self.book.iter().any(|entry| entry.count > 0)
    }
}

/// Both log books.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Books {
    pub(crate) fishing: Book,
    pub(crate) mining: Book,
}

impl Books {
    /// The books of the log files' texts.
    pub(crate) fn of(fishing: &str, mining: &str) -> Self {
        Self {
            fishing: fishing_book(&crate::fishing::Log::parse(fishing)),
            mining: mining_book(&crate::mining::Log::parse(mining)),
        }
    }

    /// Whether either holds a catch: only then has the Collection a Log book.
    pub(crate) fn any(&self) -> bool {
        self.fishing.any() || self.mining.any()
    }
}

/// Whether the Collection shows its Log book ([`load`] found a catch).
static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Read both logs from the profile `directory`, and note whether the Log book shows.
pub(crate) fn load(directory: &std::path::Path) -> Books {
    let read = |file| std::fs::read_to_string(directory.join(file)).unwrap_or_default();
    let books = Books::of(
        &read(crate::fishing::LOG_FILE),
        &read(crate::mining::LOG_FILE),
    );
    SHOWN.store(books.any(), std::sync::atomic::Ordering::Release);
    books
}

/// Whether the Collection has its Log book: a tile in the hall, a tab on the row.
pub(crate) fn shown() -> bool {
    SHOWN.load(std::sync::atomic::Ordering::Acquire)
}

/// The fish log's book.
fn fishing_book(log: &crate::fishing::Log) -> Book {
    use crate::fishing::{CATCHES, Catch, Kind};
    let entry = |c: &Catch| {
        Entry::of(
            c.name,
            log.count(c.id),
            log.first(c.id),
            crate::fishing::is_rare(c),
        )
    };
    let mut book: Vec<_> = CATCHES
        .iter()
        .filter(|c| c.kind == Kind::Fish)
        .map(entry)
        .collect();
    let book_kinds = book.len();
    book.extend(CATCHES.iter().filter(|c| c.kind == Kind::Junk).map(entry));
    Book {
        title: "Fishing",
        goods: "junk",
        book,
        book_kinds,
        brag: log
            .rarest()
            .map(|(catch, count)| brag_line("Caught", catch.name, count, log.species())),
    }
}

/// The mining log's book.
fn mining_book(log: &crate::mining::Log) -> Book {
    use crate::mining::{FINDS, Find, Kind};
    let entry = |f: &Find| {
        Entry::of(
            f.name,
            log.count(f.id),
            log.first(f.id),
            crate::mining::is_rare(f),
        )
    };
    let mut book: Vec<_> = FINDS
        .iter()
        .filter(|f| f.kind == Kind::Ore)
        .map(entry)
        .collect();
    let book_kinds = book.len();
    book.extend(FINDS.iter().filter(|f| f.kind == Kind::Rubble).map(entry));
    Book {
        title: "Mining",
        goods: "rubble",
        book,
        book_kinds,
        brag: log
            .rarest()
            .map(|(find, count)| brag_line("Dug out", find.name, count, log.kinds())),
    }
}

/// The Log book's Share: the last share's time, for [`BRAG_EVERY`].
static SHARED: std::sync::Mutex<Brag> = std::sync::Mutex::new(Brag { at: None });

/// Share `book`'s line in the SJK chat (its Share button): what the centre of the
/// screen says, if anything (nothing to share, too soon, SJK online off).
pub(crate) fn share_book(book: &Book, now: Instant) -> Option<&'static str> {
    let Some(line) = book.brag.clone() else {
        return Some("Nothing to share yet.");
    };
    let may = SHARED.lock().map_or(false, |mut brag| brag.take(now));
    if !may {
        return Some("You shared a catch a moment ago.");
    }
    share(line)
}

impl crate::GpuState {
    /// The Log book's Share button: the fish (`mining` false) or mining book's line in
    /// the SJK chat, at most once in [`BRAG_EVERY`]; what came of it on the centre of the
    /// screen when it did not go.
    pub(crate) fn share_log_book(&mut self, mining: bool) {
        let Some(directory) = self
            .console
            .as_ref()
            .map(|console| console.config_directory().to_owned())
        else {
            return;
        };
        let books = load(&directory);
        let book = if mining {
            &books.mining
        } else {
            &books.fishing
        };
        if let Some(said) = share_book(book, Instant::now()) {
            self.chat.receive(
                sjk_client::ServerEventKind::CenterPrint,
                said.to_owned(),
                None,
                Instant::now(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_kept_and_written_day_first() {
        assert_eq!(day_text(20261105).as_deref(), Some("05/11/2026"));
        assert_eq!(parse_day("20261105"), Some(20261105));
        assert_eq!(parse_day("2026115"), None, "no month 1 day 15 of year 202");
        assert_eq!(parse_day("20261341"), None);
        assert_eq!(parse_day("x"), None);
    }

    #[test]
    fn ordinals_read_as_english() {
        let words: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111]
            .into_iter()
            .map(ordinal)
            .collect();
        assert_eq!(
            words,
            [
                "1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "101st",
                "111th"
            ]
        );
    }

    #[test]
    fn a_share_names_the_catch_and_the_log() {
        assert_eq!(
            brag_line("Caught", "Old Whiskers, the great catfish", 1, (9, 12)),
            "Caught Old Whiskers, the great catfish (my first!) - 9 of 12 kinds in my log"
        );
        assert_eq!(
            brag_line("Dug out", "the Heart of the Mountain", 3, (9, 9)),
            "Dug out Heart of the Mountain (my 3rd) - 9 of 9 kinds in my log"
        );
        // The hub's chat takes it: one line, its alphabet, under 300 characters.
        let line = brag_line("Caught", "a Kyber minnow", 22, (12, 12));
        assert!(sjk_identity::chat::check(&line).is_ok(), "{line}");
    }

    #[test]
    fn a_cell_hides_what_was_never_caught() {
        let never = Entry::of("a Ghost ray", 0, Some(20261101), true);
        assert_eq!(never.name, "???");
        assert!(never.note().is_empty() && never.first.is_none());
        let caught = Entry::of("Old Whiskers, the great catfish", 2, Some(20261112), true);
        assert_eq!(caught.name, "Old Whiskers");
        assert_eq!(caught.note(), "x2  12/11/2026");
        assert_eq!(Entry::of("a Sun carp", 4, None, false).note(), "x4");
    }

    #[test]
    fn the_first_day_is_read_once_a_catch() {
        let mut firsts = Firsts::default();
        firsts.read("moon_eel", "20261102");
        firsts.read("moon_eel", "nonsense");
        firsts.read("sun_carp", "20261101");
        firsts.read("sun_carp", "20261115");
        assert_eq!(firsts.of("moon_eel"), Some(20261102));
        assert_eq!(
            firsts.of("sun_carp"),
            Some(20261115),
            "the log's last line wins"
        );
    }

    #[test]
    fn a_share_waits_a_minute_after_the_last() {
        let mut brag = Brag::default();
        let now = Instant::now();
        assert!(brag.take(now));
        assert!(!brag.take(now + Duration::from_secs(30)));
        assert!(brag.take(now + BRAG_EVERY));
    }
}
