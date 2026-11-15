//! Collection > Inventory (Sol, 15/11/2026: "plain and simple, one list of everything the
//! player owns"): the currencies (credits, kyber shards and, once the hub said, DOOM coins),
//! the holocrons and kyber crystals still sealed, the saber shaders, finishers and tags held
//! and the cards, each a row with its name and tier in the tier's colour and how many,
//! grouped by kind under a heading. S orders each group by tier, name or quantity; the
//! search field (`/`, Ctrl+F; [`crate::text_entry`]) keeps the rows whose name, tier, kind
//! or note hold every word typed. It reads only what the own hub profile already gives the
//! other tabs (`Profile::credits`, `shards`, `holocron_counts`, `holocrons`, `unlocks`,
//! `gif_tags`, `cards`) and the WHEEL OF DOOM's last read; it asks the hub nothing.
//! The look is [`super::inventory_view`].

use super::*;
use sjk_ui::Color;

/// The sort's segments ([`Sort::ALL`]), the search field and the list's rows.
pub(super) const SORT_BASE: u16 = 8_400;
pub(super) const SEARCH_TOKEN: u16 = 8_410;
pub(super) const ROW_BASE: u16 = 8_420;
/// How many places of the list show at once.
pub(super) const ROWS_SHOWN: usize = 16;
const _: () = assert!(ROW_BASE as usize + ROWS_SHOWN <= 8_500);

/// The most characters the search holds.
const SEARCH_MAX: usize = 48;

/// The kinds of thing, in the list's order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Section {
    Currencies,
    Holocrons,
    Crystals,
    Shaders,
    Finishers,
    Tags,
    Cards,
}

impl Section {
    /// Its heading.
    pub(super) fn label(self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Currencies => "Currencies".into(),
            Self::Holocrons => crate::terms::say("Holocrons"),
            Self::Crystals => "Kyber crystals".into(),
            Self::Shaders => "Saber shaders".into(),
            Self::Finishers => "Finishers".into(),
            Self::Tags => "Tags".into(),
            Self::Cards => "Cards".into(),
        }
    }
}

/// How each group stands.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Sort {
    /// The rarest first, then by name.
    #[default]
    Tier,
    /// By name, A to Z.
    Name,
    /// The most held first.
    Quantity,
}

impl Sort {
    pub(super) const ALL: [Self; 3] = [Self::Tier, Self::Name, Self::Quantity];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Tier => "Tier",
            Self::Name => "Name",
            Self::Quantity => "Quantity",
        }
    }

    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|sort| *sort == self).unwrap_or(0)
    }

    /// The next order (`back`: the one before), round.
    fn next(self, back: bool) -> Self {
        let count = Self::ALL.len();
        let at = self.index();
        Self::ALL[if back {
            (at + count - 1) % count
        } else {
            (at + 1) % count
        }]
    }
}

/// One thing held: a row of the list.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Item {
    pub(super) section: Section,
    pub(super) name: String,
    /// Its tier's name ("Mythical"), empty for a currency.
    pub(super) tier: &'static str,
    /// Its tier's rank, the commonest 0 ([`unlockables::Rarity`]'s order), for the sort.
    pub(super) rank: u8,
    pub(super) colour: Color,
    pub(super) count: u64,
    /// A word more: the finishes held, the edition, a blade's number.
    pub(super) note: String,
}

/// A place of the list: a group's heading, with its kinds and how many in all, or an item
/// (its index in the items).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Place {
    Heading {
        section: Section,
        kinds: usize,
        quantity: u64,
    },
    Item(usize),
}

/// What the profile holds, read for the list.
#[derive(Default)]
pub(super) struct Sources<'a> {
    pub(super) credits: u64,
    pub(super) shards: u64,
    /// DOOM coins, once the hub said (the WHEEL OF DOOM's read).
    pub(super) coins: Option<u64>,
    /// Sealed holocrons by tier ([`crate::holocrons::TIERS`]).
    pub(super) holocrons: [u32; crate::holocrons::COUNT],
    /// Sealed kyber crystals by tier ([`crate::kyber::TIERS`]).
    pub(super) crystals: [u32; crate::kyber::COUNT],
    pub(super) unlocks: &'a [sjk_identity::Unlock],
    /// How many GIF tags the person made.
    pub(super) gif_tags: usize,
    /// The cards, already named ([`card_items`]).
    pub(super) cards: Vec<Item>,
}

