//! The holocron drops' banners (`docs/holocrons.md`, "The banner"): the first time the
//! client sees a holocron (or a dormant kyber) in the player's own profile, it tells of it
//! once with a notification banner at the top of the screen ([`crate::unlock_toast`]), in
//! its tier's colour, with its name, where it came from and where it waits: the
//! Collection's Holocrons page. Several waiting together fold into one banner that counts
//! them ("5 new holocrons", in the rarest one's colour).
//!
//! Until 16/11/2026 a holocron showed once more, large, on a darkened screen when the main
//! menu or the game menu opened, with a ceremony and Next or Close; Sol: "remove the big
//! notification of the holocron drops (when you press ESC), we just remove it". Nothing
//! waits for a menu now: the banner shows over play and over the menus alike, never takes
//! input and blocks nothing.
//!
//! - What was shown is kept in `holocrons_seen.txt` ([`crate::holocrons::seen`]): the
//!   highest holocron number shown, written as its banner begins ([`HolocronBanners::shown`]),
//!   so each shows once per identity even when its banner waits (in a private duel, under
//!   Tactical) and the game closes first. An install that already held holocrons when it
//!   first read its profile tells of the newest 20, on one banner.
//! - `cl_holocronNotifications` ([`NOTIFICATIONS_CVAR`]): 1 (the default) shows the
//!   banners, held during a private duel until it ends as every reward's banner is; 2 shows
//!   them during duels too; 0 shows none: what comes is counted as shown without a banner,
//!   and banners waiting are taken back, so turning it on again replays nothing.
//!
//! `debug_holocron` ([`crate::holocrons::rehearsal`]) queues made-up holocrons through the
//! same banners, so they can be tried offline; those are never written to
//! `holocrons_seen.txt`.

use crate::holocrons::seen::Seen;
use crate::holocrons::{self, Entry, FIRST_READ_MAX};
use crate::unlock_toast::Unlock;
use sjk_identity::Holocron;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Whether a holocron shows a banner: [`OFF`] never, 1 (the default) yes, held back during
/// a private duel until it ends as every reward's banner is ([`crate::unlock_toast`]),
/// [`ALWAYS`] yes, during a duel too.
pub(crate) const NOTIFICATIONS_CVAR: &str = "cl_holocronNotifications";
pub(crate) const NOTIFICATIONS_DEFAULT: i64 = 1;
/// [`NOTIFICATIONS_CVAR`]'s value for no banners.
pub(crate) const OFF: i64 = 0;
/// [`NOTIFICATIONS_CVAR`]'s value for banners during a private duel too.
pub(crate) const ALWAYS: i64 = 2;

/// [`NOTIFICATIONS_CVAR`] as set, held to [`OFF`]..=[`ALWAYS`].
pub(crate) fn notification_mode(console: Option<&crate::console::ViewerConsole>) -> i64 {
    console
        .and_then(|console| console.integer_cvar(NOTIFICATIONS_CVAR))
        .unwrap_or(NOTIFICATIONS_DEFAULT)
        .clamp(OFF, ALWAYS)
}

/// A holocron whose banner is still to be given.
#[derive(Clone, Debug)]
struct Queued {
    entry: Entry,
    /// Made up by `debug_holocron`: shown like any other, never counted as seen.
    rehearsal: bool,
}

impl Queued {
    fn unlock(&self) -> Unlock {
        Unlock::Holocron {
            tier: self.entry.tier,
            id: self.entry.id,
            gift: self.entry.gift,
            earned: self
                .entry
                .earned
                .then(|| crate::kyber::Earned::of(&self.entry.note)),
            rehearsal: self.rehearsal,
        }
    }
}

/// The holocrons waiting for their banner and what was shown.
#[derive(Default)]
pub(crate) struct HolocronBanners {
    queue: VecDeque<Queued>,
    seen: Option<Seen>,
    /// The settings folder `holocrons_seen.txt` is written to.
    directory: PathBuf,
    /// The key, the number of holocrons and the highest number last offered, so an
    /// unchanged profile is skipped without reading its list.
    last: Option<(String, usize, u64)>,
    /// The highest number of the real holocrons handed to the banners and not yet counted
    /// as seen: what [`OFF`] counts at once.
    given: Option<u64>,
    /// How many holocrons `debug_holocron` made, so each gets a number of its own.
    rehearsed: u64,
}

