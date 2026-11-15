//! Collection > Inventory (Sol, 15/11/2026: "plain and simple, one list of everything the
//! player owns"; pictured rows and the item card, idea 1 of the Collection's ideas round
//! the same day). At the top a wallet strip: the credits, kyber shards and, once the hub
//! said, DOOM coins, each with how it is earned, and what the spare kybers are worth. Under
//! it the kind chips (All, then each kind, keys 1 to 7, each with how many), the sort and
//! the search; then the list, the holocrons and dormant kybers still sealed, the kybers,
//! finishers and tags held and the cards, grouped by kind under a heading, each row its
//! picture, its name in its tier's colour, the tier's mark and name, a note and how many,
//! in its tier's frame ([`crate::menu::sjk::kit::tier_frame`]). The right third holds the
//! chosen thing's card: its picture, tier, edition, name, how it came when known, the
//! copies and spares, what it is worth in kyber shards (the hub's tables,
//! [`crate::economy`]) and what can be done with it, by kind ([`offers`]): a kyber is worn,
//! a spare broken down (the Forge's own press, asked twice) or shown on Kybers; a holocron
//! opened and a dormant kyber attuned on the Holocrons tab; a tag chosen; a finisher worn;
//! a card shown on Cards. A button that cannot act says why under it, as the Forge does.
//!
//! S orders each group by tier, name or quantity; the search field (`/`, Ctrl+F;
//! [`crate::text_entry`]) keeps the rows whose name, tier, kind or note hold every word
//! typed. It reads only what the own hub profile already gives the other tabs
//! (`Profile::credits`, `shards`, `holocron_counts`, `holocrons`, `unlocks`, `gif_tags`,
//! `cards`) and the WHEEL OF DOOM's last read; it asks the hub nothing new. The look is
//! [`super::inventory_view`].

use super::*;
use crate::unlockables::Rarity;
use sjk_ui::Color;

/// The sort's segments ([`Sort::ALL`]), the kind chips ([`Filter::ALL`]), the search
/// field, the card's buttons and the list's rows.
pub(super) const SORT_BASE: u16 = 8_400;
pub(super) const CHIP_BASE: u16 = 8_403;
pub(super) const SEARCH_TOKEN: u16 = 8_410;
pub(super) const ACTION_BASE: u16 = 8_411;
pub(super) const ROW_BASE: u16 = 8_420;
/// How many places of the list show at once.
pub(super) const ROWS_SHOWN: usize = 14;
/// The most buttons the card offers.
pub(super) const OFFERS_MAX: usize = 3;
const _: () = assert!(CHIP_BASE + Filter::ALL.len() as u16 <= SEARCH_TOKEN);
const _: () = assert!(ACTION_BASE + OFFERS_MAX as u16 <= ROW_BASE);
const _: () = assert!(ROW_BASE as usize + ROWS_SHOWN <= 8_500);

/// The most characters the search holds.
const SEARCH_MAX: usize = 48;

/// The kinds of thing, in the list's order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Section {
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
            Self::Holocrons => crate::terms::say("Holocrons"),
            Self::Crystals => "Dormant kybers".into(),
            Self::Shaders => "Kybers".into(),
            Self::Finishers => "Finishers".into(),
            Self::Tags => "Tags".into(),
            Self::Cards => "Cards".into(),
        }
    }

    /// Its chip's word, short enough for the row of chips.
    fn chip(self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Crystals => "Dormant".into(),
            other => other.label(),
        }
    }
}

/// Which kinds the list shows: every one, or one alone (a chip, keys 1 to 7).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Filter {
    #[default]
    All,
    Only(Section),
}

impl Filter {
    /// The chips, in their order (key 1 the first).
    pub(super) const ALL: [Self; 7] = [
        Self::All,
        Self::Only(Section::Holocrons),
        Self::Only(Section::Crystals),
        Self::Only(Section::Shaders),
        Self::Only(Section::Finishers),
        Self::Only(Section::Tags),
        Self::Only(Section::Cards),
    ];

    pub(super) fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|filter| *filter == self)
            .unwrap_or(0)
    }

    fn keeps(self, section: Section) -> bool {
        match self {
            Self::All => true,
            Self::Only(only) => only == section,
        }
    }

    /// Its chip's word.
    fn word(self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::All => "All".into(),
            Self::Only(section) => section.chip(),
        }
    }
}