/// The rank of an unlockable's tier.
fn rank(tier: unlockables::Rarity) -> u8 {
    tier as u8
}

/// "1 Holo, 2 Solar" for a grant's finishes, empty for none.
fn finishes(unlock: &sjk_identity::Unlock) -> String {
    match (unlock.holo, unlock.solar) {
        (0, 0) => String::new(),
        (holo, 0) => format!("{holo} Holo"),
        (0, solar) => format!("{solar} Solar"),
        (holo, solar) => format!("{holo} Holo, {solar} Solar"),
    }
}

/// Every thing `sources` holds, in no order ([`places`] orders them).
pub(super) fn items(sources: &Sources<'_>) -> Vec<Item> {
    let mut items = Vec::new();
    let currency = |name: &str, colour, count| Item {
        section: Section::Currencies,
        name: name.to_owned(),
        tier: "",
        rank: 0,
        colour,
        count,
        note: String::new(),
    };
    items.push(currency("Credits", color_credits(), sources.credits));
    items.push(currency(
        "Kyber shards",
        crate::cards::shard::KYBER,
        sources.shards,
    ));
    if let Some(coins) = sources.coins {
        items.push(currency("DOOM coins", color_doom(), coins));
    }
    for (tier, count) in crate::holocrons::TIERS.iter().zip(sources.holocrons) {
        if count > 0 {
            items.push(Item {
                section: Section::Holocrons,
                name: tier.title().into_owned(),
                tier: tier.label,
                rank: tier.index as u8 + 1,
                colour: tier.colour,
                count: count.into(),
                note: String::new(),
            });
        }
    }
    for (tier, count) in crate::kyber::TIERS.iter().zip(sources.crystals) {
        if count > 0 {
            items.push(Item {
                section: Section::Crystals,
                name: tier.title().into_owned(),
                tier: tier.label,
                rank: tier.index as u8 + 1,
                colour: tier.colour,
                count: count.into(),
                note: String::new(),
            });
        }
    }
    for unlock in sources.unlocks.iter().filter(|unlock| unlock.count > 0) {
        if let Some(skin) = unlockables::blade_skin(&unlock.id) {
            items.push(Item {
                section: Section::Shaders,
                name: skin.name.to_owned(),
                tier: skin.tier.label(),
                rank: rank(skin.tier),
                colour: unlockables::name_colour(skin.id).unwrap_or(skin.tier.colour()),
                count: unlock.count.into(),
                note: format!("Blade {}", skin.number_text()),
            });
        } else if let Some(finisher) = crate::finishers::get(&unlock.id) {
            items.push(Item {
                section: Section::Finishers,
                name: finisher.name.to_owned(),
                tier: finisher.tier.label(),
                rank: rank(finisher.tier),
                colour: finisher.tier.colour(),
                count: unlock.count.into(),
                note: String::new(),
            });
        }
    }
    // The tags: the starter set everyone holds, and the ones granted.
    for tag in crate::tags::CATALOGUE {
        let grant = sources.unlocks.iter().find(|unlock| unlock.id == tag.id);
        let (count, note) = match grant {
            Some(unlock) if unlock.count > 0 => (unlock.count.into(), finishes(unlock)),
            _ if tag.free => (1, "Starter".to_owned()),
            _ => continue,
        };
        items.push(Item {
            section: Section::Tags,
            name: tag.name.to_owned(),
            tier: tag.tier.label(),
            rank: rank(tag.tier),
            colour: tag.tier.colour(),
            count,
            note,
        });
    }
    if sources.gif_tags > 0 {
        items.push(Item {
            section: Section::Tags,
            name: "GIF tags".to_owned(),
            tier: "",
            rank: 0,
            colour: crate::menu::sjk::color::TEXT,
            count: sources.gif_tags as u64,
            note: "Made from blank tags".to_owned(),
        });
    }
    items.extend(sources.cards.iter().cloned());
    items
}