impl HolocronBanners {
    /// The holocrons the player's profile lists now (`list`, newest first), for the key
    /// `key_id`, with the settings folder: any not shown yet wait for their banner.
    /// Reading the list costs a comparison unless it changed. Returns whether the identity
    /// changed, so the banners of the one before can be taken back.
    pub(crate) fn offer(&mut self, directory: &Path, key_id: &str, list: &[Holocron]) -> bool {
        let highest = list.iter().map(|holocron| holocron.id).max().unwrap_or(0);
        if self
            .last
            .as_ref()
            .is_some_and(|(key, len, top)| key == key_id && *len == list.len() && *top == highest)
        {
            return false;
        }
        self.last = Some((key_id.to_owned(), list.len(), highest));
        let changed = self
            .seen
            .as_ref()
            .is_none_or(|seen| seen.key_id() != key_id);
        if changed {
            self.seen = Some(Seen::load(directory, key_id));
            // Another identity's holocrons go; rehearsals stay.
            self.queue.retain(|waiting| waiting.rehearsal);
            self.given = None;
        }
        directory.clone_into(&mut self.directory);
        let Some(seen) = &self.seen else {
            return changed;
        };
        let given = self.given;
        let mut fresh: Vec<Entry> = list
            .iter()
            .filter_map(Entry::from_wire)
            .filter(|entry| {
                seen.is_new(entry.id)
                    && given.is_none_or(|given| entry.id > given)
                    && !self
                        .queue
                        .iter()
                        .any(|waiting| !waiting.rehearsal && waiting.entry.id == entry.id)
            })
            .collect();
        // Oldest first; of many (a PC that reinstalled), the newest.
        fresh.sort_by_key(|entry| entry.id);
        if fresh.len() > FIRST_READ_MAX {
            fresh.drain(..fresh.len() - FIRST_READ_MAX);
        }
        self.queue.extend(fresh.into_iter().map(|entry| Queued {
            entry,
            rehearsal: false,
        }));
        changed
    }

    /// Queue `entries` as made-up arrivals (`debug_holocron`): they get the same banners
    /// and sound as the hub's, but are never counted as seen.
    pub(crate) fn rehearse(&mut self, entries: Vec<Entry>) {
        self.rehearsed += entries.len() as u64;
        self.queue.extend(entries.into_iter().map(|entry| Queued {
            entry,
            rehearsal: true,
        }));
    }

    /// The number to give the first of the next rehearsed holocrons: each has one of its
    /// own, rising.
    pub(crate) fn rehearsal_base(&self) -> u64 {
        holocrons::rehearsal::FIRST_ID + self.rehearsed
    }

