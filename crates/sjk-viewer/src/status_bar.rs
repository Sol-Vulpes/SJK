//! The status bar (Sol's request, 15/11/2026): one compact line at the bottom or the top
//! of the screen, in the SJK UI's look, with what the player carries and wears: their
//! credits, kyber shards, dormant kybers and DOOM coins (the hub's numbers), each an icon
//! and its number, the kyber on their saber with its tier's diamonds and colour, what is
//! on about them (the blade's extras, the finisher, Tactical mode, Do not disturb, away,
//! toon shading) and every toy's switch, its picture lit while on. Since 15/11/2026 it
//! also says when the Collection holds New things (`crate::collection_seen`): "3 new in
//! your Collection" for the rest of the map they arrived on, then a gold dot until they
//! are seen ([`Nudge`]).
//!
//! [`CVAR`] (`cg_statusBar`) switches it on, the default since 16/11/2026 (Sol: "do a
//! default on", with the switch offered again in the News panel's settings to review);
//! [`POSITION_CVAR`] puts it at the bottom (the default) or the top. Each part has its own
//! switch ([`Part::cvar`], all on), so a player keeps only what they want. It shows over
//! play only, as the HUD's elements do: not under the menus, the console, the scoreboard
//! or an intermission, not over a demo (it tells the player's own things, not the
//! recording's), nor while the HUD switch hides the elements (`cg_drawHud 0`). A part
//! without a value (no hub, the DOOM coins not read yet) is left out rather than shown
//! empty, and what does not fit is dropped from the end ([`fit`]); Tactical mode, which
//! switches the toys off, leaves their row out.
//!
//! It keeps clear of the HUD ([`place`]): the boxes of what the HUD drew this frame
//! (`hud::placement::Bounds`: the health and Force blocks in the corners, a centred
//! weapon box, the match timer at the top, the lagometer, the speedometer...) narrow it
//! when they stand at its sides and push it past them when they stand in its middle; it
//! steps aside while something passing stands where it is (the weapon, Force and item
//! selectors at the bottom; a vote or the warmup line at the top). The bottom is the
//! default because the top's middle holds the match timer (`cg_drawTimer`, on).
//!
//! What it shows is read four times a second into fixed storage ([`Inputs`]); it is laid
//! out again only when that, its switches, its place or the screen change ([`Laid`]),
//! and a frame otherwise hands the kept shapes on, allocating nothing.

use crate::console::ViewerConsole;
use crate::menu::sjk::{color, kit, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::sjk_chat_look::Measure;
use crate::text::TextFace;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};
use std::fmt::{self, Write as _};
use std::time::{Duration, Instant};

/// The bar's switch: on by default (Sol, 16/11/2026).
pub(crate) const CVAR: &str = "cg_statusBar";
/// Where the bar stands: 0 the bottom (the default), 1 the top.
pub(crate) const POSITION_CVAR: &str = "cg_statusBarPosition";

/// [`Part::Credits`]'s switch.
pub(crate) const CREDITS_CVAR: &str = "cg_statusBarCredits";
/// [`Part::Shards`]'s switch.
pub(crate) const SHARDS_CVAR: &str = "cg_statusBarShards";
/// [`Part::Crystals`]'s switch.
pub(crate) const CRYSTALS_CVAR: &str = "cg_statusBarCrystals";
/// [`Part::Coins`]'s switch.
pub(crate) const COINS_CVAR: &str = "cg_statusBarCoins";
/// [`Part::New`]'s switch.
pub(crate) const NEW_CVAR: &str = "cg_statusBarNew";
/// [`Part::Saber`]'s switch.
pub(crate) const SABER_CVAR: &str = "cg_statusBarSaber";
/// [`Part::Effects`]'s switch.
pub(crate) const EFFECTS_CVAR: &str = "cg_statusBarEffects";
/// [`Part::Toys`]'s switch.
pub(crate) const TOYS_CVAR: &str = "cg_statusBarToys";

/// How often what the bar shows is read again.
const EVERY: Duration = Duration::from_millis(250);

/// Where the bar stands.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Place {
    #[default]
    Bottom,
    Top,
}

impl Place {
    /// From [`POSITION_CVAR`]'s value: 1 the top, anything else the bottom.
    pub(crate) fn of(value: i64) -> Self {
        if value == 1 { Self::Top } else { Self::Bottom }
    }
}

/// A part of the bar, each with its own switch.
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
    Toys,
}

impl Part {
    /// Every part, in the bar's order (and the Settings rows').
    pub(crate) const ALL: [Self; 8] = [
        Self::Credits,
        Self::Shards,
        Self::Crystals,
        Self::Coins,
        Self::New,
        Self::Saber,
        Self::Effects,
        Self::Toys,
    ];

