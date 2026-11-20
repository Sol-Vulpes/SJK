//! What the bars are made of: each [`Piece`], how wide it is ([`width`]) and how it is
//! drawn ([`draw`]), in pixels at 1080 lines before the screen's scale `s` (window
//! pixels per such pixel) and the menu text size `k` (`ui_textScale`), as the SJK UI
//! measures. Numbers that change while they show (the frame rate, the ping, the time
//! left, the clock, the ammunition) keep at least the width of their widest digits, so
//! a bar does not shuffle its neighbours as they tick.

use super::feed;
use super::times::{Clock, Date, Left};
use crate::menu::sjk::{color, kit, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::sjk_chat_look::Measure;
use crate::text::TextFace;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign, TextureId};
use std::fmt::{self, Write as _};

/// Text sizes: numbers, words (and pills), and the small units after numbers.
const VALUE_SIZE: f32 = 15.0;
const WORD_SIZE: f32 = 12.5;
const UNIT_SIZE: f32 = 11.0;
/// An amount's icon's height, and the room between an icon and its words.
const ICON: f32 = 16.0;
const ICON_GAP: f32 = 5.0;
/// A toy's, a weapon's and a Force power's picture.
const TOY: f32 = 17.0;
const WEAPON: f32 = 19.0;
const POWER: f32 = 19.0;
/// A pill's room round its words, and its height over its words' size.
const PILL_PAD: f32 = 8.0;
const PILL_RISE: f32 = 7.0;
/// The New part's dot, a feed entry's dot.
const DOT: f32 = 7.0;
/// The kyber's tier diamonds' height ([`kit::tier_mark`]).
const DIAMOND: f32 = 8.0;
/// The room between a number and its unit.
const UNIT_GAP: f32 = 3.0;
/// Room past a measured word's end: a box exactly as wide as its text can round below
/// it and cut it to an ellipsis.
const SLACK: f32 = 8.0;
/// The widest the server's name grows before it ends in an ellipsis.
const SERVER_WIDTH: f32 = 250.0;
/// The DOOM coin's red, the WHEEL OF DOOM's (as the Inventory writes its coins).
const DOOM_RED: Color = crate::holocrons::rgb(0xE5_1A1A);
/// A good ping, and a fair one; past it the ping reads in ember.
const PING_GOOD: u16 = 80;
const PING_FAIR: u16 = 150;

/// What an amount counts, told by its icon.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Money {
    /// The credits' sign (`crate::cards::mark`), in gold.
    Credits,
    /// The kyber shard (`crate::cards::shard`), in kyber cyan.
    Shards,
    /// The dormant kyber of `crate::kyber::TIERS`' tier at this index: its picture.
    Dormant(usize),
    /// A DOOM coin: a red coin with the wheel's spokes struck on it ([`coin`]).
    Doom,
}

/// Something on about the player, as its pill says it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Effect {
    pub(crate) words: &'static str,
    pub(crate) colour: Color,
}

/// A toy the bottom bar shows: its name and its quick wheel picture's name (`None`: its
/// letters in a pill instead).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Toy {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) name: &'static str,
    pub(crate) icon: Option<&'static str>,
    pub(crate) letters: &'static str,
}

/// How many toys the bar shows.
pub(crate) const TOY_COUNT: usize = 15;

