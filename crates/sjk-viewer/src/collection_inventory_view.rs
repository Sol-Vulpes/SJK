//! The Inventory tab's look, in the SJK UI's: its headline, the wallet strip (each balance
//! with its mark and how it is earned, and what the spare kybers are worth), the kind
//! chips and the sort's segments over the list with the search field over the card, then
//! the list in the left two thirds, a heading to each kind (its name, how many kinds and
//! how many in all) and a row to each thing held in its tier's frame (its picture, its
//! name in its tier's colour, the tier's mark and name, a note and how many), a bar at the
//! right when it scrolls, and in the right third the chosen thing's card: its picture,
//! tier and edition, name, how it came, the copies held, what it is worth in kyber shards
//! and its buttons, a greyed one saying why. An empty list says why.

use super::inventory::{
    ACTION_BASE, CHIP_BASE, Item, OFFERS_MAX, Place, ROW_BASE, ROWS_SHOWN, SEARCH_TOKEN, SORT_BASE,
    Sort, Standing, Thing, chip_labels, offers, worth,
};
use super::view::{LEFT_X, RIGHT_X, label_width};
use super::*;
use crate::menu::sjk::kit::{TierLook, TierSize};
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The wallet strip's top and height.
const WALLET_Y: f32 = 268.0;
const WALLET_HEIGHT: f32 = 50.0;
/// The controls' line (its middle), the list's top and a place's height.
const CONTROLS_Y: f32 = 354.0;
const LIST_Y: f32 = 390.0;
const ROW_HEIGHT: f32 = 43.0;
/// The list's right edge and the card's column.
const LIST_RIGHT: f32 = 1_188.0;
const CARD_X: f32 = 1_224.0;
const CARD_WIDTH: f32 = RIGHT_X - CARD_X;
const CARD_HEIGHT: f32 = ROWS_SHOWN as f32 * ROW_HEIGHT;
/// Inside the card: its margin.
const PAD: f32 = 26.0;
/// The columns of a row, from the list's left: the picture, the name, the tier and the
/// note; how many is at the right.
const PICTURE_X: f32 = 20.0;
const PICTURE_WIDTH: f32 = 118.0;
const NAME_X: f32 = 156.0;
const TIER_X: f32 = 580.0;
const NOTE_X: f32 = 780.0;
const COUNT_WIDTH: f32 = 120.0;

