//! The status bar (Sol's request, 15/11/2026): one compact line at the top or the bottom
//! of the screen, in the SJK UI's look, with what the player carries and wears: their
//! credits, kyber shards, unopened kyber crystals and DOOM coins (the hub's numbers), the
//! blade skin on their saber in its tier's colour, what is on about them (the blade's
//! extras, the finisher, Tactical mode, Do not disturb, away, toon shading) and every
//! toy's switch, its picture lit while on.
//!
//! [`CVAR`] (`cg_statusBar`, off by default) puts it at the top (1) or the bottom (2);
//! each part has its own switch ([`Part::cvar`], all on), so a player keeps only what
//! they want. It shows over play only, as the HUD's elements do: not under the menus,
//! the console, the scoreboard or an intermission, nor while the HUD switch hides the
//! elements (`cg_drawHud 0`). A part without a value (no hub, the DOOM coins not read
//! yet) is left out rather than shown empty, and what does not fit the screen's width
//! is dropped from the end ([`fit`]).
//!
//! What it shows is read four times a second into fixed storage ([`Inputs`]); a frame
//! lays out and draws from it without allocating.

use crate::console::ViewerConsole;
use crate::menu::sjk::{color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::sjk_chat_look::Measure;
use crate::text::TextFace;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};
use std::fmt::{self, Write as _};
use std::time::{Duration, Instant};

/// Where the bar stands: 0 off (the default), 1 the top, 2 the bottom.
pub(crate) const CVAR: &str = "cg_statusBar";

/// [`Part::Credits`]'s switch.
pub(crate) const CREDITS_CVAR: &str = "cg_statusBarCredits";
/// [`Part::Shards`]'s switch.
pub(crate) const SHARDS_CVAR: &str = "cg_statusBarShards";
/// [`Part::Crystals`]'s switch.
pub(crate) const CRYSTALS_CVAR: &str = "cg_statusBarCrystals";
/// [`Part::Coins`]'s switch.
pub(crate) const COINS_CVAR: &str = "cg_statusBarCoins";
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
    Off,
    Top,
    Bottom,
}

impl Place {
    /// From [`CVAR`]'s value: anything but 1 and 2 is off.
    pub(crate) fn of(value: i64) -> Self {
        match value {
            1 => Self::Top,
            2 => Self::Bottom,
            _ => Self::Off,
        }
    }
}

/// A part of the bar, each with its own switch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Part {
    Credits,
    Shards,
    Crystals,
    Coins,
    Saber,
    Effects,
    Toys,
}