/// The toys with a switch, in the order of `toy_help` ([`crate::toy_switch_words::TOYS`])
/// but the Tag, which has none (`Inputs::toys` keeps this order).
pub(crate) const TOYS: [Toy; TOY_COUNT] = [
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

/// Index of each toy in [`TOYS`] and `Inputs::toys`.
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

/// One thing on a bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Piece {
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
    /// A weapon held, by its selection icon; the one in hand lit and underlined.
    Weapon { texture: TextureId, current: bool },
    /// The ammunition of the weapon in hand; `None` keeps its room empty (the weapon in
    /// hand takes none, another held does). `low`: five shots or fewer.
    Ammo { value: Option<u32>, low: bool },
    /// The local time.
    Time(Clock),
    /// The date.
    Date(Date),
    /// A Force power known, by its Force bar picture: the selected one lit and
    /// underlined, one in use marked with a dot.
    Power {
        texture: TextureId,
        selected: bool,
        active: bool,
    },
    /// The player's place and score (a game without teams): `place` from 1.
    Place { place: u32, tied: bool, score: i32 },
    /// The teams' scores, the player's own `team` (1 red, 2 blue) brighter.
    Teams { red: i32, blue: i32, team: u8 },
    /// The match time left.
    Left(Left),
    /// The feed's entry at this index ([`Texts::feed`]).
    Feed(u8),
    /// The server's name ([`Texts::server`]).
    Server,
    /// The map's name ([`Texts::map`]).
    Map,
    /// The game type's short name, in a pill.
    Mode(&'static str),
    /// The players on the server and its slots.
    Players { count: u8, slots: u16 },
    /// The connection's ping, milliseconds.
    Ping(u16),
    /// The frame rate.
    Fps(u32),
}

/// A blade skin's piece: its name, tier and tier colour, or the stock blade's.
pub(crate) fn saber_piece(skin: Option<&'static str>) -> Piece {
    match skin.and_then(crate::unlockables::blade_skin) {
        Some(unlockable) => Piece::Saber {
            name: unlockable.name,
            colour: unlockable.tier.colour(),
            tier: Some(unlockable.tier),
        },
        None => Piece::Saber {
            name: "Stock blade",
            colour: color::muted(),
            tier: None,
        },
    }
}

/// The words the pieces that name things read from.
#[derive(Clone, Copy)]
pub(crate) struct Texts<'a> {
    pub(crate) server: &'a str,
    pub(crate) map: &'a str,
    pub(crate) feed: &'a [Option<feed::Shown>; feed::SHOWN],
}

/// How text measures as the bars draw it: the display family's for numbers, the body's
/// for words, in window pixels.
#[derive(Clone, Copy)]
pub(crate) struct Measures<'a> {
    pub(crate) display: Measure<'a>,
    pub(crate) body: Measure<'a>,
}

impl Measures<'_> {
    /// A number's width as the bars write it (display family, semibold), at 1080 lines.
    fn number(&self, scratch: &mut String, value: fmt::Arguments<'_>, s: f32) -> f32 {
        scratch.clear();
        let _ = scratch.write_fmt(value);
        self.display
            .width(scratch, VALUE_SIZE * s, TextFace::Semibold)
            / s
    }

    /// Words' width in the body family at `size`, at 1080 lines.
    fn words(&self, words: &str, size: f32, face: TextFace, s: f32) -> f32 {
        self.body.width(words, size * s, face) / s
    }
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