impl Panel {
    /// The Inventory tab.
    pub(super) fn inventory_tab(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        self.follow_inventory(inputs);
        self.lead(frame, "Inventory", "");
        self.inventory_wallet(frame);
        let chips = chip_labels(&self.inventory.items);
        let chip_words: Vec<&str> = chips.iter().map(String::as_str).collect();
        kit::segments(
            &mut self.ui,
            frame,
            LEFT_X + kit::segments_width(&chip_words),
            CONTROLS_Y,
            &chip_words,
            self.inventory.filter.index(),
            false,
            CHIP_BASE,
        );
        let labels: Vec<&str> = Sort::ALL.iter().map(|sort| sort.label()).collect();
        kit::segments(
            &mut self.ui,
            frame,
            LIST_RIGHT,
            CONTROLS_Y,
            &labels,
            self.inventory.sort.index(),
            false,
            SORT_BASE,
        );
        let found = (!self.inventory.query.trim().is_empty()).then(|| {
            self.inventory
                .places
                .iter()
                .filter(|place| matches!(place, Place::Item(_)))
                .count()
        });
        kit::search(
            &mut self.ui,
            frame,
            [
                CARD_X,
                CONTROLS_Y - kit::CONTROL_HEIGHT * 0.5,
                CARD_WIDTH,
                kit::CONTROL_HEIGHT,
            ],
            &self.inventory.query,
            self.inventory.search_focus,
            "Search your things",
            found,
            SEARCH_TOKEN,
        );
        if self.inventory.places.is_empty() {
            self.inventory.offers.clear();
            let (headline, line) = match inputs.holdings.reason() {
                Some(reason) => ("Inventory unavailable", reason),
                None if self.inventory.query.trim().is_empty() => (
                    "Nothing of this kind yet",
                    "What you find, earn and craft shows here.",
                ),
                None => (
                    "Nothing matches",
                    "No thing you own holds every word of the search.",
                ),
            };
            self.inventory_empty(frame, headline, line);
            return;
        }
        let first = self.inventory.first;
        let shown = self
            .inventory
            .places
            .len()
            .saturating_sub(first)
            .min(ROWS_SHOWN);
        for slot in 0..shown {
            let y = LIST_Y + slot as f32 * ROW_HEIGHT;
            match self.inventory.places[first + slot] {
                Place::Heading {
                    section,
                    kinds,
                    quantity,
                } => self.inventory_heading(frame, y, &section.label(), kinds, quantity),
                Place::Item(index) => self.inventory_row(frame, slot, y, index, inputs),
            }
        }
        let places = self.inventory.places.len();
        if places > ROWS_SHOWN {
            let track = ROWS_SHOWN as f32 * ROW_HEIGHT;
            let thumb = (track * ROWS_SHOWN as f32 / places as f32).max(24.0);
            let along = (track - thumb) * first as f32 / (places - ROWS_SHOWN) as f32;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(LIST_RIGHT + 10.0, LIST_Y + along, 4.0, thumb),
                radius: 2.0 * frame.s,
                color: color::alpha(color::HOLO, 0.35),
            });
        }
        match self.inventory.chosen_item().cloned() {
            Some(item) => self.inventory_card(frame, &item, inputs, seconds),
            None => self.inventory.offers.clear(),
        }
    }

    /// The wallet strip: each balance with its mark, its figure and how it is earned, and
    /// at the right what the spare kybers would give.
    fn inventory_wallet(&mut self, frame: &Frame) {
        let s = frame.s;
        let wallet = self.inventory.wallet;
        kit::tier_frame(
            &mut self.ui,
            frame,
            [LEFT_X, WALLET_Y, RIGHT_X - LEFT_X, WALLET_HEIGHT],
            unlockables::Rarity::Common,
            TierSize::Row,
            TierLook::at(0.0),
        );
        let middle = WALLET_Y + WALLET_HEIGHT * 0.5;
        let mut x = LEFT_X + 22.0;
        let mut balances = vec![
            (
                Mark::Credits,
                crate::cards::credits_text(wallet.credits),
                color::GOLD_BRIGHT,
                crate::terms::say("credits, with every card a holocron brings").into_owned(),
            ),
            (
                Mark::Shards,
                crate::economy::shards_text(wallet.shards),
                crate::cards::shard::KYBER,
                "kyber shards, from holocrons and kybers broken down".to_owned(),
            ),
        ];
        if let Some(coins) = wallet.coins {
            balances.push((
                Mark::None,
                crate::cards::grouped(coins),
                crate::holocrons::rgb(0xE5_1A1A),
                "DOOM coins, from the WHEEL OF DOOM".to_owned(),
            ));
        }
        let mark_height = 24.0;
        for (mark, figure, colour, words) in balances {
            let list = self.ui.draw_list_mut();
            let at = frame.point(x, middle - mark_height * 0.5);
            let mark_width = match mark {
                Mark::Credits => {
                    let _ = crate::cards::mark::draw(list, at, mark_height * s, color::GOLD);
                    crate::cards::mark::width(mark_height) + 10.0
                }
                Mark::Shards => {
                    let _ = crate::cards::shard::draw(list, at, mark_height * s, colour);
                    crate::cards::shard::width(mark_height) + 10.0
                }
                Mark::None => 0.0,
            };
            let figure_width = label_width(&figure, 26.0, self.widen) + 6.0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{figure}"),
                frame.rect(x + mark_width, middle - 16.0, figure_width, 32.0),
                26.0 * s,
                colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let words_x = x + mark_width + figure_width + 6.0;
            let words_width = crate::text::display_width(&words, 14.0) + 8.0;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(words_x, middle - 11.0, words_width + 40.0, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            x = words_x + words_width + 40.0;
        }
        let spare = if wallet.spare_copies == 0 {
            ("No spare kybers to break down".to_owned(), color::QUIET)
        } else {
            (
                format!(
                    "{} spare {} worth {} kyber shards",
                    wallet.spare_copies,
                    if wallet.spare_copies == 1 {
                        "copy"
                    } else {
                        "copies"
                    },
                    crate::economy::shards_text(wallet.spare_shards)
                ),
                color::GOLD_BRIGHT,
            )
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", spare.0),
            frame.rect(RIGHT_X - 460.0, middle - 12.0, 440.0, 24.0),
            16.0 * s,
            spare.1,
            FontWeight::Semibold,
            TextAlign::End,
        );
    }

    /// The words over an empty list.
    fn inventory_empty(&mut self, frame: &Frame, headline: &str, line: &str) {
        let s = frame.s;
        let width = LIST_RIGHT - LEFT_X;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(LEFT_X, 560.0, width, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X, 606.0, width, 26.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Center,
        );
    }

    /// A kind's heading, its top at `y`: its name, a rule, how many kinds and how many in
    /// all.
    fn inventory_heading(
        &mut self,
        frame: &Frame,
        y: f32,
        label: &str,
        kinds: usize,
        quantity: u64,
    ) {
        let s = frame.s;
        let middle = y + ROW_HEIGHT * 0.5 + 4.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(LEFT_X, middle - 14.0, 600.0, 28.0),
            22.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let kinds = if kinds == 1 {
            "1 kind".to_owned()
        } else {
            format!("{kinds} kinds")
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{kinds}, {} in all", crate::cards::grouped(quantity)),
            frame.rect(LIST_RIGHT - 500.0, middle - 12.0, 484.0, 24.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(LEFT_X, y + ROW_HEIGHT - 3.0, LIST_RIGHT - LEFT_X, 1.0),
            color: color::alpha(color::HOLO, 0.22),
        });
    }

    /// Item `index` in place `slot`, its top at `y`.
    fn inventory_row(
        &mut self,
        frame: &Frame,
        slot: usize,
        y: f32,
        index: usize,
        inputs: &Inputs<'_>,
    ) {
        let s = frame.s;
        let token = ROW_BASE + slot as u16;
        let chosen = self.inventory.first + slot == self.inventory.chosen;
        let hovered = self.ui.token_hovered(token);
        let Some(item) = self.inventory.items.get(index).cloned() else {
            return;
        };
        let band = [LEFT_X, y + 2.0, LIST_RIGHT - LEFT_X, ROW_HEIGHT - 4.0];
        kit::tier_frame(
            &mut self.ui,
            frame,
            band,
            item.rarity.unwrap_or(unlockables::Rarity::Common),
            TierSize::Row,
            TierLook {
                chosen,
                hovered: hovered || chosen,
                ..TierLook::at(crate::menu::art::motion::seconds() as f32)
            },
        );
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(band[0] - 3.0, band[1] - 3.0, band[2] + 6.0, band[3] + 6.0),
                radius: 11.0 * s,
                width: 1.6 * s,
                color: color::alpha(color::GOLD_BRIGHT, 0.85),
            });
        }
        let middle = y + ROW_HEIGHT * 0.5;
        self.inventory_picture(
            frame,
            &item,
            [LEFT_X + PICTURE_X, middle - 15.0, PICTURE_WIDTH, 30.0],
            inputs,
            false,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", item.name),
            frame.rect(LEFT_X + NAME_X, middle - 12.0, TIER_X - NAME_X - 14.0, 24.0),
            18.0 * s,
            if chosen {
                color::GOLD_BRIGHT
            } else {
                item.colour
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some(rarity) = item.rarity {
            let mark = kit::tier_mark(
                &mut self.ui,
                frame,
                [LEFT_X + TIER_X, middle],
                rarity,
                11.0,
                false,
            );
            let gap = if mark > 0.0 { mark + 8.0 } else { 0.0 };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", item.tier),
                frame.rect(
                    LEFT_X + TIER_X + gap,
                    middle - 12.0,
                    NOTE_X - TIER_X - gap - 12.0,
                    24.0,
                ),
                16.0 * s,
                color::alpha(item.colour, 0.9),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        if !item.note.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", item.note),
                frame.rect(
                    LEFT_X + NOTE_X,
                    middle - 12.0,
                    LIST_RIGHT - COUNT_WIDTH - LEFT_X - NOTE_X - 12.0,
                    24.0,
                ),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", crate::cards::grouped(item.count)),
            frame.rect(
                LIST_RIGHT - COUNT_WIDTH,
                middle - 12.0,
                COUNT_WIDTH - 18.0,
                24.0,
            ),
            20.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::End,
        );
        self.ui
            .hit_region(token, frame.rect(band[0], band[1], band[2], band[3]));
    }

    /// `item`'s picture over `rect` (frame pixels): a kyber's blade, a holocron's or a
    /// dormant kyber's icon, a tag's picture, a card's face (a sealed one its rarity's
    /// card), a finisher's tier mark. `large` is the card's (a live blade, the card
    /// itself).
    fn inventory_picture(
        &mut self,
        frame: &Frame,
        item: &Item,
        [x, y, width, height]: [f32; 4],
        inputs: &Inputs<'_>,
        large: bool,
    ) {
        let s = frame.s;
        let middle = [x + width * 0.5, y + height * 0.5];
        let side = height.min(width);
        match item.thing {
            Thing::Kyber(id) => {
                let look = self.skins.get(id);
                let turn = swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock));
                if large {
                    let seconds = crate::menu::art::motion::seconds() as f32;
                    let _ = swatch::small_swatch(
                        self.ui.draw_list_mut(),
                        frame,
                        [x, y, width, height],
                        look,
                        true,
                        seconds,
                        turn,
                    );
                } else {
                    let _ = swatch::strip(
                        self.ui.draw_list_mut(),
                        frame,
                        [x, middle[1] - height * 0.3, width, height * 0.6],
                        look,
                        true,
                        turn,
                    );
                }
            }
            Thing::Holocron(slot) | Thing::Crystal(slot) => {
                holocron_icon(&mut self.ui, frame, slot, middle, side * 0.5);
            }
            Thing::Tag(id) => {
                let texture = crate::tag_icons::texture(id);
                self.square_picture(frame, texture, middle, side, item.colour);
            }
            Thing::GifTags => {
                let texture = crate::tag_icons::texture(crate::tags::BLANK);
                self.square_picture(frame, texture, middle, side, item.colour);
            }
            Thing::Finisher(_) => {
                if let Some(rarity) = item.rarity {
                    let size = if large { 26.0 } else { 12.0 };
                    let steps = f32::from(kit::tier_step(rarity).max(1));
                    let wide = steps * size + (steps - 1.0) * size * 0.28;
                    let _ = kit::tier_mark(
                        &mut self.ui,
                        frame,
                        [middle[0] - wide * 0.5, middle[1]],
                        rarity,
                        size,
                        false,
                    );
                }
            }
            Thing::Card { face, finish, .. } => {
                let rarity = &crate::cards::RARITIES[usize::from(item.rank).min(4)];
                if large {
                    let faces = crate::cards::faces::current();
                    let card_width = height / crate::cards::look::ASPECT;
                    crate::cards::look::draw(
                        &mut self.ui,
                        frame,
                        [middle[0] - card_width * 0.5, y, card_width],
                        crate::cards::look::Card {
                            rarity,
                            finish: &crate::cards::FINISHES[finish.min(2)],
                            edition: "",
                            face: face.and_then(|index| {
                                faces
                                    .list
                                    .get(index)
                                    .map(|face| crate::cards::look::Shown { face, index })
                            }),
                            tilt: None,
                            dim: false,
                            label: false,
                        },
                        crate::menu::art::motion::seconds() as f32,
                        1.0,
                        1.0,
                    );
                    return;
                }
                let card_height = height;
                let card_width = card_height / 1.4;
                let left = middle[0] - card_width * 0.5;
                let rect = frame.rect(left, y, card_width, card_height);
                let list = self.ui.draw_list_mut();
                match face {
                    Some(index) => {
                        let _ = list.push(DrawCommand::RoundedRect {
                            rect,
                            radius: 3.0 * s,
                            color: Color::new(0.05, 0.08, 0.15, 0.95),
                        });
                        let art = card_width / crate::cards::faces::ART_ASPECT;
                        let _ = list.push(DrawCommand::TexturedQuad {
                            rect: frame.rect(left, y + (card_height - art) * 0.5, card_width, art),
                            texture: crate::cards::faces::texture(index),
                            color: Color::new(1.0, 1.0, 1.0, 1.0),
                        });
                    }
                    None => {
                        let _ = list.push(DrawCommand::RoundedRect {
                            rect,
                            radius: 3.0 * s,
                            color: color::alpha(rarity.colour, 0.3),
                        });
                    }
                }
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 3.0 * s,
                    width: 1.2 * s,
                    color: color::alpha(rarity.colour, 0.9),
                });
            }
        }
    }

    /// A square picture of side `side` centred on `centre`: `texture`, or an outline in
    /// `colour` while its pack has not come.
    fn square_picture(
        &mut self,
        frame: &Frame,
        texture: Option<sjk_ui::TextureId>,
        [cx, cy]: [f32; 2],
        side: f32,
        colour: Color,
    ) {
        let rect = frame.rect(cx - side * 0.5, cy - side * 0.5, side, side);
        let _ = self.ui.draw_list_mut().push(match texture {
            Some(texture) => DrawCommand::TexturedQuad {
                rect,
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            },
            None => DrawCommand::Border {
                rect,
                radius: 6.0 * frame.s,
                width: 1.2 * frame.s,
                color: color::alpha(colour, 0.5),
            },
        });
    }

    /// The chosen thing's card in the right third: its tier's frame, its picture, the
    /// tier's mark and name with the edition, its name, how it came, what is held, what it
    /// is worth, and its buttons with why one cannot act.
    fn inventory_card(&mut self, frame: &Frame, item: &Item, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        let rarity = item.rarity.unwrap_or(unlockables::Rarity::Common);
        kit::tier_frame(
            &mut self.ui,
            frame,
            [CARD_X, LIST_Y, CARD_WIDTH, CARD_HEIGHT],
            rarity,
            TierSize::Tile,
            TierLook {
                shine: true,
                ..TierLook::at(seconds)
            },
        );
        let x = CARD_X + PAD;
        let width = CARD_WIDTH - PAD * 2.0;
        let mut y = LIST_Y + 24.0;
        let picture_height = match item.thing {
            Thing::Card { .. } => 196.0,
            Thing::Kyber(_) => 112.0,
            Thing::Finisher(_) => 90.0,
            _ => 130.0,
        };
        self.inventory_picture(frame, item, [x, y, width, picture_height], inputs, true);
        y += picture_height + 22.0;
        // The tier's mark and name, the kind; the edition at the right.
        let kind = match item.thing {
            Thing::Kyber(_) => "Kyber",
            Thing::Holocron(_) => "Holocron",
            Thing::Crystal(_) => "Dormant kyber",
            Thing::Finisher(_) => "Finisher",
            Thing::Tag(_) | Thing::GifTags => "Tag",
            Thing::Card { .. } => "Card",
        };
        let kind = crate::terms::say(kind);
        let mark = match item.rarity {
            Some(rarity) => kit::tier_mark(&mut self.ui, frame, [x, y + 11.0], rarity, 13.0, false),
            None => 0.0,
        };
        let after = if mark > 0.0 { mark + 10.0 } else { 0.0 };
        let tier_words = if item.tier.is_empty() {
            kind.into_owned()
        } else {
            format!("{} \u{b7} {kind}", item.tier)
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{tier_words}"),
            frame.rect(x + after, y, width * 0.6 - after, 22.0),
            17.0 * s,
            item.colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some(edition) = edition_of(item) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}  {edition}", crate::symbols::EDITION),
                frame.rect(x + width * 0.5, y, width * 0.5, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        y += 30.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", item.name),
            frame.rect(x, y, width, 40.0),
            34.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 46.0;
        // What the client knows of it, each fact on up to two lines, four lines in all.
        let mut room = 4;
        for line in facts(item, inputs) {
            if room == 0 {
                break;
            }
            let lines = crate::menu::sjk::wrap(&line, 66).count().min(2).min(room);
            let bottom = super::view::left_lines(
                &mut self.ui,
                frame,
                &line,
                [x, y, width],
                (66, lines),
                16.0,
                color::MUTED,
                TextAlign::Start,
            );
            room -= lines;
            y = bottom.max(y + 22.0) + 2.0;
        }
        y += 6.0;
        let standing = self.inventory_standing(item, inputs);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", held_words(item, &standing)),
            frame.rect(x, y, width, 28.0),
            22.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 34.0;
        // What it is worth, from the hub's tables.
        if let Thing::Kyber(id) = item.thing
            && let Some(skin) = unlockables::find(id)
        {
            self.inventory_worth(frame, skin, [x, y, width]);
        }
        // The buttons: the main one across, the others side by side under it.
        let offers = offers(item, &standing);
        let bottom = LIST_Y + CARD_HEIGHT - PAD;
        let why = offers
            .iter()
            .find_map(|offer| offer.why.clone())
            .or_else(|| self.inventory_answer(item));
        let rows = if offers.len() > 1 { 2.0 } else { 1.0 };
        let why_room = 46.0;
        let mut button_y = bottom - why_room - rows * 46.0 - (rows - 1.0) * 10.0;
        for (index, offer) in offers.iter().enumerate().take(OFFERS_MAX) {
            let (left, button_width) = match (index, offers.len()) {
                (0, _) | (_, 2) => (x, width),
                (1, _) => (x, (width - 10.0) * 0.5),
                _ => (x + (width + 10.0) * 0.5, (width - 10.0) * 0.5),
            };
            let label = format!("{}  ({})", offer.label, offer.key);
            kit::button(
                &mut self.ui,
                frame,
                [left, button_y, button_width, 46.0],
                &label,
                index == 0 && offer.enabled,
                offer.enabled,
                false,
                ACTION_BASE + index as u16,
            );
            if index == 0 {
                button_y += 56.0;
            }
        }
        if let Some(why) = why {
            let _ = super::view::left_lines(
                &mut self.ui,
                frame,
                &why,
                [x, bottom - why_room + 8.0, width],
                (64, 2),
                15.0,
                color::MUTED,
                TextAlign::Start,
            );
        }
        self.inventory.offers = offers;
    }

    /// What kyber `skin` is worth, from the hub's tables, in lines from `[x, y, width]`:
    /// a copy broken down, one crafted, the spares a fusion takes.
    fn inventory_worth(
        &mut self,
        frame: &Frame,
        skin: &unlockables::Unlockable,
        [x, mut y, width]: [f32; 3],
    ) {
        let s = frame.s;
        let value = worth(skin);
        let mut lines: Vec<(String, Option<String>)> = Vec::with_capacity(3);
        if let Some(shatter) = value.shatter {
            lines.push((
                "Breaking a copy down gives".to_owned(),
                Some(format!("+{}", crate::economy::shards_text(shatter))),
            ));
        }
        if let Some(craft) = value.craft {
            lines.push((
                "Crafting one costs".to_owned(),
                Some(crate::economy::shards_text(craft)),
            ));
        }
        if let Some(need) = value.fuse {
            lines.push((
                format!("{need} spares of its tier fuse into a rarer one in the Forge"),
                None,
            ));
        }
        if value == Default::default() {
            lines.push(("Outside the trade: it has no price.".to_owned(), None));
        }
        for (words, amount) in lines {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(x, y, width - 130.0, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            if let Some(amount) = amount {
                let figure = label_width(&amount, 20.0, self.widen) + 4.0;
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{amount}"),
                    frame.rect(x + width - figure, y - 1.0, figure, 26.0),
                    20.0 * s,
                    crate::cards::shard::KYBER,
                    FontWeight::Semibold,
                    TextAlign::End,
                );
                let mark = 18.0;
                let at = frame.point(
                    x + width - figure - 6.0 - crate::cards::shard::width(mark),
                    y + 3.0,
                );
                let _ = crate::cards::shard::draw(
                    self.ui.draw_list_mut(),
                    at,
                    mark * s,
                    crate::cards::shard::KYBER,
                );
            }
            y += 28.0;
        }
    }

    /// The hub's answer to the last break down of `item`'s kyber sent from here or the
    /// Forge, in words.
    fn inventory_answer(&self, item: &Item) -> Option<String> {
        let Thing::Kyber(id) = item.thing else {
            return None;
        };
        let (outcome, asked) = self.forge.answer()?;
        (asked == Some(super::forge::Press::Break(id))).then(|| {
            crate::economy::outcome_words(
                outcome,
                unlockables::find(id).map_or(id, |skin| skin.name),
            )
        })
    }
}