    /// Its switch: archived, on by default.
    pub(crate) const fn cvar(self) -> &'static str {
        match self {
            Self::Credits => CREDITS_CVAR,
            Self::Shards => SHARDS_CVAR,
            Self::Crystals => CRYSTALS_CVAR,
            Self::Coins => COINS_CVAR,
            Self::New => NEW_CVAR,
            Self::Saber => SABER_CVAR,
            Self::Effects => EFFECTS_CVAR,
            Self::Toys => TOYS_CVAR,
        }
    }

    /// The switch's line in `cvarlist`.
    const fn help(self) -> &'static str {
        match self {
            Self::Credits => "Status bar shows your credits",
            Self::Shards => "Status bar shows your kyber shards",
            Self::Crystals => "Status bar shows your dormant kybers still to attune",
            Self::Coins => "Status bar shows your DOOM coins",
            Self::New => "Status bar says when your Collection holds new things",
            Self::Saber => "Status bar shows the kyber on your saber",
            Self::Effects => {
                "Status bar shows what is on about you: blade extras, finisher, Tactical, Do not disturb, away"
            }
            Self::Toys => "Status bar shows every toy's switch, lit while on",
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// The parts switched on, a bit per [`Part`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Parts(u8);

impl Parts {
    /// Every part.
    #[cfg(test)]
    pub(crate) const ALL: Self = Self(0xff);

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
}

/// Register the bar's options.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        CVAR,
        true,
        CvarFlags::ARCHIVE,
        "Status bar with your credits, kyber, effects and toys",
    ))?;
    cvars.register(CvarDefinition::new(
        POSITION_CVAR,
        0_i64,
        CvarFlags::ARCHIVE,
        "Where the status bar stands: 0 bottom, 1 top",
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

/// A toy the bar shows: its name and its quick wheel picture's name (`None`: its
/// letters in a pill instead).
#[derive(Clone, Copy, Debug)]
struct Toy {
    #[cfg_attr(not(test), allow(dead_code))]
    name: &'static str,
    icon: Option<&'static str>,
    letters: &'static str,
}

/// How many toys the bar shows.
pub(crate) const TOY_COUNT: usize = 15;

/// The toys with a switch, in the order of `toy_help` ([`crate::toy_switch_words::TOYS`])
/// but the Tag, which has none ([`Inputs::toys`] keeps this order).
const TOYS: [Toy; TOY_COUNT] = [
    toy("Illuminate", Some("illuminate"), "IL"),
    toy("Cardboard box", Some("cardboard_box"), "BX"),
    toy("Order 66", Some("order66"), "66"),
    toy("Hax", Some("hax"), "HX"),
    toy("Ragdoll", Some("ragdoll"), "RG"),
    toy("Drone", Some("drone"), "DR"),
    toy("Boombox", Some("boombox"), "BB"),
    toy("Monitor", Some("monitor"), "MO"),
    toy("Box hunt", Some("box_hunt"), "BH"),
    toy("Quests", Some("quests"), "QU"),
    toy("Banana peel", Some("banana"), "BA"),
    toy("Prop gun", None, "PG"),
    toy("Fishing", Some("fishing"), "FI"),
    toy("WHEEL OF DOOM", Some("wheel_of_doom"), "WD"),
    toy("Mining", Some("mining"), "MI"),
];

const fn toy(name: &'static str, icon: Option<&'static str>, letters: &'static str) -> Toy {
    Toy {
        name,
        icon,
        letters,
    }
}

/// Index of each toy in [`TOYS`] and [`Inputs::toys`].
pub(crate) mod toys {
    pub(crate) const ILLUMINATE: usize = 0;
    pub(crate) const CARDBOARD_BOX: usize = 1;
    pub(crate) const ORDER66: usize = 2;
    pub(crate) const HAX: usize = 3;
    pub(crate) const RAGDOLL: usize = 4;
    pub(crate) const DRONE: usize = 5;
    pub(crate) const BOOMBOX: usize = 6;
    pub(crate) const MONITOR: usize = 7;
    pub(crate) const BOX_HUNT: usize = 8;
    pub(crate) const QUESTS: usize = 9;
    pub(crate) const BANANA: usize = 10;
    pub(crate) const PROPGUN: usize = 11;
    pub(crate) const FISHING: usize = 12;
    pub(crate) const WHEEL_OF_DOOM: usize = 13;
    pub(crate) const MINING: usize = 14;
}

/// Something on about the player, as its pill says it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Effect {
    pub(crate) words: &'static str,
    pub(crate) colour: Color,
}

/// Most effects the bar lists.
pub(crate) const MAX_EFFECTS: usize = 10;

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

/// What the bar shows, as last read.
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
    /// Each toy's switch, in [`TOYS`]' order ([`toys`]).
    pub(crate) toys: [bool; TOY_COUNT],
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

/// What an amount counts, told by its icon.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Money {
    /// The credits' sign (`crate::cards::mark`), in gold.
    Credits,
    /// The kyber shard (`crate::cards::shard`), in kyber cyan.
    Shards,
    /// The dormant kyber of `crate::kyber::TIERS`' tier at this index: its picture.
    Dormant(usize),
    /// A DOOM coin: a red coin with the wheel's spokes struck on it ([`coin`]).
    Doom,
}

/// One thing on the bar.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    /// A number after the icon of what it counts.
    Amount { value: u64, money: Money },
    /// New things in the Collection: "3 new in your Collection" (`words`), else a gold dot.
    New { count: u32, words: bool },
    /// A kyber's name in its tier's colour after its diamonds (`tier`), or the stock blade.
    Saber {
        name: &'static str,
        colour: Color,
        tier: Option<crate::unlockables::Rarity>,
    },
    /// A pill of something on.
    Effect(Effect),
    /// A toy's picture, lit while on.
    Toy { index: usize, on: bool },
}

/// Most pieces a bar holds.
const MAX_PIECES: usize = 4 + 1 + 2 + MAX_EFFECTS + TOY_COUNT;

/// A blade skin's piece: its name, tier and tier colour, or the stock blade's.
fn saber_piece(skin: Option<&'static str>) -> Piece {
    match skin.and_then(crate::unlockables::blade_skin) {
        Some(unlockable) => Piece::Saber {
            name: unlockable.name,
            colour: unlockable.tier.colour(),
            tier: Some(unlockable.tier),
        },
        None => Piece::Saber {
            name: "Stock blade",
            colour: color::MUTED,
            tier: None,
        },
    }
}

/// The pieces `inputs` give for `parts`, in the bar's order, into `out`; how many.
fn pieces(inputs: &Inputs, parts: Parts, out: &mut [(Part, Piece); MAX_PIECES]) -> usize {
    let mut count = 0;
    let mut push = |part: Part, piece: Piece| {
        if count < MAX_PIECES {
            out[count] = (part, piece);
            count += 1;
        }
    };
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
        if let Some(value) = value.filter(|_| parts.has(part)) {
            push(part, Piece::Amount { value, money });
        }
    }
    if let Some((count, words)) = inputs.new.filter(|_| parts.has(Part::New)) {
        push(Part::New, Piece::New { count, words });
    }
    if parts.has(Part::Saber) {
        push(Part::Saber, saber_piece(inputs.saber));
        if let Some(second) = inputs.second {
            push(Part::Saber, saber_piece(second));
        }
    }
    if parts.has(Part::Effects) {
        for effect in inputs.effects.iter().flatten() {
            push(Part::Effects, Piece::Effect(*effect));
        }
    }
    if parts.has(Part::Toys) {
        for (index, on) in inputs.toys.iter().enumerate() {
            push(Part::Toys, Piece::Toy { index, on: *on });
        }
    }
    count
}

