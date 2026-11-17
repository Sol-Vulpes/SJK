//! The status bars (Sol's requests, 15/11/2026 and 16/11/2026: "stick it to the top or
//! bottom of the screen, not a bubble ... both enabled same time"): two thin strips
//! docked to the screen's edges over play, full width, flush with the edge, in the SJK
//! UI's look (navy glass, a holo hairline on the inner edge), their contents in a left,
//! a centre and a right group.
//!
//! - **The bottom bar** ([`CVAR`], `cg_statusBar`) carries what the player has and wears:
//!   on the left their credits, kyber shards, dormant kybers and DOOM coins (the hub's
//!   numbers, each an icon and its number), the Collection's New nudge ([`Nudge`]: "3 new
//!   in your Collection" for the rest of the map they arrived on, then a gold dot), the
//!   kyber on their saber with its tier's diamonds and what is on about them (the blade's
//!   extras, the finisher, Tactical mode, Do not disturb, away, toon shading); in the
//!   centre the weapons they hold, by their selection icons, the one in hand lit with its
//!   ammunition after the row; on the right every toy's switch, its picture lit while on,
//!   then the local time (24-hour) and the date (day first).
//! - **The top bar** ([`TOP_CVAR`], `cg_statusBarTop`) carries the match: on the left the
//!   Force powers known, the selected one lit and one in use dotted, and the player's
//!   place and score (the teams' scores in a team game); in the centre the time left when
//!   the server sets a time limit; on the right who joins and leaves ([`feed`]), the
//!   server's name, the map, the game type, the players on the server over its slots, the
//!   ping (the server's measure, as the scoreboard shows it, [`ping`]) and the frame rate.
//!   The Force powers' levels are not shown: protocol 26 sends only Jump's and Sense's,
//!   and the rest would be guesses.
//!
//! Both are on by default (Sol, 16/11/2026), each offered again in the News panel's
//! settings to review, and each part has its own switch ([`Part::cvar`], all on), so a
//! player keeps only what they want. A part without a value (no hub, no time limit) is
//! left out rather than shown empty; when a bar runs out of room, the parts that matter
//! least go first ([`Part::keep`], the toys and the server's name before the clock and
//! the time left, [`arrange`]). Tactical mode, which is there to fight, leaves the toys
//! and the Collection's nudge out.
//!
//! They show over play only, as the HUD's elements do: not under the menus or the
//! console, not at an intermission, not over a demo (they tell the player's own things,
//! not the recording's), nor while the HUD switch hides the elements (`cg_drawHud 0`).
//! They stay under the scoreboard, so the HUD does not jump each time it is held.
//!
//! **They keep their room** ([`Reserved`], `State::reserved`): rather than the bars
//! dodging the HUD, the HUD and every 2D element drawn against a screen edge move inside
//! them, the 3D view untouched. Each frame [`crate::GpuState::reserve_status_bars`] works
//! out, before the HUD is laid out, how tall each bar is (0 while it is off or hidden),
//! and the HUD's elements, the game HUD, the chat, the console's notify lines and the
//! other overlays move in by that much ([`Reserved::shift`] for an element nearer the
//! top or the bottom). Other modules read the room with
//! `GpuState.status_bar.reserved()`: the top and bottom heights in window pixels for this
//! frame. While a bar shows the frame rate, the match timer or the time and date, the
//! HUD's own readout of the same thing is not drawn ([`State::shows`]): `cg_drawFps`'s
//! rate (its frame times stay), the match timer (in the layouts and the upper right) and
//! `cg_drawClock`.
//!
//! What they show is read four times a second into fixed storage, the clock once a
//! second; a bar is laid out again only when what it shows, its switches or the screen
//! change, and a frame otherwise hands its kept shapes on, allocating nothing. The feed's
//! fading is counted in steps ([`feed::STEPS`]), so a fade lays the top bar out a few
//! times, not every frame.

mod feed;
mod pieces;
mod ping;
mod read;
mod times;

pub(crate) use pieces::{Effect, TOY_COUNT, toys};
#[cfg(test)]
pub(crate) use times::{Clock, Date, Left};

use crate::console::ViewerConsole;
use crate::menu::sjk::color;
use crate::menu_widgets::MenuCanvas;
use pieces::{Measures, Piece, Texts};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{DrawCommand, Rect, TextureId};
use std::time::{Duration, Instant};

/// The bottom bar's switch: on by default (Sol, 16/11/2026).
pub(crate) const CVAR: &str = "cg_statusBar";
/// The top bar's switch: on by default (Sol, 16/11/2026).
pub(crate) const TOP_CVAR: &str = "cg_statusBarTop";

/// How often what the bars show is read again; the clock is read once a second.
const EVERY: Duration = Duration::from_millis(250);
const CLOCK_EVERY: Duration = Duration::from_secs(1);

/// A bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Bar {
    Bottom,
    Top,
}

/// Where a part stands on its bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Group {
    Left,
    Centre,
    Right,
}

/// A part of a bar, each with its own switch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Part {
    Credits,
    Shards,
    Crystals,
    Coins,
    /// New things in the Collection ([`Nudge`]).
    New,
    Saber,
    Effects,
    Weapons,
    Toys,
    Time,
    Date,
    Force,
    Score,
    TimeLeft,
    Feed,
    Server,
    Map,
    Mode,
    Players,
    Ping,
    Fps,
}

impl Part {
    /// Every part, the bottom bar's first, in the Settings rows' order.
    pub(crate) const ALL: [Self; 21] = [
        Self::Credits,
        Self::Shards,
        Self::Crystals,
        Self::Coins,
        Self::New,
        Self::Saber,
        Self::Effects,
        Self::Weapons,
        Self::Toys,
        Self::Time,
        Self::Date,
        Self::Force,
        Self::Score,
        Self::TimeLeft,
        Self::Feed,
        Self::Server,
        Self::Map,
        Self::Mode,
        Self::Players,
        Self::Ping,
        Self::Fps,
    ];