/// A piece's width in pixels at 1080 lines, `s` the screen's scale.
pub(crate) fn width(
    piece: &Piece,
    texts: Texts<'_>,
    measures: Measures<'_>,
    scratch: &mut String,
    s: f32,
) -> f32 {
    let k = measures.body.scale().max(0.5);
    match *piece {
        Piece::Amount { value, money } => {
            icon_width(money, k)
                + ICON_GAP * k
                + measures.number(scratch, format_args!("{}", Grouped(value)), s)
        }
        Piece::Saber { name, tier, .. } => {
            let mark = tier.map_or(0.0, |tier| kit::tier_mark_width(tier, DIAMOND * k));
            let lead = if mark > 0.0 { mark + ICON_GAP * k } else { 0.0 };
            lead + measures.words(name, WORD_SIZE, TextFace::Semibold, s)
        }
        Piece::New { count, words } => {
            if !words {
                return DOT * k;
            }
            scratch.clear();
            let _ = write!(scratch, "{} new in your Collection", Grouped(count.into()));
            (DOT + ICON_GAP) * k + measures.words(scratch, WORD_SIZE, TextFace::Semibold, s)
        }
        Piece::Effect(Effect { words, .. }) | Piece::Mode(words) => {
            measures.words(words, WORD_SIZE, TextFace::Semibold, s) + PILL_PAD * 2.0 * k
        }
        Piece::Toy { .. } => TOY * k,
        Piece::Weapon { .. } => WEAPON * k,
        Piece::Power { .. } => POWER * k,
        Piece::Ammo { value, .. } => {
            let widest = measures.number(scratch, format_args!("000"), s);
            value.map_or(widest, |value| {
                widest.max(measures.number(scratch, format_args!("{value}"), s))
            })
        }
        Piece::Time(clock) => {
            let widest = measures.number(scratch, format_args!("00:00"), s);
            widest.max(measures.number(scratch, format_args!("{clock}"), s))
        }
        Piece::Date(date) => {
            scratch.clear();
            let _ = write!(scratch, "{date}");
            measures.words(scratch, WORD_SIZE, TextFace::Regular, s)
        }
        Piece::Place { place, tied, score } => {
            scratch.clear();
            let _ = write!(scratch, "{}", Ordinal { place, tied });
            let lead = measures
                .display
                .width(scratch, VALUE_SIZE * s, TextFace::Semibold)
                / s;
            scratch.clear();
            let _ = write!(scratch, "score {score}");
            lead + UNIT_GAP * 2.0 * k + measures.words(scratch, UNIT_SIZE, TextFace::Regular, s)
        }
        Piece::Teams { red, blue, .. } => {
            measures.words("Red", UNIT_SIZE, TextFace::Regular, s)
                + measures.words("Blue", UNIT_SIZE, TextFace::Regular, s)
                + measures.number(scratch, format_args!("{red}"), s)
                + measures.number(scratch, format_args!("{blue}"), s)
                + (UNIT_GAP * 2.0 + ICON_GAP * 2.0) * k
        }
        Piece::Left(left) => {
            let widest = if left.0 >= 3600 { "0:00:00" } else { "00:00" };
            let number = measures
                .number(scratch, format_args!("{widest}"), s)
                .max(measures.number(scratch, format_args!("{left}"), s));
            ICON * k
                + ICON_GAP * k
                + number
                + UNIT_GAP * k
                + measures.words("left", UNIT_SIZE, TextFace::Regular, s)
        }
        Piece::Feed(index) => {
            let Some(shown) = texts.feed.get(usize::from(index)).copied().flatten() else {
                return 0.0;
            };
            (DOT + ICON_GAP) * k
                + measures.words(shown.name.as_str(), WORD_SIZE, TextFace::Semibold, s)
                + UNIT_GAP * 2.0 * k
                + measures.words(feed_word(shown.joined), UNIT_SIZE, TextFace::Regular, s)
        }
        Piece::Server => measures
            .words(texts.server, WORD_SIZE, TextFace::Regular, s)
            .min(SERVER_WIDTH * k),
        Piece::Map => {
            measures
                .display
                .width(texts.map, VALUE_SIZE * s, TextFace::Semibold)
                / s
        }
        Piece::Players { count, slots } => {
            let widest = measures.number(scratch, format_args!("00"), s);
            ICON * 0.8 * k
                + ICON_GAP * k
                + widest.max(measures.number(scratch, format_args!("{count}"), s))
                + measures.words(slash_slots(scratch, slots), UNIT_SIZE, TextFace::Regular, s)
        }
        Piece::Ping(ping) => {
            let widest = measures.number(scratch, format_args!("000"), s);
            widest.max(measures.number(scratch, format_args!("{ping}"), s))
                + UNIT_GAP * k
                + measures.words("ms", UNIT_SIZE, TextFace::Regular, s)
        }
        Piece::Fps(fps) => {
            let widest = measures.number(scratch, format_args!("000"), s);
            widest.max(measures.number(scratch, format_args!("{fps}"), s))
                + UNIT_GAP * k
                + measures.words("fps", UNIT_SIZE, TextFace::Regular, s)
        }
    }
}

/// What a feed entry says after the name.
fn feed_word(joined: bool) -> &'static str {
    if joined { "joined" } else { "left" }
}

/// `/slots` written into `scratch`.
fn slash_slots(scratch: &mut String, slots: u16) -> &str {
    scratch.clear();
    let _ = write!(scratch, "/{slots}");
    scratch
}

/// A place in the match ("1st", "Tied 2nd"), `place` counted from 1.
struct Ordinal {
    place: u32,
    tied: bool,
}

impl fmt::Display for Ordinal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match (self.place % 100, self.place % 10) {
            (11..=13, _) => "th",
            (_, 1) => "st",
            (_, 2) => "nd",
            (_, 3) => "rd",
            _ => "th",
        };
        if self.tied {
            f.write_str("Tied ")?;
        }
        write!(f, "{}{suffix}", self.place)
    }
}