/// Room between two pieces of a part, and between two parts (where a thin rule stands
/// in the middle), in pixels at 1080 lines.
const GAP: f32 = 10.0;
const PART_GAP: f32 = 25.0;
/// Room inside the bar's ends.
const PAD: f32 = 14.0;
/// The bar's height before the text size, and its distance from the screen's edge.
const HEIGHT: f32 = 34.0;
const MARGIN: f32 = 6.0;
/// Below the build label at the top (`version_overlay.rs`), while it shows.
const UNDER_LABEL: f32 = 22.0;
/// The text sizes: numbers, and pills.
const VALUE_SIZE: f32 = 18.0;
const PILL_SIZE: f32 = 13.0;
/// An amount's icon's height, and the room between it and its number.
const ICON: f32 = 20.0;
const ICON_GAP: f32 = 6.0;
/// A toy's picture, and a pill's room round its words.
const TOY: f32 = 22.0;
const PILL_PAD: f32 = 9.0;
/// The New part's dot.
const DOT: f32 = 8.0;
/// The kyber's tier diamonds' height ([`crate::menu::sjk::kit::tier_mark`]).
const DIAMOND: f32 = 10.0;
/// Room kept between the bar and what the HUD drew.
const CLEARANCE: f32 = 12.0;
/// Room past a measured word's end: a box exactly as wide as its text can round below
/// it and cut it to an ellipsis.
const SLACK: f32 = 8.0;
/// A HUD box reaching nearer the screen's middle than this, on either side, stands in
/// the bar's way: the bar passes it ([`place`]) rather than narrowing to nothing.
const MIDDLE: f32 = 180.0;
/// At the top the corners are left to the followed player's name, the readouts and the
/// kill feed, whose lines come and go: the bar takes at most this share of the width.
const TOP_SHARE: f32 = 0.56;
/// The DOOM coin's red, the WHEEL OF DOOM's (as the Inventory writes its coins).
const DOOM_RED: Color = crate::holocrons::rgb(0xE5_1A1A);

/// Where the bar goes along `edge`, `height` window pixels tall, clear of `boxes` (what
/// the HUD drew this frame, window pixels): its top and how far it may reach each side of
/// the screen's middle. `start` is the top's own first line (under the build label). A
/// box beside the bar's band narrows it; one within [`MIDDLE`] of the middle moves it past
/// (up from the bottom, down from the top), which can bring other boxes beside it.
pub(crate) fn place(
    edge: Place,
    viewport: [f32; 2],
    s: f32,
    height: f32,
    start: f32,
    boxes: &[Rect],
) -> (f32, f32) {
    let middle_x = viewport[0] * 0.5;
    let gap = CLEARANCE * s;
    let middle = MIDDLE * s;
    let widest = match edge {
        Place::Bottom => middle_x - MARGIN * s,
        Place::Top => viewport[0] * TOP_SHARE * 0.5,
    };
    let lowest = (viewport[1] - height).max(0.0);
    let mut y = match edge {
        Place::Bottom => viewport[1] - MARGIN * s - height,
        Place::Top => start,
    }
    .clamp(0.0, lowest);
    let mut half = widest;
    // Each pass either settles or moves the bar past one more box, always the same way.
    for _ in 0..=boxes.len() {
        half = widest;
        let mut moved = false;
        for rect in boxes {
            if rect.bottom() + gap <= y || rect.y >= y + height + gap {
                continue;
            }
            if rect.x < middle_x + middle && rect.right() > middle_x - middle {
                y = match edge {
                    Place::Bottom => rect.y - gap - height,
                    Place::Top => rect.bottom() + gap,
                }
                .clamp(0.0, lowest);
                moved = true;
                break;
            }
            let reach = if rect.right() <= middle_x {
                middle_x - rect.right()
            } else {
                rect.x - middle_x
            };
            half = half.min(reach - gap);
        }
        if !moved {
            break;
        }
    }
    (y, half.max(0.0))
}

/// How wide `money`'s icon is, in pixels at 1080 lines, `k` the text size.
fn icon_width(money: Money, k: f32) -> f32 {
    let height = ICON * k;
    match money {
        Money::Credits => crate::cards::mark::width(height),
        Money::Shards => crate::cards::shard::width(height),
        Money::Dormant(_) => height * 1.15,
        Money::Doom => height,
    }
}

/// A DOOM coin `side` window pixels across, its top left at `at`: a red disc with a
/// darker rim, the WHEEL OF DOOM's spokes struck across it and a glint. Shapes only, so
/// it is sharp at every size and reads as a coin without its colour.
fn coin(list: &mut sjk_ui::DrawList, [x, y]: [f32; 2], side: f32) {
    let deep = Color::new(DOOM_RED.r * 0.4, DOOM_RED.g * 0.4, DOOM_RED.b * 0.4, 1.0);
    let line = (side * 0.08).max(1.0);
    let disc = Rect::new(x, y, side, side);
    let _ = list.push(DrawCommand::RoundedRect {
        rect: disc,
        radius: side * 0.5,
        color: DOOM_RED,
    });
    let rim = side * 0.14;
    let _ = list.push(DrawCommand::Border {
        rect: Rect::new(x + rim, y + rim, side - 2.0 * rim, side - 2.0 * rim),
        radius: (side - 2.0 * rim) * 0.5,
        width: line,
        color: deep,
    });
    let centre = [x + side * 0.5, y + side * 0.5];
    let reach = side * 0.3;
    for step in 0..4 {
        let angle = step as f32 * std::f32::consts::FRAC_PI_4;
        let (sin, cos) = angle.sin_cos();
        let _ = list.push(DrawCommand::Line {
            from: [centre[0] - cos * reach, centre[1] - sin * reach],
            to: [centre[0] + cos * reach, centre[1] + sin * reach],
            width: line,
            color: deep,
        });
    }
    let hub = side * 0.16;
    let _ = list.push(DrawCommand::RoundedRect {
        rect: Rect::new(centre[0] - hub * 0.5, centre[1] - hub * 0.5, hub, hub),
        radius: hub * 0.5,
        color: deep,
    });
    let _ = list.push(DrawCommand::Line {
        from: [x + side * 0.24, y + side * 0.36],
        to: [x + side * 0.36, y + side * 0.22],
        width: line,
        color: Color::new(1.0, 1.0, 1.0, 0.55),
    });
}

/// Where each piece of `parts` (the part of each, `widths` their widths) goes along the bar, as offsets from the
/// content's left edge into `xs`, with [`GAP`] between pieces of a part and
/// [`PART_GAP`] between parts, as many as fit `room` (pixels at 1080 lines); returns how
/// many fit and the content's width. A piece that would cross `room` and every one
/// after it are left out.
fn fit(parts: &[Part], widths: &[f32], room: f32, xs: &mut [f32]) -> (usize, f32) {
    let mut x = 0.0_f32;
    let mut end = 0.0_f32;
    let mut fitted = 0;
    let mut last: Option<Part> = None;
    for ((part, width), slot) in parts.iter().zip(widths).zip(xs.iter_mut()) {
        let start = match last {
            None => 0.0,
            Some(previous) if previous == *part => x + GAP,
            Some(_) => x + PART_GAP,
        };
        let right = start + width;
        if right > room {
            break;
        }
        *slot = start;
        x = right;
        end = right;
        last = Some(*part);
        fitted += 1;
    }
    (fitted, end)
}

