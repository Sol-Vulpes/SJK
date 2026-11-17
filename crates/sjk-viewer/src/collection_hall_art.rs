//! The hall's tiles' pictures (15/11/2026, Sol's review of the hall: most tiles were two
//! thirds empty): under each tile's name, what the kind holds drawn with the pictures the
//! client already has rather than a line of words. Shaders: the blades worn, new and held
//! as live strips, and each tier's share; Finishers: the one worn in its tier's light;
//! Tags: the tag chosen in its picture and finish; Toys: every toy's picture, lit while on;
//! Medals: the medals in their ranks, the ones not given dim; Achievements: the next one's
//! progress as a large figure; Cards: the cards held, or an empty binder; Holocrons: each
//! tier's picture and count; Forge: the deal of the day's blade and its price; Inventory:
//! the credits and kyber shards as large figures; History: the newest three lines.
//!
//! [`Art`] is made with the tile's summary (four times a second, from data the client
//! already holds) and drawn every frame into the room the tile leaves under its name,
//! fitting its size: pictures grow with a taller tile and rows that do not fit are left
//! out, never squeezed.

use super::super::view::label_width;
use super::*;
use crate::menu::sjk::kit;
use crate::unlockables::Rarity;

/// The most a toy's or medal's picture grows to, and a holocron's (hall frame pixels).
const ICON_MOST: f32 = 64.0;
const MEDAL_MOST: f32 = 96.0;
const HOLOCRON_MOST: f32 = 96.0;
/// A grid of medals takes the fewest rows whose pictures are this share of the largest; the
/// toys, fourteen small pictures, nearly the largest.
const MEDAL_SHARE: f32 = 0.55;
const TOY_SHARE: f32 = 0.95;
/// A blade's strip: its height and the most it runs across.
const STRIP_HEIGHT: f32 = 20.0;
const STRIP_MOST: f32 = 240.0;

/// A blade a tile shows: its id (`None`, the stock blade), its name, its colour and what
/// it is to the player ("worn", "new", nothing for one held).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Blade {
    pub(crate) id: Option<&'static str>,
    pub(crate) name: String,
    pub(crate) colour: Color,
    /// Its tier, for its mark (`None`: the stock blade).
    pub(crate) tier: Option<Rarity>,
    pub(crate) note: &'static str,
}

/// The deal of the day: its blade, its price and whether the shop sells in kyber shards.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Deal {
    pub(crate) blade: Blade,
    pub(crate) price: u64,
    pub(crate) shards: bool,
}

/// What a tile draws under its name; [`Art::Lines`] only its lines.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Art {
    #[default]
    Lines,
    /// The blades (worn first, then the new ones, then others held) and each tier's
    /// held of its total (`None` held while the holdings are unknown).
    Shaders {
        blades: Vec<Blade>,
        tiers: Vec<(Rarity, Option<usize>, usize)>,
    },
    /// The finisher worn (`None`: none).
    Finisher {
        worn: Option<(&'static str, Rarity, &'static str)>,
    },
    /// The tag chosen: its id, name, tier and finish's name (`None` for the plain one).
    Tag {
        chosen: Option<(&'static str, &'static str, Rarity, Option<&'static str>)>,
    },
    /// Each toy on or off, in the Toys tab's order of its pictures.
    Toys { on: Vec<(usize, bool)> },
    /// The medals, the ones given first with their award.
    Medals {
        shown: Vec<(crate::medals::Medal, Option<crate::medals::Award>)>,
    },
    /// The open achievement nearest to unlocking: its name, progress and goal.
    Achievement { next: Option<(String, u64, u64)> },
    /// The cards held: each face's picture (`Some` index) or a sealed copy's colour, and
    /// how many copies in all (`None` while unknown).
    Cards {
        shown: Vec<Result<usize, Color>>,
        held: Option<usize>,
    },
    /// The holocrons to open, by tier (Uncommon to Mythical).
    Holocrons { counts: Option<[u32; 4]> },
    /// The deal of the day once the shop was read, the day's other offers (a blade's
    /// note "owned" or "bought" in place of its price), and the duplicates to break down.
    Forge {
        deal: Option<Deal>,
        offers: Vec<Deal>,
        duplicates: Option<u32>,
    },
    /// The credits and kyber shards, and the rest held in words.
    Inventory {
        money: Option<(u64, u64)>,
        rest: String,
    },
    /// The newest lines read: each its day and sentence.
    History { rows: Vec<(String, String)> },
}

/// How large a tier's mark is beside words `size` high (`art_mark`).
fn mark_size(size: f32) -> f32 {
    (size * 0.6).max(8.0)
}

/// `words` cut to `width` frame pixels at `size` (the body family when `body`), ending in
/// "..." when cut.
fn clipped(words: &str, width: f32, size: f32, body: bool) -> String {
    let wide = |words: &str| label_width(words, size, if body { 1.15 } else { 1.0 });
    if wide(words) <= width {
        return words.to_owned();
    }
    let mut kept: String = words.to_owned();
    while !kept.is_empty() && wide(&format!("{kept}...")) > width {
        kept.pop();
    }
    format!("{}...", kept.trim_end())
}