/// `value` with its digits in groups of three, apart by a space ("12 340"), as the
/// Collection writes credits ([`crate::cards::grouped`]), without allocating.
pub(crate) struct Grouped(pub(crate) u64);

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

/// A DOOM coin `side` window pixels across, its top left at `at`: a red disc with a
/// darker rim, the WHEEL OF DOOM's spokes struck across it and a glint. Shapes only, so
/// it is sharp at every size and reads as a coin without its colour.
pub(crate) fn coin(list: &mut sjk_ui::DrawList, [x, y]: [f32; 2], side: f32) {
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

/// A small clock face `side` across, centred on `centre`: the time left's mark.
fn clock_face(list: &mut sjk_ui::DrawList, [x, y]: [f32; 2], side: f32, colour: Color) {
    let line = (side * 0.1).max(1.0);
    let _ = list.push(DrawCommand::Border {
        rect: Rect::new(x - side * 0.5, y - side * 0.5, side, side),
        radius: side * 0.5,
        width: line,
        color: colour,
    });
    let _ = list.push(DrawCommand::Line {
        from: [x, y],
        to: [x, y - side * 0.3],
        width: line,
        color: colour,
    });
    let _ = list.push(DrawCommand::Line {
        from: [x, y],
        to: [x + side * 0.22, y + side * 0.08],
        width: line,
        color: colour,
    });
}

/// A person's outline `side` tall, its middle at `centre`: the player count's mark.
fn person(list: &mut sjk_ui::DrawList, [x, y]: [f32; 2], side: f32, colour: Color) {
    let head = side * 0.42;
    let _ = list.push(DrawCommand::RoundedRect {
        rect: Rect::new(x - head * 0.5, y - side * 0.5, head, head),
        radius: head * 0.5,
        color: colour,
    });
    let body = side * 0.78;
    let shoulders = side * 0.44;
    let _ = list.push(DrawCommand::RoundedRect {
        rect: Rect::new(x - body * 0.5, y + side * 0.5 - shoulders, body, shoulders),
        radius: shoulders * 0.5,
        color: colour,
    });
}

/// Where the pieces draw: the canvas, the bar's middle line, the screen's scale `s` and
/// the text size `k`.
pub(crate) struct Pen<'a> {
    pub(crate) canvas: &'a mut MenuCanvas,
    pub(crate) middle: f32,
    pub(crate) s: f32,
    pub(crate) k: f32,
}