/// `value` with its digits in groups of three, apart by a space ("12 340"), as the
/// Collection writes credits ([`crate::cards::grouped`]), without allocating.
struct Grouped(u64);

impl fmt::Display for Grouped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut digits = [0_u8; 20];
        let mut value = self.0;
        let mut len = 0;
        loop {
            digits[len] = b'0' + (value % 10) as u8;
            len += 1;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        for index in (0..len).rev() {
            f.write_char(char::from(digits[index]))?;
            if index > 0 && index % 3 == 0 {
                f.write_char(' ')?;
            }
        }
        Ok(())
    }
}

/// How text measures as the bar draws it: the display family's for numbers, the body's
/// for words, in window pixels.
#[derive(Clone, Copy)]
pub(crate) struct Measures<'a> {
    pub(crate) display: Measure<'a>,
    pub(crate) body: Measure<'a>,
}

/// What the bar was last laid out from: the same again keeps its shapes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Laid {
    inputs: Inputs,
    parts: Parts,
    viewport: [f32; 2],
    /// The text size, and the bar's top and reach from [`place`].
    k: f32,
    top: f32,
    half: f32,
    /// Whether the dormant kybers' pictures were in the atlas.
    pictures: bool,
}

/// The bar's height in pixels at 1080 lines at text size `k`.
fn bar_height(k: f32) -> f32 {
    HEIGHT * k.max(1.0)
}

/// The bar's laid-out storage, kept for the game's run.
pub(crate) struct State {
    canvas: MenuCanvas,
    /// What the bar shows, as last read, and when.
    inputs: Inputs,
    read: Option<Instant>,
    /// What [`Self::canvas`] holds was laid out from, and whether it drew anything.
    laid: Option<(Laid, bool)>,
    /// The bar was laid out this frame.
    drawn: bool,
    /// Text formatted to be measured.
    scratch: String,
    /// What the New part says ([`Nudge`]).
    nudge: Nudge,
    /// What a world shot shows, kept as set.
    #[cfg(test)]
    pub(crate) preview: Option<Inputs>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            canvas: MenuCanvas::with_capacities(40, 32, 160),
            inputs: Inputs::default(),
            read: None,
            laid: None,
            drawn: false,
            scratch: String::with_capacity(32),
            nudge: Nudge::default(),
            #[cfg(test)]
            preview: None,
        }
    }
}

