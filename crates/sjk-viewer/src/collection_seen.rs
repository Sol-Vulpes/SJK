//! What the player has seen in the Collection on this PC (`docs/sjk-ui.md`, Collection,
//! "Marks", New; 15/11/2026, the ideas page's "New on every kind"): a kyber, a card, a tag
//! or a finisher the player holds and has not looked at since they got it is **New**.
//! Holocrons and dormant kybers carry no New: "to open" already says it.
//!
//! - **The list** is local, beside `config.cfg` ([`FILE`]): a [`Thing`] a line, its kind,
//!   its id and, for cards and tags, its finish (`kyber:saber_sun`,
//!   `card:sjk_alpha_000:holo`, `tag:sjk:solar`, `finisher:disintegrate`), and a
//!   `known <kind>` line for each kind whose first holdings were taken as seen.
//! - **The first holdings are seen:** while a kind is not known yet, everything of it the
//!   hub first says the player holds counts as seen, so a player never finds every card
//!   or tag marked New after an update.
//! - **The old list** (`shader-seen.txt`, kybers only, 29/10/2026) is read once when there
//!   is no list yet: its kybers stay seen and the kind known, the other kinds start fresh
//!   ([`migrate`]). The old file is left as it was.
//! - **Seeing:** a thing chosen on its page for [`LOOK_MS`] is seen ([`Watch`];
//!   `crate::shader_marks::Look` for kybers), and each page's "Mark all seen" (N) sees
//!   every New thing of its kind. One the player no longer holds leaves the list, so it is
//!   New again when it comes back.
//!
//! [`news`] is what the pages draw from (rebuilt only when the list or the holdings
//! change, [`revision`]); [`total`] is the count the status bar reads.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// The list's file, beside `config.cfg`.
pub(crate) const FILE: &str = "collection-seen.txt";
/// The kybers-only list it replaces (`crate::shader_marks`, 29/10/2026), read once.
pub(crate) const OLD_FILE: &str = "shader-seen.txt";
/// How long a New thing must stay chosen to count as seen, in milliseconds.
pub(crate) const LOOK_MS: u64 = 1_000;
/// The file's largest size read: far past every thing a line.
const MAX_FILE_BYTES: u64 = 512 * 1024;
/// The longest id a line may carry.
const MAX_ID: usize = 96;

/// A kind of thing that can be New.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Kind {
    /// A saber's blade look ("saber shader" in code).
    Kyber,
    Card,
    Tag,
    Finisher,
}

impl Kind {
    /// Every kind.
    pub(crate) const ALL: [Self; 4] = [Self::Kyber, Self::Card, Self::Tag, Self::Finisher];

    /// Its word in the file.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Kyber => "kyber",
            Self::Card => "card",
            Self::Tag => "tag",
            Self::Finisher => "finisher",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }

    const fn index(self) -> usize {
        self as usize
    }

    /// Whether its things come in finishes (Normal, Holo, Solar).
    const fn finished(self) -> bool {
        matches!(self, Self::Card | Self::Tag)
    }
}

/// One thing that can be New: its kind, its id and its finish (an index into
/// [`crate::cards::FINISHES`]; 0 for the kinds without finishes).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Thing {
    pub(crate) kind: Kind,
    pub(crate) id: String,
    pub(crate) finish: u8,
}

impl Thing {
    pub(crate) fn new(kind: Kind, id: &str, finish: u8) -> Self {
        Self {
            kind,
            id: id.to_owned(),
            finish: if kind.finished() { finish } else { 0 },
        }
    }

    /// Its line in the file: `kind:id`, or `kind:id:finish` for cards and tags.
    pub(crate) fn key(&self) -> String {
        if self.kind.finished() {
            let finish = crate::cards::FINISHES
                .get(usize::from(self.finish))
                .map_or("normal", |finish| finish.id);
            format!("{}:{}:{finish}", self.kind.word(), self.id)
        } else {
            format!("{}:{}", self.kind.word(), self.id)
        }
    }

