//! The Mining toy's find table and mining log (`mining_log.txt`): what the Collection's
//! Log book ([`crate::catch_book`]) and the retired mining achievements read. The toy
//! itself (vinry's idea, built by Slash, 04/11/2026: a pickaxe, boulders broken with
//! clean strikes, the catch market and its daily goal) was removed on 20/11/2026 in
//! Sol's cleanup before the release; the log it wrote stays the player's own record,
//! read only.

/// The mining log, in the profile folder.
pub(crate) const LOG_FILE: &str = "mining_log.txt";

/// What a broken boulder gave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Rubble,
    Ore,
}

/// One thing in the find table: its id in the mining log, its name, its kind and its
/// weight among its kind (how often it came).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Find {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    pub(crate) weight: u32,
}

const fn rubble(id: &'static str, name: &'static str) -> Find {
    Find {
        id,
        name,
        kind: Kind::Rubble,
        weight: 1,
    }
}

const fn ore(id: &'static str, name: &'static str, weight: u32) -> Find {
    Find {
        id,
        name,
        kind: Kind::Ore,
        weight,
    }
}

/// How many kinds of ore [`FINDS`] holds (Prospector's goal).
pub(crate) const ORE_KINDS: usize = {
    let mut count = 0;
    let mut index = 0;
    while index < FINDS.len() {
        if matches!(FINDS[index].kind, Kind::Ore) {
            count += 1;
        }
        index += 1;
    }
    count
};

/// The rarest ore's id (its own achievement, Heart of the Mountain).
pub(crate) const HEART: &str = "heart_of_the_mountain";
/// An ore this light in [`FINDS`] (star quartz, void crystal, the Heart: about one ore
/// in seventeen) was a rare find, drawn in gold in the log book.
pub(crate) const RARE_WEIGHT: u32 = 11;

/// Whether `find` is a rare find.
pub(crate) fn is_rare(find: &Find) -> bool {
    find.kind == Kind::Ore && find.weight <= RARE_WEIGHT
}

/// Everything a boulder could give: SJK's own names.
pub(crate) const FINDS: [Find; 13] = [
    rubble("rubble", "a handful of rubble"),
    rubble("slag", "a lump of slag"),
    rubble("drill_bit", "a worn drill bit"),
    rubble("fossil", "a fossil shell"),
    ore("iron_ore", "a chunk of iron ore", 96),
    ore("copper_ore", "a chunk of copper ore", 86),
    ore("silver_nugget", "a silver nugget", 57),
    ore("glowstone", "a glowstone", 43),
    ore("sunstone", "a sunstone", 28),
    ore("moonstone", "a moonstone", 19),
    ore("star_quartz", "a star quartz", 11),
    ore("void_crystal", "a void crystal", 9),
    ore(HEART, "the Heart of the Mountain", 1),
];

/// The mining log as the toy wrote it: how many of each find the player dug out (sold or
/// not) and the day each first came up. Its `sold` lines are skipped.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Log {
    counts: Vec<(String, u32)>,
    /// The day each find first came up ([`crate::catch_book`]).
    firsts: crate::catch_book::Firsts,
}

impl Log {
    /// Read `text` (`<id> <count>` a line and `first <id> <yyyymmdd>`; anything else,
    /// `sold` lines included, is skipped).
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

    /// The rarest ore dug out (the lowest weight; the first of the table among equals),
    /// with its count.
    pub(crate) fn rarest(&self) -> Option<(&'static Find, u32)> {
        FINDS
            .iter()
            .filter(|f| f.kind == Kind::Ore && self.count(f.id) > 0)
            .min_by_key(|f| f.weight)
            .map(|f| (f, self.count(f.id)))
    }

    pub(crate) fn count(&self, id: &str) -> u32 {
        self.counts
            .iter()
            .find(|(known, _)| known == id)
            .map_or(0, |(_, count)| *count)
    }

    /// The different ores dug out, of all there are.
    pub(crate) fn kinds(&self) -> (usize, usize) {
        let all = FINDS.iter().filter(|f| f.kind == Kind::Ore);
        let found = all.clone().filter(|f| self.count(f.id) > 0).count();
        (found, all.count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A log the toy wrote reads back its counts and first days; its sold lines are
    /// skipped.
    #[test]
    fn a_log_reads_its_counts_and_first_days() {
        let log = Log::parse(
            "iron_ore 4
void_crystal 1
sold iron_ore 2
first void_crystal 20261116
",
        );
        assert_eq!((log.count("iron_ore"), log.count("void_crystal")), (4, 1));
        assert_eq!(log.first("void_crystal"), Some(20_261_116));
        let (rarest, count) = log.rarest().unwrap();
        assert_eq!((rarest.id, count), ("void_crystal", 1));
        assert!(is_rare(rarest));
        assert_eq!(log.kinds(), (2, ORE_KINDS));
    }
}