impl State {
    /// The bar's shapes, after [`crate::GpuState::append_status_bar`] laid it out.
    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.canvas.draw_list()
    }

    /// Whether the bar is drawn this frame (world shots: the 2D pass runs for it without
    /// a session).
    #[cfg(test)]
    pub(crate) fn drawn(&self) -> bool {
        self.drawn
    }

    /// Forget the kept layout, so the next frame lays the bar out again (the timing
    /// shot measures both ways).
    #[cfg(test)]
    pub(crate) fn forget_layout(&mut self) {
        self.laid = None;
    }

    /// Whether what the bar shows is due to be read again at `now`.
    fn due(&self, now: Instant) -> bool {
        self.read
            .is_none_or(|at| now.saturating_duration_since(at) >= EVERY)
    }

    /// A piece's width in pixels at 1080 lines, `s` the layout scale.
    fn width(&mut self, piece: &Piece, measures: Measures<'_>, s: f32) -> f32 {
        let k = measures.body.scale().max(0.5);
        match *piece {
            Piece::Amount { value, money } => {
                self.scratch.clear();
                let _ = write!(self.scratch, "{}", Grouped(value));
                let number =
                    measures
                        .display
                        .width(&self.scratch, VALUE_SIZE * s, TextFace::Semibold);
                icon_width(money, k) + ICON_GAP * k + number / s
            }
            Piece::Saber { name, tier, .. } => {
                let mark = tier.map_or(0.0, |tier| kit::tier_mark_width(tier, DIAMOND * k));
                let lead = if mark > 0.0 { mark + 6.0 * k } else { 0.0 };
                lead + measures.body.width(name, PILL_SIZE * s, TextFace::Semibold) / s
            }
            Piece::New { count, words } => {
                if !words {
                    return DOT * k;
                }
                self.scratch.clear();
                let _ = write!(
                    self.scratch,
                    "{} new in your Collection",
                    Grouped(count.into())
                );
                (DOT + 6.0) * k
                    + measures
                        .body
                        .width(&self.scratch, PILL_SIZE * s, TextFace::Semibold)
                        / s
            }
            Piece::Effect(effect) => {
                measures
                    .body
                    .width(effect.words, PILL_SIZE * s, TextFace::Semibold)
                    / s
                    + PILL_PAD * 2.0 * k
            }
            Piece::Toy { .. } => TOY * k,
        }
    }

    /// Lay the bar out from `laid` (its top and reach found by [`place`]) with the parts
    /// it names of [`Self::inputs`]. Whether anything was drawn.
    fn build(&mut self, laid: &Laid, measures: Measures<'_>) -> bool {
        let viewport = laid.viewport;
        let s = crate::menu::sjk::scale(viewport);
        let k = laid.k;
        let parts = laid.parts;
        let mut list = [(
            Part::Credits,
            Piece::Toy {
                index: 0,
                on: false,
            },
        ); MAX_PIECES];
        let count = pieces(&self.inputs, parts, &mut list);
        self.canvas.begin_transparent(viewport);
        if count == 0 {
            self.canvas.finish(0);
            return false;
        }
        let mut widths = [0.0_f32; MAX_PIECES];
        let mut owners = [Part::Credits; MAX_PIECES];
        for (index, (part, piece)) in list[..count].iter().enumerate() {
            widths[index] = self.width(piece, measures, s);
            owners[index] = *part;
        }
        let room = 2.0 * laid.half / s - 2.0 * PAD;
        let mut xs = [0.0_f32; MAX_PIECES];
        let (fitted, content) = fit(&owners[..count], &widths[..count], room, &mut xs);
        if fitted == 0 {
            self.canvas.finish(0);
            return false;
        }
        let height = bar_height(k);
        let width = content + PAD * 2.0;
        let left = (viewport[0] - width * s) * 0.5;
        let top = laid.top;
        let bar = Rect::new(left, top, width * s, height * s);
        self.glass(bar, s);
        let middle = top + bar.height * 0.5;
        let mut previous: Option<Part> = None;
        for index in 0..fitted {
            let (part, piece) = list[index];
            let x = left + (PAD + xs[index]) * s;
            if previous.is_some_and(|previous| previous != part) {
                let rule = x - PART_GAP * 0.5 * s;
                let _ = self.canvas.draw_list_mut().push(DrawCommand::SolidRect {
                    rect: Rect::new(rule, top + 8.0 * s, 1.0_f32.max(s), bar.height - 16.0 * s),
                    color: color::alpha(color::HOLO, 0.3),
                });
            }
            previous = Some(part);
            let piece_width = widths[index] * s;
            self.piece(piece, x, middle, piece_width, s, k);
        }
        self.canvas.finish(0);
        true
    }

    /// The bar's glass: the navy ground, faint, with a holo edge.
    fn glass(&mut self, bar: Rect, s: f32) {
        let radius = bar.height * 0.5;
        let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: bar,
            radius,
            color: color::alpha(color::SPACE, 0.78),
        });
        let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
            rect: bar,
            radius,
            width: 1.0_f32.max(1.25 * s),
            color: color::alpha(color::HOLO, 0.3),
        });
    }

    /// Draw `piece` from `x` on the bar's middle line `middle`, `width` wide (window
    /// pixels).
    fn piece(&mut self, piece: Piece, x: f32, middle: f32, width: f32, s: f32, k: f32) {
        match piece {
            Piece::Amount { value, money } => {
                let icon = ICON * k * s;
                let at = [x, middle - icon * 0.5];
                let list = self.canvas.draw_list_mut();
                match money {
                    Money::Credits => {
                        let _ = crate::cards::mark::draw(list, at, icon, color::GOLD_BRIGHT);
                    }
                    Money::Shards => {
                        let _ =
                            crate::cards::shard::draw(list, at, icon, crate::cards::shard::KYBER);
                    }
                    Money::Dormant(index) => {
                        let tier = &crate::kyber::TIERS[index];
                        let side = icon * 1.15;
                        if crate::holocrons::icons::is_loaded(tier.slot) {
                            let _ = list.push(DrawCommand::TexturedQuad {
                                rect: Rect::new(x, middle - side * 0.5, side, side),
                                texture: crate::holocrons::icons::texture(tier.slot),
                                color: Color::new(1.0, 1.0, 1.0, 1.0),
                            });
                        } else {
                            crate::holocrons::gem::draw(
                                list,
                                [x + side * 0.5, middle],
                                side * 0.45,
                                tier.colour,
                                1.0,
                                crate::holocrons::gem::ROWS,
                            );
                        }
                    }
                    Money::Doom => coin(list, at, icon),
                }
                let lead = (icon_width(money, k) + ICON_GAP * k) * s;
                text(
                    &mut self.canvas,
                    TextFamily::Display,
                    format_args!("{}", Grouped(value)),
                    Rect::new(
                        x + lead,
                        middle - VALUE_SIZE * s * 0.75,
                        width - lead + SLACK * s,
                        VALUE_SIZE * s * 1.5,
                    ),
                    VALUE_SIZE * s,
                    color::TEXT,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
            }
            Piece::Saber { name, colour, tier } => {
                let mark = tier.map_or(0.0, |tier| {
                    kit::tier_mark(
                        &mut self.canvas,
                        &crate::menu::sjk::Frame {
                            s: 1.0,
                            origin: [0.0; 2],
                        },
                        [x, middle],
                        tier,
                        DIAMOND * k * s,
                        false,
                    )
                });
                let room = if mark > 0.0 { mark + 6.0 * k * s } else { 0.0 };
                text(
                    &mut self.canvas,
                    TextFamily::Body,
                    format_args!("{name}"),
                    Rect::new(
                        x + room,
                        middle - PILL_SIZE * s,
                        width - room + SLACK * s,
                        PILL_SIZE * 2.0 * s,
                    ),
                    PILL_SIZE * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
            }
            Piece::New { count, words } => {
                let dot = DOT * k * s;
                let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: Rect::new(x, middle - dot * 0.5, dot, dot),
                    radius: dot * 0.5,
                    color: color::GOLD_BRIGHT,
                });
                if words {
                    let room = (DOT + 6.0) * k * s;
                    text(
                        &mut self.canvas,
                        TextFamily::Body,
                        format_args!("{} new in your Collection", Grouped(count.into())),
                        Rect::new(
                            x + room,
                            middle - PILL_SIZE * s,
                            width - room + SLACK * s,
                            PILL_SIZE * 2.0 * s,
                        ),
                        PILL_SIZE * s,
                        color::GOLD_BRIGHT,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                }
            }
            Piece::Effect(effect) => {
                let height = (PILL_SIZE + 9.0) * k * s;
                let pill = Rect::new(x, middle - height * 0.5, width, height);
                let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: pill,
                    radius: height * 0.5,
                    color: color::alpha(effect.colour, 0.14),
                });
                let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                    rect: pill,
                    radius: height * 0.5,
                    width: 1.0_f32.max(s),
                    color: color::alpha(effect.colour, 0.5),
                });
                text(
                    &mut self.canvas,
                    TextFamily::Body,
                    format_args!("{}", effect.words),
                    pill,
                    PILL_SIZE * s,
                    effect.colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
            Piece::Toy { index, on } => {
                let side = TOY * k * s;
                let rect = Rect::new(x, middle - side * 0.5 - 2.0 * s, side, side);
                let toy = TOYS[index];
                let alpha = if on { 1.0 } else { 0.3 };
                match toy.icon.and_then(crate::quick_wheel::catalog::icon_index) {
                    Some(icon) => {
                        let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                            rect,
                            texture: crate::ui_renderer::wheel_icon(icon),
                            color: Color::new(1.0, 1.0, 1.0, alpha),
                        });
                    }
                    None => {
                        let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                            rect,
                            radius: side * 0.3,
                            width: 1.0_f32.max(s),
                            color: color::alpha(color::HOLO, alpha * 0.8),
                        });
                        text(
                            &mut self.canvas,
                            TextFamily::Display,
                            format_args!("{}", toy.letters),
                            rect,
                            11.0 * s,
                            color::alpha(color::TEXT, alpha),
                            FontWeight::Semibold,
                            TextAlign::Center,
                        );
                    }
                }
                if on {
                    let dot = 4.0 * s;
                    let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: Rect::new(x + (side - dot) * 0.5, rect.bottom() + 2.0 * s, dot, dot),
                        radius: dot * 0.5,
                        color: color::GOLD_BRIGHT,
                    });
                }
            }
        }
    }
}

