//! Collection marks on saber shaders (`docs/sjk-ui.md`, Collection, "Marks"; 29/10/2026,
//! ideas of the players Atrosex and Kushina, extended by Sol): what the Shaders tab, the
//! Forge and the trade picker show on a shader besides its tier.
//!
//! - **Worn:** a blade the player wears: the first saber's (`cg_saberSkin`) or the second
//!   saber's (or the staff's other half's) own choice (`cg_saberSkin2`).
//! - **Favourite** (a star) and **Locked** (a padlock): the player's own marks, kept at the
//!   hub so they follow the player to any PC (`sjk_identity::UnlockMarks`). Favourites are
//!   listed first and a chip shows them alone; the hub refuses to break down or trade a
//!   locked shader, and the client greys those buttons with the reason. Only shown on a hub
//!   whose economy says `marks` ([`sjk_identity::MARKS_ABILITY`]).
//! - **New:** a shader the player holds and has not looked at since they got it. Local, on
//!   this PC: the Collection's seen-list (`crate::collection_seen`, which took over
//!   `shader-seen.txt` on 15/11/2026), its `kyber:` lines. With no list yet, everything
//!   already held the first time the hub says what the player holds counts as seen, so a
//!   player never finds every shader marked New. A shader chosen on the
//!   Shaders tab or in the Forge (by the pointer or the keys) is seen once it has stayed
//!   chosen for [`LOOK_MS`] ([`Look`]): sweeping the pointer across the wall sees nothing
//!   (Sol's decision, 29/10/2026); "Mark all seen" sees every one. One the player no longer
//!   holds leaves the list, so it is New again when it comes back.

use std::collections::HashSet;

pub(crate) use crate::collection_seen::Seen;

/// How long a New shader must stay chosen to count as seen, in milliseconds.
pub(crate) const LOOK_MS: u64 = crate::collection_seen::LOOK_MS;

/// The New shader the choice rests on, and since when: it is seen once it has stayed
/// chosen [`LOOK_MS`]. A choice that only passes over it, as a pointer swept across the
/// wall chooses each shader it crosses, sees nothing; nor does leaving it sooner.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Look {
    on: Option<(&'static str, u64)>,
}

impl Look {
    /// The choice is on `chosen` at `now` (milliseconds on any steady clock), `new`
    /// whether that shader is New: the shader now seen, once, when it has stayed chosen
    /// [`LOOK_MS`].
    pub(crate) fn observe(
        &mut self,
        chosen: Option<&'static str>,
        new: bool,
        now: u64,
    ) -> Option<&'static str> {
        match (self.on, chosen) {
            (Some((id, since)), Some(chosen)) if id == chosen => {
                if now.saturating_sub(since) < LOOK_MS {
                    return None;
                }
                // Seen: nothing more to wait for while the choice stays.
                self.on = Some((id, u64::MAX));
                (since != u64::MAX).then_some(id)
            }
            _ => {
                self.on = chosen.filter(|_| new).map(|id| (id, now));
                None
            }
        }
    }

    /// The choice left the page (another tab, the page closed) at `now`: the shader it
    /// was on when it had stayed [`LOOK_MS`] and was not seen yet.
    pub(crate) fn leave(&mut self, now: u64) -> Option<&'static str> {
        let (id, since) = self.on.take()?;
        (since != u64::MAX && now.saturating_sub(since) >= LOOK_MS).then_some(id)
    }
}

/// See shader `id`: it is New no more (`crate::collection_seen`).
pub(crate) fn see(id: &str) {
    crate::collection_seen::see(kyber(id));
}

/// See every one of `ids` ("Mark all seen").
pub(crate) fn see_all<'a>(ids: impl Iterator<Item = &'a str>) {
    crate::collection_seen::see_all(ids.map(kyber));
}

fn kyber(id: &str) -> crate::collection_seen::Thing {
    crate::collection_seen::Thing::new(crate::collection_seen::Kind::Kyber, id, 0)
}