    /// Its switch: archived, on by default.
    pub(crate) const fn cvar(self) -> &'static str {
        match self {
            Self::Credits => "cg_statusBarCredits",
            Self::Shards => "cg_statusBarShards",
            Self::Crystals => "cg_statusBarCrystals",
            Self::Coins => "cg_statusBarCoins",
            Self::New => "cg_statusBarNew",
            Self::Saber => "cg_statusBarSaber",
            Self::Effects => "cg_statusBarEffects",
            Self::Weapons => "cg_statusBarWeapons",
            Self::Toys => "cg_statusBarToys",
            Self::Time => "cg_statusBarTime",
            Self::Date => "cg_statusBarDate",
            Self::Force => "cg_statusBarTopForce",
            Self::Score => "cg_statusBarTopScore",
            Self::TimeLeft => "cg_statusBarTopTimeLeft",
            Self::Feed => "cg_statusBarTopFeed",
            Self::Server => "cg_statusBarTopServer",
            Self::Map => "cg_statusBarTopMap",
            Self::Mode => "cg_statusBarTopMode",
            Self::Players => "cg_statusBarTopPlayers",
            Self::Ping => "cg_statusBarTopPing",
            Self::Fps => "cg_statusBarTopFps",
        }
    }

    /// The switch's line in `cvarlist`.
    const fn help(self) -> &'static str {
        match self {
            Self::Credits => "Bottom bar shows your credits",
            Self::Shards => "Bottom bar shows your kyber shards",
            Self::Crystals => "Bottom bar shows your dormant kybers still to attune",
            Self::Coins => "Bottom bar shows your DOOM coins",
            Self::New => "Bottom bar says when your Collection holds new things",
            Self::Saber => "Bottom bar shows the kyber on your saber",
            Self::Effects => {
                "Bottom bar shows what is on about you: blade extras, finisher, Tactical, Do not disturb, away"
            }
            Self::Weapons => {
                "Bottom bar shows the weapons you hold, the one in hand lit with its ammunition"
            }
            Self::Toys => "Bottom bar shows every toy's switch, lit while on",
            Self::Time => "Bottom bar shows the local time, 24-hour",
            Self::Date => "Bottom bar shows the date, day first",
            Self::Force => "Top bar shows the Force powers you know, the selected one lit",
            Self::Score => "Top bar shows your place and score, or the teams' scores",
            Self::TimeLeft => "Top bar shows the match time left when the server sets a limit",
            Self::Feed => "Top bar tells who joins and leaves the server",
            Self::Server => "Top bar shows the server's name",
            Self::Map => "Top bar shows the map",
            Self::Mode => "Top bar shows the game type",
            Self::Players => "Top bar shows the players on the server and its slots",
            Self::Ping => "Top bar shows your ping",
            Self::Fps => "Top bar shows your frame rate",
        }
    }

    /// The bar it is on.
    pub(crate) const fn bar(self) -> Bar {
        match self {
            Self::Credits
            | Self::Shards
            | Self::Crystals
            | Self::Coins
            | Self::New
            | Self::Saber
            | Self::Effects
            | Self::Weapons
            | Self::Toys
            | Self::Time
            | Self::Date => Bar::Bottom,
            _ => Bar::Top,
        }
    }

    /// Where it stands on its bar.
    pub(crate) const fn group(self) -> Group {
        match self {
            Self::Credits
            | Self::Shards
            | Self::Crystals
            | Self::Coins
            | Self::New
            | Self::Saber
            | Self::Effects
            | Self::Force
            | Self::Score => Group::Left,
            Self::Weapons | Self::TimeLeft => Group::Centre,
            _ => Group::Right,
        }
    }

    /// How long it stays when its bar runs out of room: the lowest goes first. The toys,
    /// the effects' pills and the server's name before the clock and the time left.
    pub(crate) const fn keep(self) -> u8 {
        match self {
            Self::Toys => 1,
            Self::Effects => 2,
            Self::Coins => 3,
            Self::Crystals => 4,
            Self::Shards => 5,
            Self::New => 6,
            Self::Date => 7,
            Self::Saber => 8,
            Self::Credits => 9,
            Self::Weapons => 10,
            Self::Time => 11,
            Self::Server => 1,
            Self::Mode => 2,
            Self::Score => 3,
            Self::Ping => 4,
            Self::Feed => 5,
            Self::Players => 6,
            Self::Force => 7,
            Self::Map => 8,
            Self::Fps => 9,
            Self::TimeLeft => 10,
        }
    }

    /// Which of its pieces goes first when room runs out: its first (the feed's oldest
    /// entry) rather than its last.
    const fn drops_first_piece(self) -> bool {
        matches!(self, Self::Feed)
    }

    const fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// The parts switched on, a bit per [`Part`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Parts(u32);

impl Parts {
    /// Every part.
    #[cfg(test)]
    pub(crate) const ALL: Self = Self((1 << Part::ALL.len()) - 1);

    /// The parts whose switch is on in `console` (on when a switch is unread).
    pub(crate) fn read(console: Option<&ViewerConsole>) -> Self {
        Self::from_switches(|part| {
            console
                .and_then(|console| console.bool_cvar(part.cvar()))
                .unwrap_or(true)
        })
    }

    /// The parts `on` says are on.
    pub(crate) fn from_switches(on: impl Fn(Part) -> bool) -> Self {
        Self(
            Part::ALL
                .into_iter()
                .filter(|part| on(*part))
                .fold(0, |bits, part| bits | part.bit()),
        )
    }

    pub(crate) fn has(self, part: Part) -> bool {
        self.0 & part.bit() != 0
    }

    /// These parts but `part`.
    pub(crate) fn without(self, part: Part) -> Self {
        Self(self.0 & !part.bit())
    }

    /// Whether any part of `bar` is on.
    pub(crate) fn any_on(self, bar: Bar) -> bool {
        Part::ALL
            .into_iter()
            .any(|part| part.bar() == bar && self.has(part))
    }
}

/// Register the bars' options.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        CVAR,
        true,
        CvarFlags::ARCHIVE,
        "Bottom bar with your credits, kyber, effects, weapons, toys, time and date",
    ))?;
    cvars.register(CvarDefinition::new(
        TOP_CVAR,
        true,
        CvarFlags::ARCHIVE,
        "Top bar with your Force powers, score, time left, joins and leaves, server, map, players, ping and frame rate",
    ))?;
    for part in Part::ALL {
        cvars.register(CvarDefinition::new(
            part.cvar(),
            true,
            CvarFlags::ARCHIVE,
            part.help(),
        ))?;
    }
    Ok(())
}

/// What the bars keep of the screen's edges this frame, in window pixels: 0 for a bar
/// off or hidden. The HUD and the overlays drawn against an edge stay inside it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Reserved {
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl Reserved {
    /// No bar.
    pub(crate) const NONE: Self = Self {
        top: 0.0,
        bottom: 0.0,
    };

    /// How far down an element whose box is `rect` moves to stay inside the bars on a
    /// screen `height` tall: by the top bar's height when it stands nearer the top edge
    /// (its box's middle above the screen's), up by the bottom bar's otherwise, so each
    /// half of the HUD keeps its own arrangement.
    pub(crate) fn shift(self, rect: Rect, height: f32) -> f32 {
        if rect.y + rect.bottom() < height {
            self.top
        } else {
            -self.bottom
        }
    }
}

/// Text of at most `N` bytes (`N` below 256) kept inline, so what a bar shows copies and
/// compares without allocating. Longer text is cut at a character's boundary.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct Line<const N: usize> {
    bytes: [u8; N],
    len: u8,
}

impl<const N: usize> Default for Line<N> {
    fn default() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
}

impl<const N: usize> Line<N> {
    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> From<&str> for Line<N> {
    fn from(text: &str) -> Self {
        let room = N.min(usize::from(u8::MAX));
        let mut end = text.len().min(room);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut line = Self::default();
        line.bytes[..end].copy_from_slice(&text.as_bytes()[..end]);
        line.len = end as u8;
        line
    }
}

impl<const N: usize> std::fmt::Debug for Line<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

/// Most effects the bottom bar lists.
pub(crate) const MAX_EFFECTS: usize = 10;
/// Most weapons it lists (`LAST_USEABLE_WEAPON`).
pub(crate) const MAX_WEAPONS: usize = 16;
/// Most Force powers the top bar lists (the Force wheel's entries).
pub(crate) const MAX_POWERS: usize = sjk_client::force_wheel::MAX_SLOTS;

/// What the bar says of the Collection's New things (`crate::collection_seen`): "N new
/// in your Collection" for the rest of the map they arrived on (or the first map played
/// after they came between maps), then a gold dot alone until they are seen. More
/// arriving says it again.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Nudge {
    /// How many were New at the last read.
    count: usize,
    say: Say,
}

/// Where the words stand ([`Nudge`]).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Say {
    /// The dot alone (or nothing New).
    #[default]
    Quiet,
    /// New things came outside a map: the words wait for the next.
    Pending,
    /// The words show on this map (its [`map_key`]).
    During(u64),
}

impl Nudge {
    /// `count` things New now, on the map of key `map` (`None` outside one): how many to
    /// show and whether in words; `None` when nothing is New.
    pub(crate) fn follow(&mut self, count: usize, map: Option<u64>) -> Option<(u32, bool)> {
        if count == 0 {
            *self = Self::default();
            return None;
        }
        if count > self.count {
            self.say = map.map_or(Say::Pending, Say::During);
        }
        self.count = count;
        if let (Say::Pending, Some(map)) = (self.say, map) {
            self.say = Say::During(map);
        }
        let words = match self.say {
            Say::During(on) => map == Some(on),
            Say::Pending => true,
            Say::Quiet => false,
        };
        if !words {
            self.say = Say::Quiet;
        }
        Some((u32::try_from(count).unwrap_or(u32::MAX), words))
    }
}

/// A steady number for the map `path` played (`maps/<name>.bsp`), `None` for none.
pub(crate) fn map_key(path: &str) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    if path.is_empty() {
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    Some(hasher.finish())
}

/// What Tactical mode's pill says at `level`, none while it is off.
pub(crate) fn tactical_words(level: crate::tactical::Level) -> Option<&'static str> {
    use crate::tactical::Level;
    match level {
        Level::Off => None,
        Level::Minimal => Some("Tactical minimal"),
        Level::Medium => Some("Tactical medium"),
        Level::Max => Some("Tactical max"),
    }
}