/// The chips' words, each with how many things of its kind are held in all: "Kybers 14".
pub(super) fn chip_labels(items: &[Item]) -> Vec<String> {
    Filter::ALL
        .iter()
        .map(|filter| {
            let quantity: u64 = items
                .iter()
                .filter(|item| filter.keeps(item.section))
                .map(|item| item.count)
                .sum();
            format!("{} {}", filter.word(), crate::cards::grouped(quantity))
        })
        .collect()
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

/// What a row is, for its picture and what its card offers.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Thing {
    /// Sealed holocrons of the Holocrons tab's tier at this slot ([`crate::holocrons::slot`]).
    Holocron(usize),
    /// Dormant kybers of the tier at this slot (after the holocrons').
    Crystal(usize),
    /// A kyber (a saber shader), by id.
    Kyber(&'static str),
    /// A finisher, by id ([`crate::finishers`]).
    Finisher(&'static str),
    /// A tag of the catalogue, by id, and since when it is held (unix seconds, 0 unknown).
    Tag(&'static str),
    /// The GIF tags the player made.
    GifTags,
    /// A kind of card copy: its face in the packs (none while sealed or unknown), its
    /// finish's place in [`crate::cards::FINISHES`], its tile on the Cards tab with no
    /// filter (for Show in Cards) and when the first came (unix seconds).
    Card {
        face: Option<usize>,
        finish: usize,
        tile: Option<usize>,
        first: i64,
    },
}

/// One thing held: a row of the list.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Item {
    pub(super) section: Section,
    pub(super) name: String,
    /// Its tier's name ("Mythical"), empty for none.
    pub(super) tier: &'static str,
    /// Its tier's rank, the commonest 0 ([`unlockables::Rarity`]'s order), for the sort.
    pub(super) rank: u8,
    /// Its tier on the ladder, for its frame and mark; `None` for no tier (GIF tags).
    pub(super) rarity: Option<Rarity>,
    pub(super) colour: Color,
    pub(super) count: u64,
    /// A word more: the finishes held, the edition, a blade's number.
    pub(super) note: String,
    pub(super) thing: Thing,
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

/// The balances over the list, and what the spare kybers would give.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Wallet {
    pub(super) credits: u64,
    pub(super) shards: u64,
    /// DOOM coins, once the hub said (the WHEEL OF DOOM's read).
    pub(super) coins: Option<u64>,
    /// The spare copies (one of each kept, never a locked one, a medal's or a card's) and
    /// the kyber shards breaking them all down gives ([`super::forge::duplicates`]).
    pub(super) spare_copies: u32,
    pub(super) spare_shards: u64,
}

/// The rank of an unlockable's tier.
fn rank(tier: Rarity) -> u8 {
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
    for (tier, count) in crate::holocrons::TIERS.iter().zip(sources.holocrons) {
        if count > 0 {
            items.push(Item {
                section: Section::Holocrons,
                name: tier.title().into_owned(),
                tier: tier.label,
                rank: tier.index as u8 + 1,
                rarity: Some(crate::menu::sjk::kit::holocron_tier(tier.index)),
                colour: tier.colour,
                count: count.into(),
                note: String::new(),
                thing: Thing::Holocron(tier.slot),
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
                rarity: Some(crate::menu::sjk::kit::holocron_tier(tier.index)),
                colour: tier.colour,
                count: count.into(),
                note: String::new(),
                thing: Thing::Crystal(tier.slot),
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
                rarity: Some(skin.tier),
                colour: unlockables::name_colour(skin.id).unwrap_or(skin.tier.colour()),
                count: unlock.count.into(),
                note: format!("Blade {}", skin.number_text()),
                thing: Thing::Kyber(skin.id),
            });
        } else if let Some(finisher) = crate::finishers::get(&unlock.id) {
            items.push(Item {
                section: Section::Finishers,
                name: finisher.name.to_owned(),
                tier: finisher.tier.label(),
                rank: rank(finisher.tier),
                rarity: Some(finisher.tier),
                colour: finisher.tier.colour(),
                count: unlock.count.into(),
                note: String::new(),
                thing: Thing::Finisher(finisher.id),
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
            rarity: Some(tag.tier),
            colour: tag.tier.colour(),
            count,
            note,
            thing: Thing::Tag(tag.id),
        });
    }
    if sources.gif_tags > 0 {
        items.push(Item {
            section: Section::Tags,
            name: "GIF tags".to_owned(),
            tier: "",
            rank: 0,
            rarity: None,
            colour: crate::menu::sjk::color::TEXT,
            count: sources.gif_tags as u64,
            note: "Made from blank tags".to_owned(),
            thing: Thing::GifTags,
        });
    }
    items.extend(sources.cards.iter().cloned());
    items
}

/// The cards of `stacks`, named from the packs' `faces`: a row to each kind of copy, with
/// its tile among the Cards tab's unfiltered `tiles` ([`crate::cards::binder::tiles`]).
pub(super) fn card_items(
    stacks: &[crate::cards::Stack],
    faces: &crate::cards::faces::Faces,
    tiles: &[crate::cards::binder::Tile],
) -> Vec<Item> {
    use crate::cards::binder::Slot;
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
            let face = stack
                .card
                .as_deref()
                .and_then(|id| faces.find(id))
                .map(|(index, _)| index);
            let tile = tiles
                .iter()
                .position(|tile| match (&tile.slot, &stack.card) {
                    (Slot::Sealed, None) => tile.rarity.id == stack.rarity.id,
                    (Slot::Card(index), Some(_)) => face == Some(*index),
                    (Slot::Unknown(id), Some(card)) => id == card,
                    _ => false,
                });
            Item {
                section: Section::Cards,
                name: stack.name_in(faces),
                tier: stack.rarity.name,
                rank: stack.rarity.index as u8,
                rarity: Some(crate::menu::sjk::kit::card_tier(stack.rarity.index)),
                colour: stack.rarity.colour,
                count: stack.count.into(),
                note: note.join(", "),
                thing: Thing::Card {
                    face,
                    finish: stack.finish.index,
                    tile,
                    first: stack.first,
                },
            }
        })
        .collect()
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

/// The list of `items` that `query` and `filter` keep, a heading over each kind that has
/// any, each group in `sort`'s order.
pub(super) fn places(items: &[Item], query: &str, sort: Sort, filter: Filter) -> Vec<Place> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut kept: Vec<usize> = (0..items.len())
        .filter(|index| filter.keeps(items[*index].section) && matches(&items[*index], &words))
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

/// What a kyber is worth in kyber shards, from the hub's tables ([`crate::economy`]):
/// breaking a copy down, crafting one, and the spares a fusion of its tier takes; all
/// `None` for a kyber outside the trade (a Unique one, a medal's, one holocrons do not drop).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Worth {
    pub(super) shatter: Option<u64>,
    pub(super) craft: Option<u64>,
    pub(super) fuse: Option<u32>,
}

/// What `skin` is worth ([`Worth`]).
pub(super) fn worth(skin: &unlockables::Unlockable) -> Worth {
    if !crate::economy::in_trade(skin) {
        return Worth::default();
    }
    Worth {
        shatter: Some(crate::economy::shatter_yield(skin.tier)).filter(|shards| *shards > 0),
        craft: Some(crate::economy::craft_cost(skin.tier)).filter(|shards| *shards > 0),
        fuse: crate::economy::fuse_need(skin.tier),
    }
}

/// What a button of the card does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Act {
    /// Wear this kyber (`cg_saberSkin`, as Kybers' Equip).
    Wear(&'static str),
    /// Wear this finisher, or take it off (`""`), as the Finishers page's cards.
    WearFinisher(&'static str),
    /// Choose this tag in this finish (`cg_tag`, `cg_tagFinish`), as the Tags card.
    Choose(&'static str, &'static str),
    /// Break a spare copy of this kyber down: the Forge's press, asked twice.
    Break(&'static str),
    /// Show this kyber on the Kybers tab.
    ShowKyber(&'static str),
    /// Open (or attune) the oldest at this slot on the Holocrons tab.
    Open(usize),
    /// Show this tile on the Cards tab.
    ShowCard(usize),
    /// Show the Tags tab (the GIF tags).
    ShowTags,
}

/// What `act` does, for the keys line ("wear", "break down").
pub(super) fn verb(act: Act) -> &'static str {
    match act {
        Act::Wear(_) | Act::WearFinisher(_) if !matches!(act, Act::WearFinisher("")) => "wear",
        Act::WearFinisher(_) | Act::Wear(_) => "take off",
        Act::Choose(..) => "choose",
        Act::Break(_) => "break down",
        Act::ShowKyber(_) | Act::ShowCard(_) | Act::ShowTags => "show",
        Act::Open(slot) if crate::holocrons::slot(slot).is_kyber() => "attune",
        Act::Open(_) => "open",
    }
}

/// A button of the card: what it does, its words, its key, whether it can act and, when
/// it cannot, why.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Offer {
    pub(super) act: Act,
    pub(super) label: String,
    pub(super) key: &'static str,
    pub(super) enabled: bool,
    pub(super) why: Option<String>,
}

impl Offer {
    fn new(act: Act, label: impl Into<String>, key: &'static str) -> Self {
        Self {
            act,
            label: label.into(),
            key,
            enabled: true,
            why: None,
        }
    }

    /// The button greyed, with why.
    fn refused(mut self, why: impl Into<String>) -> Self {
        self.enabled = false;
        self.why = Some(why.into());
        self
    }

    /// The button greyed under words that say why themselves ("Worn", "In use"), leaving
    /// the line under the buttons to the next one's reason.
    fn done(mut self, label: &str) -> Self {
        self.enabled = false;
        label.clone_into(&mut self.label);
        self
    }
}

/// What the card's buttons need to know beyond the thing chosen, read as it is drawn.
#[derive(Clone, Debug, Default)]
pub(super) struct Standing {
    /// The kyber, finisher or tag is worn or in use now.
    pub(super) worn: bool,
    /// The kyber's copies the player's own (a medal's or a card's not counted) and whether
    /// one of them came with a medal or a card.
    pub(super) own: u32,
    pub(super) bound: bool,
    /// The kyber is locked (`crate::shader_marks`).
    pub(super) locked: bool,
    /// The Forge may trade now, and why not.
    pub(super) can_trade: bool,
    pub(super) why_not: Option<String>,
    /// A trade sent from the Forge waits for the hub, and it is this kyber's break down;
    /// Break down is armed for it (a second press sends it).
    pub(super) waiting: bool,
    pub(super) sending: bool,
    pub(super) armed: bool,
    /// The hub lets the player open holocrons now, and why not.
    pub(super) can_open: bool,
    pub(super) open_why: Option<String>,
    /// The tag's finish to choose: the one in use when it is, else Normal.
    pub(super) finish: &'static str,
}

/// The words of a greyed Break down.
const NOT_TRADED: &str = "Not part of the trade: it is never broken down.";
const ONLY_COPY: &str = "Your only copy: the Forge breaks a last copy down after asking.";
const BOUND_COPY: &str = "It came with a medal or a card and stays with it.";
const LOCKED: &str = "Locked: unlock it in the Forge (L) to break it down.";

/// What the card offers for `item`, standing as `standing` says: the first button is the
/// main one (Enter), then Break down (B) and a way to its kind's page (G).
pub(super) fn offers(item: &Item, standing: &Standing) -> Vec<Offer> {
    let mut offers = Vec::with_capacity(OFFERS_MAX);
    match item.thing {
        Thing::Kyber(id) => {
            let wear = Offer::new(Act::Wear(id), "Wear", "Enter");
            offers.push(if standing.worn {
                wear.done("Worn")
            } else {
                wear
            });
            let gives = unlockables::find(id)
                .map(worth)
                .and_then(|worth| worth.shatter);
            let label = match gives {
                Some(_) if standing.sending => "Breaking down...".to_owned(),
                Some(gives) if standing.armed => {
                    format!("Confirm: +{} shards", crate::economy::shards_text(gives))
                }
                Some(gives) => format!("Break down  +{}", crate::economy::shards_text(gives)),
                None => "Break down".to_owned(),
            };
            let shatter = Offer::new(Act::Break(id), label, "B");
            let why = if gives.is_none() {
                Some(NOT_TRADED.to_owned())
            } else if standing.locked {
                Some(LOCKED.to_owned())
            } else if standing.own <= 1 && standing.bound {
                Some(BOUND_COPY.to_owned())
            } else if standing.own <= 1 {
                Some(ONLY_COPY.to_owned())
            } else if !standing.can_trade {
                Some(
                    standing
                        .why_not
                        .clone()
                        .unwrap_or_else(|| "The SJK hub does not trade now.".to_owned()),
                )
            } else if standing.waiting {
                Some("Asking the SJK hub...".to_owned())
            } else {
                None
            };
            offers.push(match why {
                Some(why) => shatter.refused(why),
                None => shatter,
            });
            offers.push(Offer::new(Act::ShowKyber(id), "Show in Kybers", "G"));
        }
        Thing::Holocron(slot) | Thing::Crystal(slot) => {
            let verb = if matches!(item.thing, Thing::Crystal(_)) {
                "Attune"
            } else {
                "Open"
            };
            let open = Offer::new(Act::Open(slot), verb, "Enter");
            offers.push(if standing.can_open {
                open
            } else {
                let why = standing
                    .open_why
                    .clone()
                    .unwrap_or_else(|| "The SJK hub does not open them now.".to_owned());
                open.refused(why)
            });
        }
        Thing::Tag(id) => {
            let choose = Offer::new(Act::Choose(id, standing.finish), "Choose", "Enter");
            offers.push(if standing.worn {
                choose.done("In use")
            } else if !crate::tags::stickable(id) {
                choose.refused("Spend it on a GIF on the Tags page.")
            } else {
                choose
            });
        }
        Thing::GifTags => offers.push(Offer::new(Act::ShowTags, "Show in Tags", "G")),
        Thing::Finisher(id) => offers.push(if standing.worn {
            Offer::new(Act::WearFinisher(""), "Take off", "Enter")
        } else {
            Offer::new(Act::WearFinisher(id), "Wear", "Enter")
        }),
        Thing::Card { tile, .. } => offers.push(match tile {
            Some(tile) => Offer::new(Act::ShowCard(tile), "Show in Cards", "G"),
            None => Offer::new(Act::ShowCard(0), "Show in Cards", "G")
                .refused("Its card is not in this client's packs yet."),
        }),
    }
    offers
}

/// The Inventory tab's state: its search, order, kind, choice and scroll, the list and
/// the wallet as last drawn, and the card's buttons as the last frame offered them.
#[derive(Debug, Default)]
pub(crate) struct Inventory {
    pub(super) query: String,
    pub(super) search_focus: bool,
    pub(super) sort: Sort,
    pub(super) filter: Filter,
    /// The place chosen (an item's, never a heading's while there is an item).
    pub(super) chosen: usize,
    /// The first place shown.
    pub(super) first: usize,
    pub(super) items: Vec<Item>,
    pub(super) places: Vec<Place>,
    pub(super) wallet: Wallet,
    pub(super) offers: Vec<Offer>,
}

impl Inventory {
    /// Read the list again from `items`, the choice kept on the same thing where it is
    /// still listed.
    pub(super) fn refresh(&mut self, items: Vec<Item>) {
        if items == self.items {
            return;
        }
        let was = self.chosen_item().map(|item| item.thing.clone());
        self.places = places(&items, &self.query, self.sort, self.filter);
        self.items = items;
        if let Some(was) = was
            && let Some(at) = self.places.iter().position(
                |place| matches!(place, Place::Item(index) if self.items[*index].thing == was),
            )
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

    /// The list's words, order or kind changed: back to its top.
    fn changed(&mut self) {
        self.places = places(&self.items, &self.query, self.sort, self.filter);
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

    /// Show the kind of chip `index` ([`Filter::ALL`]) alone, or every kind.
    pub(super) fn set_filter(&mut self, index: usize) {
        if let Some(filter) = Filter::ALL.get(index).copied()
            && filter != self.filter
        {
            self.filter = filter;
            self.changed();
        }
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
        let digit = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
        ]
        .iter()
        .position(|digit| *digit == key);
        match key {
            KeyCode::Slash => self.focus_search(),
            KeyCode::KeyF if control => self.focus_search(),
            _ if control => return None,
            _ if digit.is_some() => self.set_filter(digit.unwrap_or(0)),
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
    /// token that is not the list's (the card's buttons are the page's).
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
            .filter(|_| token < CHIP_BASE)
        {
            if activate && *sort != self.sort {
                self.sort = *sort;
                self.changed();
            }
            return Some(PanelAction::None);
        }
        if let Some(chip) = token
            .checked_sub(CHIP_BASE)
            .map(usize::from)
            .filter(|chip| *chip < Filter::ALL.len())
        {
            if activate {
                self.set_filter(chip);
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
    /// credits and shards in their place), and the wallet over it.
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
        let tiles = crate::cards::binder::tiles(&stacks, &faces);
        let unlocks = match inputs.holdings {
            Holdings::Known(unlocks) => unlocks,
            _ => &[],
        };
        let sources = Sources {
            holocrons: me.map_or([0; crate::holocrons::COUNT], |me| {
                crate::holocrons::counts_of(&me.holocron_counts)
            }),
            crystals,
            unlocks,
            gif_tags: crate::tags::gif_tags(me).map_or(0, <[_]>::len),
            cards: card_items(&stacks, &faces, &tiles),
        };
        let items = if inputs.holdings.reason().is_none() {
            items(&sources)
        } else {
            Vec::new()
        };
        self.inventory.refresh(items);
        self.follow_marks(inputs);
        let (spare_copies, _, spare_shards) =
            super::forge::duplicates(&super::forge::owned(&inputs.holdings, false, &self.marks));
        self.inventory.wallet = Wallet {
            credits,
            shards,
            coins: snapshot
                .and_then(|snapshot| snapshot.wheel_of_doom.as_ref())
                .map(|wheel| wheel.coins),
            spare_copies,
            spare_shards,
        };
    }

    /// How the chosen thing stands for its card's buttons, from `inputs`.
    pub(super) fn inventory_standing(&self, item: &Item, inputs: &Inputs<'_>) -> Standing {
        let now = Instant::now();
        let snapshot = inputs.snapshot.filter(|_| inputs.enabled);
        let (can_open, open_why) = match snapshot {
            Some(snapshot) if crate::economy::may_act(snapshot) => (true, None),
            Some(snapshot) => (
                false,
                Some(crate::economy::update_to(snapshot).map_or_else(
                    || {
                        crate::terms::say("The SJK hub does not let you open holocrons now.")
                            .into_owned()
                    },
                    crate::economy::update_line,
                )),
            ),
            None => (false, Some("Not connected to the SJK hub.".to_owned())),
        };
        let mut standing = Standing {
            can_trade: self.forge.data.can_trade,
            why_not: self.forge.data.why_not.clone(),
            waiting: self.forge.waiting(),
            can_open,
            open_why,
            finish: crate::cards::FINISHES[0].id,
            ..Standing::default()
        };
        match item.thing {
            Thing::Kyber(id) => {
                let unlock = inputs.holdings.unlock(id);
                let bound =
                    unlock.is_some_and(|unlock| unlock.medal.is_some() || unlock.card.is_some());
                standing.bound = bound;
                standing.own = if bound {
                    0
                } else {
                    unlock.map_or(0, |unlock| unlock.count)
                };
                standing.worn = inputs.setting.trim() == id;
                standing.locked = self.marks.locked(id);
                let press = super::forge::Press::Break(id);
                standing.armed = self.forge.is_armed(press, now);
                standing.sending =
                    standing.waiting && self.forge.sent.is_some_and(|(_, sent)| sent == press);
            }
            Thing::Finisher(id) => standing.worn = inputs.finisher.trim() == id,
            Thing::Tag(id) => {
                let in_use = {
                    let chosen = inputs.tag.trim();
                    if chosen.is_empty() {
                        crate::tags::DEFAULT
                    } else {
                        chosen
                    }
                };
                if in_use == id {
                    standing.worn = true;
                    let held = crate::tags::Held::of_parts(
                        id,
                        Some(match inputs.holdings {
                            Holdings::Known(unlocks) => unlocks,
                            _ => &[],
                        }),
                        snapshot.and_then(|snapshot| snapshot.me.as_ref()),
                    );
                    standing.finish = held.finish(inputs.tag_finish.trim()).id;
                }
            }
            _ => {}
        }
        standing
    }

    /// Carry out the card's button `index` as the last frame offered it, at `now`.
    pub(super) fn inventory_act(&mut self, index: usize, now: Instant) -> PanelAction {
        let Some(offer) = self.inventory.offers.get(index).cloned() else {
            return PanelAction::None;
        };
        if !offer.enabled {
            return PanelAction::None;
        }
        self.inventory.leave_search();
        match offer.act {
            Act::Wear(id) => {
                ui_cues_click();
                PanelAction::Wear(id)
            }
            Act::WearFinisher(id) => {
                ui_cues_click();
                PanelAction::WearFinisher(id)
            }
            Act::Choose(id, finish) => {
                ui_cues_click();
                PanelAction::ChooseTag(id.to_owned(), finish)
            }
            Act::Break(id) => self.forge.press(super::forge::Press::Break(id), now),
            Act::ShowKyber(id) => {
                self.show(Tab::Shaders);
                self.finishers_shown = false;
                self.shader_filter = ShaderFilter::default();
                self.shader_focus = Some(id);
                ui_cues_click();
                PanelAction::None
            }
            Act::Open(slot) => {
                ui_cues_click();
                PanelAction::OpenHolocron(slot)
            }
            Act::ShowCard(tile) => {
                self.card_filter = crate::cards::binder::Filter::default();
                self.card = tile;
                self.card_look = None;
                self.show(Tab::Cards);
                ui_cues_click();
                PanelAction::None
            }
            Act::ShowTags => {
                self.show(Tab::Tags);
                ui_cues_click();
                PanelAction::None
            }
        }
    }

    /// A key on the Inventory tab: the card's keys (Enter, B, G) on the thing chosen, then
    /// the list's; `None` for one it leaves to the page.
    pub(super) fn inventory_key(
        &mut self,
        text: Option<&str>,
        key: KeyCode,
        shift: bool,
        control: bool,
    ) -> Option<PanelAction> {
        if !self.inventory.search_focus && !control {
            let wanted = match key {
                KeyCode::Enter | KeyCode::NumpadEnter => Some("Enter"),
                KeyCode::KeyB => Some("B"),
                KeyCode::KeyG => Some("G"),
                _ => None,
            };
            if let Some(wanted) = wanted {
                let index = self
                    .inventory
                    .offers
                    .iter()
                    .position(|offer| offer.key == wanted);
                return Some(index.map_or(PanelAction::None, |index| {
                    self.inventory_act(index, Instant::now())
                }));
            }
        }
        self.inventory.key(text, key, shift, control)
    }

    /// A press on the card's button `token`, if it is one.
    pub(super) fn inventory_button(&mut self, token: u16, activate: bool) -> Option<PanelAction> {
        let index = token
            .checked_sub(ACTION_BASE)
            .map(usize::from)
            .filter(|index| *index < OFFERS_MAX)?;
        Some(if activate {
            self.inventory_act(index, Instant::now())
        } else {
            PanelAction::None
        })
    }

    /// Search the Inventory tab for `query` and sort it the `sort`th way
    /// ([`Sort::ALL`]), for a world shot.
    #[cfg(test)]
    pub(crate) fn inventory_for_shot(&mut self, query: &str, sort: usize) {
        self.inventory.query = query.to_owned();
        self.inventory.sort = Sort::ALL[sort.min(Sort::ALL.len() - 1)];
        self.inventory.changed();
    }

    /// Show chip `filter`'s kind ([`Filter::ALL`]) and choose its `nth` thing, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn inventory_choose_for_shot(&mut self, filter: usize, nth: usize) {
        self.inventory.query.clear();
        self.inventory.filter = Filter::ALL[filter.min(Filter::ALL.len() - 1)];
        self.inventory.changed();
        for _ in 0..nth {
            self.inventory.step(1);
        }
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
            rarity: Some(Rarity::Common),
            colour: Color::new(1.0, 1.0, 1.0, 1.0),
            count,
            note: String::new(),
            thing: Thing::GifTags,
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

    /// Every kind the profile holds gets its row, under its group: the sealed holocrons
    /// and crystals by tier, a kyber with its copies and number, a held tag with its
    /// finishes, the starter tags, GIF tags; each with its tier on the ladder.
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
            holocrons: [3, 0, 1, 0],
            crystals: [0, 1, 0, 0],
            unlocks: &unlocks,
            gif_tags: 2,
            cards: Vec::new(),
        };
        let items = items(&sources);
        let find = |name: &str| items.iter().find(|item| item.name == name).unwrap();
        let rare_crystal = find(crate::kyber::TIERS[1].title().as_ref());
        assert_eq!(
            (rare_crystal.section, rare_crystal.count),
            (Section::Crystals, 1)
        );
        assert_eq!(rare_crystal.rarity, Some(Rarity::Rare));
        assert_eq!(
            rare_crystal.thing,
            Thing::Crystal(crate::kyber::TIERS[1].slot)
        );
        let holocrons: Vec<_> = items
            .iter()
            .filter(|item| item.section == Section::Holocrons)
            .map(|item| (item.tier, item.count, item.rarity))
            .collect();
        assert_eq!(
            holocrons,
            [
                ("Uncommon", 3, Some(Rarity::Uncommon)),
                ("Legendary", 1, Some(Rarity::Legendary))
            ]
        );
        let shader = find(sun.name);
        assert_eq!(shader.section, Section::Shaders);
        assert_eq!(shader.count, 2);
        assert_eq!(shader.note, format!("Blade {}", sun.number_text()));
        assert_eq!(shader.thing, Thing::Kyber("saber_sun"));
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
        assert_eq!(find("GIF tags").rarity, None);
        assert!(items.iter().all(|item| item.name != "saber_none_known"));
        // Nothing sealed: no row for them.
        let none = super::items(&Sources::default());
        assert!(none.iter().all(|item| item.section != Section::Holocrons));
    }

    /// The groups keep the list's order whatever the sort; within each, Tier puts the
    /// rarest first, Name goes A to Z, Quantity the most held first.
    #[test]
    fn groups_stand_in_order_and_sort_within() {
        let items = vec![
            item(Section::Shaders, "Beta", 1, 5),
            item(Section::Holocrons, "Echo", 0, 10),
            item(Section::Shaders, "alpha", 4, 1),
            item(Section::Shaders, "Gamma", 2, 9),
            item(Section::Cards, "Card", 0, 1),
        ];
        let tier = places(&items, "", Sort::Tier, Filter::All);
        let holocrons = format!("# {}", Section::Holocrons.label());
        assert_eq!(
            names(&items, &tier),
            [
                holocrons.as_str(),
                "Echo",
                "# Kybers",
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
        let name = places(&items, "", Sort::Name, Filter::All);
        assert_eq!(names(&items, &name)[3..6], ["alpha", "Beta", "Gamma"]);
        let quantity = places(&items, "", Sort::Quantity, Filter::All);
        assert_eq!(names(&items, &quantity)[3..6], ["Gamma", "Beta", "alpha"]);
    }

    /// The kind chips: each says how many of its kind are held in all, keys 1 to 7 choose
    /// them, one shows its kind alone, and All every kind again.
    #[test]
    fn the_chips_count_and_filter_the_kinds() {
        let items = vec![
            item(Section::Shaders, "Sun", 4, 2),
            item(Section::Shaders, "Storm", 3, 1),
            item(Section::Tags, "Fox", 0, 1),
        ];
        let labels = chip_labels(&items);
        assert_eq!(labels.len(), 7);
        assert_eq!(labels[0], "All 4");
        assert_eq!(labels[3], "Kybers 3");
        assert_eq!(labels[5], "Tags 1");
        assert_eq!(labels[6], "Cards 0");
        assert_eq!(labels[2], "Dormant 0");
        let mut inventory = Inventory::default();
        inventory.refresh(items);
        let _ = inventory.key(None, KeyCode::Digit6, false, false);
        assert_eq!(inventory.filter, Filter::Only(Section::Tags));
        assert_eq!(
            names(&inventory.items, &inventory.places),
            ["# Tags", "Fox"]
        );
        assert_eq!(inventory.chosen_item().unwrap().name, "Fox");
        let _ = inventory.token(CHIP_BASE + 3, true, false);
        assert_eq!(inventory.filter, Filter::Only(Section::Shaders));
        assert_eq!(inventory.places.len(), 3);
        let _ = inventory.key(None, KeyCode::Digit1, false, false);
        assert_eq!(inventory.filter, Filter::All);
        assert_eq!(inventory.places.len(), 5);
        // While typing in the search, a digit is a letter of the search.
        inventory.focus_search();
        let _ = inventory.key(Some("4"), KeyCode::Digit4, false, false);
        assert_eq!(inventory.filter, Filter::All);
        assert_eq!(inventory.query, "4");
    }

    /// The search keeps the things holding every word, in their name, tier, kind or note,
    /// whatever the case, and drops the headings of emptied groups.
    #[test]
    fn the_search_keeps_every_word_and_drops_empty_groups() {
        let mut shiny = item(Section::Tags, "Fox", 0, 1);
        shiny.note = "1 Holo".to_owned();
        let items = vec![item(Section::Shaders, "Sun", 4, 1), shiny];
        let search = |query| names(&items, &places(&items, query, Sort::Tier, Filter::All));
        assert_eq!(search("sun"), ["# Kybers", "Sun"]);
        assert_eq!(search("  HOLO  fox"), ["# Tags", "Fox"]);
        assert_eq!(search("tags"), ["# Tags", "Fox"]);
        assert_eq!(search("common").len(), 4);
        assert!(search("sun fox").is_empty());
    }

    /// The choice steps over headings, the search's typing goes through the shared field
    /// (Backspace edits), Escape clears it, and S changes the order.
    #[test]
    fn keys_choose_past_headings_and_edit_the_search() {
        let mut inventory = Inventory::default();
        inventory.refresh(vec![
            item(Section::Holocrons, "Echo", 0, 10),
            item(Section::Shaders, "Sun", 4, 1),
            item(Section::Shaders, "Storm", 3, 1),
        ]);
        assert_eq!(inventory.chosen_item().unwrap().name, "Echo");
        let _ = inventory.key(None, KeyCode::ArrowDown, false, false);
        assert_eq!(inventory.chosen_item().unwrap().name, "Sun");
        let _ = inventory.key(None, KeyCode::ArrowUp, false, false);
        assert_eq!(inventory.chosen_item().unwrap().name, "Echo");
        let _ = inventory.key(None, KeyCode::End, false, false);
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
        let _ = inventory.key(None, KeyCode::KeyS, false, false);
        assert_eq!(inventory.sort, Sort::Name);
        assert_eq!(inventory.chosen_item().unwrap().name, "Echo");
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
        let mut sun = item(Section::Shaders, "Sun", 4, 1);
        sun.thing = Thing::Kyber("saber_sun");
        let mut storm = item(Section::Shaders, "Storm", 3, 1);
        storm.thing = Thing::Kyber("saber_storm");
        let list = vec![sun, storm];
        inventory.refresh(list.clone());
        inventory.step(1);
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
        let mut more = vec![item(Section::Holocrons, "Echo", 0, 1)];
        more.extend(list);
        inventory.refresh(more);
        assert_eq!(inventory.chosen_item().unwrap().name, "Storm");
    }

    /// A kyber's worth reads the hub's tables: a copy broken down, one crafted and the
    /// spares a fusion takes; a kyber outside the trade is worth nothing there.
    #[test]
    fn a_kybers_worth_comes_from_the_economy() {
        let traded = unlockables::blade_skins()
            .find(|skin| crate::economy::in_trade(skin) && skin.tier == Rarity::Legendary)
            .unwrap();
        let value = worth(traded);
        assert_eq!(
            value.shatter,
            Some(crate::economy::shatter_yield(Rarity::Legendary))
        );
        assert_eq!(
            value.craft,
            Some(crate::economy::craft_cost(Rarity::Legendary))
        );
        assert_eq!(value.fuse, crate::economy::fuse_need(Rarity::Legendary));
        let kept = unlockables::blade_skins()
            .find(|skin| !crate::economy::in_trade(skin))
            .unwrap();
        assert_eq!(worth(kept), Worth::default());
        // The wallet's spares: one copy of each kept, a locked one never counted.
        let a = traded.id;
        let b = unlockables::blade_skins()
            .filter(|skin| crate::economy::in_trade(skin) && skin.id != a)
            .nth(3)
            .unwrap();
        let unlocks = [unlock(a, 3), unlock(b.id, 2)];
        let marks = crate::shader_marks::Marks::default();
        let owned = super::super::forge::owned(&Holdings::Known(&unlocks), false, &marks);
        let (copies, kinds, shards) = super::super::forge::duplicates(&owned);
        assert_eq!((copies, kinds), (3, 2));
        assert_eq!(
            shards,
            2 * crate::economy::shatter_yield(traded.tier) + crate::economy::shatter_yield(b.tier)
        );
    }

    /// The card's buttons by kind: a kyber is worn, broken down (only a spare of the trade,
    /// never a locked one, a bound one or the last, saying why) and shown on Kybers; a
    /// holocron opened and a dormant kyber attuned while the hub lets them; a tag chosen
    /// unless in use; a finisher worn or taken off; a card shown on Cards.
    #[test]
    fn the_card_offers_what_each_kind_can_do() {
        let traded = unlockables::blade_skins()
            .find(|skin| crate::economy::in_trade(skin))
            .unwrap();
        let mut kyber = item(Section::Shaders, traded.name, 2, 3);
        kyber.thing = Thing::Kyber(traded.id);
        let free = Standing {
            own: 3,
            can_trade: true,
            can_open: true,
            finish: "normal",
            ..Standing::default()
        };
        let acts = |item: &Item, standing: &Standing| -> Vec<(Act, bool, &'static str)> {
            offers(item, standing)
                .into_iter()
                .map(|offer| (offer.act, offer.enabled, offer.key))
                .collect()
        };
        assert_eq!(
            acts(&kyber, &free),
            [
                (Act::Wear(traded.id), true, "Enter"),
                (Act::Break(traded.id), true, "B"),
                (Act::ShowKyber(traded.id), true, "G"),
            ]
        );
        let why = |standing: Standing| offers(&kyber, &standing)[1].why.clone();
        assert_eq!(
            why(Standing {
                locked: true,
                ..free.clone()
            })
            .as_deref(),
            Some(LOCKED)
        );
        assert_eq!(
            why(Standing {
                own: 1,
                ..free.clone()
            })
            .as_deref(),
            Some(ONLY_COPY)
        );
        assert_eq!(
            why(Standing {
                own: 0,
                bound: true,
                ..free.clone()
            })
            .as_deref(),
            Some(BOUND_COPY)
        );
        assert!(
            why(Standing {
                can_trade: false,
                why_not: Some("Staff only".to_owned()),
                ..free.clone()
            })
            .is_some_and(|why| why == "Staff only")
        );
        let armed = offers(
            &kyber,
            &Standing {
                armed: true,
                ..free.clone()
            },
        );
        assert!(armed[1].label.starts_with("Confirm"), "{}", armed[1].label);
        let worn = offers(
            &kyber,
            &Standing {
                worn: true,
                ..free.clone()
            },
        );
        assert!(!worn[0].enabled && worn[0].label == "Worn");
        let unique = unlockables::blade_skins()
            .find(|skin| !crate::economy::in_trade(skin))
            .unwrap();
        let mut kept = kyber.clone();
        kept.thing = Thing::Kyber(unique.id);
        assert_eq!(
            offers(&kept, &free)[1].why.as_deref(),
            Some(NOT_TRADED),
            "no Break down outside the trade"
        );
        // Holocrons open, dormant kybers attune, on the Holocrons tab.
        let mut echo = item(Section::Holocrons, "Echo", 1, 1);
        echo.thing = Thing::Holocron(1);
        assert_eq!(acts(&echo, &free), [(Act::Open(1), true, "Enter")]);
        let mut crystal = item(Section::Crystals, "Dormant", 1, 1);
        crystal.thing = Thing::Crystal(5);
        let attune = offers(&crystal, &free);
        assert_eq!(
            (attune[0].act, attune[0].label.as_str()),
            (Act::Open(5), "Attune")
        );
        let closed = offers(
            &crystal,
            &Standing {
                can_open: false,
                open_why: Some("Update SJK".to_owned()),
                ..free.clone()
            },
        );
        assert!(!closed[0].enabled && closed[0].why.as_deref() == Some("Update SJK"));
        // A tag is chosen unless in use.
        let tag = crate::tags::CATALOGUE
            .iter()
            .find(|tag| tag.id != crate::tags::BLANK)
            .unwrap();
        let mut row = item(Section::Tags, tag.name, 1, 1);
        row.thing = Thing::Tag(tag.id);
        assert_eq!(
            acts(&row, &free),
            [(Act::Choose(tag.id, "normal"), true, "Enter")]
        );
        let in_use = offers(
            &row,
            &Standing {
                worn: true,
                ..free.clone()
            },
        );
        assert!(!in_use[0].enabled);
        // A finisher is worn, or taken off when worn.
        let mut finisher = item(Section::Finishers, "Disintegrate", 3, 1);
        finisher.thing = Thing::Finisher("finisher_disintegrate");
        assert_eq!(
            acts(&finisher, &free),
            [(Act::WearFinisher("finisher_disintegrate"), true, "Enter")]
        );
        let off = offers(
            &finisher,
            &Standing {
                worn: true,
                ..free.clone()
            },
        );
        assert_eq!(off[0].act, Act::WearFinisher(""));
        // A card is shown on Cards, where its tile is.
        let mut card = item(Section::Cards, "Sol", 3, 1);
        card.thing = Thing::Card {
            face: Some(0),
            finish: 0,
            tile: Some(4),
            first: 0,
        };
        assert_eq!(acts(&card, &free), [(Act::ShowCard(4), true, "G")]);
        card.thing = Thing::Card {
            face: None,
            finish: 0,
            tile: None,
            first: 0,
        };
        assert!(!offers(&card, &free)[0].enabled);
        // Every kind offers at most the card's buttons.
        for thing in [kyber, echo, crystal, row, finisher, card] {
            assert!(offers(&thing, &free).len() <= OFFERS_MAX);
        }
    }
}