    /// The thing a line names; `None` for a line that cannot be one.
    fn parse(line: &str) -> Option<Self> {
        let (kind, rest) = line.split_once(':')?;
        let kind = Kind::from_word(kind)?;
        let (id, finish) = if kind.finished() {
            let (id, finish) = rest.rsplit_once(':')?;
            (id, crate::cards::Finish::from_id(finish)?.index as u8)
        } else {
            (rest, 0)
        };
        valid_id(id).then(|| Self::new(kind, id, finish))
    }
}

/// Whether `id` can be an unlock's or a card's id: letters, digits, `_`, `-` and `.`.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

/// The things seen, and the kinds whose first holdings were taken as seen.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Seen {
    things: BTreeSet<Thing>,
    known: [bool; 4],
}

impl Seen {
    /// Whether `kind`'s first holdings were taken: before, nothing of it is New.
    pub(crate) fn known(&self, kind: Kind) -> bool {
        self.known[kind.index()]
    }

    /// Whether `thing` was seen.
    pub(crate) fn contains(&self, thing: &Thing) -> bool {
        self.things.contains(thing)
    }

    /// Whether kyber `id` was seen (`crate::shader_marks`).
    pub(crate) fn has_kyber(&self, id: &str) -> bool {
        self.contains(&Thing::new(Kind::Kyber, id, 0))
    }

    /// See `thing`; whether it was not seen before.
    pub(crate) fn insert(&mut self, thing: Thing) -> bool {
        self.things.insert(thing)
    }

    /// The list a file holds: blank lines and lines that cannot be a thing are left out.
    pub(crate) fn parse(text: &str) -> Self {
        let mut seen = Self::default();
        for line in text.lines().map(str::trim) {
            if let Some(kind) = line.strip_prefix("known ").and_then(Kind::from_word) {
                seen.known[kind.index()] = true;
            } else if let Some(thing) = Thing::parse(line) {
                seen.things.insert(thing);
            }
        }
        seen
    }

    /// The kybers-only list of `shader-seen.txt` (one id a line): its kybers seen, the
    /// kind known, the others still to be taken.
    pub(crate) fn from_old(text: &str) -> Self {
        let mut seen = Self::default();
        seen.known[Kind::Kyber.index()] = true;
        for id in text
            .lines()
            .map(str::trim)
            .filter(|id| valid_id(id) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        {
            seen.things.insert(Thing::new(Kind::Kyber, id, 0));
        }
        seen
    }

    /// The file's text.
    pub(crate) fn to_text(&self) -> String {
        let mut text = String::with_capacity(96 + self.things.len() * 24);
        text.push_str("# SJK: what this PC has seen in the Collection (kind:id[:finish])\n");
        for kind in Kind::ALL.into_iter().filter(|kind| self.known(*kind)) {
            text.push_str("known ");
            text.push_str(kind.word());
            text.push('\n');
        }
        for thing in &self.things {
            text.push_str(&thing.key());
            text.push('\n');
        }
        text
    }

    /// Follow what the player holds now (`held`, the hub's own list): a kind not known yet
    /// takes all it holds as seen and becomes known; a known kind lets go of the things
    /// no longer held, so one that comes back is New again. Whether the list changed.
    pub(crate) fn follow(&mut self, held: &[Thing]) -> bool {
        let mut changed = false;
        for kind in Kind::ALL {
            if !self.known(kind) {
                self.known[kind.index()] = true;
                for thing in held.iter().filter(|thing| thing.kind == kind) {
                    self.things.insert(thing.clone());
                }
                changed = true;
            }
        }
        let held: HashSet<&Thing> = held.iter().collect();
        let before = self.things.len();
        self.things.retain(|thing| held.contains(thing));
        changed || self.things.len() != before
    }
}

/// The things the player holds, from their own hub profile `me`: kybers, finishers and
/// dropped tags (each finish held) from the unlocks, cards (each finish) from the cards.
/// Sealed cards, free tags and GIF tags are left out: none of them can be New.
pub(crate) fn held(me: &sjk_identity::Profile) -> Vec<Thing> {
    let mut things = Vec::with_capacity(me.unlocks.len() + me.cards.len());
    for unlock in me.unlocks.iter().filter(|unlock| unlock.count > 0) {
        if crate::unlockables::blade_skin(&unlock.id).is_some() {
            things.push(Thing::new(Kind::Kyber, &unlock.id, 0));
        } else if crate::finishers::get(&unlock.id).is_some() {
            things.push(Thing::new(Kind::Finisher, &unlock.id, 0));
        } else if crate::tags::get(&unlock.id).is_some_and(|tag| !tag.free) {
            let normal = unlock
                .count
                .saturating_sub(unlock.holo.saturating_add(unlock.solar));
            for (finish, copies) in [normal, unlock.holo, unlock.solar].into_iter().enumerate() {
                if copies > 0 {
                    things.push(Thing::new(Kind::Tag, &unlock.id, finish as u8));
                }
            }
        }
    }
    for stack in me.cards.iter().filter(|stack| stack.count > 0) {
        if let Some(card) = stack.card.as_deref() {
            let finish = crate::cards::Finish::from_id(&stack.finish).map_or(0, |f| f.index);
            things.push(Thing::new(Kind::Card, card, finish as u8));
        }
    }
    things.sort_unstable();
    things.dedup();
    things
}

/// The New things, by kind: each id with a bit for each New finish (bit 0 for the kinds
/// without finishes). What the pages draw from, without allocating.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct News {
    new: [BTreeMap<String, u8>; 4],
}