impl crate::GpuState {
    /// What the bar shows now: the hub's numbers, the own look and the toys' switches.
    fn status_bar_inputs(&self) -> Inputs {
        let mut inputs = Inputs::default();
        if let Some((credits, shards, crystals, coins)) =
            crate::player_identity::with_snapshot(|snapshot| {
                let me = snapshot.me.as_ref();
                // How many dormant kybers wait, and the best tier among them.
                let crystals = me.map(|me| {
                    me.holocrons
                        .iter()
                        .filter(|holocron| holocron.opened.is_none())
                        .filter_map(|holocron| crate::kyber::from_id(&holocron.tier))
                        .fold((0_u32, 0_usize), |(count, best), tier| {
                            (count + 1, best.max(tier.index))
                        })
                });
                (
                    me.map(|me| me.credits),
                    me.map(|me| me.shards),
                    crystals,
                    snapshot.wheel_of_doom.as_ref().map(|wheel| wheel.coins),
                )
            })
        {
            inputs.credits = credits;
            inputs.shards = shards;
            if let Some((count, best)) = crystals {
                inputs.crystals = Some(count);
                inputs.crystal_tier = best;
            }
            inputs.coins = coins;
        }
        let own = self.looks.own();
        inputs.saber = own.saber_skin;
        if let crate::looks::SecondSkin::Own(second) = own.second
            && second != own.saber_skin
        {
            inputs.second = Some(second);
        }
        // The blade's extras switched on (`blade_extras.rs`): a word of the options names
        // an extra on.
        if let Some(extras) = own.saber_skin.and_then(crate::blade_extras::of) {
            for extra in &extras.list {
                if own.options_off & crate::blade_skin_file::extras::option_bit(&extra.id) != 0 {
                    inputs.push_effect(Effect {
                        words: extra.name.as_str(),
                        colour: color::GOLD_BRIGHT,
                    });
                }
            }
        }
        if let Some(finisher) = own.finisher.and_then(crate::finishers::get) {
            inputs.push_effect(Effect {
                words: finisher.name,
                colour: finisher.tier.colour(),
            });
        }
        let console = self.console.as_ref();
        let on = |cvar: &str| {
            console
                .and_then(|console| console.bool_cvar(cvar))
                .unwrap_or(false)
        };
        if let Some(words) = tactical_words(crate::tactical::level()) {
            inputs.push_effect(Effect {
                words,
                colour: color::HOLO,
            });
        }
        if on(crate::chat::quiet::CVAR) {
            inputs.push_effect(Effect {
                words: "Do not disturb",
                colour: color::EMBER,
            });
        }
        if crate::away_emote::own_away() {
            inputs.push_effect(Effect {
                words: "Away",
                colour: color::MUTED,
            });
        }
        if on(crate::toon_choice::CVAR) {
            inputs.push_effect(Effect {
                words: "Toon",
                colour: color::HOLO,
            });
        }
        let mut switches = [false; TOY_COUNT];
        switches[toys::ILLUMINATE] = self.illuminate.lit();
        switches[toys::CARDBOARD_BOX] = self.cardboard_box.on();
        switches[toys::ORDER66] = self.order66.on();
        switches[toys::HAX] = self.hax.on();
        switches[toys::RAGDOLL] = self.ragdoll.on();
        switches[toys::DRONE] = self.drone.on();
        switches[toys::BOOMBOX] = self.boombox.on();
        switches[toys::MONITOR] = self.monitor.on();
        switches[toys::BOX_HUNT] = self.box_hunt.on();
        switches[toys::QUESTS] = self.quests.on();
        switches[toys::BANANA] = self.banana.on();
        switches[toys::PROPGUN] = self.propgun.on();
        switches[toys::FISHING] = self.fishing.on();
        switches[toys::WHEEL_OF_DOOM] = self.wheel_of_doom.running();
        switches[toys::MINING] = self.mining.on();
        inputs.toys = switches;
        inputs
    }