/// Counts changes to the seen-list.
pub(crate) fn revision() -> u64 {
    crate::collection_seen::revision()
}

/// Read the list in place (keep `read` short); `None` while the first holdings are still
/// to be seen whole, when no shader is New.
pub(crate) fn with_seen<R>(read: impl FnOnce(Option<&Seen>) -> R) -> R {
    crate::collection_seen::with_seen(|seen| {
        read(
            seen.known(crate::collection_seen::Kind::Kyber)
                .then_some(seen),
        )
    })
}

/// What the pages show on each shader: worn, favourite, locked, New.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Marks {
    /// The hub keeps favourites and locks: the star and the padlock show, and can be set.
    pub(crate) kept: bool,
    favourite: HashSet<&'static str>,
    locked: HashSet<&'static str>,
    new: HashSet<&'static str>,
    /// The blades worn: the first saber's and the second's own.
    worn: [Option<&'static str>; 2],
}

impl Marks {
    /// The marks from the hub's (`hub`, while it keeps them: `kept`), the player's
    /// holdings (`owned`, the hub's list; none while not known), what was seen on this PC
    /// (`seen`; `None` while nothing can be New) and the blades `worn`. `debug`: the debug
    /// collectibles show, and nothing is New.
    pub(crate) fn of(
        hub: Option<&sjk_identity::UnlockMarks>,
        kept: bool,
        owned: &[sjk_identity::Unlock],
        seen: Option<&Seen>,
        debug: bool,
        worn: [Option<&'static str>; 2],
    ) -> Self {
        let ids = |list: &[String]| {
            list.iter()
                .filter_map(|id| crate::unlockables::blade_skin(id))
                .map(|skin| skin.id)
                .collect()
        };
        let (favourite, locked) = match hub.filter(|_| kept) {
            Some(marks) => (ids(&marks.favourite), ids(&marks.locked)),
            None => Default::default(),
        };
        let new = match seen.filter(|_| !debug) {
            Some(seen) => owned
                .iter()
                .filter_map(|unlock| crate::unlockables::blade_skin(&unlock.id))
                .filter(|skin| !seen.has_kyber(skin.id))
                .map(|skin| skin.id)
                .collect(),
            None => HashSet::new(),
        };
        Self {
            kept: kept && hub.is_some(),
            favourite,
            locked,
            new,
            worn,
        }
    }

    /// Whether shader `id` is a favourite.
    pub(crate) fn favourite(&self, id: &str) -> bool {
        self.favourite.contains(id)
    }

    /// Whether shader `id` is locked: never broken down or traded until unlocked.
    pub(crate) fn locked(&self, id: &str) -> bool {
        self.locked.contains(id)
    }

    /// Whether shader `id` is New.
    pub(crate) fn is_new(&self, id: &str) -> bool {
        self.new.contains(id)
    }

    /// Whether the player wears shader `id`, on either saber.
    pub(crate) fn worn(&self, id: &str) -> bool {
        self.worn.contains(&Some(id))
    }

    /// How many shaders are New.
    pub(crate) fn new_count(&self) -> usize {
        self.new.len()
    }

    /// The New shaders, for "Mark all seen".
    pub(crate) fn new_ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.new.iter().copied()
    }