impl Pen<'_> {
    /// One run of text from `x`, `width` wide (window pixels), centred on the middle line
    /// moved down by `drop`.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        family: TextFamily,
        value: fmt::Arguments<'_>,
        x: f32,
        width: f32,
        size: f32,
        colour: Color,
        weight: FontWeight,
        drop: f32,
    ) {
        let size = size * self.s;
        let middle = self.middle + drop;
        text(
            self.canvas,
            family,
            value,
            Rect::new(x, middle - size, width, size * 2.0),
            size,
            colour,
            weight,
            TextAlign::Start,
        );
    }

    /// A number in the display family from `x`.
    fn number(&mut self, value: fmt::Arguments<'_>, x: f32, width: f32, colour: Color) {
        self.run(
            TextFamily::Display,
            value,
            x,
            width + SLACK * self.s,
            VALUE_SIZE,
            colour,
            FontWeight::Semibold,
            0.0,
        );
    }

    /// A small unit or label word from `x`, sat on the numbers' line.
    fn unit(&mut self, words: fmt::Arguments<'_>, x: f32, colour: Color) {
        let drop = (VALUE_SIZE - UNIT_SIZE) * 0.28 * self.s * self.k;
        self.run(
            TextFamily::Body,
            words,
            x,
            200.0 * self.s,
            UNIT_SIZE,
            colour,
            FontWeight::Regular,
            drop,
        );
    }

    /// The window width of a number as [`Measures::number`] measures it.
    fn measured(
        &self,
        measures: Measures<'_>,
        scratch: &mut String,
        value: fmt::Arguments<'_>,
    ) -> f32 {
        measures.number(scratch, value, self.s) * self.s
    }

    /// A pill round `words` in `colour`, `width` wide from `x`.
    fn pill(&mut self, words: &str, colour: Color, x: f32, width: f32) {
        let height = (WORD_SIZE + PILL_RISE) * self.k * self.s;
        let pill = Rect::new(x, self.middle - height * 0.5, width, height);
        let list = self.canvas.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect: pill,
            radius: height * 0.5,
            color: color::alpha(colour, 0.14),
        });
        let _ = list.push(DrawCommand::Border {
            rect: pill,
            radius: height * 0.5,
            width: 1.0_f32.max(self.s),
            color: color::alpha(colour, 0.5),
        });
        text(
            self.canvas,
            TextFamily::Body,
            format_args!("{words}"),
            pill,
            WORD_SIZE * self.s,
            colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
    }

    /// A square picture `side` across from `x`, lit (`on`: underlined in gold on a holo
    /// ground) or faint; `dot` marks it in use.
    fn picture(&mut self, texture: TextureId, x: f32, side: f32, on: bool, dot: bool) {
        let s = self.s;
        let top = self.middle - side * 0.5 - 1.5 * s;
        let rect = Rect::new(x, top, side, side);
        let list = self.canvas.draw_list_mut();
        if on {
            let ground = Rect::new(
                rect.x - 2.0 * s,
                rect.y - 1.0 * s,
                rect.width + 4.0 * s,
                rect.height + 2.0 * s,
            );
            let _ = list.push(DrawCommand::RoundedRect {
                rect: ground,
                radius: 3.0 * s,
                color: color::alpha(color::holo(), 0.16),
            });
            let _ = list.push(DrawCommand::SolidRect {
                rect: Rect::new(
                    ground.x + 2.0 * s,
                    ground.bottom() + 0.5 * s,
                    ground.width - 4.0 * s,
                    2.0_f32.max(1.5 * s),
                ),
                color: color::gold_bright(),
            });
        }
        let _ = list.push(DrawCommand::TexturedQuad {
            rect,
            texture,
            color: Color::new(1.0, 1.0, 1.0, if on || dot { 1.0 } else { 0.5 }),
        });
        if dot {
            let side = 4.0 * s;
            let _ = list.push(DrawCommand::RoundedRect {
                rect: Rect::new(rect.right() - side * 0.6, rect.y - side * 0.4, side, side),
                radius: side * 0.5,
                color: color::holo(),
            });
        }
    }
}