    /// Lay the status bar out when `cg_statusBar` is on and play shows (`shown`: no
    /// menu, console, scoreboard or intermission over it, the HUD's elements not hidden),
    /// clear of what the HUD drew this frame (`self.hud.element_bounds`, filled before);
    /// whether it was drawn, so its shapes go into the frame's layers.
    pub(crate) fn append_status_bar(&mut self, viewport: [f32; 2], shown: bool) -> bool {
        let console = self.console.as_ref();
        let on = console
            .and_then(|console| console.bool_cvar(CVAR))
            .unwrap_or(true);
        // Over a demo it would tell the viewer's own things, not the recording's.
        let demo = self.demo_session.is_some() && self.live_session.is_none();
        #[cfg(test)]
        let (on, demo, shown) = match self.status_bar.preview {
            Some(_) => (true, false, true),
            None => (on, demo, shown),
        };
        if !on || demo || !shown {
            self.status_bar.drawn = false;
            return false;
        }
        let edge = Place::of(
            console
                .and_then(|console| console.integer_cvar(POSITION_CVAR))
                .unwrap_or(0),
        );
        let mut parts = Parts::read(console);
        // Tactical mode switches the toys off: their faint row would say nothing.
        if crate::tactical::level() != crate::tactical::Level::Off {
            parts = parts.without(Part::Toys);
        }
        let label = console.is_none_or(|console| {
            console
                .bool_cvar(crate::version_overlay::CVAR)
                .unwrap_or(true)
        });
        let now = Instant::now();
        if self.status_bar.due(now) {
            self.status_bar.inputs = self.status_bar_inputs();
            self.status_bar.inputs.new = self.status_bar.nudge.follow(
                crate::collection_seen::total(),
                map_key(&self.world_load_map),
            );
            self.status_bar.read = Some(now);
        }
        #[cfg(test)]
        if let Some(preview) = self.status_bar.preview {
            self.status_bar.inputs = preview;
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
        let s = crate::menu::sjk::scale(viewport);
        let k = measures.body.scale().max(0.5);
        let height = bar_height(k) * s;
        // What the HUD drew this frame: the bar keeps clear of what stays, and steps
        // aside while something passing stands where it is.
        use crate::hud::placement::Element;
        let mut boxes = [Rect::new(0.0, 0.0, 0.0, 0.0); crate::hud::placement::COUNT];
        let mut held = 0;
        let mut passing = [None; 2];
        for (element, rect) in self.hud.element_bounds.iter() {
            match element {
                Element::Selection => passing[0] = Some(rect),
                Element::Vote => passing[1] = Some(rect),
                // Its lines come and go at the top right: [`TOP_SHARE`] keeps clear.
                Element::KillFeed | Element::Chat => {}
                _ => {
                    boxes[held] = rect;
                    held += 1;
                }
            }
        }
        let start = (MARGIN + if label { UNDER_LABEL } else { 0.0 }) * s;
        let (top, half) = place(edge, viewport, s, height, start, &boxes[..held]);
        let band = Rect::new(viewport[0] * 0.5 - half, top, half * 2.0, height);
        let gives_way = passing.iter().flatten().any(|rect| rect.overlaps(band))
            || match edge {
                // The weapon row's icons and name stand at the bottom's middle.
                Place::Bottom => self.hud.weapon_select.shown.is_some(),
                // The warmup line stands under the top's middle.
                Place::Top => self.hud.warmup_shown(),
            };
        if gives_way {
            self.status_bar.drawn = false;
            return false;
        }
        let best = self.status_bar.inputs.crystal_tier;
        let tier = best.min(crate::kyber::COUNT - 1);
        let pictures = crate::holocrons::icons::is_loaded(crate::kyber::TIERS[tier].slot);
        let laid = Laid {
            inputs: self.status_bar.inputs,
            parts,
            viewport,
            k,
            top,
            half,
            pictures,
        };
        let drawn = match self.status_bar.laid {
            Some((kept, drawn)) if kept == laid => drawn,
            _ => {
                let drawn = self.status_bar.build(&laid, measures);
                self.status_bar.laid = Some((laid, drawn));
                drawn
            }
        };
        self.status_bar.drawn = drawn;
        if drawn {
            target.append(&self.status_bar.canvas, viewport);
        }
        drawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(value: u64) -> Piece {
        Piece::Amount {
            value,
            money: Money::Credits,
        }
    }

    #[test]
    fn the_position_reads_bottom_unless_top() {
        assert_eq!(Place::of(0), Place::Bottom);
        assert_eq!(Place::of(1), Place::Top);
        assert_eq!(Place::of(2), Place::Bottom);
        assert_eq!(Place::of(-1), Place::Bottom);
        assert_eq!(Place::default(), Place::Bottom);
    }

    /// The bar is on, at the bottom, with every part, for a profile that never chose.
    #[test]
    fn the_bar_is_on_at_the_bottom_by_default() {
        let mut cvars = CvarRegistry::default();
        register(&mut cvars).unwrap();
        assert_eq!(
            cvars.get(CVAR).unwrap().value,
            sjk_shell::CvarValue::Bool(true)
        );
        assert_eq!(
            cvars.get(POSITION_CVAR).unwrap().value,
            sjk_shell::CvarValue::Integer(0)
        );
        for part in Part::ALL {
            assert_eq!(
                cvars.get(part.cvar()).unwrap().value,
                sjk_shell::CvarValue::Bool(true)
            );
        }
        assert!(!Parts::ALL.without(Part::Toys).has(Part::Toys));
        assert!(Parts::ALL.without(Part::Toys).has(Part::Credits));
    }

    const SCREEN: [f32; 2] = [1920.0, 1080.0];
    const BAR: f32 = 34.0;

    /// Nothing drawn: the bar sits on its edge with the screen's width, less the margins
    /// (at the top, its share of it).
    #[test]
    fn an_empty_screen_puts_the_bar_on_its_edge() {
        let (top, half) = place(Place::Bottom, SCREEN, 1.0, BAR, 28.0, &[]);
        assert_eq!(top, 1080.0 - MARGIN - BAR);
        assert_eq!(half, 960.0 - MARGIN);
        let (top, half) = place(Place::Top, SCREEN, 1.0, BAR, 28.0, &[]);
        assert_eq!(top, 28.0);
        assert_eq!(half, 1920.0 * TOP_SHARE * 0.5);
    }

    /// The health and Force blocks in the bottom corners narrow the bar; a weapon box in
    /// the bottom's middle lifts it above; the match timer at the top's middle drops it
    /// below.
    #[test]
    fn the_bar_keeps_clear_of_the_hud() {
        let health = Rect::new(0.0, 940.0, 300.0, 140.0);
        let force = Rect::new(1640.0, 940.0, 280.0, 140.0);
        let (top, half) = place(Place::Bottom, SCREEN, 1.0, BAR, 28.0, &[health, force]);
        assert_eq!(top, 1080.0 - MARGIN - BAR);
        assert_eq!(half, 960.0 - 300.0 - CLEARANCE);
        let weapon = Rect::new(800.0, 1000.0, 320.0, 70.0);
        let (top, half) = place(
            Place::Bottom,
            SCREEN,
            1.0,
            BAR,
            28.0,
            &[health, force, weapon],
        );
        assert_eq!(top, 1000.0 - CLEARANCE - BAR);
        // Still beside the corner blocks, which reach above it.
        assert_eq!(half, 960.0 - 300.0 - CLEARANCE);
        let timer = Rect::new(870.0, 28.0, 180.0, 42.0);
        let (top, _) = place(Place::Top, SCREEN, 1.0, BAR, 28.0, &[timer]);
        assert_eq!(top, 70.0 + CLEARANCE);
        // A box well above the bar's band leaves it alone.
        let high = Rect::new(800.0, 300.0, 320.0, 40.0);
        let (top, _) = place(Place::Bottom, SCREEN, 1.0, BAR, 28.0, &[high]);
        assert_eq!(top, 1080.0 - MARGIN - BAR);
    }

    /// Boxes stacked up the middle lift the bar past each in turn, and it never leaves
    /// the screen.
    #[test]
    fn stacked_boxes_lift_the_bar_past_each() {
        let low = Rect::new(900.0, 1040.0, 120.0, 40.0);
        let higher = Rect::new(880.0, 990.0, 160.0, 30.0);
        let (top, _) = place(Place::Bottom, SCREEN, 1.0, BAR, 28.0, &[higher, low]);
        assert_eq!(top, 990.0 - CLEARANCE - BAR);
        let wall = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        let (top, _) = place(Place::Bottom, SCREEN, 1.0, BAR, 28.0, &[wall]);
        assert_eq!(top, 0.0);
    }

    #[test]
    fn a_doom_coin_and_the_icons_have_their_sizes() {
        let mut list = sjk_ui::DrawList::new(16);
        coin(&mut list, [10.0, 10.0], 20.0);
        assert_eq!(list.len(), 8);
        for money in [
            Money::Credits,
            Money::Shards,
            Money::Dormant(3),
            Money::Doom,
        ] {
            let width = icon_width(money, 1.0);
            assert!(width > 5.0 && width <= ICON * 1.15, "{money:?}: {width}");
        }
    }

    #[test]
    fn each_part_has_its_own_switch() {
        let mut cvars: Vec<&str> = Part::ALL.iter().map(|part| part.cvar()).collect();
        cvars.sort_unstable();
        cvars.dedup();
        assert_eq!(cvars.len(), Part::ALL.len());
        assert!(cvars.iter().all(|cvar| cvar.starts_with("cg_statusBar")));
        let only_toys = Parts::from_switches(|part| part == Part::Toys);
        assert!(only_toys.has(Part::Toys));
        assert!(Part::ALL[..7].iter().all(|part| !only_toys.has(*part)));
        assert!(Part::ALL.iter().all(|part| Parts::ALL.has(*part)));
        assert_eq!(Parts::from_switches(|_| true), Parts::ALL);
    }

    #[test]
    fn unknown_numbers_and_switched_off_parts_are_left_out() {
        let mut inputs = Inputs {
            credits: Some(1250),
            coins: Some(1),
            ..Inputs::default()
        };
        inputs.toys[toys::HAX] = true;
        let mut out = [(Part::Credits, amount(0)); MAX_PIECES];
        // Shards and crystals unknown: not shown; one DOOM coin is said in the singular.
        let count = pieces(&inputs, Parts::ALL, &mut out);
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
        assert_eq!(
            out[2],
            (
                Part::Saber,
                Piece::Saber {
                    name: "Stock blade",
                    colour: color::MUTED,
                    tier: None
                }
            )
        );
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
        assert_eq!(pieces(&inputs, without, &mut out), 2);
        assert_eq!(out[1].0, Part::Saber);
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
        let mut out = [(Part::Credits, amount(0)); MAX_PIECES];
        let parts = Parts::from_switches(|part| matches!(part, Part::Saber | Part::Effects));
        assert_eq!(pieces(&inputs, parts, &mut out), 3);
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

    #[test]
    fn pieces_sit_apart_with_more_room_between_parts() {
        let parts = [Part::Credits, Part::Shards, Part::Toys, Part::Toys];
        let mut xs = [0.0; 4];
        let (fitted, width) = fit(&parts, &[20.0; 4], 1000.0, &mut xs);
        assert_eq!(fitted, 4);
        assert_eq!(
            xs,
            [
                0.0,
                20.0 + PART_GAP,
                40.0 + 2.0 * PART_GAP,
                60.0 + 2.0 * PART_GAP + GAP
            ]
        );
        assert_eq!(width, 80.0 + 2.0 * PART_GAP + GAP);
    }

    #[test]
    fn what_does_not_fit_is_dropped_from_the_end() {
        let parts = [Part::Credits, Part::Toys, Part::Toys];
        let mut xs = [0.0; 3];
        // Room for the credits and one toy only.
        let room = 20.0 + PART_GAP + 20.0 + GAP / 2.0;
        let (fitted, width) = fit(&parts, &[20.0; 3], room, &mut xs);
        assert_eq!(fitted, 2);
        assert_eq!(width, 40.0 + PART_GAP);
        // Nothing fits a room narrower than the first piece.
        assert_eq!(fit(&parts, &[20.0; 3], 10.0, &mut xs), (0, 0.0));
    }

    #[test]
    fn numbers_are_grouped_by_three() {
        for (value, words) in [
            (0, "0"),
            (7, "7"),
            (999, "999"),
            (1000, "1 000"),
            (12_340, "12 340"),
            (1_234_567, "1 234 567"),
            (u64::MAX, "18 446 744 073 709 551 615"),
        ] {
            assert_eq!(Grouped(value).to_string(), words);
            assert_eq!(Grouped(value).to_string(), crate::cards::grouped(value));
        }
    }

    #[test]
    fn every_toy_with_a_picture_has_it_in_the_quick_wheel() {
        for toy in TOYS {
            if let Some(icon) = toy.icon {
                assert!(
                    crate::quick_wheel::catalog::icon_index(icon).is_some(),
                    "{}",
                    toy.name
                );
            }
            assert!(
                !toy.letters.is_empty() && toy.letters.len() <= 2,
                "{}",
                toy.name
            );
        }
        // Every toy `toy_help` lists but the Tag, which has no switch, is on the bar.
        for (name, _, _) in crate::toy_switch_words::TOYS {
            assert!(
                name == "Tag" || TOYS.iter().any(|toy| toy.name == name),
                "{name}"
            );
        }
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
        let mut out = [(Part::Credits, amount(0)); MAX_PIECES];
        let inputs = Inputs {
            new: Some((1, false)),
            ..Inputs::default()
        };
        let parts = Parts::from_switches(|part| part == Part::New);
        assert_eq!(pieces(&inputs, parts, &mut out), 1);
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
        let count = pieces(&inputs, off, &mut out);
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
        let mut out = [(Part::Credits, amount(0)); MAX_PIECES];
        let parts = Parts::from_switches(|part| part == Part::Crystals);
        for (tier, shown) in [(0, 0), (2, 2), (99, crate::kyber::COUNT - 1)] {
            let inputs = Inputs {
                crystals: Some(2),
                crystal_tier: tier,
                ..Inputs::default()
            };
            assert_eq!(pieces(&inputs, parts, &mut out), 1);
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
    fn a_full_bar_fits_its_storage() {
        let mut inputs = Inputs {
            credits: Some(u64::MAX),
            shards: Some(1),
            crystals: Some(3),
            crystal_tier: 3,
            coins: Some(4),
            new: Some((3, true)),
            saber: None,
            second: Some(None),
            ..Inputs::default()
        };
        inputs.toys = [true; TOY_COUNT];
        for _ in 0..MAX_EFFECTS {
            inputs.push_effect(Effect {
                words: "Tactical medium",
                colour: color::HOLO,
            });
        }
        let mut out = [(Part::Credits, amount(0)); MAX_PIECES];
        assert_eq!(pieces(&inputs, Parts::ALL, &mut out), MAX_PIECES);
    }
}