/// The cards of `stacks`, named from the packs' `faces`: a row to each kind of copy.
pub(super) fn card_items(
    stacks: &[crate::cards::Stack],
    faces: &crate::cards::faces::Faces,
) -> Vec<Item> {
    stacks
        .iter()
        .filter(|stack| stack.count > 0)
        .map(|stack| {
            let mut note = Vec::with_capacity(3);
            if stack.sealed() {
                note.push("Sealed");
            }
            if stack.finish.index > 0 {
                note.push(stack.finish.name);
            }
            if !stack.edition.is_empty() {
                note.push(&stack.edition);
            }
            Item {
                section: Section::Cards,
                name: stack.name_in(faces),
                tier: stack.rarity.name,
                rank: stack.rarity.index as u8,
                colour: stack.rarity.colour,
                count: stack.count.into(),
                note: note.join(", "),
            }
        })
        .collect()
}

/// The credits' colour: the gold they are drawn in on the Forge and the Cards tab.
fn color_credits() -> Color {
    crate::menu::sjk::color::GOLD_BRIGHT
}

/// The DOOM coins' colour: the DOOM blade's red ([`unlockables::NAME_COLOURS`]).
fn color_doom() -> Color {
    crate::holocrons::rgb(0xE5_1A1A)
}

/// Whether `item` holds every word of `query` (lowercase, apart by spaces) in its name,
/// tier, kind or note.
fn matches(item: &Item, words: &[String]) -> bool {
    if words.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {}",
        item.name,
        item.tier,
        item.section.label(),
        item.note
    )
    .to_lowercase();
    words.iter().all(|word| haystack.contains(word.as_str()))
}

/// The list of `items` that `query` keeps, a heading over each kind that has any, each
/// group in `sort`'s order.
pub(super) fn places(items: &[Item], query: &str, sort: Sort) -> Vec<Place> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut kept: Vec<usize> = (0..items.len())
        .filter(|index| matches(&items[*index], &words))
        .collect();
    let name = |item: &Item| item.name.to_lowercase();
    kept.sort_by(|a, b| {
        let (a, b) = (&items[*a], &items[*b]);
        a.section.cmp(&b.section).then_with(|| match sort {
            Sort::Tier => b
                .rank
                .cmp(&a.rank)
                .then_with(|| name(a).cmp(&name(b)))
                .then_with(|| b.count.cmp(&a.count)),
            Sort::Name => name(a)
                .cmp(&name(b))
                .then_with(|| b.rank.cmp(&a.rank))
                .then_with(|| b.count.cmp(&a.count)),
            Sort::Quantity => b
                .count
                .cmp(&a.count)
                .then_with(|| b.rank.cmp(&a.rank))
                .then_with(|| name(a).cmp(&name(b))),
        })
    });
    let mut places = Vec::with_capacity(kept.len() + 7);
    let mut at = 0;
    while at < kept.len() {
        let section = items[kept[at]].section;
        let end = kept[at..]
            .iter()
            .position(|index| items[*index].section != section)
            .map_or(kept.len(), |len| at + len);
        places.push(Place::Heading {
            section,
            kinds: end - at,
            quantity: kept[at..end].iter().map(|index| items[*index].count).sum(),
        });
        places.extend(kept[at..end].iter().map(|index| Place::Item(*index)));
        at = end;
    }
    places
}

/// The Inventory tab's state: its search, order, choice and scroll, and the list as last
/// drawn.
#[derive(Debug, Default)]
pub(crate) struct Inventory {
    pub(super) query: String,
    pub(super) search_focus: bool,
    pub(super) sort: Sort,
    /// The place chosen (an item's, never a heading's while there is an item).
    pub(super) chosen: usize,
    /// The first place shown.
    pub(super) first: usize,
    pub(super) items: Vec<Item>,
    pub(super) places: Vec<Place>,
}

impl Inventory {
    /// Read the list again from `items`, the choice kept on the same thing where it is
    /// still listed.
    pub(super) fn refresh(&mut self, items: Vec<Item>) {
        let was = self.chosen_item().cloned();
        self.places = places(&items, &self.query, self.sort);
        self.items = items;
        if let Some(was) = was
            && let Some(at) = self.places.iter().position(|place| {
                matches!(place, Place::Item(index)
                    if self.items[*index].name == was.name && self.items[*index].section == was.section)
            })
        {
            self.chosen = at;
        }
        self.chosen = self.chosen.min(self.places.len().saturating_sub(1));
        self.settle(1);
        self.first = self.first.min(self.places.len().saturating_sub(ROWS_SHOWN));
        self.keep_in_view();
    }