    /// A holocron waits for its banner.
    pub(crate) fn pending(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Hand every waiting holocron to `show`, oldest first, as one banner each (the
    /// banners fold several waiting together into one, [`crate::unlock_toast::GROUP_AT`]).
    pub(crate) fn announce(&mut self, mut show: impl FnMut(Unlock)) {
        for waiting in self.queue.drain(..) {
            if !waiting.rehearsal {
                self.given = Some(
                    self.given
                        .map_or(waiting.entry.id, |given| given.max(waiting.entry.id)),
                );
            }
            show(waiting.unlock());
        }
    }

    /// A banner began ([`crate::unlock_toast::UnlockToast::take_started`]): a real
    /// holocron's (or several) counts it, and every one before it, as seen.
    pub(crate) fn shown(&mut self, banner: Unlock) {
        if let Some(top) = banner.holocron_top() {
            self.mark(top);
        }
    }

    /// No banners ([`OFF`]): everything waiting or given counts as seen at once.
    fn drop_all(&mut self) {
        let waiting = self
            .queue
            .iter()
            .filter(|waiting| !waiting.rehearsal)
            .map(|waiting| waiting.entry.id)
            .max();
        self.queue.clear();
        if let Some(top) = waiting.max(self.given) {
            self.mark(top);
        }
    }

    /// Count holocron `top`, and every one before it, as seen, and write it down.
    fn mark(&mut self, top: u64) {
        let Some(seen) = &mut self.seen else {
            return;
        };
        if !seen.is_new(top) {
            return;
        }
        seen.mark(top);
        if self.given.is_some_and(|given| given <= top) {
            self.given = None;
        }
        if let Err(error) = seen.save(&self.directory) {
            crate::log::progress(format_args!(
                "holocrons: cannot write {}: {error}",
                holocrons::seen::FILE
            ));
        }
    }
}

impl crate::GpuState {
    /// Hand the banners the holocrons of the player's own profile (twice a second, with
    /// the identity's settings); a new identity takes the banners of the one before back.
    pub(crate) fn offer_holocrons(&mut self) {
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let directory = console.config_directory();
        let banners = &mut self.holocron_banners;
        let changed = crate::player_identity::with_own_holocrons(|key_id, _, list| {
            banners.offer(directory, key_id, list)
        })
        .unwrap_or(false);
        if changed {
            self.unlock_toast
                .withdraw(Instant::now(), |unlock| unlock.holocron_top().is_some());
        }
    }

    /// Give the waiting holocrons their banners, over play or a menu; with [`OFF`], count
    /// them as seen and
    /// take back the holocron banners waiting or showing. A private duel or Tactical
    /// holding them is the banners' own wait ([`crate::unlock_toast`]).
    pub(crate) fn prepare_holocron_banners(&mut self) {
        if notification_mode(self.console.as_ref()) == OFF {
            if self.holocron_banners.pending() || self.unlock_toast.pending() > 0 {
                self.unlock_toast
                    .withdraw(Instant::now(), Unlock::is_holocron);
                self.holocron_banners.drop_all();
            }
            return;
        }
        if self.holocron_banners.pending() {
            let toast = &mut self.unlock_toast;
            self.holocron_banners.announce(|unlock| toast.push(unlock));
        }
    }

    /// `debug_holocron`: queue made-up holocrons as if the hub had just dropped them
    /// ([`holocrons::rehearsal`]); alone, list the tiers. Nothing is sent and
    /// `holocrons_seen.txt` is left alone. The console closes so the banners show at once,
    /// and the first one's chat line shows in the game's feed.
    pub(crate) fn debug_holocron_command(
        &mut self,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if args.is_empty() {
            return Ok(holocrons::rehearsal::listing());
        }
        let entries = holocrons::rehearsal::parse(
            args,
            crate::medal_popup::rehearsal::unix_now(),
            self.holocron_banners.rehearsal_base(),
        )?;
        let lines = holocrons::rehearsal::queued(&entries);
        // A drop's chat line; a kyber crystal's grant is not announced, so none.
        if let Some(entry) = entries.first().filter(|entry| !entry.tier.is_kyber()) {
            self.chat.rehearse_holocron(entry.tier.id, Instant::now());
        }
        self.holocron_banners.rehearse(entries);
        if let Some(console) = &mut self.console {
            console.set_open(false);
        }
        self.sync_cursor_policy();
        Ok(lines)
    }

    /// `debug_find`: a made-up grand find of a blade in the chat feed (`chat/find_blade.rs`).
    pub(crate) fn debug_find_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let (tier, skin) = crate::chat::find_blade::parse(args)?;
        self.chat.rehearse_find(tier, skin, Instant::now());
        if let Some(console) = &mut self.console {
            console.set_open(false);
        }
        self.sync_cursor_policy();
        Ok(vec![format!(
            "A {} find of {} is in the chat: open the chat and rest the pointer on its blade.",
            tier.title(),
            skin.id
        )])
    }

    /// `debug_open`: play the Holocrons tab's opening of made-up holocrons
    /// ([`crate::cards::rehearsal`]); alone, list the tiers. In the SJK UI the
    /// Collection's Holocrons tab shows it, in the classic menus the page on its own.
    /// Nothing is sent.
    pub(crate) fn debug_open_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        if args.is_empty() {
            return Ok(crate::cards::rehearsal::listing());
        }
        // Check the words before any screen changes.
        crate::cards::rehearsal::parse(args)?;
        if self.in_game_menu.is_sjk() {
            self.open_profile_hub(crate::profile_hub::Tab::Holocrons);
        }
        let result = self.console.as_mut().map_or_else(
            || Err("the console is not ready".to_owned()),
            |console| console.debug_open(args),
        );
        self.sync_cursor_policy();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unlock_toast::UnlockToast;

    fn wire(id: u64, tier: &str) -> Holocron {
        Holocron {
            opened: None,
            traded: None,
            traded_from_key: None,
            traded_from: None,
            edition: sjk_identity::wire::EDITION_FIRST.to_owned(),
            id,
            tier: tier.to_owned(),
            dropped: 1_791_641_100,
            source: "play".to_owned(),
            note: String::new(),
        }
    }

    fn entry(id: u64, tier: usize) -> Entry {
        Entry {
            id,
            tier: &holocrons::TIERS[tier],
            dropped: 1_791_641_100,
            gift: false,
            earned: false,
            note: String::new(),
            played: false,
            traded: None,
            edition: sjk_identity::wire::EDITION_FIRST.to_owned(),
        }
    }

    /// The banners `announce` gives now.
    fn give(banners: &mut HolocronBanners) -> Vec<Unlock> {
        let mut given = Vec::new();
        banners.announce(|unlock| given.push(unlock));
        given
    }

    /// The names on the banners the queue would show, folded as it folds them.
    fn names(given: &[Unlock]) -> Vec<String> {
        let mut toast = UnlockToast::default();
        for unlock in given {
            toast.push(*unlock);
        }
        let mut names = Vec::new();
        let mut now = Instant::now();
        for _ in 0..10_000 {
            if toast.pending() == 0 {
                break;
            }
            now += std::time::Duration::from_millis(100);
            toast.update(now, true, false);
            if let Some(started) = toast.take_started() {
                names.push(started.name().to_string());
            }
        }
        names
    }

    /// New holocrons get one banner each, oldest first, once: counted as seen when the
    /// banner begins, so the same list (or a restart) gives none, a higher number does,
    /// and another identity sees its own.
    #[test]
    fn new_holocrons_get_their_banners_once_and_are_remembered() {
        let directory = tempfile::tempdir().expect("a folder");
        let mut banners = HolocronBanners::default();
        // The hub lists them newest first.
        let list = [
            wire(9, "legendary"),
            wire(8, "from_the_future"),
            wire(5, "rare"),
        ];
        assert!(
            banners.offer(directory.path(), "aa", &list),
            "a new identity"
        );
        assert!(banners.pending());
        assert_eq!(banners.queue.len(), 2, "the unknown tier is left out");
        let given = give(&mut banners);
        assert_eq!(names(&given), ["Rare Holocron", "Legendary Holocron"]);
        assert!(give(&mut banners).is_empty(), "given once");
        // Not shown yet: an unchanged profile read again gives nothing either.
        banners.last = None;
        assert!(!banners.offer(directory.path(), "aa", &list));
        assert!(!banners.pending(), "given, waiting for its banner");
        // Each banner begins: counted.
        for unlock in given {
            banners.shown(unlock);
        }
        let mut restarted = HolocronBanners::default();
        restarted.offer(directory.path(), "aa", &list);
        assert!(!restarted.pending());
        // A higher number shows; another identity sees its own.
        let longer = [wire(11, "mythical"), wire(9, "legendary"), wire(5, "rare")];
        assert!(!restarted.offer(directory.path(), "aa", &longer));
        assert_eq!(restarted.queue.len(), 1);
        assert_eq!(restarted.queue[0].entry.id, 11);
        assert!(restarted.offer(directory.path(), "bb", &list));
        assert_eq!(restarted.queue.len(), 2, "its own, the other's gone");
    }

    /// A holocron given a banner the game closed before showing (a duel held it) shows
    /// again at the next start: it is counted only once its banner begins.
    #[test]
    fn a_banner_never_shown_is_not_counted() {
        let directory = tempfile::tempdir().expect("a folder");
        let list = [wire(8, "rare"), wire(7, "rare")];
        let mut banners = HolocronBanners::default();
        banners.offer(directory.path(), "aa", &list);
        let given = give(&mut banners);
        banners.shown(given[0]);
        let seen = Seen::load(directory.path(), "aa");
        assert!(!seen.is_new(7) && seen.is_new(8), "7 began, 8 did not");
        // The game quit there: the next start gives only 8.
        let mut restarted = HolocronBanners::default();
        restarted.offer(directory.path(), "aa", &list);
        assert_eq!(restarted.queue.len(), 1);
        assert_eq!(restarted.queue[0].entry.id, 8);
    }

    /// A profile read for the first time gives its newest twenty, oldest first, which the
    /// banners fold into one; once it begins, the older ten never come back.
    #[test]
    fn a_profile_read_for_the_first_time_is_one_banner_of_its_newest_twenty() {
        let directory = tempfile::tempdir().expect("a folder");
        let mut banners = HolocronBanners::default();
        let mut list: Vec<Holocron> = (1..=30).rev().map(|id| wire(id, "uncommon")).collect();
        list[3].tier = "legendary".to_owned();
        banners.offer(directory.path(), "aa", &list);
        let ids: Vec<u64> = banners
            .queue
            .iter()
            .map(|waiting| waiting.entry.id)
            .collect();
        assert_eq!(ids, (11..=30).collect::<Vec<u64>>());
        let given = give(&mut banners);
        assert_eq!(given.len(), 20);
        let mut toast = UnlockToast::default();
        for unlock in &given {
            toast.push(*unlock);
        }
        assert_eq!(toast.pending(), 1, "one banner");
        assert!(toast.update(Instant::now(), true, false));
        let banner = toast.take_started().expect("it began");
        assert_eq!(banner.name().to_string(), "20 new holocrons");
        assert_eq!(
            banner.description().to_string(),
            "The rarest: a Legendary Holocron"
        );
        banners.shown(banner);
        let mut again = HolocronBanners::default();
        again.offer(directory.path(), "aa", &list);
        assert!(!again.pending());
    }

    /// `cl_holocronNotifications` 0: no banner, now or later; what came counts as seen.
    #[test]
    fn off_counts_them_as_seen_without_a_banner() {
        let directory = tempfile::tempdir().expect("a folder");
        let mut banners = HolocronBanners::default();
        let list = [wire(2, "rare"), wire(1, "uncommon")];
        banners.offer(directory.path(), "aa", &list);
        banners.drop_all();
        assert!(!banners.pending());
        assert!(give(&mut banners).is_empty(), "nothing replayed");
        let mut restarted = HolocronBanners::default();
        restarted.offer(directory.path(), "aa", &list);
        assert!(!restarted.pending(), "turning it on again replays nothing");
        // Given, then turned off before they showed: counted too.
        let mut late = HolocronBanners::default();
        late.offer(directory.path(), "aa", &[wire(4, "rare"), wire(3, "rare")]);
        give(&mut late);
        late.drop_all();
        assert!(!Seen::load(directory.path(), "aa").is_new(4));
    }

    #[test]
    fn an_unchanged_profile_is_not_read_again_and_one_waiting_is_not_queued_twice() {
        let directory = tempfile::tempdir().expect("a folder");
        let mut banners = HolocronBanners::default();
        let list = [wire(3, "rare")];
        banners.offer(directory.path(), "aa", &list);
        banners.offer(directory.path(), "aa", &list);
        assert_eq!(banners.queue.len(), 1);
        // A new one arrives while the first waits: it comes after it, the first not queued
        // twice.
        banners.offer(
            directory.path(),
            "aa",
            &[wire(4, "legendary"), wire(3, "rare")],
        );
        let ids: Vec<u64> = banners
            .queue
            .iter()
            .map(|waiting| waiting.entry.id)
            .collect();
        assert_eq!(ids, [3, 4]);
        // One already given is not given again.
        let mut banners = HolocronBanners::default();
        banners.offer(directory.path(), "cc", &list);
        give(&mut banners);
        banners.offer(directory.path(), "cc", &[wire(4, "rare"), wire(3, "rare")]);
        assert_eq!(banners.queue.len(), 1);
        assert_eq!(banners.queue[0].entry.id, 4);
    }

    /// A rehearsal gets the same banners but never goes into `holocrons_seen.txt`, so the
    /// real holocron still shows when the hub drops it; a new identity keeps the
    /// rehearsals.
    #[test]
    fn rehearsals_are_never_counted_as_seen() {
        let directory = tempfile::tempdir().expect("a folder");
        let file = directory.path().join(holocrons::seen::FILE);
        let mut banners = HolocronBanners::default();
        banners.offer(directory.path(), "aa", &[]);
        banners.rehearse(vec![entry(1, 0), entry(2, 3)]);
        for unlock in give(&mut banners) {
            assert!(matches!(
                unlock,
                Unlock::Holocron {
                    rehearsal: true,
                    ..
                }
            ));
            banners.shown(unlock);
        }
        assert!(!file.exists(), "nothing written");
        // The hub drops one now: it shows, and is remembered.
        banners.offer(directory.path(), "aa", &[wire(1, "uncommon")]);
        assert!(banners.pending() && !banners.queue[0].rehearsal);
        let mut early = HolocronBanners::default();
        early.rehearse(vec![entry(1, 0)]);
        early.offer(directory.path(), "bb", &[]);
        assert_eq!(
            early.queue.len(),
            1,
            "the identity arriving keeps rehearsals"
        );
        assert_eq!(names(&give(&mut early)), ["Uncommon Holocron"]);
        // Each rehearsal gets a number of its own.
        assert_eq!(early.rehearsal_base(), holocrons::rehearsal::FIRST_ID + 1);
        early.rehearse(vec![entry(1, 0), entry(2, 0)]);
        assert_eq!(early.rehearsal_base(), holocrons::rehearsal::FIRST_ID + 3);
    }
}