/// Which balance mark a wallet figure carries.
#[derive(Clone, Copy)]
enum Mark {
    Credits,
    Shards,
    None,
}

/// The edition `item` belongs to, as players read it ("Alpha Edition"), when it has one.
fn edition_of(item: &Item) -> Option<String> {
    match item.thing {
        Thing::Kyber(id) => unlockables::find(id)
            .map(|skin| crate::editions::name(skin.edition))
            .filter(|name| !name.is_empty()),
        Thing::Card { .. } => item
            .note
            .split(", ")
            .find(|part| part.ends_with("Edition"))
            .map(str::to_owned),
        _ => None,
    }
}

/// What the card says of `item` under its name: its number and smith, how it came and
/// since when, what it does, as far as the client knows.
fn facts(item: &Item, inputs: &Inputs<'_>) -> Vec<String> {
    let mut lines = Vec::with_capacity(3);
    let came = |id: &str, lines: &mut Vec<String>| {
        let Some(unlock) = inputs.holdings.unlock(id) else {
            return;
        };
        if let Some(medal) = unlock
            .medal
            .as_deref()
            .and_then(crate::medals::Medal::from_id)
        {
            lines.push(format!("Came with the {} medal", medal.name()));
        } else if let Some(card) = &unlock.card {
            lines.push(format!(
                "Came with the card {}",
                crate::cards::faces::name_of(card)
            ));
        } else {
            let note = crate::medals::plain_note(&unlock.note);
            if !note.trim().is_empty() {
                lines.push(note.trim().to_owned());
            }
        }
        let date = crate::medals::date_text(unlock.granted);
        if !date.is_empty() {
            lines.push(format!("First: {date}"));
        }
    };
    match item.thing {
        Thing::Kyber(id) => {
            if let Some(skin) = unlockables::find(id) {
                let mut first = format!("Blade {}", skin.number_text());
                if !skin.smith.is_empty() {
                    first.push_str(&format!(" \u{b7} by {}", skin.smith));
                }
                if skin.chroma {
                    first.push_str(" \u{b7} takes your colour");
                }
                lines.push(first);
            }
            came(id, &mut lines);
        }
        Thing::Finisher(id) => {
            if let Some(finisher) = crate::finishers::get(id) {
                lines.push(finisher.description.to_owned());
            }
            came(id, &mut lines);
        }
        Thing::Tag(id) => {
            if let Some(tag) = crate::tags::get(id) {
                lines.push(tag.collection().heading.to_owned());
                if tag.free {
                    lines.push("Every player starts with it.".to_owned());
                }
            }
            if !item.note.is_empty() && item.note != "Starter" {
                lines.push(format!("Finishes: {}", item.note));
            }
            came(id, &mut lines);
        }
        Thing::GifTags => lines.push("Made from blank tags on the Tags page.".to_owned()),
        Thing::Holocron(slot) | Thing::Crystal(slot) => {
            let tier = crate::holocrons::slot(slot);
            if tier.is_kyber() {
                lines.push(crate::kyber::inside(tier.index));
            } else if let Some((least, most)) = crate::cards::CREDITS.get(tier.index) {
                lines.push(crate::terms::say_owned(format!(
                    "Opening one brings a card and {least} to {most} credits."
                )));
            }
            lines.push(crate::terms::say(tier.about()).into_owned());
        }
        Thing::Card {
            first,
            finish,
            face,
            ..
        } => {
            let finish = &crate::cards::FINISHES[finish.min(2)];
            let mut about = finish.about.chars();
            let about: String = about
                .next()
                .map(|first| first.to_lowercase().chain(about).collect())
                .unwrap_or_default();
            lines.push(format!("{} finish: {about}", finish.name));
            if face.is_none() && item.note.starts_with("Sealed") {
                lines.push("Sealed: its face is chosen when you reveal it on Cards.".to_owned());
            }
            let date = crate::medals::date_text(first);
            if !date.is_empty() {
                lines.push(format!("First found {date}"));
            }
        }
    }
    lines
}