/// The weapons held, as the bottom bar shows them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Weapons {
    /// Each weapon's selection icon in the wheel's order, and whether it is in hand.
    pub(crate) held: [Option<(TextureId, bool)>; MAX_WEAPONS],
    /// The ammunition's room after the row, while a weapon held takes ammunition: the
    /// one in hand's count (`None` when it takes none) and whether it runs low.
    pub(crate) ammo: Option<(Option<u32>, bool)>,
}

/// What the bottom bar shows, as last read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Inputs {
    /// The hub's numbers; `None` while unknown (no hub, or not read yet).
    pub(crate) credits: Option<u64>,
    pub(crate) shards: Option<u64>,
    pub(crate) crystals: Option<u32>,
    /// The best tier among the dormant kybers held (`crate::kyber::TIERS`' index), whose
    /// picture stands for them all.
    pub(crate) crystal_tier: usize,
    pub(crate) coins: Option<u64>,
    /// How many things are New in the Collection and whether the words show ([`Nudge`]);
    /// `None` for none.
    pub(crate) new: Option<(u32, bool)>,
    /// The first saber's blade skin, a catalogue id; `None` for the stock blade.
    pub(crate) saber: Option<&'static str>,
    /// The second saber's own skin when it wears another than the first's (`Some(None)`:
    /// the stock blade).
    pub(crate) second: Option<Option<&'static str>>,
    /// What is on about the player, the first ones first.
    pub(crate) effects: [Option<Effect>; MAX_EFFECTS],
    /// Each toy's switch, in `pieces::TOYS`' order ([`toys`]).
    pub(crate) toys: [bool; TOY_COUNT],
    pub(crate) weapons: Weapons,
    /// The local time and date, once read.
    pub(crate) clock: Option<times::Clock>,
    pub(crate) date: Option<times::Date>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            credits: None,
            shards: None,
            crystals: None,
            crystal_tier: 0,
            coins: None,
            new: None,
            saber: None,
            second: None,
            effects: [None; MAX_EFFECTS],
            toys: [false; TOY_COUNT],
            weapons: Weapons::default(),
            clock: None,
            date: None,
        }
    }
}

impl Inputs {
    /// Add `effect` after the others, when there is room.
    pub(crate) fn push_effect(&mut self, effect: Effect) {
        if let Some(slot) = self.effects.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(effect);
        }
    }
}

/// The player's standing in the match, as the top bar shows it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Score {
    /// A game without teams: the place (from 1, tied or not) and the score.
    Place { place: u32, tied: bool, score: i32 },
    /// A team game: the teams' scores and the player's team (1 red, 2 blue).
    Teams { red: i32, blue: i32, team: u8 },
}

/// The bytes kept of the server's and the map's names.
pub(crate) const SERVER_BYTES: usize = 96;
pub(crate) const MAP_BYTES: usize = 48;

/// What the top bar shows, as last read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TopInputs {
    /// The Force powers known in the wheel's order: picture, selected, in use.
    pub(crate) powers: [Option<(TextureId, bool, bool)>; MAX_POWERS],
    pub(crate) score: Option<Score>,
    pub(crate) left: Option<times::Left>,
    /// The feed's entries showing, the newest first.
    pub(crate) feed: [Option<feed::Shown>; feed::SHOWN],
    /// The server's name, colour codes kept; empty when unknown.
    pub(crate) server: Line<SERVER_BYTES>,
    /// The map's name without `mp/`; empty when unknown.
    pub(crate) map: Line<MAP_BYTES>,
    pub(crate) mode: Option<&'static str>,
    /// The players on the server and its slots.
    pub(crate) players: Option<(u8, u16)>,
    pub(crate) ping: Option<u16>,
    pub(crate) fps: Option<u32>,
}

impl Default for TopInputs {
    fn default() -> Self {
        Self {
            powers: [None; MAX_POWERS],
            score: None,
            left: None,
            feed: [None; feed::SHOWN],
            server: Line::default(),
            map: Line::default(),
            mode: None,
            players: None,
            ping: None,
            fps: None,
        }
    }
}

/// A made-up feed for world shots: each entry's name, whether it joined and its strength
/// in [`feed::STEPS`]ths, the newest first.
#[cfg(test)]
pub(crate) fn feed_for_shot(entries: &[(&str, bool, u8)]) -> [Option<feed::Shown>; feed::SHOWN] {
    let mut out = [None; feed::SHOWN];
    for (slot, (name, joined, step)) in out.iter_mut().zip(entries) {
        *slot = Some(feed::Shown {
            name: feed::Name::from(*name),
            joined: *joined,
            step: *step,
        });
    }
    out
}

/// Most pieces a bar holds.
const MAX_PIECES: usize = 64;
const _: () = assert!(4 + 1 + 2 + MAX_EFFECTS + MAX_WEAPONS + 1 + TOY_COUNT + 2 <= MAX_PIECES);
const _: () = assert!(MAX_POWERS + 1 + 1 + feed::SHOWN + 7 <= MAX_PIECES);

/// A placeholder piece, to fill fixed storage.
const NO_PIECE: (Part, Piece) = (
    Part::Toys,
    Piece::Toy {
        index: 0,
        on: false,
    },
);

/// The bottom bar's pieces `inputs` give for `parts`, in the bar's order, into `out`;
/// how many.
fn bottom_pieces(inputs: &Inputs, parts: Parts, out: &mut [(Part, Piece); MAX_PIECES]) -> usize {
    let mut count = 0;
    let mut push = |part: Part, piece: Piece| {
        if parts.has(part) && count < MAX_PIECES {
            out[count] = (part, piece);
            count += 1;
        }
    };
    use pieces::Money;
    let amounts = [
        (Part::Credits, inputs.credits, Money::Credits),
        (Part::Shards, inputs.shards, Money::Shards),
        (
            Part::Crystals,
            inputs.crystals.map(u64::from),
            Money::Dormant(inputs.crystal_tier.min(crate::kyber::COUNT - 1)),
        ),
        (Part::Coins, inputs.coins, Money::Doom),
    ];
    for (part, value, money) in amounts {
        if let Some(value) = value {
            push(part, Piece::Amount { value, money });
        }
    }
    if let Some((count, words)) = inputs.new {
        push(Part::New, Piece::New { count, words });
    }
    push(Part::Saber, pieces::saber_piece(inputs.saber));
    if let Some(second) = inputs.second {
        push(Part::Saber, pieces::saber_piece(second));
    }
    for effect in inputs.effects.iter().flatten() {
        push(Part::Effects, Piece::Effect(*effect));
    }
    for (texture, current) in inputs.weapons.held.iter().flatten() {
        push(
            Part::Weapons,
            Piece::Weapon {
                texture: *texture,
                current: *current,
            },
        );
    }
    if let Some((value, low)) = inputs.weapons.ammo {
        push(Part::Weapons, Piece::Ammo { value, low });
    }
    for (index, on) in inputs.toys.iter().enumerate() {
        push(Part::Toys, Piece::Toy { index, on: *on });
    }
    if let Some(clock) = inputs.clock {
        push(Part::Time, Piece::Time(clock));
    }
    if let Some(date) = inputs.date {
        push(Part::Date, Piece::Date(date));
    }
    count
}

/// The top bar's pieces, as [`bottom_pieces`]: the feed's oldest entry first, so the
/// newest stands beside the server's name.
fn top_pieces(inputs: &TopInputs, parts: Parts, out: &mut [(Part, Piece); MAX_PIECES]) -> usize {
    let mut count = 0;
    let mut push = |part: Part, piece: Piece| {
        if parts.has(part) && count < MAX_PIECES {
            out[count] = (part, piece);
            count += 1;
        }
    };
    for (texture, selected, active) in inputs.powers.iter().flatten() {
        push(
            Part::Force,
            Piece::Power {
                texture: *texture,
                selected: *selected,
                active: *active,
            },
        );
    }
    match inputs.score {
        Some(Score::Place { place, tied, score }) => {
            push(Part::Score, Piece::Place { place, tied, score });
        }
        Some(Score::Teams { red, blue, team }) => {
            push(Part::Score, Piece::Teams { red, blue, team });
        }
        None => {}
    }
    if let Some(left) = inputs.left {
        push(Part::TimeLeft, Piece::Left(left));
    }
    for index in (0..feed::SHOWN).rev() {
        if inputs.feed[index].is_some() {
            push(Part::Feed, Piece::Feed(index as u8));
        }
    }
    if !inputs.server.is_empty() {
        push(Part::Server, Piece::Server);
    }
    if !inputs.map.is_empty() {
        push(Part::Map, Piece::Map);
    }
    if let Some(mode) = inputs.mode {
        push(Part::Mode, Piece::Mode(mode));
    }
    if let Some((count, slots)) = inputs.players {
        push(Part::Players, Piece::Players { count, slots });
    }
    if let Some(ping) = inputs.ping {
        push(Part::Ping, Piece::Ping(ping));
    }
    if let Some(fps) = inputs.fps {
        push(Part::Fps, Piece::Fps(fps));
    }
    count
}