/// The cells of a grid of `count` pictures in a box `width` by `height` with `gap` between:
/// the columns and the side of the pictures, at most `most`. The fewest rows whose
/// pictures are at least `share` of the largest possible: a short last row of one medal
/// under three reads as left over.
fn grid(count: usize, width: f32, height: f32, gap: f32, [most, share]: [f32; 2]) -> (usize, f32) {
    let count = count.max(1);
    let options: Vec<(usize, f32)> = (1..=count)
        .map(|columns| {
            let rows = count.div_ceil(columns);
            let across = (width - gap * (columns - 1) as f32) / columns as f32;
            let down = (height - gap * (rows - 1) as f32) / rows as f32;
            (columns, across.min(down).min(most))
        })
        .collect();
    let best = options.iter().map(|option| option.1).fold(0.0, f32::max);
    options
        .iter()
        .rev()
        .find(|option| option.1 >= best * share)
        .copied()
        .unwrap_or((count, best.max(1.0)))
}

impl Panel {
    /// Tile art `art` in its room `[x, y, width, height]` (hall frame pixels) at `seconds`,
    /// the menu text drawn `ts` times its size.
    pub(super) fn tile_art(
        &mut self,
        frame: &Frame,
        art: &Art,
        room: [f32; 4],
        ts: f32,
        seconds: f32,
        stock: Color,
    ) {
        match art {
            Art::Lines => {}
            Art::Shaders { blades, tiers } => {
                self.art_shaders(frame, blades, tiers, room, ts, stock)
            }
            Art::Finisher { worn } => self.art_finisher(frame, *worn, room, ts),
            Art::Tag { chosen } => self.art_tag(frame, *chosen, room, ts),
            Art::Toys { on } => self.art_toys(frame, on, room, ts),
            Art::Medals { shown } => self.art_medals(frame, shown, room, ts, seconds),
            Art::Achievement { next } => self.art_achievement(frame, next.as_ref(), room, ts),
            Art::Cards { shown, held } => self.art_cards(frame, shown, *held, room, ts),
            Art::Holocrons { counts } => self.art_holocrons(frame, *counts, room, ts),
            Art::Forge {
                deal,
                offers,
                duplicates,
            } => {
                self.art_forge(frame, deal.as_ref(), offers, *duplicates, room, ts, stock);
            }
            Art::Inventory { money, rest } => self.art_inventory(frame, *money, rest, room, ts),
            Art::History { rows } => self.art_history(frame, rows, room, ts),
        }
    }