impl News {
    /// The things of `held` that `seen` has not seen, in the kinds it knows.
    pub(crate) fn of(held: &[Thing], seen: &Seen) -> Self {
        let mut news = Self::default();
        for thing in held
            .iter()
            .filter(|thing| seen.known(thing.kind) && !seen.contains(thing))
        {
            *news.new[thing.kind.index()]
                .entry(thing.id.clone())
                .or_default() |= 1 << thing.finish.min(7);
        }
        news
    }

    /// Whether anything of `kind` with `id` is New (any finish).
    pub(crate) fn is_new(&self, kind: Kind, id: &str) -> bool {
        self.new[kind.index()].contains_key(id)
    }

    /// Whether `id` of `kind` is New in `finish` (an index into
    /// [`crate::cards::FINISHES`]).
    pub(crate) fn is_new_in(&self, kind: Kind, id: &str, finish: usize) -> bool {
        self.new[kind.index()]
            .get(id)
            .is_some_and(|bits| bits & (1 << finish.min(7)) != 0)
    }

    /// How many things of `kind` are New, each finish apart.
    pub(crate) fn count(&self, kind: Kind) -> usize {
        self.new[kind.index()]
            .values()
            .map(|bits| bits.count_ones() as usize)
            .sum()
    }

    /// How many things are New in all.
    pub(crate) fn total(&self) -> usize {
        Kind::ALL.into_iter().map(|kind| self.count(kind)).sum()
    }

    /// The New things of `kind` with `id` (each finish), to see them.
    pub(crate) fn things_of(&self, kind: Kind, id: &str) -> Vec<Thing> {
        let bits = self.new[kind.index()].get(id).copied().unwrap_or(0);
        (0..8)
            .filter(|finish| bits & (1 << finish) != 0)
            .map(|finish| Thing::new(kind, id, finish))
            .collect()
    }

    /// Every New thing of `kind` ("Mark all seen").
    pub(crate) fn all_of(&self, kind: Kind) -> Vec<Thing> {
        self.new[kind.index()]
            .keys()
            .flat_map(|id| self.things_of(kind, id))
            .collect()
    }

    /// A world shot's or a test's: these things New.
    #[cfg(test)]
    pub(crate) fn shown(things: &[(Kind, &str, u8)]) -> Self {
        let mut news = Self::default();
        for (kind, id, finish) in things {
            *news.new[kind.index()].entry((*id).to_owned()).or_default() |= 1 << finish;
        }
        news
    }
}

/// What a page's choice rests on, for the kinds but kybers: the chosen thing's key (any
/// steady number for it, [`key_of`]), since when, and the New things it shows, read once
/// when the choice comes to it. They are seen once it has stayed chosen [`LOOK_MS`]; a
/// sweep across the page sees nothing, as for kybers.
#[derive(Clone, Debug, Default)]
pub(crate) struct Watch {
    on: Option<u64>,
    since: u64,
    things: Vec<Thing>,
}

