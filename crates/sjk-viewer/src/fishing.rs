//! The Fishing toy's catch table and fish log (`fishing_log.txt`): what the Collection's
//! Log book ([`crate::catch_book`]) and the retired fishing achievements read. The toy
//! itself (Sanji's idea, 29/10/2026, Slash's design: a rod, a bucket and a float, bites
//! hooked with `+use`, Echo bites, the catch market and its daily goal) was removed on
//! 20/11/2026 in Sol's cleanup before the release; the log it wrote stays the player's
//! own record, read only.

/// The fish log, in the profile folder.
pub(crate) const LOG_FILE: &str = "fishing_log.txt";

/// What a hooked bite brought up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Junk,
    Fish,
}

/// One thing in the catch table: its id in the fish log, its name, its kind and its
/// weight among its kind (how often it bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Catch {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    pub(crate) weight: u32,
}

const fn junk(id: &'static str, name: &'static str) -> Catch {
    Catch {
        id,
        name,
        kind: Kind::Junk,
        weight: 1,
    }
}

const fn fish(id: &'static str, name: &'static str, weight: u32) -> Catch {
    Catch {
        id,
        name,
        kind: Kind::Fish,
        weight,
    }
}

/// A fish this light in [`CATCHES`] (one in about 35 fish or rarer) was a rare catch,
/// drawn in gold in the log book.
pub(crate) const RARE_WEIGHT: u32 = 3;

/// Whether `catch` is a rare catch.
pub(crate) fn is_rare(catch: &Catch) -> bool {
    catch.kind == Kind::Fish && catch.weight <= RARE_WEIGHT
}

/// How many kinds of fish [`CATCHES`] holds.
pub(crate) const FISH_KINDS: usize = {
    let mut count = 0;
    let mut index = 0;
    while index < CATCHES.len() {
        if matches!(CATCHES[index].kind, Kind::Fish) {
            count += 1;
        }
        index += 1;
    }
    count
};

/// How many kinds of junk [`CATCHES`] holds (Beachcomber).
pub(crate) const JUNK_KINDS: usize = CATCHES.len() - FISH_KINDS;

/// Everything a bite could bring up but nothing: SJK's own names.
pub(crate) const CATCHES: [Catch; 18] = [
    junk("boot", "an old boot"),
    junk("blaster", "a rusted blaster"),
    junk("droid_head", "a droid's head"),
    junk("weed", "a tangle of weed"),
    junk("hilt", "a broken saber hilt"),
    junk("holopad", "a soggy holopad"),
    fish("silver_darter", "a Silver darter", 20),
    fish("mud_snapper", "a Mud snapper", 20),
    fish("swamp_gulper", "a Swamp gulper", 16),
    fish("striped_bonefish", "a Striped bonefish", 14),
    fish("glow_perch", "a Glow perch", 10),
    fish("spiny_pike", "a Spiny pike", 8),
    fish("sun_carp", "a Sun carp", 6),
    fish("moon_eel", "a Moon eel", 4),
    fish("kyber_minnow", "a Kyber minnow", 3),
    fish("ghost_ray", "a Ghost ray", 2),
    fish("temple_koi", "a Temple koi", 2),
    fish("old_whiskers", "Old Whiskers, the great catfish", 1),
];

/// The fish log as the toy wrote it: how many of each catch the player brought up (sold
/// or not) and the day each first came up. Its `sold` and `last` lines are skipped.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Log {
    counts: Vec<(String, u32)>,
    /// The day each catch first came up ([`crate::catch_book`]).
    firsts: crate::catch_book::Firsts,
}

impl Log {
    /// Read `text` (`<id> <count>` a line and `first <id> <yyyymmdd>`; anything else,
    /// `sold` and `last` lines included, is skipped).
    pub(crate) fn parse(text: &str) -> Self {
        let mut log = Self::default();
        for line in text.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                [crate::catch_book::FIRST, id, day] => log.firsts.read(id, day),
                [id, value] => {
                    if let Ok(count) = value.parse::<u32>() {
                        log.counts.retain(|(known, _)| known != id);
                        log.counts.push(((*id).to_owned(), count));
                    }
                }
                _ => {}
            }
        }
        log
    }

    /// The day `id` first came up, when the log knows it.
    pub(crate) fn first(&self, id: &str) -> Option<u32> {
        self.firsts.of(id)
    }

    /// The rarest fish caught (the lowest weight; the first of the table among equals),
    /// with its count.
    pub(crate) fn rarest(&self) -> Option<(&'static Catch, u32)> {
        CATCHES
            .iter()
            .filter(|c| c.kind == Kind::Fish && self.count(c.id) > 0)
            .min_by_key(|c| c.weight)
            .map(|c| (c, self.count(c.id)))
    }

    pub(crate) fn count(&self, id: &str) -> u32 {
        self.counts
            .iter()
            .find(|(known, _)| known == id)
            .map_or(0, |(_, count)| *count)
    }

    /// The different fish caught, of all there are.
    pub(crate) fn species(&self) -> (usize, usize) {
        let all = CATCHES.iter().filter(|c| c.kind == Kind::Fish);
        let caught = all.clone().filter(|c| self.count(c.id) > 0).count();
        (caught, all.count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A log the toy wrote reads back its counts and first days; its sold and last
    /// lines, and anything else, are skipped.
    #[test]
    fn a_log_reads_its_counts_and_first_days() {
        let log = Log::parse(
            "boot 3
kyber_minnow 2
sold boot 2
last kyber_minnow
first kyber_minnow 20261115
noise
",
        );
        assert_eq!((log.count("boot"), log.count("kyber_minnow")), (3, 2));
        assert_eq!(log.first("kyber_minnow"), Some(20_261_115));
        assert_eq!(log.count("sold"), 0);
        let (rarest, count) = log.rarest().unwrap();
        assert_eq!((rarest.id, count), ("kyber_minnow", 2));
        assert!(is_rare(rarest));
        assert_eq!(log.species(), (1, FISH_KINDS));
        assert_eq!(FISH_KINDS + JUNK_KINDS, CATCHES.len());
    }
}