/// Sizes in pixels at 1080 lines, before the text size: the bar's height, the room
/// inside the screen's ends, between two pieces of a part, between two parts (where a
/// thin rule stands in the middle) and the least between two groups.
const HEIGHT: f32 = 28.0;
const PAD: f32 = 14.0;
const GAP: f32 = 7.0;
const PART_GAP: f32 = 21.0;
const GROUP_GAP: f32 = 32.0;
/// The rule's distance from the bar's ends, top and bottom.
const RULE_INSET: f32 = 7.0;

/// A bar's height in pixels at 1080 lines at text size `k`.
fn bar_height(k: f32) -> f32 {
    HEIGHT * k.max(1.0)
}

/// Where each piece goes along a bar `screen` pixels wide (at 1080 lines), the pieces
/// given by their part (`parts`, in the bar's order: the left group's, the centre's, the
/// right's) and width: their left edges from the screen's into `xs`, and whether each is
/// kept into `kept`. The left group starts at the screen's left (past [`PAD`]), the right
/// one ends at its right, the centre one is centred; while groups would come nearer than
/// [`GROUP_GAP`], the piece of the part that matters least ([`Part::keep`]) among those
/// in the way is left out, one at a time. A piece without width is left out.
fn arrange(parts: &[Part], widths: &[f32], screen: f32, xs: &mut [f32], kept: &mut [bool]) {
    let count = parts.len().min(widths.len()).min(xs.len()).min(kept.len());
    for index in 0..count {
        kept[index] = widths[index] > 0.0;
    }
    let group_width = |group: Group, kept: &[bool]| {
        let mut width = 0.0_f32;
        let mut last: Option<Part> = None;
        for index in 0..count {
            if !kept[index] || parts[index].group() != group {
                continue;
            }
            width += match last {
                None => 0.0,
                Some(previous) if previous == parts[index] => GAP,
                Some(_) => PART_GAP,
            } + widths[index];
            last = Some(parts[index]);
        }
        width
    };
    let room = (screen - 2.0 * PAD).max(0.0);
    let middle = screen * 0.5;
    // Each pass leaves one piece out, so this ends.
    for _ in 0..=count {
        let [left, centre, right] =
            [Group::Left, Group::Centre, Group::Right].map(|group| group_width(group, kept));
        let mut crowded = [false; 3];
        if centre > 0.0 {
            if left > 0.0 && PAD + left + GROUP_GAP > middle - centre * 0.5 {
                crowded[0] = true;
                crowded[1] = true;
            }
            if right > 0.0 && middle + centre * 0.5 + GROUP_GAP > screen - PAD - right {
                crowded[1] = true;
                crowded[2] = true;
            }
        } else if left > 0.0 && right > 0.0 && left + GROUP_GAP + right > room {
            crowded[0] = true;
            crowded[2] = true;
        }
        for (group, width) in [left, centre, right].into_iter().enumerate() {
            if width > room {
                crowded[group] = true;
            }
        }
        if crowded == [false; 3] {
            break;
        }
        let in_way = |index: usize| {
            kept[index]
                && crowded[match parts[index].group() {
                    Group::Left => 0,
                    Group::Centre => 1,
                    Group::Right => 2,
                }]
        };
        let Some(least) = (0..count)
            .filter(|index| in_way(*index))
            .map(|index| parts[index].keep())
            .min()
        else {
            break;
        };
        let mut losing = (0..count).filter(|index| in_way(*index) && parts[*index].keep() == least);
        let drop = if parts[losing.clone().next().unwrap_or(0)].drops_first_piece() {
            losing.next()
        } else {
            losing.next_back()
        };
        match drop {
            Some(index) => kept[index] = false,
            None => break,
        }
    }
    let [left, centre, right] =
        [Group::Left, Group::Centre, Group::Right].map(|group| group_width(group, kept));
    for (group, start) in [
        (Group::Left, PAD),
        (Group::Centre, middle - centre * 0.5),
        (Group::Right, screen - PAD - right),
    ] {
        let mut x = start;
        let mut last: Option<Part> = None;
        for index in 0..count {
            if !kept[index] || parts[index].group() != group {
                continue;
            }
            x += match last {
                None => 0.0,
                Some(previous) if previous == parts[index] => GAP,
                Some(_) => PART_GAP,
            };
            xs[index] = x;
            x += widths[index];
            last = Some(parts[index]);
        }
    }
    let _ = left;
}

/// What a bar was last laid out from: the same again keeps its shapes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Laid<I> {
    inputs: I,
    parts: Parts,
    viewport: [f32; 2],
    /// The text size.
    k: f32,
    /// Whether the dormant kybers' pictures were in the atlas.
    pictures: bool,
}

/// One bar's laid-out storage.
struct Strip<I> {
    canvas: MenuCanvas,
    laid: Option<(Laid<I>, bool)>,
    /// The bar shows this frame ([`crate::GpuState::reserve_status_bars`]).
    shown: bool,
    /// It was drawn this frame.
    drawn: bool,
}

impl<I> Default for Strip<I> {
    fn default() -> Self {
        Self {
            canvas: MenuCanvas::with_capacities(64, 48, 200),
            laid: None,
            shown: false,
            drawn: false,
        }
    }
}

/// The bars' storage, kept for the game's run.
pub(crate) struct State {
    bottom: Strip<Inputs>,
    top: Strip<TopInputs>,
    /// What the bars show, as last read, and when.
    inputs: Inputs,
    top_inputs: TopInputs,
    read: Option<Instant>,
    clock_read: Option<Instant>,
    /// What the New part says ([`Nudge`]).
    nudge: Nudge,
    /// Who joins and leaves, and when the slots were last read.
    feed: feed::Feed,
    feed_read: Option<Instant>,
    /// The ping the top bar shows, followed snapshot by snapshot ([`ping`]).
    ping: ping::Ping,
    /// The bars' switches and their parts', as read at an archive revision of the
    /// console's cvars ([`ViewerConsole::archive_revision`]).
    switches: Option<(u64, [bool; 2], Parts)>,
    /// The parts on this frame (Tactical mode's taken out).
    parts: Parts,
    /// The room the bars keep this frame.
    reserved: Reserved,
    /// The parts showing a value this frame.
    showing: Parts,
    /// Text formatted to be measured.
    scratch: String,
    /// What a world shot shows, kept as set.
    #[cfg(test)]
    pub(crate) preview: Option<Inputs>,
    #[cfg(test)]
    pub(crate) preview_top: Option<TopInputs>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            bottom: Strip::default(),
            top: Strip::default(),
            inputs: Inputs::default(),
            top_inputs: TopInputs::default(),
            read: None,
            clock_read: None,
            nudge: Nudge::default(),
            feed: feed::Feed::default(),
            feed_read: None,
            ping: ping::Ping::default(),
            switches: None,
            parts: Parts(0),
            reserved: Reserved::NONE,
            showing: Parts(0),
            scratch: String::with_capacity(48),
            #[cfg(test)]
            preview: None,
            #[cfg(test)]
            preview_top: None,
        }
    }
}

impl State {
    /// The room the bars keep at the screen's top and bottom this frame, in window
    /// pixels: a bar's height while it shows, 0 while it is off or hidden. Set before the
    /// HUD is laid out ([`crate::GpuState::reserve_status_bars`]), for everything drawn
    /// against an edge to stay inside the bars.
    pub(crate) fn reserved(&self) -> Reserved {
        self.reserved
    }