impl Watch {
    /// The choice is on `chosen` at `now` (milliseconds on any steady clock); `things`
    /// gives the New things it shows, asked only when the choice changes. The things now
    /// seen, once.
    pub(crate) fn observe(
        &mut self,
        chosen: Option<u64>,
        now: u64,
        things: impl FnOnce() -> Vec<Thing>,
    ) -> Vec<Thing> {
        if chosen != self.on {
            self.on = chosen;
            self.since = now;
            self.things = if chosen.is_some() {
                things()
            } else {
                Vec::new()
            };
            return Vec::new();
        }
        if self.things.is_empty() || now.saturating_sub(self.since) < LOOK_MS {
            return Vec::new();
        }
        std::mem::take(&mut self.things)
    }

    /// The choice left the page at `now`: what it showed, when it had stayed long enough.
    pub(crate) fn leave(&mut self, now: u64) -> Vec<Thing> {
        let stayed = self.on.take().is_some() && now.saturating_sub(self.since) >= LOOK_MS;
        let things = std::mem::take(&mut self.things);
        if stayed { things } else { Vec::new() }
    }
}

/// A steady number for the chosen thing `id` of `kind` ([`Watch`]), without allocating.
pub(crate) fn key_of(kind: Kind, id: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    kind.hash(&mut hasher);
    id.hash(&mut hasher);
    hasher.finish()
}

/// Read the list from `path`: `None` when there is no file yet.
pub(crate) fn load_file(path: &Path) -> std::io::Result<Option<String>> {
    use std::io::Read;
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(MAX_FILE_BYTES).read_to_end(&mut bytes)?;
            Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Write the list to `path` through a temporary file, so a failed write leaves the last
/// list whole.
pub(crate) fn save_file(path: &Path, seen: &Seen) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pending = path.with_extension("txt.pending");
    std::fs::write(&pending, seen.to_text())?;
    std::fs::rename(&pending, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&pending);
    })
}

/// The list in `directory` (the settings folder): [`FILE`]; without it the old
/// [`OLD_FILE`]'s kybers, written to [`FILE`] at once (whether it moved); without
/// either, nothing known.
pub(crate) fn migrate(directory: &Path) -> std::io::Result<(Seen, bool)> {
    if let Some(text) = load_file(&directory.join(FILE))? {
        return Ok((Seen::parse(&text), false));
    }
    let Some(old) = load_file(&directory.join(OLD_FILE))? else {
        return Ok((Seen::default(), false));
    };
    let seen = Seen::from_old(&old);
    save_file(&directory.join(FILE), &seen)?;
    Ok((seen, true))
}

struct State {
    seen: Seen,
    /// What the player held when last followed.
    held: Vec<Thing>,
    news: News,
    /// Where the list is kept, once read.
    path: Option<PathBuf>,
    /// The own profile's revision the holdings were last followed at.
    followed: Option<u64>,
    /// Counts changes, so the pages know to read the list again.
    revision: u64,
}

static STATE: Mutex<State> = Mutex::new(State {
    seen: Seen {
        things: BTreeSet::new(),
        known: [false; 4],
    },
    held: Vec::new(),
    news: News {
        new: [const { BTreeMap::new() }; 4],
    },
    path: None,
    followed: None,
    revision: 0,
});

fn lock() -> MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The list or the holdings changed in `state`: the news again, and the list kept.
fn changed(state: &mut State, save: bool) {
    state.revision += 1;
    state.news = News::of(&state.held, &state.seen);
    if save
        && let Some(path) = &state.path
        && let Err(error) = save_file(path, &state.seen)
    {
        crate::log::progress(format_args!("Collection seen: could not save: {error}"));
    }
}