    /// One line of words at `[x, y]`, `width` across, cut to fit.
    #[allow(clippy::too_many_arguments)]
    fn art_line(
        &mut self,
        frame: &Frame,
        words: &str,
        [x, y, width]: [f32; 3],
        size: f32,
        colour: Color,
        display: bool,
        ts: f32,
    ) {
        // Through `terms::say`, so JoF naming reads Echoes and Honors here too.
        let words = crate::terms::say(words);
        let words = clipped(&words, width, size, !display);
        text(
            &mut self.ui,
            if display {
                TextFamily::Display
            } else {
                TextFamily::Body
            },
            format_args!("{words}"),
            frame.rect(x, y, width, size * 1.4 * ts),
            size * frame.s,
            colour,
            if display {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
            TextAlign::Start,
        );
    }

    /// Tier `tier`'s mark at the start of the line from `[x, y]`, `width` across (`size`
    /// its words'), as every page tells a tier ([`kit::tier_mark`], `docs/sjk-ui.md`
    /// "Tier frame"): the room left after it for the words. Nothing for Common.
    fn art_mark(
        &mut self,
        frame: &Frame,
        tier: Rarity,
        [x, y, width]: [f32; 3],
        size: f32,
        ts: f32,
    ) -> [f32; 3] {
        let mark = mark_size(size);
        let middle = y + size * 1.4 * ts * 0.5;
        let drawn = kit::tier_mark(&mut self.ui, frame, [x, middle], tier, mark, false);
        if drawn <= 0.0 {
            return [x, y, width];
        }
        let used = drawn + size * 0.45;
        [x + used, y, (width - used).max(0.0)]
    }

    /// A thin bar at `[x, y, width]`, `share` of it filled in `colour`.
    fn art_bar(&mut self, frame: &Frame, [x, y, width]: [f32; 3], share: f32, colour: Color) {
        let s = frame.s;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, 4.0),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.16),
        });
        if share > 0.0 {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x, y, (width * share.min(1.0)).max(4.0), 4.0),
                radius: 2.0 * s,
                color: colour,
            });
        }
    }

    /// A blade's strip over `rect`: the stock blade in the player's colour, a skin live.
    fn art_strip(&mut self, frame: &Frame, rect: [f32; 4], id: Option<&str>, stock: Color) {
        match id {
            None => super::super::swatch::stock_strip(self.ui.draw_list_mut(), frame, rect, stock),
            Some(id) => {
                let look = self.skins.get(id);
                let _ = super::super::swatch::strip(
                    self.ui.draw_list_mut(),
                    frame,
                    rect,
                    look,
                    true,
                    super::super::swatch::chroma_turn(look, super::super::swatch::rgb_bytes(stock)),
                );
            }
        }
    }

    /// Shaders: the blades as strips with their names down the left, each tier's share
    /// down the right.
    fn art_shaders(
        &mut self,
        frame: &Frame,
        blades: &[Blade],
        tiers: &[(Rarity, Option<usize>, usize)],
        [x, y, width, height]: [f32; 4],
        ts: f32,
        stock: Color,
    ) {
        let split = 26.0;
        let left = (width - split) * 0.56;
        let right_x = x + left + split;
        let right = width - left - split;
        // The blades: a strip and its name a row.
        let row = (STRIP_HEIGHT + 10.0).max(17.0 * 1.4 * ts + 6.0);
        let strip = (left * 0.46).min(STRIP_MOST);
        let rows = ((height + 6.0) / row).floor().max(0.0) as usize;
        for (index, blade) in blades.iter().take(rows).enumerate() {
            let top = y + index as f32 * row;
            let middle = top + (row - 6.0) * 0.5;
            self.art_strip(
                frame,
                [x, middle - STRIP_HEIGHT * 0.5, strip, STRIP_HEIGHT],
                blade.id,
                stock,
            );
            let size = 16.0;
            let line = size * 1.4 * ts;
            // The blade's tier told by its mark after the strip, as on every page.
            let words_x = match blade.tier {
                Some(tier) => {
                    let start = [x + strip + 12.0, middle - line * 0.5, left];
                    self.art_mark(frame, tier, start, 13.0, ts)[0].max(x + strip + 14.0)
                }
                None => x + strip + 14.0,
            };
            let note = if blade.note.is_empty() {
                String::new()
            } else {
                format!(" \u{b7} {}", blade.note)
            };
            let note_width = if note.is_empty() {
                0.0
            } else {
                label_width(&note, 14.0, 1.15)
            };
            let name_room = (x + left - words_x - note_width).max(20.0);
            let name = clipped(&blade.name, name_room, size, true);
            let name_width = label_width(&name, size, 1.15).min(name_room);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{name}"),
                frame.rect(words_x, middle - line * 0.5, name_room, line),
                size * frame.s,
                blade.colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if !note.is_empty() {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{note}"),
                    frame.rect(
                        words_x + name_width + 3.0,
                        middle - line * 0.5 + 1.0,
                        note_width + 10.0,
                        line,
                    ),
                    14.0 * frame.s,
                    if blade.note == "new" {
                        color::GOLD
                    } else {
                        color::QUIET
                    },
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
        }
        // Each tier: its name in its colour, held of its total and a bar of that share.
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + left + split * 0.5, y, 1.0, height.max(0.0)),
            radius: 0.0,
            color: color::alpha(color::HOLO, 0.14),
        });
        let size = 14.0;
        let line = size * 1.35 * ts;
        let step = line + 4.0 + 6.0;
        // Each bar under its name, or, when the tiers do not all fit so, beside it on the
        // same line.
        let roomy = ((height + 6.0) / step).floor() as usize >= tiers.len();
        let step = if roomy {
            (height / tiers.len().max(1) as f32).min(step + 8.0)
        } else {
            line
        };
        let fit = (((height + 2.0) / step).floor().max(0.0) as usize).min(tiers.len());
        let mark_room = |tier: Rarity| {
            let width = kit::tier_mark_width(tier, mark_size(size));
            if width > 0.0 {
                width + size * 0.45
            } else {
                0.0
            }
        };
        let label_room = if roomy {
            right * 0.62
        } else {
            tiers
                .iter()
                .map(|(tier, _, _)| mark_room(*tier) + label_width(tier.label(), size, 1.0))
                .fold(0.0, f32::max)
                + 10.0
        };
        let count_room = label_width("000/000", size, 1.0) + 10.0;
        for (index, (tier, held, total)) in tiers.iter().take(fit).enumerate() {
            let top = y + index as f32 * step;
            let words = self.art_mark(frame, *tier, [right_x, top, label_room], size, ts);
            self.art_line(frame, tier.label(), words, size, tier.colour(), true, ts);
            let words = match held {
                Some(held) => format!("{held}/{total}"),
                None => total.to_string(),
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(right_x, top, right, line),
                size * frame.s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::End,
            );
            let share = match held {
                Some(held) if *total > 0 => *held as f32 / *total as f32,
                _ => 0.0,
            };
            if roomy {
                self.art_bar(
                    frame,
                    [right_x, top + line + 2.0, right],
                    share,
                    tier.colour(),
                );
            } else {
                let bar_width = right - label_room - count_room;
                if bar_width > 12.0 {
                    let bar = [right_x + label_room, top + line * 0.5 - 2.0, bar_width];
                    self.art_bar(frame, bar, share, tier.colour());
                }
            }
        }
    }

    /// Finishers: the one worn named large in its tier's colour, what it does under it,
    /// its tier's mark glowing at the right.
    fn art_finisher(
        &mut self,
        frame: &Frame,
        worn: Option<(&'static str, Rarity, &'static str)>,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let Some((name, tier, about)) = worn else {
            self.art_line(
                frame,
                "None worn",
                [x, y, width],
                18.0,
                color::MUTED,
                true,
                ts,
            );
            self.art_paragraph(
                frame,
                "What a saber kill does to the body: a finisher is worn with any blade.",
                [x, y + 30.0 * ts, width, height - 30.0 * ts],
                ts,
            );
            return;
        };
        let mark = (height * 0.62).clamp(36.0, 84.0);
        let mark_x = x + width - mark * 0.5 - 4.0;
        let mark_y = y + mark * 0.5 + 4.0;
        super::super::view::glow(
            &mut self.ui,
            frame,
            [mark_x, mark_y],
            mark * 0.75,
            tier.colour(),
            0.35,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", tier.symbol()),
            frame.rect(
                mark_x - mark * 0.6,
                mark_y - mark * 0.55,
                mark * 1.2,
                mark * 1.1,
            ),
            mark * 0.78 * frame.s / ts.max(0.1),
            tier.colour(),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let words_width = width - mark - 14.0;
        self.art_line(
            frame,
            name,
            [x, y, words_width],
            22.0,
            tier.colour(),
            true,
            ts,
        );
        let below = y + 22.0 * 1.4 * ts;
        let words = self.art_mark(frame, tier, [x, below, words_width], 14.0, ts);
        self.art_line(
            frame,
            &format!("{} \u{b7} worn", tier.label()),
            words,
            14.0,
            color::MUTED,
            false,
            ts,
        );
        let rest = below + 14.0 * 1.4 * ts + 8.0;
        self.art_paragraph(frame, about, [x, rest, words_width, y + height - rest], ts);
    }

    /// `words` in quiet lines over `[x, y, width, height]`, as many as fit, the last cut.
    fn art_paragraph(
        &mut self,
        frame: &Frame,
        words: &str,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let size = 14.0;
        let step = size * 1.4 * ts;
        let chars = ((width / (size * 0.52 * ts)) as usize).max(8);
        let words = crate::terms::say(words);
        let lines: Vec<&str> = crate::menu::sjk::wrap(&words, chars).collect();
        let fit = ((height / step).floor().max(0.0) as usize).min(lines.len());
        for (index, line) in lines.iter().take(fit).enumerate() {
            let line = if index + 1 == fit && fit < lines.len() {
                format!("{}...", line.trim_end_matches(['.', ',', ':', ' ']))
            } else {
                (*line).to_owned()
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(x, y + index as f32 * step, width, step),
                size * frame.s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// Tags: the tag chosen in its picture, its name, tier and finish beside it.
    fn art_tag(
        &mut self,
        frame: &Frame,
        chosen: Option<(&'static str, &'static str, Rarity, Option<&'static str>)>,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let Some((id, name, tier, finish)) = chosen else {
            self.art_line(
                frame,
                "No tag chosen",
                [x, y, width],
                18.0,
                color::MUTED,
                true,
                ts,
            );
            self.art_paragraph(
                frame,
                "A tag shows beside your name: choose one on the Tags page.",
                [x, y + 30.0 * ts, width, height - 30.0 * ts],
                ts,
            );
            return;
        };
        // Beside the picture, or under it in a tile taller than it is wide (21:9).
        let words = 22.0 * 1.4 * ts + 14.0 * 1.4 * ts + 30.0;
        let stacked = height - words - 10.0 > width * 0.55;
        let side = if stacked {
            (height - words - 10.0).min(width * 0.7).min(220.0)
        } else {
            height.min(width * 0.42)
        }
        .max(0.0);
        // The picture in its tier's frame, as the Tags wall frames it.
        kit::tier_frame(
            &mut self.ui,
            frame,
            [x, y, side, side],
            tier,
            kit::TierSize::Tile,
            kit::TierLook {
                still: true,
                ..kit::TierLook::at(0.0)
            },
        );
        if let Some(texture) = crate::tag_icons::texture(id) {
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(x + 4.0, y + 4.0, side - 8.0, side - 8.0),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        let (words_x, y) = if stacked {
            (x, y + side + 12.0)
        } else {
            (x + side + 16.0, y)
        };
        let words_width = (x + width - words_x).max(0.0);
        let short = name.trim_end_matches(" tag");
        self.art_line(
            frame,
            short,
            [words_x, y, words_width],
            22.0,
            tier.colour(),
            true,
            ts,
        );
        let below = y + 22.0 * 1.4 * ts;
        let words = self.art_mark(frame, tier, [words_x, below, words_width], 14.0, ts);
        self.art_line(
            frame,
            &format!("{} \u{b7} chosen", tier.label()),
            words,
            14.0,
            color::MUTED,
            false,
            ts,
        );
        if let Some(finish) = finish {
            super::super::tags::finish_badge(
                &mut self.ui,
                frame,
                [words_x, below + 14.0 * 1.4 * ts + 8.0],
                finish,
            );
        }
    }

    /// Toys: every toy's picture, lit in a gold frame while on, faint while off, under
    /// the names of the ones on when there is room for both.
    fn art_toys(
        &mut self,
        frame: &Frame,
        on: &[(usize, bool)],
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let s = frame.s;
        let gap = 6.0;
        let words_room = 15.0 * 1.4 * ts + 8.0;
        let (_, alone) = grid(on.len(), width, height, gap, [ICON_MOST, TOY_SHARE]);
        let (columns, side) = grid(
            on.len(),
            width,
            height - words_room,
            gap,
            [ICON_MOST, TOY_SHARE],
        );
        // The names only while they cost the pictures little.
        let (top, columns, side) = if side >= alone * 0.8 {
            let lit: Vec<String> = on
                .iter()
                .filter(|(_, on)| *on)
                .map(|(toy, _)| super::super::toys::name(*toy).to_owned())
                .collect();
            let (words, colour) = if lit.is_empty() {
                ("None on now".to_owned(), color::MUTED)
            } else {
                (
                    format!("{} on: {}", lit.len(), super::listed(&lit, 2)),
                    color::GOLD,
                )
            };
            self.art_line(frame, &words, [x, y, width], 15.0, colour, false, ts);
            (y + words_room, columns, side)
        } else {
            let (columns, side) = grid(on.len(), width, height, gap, [ICON_MOST, TOY_SHARE]);
            (y, columns, side)
        };
        if side < 12.0 {
            return;
        }
        for (index, (toy, lit)) in on.iter().enumerate() {
            let cx = x + (index % columns) as f32 * (side + gap);
            let cy = top + (index / columns) as f32 * (side + gap);
            if *lit {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(cx, cy, side, side),
                    radius: side * 0.22 * s,
                    color: color::alpha(color::GOLD, 0.16),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(cx, cy, side, side),
                    radius: side * 0.22 * s,
                    width: 1.6 * s,
                    color: color::GOLD_BRIGHT,
                });
            }
            let pad = if *lit { side * 0.08 } else { 0.0 };
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(cx + pad, cy + pad, side - pad * 2.0, side - pad * 2.0),
                texture: super::super::toys::picture(*toy),
                color: Color::new(1.0, 1.0, 1.0, if *lit { 1.0 } else { 0.38 }),
            });
        }
    }

    /// Medals: the names given, then every medal's picture, the ones given in their rank's
    /// light first, the others dim.
    fn art_medals(
        &mut self,
        frame: &Frame,
        shown: &[(crate::medals::Medal, Option<crate::medals::Award>)],
        [x, y, width, height]: [f32; 4],
        ts: f32,
        seconds: f32,
    ) {
        let names: Vec<String> = shown
            .iter()
            .filter_map(|(_, award)| award.as_ref().map(crate::medals::Award::label))
            .collect();
        let (words, colour) = if names.is_empty() {
            ("None given yet".to_owned(), color::MUTED)
        } else {
            (super::listed(&names, 2), color::GOLD)
        };
        self.art_line(frame, &words, [x, y, width], 15.0, colour, false, ts);
        let top = y + 15.0 * 1.4 * ts + 10.0;
        let gap = 10.0;
        let (columns, side) = grid(
            shown.len(),
            width,
            y + height - top,
            gap,
            [MEDAL_MOST, MEDAL_SHARE],
        );
        if side < 14.0 {
            return;
        }
        for (index, (medal, award)) in shown.iter().enumerate() {
            let cx = x + (index % columns) as f32 * (side + gap);
            let cy = top + (index / columns) as f32 * (side + gap);
            let rect = frame.rect(cx, cy, side, side);
            match award {
                Some(award) => crate::medals::rank::draw_art(
                    self.ui.draw_list_mut(),
                    crate::medals::rank::Art::new(award.medal, award.rank, rect, seconds),
                ),
                None => {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                        rect,
                        texture: medal.art(),
                        color: Color::new(0.42, 0.45, 0.52, 0.32),
                    });
                }
            }
        }
    }

    /// Achievements: the next one's name, its progress as a large figure and its bar.
    fn art_achievement(
        &mut self,
        frame: &Frame,
        next: Option<&(String, u64, u64)>,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let Some((name, progress, goal)) = next else {
            self.art_line(
                frame,
                "Every one unlocked",
                [x, y, width],
                18.0,
                color::GOLD,
                true,
                ts,
            );
            return;
        };
        // "Next up" over the name in a tall tile, before it on one line in a short one, so
        // the figure keeps its size.
        let mut top = y;
        if height >= 120.0 * ts {
            self.art_line(
                frame,
                "Next up",
                [x, top, width],
                13.0,
                color::QUIET,
                false,
                ts,
            );
            top += 13.0 * 1.4 * ts;
            self.art_line(frame, name, [x, top, width], 19.0, color::TEXT, true, ts);
            top += 19.0 * 1.4 * ts + 2.0;
        } else {
            let lead = "Next up ";
            let lead_width = label_width(lead, 14.0, 1.15);
            self.art_line(
                frame,
                lead,
                [x, top + 3.0 * ts, lead_width + 8.0],
                14.0,
                color::QUIET,
                false,
                ts,
            );
            let rest = [x + lead_width + 6.0, top, width - lead_width - 6.0];
            self.art_line(frame, name, rest, 18.0, color::TEXT, true, ts);
            top += 18.0 * 1.4 * ts;
        }
        let room = y + height - top - 12.0;
        let big = (room / (1.25 * ts)).clamp(22.0, 64.0);
        let figure = crate::cards::grouped((*progress).min(*goal));
        let figure_width = label_width(&figure, big, 1.0);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{figure}"),
            frame.rect(x, top, figure_width + big, big * 1.25 * ts),
            big * frame.s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let small = big * 0.5;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("/ {}", crate::cards::grouped(*goal)),
            frame.rect(
                x + figure_width + 8.0,
                top + (big - small) * 0.95 * ts,
                width - figure_width - 8.0,
                small * 1.3 * ts,
            ),
            small * frame.s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let share = if *goal > 0 {
            (*progress).min(*goal) as f32 / *goal as f32
        } else {
            0.0
        };
        let bar_y = (top + big * 1.25 * ts + 4.0).min(y + height - 4.0);
        self.art_bar(frame, [x, bar_y, width], share, color::HOLO);
    }

    /// Cards: the cards held as small faces (a sealed copy in its rarity's colour), or an
    /// empty binder.
    fn art_cards(
        &mut self,
        frame: &Frame,
        shown: &[Result<usize, Color>],
        held: Option<usize>,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let s = frame.s;
        if shown.is_empty() {
            // Three blank cards fanned out, and what brings them.
            let card_h = height.clamp(40.0, 110.0);
            let card_w = card_h / 1.4;
            let cx = x + card_w * 0.9;
            for (index, shift) in [(-1.0_f32), 0.0, 1.0].into_iter().enumerate() {
                let left = cx - card_w * 0.5 + shift * card_w * 0.32;
                let top = y + (height - card_h) * 0.5 + shift.abs() * 6.0;
                let alpha = if index == 1 { 0.5 } else { 0.28 };
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(left, top, card_w, card_h),
                    radius: 8.0 * s,
                    color: color::alpha(color::GLASS, 0.95),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(left, top, card_w, card_h),
                    radius: 8.0 * s,
                    width: 1.4 * s,
                    color: color::alpha(color::HOLO, alpha),
                });
            }
            let words_x = cx + card_w * 0.9 + 16.0;
            let words_width = (x + width - words_x).max(0.0);
            let middle = y + height * 0.5;
            let (first, second) = if held.is_some() {
                ("No cards yet", "Found in Holocrons")
            } else {
                ("Cards", "Show once the hub answers")
            };
            self.art_line(
                frame,
                first,
                [words_x, middle - 20.0 * 1.4 * ts, words_width],
                18.0,
                color::TEXT,
                true,
                ts,
            );
            self.art_paragraph(
                frame,
                second,
                [words_x, middle + 2.0, words_width, 14.0 * 1.4 * ts * 2.0],
                ts,
            );
            return;
        }
        let words = crate::cards::grouped(held.unwrap_or(shown.len()) as u64);
        self.art_line(
            frame,
            &format!("{words} held"),
            [x, y, width],
            15.0,
            color::MUTED,
            false,
            ts,
        );
        let top = y + 15.0 * 1.4 * ts + 8.0;
        let card_h = (y + height - top).max(0.0);
        let card_w = card_h / 1.4;
        let gap = 8.0;
        let fit = (((width + gap) / (card_w + gap)).floor() as usize).min(shown.len());
        for (index, card) in shown.iter().take(fit).enumerate() {
            let left = x + index as f32 * (card_w + gap);
            let rect = frame.rect(left, top, card_w, card_h);
            match card {
                Ok(face) => {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect,
                        radius: 6.0 * s,
                        color: color::alpha(color::GLASS, 0.95),
                    });
                    let art_h = card_w / crate::cards::faces::ART_ASPECT;
                    let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                        rect: frame.rect(left, top + (card_h - art_h) * 0.5, card_w, art_h),
                        texture: crate::cards::faces::texture(*face),
                        color: Color::new(1.0, 1.0, 1.0, 1.0),
                    });
                }
                Err(colour) => {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect,
                        radius: 6.0 * s,
                        color: color::alpha(*colour, 0.25),
                    });
                }
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 6.0 * s,
                width: 1.2 * s,
                color: match card {
                    Ok(_) => color::alpha(color::HOLO, 0.4),
                    Err(colour) => *colour,
                },
            });
        }
    }

    /// Holocrons: each tier's picture with how many to open under it, the tiers with none
    /// dim.
    fn art_holocrons(
        &mut self,
        frame: &Frame,
        counts: Option<[u32; 4]>,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let counts = counts.unwrap_or_default();
        let tiers = crate::holocrons::TIERS;
        let column = width / tiers.len() as f32;
        // The tiers' names under the counts only when every one fits its column.
        let named = tiers
            .iter()
            .all(|tier| label_width(tier.label, 12.0, 1.15) <= column - 4.0);
        let marked = tiers.iter().enumerate().all(|(index, tier)| {
            kit::tier_mark_width(kit::holocron_tier(index), 8.0)
                + 5.0
                + label_width(tier.label, 12.0, 1.15)
                <= column - 4.0
        });
        let words = 20.0 * 1.3 * ts + if named { 13.0 * 1.3 * ts } else { 0.0 };
        let side = (height - words - 6.0).min(column - 10.0).min(HOLOCRON_MOST);
        // Centred down a tall tile.
        let y = y + ((height - side.max(0.0) - 6.0 - words) * 0.5).max(0.0);
        for (index, (tier, count)) in tiers.iter().zip(counts).enumerate().rev() {
            let place = tiers.len() - 1 - index;
            let middle = x + column * (place as f32 + 0.5);
            let alpha = if count > 0 { 1.0 } else { 0.28 };
            if side > 12.0 {
                holocron_picture(
                    &mut self.ui,
                    frame,
                    index,
                    [middle, y + side * 0.5],
                    side * 0.5,
                    alpha,
                );
            }
            let top = y + side.max(0.0) + 6.0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{count}"),
                frame.rect(middle - column * 0.5, top, column, 20.0 * 1.3 * ts),
                20.0 * frame.s,
                if count > 0 { tier.colour } else { color::QUIET },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            if !named {
                continue;
            }
            // The tier's mark before its name, centred together, when every tier's fits.
            let rank = kit::holocron_tier(index);
            let label_top = top + 20.0 * 1.3 * ts;
            let mark = kit::tier_mark_width(rank, 8.0);
            let label = label_width(tier.label, 12.0, 1.15);
            let offset = if marked {
                let start = middle - (mark + 5.0 + label) * 0.5;
                let line = label_top + 13.0 * 1.3 * ts * 0.5;
                kit::tier_mark(&mut self.ui, frame, [start, line], rank, 8.0, count == 0);
                (mark + 5.0) * 0.5
            } else {
                0.0
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", tier.label),
                frame.rect(
                    middle - column * 0.5 + offset * 2.0,
                    label_top,
                    column - offset * 2.0,
                    13.0 * 1.3 * ts,
                ),
                12.0 * frame.s,
                color::alpha(color::QUIET, if count > 0 { 1.0 } else { 0.6 }),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
    }

    /// A price ending at `right` on the line from `top` (`size` its words): the currency's
    /// mark and the figure, or the offer's note ("owned", "bought") in quiet words. Its
    /// width.
    fn art_price(
        &mut self,
        frame: &Frame,
        [right, top]: [f32; 2],
        offer: &Deal,
        size: f32,
        ts: f32,
    ) -> f32 {
        let line = size * 1.4 * ts;
        if !offer.blade.note.is_empty() {
            let width = label_width(offer.blade.note, size - 2.0, 1.15) + 4.0;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", offer.blade.note),
                frame.rect(right - width, top + 1.0, width, line),
                (size - 2.0) * frame.s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            return width;
        }
        let price = crate::cards::grouped(offer.price);
        let mark = size * 0.95 * ts;
        let mark_width = if offer.shards {
            crate::cards::shard::width(mark)
        } else {
            crate::cards::mark::width(mark)
        };
        let figure_width = label_width(&price, size, 1.0);
        let width = mark_width + 5.0 + figure_width;
        let left = right - width;
        let at = frame.point(left, top + (line - mark) * 0.5);
        let ink = if offer.shards {
            let kyber = crate::cards::shard::KYBER;
            crate::cards::shard::draw(self.ui.draw_list_mut(), at, mark * frame.s, kyber);
            kyber
        } else {
            crate::cards::mark::draw(self.ui.draw_list_mut(), at, mark * frame.s, color::GOLD);
            color::GOLD_BRIGHT
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{price}"),
            frame.rect(left + mark_width + 5.0, top, figure_width + size, line),
            size * frame.s,
            ink,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        width
    }

    /// The Forge: the deal of the day's blade, its name and price, the day's other offers
    /// under it as long as they fit, and the duplicates to break down at the foot.
    #[allow(clippy::too_many_arguments)]
    fn art_forge(
        &mut self,
        frame: &Frame,
        deal: Option<&Deal>,
        offers: &[Deal],
        duplicates: Option<u32>,
        [x, y, width, height]: [f32; 4],
        ts: f32,
        stock: Color,
    ) {
        let foot = y + height;
        let duplicate_line = duplicates.filter(|copies| *copies > 0).map(|copies| {
            super::counted(
                u64::from(copies),
                "duplicate to break down",
                "duplicates to break down",
            )
        });
        let small = 14.0 * 1.4 * ts;
        let bottom = if duplicate_line.is_some() {
            foot - small - 4.0
        } else {
            foot
        };
        let mut top = y;
        match deal {
            Some(deal) => {
                self.art_line(
                    frame,
                    "Deal of the day",
                    [x, top, width],
                    13.0,
                    color::QUIET,
                    false,
                    ts,
                );
                top += 13.0 * 1.4 * ts + 4.0;
                let strip = (height * 0.16).clamp(STRIP_HEIGHT, 32.0);
                self.art_strip(frame, [x, top, width, strip], deal.blade.id, stock);
                top += strip + 6.0;
                let line = 17.0 * 1.4 * ts;
                let price = self.art_price(frame, [x + width, top], deal, 17.0, ts);
                let name = [x, top, width - price - 12.0];
                let name = match deal.blade.tier {
                    Some(tier) => self.art_mark(frame, tier, name, 17.0, ts),
                    None => name,
                };
                self.art_line(
                    frame,
                    &deal.blade.name,
                    name,
                    17.0,
                    deal.blade.colour,
                    true,
                    ts,
                );
                top += line + 10.0;
                // The day's other offers: a small strip, the name and the price a row.
                let row = (15.0 * 1.4 * ts).max(STRIP_HEIGHT * 0.8) + 6.0;
                if !offers.is_empty() && top + row <= bottom {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: frame.rect(x, top - 5.0, width, 1.0),
                        radius: 0.0,
                        color: color::alpha(color::HOLO, 0.12),
                    });
                    top += 2.0;
                }
                for offer in offers {
                    if top + row > bottom {
                        break;
                    }
                    let strip_width = (width * 0.3).min(110.0);
                    let strip_height = STRIP_HEIGHT * 0.7;
                    let middle = top + (row - 6.0) * 0.5;
                    self.art_strip(
                        frame,
                        [x, middle - strip_height * 0.5, strip_width, strip_height],
                        offer.blade.id,
                        stock,
                    );
                    let line_top = middle - 15.0 * 0.7 * ts;
                    let price = self.art_price(frame, [x + width, line_top], offer, 15.0, ts);
                    let name_x = x + strip_width + 10.0;
                    let name = [
                        name_x,
                        line_top,
                        (x + width - price - 10.0 - name_x).max(0.0),
                    ];
                    let name = match offer.blade.tier {
                        Some(tier) => self.art_mark(frame, tier, name, 15.0, ts),
                        None => name,
                    };
                    self.art_line(
                        frame,
                        &offer.blade.name,
                        name,
                        15.0,
                        offer.blade.colour,
                        false,
                        ts,
                    );
                    top += row;
                }
            }
            None => {
                self.art_paragraph(
                    frame,
                    "Open the Forge to see the deal of the day.",
                    [x, top, width, small * 2.0],
                    ts,
                );
            }
        }
        if let Some(words) = duplicate_line
            && bottom >= y
        {
            self.art_line(
                frame,
                &words,
                [x, bottom + 4.0, width],
                14.0,
                color::MUTED,
                false,
                ts,
            );
        }
    }

    /// Inventory: the credits and kyber shards as large figures with their marks, the rest
    /// held under them.
    fn art_inventory(
        &mut self,
        frame: &Frame,
        money: Option<(u64, u64)>,
        rest: &str,
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        let Some((credits, shards)) = money else {
            return;
        };
        let rest_room = 14.0 * 1.4 * ts + 4.0;
        let big = ((height - rest_room - 8.0) / (2.0 * 1.3 * ts)).clamp(18.0, 40.0);
        let line = big * 1.3 * ts;
        // Centred down a tall tile.
        let y = y + ((height - 2.0 * (line + 4.0) - 4.0 - rest_room) * 0.5).max(0.0);
        for (index, (amount, words, shard)) in
            [(credits, "credits", false), (shards, "kyber shards", true)]
                .into_iter()
                .enumerate()
        {
            let top = y + index as f32 * (line + 4.0);
            let mark = big * 0.8 * ts;
            let at = frame.point(x, top + (line - mark) * 0.5);
            let (ink, mark_width) = if shard {
                let kyber = crate::cards::shard::KYBER;
                crate::cards::shard::draw(self.ui.draw_list_mut(), at, mark * frame.s, kyber);
                (kyber, crate::cards::shard::width(mark))
            } else {
                crate::cards::mark::draw(self.ui.draw_list_mut(), at, mark * frame.s, color::GOLD);
                (color::GOLD_BRIGHT, crate::cards::mark::width(mark))
            };
            let figure = crate::cards::grouped(amount);
            let left = x + mark_width + 10.0;
            let figure_width = label_width(&figure, big, 1.0);
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{figure}"),
                frame.rect(left, top, figure_width + big, line),
                big * frame.s,
                ink,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let after = left + figure_width + 10.0;
            self.art_line(
                frame,
                words,
                [
                    after,
                    top + (line - 14.0 * 1.4 * ts) * 0.6,
                    (x + width - after).max(0.0),
                ],
                14.0,
                color::QUIET,
                false,
                ts,
            );
        }
        let top = y + 2.0 * (line + 4.0) + 4.0;
        if !rest.is_empty() && top + 14.0 * 1.4 * ts <= y + height + 4.0 {
            self.art_line(frame, rest, [x, top, width], 14.0, color::MUTED, false, ts);
        }
    }

    /// History: the newest lines, each its day in gold and its sentence, as many as fit.
    fn art_history(
        &mut self,
        frame: &Frame,
        rows: &[(String, String)],
        [x, y, width, height]: [f32; 4],
        ts: f32,
    ) {
        if rows.is_empty() {
            self.art_paragraph(
                frame,
                "What came, was opened, spent and traded.",
                [x, y, width, height],
                ts,
            );
            return;
        }
        let size = 15.0;
        let line = size * 1.4 * ts;
        let step = line + 12.0;
        let day_width = label_width("00/00", 14.0, 1.0) + 12.0;
        for (index, (day, words)) in rows.iter().enumerate() {
            let top = y + index as f32 * step;
            if top + line > y + height {
                break;
            }
            if index > 0 {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x, top - 6.0, width, 1.0),
                    radius: 0.0,
                    color: color::alpha(color::HOLO, 0.12),
                });
            }
            self.art_line(
                frame,
                day,
                [x, top + 1.0, day_width],
                14.0,
                color::GOLD,
                true,
                ts,
            );
            self.art_line(
                frame,
                words,
                [x + day_width, top, width - day_width],
                size,
                if index == 0 {
                    color::TEXT
                } else {
                    color::MUTED
                },
                false,
                ts,
            );
        }
    }
}