/// "You hold 3 copies · 2 spare": what is held of `item`.
fn held_words(item: &Item, standing: &Standing) -> String {
    let count = item.count;
    let things =
        match item.thing {
            Thing::Kyber(_) | Thing::Tag(_) | Thing::Card { .. } | Thing::Finisher(_) => {
                if count == 1 { "copy" } else { "copies" }
            }
            Thing::GifTags => {
                if count == 1 {
                    "GIF tag"
                } else {
                    "GIF tags"
                }
            }
            Thing::Holocron(_) | Thing::Crystal(_) => "sealed",
        };
    let mut words = format!("You hold {} {things}", crate::cards::grouped(count));
    if matches!(item.thing, Thing::Kyber(_)) {
        let spare = standing.own.saturating_sub(1);
        if spare > 0 && !standing.locked {
            words.push_str(&format!(" \u{b7} {spare} spare"));
        }
    }
    if standing.worn {
        words.push_str(match item.thing {
            Thing::Tag(_) => " \u{b7} in use",
            _ => " \u{b7} worn",
        });
    }
    words
}

/// The holocron or dormant kyber tier at `slot`'s picture over the square of side
/// `2 * half` centred on `centre` (frame pixels): its icon, or its gem where the icon did
/// not load.
fn holocron_icon(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    slot: usize,
    [x, y]: [f32; 2],
    half: f32,
) {
    use crate::holocrons::{gem, icons};
    if icons::is_loaded(slot) {
        let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: frame.rect(x - half, y - half, half * 2.0, half * 2.0),
            texture: icons::texture(slot),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
    } else {
        gem::draw(
            canvas.draw_list_mut(),
            frame.point(x, y),
            half * frame.s,
            crate::holocrons::slot(slot).colour,
            1.0,
            gem::ROWS,
        );
    }
}