/// Read the list from `directory` (the settings folder), once.
pub(crate) fn load(directory: &Path) {
    let mut state = lock();
    if state.path.is_some() {
        return;
    }
    match migrate(directory) {
        Ok((seen, moved)) => {
            if moved {
                crate::log::progress(format_args!(
                    "Collection seen: the kybers of {OLD_FILE} moved to {FILE}"
                ));
            }
            state.seen = seen;
            changed(&mut state, false);
        }
        Err(error) => crate::log::progress(format_args!("Collection seen: {error}")),
    }
    state.path = Some(directory.join(FILE));
}

/// Follow the own profile in `snapshot` when it changed ([`Seen::follow`]). Only once the
/// list was read ([`load`]), and never on the debug collectibles' holdings.
pub(crate) fn follow(snapshot: &sjk_identity::Snapshot) {
    let Some(me) = snapshot.me.as_ref() else {
        return;
    };
    let mut state = lock();
    if state.path.is_none() || state.followed == Some(snapshot.me_revision) {
        return;
    }
    state.followed = Some(snapshot.me_revision);
    if me.unlocks.iter().any(crate::debug_collectibles::added) {
        return;
    }
    let held = held(me);
    let save = state.seen.follow(&held);
    if save || held != state.held {
        state.held = held;
        changed(&mut state, save);
    }
}

/// See every one of `things`: they are New no more (a kind not known yet sees nothing).
pub(crate) fn see_all(things: impl IntoIterator<Item = Thing>) {
    let mut state = lock();
    let mut any = false;
    for thing in things {
        if state.seen.known(thing.kind) {
            any |= state.seen.insert(thing);
        }
    }
    if any {
        changed(&mut state, true);
    }
}

/// See `thing`.
pub(crate) fn see(thing: Thing) {
    see_all([thing]);
}

/// Counts changes to the list and the holdings it follows.
pub(crate) fn revision() -> u64 {
    lock().revision
}

/// Read the list in place (keep `read` short).
pub(crate) fn with_seen<R>(read: impl FnOnce(&Seen) -> R) -> R {
    read(&lock().seen)
}