    /// Whether a bar shows `part` with a value this frame, so the HUD leaves out its own
    /// readout of the same thing.
    pub(crate) fn shows(&self, part: Part) -> bool {
        self.showing.has(part)
    }

    /// The bottom and the top bar's shapes, after [`crate::GpuState::append_status_bars`]
    /// laid them out.
    pub(crate) fn draw_lists(&self) -> [Option<&sjk_ui::DrawList>; 2] {
        [
            self.bottom.drawn.then(|| self.bottom.canvas.draw_list()),
            self.top.drawn.then(|| self.top.canvas.draw_list()),
        ]
    }

    /// Whether a bar is drawn this frame (world shots: the 2D pass runs for it without a
    /// session).
    #[cfg(test)]
    pub(crate) fn drawn(&self) -> bool {
        self.bottom.drawn || self.top.drawn
    }

    /// Whether each bar, bottom then top, is drawn this frame.
    #[cfg(test)]
    pub(crate) fn drawn_bars(&self) -> [bool; 2] {
        [self.bottom.drawn, self.top.drawn]
    }

    /// Forget the kept layouts, so the next frame lays the bars out again (the timing
    /// shot measures both ways).
    #[cfg(test)]
    pub(crate) fn forget_layout(&mut self) {
        self.bottom.laid = None;
        self.top.laid = None;
    }

    /// A snapshot that shows our own commands run (not a followed player's, not the
    /// intermission's) came at `now` with the stock ping `ping`
    /// ([`sjk_client::ClientSession::snapshot_ping`]), for the top bar's ping ([`ping`]).
    pub(crate) fn observe_snapshot_ping(&mut self, ping: i32, now: Instant) {
        self.ping.snapshot(ping, now);
    }

    /// Note a `scores` reply newer than the last seen, with our own row's ping, for the top
    /// bar's ping ([`ping`]): a revision compared each frame, the rows read only when it
    /// changes.
    pub(crate) fn observe_scores(&mut self, session: &sjk_client::ClientSession, now: Instant) {
        let own = session.game_state().client_num;
        self.ping.scores(
            session.scores_revision(),
            || {
                session
                    .scores()
                    .iter()
                    .find(|row| i32::from(row.client_num) == own)
                    .map(|row| row.ping)
            },
            now,
        );
    }

    /// Forget the ping followed: another session or time line.
    pub(crate) fn forget_ping(&mut self) {
        self.ping.clear();
    }

    /// Whether what the bars show is due to be read again at `now`.
    fn due(&self, now: Instant) -> bool {
        self.read
            .is_none_or(|at| now.saturating_duration_since(at) >= EVERY)
    }
}

/// Lay a bar out from `list` (its pieces, `count` of them) on `canvas`, `edge` the
/// screen edge it is docked to; whether anything was drawn.
#[allow(clippy::too_many_arguments)]
fn build(
    canvas: &mut MenuCanvas,
    edge: Bar,
    list: &[(Part, Piece)],
    texts: Texts<'_>,
    viewport: [f32; 2],
    k: f32,
    measures: Measures<'_>,
    scratch: &mut String,
) -> bool {
    let s = crate::menu::sjk::scale(viewport);
    canvas.begin_transparent(viewport);
    let count = list.len().min(MAX_PIECES);
    let mut widths = [0.0_f32; MAX_PIECES];
    let mut owners = [Part::Toys; MAX_PIECES];
    for (index, (part, piece)) in list[..count].iter().enumerate() {
        widths[index] = pieces::width(piece, texts, measures, scratch, s);
        owners[index] = *part;
    }
    let mut xs = [0.0_f32; MAX_PIECES];
    let mut kept = [false; MAX_PIECES];
    arrange(
        &owners[..count],
        &widths[..count],
        viewport[0] / s,
        &mut xs[..count],
        &mut kept[..count],
    );
    let height = bar_height(k) * s;
    let top = match edge {
        Bar::Bottom => viewport[1] - height,
        Bar::Top => 0.0,
    };
    let bar = Rect::new(0.0, top, viewport[0], height);
    glass(canvas, bar, edge, s);
    let mut pen = pieces::Pen {
        canvas,
        middle: top + height * 0.5,
        s,
        k,
    };
    let mut previous: Option<Part> = None;
    for index in 0..count {
        if !kept[index] {
            continue;
        }
        let (part, piece) = list[index];
        let x = xs[index] * s;
        if let Some(previous) = previous
            && previous != part
            && previous.group() == part.group()
        {
            let rule = x - PART_GAP * 0.5 * s;
            let _ = pen.canvas.draw_list_mut().push(DrawCommand::SolidRect {
                rect: Rect::new(
                    rule,
                    top + RULE_INSET * s,
                    1.0_f32.max(s),
                    height - 2.0 * RULE_INSET * s,
                ),
                color: color::alpha(color::HOLO, 0.28),
            });
        }
        previous = Some(part);
        pieces::draw(
            &mut pen,
            piece,
            texts,
            measures,
            scratch,
            x,
            widths[index] * s,
        );
    }
    canvas.finish(0);
    true
}

/// A bar's glass: the navy ground across the screen, a holo hairline along its inner
/// edge (the bottom bar's top, the top bar's bottom).
fn glass(canvas: &mut MenuCanvas, bar: Rect, edge: Bar, s: f32) {
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::SolidRect {
        rect: bar,
        color: color::alpha(color::SPACE, 0.84),
    });
    let hair = 1.0_f32.max(s);
    let y = match edge {
        Bar::Bottom => bar.y,
        Bar::Top => bar.bottom() - hair,
    };
    let _ = list.push(DrawCommand::SolidRect {
        rect: Rect::new(bar.x, y, bar.width, hair),
        color: color::alpha(color::HOLO, 0.34),
    });
}

impl crate::GpuState {
    /// Whether play shows for the bars this frame, as for the HUD's elements: a game
    /// (or a demo) on, `cg_draw2D` on, no menu or console over it, no intermission
    /// (`intermission`), the elements not hidden (`cg_drawHud 0`). It mirrors the
    /// frame's own `information_visible`, read here before the HUD is laid out.
    pub(crate) fn status_bar_play(&self, intermission: bool) -> bool {
        let console = self.console.as_ref();
        (self.live_session.is_some() || self.demo_session.is_some())
            && console
                .and_then(|console| console.bool_cvar("cg_draw2D"))
                .unwrap_or(true)
            && !self.game_menu
            && !console.is_some_and(ViewerConsole::is_open)
            && !self
                .client_menu
                .as_ref()
                .is_some_and(|menu| menu.is_visible())
            && !intermission
            && !crate::hud::toggle::hides(console, crate::hud::toggle::Part::Elements)
    }

    /// What the bars show this frame that the HUD then leaves out ([`State::shows`]).
    pub(crate) fn status_bar_readouts(&self) -> crate::hud::BarReadouts {
        let state = &self.status_bar;
        crate::hud::BarReadouts {
            timer: state.shows(Part::TimeLeft),
            time: state.shows(Part::Time),
            date: state.shows(Part::Date),
            fps: state.shows(Part::Fps),
        }
    }