    /// The item chosen, if any.
    pub(super) fn chosen_item(&self) -> Option<&Item> {
        match self.places.get(self.chosen)? {
            Place::Item(index) => self.items.get(*index),
            Place::Heading { .. } => None,
        }
    }

    /// Off a heading, towards `by`'s side first, else the other.
    fn settle(&mut self, by: isize) {
        let item = |at: usize, places: &[Place]| matches!(places.get(at), Some(Place::Item(_)));
        if self.places.is_empty() || item(self.chosen, &self.places) {
            return;
        }
        let forward = (self.chosen..self.places.len()).find(|at| item(*at, &self.places));
        let back = (0..self.chosen).rev().find(|at| item(*at, &self.places));
        let found = if by >= 0 {
            forward.or(back)
        } else {
            back.or(forward)
        };
        if let Some(at) = found {
            self.chosen = at;
        }
    }

    /// Move the choice `by` places, stopping at the ends, past headings.
    pub(super) fn step(&mut self, by: isize) {
        let last = self.places.len().saturating_sub(1) as isize;
        self.chosen = (self.chosen as isize).saturating_add(by).clamp(0, last) as usize;
        self.settle(by);
        // At the top, show the first group's heading too.
        if self.chosen <= 1 {
            self.first = 0;
        }
        self.keep_in_view();
    }

    /// Scroll `by` places (the wheel), the choice kept within what shows.
    pub(super) fn scroll(&mut self, by: isize) {
        let most = self.places.len().saturating_sub(ROWS_SHOWN) as isize;
        self.first = (self.first as isize + by).clamp(0, most) as usize;
        let last = (self.first + ROWS_SHOWN).min(self.places.len());
        if self.chosen < self.first || self.chosen >= last {
            self.chosen = self.chosen.clamp(self.first, last.saturating_sub(1));
            self.settle(by);
        }
    }

    /// Scroll so the choice shows.
    fn keep_in_view(&mut self) {
        if self.chosen < self.first {
            self.first = self.chosen;
        } else if self.chosen >= self.first + ROWS_SHOWN {
            self.first = self.chosen + 1 - ROWS_SHOWN;
        }
    }

    /// The list's words or order changed: back to its top.
    fn changed(&mut self) {
        self.places = places(&self.items, &self.query, self.sort);
        self.chosen = 0;
        self.first = 0;
        self.settle(1);
    }

    /// Put the typing in the search field.
    pub(super) fn focus_search(&mut self) {
        self.search_focus = true;
        crate::text_entry::select_all(&self.query);
    }

    /// The search field lets go of the typing (what it holds stays).
    pub(super) fn leave_search(&mut self) {
        if self.search_focus {
            self.search_focus = false;
            crate::text_entry::deselect();
        }
    }

    /// Order each group the next way (`back`: the one before).
    pub(super) fn cycle_sort(&mut self, back: bool) {
        self.sort = self.sort.next(back);
        self.changed();
    }

