//! The Inventory tab's look, in the SJK UI's: its headline, the sort's segments with the
//! search field at the list's right, then the list, a heading to each kind (its name, how
//! many kinds and how many in all) and a row to each thing held (a bar and its name in its
//! tier's colour, the tier, a note and how many), a bar at the right when it scrolls, and
//! the words of an empty list or one not known yet.

use super::inventory::{Place, ROW_BASE, ROWS_SHOWN, SEARCH_TOKEN, SORT_BASE, Section, Sort};
use super::view::{LEFT_X, RIGHT_X};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The controls' line (its middle), the list's top and a place's height.
const CONTROLS_Y: f32 = 320.0;
const LIST_Y: f32 = 352.0;
const ROW_HEIGHT: f32 = 38.0;
/// The search field's width and height.
const SEARCH_WIDTH: f32 = 380.0;
const SEARCH_HEIGHT: f32 = 38.0;
/// The columns, from the list's left: the name, the tier and the note; how many is at the
/// right.
const NAME_X: f32 = 40.0;
const TIER_X: f32 = 720.0;
const NOTE_X: f32 = 900.0;
const COUNT_WIDTH: f32 = 220.0;

impl Panel {
    /// The Inventory tab.
    pub(super) fn inventory_tab(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        self.follow_inventory(inputs);
        self.lead(
            frame,
            "Inventory",
            "Everything you own, as the SJK hub keeps it: currencies, sealed holocrons and dormant kybers, kybers, tags and cards.",
        );
        let labels: Vec<&str> = Sort::ALL.iter().map(|sort| sort.label()).collect();
        let right = LEFT_X + kit::segments_width(&labels);
        kit::segments(
            &mut self.ui,
            frame,
            right,
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
                RIGHT_X - SEARCH_WIDTH,
                CONTROLS_Y - SEARCH_HEIGHT * 0.5,
                SEARCH_WIDTH,
                SEARCH_HEIGHT,
            ],
            &self.inventory.query,
            self.inventory.search_focus,
            "Search your things",
            found,
            SEARCH_TOKEN,
        );
        if self.inventory.places.is_empty() {
            let (headline, line) = match inputs.holdings.reason() {
                Some(reason) => ("Inventory unavailable", reason),
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
                } => {
                    // Credits and shards do not add up: the currencies say their kinds only.
                    let quantity = (section != Section::Currencies).then_some(quantity);
                    self.inventory_heading(frame, y, &section.label(), kinds, quantity);
                }
                Place::Item(index) => self.inventory_row(frame, slot, y, index),
            }
        }
        let places = self.inventory.places.len();
        if places > ROWS_SHOWN {
            let track = ROWS_SHOWN as f32 * ROW_HEIGHT;
            let thumb = (track * ROWS_SHOWN as f32 / places as f32).max(24.0);
            let along = (track - thumb) * first as f32 / (places - ROWS_SHOWN) as f32;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(RIGHT_X + 10.0, LIST_Y + along, 4.0, thumb),
                radius: 2.0 * frame.s,
                color: color::alpha(color::HOLO, 0.35),
            });
        }
    }

    /// The words over an empty list.
    fn inventory_empty(&mut self, frame: &Frame, headline: &str, line: &str) {
        let s = frame.s;
        let width = RIGHT_X - LEFT_X;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(LEFT_X, 520.0, width, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X, 566.0, width, 26.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Center,
        );
    }

    /// A kind's heading, its top at `y`: its name, a rule, how many kinds and (`quantity`,
    /// none for the currencies) how many in all.
    fn inventory_heading(
        &mut self,
        frame: &Frame,
        y: f32,
        label: &str,
        kinds: usize,
        quantity: Option<u64>,
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
        let summary = match quantity {
            Some(quantity) => format!("{kinds}, {} in all", crate::cards::grouped(quantity)),
            None => kinds,
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{summary}"),
            frame.rect(RIGHT_X - 500.0, middle - 12.0, 484.0, 24.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(LEFT_X, y + ROW_HEIGHT - 3.0, RIGHT_X - LEFT_X, 1.0),
            color: color::alpha(color::HOLO, 0.22),
        });
    }

    /// Item `index` in place `slot`, its top at `y`.
    fn inventory_row(&mut self, frame: &Frame, slot: usize, y: f32, index: usize) {
        let s = frame.s;
        let token = ROW_BASE + slot as u16;
        let chosen = self.inventory.first + slot == self.inventory.chosen;
        let hovered = self.ui.token_hovered(token);
        let Some(item) = self.inventory.items.get(index) else {
            return;
        };
        let band = [LEFT_X, y + 2.0, RIGHT_X - LEFT_X, ROW_HEIGHT - 4.0];
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(band[0], band[1], band[2], band[3]),
            radius: 8.0 * s,
            color: if chosen {
                color::alpha(color::GOLD, 0.12)
            } else if hovered {
                color::alpha(color::HOLO, 0.08)
            } else {
                color::alpha(color::SPACE, 0.62)
            },
        });
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(band[0], band[1], band[2], band[3]),
                radius: 8.0 * s,
                width: 1.2 * s,
                color: color::alpha(color::GOLD_BRIGHT, 0.7),
            });
        }
        let middle = y + ROW_HEIGHT * 0.5;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(LEFT_X + 14.0, middle - 9.0, 5.0, 18.0),
            radius: 2.5 * s,
            color: item.colour,
        });
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", item.name),
            frame.rect(LEFT_X + NAME_X, middle - 12.0, TIER_X - NAME_X - 16.0, 24.0),
            18.0 * s,
            item.colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if !item.tier.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", item.tier),
                frame.rect(LEFT_X + TIER_X, middle - 12.0, NOTE_X - TIER_X - 16.0, 24.0),
                16.0 * s,
                color::alpha(item.colour, 0.85),
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
                    RIGHT_X - COUNT_WIDTH - LEFT_X - NOTE_X - 16.0,
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
                RIGHT_X - COUNT_WIDTH,
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
}