    /// Decide which bars show this frame and how much of the screen's edges they keep
    /// ([`State::reserved`]), before the HUD is laid out: a bar shows when its switch and
    /// a part of it are on and `play` shows (a live game, no menu or console over it, no
    /// intermission, the HUD's elements not hidden), never over a demo. Reads what the
    /// bars show when due, and who joins and leaves while a game is on.
    pub(crate) fn reserve_status_bars(&mut self, viewport: [f32; 2], play: bool) {
        let now = Instant::now();
        self.observe_join_feed(now);
        let console = self.console.as_ref();
        // The switches are archived: read again only when one of those changes.
        let revision = console.map(ViewerConsole::archive_revision);
        let (on, mut parts) = match self.status_bar.switches {
            Some((kept, on, parts)) if Some(kept) == revision => (on, parts),
            _ => {
                let switch = |cvar: &str| {
                    console
                        .and_then(|console| console.bool_cvar(cvar))
                        .unwrap_or(true)
                };
                let read = ([switch(CVAR), switch(TOP_CVAR)], Parts::read(console));
                self.status_bar.switches = revision.map(|revision| (revision, read.0, read.1));
                read
            }
        };
        // Tactical mode is there to fight: no toys (switched off anyway), no nudge.
        if crate::tactical::level() != crate::tactical::Level::Off {
            parts = parts.without(Part::Toys).without(Part::New);
        }
        // Over a demo they would tell the viewer's own things, not the recording's.
        let live = self.live_session.is_some() && play;
        let shown = [
            live && on[0] && parts.any_on(Bar::Bottom),
            live && on[1] && parts.any_on(Bar::Top),
        ];
        // A world shot shows the bars it made up, without a game.
        #[cfg(test)]
        let shown = [
            shown[0] || self.status_bar.preview.is_some(),
            shown[1] || self.status_bar.preview_top.is_some(),
        ];
        if (shown[0] || shown[1]) && self.status_bar.due(now) {
            self.read_status_bars(now);
        }
        #[cfg(test)]
        {
            if let Some(preview) = self.status_bar.preview {
                self.status_bar.inputs = preview;
            }
            if let Some(preview) = self.status_bar.preview_top {
                self.status_bar.top_inputs = preview;
            }
        }
        let held = self.status_bar_feed_held();
        let state = &mut self.status_bar;
        if !held {
            state.top_inputs.feed = state.feed.shown(now);
        }
        state.bottom.shown = shown[0];
        state.top.shown = shown[1];
        state.parts = parts;
        let s = crate::menu::sjk::scale(viewport);
        let height = bar_height(self.ui_font.style().scale.max(0.5)) * s;
        state.reserved = Reserved {
            top: if shown[1] { height } else { 0.0 },
            bottom: if shown[0] { height } else { 0.0 },
        };
        let mut showing = Parts(0);
        let mut show = |part: Part, value: bool| {
            let bar = match part.bar() {
                Bar::Bottom => shown[0],
                Bar::Top => shown[1],
            };
            if bar && value && parts.has(part) {
                showing.0 |= part.bit();
            }
        };
        show(Part::Fps, state.top_inputs.fps.is_some());
        show(Part::TimeLeft, state.top_inputs.left.is_some());
        show(Part::Time, state.inputs.clock.is_some());
        show(Part::Date, state.inputs.date.is_some());
        state.showing = showing;
    }

    /// A world shot's made-up feed stays as set.
    fn status_bar_feed_held(&self) -> bool {
        #[cfg(test)]
        if self.status_bar.preview_top.is_some() {
            return true;
        }
        false
    }