impl Part {
    /// Every part, in the bar's order (and the Settings rows').
    pub(crate) const ALL: [Self; 7] = [
        Self::Credits,
        Self::Shards,
        Self::Crystals,
        Self::Coins,
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
            Self::Crystals => "Status bar shows your unopened kyber crystals",
            Self::Coins => "Status bar shows your DOOM coins",
            Self::Saber => "Status bar shows the blade skin on your saber",
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
    pub(crate) const ALL: Self = Self(0x7f);

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
}

/// Register the bar's options.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        CVAR,
        0_i64,
        CvarFlags::ARCHIVE,
        "Status bar with your credits, blade, effects and toys: 0 off, 1 top, 2 bottom",
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
    pub(crate) coins: Option<u64>,
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
            coins: None,
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

/// One thing on the bar.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    /// A number and what it counts.
    Amount { value: u64, unit: &'static str },
    /// A blade skin's name in its tier's colour, or the stock blade.
    Saber { name: &'static str, colour: Color },
    /// A pill of something on.
    Effect(Effect),
    /// A toy's picture, lit while on.
    Toy { index: usize, on: bool },
}

/// Most pieces a bar holds.
const MAX_PIECES: usize = 4 + 2 + MAX_EFFECTS + TOY_COUNT;

/// What `unit` reads for `value`: one or many.
fn unit(value: u64, one: &'static str, many: &'static str) -> &'static str {
    if value == 1 { one } else { many }
}

/// A blade skin's piece: its name and tier colour, or the stock blade's.
fn saber_piece(skin: Option<&'static str>) -> Piece {
    match skin.and_then(crate::unlockables::blade_skin) {
        Some(unlockable) => Piece::Saber {
            name: unlockable.name,
            colour: unlockable.tier.colour(),
        },
        None => Piece::Saber {
            name: "Stock blade",
            colour: color::MUTED,
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
        (Part::Credits, inputs.credits, ("credit", "credits")),
        (Part::Shards, inputs.shards, ("shard", "shards")),
        (
            Part::Crystals,
            inputs.crystals.map(u64::from),
            ("crystal", "crystals"),
        ),
        (Part::Coins, inputs.coins, ("DOOM coin", "DOOM coins")),
    ];
    for (part, value, (one, many)) in amounts {
        if let Some(value) = value.filter(|_| parts.has(part)) {
            push(
                part,
                Piece::Amount {
                    value,
                    unit: unit(value, one, many),
                },
            );
        }
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
/// The text sizes: numbers, words beside them, and pills.
const VALUE_SIZE: f32 = 18.0;
const UNIT_SIZE: f32 = 13.0;
const PILL_SIZE: f32 = 13.0;
/// A toy's picture, and a pill's room round its words.
const TOY: f32 = 22.0;
const PILL_PAD: f32 = 9.0;
/// The saber's tier dot.
const DOT: f32 = 8.0;

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

/// The bar's laid-out storage, kept for the game's run.
pub(crate) struct State {
    canvas: MenuCanvas,
    /// What the bar shows, as last read, and when.
    inputs: Inputs,
    read: Option<Instant>,
    /// The bar was laid out this frame.
    drawn: bool,
    /// Text formatted to be measured.
    scratch: String,
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
            drawn: false,
            scratch: String::with_capacity(32),
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

    /// Whether what the bar shows is due to be read again at `now`.
    fn due(&self, now: Instant) -> bool {
        self.read
            .is_none_or(|at| now.saturating_duration_since(at) >= EVERY)
    }

    /// A piece's width in pixels at 1080 lines, `s` the layout scale.
    fn width(&mut self, piece: &Piece, measures: Measures<'_>, s: f32) -> f32 {
        let k = measures.body.scale().max(0.5);
        match *piece {
            Piece::Amount { value, unit } => {
                self.scratch.clear();
                let _ = write!(self.scratch, "{}", Grouped(value));
                let number =
                    measures
                        .display
                        .width(&self.scratch, VALUE_SIZE * s, TextFace::Semibold);
                let words = measures.body.width(unit, UNIT_SIZE * s, TextFace::Regular);
                (number + words) / s + 5.0 * k
            }
            Piece::Saber { name, .. } => {
                (DOT + 6.0) * k + measures.body.width(name, PILL_SIZE * s, TextFace::Semibold) / s
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

    /// Lay the bar out at `place` with `parts` of [`Self::inputs`]; `label` says whether
    /// the build label shows at the top. Whether anything was drawn.
    fn build(
        &mut self,
        place: Place,
        parts: Parts,
        label: bool,
        viewport: [f32; 2],
        measures: Measures<'_>,
    ) -> bool {
        let s = crate::menu::sjk::scale(viewport);
        let k = measures.body.scale().max(0.5);
        let mut list = [(
            Part::Credits,
            Piece::Toy {
                index: 0,
                on: false,
            },
        ); MAX_PIECES];
        let count = pieces(&self.inputs, parts, &mut list);
        self.canvas.begin_transparent(viewport);
        if count == 0 || place == Place::Off {
            self.canvas.finish(0);
            return false;
        }
        let mut widths = [0.0_f32; MAX_PIECES];
        let mut owners = [Part::Credits; MAX_PIECES];
        for (index, (part, piece)) in list[..count].iter().enumerate() {
            widths[index] = self.width(piece, measures, s);
            owners[index] = *part;
        }
        let room = viewport[0] / s - 2.0 * (PAD + MARGIN);
        let mut xs = [0.0_f32; MAX_PIECES];
        let (fitted, content) = fit(&owners[..count], &widths[..count], room, &mut xs);
        if fitted == 0 {
            self.canvas.finish(0);
            return false;
        }
        let height = HEIGHT * k.max(1.0);
        let width = content + PAD * 2.0;
        let left = (viewport[0] - width * s) * 0.5;
        let top = match place {
            Place::Bottom => viewport[1] - (MARGIN + height) * s,
            _ => (MARGIN + if label { UNDER_LABEL } else { 0.0 }) * s,
        };
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
        let line = |size: f32| Rect::new(x, middle - size * 0.75, width, size * 1.5);
        match piece {
            Piece::Amount { value, unit } => {
                let rect = line(VALUE_SIZE * s);
                text(
                    &mut self.canvas,
                    TextFamily::Display,
                    format_args!("{}", Grouped(value)),
                    rect,
                    VALUE_SIZE * s,
                    color::TEXT,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
                text(
                    &mut self.canvas,
                    TextFamily::Body,
                    format_args!("{unit}"),
                    rect,
                    UNIT_SIZE * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::End,
                );
            }
            Piece::Saber { name, colour } => {
                let dot = DOT * k * s;
                let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: Rect::new(x, middle - dot * 0.5, dot, dot),
                    radius: dot * 0.5,
                    color: colour,
                });
                let room = (DOT + 6.0) * k * s;
                text(
                    &mut self.canvas,
                    TextFamily::Body,
                    format_args!("{name}"),
                    Rect::new(
                        x + room,
                        middle - PILL_SIZE * s,
                        width - room,
                        PILL_SIZE * 2.0 * s,
                    ),
                    PILL_SIZE * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
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
                let crystals = me.map(|me| {
                    me.holocrons
                        .iter()
                        .filter(|holocron| {
                            holocron.opened.is_none()
                                && crate::kyber::from_id(&holocron.tier).is_some()
                        })
                        .count() as u32
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
            inputs.crystals = crystals;
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

    /// Lay the status bar out when `cg_statusBar` puts it somewhere and play shows
    /// (`shown`: no menu, console, scoreboard or intermission over it, the HUD's elements
    /// not hidden); whether it was drawn, so its shapes go into the frame's layers.
    pub(crate) fn append_status_bar(&mut self, viewport: [f32; 2], shown: bool) -> bool {
        let console = self.console.as_ref();
        let place = Place::of(console.and_then(|c| c.integer_cvar(CVAR)).unwrap_or(0));
        #[cfg(test)]
        let shown = shown || self.status_bar.preview.is_some();
        if place == Place::Off || !shown {
            self.status_bar.drawn = false;
            return false;
        }
        let parts = Parts::read(console);
        let label = console.is_none_or(|console| {
            console
                .bool_cvar(crate::version_overlay::CVAR)
                .unwrap_or(true)
        });
        let now = Instant::now();
        if self.status_bar.due(now) {
            self.status_bar.inputs = self.status_bar_inputs();
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
        let drawn = self
            .status_bar
            .build(place, parts, label, viewport, measures);
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
            unit: "credits",
        }
    }

    #[test]
    fn the_place_reads_off_top_and_bottom() {
        assert_eq!(Place::of(0), Place::Off);
        assert_eq!(Place::of(1), Place::Top);
        assert_eq!(Place::of(2), Place::Bottom);
        assert_eq!(Place::of(3), Place::Off);
        assert_eq!(Place::of(-1), Place::Off);
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
        assert!(Part::ALL[..6].iter().all(|part| !only_toys.has(*part)));
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
                    unit: "DOOM coin"
                }
            )
        );
        assert_eq!(
            out[2],
            (
                Part::Saber,
                Piece::Saber {
                    name: "Stock blade",
                    colour: color::MUTED
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
                colour: skin.tier.colour()
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

    #[test]
    fn tactical_says_its_level_only_while_on() {
        use crate::tactical::Level;
        assert_eq!(tactical_words(Level::Off), None);
        for level in [Level::Minimal, Level::Medium, Level::Max] {
            let words = tactical_words(level).unwrap();
            assert!(words.ends_with(level.name()), "{words}");
        }
    }

    #[test]
    fn a_full_bar_fits_its_storage() {
        let mut inputs = Inputs {
            credits: Some(u64::MAX),
            shards: Some(1),
            crystals: Some(3),
            coins: Some(4),
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