/// Holocron tier `index`'s picture over the square of side `2 * half` centred on `centre`
/// (frame pixels) at `alpha`: its icon, or its gem where the icon did not load.
fn holocron_picture(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    index: usize,
    [x, y]: [f32; 2],
    half: f32,
    alpha: f32,
) {
    use crate::holocrons::{gem, icons};
    if icons::is_loaded(index) {
        let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: frame.rect(x - half, y - half, half * 2.0, half * 2.0),
            texture: icons::texture(index),
            color: Color::new(1.0, 1.0, 1.0, alpha),
        });
    } else {
        gem::draw(
            canvas.draw_list_mut(),
            frame.point(x, y),
            half * frame.s,
            crate::holocrons::TIERS[index].colour,
            alpha,
            gem::ROWS,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The grid of pictures takes the columns giving the largest, within its box.
    #[test]
    fn pictures_take_the_largest_grid_that_fits() {
        // Fourteen toys in a wide, short box: two rows of seven.
        let (columns, side) = grid(14, 280.0, 80.0, 8.0, [ICON_MOST, TOY_SHARE]);
        assert_eq!(columns, 7);
        assert!(side * 2.0 + 8.0 <= 80.0 + 0.01 && side * 7.0 + 48.0 <= 280.0 + 0.01);
        // A tall box: fewer columns, larger pictures, never past the most.
        let (columns, side) = grid(14, 280.0, 400.0, 8.0, [ICON_MOST, TOY_SHARE]);
        assert!(columns < 7);
        assert!(side <= ICON_MOST);
    }
}