    /// Lay out the bars [`Self::reserve_status_bars`] showed this frame (each kept as it
    /// was when what it shows has not changed) and add their text; whether each, bottom
    /// then top, was drawn, so its shapes go into the frame's layers.
    pub(crate) fn append_status_bars(&mut self, viewport: [f32; 2]) -> [bool; 2] {
        let parts = self.status_bar.parts;
        let k = self.ui_font.style().scale.max(0.5);
        let best = self.status_bar.inputs.crystal_tier;
        let tier = best.min(crate::kyber::COUNT - 1);
        let pictures = crate::holocrons::icons::is_loaded(crate::kyber::TIERS[tier].slot);
        let mut drawn = [false; 2];
        for (index, edge) in [Bar::Bottom, Bar::Top].into_iter().enumerate() {
            let state = &mut self.status_bar;
            let shown = match edge {
                Bar::Bottom => state.bottom.shown,
                Bar::Top => state.top.shown,
            };
            if !shown {
                match edge {
                    Bar::Bottom => state.bottom.drawn = false,
                    Bar::Top => state.top.drawn = false,
                }
                continue;
            }
            let target = crate::ingame_menu::sjk_view::text_target(
                &mut self.game_fonts,
                &mut self.text_vertices,
                &self.ui_font,
            );
            let measures = Measures {
                display: target.display_measure(),
                body: target.body_measure(),
            };
            let mut list = [NO_PIECE; MAX_PIECES];
            let texts = Texts {
                server: state.top_inputs.server.as_str(),
                map: state.top_inputs.map.as_str(),
                feed: &state.top_inputs.feed,
            };
            let ok = match edge {
                Bar::Bottom => {
                    let laid = Laid {
                        inputs: state.inputs,
                        parts,
                        viewport,
                        k,
                        pictures,
                    };
                    match state.bottom.laid {
                        Some((kept, ok)) if kept == laid => ok,
                        _ => {
                            let count = bottom_pieces(&state.inputs, parts, &mut list);
                            let ok = build(
                                &mut state.bottom.canvas,
                                edge,
                                &list[..count],
                                texts,
                                viewport,
                                k,
                                measures,
                                &mut state.scratch,
                            );
                            state.bottom.laid = Some((laid, ok));
                            ok
                        }
                    }
                }
                Bar::Top => {
                    let laid = Laid {
                        inputs: state.top_inputs,
                        parts,
                        viewport,
                        k,
                        pictures: false,
                    };
                    match state.top.laid {
                        Some((kept, ok)) if kept == laid => ok,
                        _ => {
                            let count = top_pieces(&state.top_inputs, parts, &mut list);
                            let ok = build(
                                &mut state.top.canvas,
                                edge,
                                &list[..count],
                                texts,
                                viewport,
                                k,
                                measures,
                                &mut state.scratch,
                            );
                            state.top.laid = Some((laid, ok));
                            ok
                        }
                    }
                }
            };
            let canvas = match edge {
                Bar::Bottom => {
                    state.bottom.drawn = ok;
                    &state.bottom.canvas
                }
                Bar::Top => {
                    state.top.drawn = ok;
                    &state.top.canvas
                }
            };
            if ok {
                target.append(canvas, viewport);
            }
            drawn[index] = ok;
        }
        drawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pieces::Money;

    fn amount(value: u64) -> Piece {
        Piece::Amount {
            value,
            money: Money::Credits,
        }
    }

    /// Both bars are on, with every part, for a profile that never chose.
    #[test]
    fn both_bars_are_on_by_default() {
        let mut cvars = CvarRegistry::default();
        register(&mut cvars).unwrap();
        for cvar in [CVAR, TOP_CVAR]
            .into_iter()
            .chain(Part::ALL.map(Part::cvar))
        {
            assert_eq!(
                cvars.get(cvar).unwrap().value,
                sjk_shell::CvarValue::Bool(true),
                "{cvar}"
            );
        }
        assert!(cvars.get("cg_statusBarPosition").is_none(), "gone");
        assert!(!Parts::ALL.without(Part::Toys).has(Part::Toys));
        assert!(Parts::ALL.without(Part::Toys).has(Part::Credits));
    }

    #[test]
    fn each_part_has_its_own_switch_and_bar() {
        let mut cvars: Vec<&str> = Part::ALL.iter().map(|part| part.cvar()).collect();
        cvars.sort_unstable();
        cvars.dedup();
        assert_eq!(cvars.len(), Part::ALL.len());
        for part in Part::ALL {
            let prefix = match part.bar() {
                Bar::Bottom => "cg_statusBar",
                Bar::Top => "cg_statusBarTop",
            };
            assert!(part.cvar().starts_with(prefix), "{part:?}");
            assert_eq!(
                part.cvar().starts_with("cg_statusBarTop"),
                part.bar() == Bar::Top,
                "{part:?}"
            );
        }
        let only_toys = Parts::from_switches(|part| part == Part::Toys);
        assert!(only_toys.has(Part::Toys));
        assert!(only_toys.any_on(Bar::Bottom));
        assert!(!only_toys.any_on(Bar::Top));
        assert!(Part::ALL.iter().all(|part| Parts::ALL.has(*part)));
        assert_eq!(Parts::from_switches(|_| true), Parts::ALL);
        // Each bar has every group, and each part a rank of its own on its bar.
        for bar in [Bar::Bottom, Bar::Top] {
            let mut keeps: Vec<u8> = Part::ALL
                .iter()
                .filter(|part| part.bar() == bar)
                .map(|part| part.keep())
                .collect();
            let len = keeps.len();
            keeps.sort_unstable();
            keeps.dedup();
            assert_eq!(keeps.len(), len, "{bar:?}");
            for group in [Group::Left, Group::Centre, Group::Right] {
                assert!(
                    Part::ALL
                        .iter()
                        .any(|part| part.bar() == bar && part.group() == group),
                    "{bar:?} {group:?}"
                );
            }
        }
    }

    /// An element nearer the top moves down by the top bar's height, one nearer the
    /// bottom up by the bottom's; no bar, no move.
    #[test]
    fn elements_move_inside_the_bars_by_their_nearer_edge() {
        let room = Reserved {
            top: 28.0,
            bottom: 30.0,
        };
        let timer = Rect::new(900.0, 10.0, 120.0, 40.0);
        let health = Rect::new(0.0, 940.0, 300.0, 140.0);
        let middle_low = Rect::new(800.0, 560.0, 100.0, 40.0);
        assert_eq!(room.shift(timer, 1080.0), 28.0);
        assert_eq!(room.shift(health, 1080.0), -30.0);
        assert_eq!(room.shift(middle_low, 1080.0), -30.0);
        assert_eq!(Reserved::NONE.shift(timer, 1080.0), 0.0);
        assert_eq!(Reserved::NONE.shift(health, 1080.0), 0.0);
        // One bar alone moves only its own half.
        let top_only = Reserved {
            top: 28.0,
            bottom: 0.0,
        };
        assert_eq!(top_only.shift(timer, 1080.0), 28.0);
        assert_eq!(top_only.shift(health, 1080.0), 0.0);
    }

    #[test]
    fn lines_keep_their_text_cut_at_a_character() {
        let line = Line::<8>::from("duel6");
        assert_eq!(line.as_str(), "duel6");
        assert!(!line.is_empty());
        assert!(Line::<8>::default().is_empty());
        assert_eq!(Line::<4>::from("abcdef").as_str(), "abcd");
        // "é" is two bytes: cut before it rather than through it.
        assert_eq!(Line::<4>::from("abcé").as_str(), "abc");
        assert_eq!(Line::<8>::from("abc"), Line::<8>::from("abc"));
        assert_ne!(Line::<8>::from("abc"), Line::<8>::from("abd"));
    }

    #[test]
    fn unknown_numbers_and_switched_off_parts_are_left_out() {
        let mut inputs = Inputs {
            credits: Some(1250),
            coins: Some(1),
            ..Inputs::default()
        };
        inputs.toys[toys::HAX] = true;
        let mut out = [NO_PIECE; MAX_PIECES];
        // Shards and crystals unknown, no weapon, no clock read yet: not shown.
        let count = bottom_pieces(&inputs, Parts::ALL, &mut out);
        assert_eq!(out[0], (Part::Credits, amount(1250)));
        assert_eq!(
            out[1],
            (
                Part::Coins,
                Piece::Amount {
                    value: 1,
                    money: Money::Doom
                }
            )
        );
        assert!(matches!(
            out[2],
            (
                Part::Saber,
                Piece::Saber {
                    name: "Stock blade",
                    tier: None,
                    ..
                }
            )
        ));
        assert_eq!(count, 3 + TOY_COUNT);
        assert_eq!(
            out[3 + toys::HAX],
            (
                Part::Toys,
                Piece::Toy {
                    index: toys::HAX,
                    on: true
                }
            )
        );
        let without = Parts::from_switches(|part| !matches!(part, Part::Toys | Part::Coins));
        assert_eq!(bottom_pieces(&inputs, without, &mut out), 2);
        assert_eq!(out[1].0, Part::Saber);
    }

    /// The weapons in hand and held, the ammunition's room after them, then the clock and
    /// the date at the end of the bottom bar.
    #[test]
    fn weapons_then_the_toys_the_time_and_the_date() {
        let mut inputs = Inputs {
            clock: Some(times::Clock {
                hour: 21,
                minute: 7,
            }),
            date: Some(times::Date {
                day: 16,
                month: 11,
                year: 2026,
            }),
            ..Inputs::default()
        };
        inputs.weapons.held[0] = Some((TextureId(1), false));
        inputs.weapons.held[1] = Some((TextureId(2), true));
        inputs.weapons.ammo = Some((Some(40), false));
        let parts =
            Parts::from_switches(|part| matches!(part, Part::Weapons | Part::Time | Part::Date));
        let mut out = [NO_PIECE; MAX_PIECES];
        assert_eq!(bottom_pieces(&inputs, parts, &mut out), 5);
        assert_eq!(
            out[1].1,
            Piece::Weapon {
                texture: TextureId(2),
                current: true
            }
        );
        assert_eq!(
            out[2].1,
            Piece::Ammo {
                value: Some(40),
                low: false
            }
        );
        assert!(matches!(out[3].1, Piece::Time(_)));
        assert!(matches!(out[4].1, Piece::Date(_)));
        assert!(
            out[..5]
                .windows(2)
                .all(|pair| pair[0].0.group() as u8 <= pair[1].0.group() as u8),
            "in the groups' order"
        );
    }

    /// The top bar: powers, the score, the time left, then the feed oldest first and the
    /// match's facts, the frame rate last; parts without a value are left out.
    #[test]
    fn the_top_bar_reads_from_the_powers_to_the_frame_rate() {
        let mut inputs = TopInputs {
            score: Some(Score::Place {
                place: 2,
                tied: false,
                score: 15,
            }),
            left: Some(times::Left(754)),
            map: Line::from("duel6"),
            mode: Some("Duel"),
            players: Some((5, 24)),
            ping: Some(48),
            fps: Some(144),
            ..TopInputs::default()
        };
        inputs.powers[0] = Some((TextureId(9), true, false));
        let shown = |name: &str| feed::Shown {
            name: feed::Name::from(name),
            joined: true,
            step: feed::STEPS,
        };
        inputs.feed[0] = Some(shown("newest"));
        inputs.feed[1] = Some(shown("older"));
        let mut out = [NO_PIECE; MAX_PIECES];
        let count = top_pieces(&inputs, Parts::ALL, &mut out);
        let order: Vec<Part> = out[..count].iter().map(|(part, _)| *part).collect();
        assert_eq!(
            order,
            [
                Part::Force,
                Part::Score,
                Part::TimeLeft,
                Part::Feed,
                Part::Feed,
                Part::Map,
                Part::Mode,
                Part::Players,
                Part::Ping,
                Part::Fps
            ],
            "no server name known: left out"
        );
        assert_eq!(out[3].1, Piece::Feed(1), "the older entry first");
        assert_eq!(out[4].1, Piece::Feed(0));
        let none = top_pieces(&TopInputs::default(), Parts::ALL, &mut out);
        assert_eq!(none, 0, "nothing known, nothing shown");
    }

    /// Groups sit at the bar's left, centre and right, with more room between parts than
    /// between pieces of a part.
    #[test]
    fn groups_stand_at_the_ends_and_the_middle() {
        let parts = [
            Part::Credits,
            Part::Credits,
            Part::Saber,
            Part::Weapons,
            Part::Time,
        ];
        let widths = [20.0, 20.0, 50.0, 40.0, 30.0];
        let mut xs = [0.0; 5];
        let mut kept = [false; 5];
        arrange(&parts, &widths, 1920.0, &mut xs, &mut kept);
        assert_eq!(kept, [true; 5]);
        assert_eq!(xs[0], PAD);
        assert_eq!(xs[1], PAD + 20.0 + GAP);
        assert_eq!(xs[2], PAD + 40.0 + GAP + PART_GAP);
        assert_eq!(xs[3], 960.0 - 20.0, "centred");
        assert_eq!(xs[4], 1920.0 - PAD - 30.0, "against the right end");
    }

    /// When the groups crowd each other, the part that matters least among those in the
    /// way goes first, one piece at a time from its end (the feed from its oldest);
    /// a group out of the way keeps everything.
    #[test]
    fn what_matters_least_goes_first_when_room_runs_out() {
        // Right group: 4 toys and the time; centre: the weapons. 400 wide.
        let parts = [
            Part::Weapons,
            Part::Toys,
            Part::Toys,
            Part::Toys,
            Part::Toys,
            Part::Time,
        ];
        let widths = [60.0, 30.0, 30.0, 30.0, 30.0, 40.0];
        let mut xs = [0.0; 6];
        let mut kept = [false; 6];
        arrange(&parts, &widths, 410.0, &mut xs, &mut kept);
        // Middle 205, the weapons 175..235; the right group must end by 396 and start at
        // 267 or later: the time (40) and two toys (30 + 7 + 30, + 21 to the time) fit.
        assert_eq!(kept, [true, true, true, false, false, true]);
        assert!(xs[1] >= 235.0 + GROUP_GAP, "{xs:?}");
        // Ten narrower: one toy fewer.
        arrange(&parts, &widths, 400.0, &mut xs, &mut kept);
        assert_eq!(kept, [true, true, false, false, false, true]);
        // Narrower: the toys all go, the time stays.
        arrange(&parts, &widths, 300.0, &mut xs, &mut kept);
        assert_eq!(kept, [true, false, false, false, false, true]);
        // The feed loses its oldest (first) entry first.
        let parts = [Part::TimeLeft, Part::Feed, Part::Feed, Part::Fps];
        let widths = [60.0, 50.0, 50.0, 40.0];
        let mut xs = [0.0; 4];
        let mut kept = [false; 4];
        arrange(&parts, &widths, 420.0, &mut xs, &mut kept);
        assert_eq!(kept, [true, false, true, true]);
        // A piece without width is never kept; with room, everything is.
        arrange(&parts, &[60.0, 0.0, 50.0, 40.0], 1920.0, &mut xs, &mut kept);
        assert_eq!(kept, [true, false, true, true]);
    }

    /// Two sides without a centre share the room; one group too wide for the screen loses
    /// its least kept pieces.
    #[test]
    fn sides_without_a_centre_share_the_room() {
        let parts = [Part::Force, Part::Force, Part::Server, Part::Fps];
        let widths = [100.0, 100.0, 150.0, 50.0];
        let mut xs = [0.0; 4];
        let mut kept = [false; 4];
        arrange(&parts, &widths, 480.0, &mut xs, &mut kept);
        // Room 452: 207 + 32 + 221 = 460 > 452, so the server's name goes.
        assert_eq!(kept, [true, true, false, true]);
        let parts = [Part::Force; 3];
        let mut xs = [0.0; 3];
        let mut kept = [false; 3];
        arrange(&parts, &[100.0; 3], 250.0, &mut xs, &mut kept);
        assert_eq!(kept, [true, true, false]);
    }

    #[test]
    fn the_second_saber_and_effects_follow_the_first() {
        let skin = crate::unlockables::blade_skins().next().unwrap();
        let mut inputs = Inputs {
            saber: Some(skin.id),
            second: Some(None),
            ..Inputs::default()
        };
        inputs.push_effect(Effect {
            words: "Do not disturb",
            colour: color::EMBER,
        });
        let mut out = [NO_PIECE; MAX_PIECES];
        let parts = Parts::from_switches(|part| matches!(part, Part::Saber | Part::Effects));
        assert_eq!(bottom_pieces(&inputs, parts, &mut out), 3);
        assert_eq!(
            out[0].1,
            Piece::Saber {
                name: skin.name,
                colour: skin.tier.colour(),
                tier: Some(skin.tier)
            }
        );
        assert!(matches!(
            out[1].1,
            Piece::Saber {
                name: "Stock blade",
                ..
            }
        ));
        assert!(matches!(
            out[2].1,
            Piece::Effect(Effect {
                words: "Do not disturb",
                ..
            })
        ));
    }

    #[test]
    fn effects_beyond_the_room_are_dropped() {
        let mut inputs = Inputs::default();
        for _ in 0..MAX_EFFECTS + 3 {
            inputs.push_effect(Effect {
                words: "Away",
                colour: color::MUTED,
            });
        }
        assert!(inputs.effects.iter().all(Option::is_some));
    }

    /// New things say so in words for the rest of the map they came on, then as a dot
    /// until seen; more coming says it again; ones that came between maps are said on the
    /// next; none, nothing.
    #[test]
    fn new_things_are_said_for_a_map_then_dotted() {
        let mut nudge = Nudge::default();
        let (first, second) = (map_key("maps/mp/ffa3.bsp"), map_key("maps/mp/duel1.bsp"));
        assert_eq!(map_key(""), None);
        assert_ne!(first, second);
        assert_eq!(nudge.follow(0, first), None);
        assert_eq!(nudge.follow(2, first), Some((2, true)));
        assert_eq!(nudge.follow(2, first), Some((2, true)), "the whole map");
        assert_eq!(
            nudge.follow(2, second),
            Some((2, false)),
            "the next map: the dot"
        );
        assert_eq!(
            nudge.follow(2, first),
            Some((2, false)),
            "and it stays a dot"
        );
        assert_eq!(nudge.follow(1, first), Some((1, false)), "one seen");
        assert_eq!(nudge.follow(3, first), Some((3, true)), "more came");
        assert_eq!(nudge.follow(0, first), None, "all seen");
        // Between maps: said on the next one.
        assert_eq!(nudge.follow(1, None), Some((1, true)));
        assert_eq!(nudge.follow(1, second), Some((1, true)));
        assert_eq!(nudge.follow(1, first), Some((1, false)));
        let mut out = [NO_PIECE; MAX_PIECES];
        let inputs = Inputs {
            new: Some((1, false)),
            ..Inputs::default()
        };
        let parts = Parts::from_switches(|part| part == Part::New);
        assert_eq!(bottom_pieces(&inputs, parts, &mut out), 1);
        assert_eq!(
            out[0],
            (
                Part::New,
                Piece::New {
                    count: 1,
                    words: false
                }
            )
        );
        let off = Parts::from_switches(|part| part != Part::New);
        let count = bottom_pieces(&inputs, off, &mut out);
        assert!(out[..count].iter().all(|(part, _)| *part != Part::New));
    }

    #[test]
    fn tactical_says_its_level_only_while_on() {
        use crate::tactical::Level;
        assert_eq!(tactical_words(Level::Off), None);
        for level in [Level::Minimal, Level::Medium, Level::Max] {
            let words = tactical_words(level).unwrap();
            assert!(words.ends_with(level.name()), "{words}");
        }
    }

    /// Dormant kybers show the best tier's picture, kept within the tiers.
    #[test]
    fn dormant_kybers_show_their_best_tier() {
        let mut out = [NO_PIECE; MAX_PIECES];
        let parts = Parts::from_switches(|part| part == Part::Crystals);
        for (tier, shown) in [(0, 0), (2, 2), (99, crate::kyber::COUNT - 1)] {
            let inputs = Inputs {
                crystals: Some(2),
                crystal_tier: tier,
                ..Inputs::default()
            };
            assert_eq!(bottom_pieces(&inputs, parts, &mut out), 1);
            assert_eq!(
                out[0].1,
                Piece::Amount {
                    value: 2,
                    money: Money::Dormant(shown)
                }
            );
        }
    }

    #[test]
    fn full_bars_fit_their_storage() {
        let mut inputs = Inputs {
            credits: Some(u64::MAX),
            shards: Some(1),
            crystals: Some(3),
            crystal_tier: 3,
            coins: Some(4),
            new: Some((3, true)),
            saber: None,
            second: Some(None),
            clock: Some(times::Clock::default()),
            date: Some(times::Date::default()),
            ..Inputs::default()
        };
        inputs.toys = [true; TOY_COUNT];
        inputs.weapons.held = [Some((TextureId(1), false)); MAX_WEAPONS];
        inputs.weapons.ammo = Some((Some(1), true));
        for _ in 0..MAX_EFFECTS {
            inputs.push_effect(Effect {
                words: "Tactical medium",
                colour: color::HOLO,
            });
        }
        let mut out = [NO_PIECE; MAX_PIECES];
        assert_eq!(
            bottom_pieces(&inputs, Parts::ALL, &mut out),
            4 + 1 + 2 + MAX_EFFECTS + MAX_WEAPONS + 1 + TOY_COUNT + 2
        );
        let top = TopInputs {
            powers: [Some((TextureId(1), false, true)); MAX_POWERS],
            score: Some(Score::Teams {
                red: 1,
                blue: 2,
                team: 1,
            }),
            left: Some(times::Left(1)),
            feed: [Some(feed::Shown::default()); feed::SHOWN],
            server: Line::from("server"),
            map: Line::from("map"),
            mode: Some("CTF"),
            players: Some((1, 2)),
            ping: Some(1),
            fps: Some(1),
        };
        assert_eq!(
            top_pieces(&top, Parts::ALL, &mut out),
            MAX_POWERS + 2 + feed::SHOWN + 6
        );
    }
}