/// The New things now.
pub(crate) fn news() -> News {
    lock().news.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thing(kind: Kind, id: &str, finish: u8) -> Thing {
        Thing::new(kind, id, finish)
    }

    /// Every kind's line: written and read back, finishes for cards and tags only; odd
    /// lines left out.
    #[test]
    fn the_list_round_trips_and_bad_lines_are_left_out() {
        let mut seen = Seen::default();
        seen.known[Kind::Kyber.index()] = true;
        seen.known[Kind::Card.index()] = true;
        seen.insert(thing(Kind::Kyber, "saber_storm", 2));
        seen.insert(thing(Kind::Card, "sjk_alpha_000", 1));
        seen.insert(thing(Kind::Tag, "sjk", 2));
        seen.insert(thing(Kind::Finisher, "disintegrate", 0));
        let text = seen.to_text();
        assert!(text.contains("\nkyber:saber_storm\n"), "{text}");
        assert!(text.contains("\ncard:sjk_alpha_000:holo\n"), "{text}");
        assert!(text.contains("\ntag:sjk:solar\n"), "{text}");
        assert!(text.contains("\nfinisher:disintegrate\n"), "{text}");
        assert!(text.contains("\nknown kyber\nknown card\n"), "{text}");
        assert_eq!(Seen::parse(&text), seen);
        let odd = Seen::parse(
            "kyber:\nshader:saber_storm\ncard:sjk_alpha_000\ncard:x:gold\ntag:../x:holo\n\
             kyber:not an id\nknown nothing\n",
        );
        assert_eq!(odd, Seen::default());
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        assert_eq!(load_file(&path).unwrap(), None, "no file yet");
        save_file(&path, &seen).unwrap();
        assert_eq!(Seen::parse(&load_file(&path).unwrap().unwrap()), seen);
    }

    /// The kybers-only `shader-seen.txt` moves into the new list once: its kybers stay
    /// seen and the kyber kind known, the other kinds still to be taken; the new list
    /// then stands and the old file is left as it was.
    #[test]
    fn the_old_shader_list_moves_over_without_losing_a_kyber() {
        let directory = tempfile::tempdir().unwrap();
        let old = "saber_storm\nsaber_void\nnot an id\n";
        std::fs::write(directory.path().join(OLD_FILE), old).unwrap();
        let (seen, moved) = migrate(directory.path()).unwrap();
        assert!(moved);
        assert!(seen.known(Kind::Kyber));
        assert!(!seen.known(Kind::Card) && !seen.known(Kind::Tag));
        assert!(seen.has_kyber("saber_storm") && seen.has_kyber("saber_void"));
        assert_eq!(seen.things.len(), 2);
        assert!(directory.path().join(FILE).exists());
        assert_eq!(
            std::fs::read_to_string(directory.path().join(OLD_FILE)).unwrap(),
            old
        );
        // Read again: the new list, nothing moved.
        let (again, moved) = migrate(directory.path()).unwrap();
        assert!(!moved);
        assert_eq!(again, seen);
        // A kyber gained since is New; the cards held at the first follow are not.
        let mut seen = again;
        let held = [
            thing(Kind::Kyber, "saber_storm", 0),
            thing(Kind::Kyber, "saber_void", 0),
            thing(Kind::Kyber, "saber_frost", 0),
            thing(Kind::Card, "sjk_alpha_001", 0),
        ];
        assert!(seen.follow(&held));
        let news = News::of(&held, &seen);
        assert!(news.is_new(Kind::Kyber, "saber_frost"));
        assert!(!news.is_new(Kind::Kyber, "saber_storm"));
        assert_eq!(news.count(Kind::Card), 0);
        // Neither file: nothing known, so nothing New until the first holdings.
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(migrate(empty.path()).unwrap(), (Seen::default(), false));
    }

    /// The first holdings of a kind not known are seen whole; after, a gain is New until
    /// seen, and one let go leaves the list (New when it comes back).
    #[test]
    fn the_first_holdings_are_seen_and_later_gains_are_new() {
        let mut seen = Seen::default();
        let first = [
            thing(Kind::Card, "sjk_alpha_000", 0),
            thing(Kind::Tag, "sjk", 1),
            thing(Kind::Finisher, "disintegrate", 0),
        ];
        assert!(seen.follow(&first));
        assert!(Kind::ALL.iter().all(|kind| seen.known(*kind)));
        assert_eq!(News::of(&first, &seen).total(), 0);
        assert!(!seen.follow(&first), "nothing changed");
        let more = [
            thing(Kind::Card, "sjk_alpha_000", 0),
            thing(Kind::Card, "sjk_alpha_000", 2),
            thing(Kind::Tag, "sjk", 1),
            thing(Kind::Finisher, "disintegrate", 0),
        ];
        assert!(!seen.follow(&more));
        let news = News::of(&more, &seen);
        assert!(news.is_new(Kind::Card, "sjk_alpha_000"));
        assert!(news.is_new_in(Kind::Card, "sjk_alpha_000", 2));
        assert!(!news.is_new_in(Kind::Card, "sjk_alpha_000", 0));
        assert!(seen.insert(thing(Kind::Card, "sjk_alpha_000", 2)));
        assert_eq!(News::of(&more, &seen).total(), 0);
        // Let go: off the list, New when it comes back.
        assert!(seen.follow(&[thing(Kind::Tag, "sjk", 1)]));
        assert!(!seen.contains(&thing(Kind::Finisher, "disintegrate", 0)));
        assert_eq!(News::of(&more, &seen).count(Kind::Finisher), 1);
    }

    /// Each kind counts on its own, each finish apart; Mark all seen sees one kind only.
    #[test]
    fn each_kind_counts_its_own_and_mark_all_seen_clears_one_kind() {
        let mut seen = Seen::default();
        seen.follow(&[]);
        let held = [
            thing(Kind::Kyber, "saber_sun", 0),
            thing(Kind::Kyber, "saber_void", 0),
            thing(Kind::Card, "sjk_alpha_002", 1),
            thing(Kind::Card, "sjk_alpha_002", 2),
            thing(Kind::Tag, "sjk", 2),
            thing(Kind::Finisher, "disintegrate", 0),
        ];
        let news = News::of(&held, &seen);
        assert_eq!(news.count(Kind::Kyber), 2);
        assert_eq!(news.count(Kind::Card), 2);
        assert_eq!(news.count(Kind::Tag), 1);
        assert_eq!(news.count(Kind::Finisher), 1);
        assert_eq!(news.total(), 6);
        assert_eq!(news.things_of(Kind::Card, "sjk_alpha_002").len(), 2);
        for thing in news.all_of(Kind::Card) {
            seen.insert(thing);
        }
        let after = News::of(&held, &seen);
        assert_eq!(after.count(Kind::Card), 0);
        assert_eq!(after.count(Kind::Kyber), 2);
        assert_eq!(after.total(), 4);
    }

    /// The profile's holdings: kybers, finishers, dropped tags in each finish held and
    /// revealed cards in theirs; free tags and sealed cards are never New.
    #[test]
    fn holdings_are_read_from_the_profile() {
        let skin = crate::unlockables::blade_skins().next().unwrap();
        let dropped = crate::tags::CATALOGUE.iter().find(|tag| !tag.free).unwrap();
        let free = crate::tags::CATALOGUE.iter().find(|tag| tag.free).unwrap();
        let finisher = &crate::finishers::CATALOGUE[0];
        let unlock = |id: &str, count, holo, solar| sjk_identity::Unlock {
            id: id.to_owned(),
            granted: 1,
            note: String::new(),
            count,
            medal: None,
            card: None,
            holo,
            solar,
        };
        let stack = |card: Option<&str>, finish: &str| sjk_identity::CardStack {
            card: card.map(str::to_owned),
            rarity: "rare".to_owned(),
            finish: finish.to_owned(),
            edition: String::new(),
            count: 1,
            first: 0,
            last: 0,
        };
        let mut me = crate::console::profile_panel::tests::me("");
        me.unlocks = vec![
            unlock(skin.id, 2, 0, 0),
            unlock(finisher.id, 1, 0, 0),
            unlock(dropped.id, 3, 0, 2),
            unlock(free.id, 1, 0, 0),
        ];
        me.cards = vec![stack(Some("sjk_alpha_000"), "holo"), stack(None, "normal")];
        let things = held(&me);
        assert!(things.contains(&thing(Kind::Kyber, skin.id, 0)));
        assert!(things.contains(&thing(Kind::Finisher, finisher.id, 0)));
        assert!(things.contains(&thing(Kind::Tag, dropped.id, 0)));
        assert!(things.contains(&thing(Kind::Tag, dropped.id, 2)));
        assert!(!things.contains(&thing(Kind::Tag, dropped.id, 1)));
        assert!(things.contains(&thing(Kind::Card, "sjk_alpha_000", 1)));
        assert!(!things.iter().any(|thing| thing.id == free.id));
        assert_eq!(things.len(), 5, "{things:?}");
    }

    /// A New thing is seen once it has stayed chosen a second; a sweep sees nothing, and
    /// leaving sooner sees nothing.
    #[test]
    fn a_watched_thing_is_seen_after_a_second_chosen() {
        let card = || vec![thing(Kind::Card, "sjk_alpha_000", 1)];
        let mut watch = Watch::default();
        let a = key_of(Kind::Card, "sjk_alpha_000");
        let b = key_of(Kind::Card, "sjk_alpha_001");
        assert_ne!(a, b);
        assert!(watch.observe(Some(a), 0, card).is_empty());
        assert!(watch.observe(Some(a), 999, card).is_empty());
        assert_eq!(watch.observe(Some(a), 1_000, card), card());
        assert!(watch.observe(Some(a), 2_000, card).is_empty(), "once");
        // A sweep: each for 150 ms.
        let mut now = 5_000;
        for _ in 0..10 {
            for key in [a, b] {
                now += 150;
                assert!(watch.observe(Some(key), now, card).is_empty());
            }
        }
        // Leaving: seen only after a second.
        watch.observe(Some(b), 10_000, card);
        assert!(watch.leave(10_400).is_empty());
        watch.observe(Some(b), 11_000, card);
        assert_eq!(watch.leave(12_000), card());
        assert!(watch.leave(13_000).is_empty());
    }
}