    /// A key on the tab (`text`: what it types); `None` for one it leaves to the page.
    pub(super) fn key(
        &mut self,
        text: Option<&str>,
        key: KeyCode,
        shift: bool,
        control: bool,
    ) -> Option<PanelAction> {
        if self.search_focus {
            match key {
                KeyCode::Escape => {
                    self.leave_search();
                    if !self.query.is_empty() {
                        self.query.clear();
                        self.changed();
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => self.leave_search(),
                KeyCode::ArrowUp => self.step(-1),
                KeyCode::ArrowDown => self.step(1),
                _ => {
                    if crate::text_entry::edit(
                        &mut self.query,
                        key,
                        text,
                        shift,
                        control,
                        SEARCH_MAX,
                        true,
                    ) {
                        self.changed();
                    }
                }
            }
            return Some(PanelAction::None);
        }
        match key {
            KeyCode::Slash => self.focus_search(),
            KeyCode::KeyF if control => self.focus_search(),
            _ if control => return None,
            KeyCode::ArrowUp => self.step(-1),
            KeyCode::ArrowDown => self.step(1),
            KeyCode::PageUp => self.step(-(ROWS_SHOWN as isize)),
            KeyCode::PageDown => self.step(ROWS_SHOWN as isize),
            KeyCode::Home => self.step(isize::MIN / 2),
            KeyCode::End => self.step(isize::MAX / 2),
            KeyCode::KeyS => self.cycle_sort(shift),
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// A press (`activate`) or a rest (`resting`) of the pointer on `token`; `None` for a
    /// token that is not the tab's.
    pub(super) fn token(
        &mut self,
        token: u16,
        activate: bool,
        resting: bool,
    ) -> Option<PanelAction> {
        if activate && token != SEARCH_TOKEN {
            self.leave_search();
        }
        if token == SEARCH_TOKEN {
            if activate {
                self.focus_search();
            }
            return Some(PanelAction::None);
        }
        if let Some(sort) = token
            .checked_sub(SORT_BASE)
            .and_then(|index| Sort::ALL.get(usize::from(index)))
        {
            if activate && *sort != self.sort {
                self.sort = *sort;
                self.changed();
            }
            return Some(PanelAction::None);
        }
        let slot = token
            .checked_sub(ROW_BASE)
            .map(usize::from)
            .filter(|slot| *slot < ROWS_SHOWN)?;
        let at = self.first + slot;
        if (activate || resting) && matches!(self.places.get(at), Some(Place::Item(_))) {
            self.chosen = at;
        }
        Some(PanelAction::None)
    }

    /// The token of the chosen place, while it shows.
    pub(super) fn focus(&self) -> u16 {
        if self.search_focus {
            return SEARCH_TOKEN;
        }
        match self.chosen.checked_sub(self.first) {
            Some(slot) if slot < ROWS_SHOWN && self.chosen < self.places.len() => {
                ROW_BASE + slot as u16
            }
            _ => BACK_TOKEN,
        }
    }
}

impl Panel {
    /// What the Inventory tab lists this frame, from the own profile (a world shot's cards,
    /// credits and shards in their place).
    pub(super) fn follow_inventory(&mut self, inputs: &Inputs<'_>) {
        let snapshot = inputs.snapshot.filter(|_| inputs.enabled);
        let me = snapshot.and_then(|snapshot| snapshot.me.as_ref());
        let (credits, shards) = me.map_or((0, 0), |me| (me.credits, me.shards));
        // A world shot's own in their place.
        #[cfg(test)]
        let (credits, shards) = self
            .preview_cards
            .as_ref()
            .map_or((credits, shards), |(_, credits, shards)| {
                (*credits, *shards)
            });
        let mut crystals = [0; crate::kyber::COUNT];
        for holocron in me
            .map(|me| me.holocrons.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|holocron| holocron.is_sealed())
        {
            if let Some(tier) = crate::kyber::from_id(&holocron.tier) {
                crystals[tier.index] += 1;
            }
        }
        let faces = crate::cards::faces::current();
        let stacks = self.holdings(inputs).unwrap_or_default();
        let unlocks = match inputs.holdings {
            Holdings::Known(unlocks) => unlocks,
            _ => &[],
        };
        let sources = Sources {
            credits,
            shards,
            coins: snapshot
                .and_then(|snapshot| snapshot.wheel_of_doom.as_ref())
                .map(|wheel| wheel.coins),
            holocrons: me.map_or([0; crate::holocrons::COUNT], |me| {
                crate::holocrons::counts_of(&me.holocron_counts)
            }),
            crystals,
            unlocks,
            gif_tags: crate::tags::gif_tags(me).map_or(0, <[_]>::len),
            cards: card_items(&stacks, &faces),
        };
        let items = if inputs.holdings.reason().is_none() {
            items(&sources)
        } else {
            Vec::new()
        };
        self.inventory.refresh(items);
    }

    /// Search the Inventory tab for `query` and sort it the `sort`th way
    /// ([`Sort::ALL`]), for a world shot.
    #[cfg(test)]
    pub(crate) fn inventory_for_shot(&mut self, query: &str, sort: usize) {
        self.inventory.query = query.to_owned();
        self.inventory.sort = Sort::ALL[sort.min(Sort::ALL.len() - 1)];
        self.inventory.changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlock(id: &str, count: u32) -> sjk_identity::Unlock {
        sjk_identity::Unlock {
            id: id.to_owned(),
            granted: 0,
            note: String::new(),
            count,
            medal: None,
            card: None,
            holo: 0,
            solar: 0,
        }
    }

    fn item(section: Section, name: &str, rank: u8, count: u64) -> Item {
        Item {
            section,
            name: name.to_owned(),
            tier: unlockables::Rarity::Common.label(),
            rank,
            colour: Color::new(1.0, 1.0, 1.0, 1.0),
            count,
            note: String::new(),
        }
    }

    fn names(items: &[Item], places: &[Place]) -> Vec<String> {
        places
            .iter()
            .map(|place| match place {
                Place::Heading { section, .. } => format!("# {}", section.label()),
                Place::Item(index) => items[*index].name.clone(),
            })
            .collect()
    }

    /// Every kind the profile holds gets its row, under its group: the currencies always,
    /// the DOOM coins once read, the sealed holocrons and crystals by tier, a shader with
    /// its copies and number, a held tag with its finishes, the starter tags, GIF tags.
    #[test]
    fn every_kind_held_is_listed_in_its_group() {
        let sun = unlockables::blade_skin("saber_sun").unwrap();
        let tag = crate::tags::CATALOGUE.iter().find(|tag| !tag.free).unwrap();
        let mut tag_grant = unlock(tag.id, 3);
        tag_grant.holo = 1;
        tag_grant.solar = 1;
        let unlocks = [
            unlock("saber_sun", 2),
            tag_grant,
            unlock("saber_none_known", 1),
        ];
        let sources = Sources {
            credits: 1_200,
            shards: 40,
            coins: Some(2),
            holocrons: [3, 0, 1, 0],
            crystals: [0, 1, 0, 0],
            unlocks: &unlocks,
            gif_tags: 2,
            cards: Vec::new(),
        };
        let items = items(&sources);
        let find = |name: &str| items.iter().find(|item| item.name == name).unwrap();
        assert_eq!(find("Credits").count, 1_200);
        assert_eq!(find("Kyber shards").count, 40);
        assert_eq!(find("DOOM coins").count, 2);
        let rare_crystal = find(crate::kyber::TIERS[1].title().as_ref());
        assert_eq!(
            (rare_crystal.section, rare_crystal.count),
            (Section::Crystals, 1)
        );
        let holocrons: Vec<_> = items
            .iter()
            .filter(|item| item.section == Section::Holocrons)
            .map(|item| (item.tier, item.count))
            .collect();
        assert_eq!(holocrons, [("Uncommon", 3), ("Legendary", 1)]);
        let shader = find(sun.name);
        assert_eq!(shader.section, Section::Shaders);
        assert_eq!(shader.count, 2);
        assert_eq!(shader.note, format!("Blade {}", sun.number_text()));
        let held = find(tag.name);
        assert_eq!((held.count, held.note.as_str()), (3, "1 Holo, 1 Solar"));
        let starters = crate::tags::CATALOGUE.iter().filter(|tag| tag.free).count();
        let tags = items
            .iter()
            .filter(|item| item.section == Section::Tags)
            .count();
        assert_eq!(
            tags,
            starters + 2,
            "the starter set, the held tag and GIF tags"
        );
        assert!(items.iter().all(|item| item.name != "saber_none_known"));
        // Unknown coins and nothing sealed: no row for them.
        let none = super::items(&Sources::default());
        assert!(none.iter().all(|item| item.name != "DOOM coins"));
        assert!(none.iter().all(|item| item.section != Section::Holocrons));
        assert_eq!(
            none.iter()
                .filter(|item| item.section == Section::Currencies)
                .count(),
            2
        );
    }

    /// The groups keep the list's order whatever the sort; within each, Tier puts the
    /// rarest first, Name goes A to Z, Quantity the most held first.
    #[test]
    fn groups_stand_in_order_and_sort_within() {
        let items = vec![
            item(Section::Shaders, "Beta", 1, 5),
            item(Section::Currencies, "Credits", 0, 10),
            item(Section::Shaders, "alpha", 4, 1),
            item(Section::Shaders, "Gamma", 2, 9),
            item(Section::Cards, "Card", 0, 1),
        ];
        let tier = places(&items, "", Sort::Tier);
        assert_eq!(
            names(&items, &tier),
            [
                "# Currencies",
                "Credits",
                "# Saber shaders",
                "alpha",
                "Gamma",
                "Beta",
                "# Cards",
                "Card"
            ]
        );
        assert_eq!(
            tier[2],
            Place::Heading {
                section: Section::Shaders,
                kinds: 3,
                quantity: 15
            }
        );
        let name = places(&items, "", Sort::Name);
        assert_eq!(names(&items, &name)[3..6], ["alpha", "Beta", "Gamma"]);
        let quantity = places(&items, "", Sort::Quantity);
        assert_eq!(names(&items, &quantity)[3..6], ["Gamma", "Beta", "alpha"]);
    }

    /// The search keeps the things holding every word, in their name, tier, kind or note,
    /// whatever the case, and drops the headings of emptied groups.
    #[test]
    fn the_search_keeps_every_word_and_drops_empty_groups() {
        let mut shiny = item(Section::Tags, "Fox", 0, 1);
        shiny.note = "1 Holo".to_owned();
        let items = vec![
            item(Section::Currencies, "Credits", 0, 10),
            item(Section::Shaders, "Sun", 4, 1),
            shiny,
        ];
        assert_eq!(
            names(&items, &places(&items, "sun", Sort::Tier)),
            ["# Saber shaders", "Sun"]
        );
        assert_eq!(
            names(&items, &places(&items, "  HOLO  fox", Sort::Tier)),
            ["# Tags", "Fox"]
        );
        assert_eq!(
            names(&items, &places(&items, "tags", Sort::Tier)),
            ["# Tags", "Fox"]
        );
        assert_eq!(
            names(&items, &places(&items, "common", Sort::Tier)).len(),
            6
        );
        assert!(places(&items, "sun fox", Sort::Tier).is_empty());
    }

    /// The choice steps over headings, the search's typing goes through the shared field
    /// (Backspace edits), Escape clears it, and S changes the order.
    #[test]
    fn keys_choose_past_headings_and_edit_the_search() {
        let mut inventory = Inventory::default();
        inventory.refresh(vec![
            item(Section::Currencies, "Credits", 0, 10),
            item(Section::Shaders, "Sun", 4, 1),
            item(Section::Shaders, "Storm", 3, 1),
        ]);
        assert_eq!(inventory.chosen_item().unwrap().name, "Credits");
        let _ = inventory.key(None, KeyCode::ArrowDown, false, false);
        assert_eq!(inventory.chosen_item().unwrap().name, "Sun");
        let _ = inventory.key(None, KeyCode::ArrowUp, false, false);
        assert_eq!(inventory.chosen_item().unwrap().name, "Credits");
        let _ = inventory.key(None, KeyCode::End, false, false);
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
        let _ = inventory.key(None, KeyCode::KeyS, false, false);
        assert_eq!(inventory.sort, Sort::Name);
        assert_eq!(inventory.chosen_item().unwrap().name, "Credits");
        // Ctrl+Tab is the page's.
        assert!(inventory.key(None, KeyCode::Tab, false, true).is_none());
        let _ = inventory.key(Some("/"), KeyCode::Slash, false, false);
        assert!(inventory.search_focus);
        for (text, key) in [("s", KeyCode::KeyS), ("t", KeyCode::KeyT)] {
            let _ = inventory.key(Some(text), key, false, false);
        }
        assert_eq!(inventory.query, "st");
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
        let _ = inventory.key(None, KeyCode::Backspace, false, false);
        assert_eq!(inventory.query, "s");
        // (Ctrl+A, Ctrl+C and Ctrl+V are the shared field's, tested in `text_entry`: its
        // selection is process-wide, so it is not raced here.)
        let _ = inventory.key(None, KeyCode::Escape, false, false);
        assert!(!inventory.search_focus);
        assert!(inventory.query.is_empty());
        assert_eq!(inventory.places.len(), 5);
    }

    /// Reading the list again keeps the choice on the same thing, wherever it now stands.
    #[test]
    fn a_refresh_keeps_the_thing_chosen() {
        let mut inventory = Inventory::default();
        let list = vec![
            item(Section::Shaders, "Sun", 4, 1),
            item(Section::Shaders, "Storm", 3, 1),
        ];
        inventory.refresh(list.clone());
        inventory.step(1);
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
        let mut more = vec![item(Section::Currencies, "Credits", 0, 1)];
        more.extend(list);
        inventory.refresh(more);
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
    }
}