/// Draw `piece` from `x` (window pixels), `width` wide.
pub(crate) fn draw(
    pen: &mut Pen<'_>,
    piece: Piece,
    texts: Texts<'_>,
    measures: Measures<'_>,
    scratch: &mut String,
    x: f32,
    width: f32,
) {
    let (s, k, middle) = (pen.s, pen.k, pen.middle);
    match piece {
        Piece::Amount { value, money } => {
            let icon = ICON * k * s;
            let at = [x, middle - icon * 0.5];
            let list = pen.canvas.draw_list_mut();
            match money {
                Money::Credits => {
                    let _ = crate::cards::mark::draw(list, at, icon, color::gold_bright());
                }
                Money::Shards => {
                    let _ = crate::cards::shard::draw(list, at, icon, crate::cards::shard::KYBER);
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
            pen.number(
                format_args!("{}", Grouped(value)),
                x + lead,
                width - lead,
                color::text(),
            );
        }
        Piece::Saber { name, colour, tier } => {
            let mark = tier.map_or(0.0, |tier| {
                kit::tier_mark(
                    pen.canvas,
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
            let room = if mark > 0.0 {
                mark + ICON_GAP * k * s
            } else {
                0.0
            };
            pen.run(
                TextFamily::Body,
                format_args!("{name}"),
                x + room,
                width - room + SLACK * s,
                WORD_SIZE,
                colour,
                FontWeight::Semibold,
                0.0,
            );
        }
        Piece::New { count, words } => {
            let dot = DOT * k * s;
            let _ = pen.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(x, middle - dot * 0.5, dot, dot),
                radius: dot * 0.5,
                color: color::gold_bright(),
            });
            if words {
                let room = (DOT + ICON_GAP) * k * s;
                pen.run(
                    TextFamily::Body,
                    format_args!("{} new in your Collection", Grouped(count.into())),
                    x + room,
                    width - room + SLACK * s,
                    WORD_SIZE,
                    color::gold_bright(),
                    FontWeight::Semibold,
                    0.0,
                );
            }
        }
        Piece::Effect(effect) => pen.pill(effect.words, effect.colour, x, width),
        Piece::Mode(words) => pen.pill(words, color::holo(), x, width),
        Piece::Toy { index, on } => {
            let side = TOY * k * s;
            let rect = Rect::new(x, middle - side * 0.5 - 1.5 * s, side, side);
            let toy = TOYS[index];
            let alpha = if on { 1.0 } else { 0.3 };
            match toy.icon.and_then(crate::quick_wheel::catalog::icon_index) {
                Some(icon) => {
                    let _ = pen.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                        rect,
                        texture: crate::ui_renderer::wheel_icon(icon),
                        color: Color::new(1.0, 1.0, 1.0, alpha),
                    });
                }
                None => {
                    let _ = pen.canvas.draw_list_mut().push(DrawCommand::Border {
                        rect,
                        radius: side * 0.3,
                        width: 1.0_f32.max(s),
                        color: color::alpha(color::holo(), alpha * 0.8),
                    });
                    text(
                        pen.canvas,
                        TextFamily::Display,
                        format_args!("{}", toy.letters),
                        rect,
                        side * 0.5,
                        color::alpha(color::text(), alpha),
                        FontWeight::Semibold,
                        TextAlign::Center,
                    );
                }
            }
            if on {
                let dot = 3.5 * s;
                let _ = pen.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: Rect::new(x + (side - dot) * 0.5, rect.bottom() + 1.5 * s, dot, dot),
                    radius: dot * 0.5,
                    color: color::gold_bright(),
                });
            }
        }
        Piece::Weapon { texture, current } => {
            pen.picture(texture, x, WEAPON * k * s, current, false);
        }
        Piece::Power {
            texture,
            selected,
            active,
        } => pen.picture(texture, x, POWER * k * s, selected, active && !selected),
        Piece::Ammo { value, low } => {
            if let Some(value) = value {
                let colour = if low { color::ember() } else { color::text() };
                pen.number(format_args!("{value}"), x, width, colour);
            }
        }
        Piece::Time(clock) => pen.number(format_args!("{clock}"), x, width, color::text()),
        Piece::Date(date) => pen.run(
            TextFamily::Body,
            format_args!("{date}"),
            x,
            width + SLACK * s,
            WORD_SIZE,
            color::muted(),
            FontWeight::Regular,
            0.0,
        ),
        Piece::Place { place, tied, score } => {
            let colour = if place == 1 {
                color::gold_bright()
            } else {
                color::text()
            };
            let ordinal = Ordinal { place, tied };
            pen.number(format_args!("{ordinal}"), x, width, colour);
            let lead = pen.measured(measures, scratch, format_args!("{ordinal}"));
            pen.unit(
                format_args!("score {score}"),
                x + lead + UNIT_GAP * 2.0 * k * s,
                color::muted(),
            );
        }
        Piece::Teams { red, blue, team } => {
            let mut at = x;
            for (label, value, colour, own) in [
                ("Red", red, color::RED_TEAM, team == 1),
                ("Blue", blue, color::BLUE_TEAM, team == 2),
            ] {
                pen.unit(format_args!("{label}"), at, colour);
                at += measures.words(label, UNIT_SIZE, TextFace::Regular, s) * s + UNIT_GAP * k * s;
                let shade = if own { color::text() } else { color::muted() };
                pen.number(format_args!("{value}"), at, 100.0 * s, shade);
                at += pen.measured(measures, scratch, format_args!("{value}"))
                    + ICON_GAP * 2.0 * k * s;
            }
        }
        Piece::Left(left) => {
            let side = ICON * 0.85 * k * s;
            let colour = if left.closing() {
                color::ember()
            } else {
                color::holo()
            };
            clock_face(
                pen.canvas.draw_list_mut(),
                [x + side * 0.5, middle],
                side,
                colour,
            );
            let lead = (ICON + ICON_GAP) * k * s;
            let shade = if left.closing() {
                color::ember()
            } else {
                color::text()
            };
            pen.number(format_args!("{left}"), x + lead, width - lead, shade);
            let widest = if left.0 >= 3600 { "0:00:00" } else { "00:00" };
            let number = pen
                .measured(measures, scratch, format_args!("{widest}"))
                .max(pen.measured(measures, scratch, format_args!("{left}")));
            pen.unit(
                format_args!("left"),
                x + lead + number + UNIT_GAP * k * s,
                color::muted(),
            );
        }
        Piece::Feed(index) => {
            let Some(shown) = texts.feed.get(usize::from(index)).copied().flatten() else {
                return;
            };
            let _ = pen
                .canvas
                .draw_list_mut()
                .push(DrawCommand::PushOpacity(shown.alpha()));
            let dot = DOT * k * s;
            let _ = pen.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(x, middle - dot * 0.5, dot, dot),
                radius: dot * 0.5,
                color: if shown.joined {
                    color::TRUSTED
                } else {
                    color::ember()
                },
            });
            let lead = (DOT + ICON_GAP) * k * s;
            let name = measures.words(shown.name.as_str(), WORD_SIZE, TextFace::Semibold, s) * s;
            pen.run(
                TextFamily::Body,
                format_args!("{}", shown.name.as_str()),
                x + lead,
                name + SLACK * s,
                WORD_SIZE,
                color::text(),
                FontWeight::Semibold,
                0.0,
            );
            pen.unit(
                format_args!("{}", feed_word(shown.joined)),
                x + lead + name + UNIT_GAP * 2.0 * k * s,
                color::muted(),
            );
            let _ = pen.canvas.draw_list_mut().push(DrawCommand::PopOpacity);
        }
        Piece::Server => {
            // Room past the words when they fit; none when they were cut to the widest.
            let cut = width >= SERVER_WIDTH * k * s - 0.5;
            pen.run(
                TextFamily::Body,
                format_args!("{}", texts.server),
                x,
                if cut { width } else { width + SLACK * s },
                WORD_SIZE,
                color::muted(),
                FontWeight::Regular,
                0.0,
            );
        }
        Piece::Map => pen.number(format_args!("{}", texts.map), x, width, color::text()),
        Piece::Players { count, slots } => {
            let side = ICON * 0.8 * k * s;
            person(
                pen.canvas.draw_list_mut(),
                [x + side * 0.5, middle],
                side,
                color::muted(),
            );
            let lead = side + ICON_GAP * k * s;
            pen.number(format_args!("{count}"), x + lead, 60.0 * s, color::text());
            let widest = pen
                .measured(measures, scratch, format_args!("00"))
                .max(pen.measured(measures, scratch, format_args!("{count}")));
            pen.unit(format_args!("/{slots}"), x + lead + widest, color::muted());
        }
        Piece::Ping(ping) => {
            let colour = if ping < PING_GOOD {
                color::TRUSTED
            } else if ping < PING_FAIR {
                color::gold_bright()
            } else {
                color::ember()
            };
            pen.number(format_args!("{ping}"), x, width, colour);
            let widest = pen
                .measured(measures, scratch, format_args!("000"))
                .max(pen.measured(measures, scratch, format_args!("{ping}")));
            pen.unit(
                format_args!("ms"),
                x + widest + UNIT_GAP * k * s,
                color::muted(),
            );
        }
        Piece::Fps(fps) => {
            pen.number(format_args!("{fps}"), x, width, color::text());
            let widest = pen
                .measured(measures, scratch, format_args!("000"))
                .max(pen.measured(measures, scratch, format_args!("{fps}")));
            pen.unit(
                format_args!("fps"),
                x + widest + UNIT_GAP * k * s,
                color::muted(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn places_read_as_ordinals() {
        for (place, tied, words) in [
            (1, false, "1st"),
            (2, false, "2nd"),
            (3, true, "Tied 3rd"),
            (4, false, "4th"),
            (11, false, "11th"),
            (12, false, "12th"),
            (13, false, "13th"),
            (21, false, "21st"),
            (22, true, "Tied 22nd"),
        ] {
            assert_eq!(Ordinal { place, tied }.to_string(), words);
        }
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
            assert!(width > 4.0 && width <= ICON * 1.15, "{money:?}: {width}");
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
}