    /// The marks of a world shot or a test: these favourites, locks and New ones, the hub
    /// keeping them, `worn` on the first saber.
    #[cfg(test)]
    pub(crate) fn shown(
        favourite: &[&'static str],
        locked: &[&'static str],
        new: &[&'static str],
        worn: Option<&'static str>,
    ) -> Self {
        Self {
            kept: true,
            favourite: favourite.iter().copied().collect(),
            locked: locked.iter().copied().collect(),
            new: new.iter().copied().collect(),
            worn: [worn, None],
        }
    }
}

/// The blades `first` (`cg_saberSkin`) and `second` (`cg_saberSkin2`) the player wears,
/// among those they hold (`owned`): the second's own choice only, not the stock blade's
/// word nor an empty one (it follows the first).
pub(crate) fn worn_of(
    first: &str,
    second: &str,
    owned: &[sjk_identity::Unlock],
) -> [Option<&'static str>; 2] {
    let held = |id: &str| {
        crate::unlockables::blade_skin(id.trim())
            .filter(|skin| owned.iter().any(|unlock| unlock.id == skin.id))
            .map(|skin| skin.id)
    };
    [held(first), held(second)]
}

/// Favourites first, each part in the order it stood (`list` sorted stable).
pub(crate) fn favourites_first<T>(list: &mut [T], marks: &Marks, id: impl Fn(&T) -> &str) {
    if marks.favourite.is_empty() {
        return;
    }
    list.sort_by_key(|item| !marks.favourite(id(item)));
}

/// The hover card's lines for `id`'s marks, in the order the page shows them: worn, New,
/// favourite, locked. Each says what the mark means.
pub(crate) fn hover_lines(marks: &Marks, id: &str) -> Vec<String> {
    let mut lines = Vec::new();
    if marks.worn(id) {
        lines.push(format!(
            "{}  Worn: on your saber now.",
            crate::symbols::SABER
        ));
    }
    if marks.is_new(id) {
        lines.push("New: yours since you last looked. A second on it marks it seen.".to_owned());
    }
    if marks.favourite(id) {
        lines.push(format!(
            "{}  Favourite: listed first (F to unstar).",
            crate::symbols::EDITION
        ));
    }
    if marks.locked(id) {
        lines.push(format!(
            "{}  Locked: never broken down or traded until you unlock it (L).",
            crate::symbols::LOCK
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlock(id: &str) -> sjk_identity::Unlock {
        sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: id.to_owned(),
            granted: 1,
            note: String::new(),
            count: 1,
            medal: None,
            card: None,
        }
    }

    /// New is what is held and not seen; nothing is New while the first holdings wait or
    /// the debug collectibles show. The hub's favourites and locks show only while it keeps
    /// them; worn is either saber's own choice among those held.
    #[test]
    fn marks_read_the_hub_the_seen_list_and_the_worn_blades() {
        let owned = [unlock("saber_storm"), unlock("saber_void")];
        let seen = Seen::parse("kyber:saber_storm\n");
        let hub = sjk_identity::UnlockMarks {
            favourite: vec!["saber_void".to_owned(), "saber_nothing".to_owned()],
            locked: vec!["saber_storm".to_owned()],
        };
        let worn = worn_of("saber_void", "saber_frost", &owned);
        assert_eq!(worn, [Some("saber_void"), None], "the second's is not held");
        let marks = Marks::of(Some(&hub), true, &owned, Some(&seen), false, worn);
        assert!(marks.kept);
        assert!(marks.favourite("saber_void") && !marks.favourite("saber_nothing"));
        assert!(marks.locked("saber_storm") && !marks.locked("saber_void"));
        assert!(marks.is_new("saber_void") && !marks.is_new("saber_storm"));
        assert_eq!(marks.new_count(), 1);
        assert!(marks.worn("saber_void") && !marks.worn("saber_storm"));
        assert_eq!(
            worn_of("", crate::looks::STOCK_SECOND, &owned),
            [None, None]
        );
        // An older hub: no star, no padlock; New and worn still show.
        let old = Marks::of(None, false, &owned, Some(&seen), false, worn);
        assert!(!old.kept && !old.locked("saber_storm") && !old.favourite("saber_void"));
        assert!(old.is_new("saber_void"));
        let unsaid = Marks::of(Some(&hub), false, &owned, Some(&seen), false, worn);
        assert!(!unsaid.kept && !unsaid.locked("saber_storm"));
        assert_eq!(
            Marks::of(Some(&hub), true, &owned, None, false, worn).new_count(),
            0,
            "the first holdings wait"
        );
        assert_eq!(
            Marks::of(Some(&hub), true, &owned, Some(&seen), true, worn).new_count(),
            0,
            "debug collectibles"
        );
        let lines = hover_lines(&marks, "saber_void");
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines[0].contains("Worn") && lines[1].starts_with("New"));
    }

    /// A New shader is seen once it has stayed chosen a second, by the pointer or the
    /// keys; one the choice only passed over is not, and leaving it sooner sees nothing.
    #[test]
    fn a_new_shader_is_seen_after_a_second_chosen() {
        let mut look = Look::default();
        // Chosen and kept: seen at one second, once.
        assert_eq!(look.observe(Some("saber_storm"), true, 10_000), None);
        assert_eq!(look.observe(Some("saber_storm"), true, 10_500), None);
        assert_eq!(look.observe(Some("saber_storm"), true, 10_999), None);
        assert_eq!(
            look.observe(Some("saber_storm"), true, 11_000),
            Some("saber_storm")
        );
        assert_eq!(look.observe(Some("saber_storm"), true, 11_016), None);
        assert_eq!(look.leave(12_000), None, "already seen");
        // One that is not New starts no wait.
        assert_eq!(look.observe(Some("saber_void"), false, 20_000), None);
        assert_eq!(look.observe(Some("saber_void"), false, 25_000), None);
        // Nothing chosen, then a New one: the wait starts when it is chosen.
        assert_eq!(look.observe(None, false, 30_000), None);
        assert_eq!(look.observe(Some("saber_frost"), true, 30_016), None);
        assert_eq!(
            look.observe(Some("saber_frost"), true, 31_016),
            Some("saber_frost")
        );
    }

    /// Sweeping the pointer across the wall chooses each shader for a few frames: none of
    /// them is seen, and the one it stops on is seen a second later.
    #[test]
    fn a_sweep_across_new_shaders_sees_none() {
        let wall = ["saber_storm", "saber_void", "saber_frost", "saber_tidal"];
        let mut look = Look::default();
        let mut now = 0;
        let mut seen = Vec::new();
        for _ in 0..3 {
            for id in wall.iter().chain(wall.iter().rev()) {
                // About 150 ms on each, a frame every 16 ms.
                for _ in 0..10 {
                    now += 16;
                    seen.extend(look.observe(Some(id), true, now));
                }
            }
        }
        assert!(seen.is_empty(), "{seen:?}");
        // Off the wall (between tiles) and back: the wait starts again.
        assert_eq!(look.observe(None, false, now + 16), None);
        now += 32;
        for _ in 0..70 {
            now += 16;
            seen.extend(look.observe(Some("saber_tidal"), true, now));
        }
        assert_eq!(seen, ["saber_tidal"]);
    }

    /// Closing the page or turning to another tab sees the shader chosen only when it had
    /// stayed a second (a frame may not have come since); sooner, it stays New.
    #[test]
    fn leaving_sees_only_a_shader_looked_at_for_a_second() {
        let mut look = Look::default();
        look.observe(Some("saber_storm"), true, 5_000);
        assert_eq!(look.leave(5_400), None, "left too soon");
        assert_eq!(look.leave(9_000), None, "nothing chosen any more");
        look.observe(Some("saber_storm"), true, 6_000);
        assert_eq!(look.leave(7_000), Some("saber_storm"));
        assert_eq!(look.observe(Some("saber_storm"), true, 7_100), None);
        assert_eq!(Look::default().leave(1_000), None);
    }

    /// Favourites come first, each part in its order.
    #[test]
    fn favourites_are_listed_first_in_order() {
        let marks = Marks::shown(&["saber_frost", "saber_storm"], &[], &[], None);
        let mut list = ["saber_void", "saber_storm", "saber_tidal", "saber_frost"];
        favourites_first(&mut list, &marks, |id| id);
        assert_eq!(
            list,
            ["saber_storm", "saber_frost", "saber_void", "saber_tidal"]
        );
        let mut same = ["saber_void", "saber_tidal"];
        favourites_first(&mut same, &Marks::default(), |id| id);
        assert_eq!(same, ["saber_void", "saber_tidal"]);
    }
}
