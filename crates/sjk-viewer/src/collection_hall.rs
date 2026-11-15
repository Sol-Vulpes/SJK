//! The Collection's hall (15/11/2026, Sol's pick of "The hall" from the Collection's
//! design canvas, `docs/sjk-ui.md`, Collection): the page the Collection opens on, a
//! grid of large tiles, one to each kind (Shaders across two cells, Finishers, Tags, Toys,
//! Medals, Achievements, Cards, Holocrons, Forge, Inventory, History, and the Saber Lab
//! while the lab is on), each with what the client already knows of it: the tab's count,
//! "2 new", "3 to open", the deal of the day, the toys on, the newest History line. A
//! summary the client does not know is left out, never guessed. The player's model stands
//! on the right in the whole look worn, named under it, with Loadouts (P).
//!
//! The arrows move across the tiles, Enter (or a click) opens a kind: today's tab, under
//! a breadcrumb ("Collection › Shaders") in place of the row of tabs, where Ctrl+Tab still
//! steps to the next kind and Escape comes back here; Escape here leaves the Collection.
//! [`CVAR`] (`ui_collectionHall`, on by default) opens the Collection here, or off on the
//! kind used last; the commands that name a kind (`unlockables`, `holocrons`,
//! `achievements`) open it directly.
//!
//! The grid reflows to the window rather than scaling: its frame is 1080 lines tall and as
//! wide as the window ([`hall_frame`]), three columns under 1700 of its pixels across
//! (4:3), four to 2200 (16:9), six past (21:9); a tile is at least [`TILE_MIN_HEIGHT`]
//! tall times the menu text size (`ui_textScale`), and when the rows do not fit the grid
//! scrolls by rows ([`layout`]).

use super::view::{Count, counts};
use super::*;
use crate::menu::sjk::{Frame, color, fade, fade_across, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};

/// The setting: open the Collection on the hall (1, the default) or on the kind used
/// last (0).
pub(crate) const CVAR: &str = "ui_collectionHall";

/// The tiles' tokens, by their place in [`tiles`]; clear of the breadcrumb's.
pub(super) const TILE_BASE: u16 = 2_100;
const TILES_MAX: usize = 16;
const _: () = assert!(TILE_BASE > crate::profile_hub::HALL_TOKEN);
const _: () = assert!(TILE_BASE as usize + TILES_MAX <= 2_400);

/// Where the grid stands (hall frame pixels): its left edge, its top under the title and
/// its foot over the keys' line, the gap between tiles.
const GRID_X: f32 = 96.0;
const GRID_Y: f32 = 176.0;
const GRID_BOTTOM: f32 = 972.0;
const GAP: f32 = 18.0;
/// The model's column on the right: its width, its margin to the window's right edge and
/// the room between it and the grid.
const STAGE_WIDTH: f32 = 300.0;
/// The column widens with a wider window, to this at most (a 21:9 window's model stands
/// larger than its 300), and the camera never puts the model further right than
/// [`STAGE_SHARE_MOST`] of the window across, or it is cut by the window's edge.
const STAGE_WIDTH_MOST: f32 = 480.0;
const STAGE_SHARE_MOST: f32 = 0.88;

/// The model's column's width in a hall frame `width` pixels across.
fn stage_width(width: f32) -> f32 {
    (width * 0.19).clamp(STAGE_WIDTH, STAGE_WIDTH_MOST)
}
const STAGE_RIGHT: f32 = 60.0;
const STAGE_GAP: f32 = 80.0;
/// The shortest a tile stands at the ordinary text size: its name, two lines and its bar.
pub(super) const TILE_MIN_HEIGHT: f32 = 156.0;
/// Under this width (hall frame pixels) the grid has three columns, past [`WIDE`] six,
/// four between.
const NARROW: f32 = 1_700.0;
const WIDE: f32 = 2_200.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;

/// A tile of the hall: a kind of the Collection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tile {
    Shaders,
    /// The Shaders tab's Finishers pill.
    Finishers,
    Tags,
    Toys,
    Medals,
    Achievements,
    Cards,
    Holocrons,
    Forge,
    Inventory,
    History,
    SaberLab,
}

/// The hall's tiles, in reading order.
const TILES: [Tile; 11] = [
    Tile::Shaders,
    Tile::Finishers,
    Tile::Tags,
    Tile::Toys,
    Tile::Medals,
    Tile::Achievements,
    Tile::Cards,
    Tile::Holocrons,
    Tile::Forge,
    Tile::Inventory,
    Tile::History,
];
/// The same while the Saber Lab is on: its tile last.
const TILES_LAB: [Tile; 12] = [
    Tile::Shaders,
    Tile::Finishers,
    Tile::Tags,
    Tile::Toys,
    Tile::Medals,
    Tile::Achievements,
    Tile::Cards,
    Tile::Holocrons,
    Tile::Forge,
    Tile::Inventory,
    Tile::History,
    Tile::SaberLab,
];
const _: () = assert!(TILES_LAB.len() <= TILES_MAX);

/// The tiles the hall shows now.
pub(crate) fn tiles() -> &'static [Tile] {
    if crate::saber_lab::active() {
        &TILES_LAB
    } else {
        &TILES
    }
}

impl Tile {
    /// The tab it opens.
    pub(crate) const fn tab(self) -> Tab {
        match self {
            Self::Shaders | Self::Finishers => Tab::Shaders,
            Self::Tags => Tab::Tags,
            Self::Toys => Tab::Toys,
            Self::Medals => Tab::Medals,
            Self::Achievements => Tab::Achievements,
            Self::Cards => Tab::Cards,
            Self::Holocrons => Tab::Holocrons,
            Self::Forge => Tab::Forge,
            Self::Inventory => Tab::Inventory,
            Self::History => Tab::History,
            Self::SaberLab => Tab::SaberLab,
        }
    }

    /// How many of the grid's cells it takes across.
    pub(crate) const fn span(self) -> usize {
        match self {
            Self::Shaders => 2,
            _ => 1,
        }
    }

    /// Its name, as the player reads it (`crate::terms`).
    pub(crate) fn label(self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Finishers => "Finishers".into(),
            tile => crate::terms::say(tile.tab().label()),
        }
    }
}

/// The hall's frame for a window of `viewport`: 1080 lines tall at the menus' scale and
/// as wide as the window, its left edge the window's (a 16:9 frame scaled to fit would
/// leave a 21:9 window's sides bare and shrink a 4:3 one's words); and its width in its
/// own pixels.
pub(super) fn hall_frame(viewport: [f32; 2]) -> (Frame, f32) {
    let s = crate::menu::sjk::scale(viewport);
    let frame = Frame {
        s,
        origin: [0.0, (viewport[1] - 1_080.0 * s) * 0.5],
    };
    (frame, viewport[0] / s.max(0.01))
}

/// Where the middle of the model's column stands across a window of `viewport`, as a share
/// of its width: the backdrop camera turns the model there
/// ([`crate::menu_backdrop::window_yaw`]).
pub(crate) fn stage_share(viewport: [f32; 2]) -> f32 {
    let (_, width) = hall_frame(viewport);
    ((width - STAGE_RIGHT - stage_width(width) * 0.5) / width.max(1.0)).clamp(0.5, STAGE_SHARE_MOST)
}

/// Where a tile stands in the grid: its row, its first column and how many it takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Place {
    pub(crate) row: usize,
    pub(crate) column: usize,
    pub(crate) span: usize,
}

/// The grid laid out for a window: its columns and rows, the rows shown at once, a cell's
/// size and each tile's place (hall frame pixels).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub(crate) columns: usize,
    pub(crate) rows: usize,
    pub(crate) shown_rows: usize,
    pub(crate) cell: [f32; 2],
    pub(crate) places: Vec<Place>,
    /// The model's column: its left edge and width.
    pub(crate) stage: [f32; 2],
    /// The grid's right edge.
    pub(crate) right: f32,
}

/// The columns a hall frame `width` pixels across gets.
pub(crate) fn columns(width: f32) -> usize {
    if width < NARROW {
        3
    } else if width > WIDE {
        6
    } else {
        4
    }
}

/// Lay tiles `spans` wide (cells each, in reading order) out in a hall frame `width`
/// pixels across, the menu text drawn `text_scale` times its size: row by row, a tile
/// that does not fit the rest of a row going to the next; the rows share the grid's
/// height, each at least [`TILE_MIN_HEIGHT`] times `text_scale`, and those that do not fit
/// are scrolled to.
pub(crate) fn layout(width: f32, spans: &[usize], text_scale: f32) -> Layout {
    let columns = columns(width);
    let mut places = Vec::with_capacity(spans.len());
    let (mut row, mut column) = (0, 0);
    for &span in spans {
        let span = span.clamp(1, columns);
        if column + span > columns {
            row += 1;
            column = 0;
        }
        places.push(Place { row, column, span });
        column += span;
        if column == columns {
            row += 1;
            column = 0;
        }
    }
    let rows = (if column == 0 { row } else { row + 1 }).max(1);
    let stage_width = stage_width(width);
    let stage_x = width - STAGE_RIGHT - stage_width;
    let right = stage_x - STAGE_GAP;
    let cell_width = ((right - GRID_X - GAP * (columns - 1) as f32) / columns as f32).max(1.0);
    let room = GRID_BOTTOM - GRID_Y;
    let least = TILE_MIN_HEIGHT * text_scale.max(0.1);
    let fill = (room - GAP * (rows - 1) as f32) / rows as f32;
    let shown_rows = if fill >= least {
        rows
    } else {
        (((room + GAP) / (least + GAP)).floor() as usize).clamp(1, rows)
    };
    let height = (room - GAP * (shown_rows - 1) as f32) / shown_rows as f32;
    Layout {
        columns,
        rows,
        shown_rows,
        cell: [cell_width, height],
        places,
        stage: [stage_x, stage_width],
        right,
    }
}

impl Layout {
    /// Where tile `index` stands with row `first` at the top (hall frame pixels), if it is
    /// among the rows shown.
    pub(crate) fn rect(&self, index: usize, first: usize) -> Option<[f32; 4]> {
        let place = self.places.get(index)?;
        let shown = place.row.checked_sub(first)?;
        (shown < self.shown_rows).then(|| {
            let [width, height] = self.cell;
            [
                GRID_X + place.column as f32 * (width + GAP),
                GRID_Y + shown as f32 * (height + GAP),
                width * place.span as f32 + GAP * (place.span - 1) as f32,
                height,
            ]
        })
    }

    /// The row to show first so that tile `chosen`'s row is in view, from `first`.
    pub(crate) fn keep_in_view(&self, first: usize, chosen: usize) -> usize {
        let Some(row) = self.places.get(chosen).map(|place| place.row) else {
            return 0;
        };
        let most = self.rows - self.shown_rows;
        if row < first {
            row
        } else if row >= first + self.shown_rows {
            row + 1 - self.shown_rows
        } else {
            first
        }
        .min(most)
    }

    /// The tile `by` on from `chosen` in reading order, stopping at the ends.
    pub(crate) fn across(&self, chosen: usize, by: isize) -> usize {
        let last = self.places.len().saturating_sub(1) as isize;
        (chosen as isize + by).clamp(0, last) as usize
    }

    /// The tile in the row under `chosen` (`down`) or over it, the one whose middle is
    /// nearest to its middle; `chosen` itself at the first or last row.
    pub(crate) fn vertical(&self, chosen: usize, down: bool) -> usize {
        let Some(place) = self.places.get(chosen) else {
            return chosen;
        };
        let middle = |place: &Place| place.column as f32 + place.span as f32 * 0.5;
        let Some(row) = (if down {
            Some(place.row + 1)
        } else {
            place.row.checked_sub(1)
        }) else {
            return chosen;
        };
        let here = middle(place);
        self.places
            .iter()
            .enumerate()
            .filter(|(_, other)| other.row == row)
            .min_by(|a, b| {
                (middle(a.1) - here)
                    .abs()
                    .total_cmp(&(middle(b.1) - here).abs())
            })
            .map_or(chosen, |(index, _)| index)
    }
}

/// What a tile says under its name: its count after the name, its lines (each with its
/// colour), how much of the kind is held (its bar; `None` for none) and whether something
/// waits there (the Forge's gold dot).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Summary {
    pub(crate) count: String,
    pub(crate) lines: Vec<(String, Color)>,
    pub(crate) share: Option<f32>,
    pub(crate) alert: bool,
}

/// The hall's state: the tile chosen, the first row shown, and what the last frame laid
/// out and summed up.
#[derive(Debug, Default)]
pub(crate) struct Hall {
    pub(super) chosen: usize,
    pub(super) first: usize,
    pub(super) layout: Option<Layout>,
    summaries: Vec<Summary>,
    /// When the summaries were made (the page's seconds): made again four times a second.
    built_at: Option<f32>,
}

impl Hall {
    /// The tile chosen, as the tiles stand now.
    pub(crate) fn chosen_tile(&self) -> Tile {
        let tiles = tiles();
        tiles[self.chosen.min(tiles.len() - 1)]
    }
}

/// "3 of 8" share of a count with a set total.
fn share(count: Count) -> Option<f32> {
    let held = count.held?;
    (count.total > 0).then(|| (held.min(count.total) as f32 / count.total as f32).clamp(0.0, 1.0))
}

/// `words`, the first `most` of them joined with ", ", and how many more.
fn listed(words: &[String], most: usize) -> String {
    let mut line = words
        .iter()
        .take(most)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if words.len() > most {
        line.push_str(&format!(" and {} more", words.len() - most));
    }
    line
}

/// "1 thing", "2 things".
fn counted(count: u64, one: &str, many: &str) -> String {
    format!(
        "{} {}",
        crate::cards::grouped(count),
        if count == 1 { one } else { many }
    )
}

impl Panel {
    /// Whether the hall shows.
    #[cfg(test)]
    pub(crate) fn on_hall(&self) -> bool {
        self.hall
    }

    /// The tile of the kind on show: the one the hall chooses when the player comes back.
    fn tile_of_kind(&self) -> Option<usize> {
        let tile = match self.tab {
            Tab::Shaders if self.finishers_shown => Tile::Finishers,
            tab => *tiles().iter().find(|tile| tile.tab() == tab)?,
        };
        tiles().iter().position(|each| *each == tile)
    }

    /// Show the hall, its tile on the kind that showed (the first tile until one has).
    pub(super) fn enter_hall(&mut self) {
        if !self.hall {
            if let Some(index) = self.tile_of_kind().filter(|_| self.kind_shown) {
                self.hall_state.chosen = index;
            }
            self.leave_viewing();
            self.leave_search();
            self.forge.leave_search();
            self.inventory.leave_search();
            self.forge.ask = None;
            self.card_inspect = false;
            self.bind_wait = None;
            self.new_note = false;
            self.extras_note = false;
        }
        self.hall = true;
        // Made again at once: the kind just left may have changed them.
        self.hall_state.built_at = None;
    }

    /// Open tile `index`'s kind: its tab (the Finishers pill on Shaders, the Holocrons
    /// page through the console).
    pub(super) fn open_tile(&mut self, index: usize) -> PanelAction {
        let Some(&tile) = tiles().get(index) else {
            return PanelAction::None;
        };
        self.hall_state.chosen = index;
        ui_cues_click();
        if tile == Tile::Holocrons {
            return PanelAction::Hub(Tab::Holocrons);
        }
        self.show(tile.tab());
        match tile {
            Tile::Finishers => self.finishers_shown = true,
            Tile::Shaders => self.finishers_shown = false,
            _ => {}
        }
        PanelAction::None
    }

    /// A pressed key on the hall: the arrows (Page Up, Page Down, Home, End, Tab) choose,
    /// Enter or Space opens, P shows the Loadouts card, Escape leaves.
    pub(super) fn hall_key(&mut self, key: KeyCode, shift: bool, control: bool) -> PanelAction {
        let count = tiles().len();
        let layout = self.hall_state.layout.clone().unwrap_or_else(|| {
            layout(
                1_920.0,
                &tiles().iter().map(|tile| tile.span()).collect::<Vec<_>>(),
                1.0,
            )
        });
        let chosen = self.hall_state.chosen.min(count - 1);
        let next = match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                return self.open_tile(chosen);
            }
            KeyCode::KeyP if !control => {
                self.loadouts.open();
                return PanelAction::None;
            }
            KeyCode::ArrowLeft => layout.across(chosen, -1),
            KeyCode::ArrowRight => layout.across(chosen, 1),
            KeyCode::ArrowUp => layout.vertical(chosen, false),
            KeyCode::ArrowDown => layout.vertical(chosen, true),
            KeyCode::Tab if !control => {
                if shift {
                    (chosen + count - 1) % count
                } else {
                    (chosen + 1) % count
                }
            }
            KeyCode::PageUp => {
                (0..layout.shown_rows).fold(chosen, |at, _| layout.vertical(at, false))
            }
            KeyCode::PageDown => {
                (0..layout.shown_rows).fold(chosen, |at, _| layout.vertical(at, true))
            }
            KeyCode::Home => 0,
            KeyCode::End => count - 1,
            _ => return PanelAction::None,
        };
        self.hall_state.chosen = next;
        self.hall_state.first = layout.keep_in_view(self.hall_state.first, next);
        PanelAction::None
    }

    /// A pointer event on the hall (the Loadouts card closed): resting on a tile chooses
    /// it, a click opens it, the wheel scrolls a row a notch.
    pub(super) fn hall_pointer(&mut self, event: InputEvent) -> PanelAction {
        if let InputEvent::PointerWheel { delta, .. } = event {
            if let Some(layout) = &self.hall_state.layout {
                let most = layout.rows - layout.shown_rows;
                let first = self.hall_state.first;
                self.hall_state.first = if delta.y > 0.0 {
                    first.saturating_sub(1)
                } else if delta.y < 0.0 {
                    (first + 1).min(most)
                } else {
                    first
                };
            }
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        let Some(token) = event.token else {
            return PanelAction::None;
        };
        if let Some(index) = token
            .checked_sub(TILE_BASE)
            .map(usize::from)
            .filter(|index| *index < tiles().len())
        {
            return match event.kind {
                UiEventKind::Activate => self.open_tile(index),
                UiEventKind::HoverEnter | UiEventKind::Hover => {
                    self.hall_state.chosen = index;
                    PanelAction::None
                }
                _ => PanelAction::None,
            };
        }
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match token {
            BACK_TOKEN => PanelAction::Close,
            loadouts::OPEN_TOKEN => {
                self.loadouts.open();
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// What each tile says this frame, from `inputs` (`counts`, the tabs' counts).
    fn summaries(&mut self, inputs: &Inputs<'_>, counts: &[Count]) -> Vec<Summary> {
        self.follow_marks(inputs);
        let me = inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.me.as_ref());
        let known = inputs.holdings.reason().is_none();
        tiles()
            .iter()
            .map(|tile| {
                let count = |tab: Tab| counts.get(tab.index()).copied().unwrap_or_default();
                let mut summary = Summary::default();
                match tile {
                    Tile::Shaders => {
                        let shaders = count(Tab::Shaders);
                        summary.count = shaders.label();
                        summary.share = share(shaders);
                        let new = self.marks.new_count();
                        if new > 0 {
                            summary.count = format!("{} \u{b7} {new} new", summary.count);
                        }
                        let owned = match inputs.holdings {
                            Holdings::Known(owned) => owned,
                            _ => &[],
                        };
                        let worn =
                            crate::shader_marks::worn_of(inputs.setting, inputs.second, owned);
                        match worn[0].and_then(unlockables::blade_skin) {
                            Some(skin) => summary
                                .lines
                                .push((format!("{} \u{b7} worn", skin.name), skin.tier.colour())),
                            None => summary
                                .lines
                                .push(("Stock blade \u{b7} worn".to_owned(), inputs.stock)),
                        }
                        for skin in self
                            .marks
                            .new_ids()
                            .filter_map(unlockables::blade_skin)
                            .take(2)
                        {
                            summary
                                .lines
                                .push((format!("{} \u{b7} new", skin.name), skin.tier.colour()));
                        }
                    }
                    Tile::Finishers => {
                        let held = |id: &str| {
                            inputs.holdings.debug() || inputs.holdings.unlock(id).is_some()
                        };
                        let total = crate::finishers::CATALOGUE.len();
                        if known {
                            let owned = crate::finishers::CATALOGUE
                                .iter()
                                .filter(|finisher| held(finisher.id))
                                .count();
                            summary.count = format!("{owned}/{total}");
                            summary.share = Some(owned as f32 / total.max(1) as f32);
                        }
                        match crate::finishers::own(inputs.finisher, held)
                            .and_then(crate::finishers::get)
                        {
                            Some(finisher) => summary.lines.push((
                                format!("Wearing {}", finisher.name),
                                finisher.tier.colour(),
                            )),
                            None => summary.lines.push(("None worn".to_owned(), color::MUTED)),
                        }
                        summary.lines.push((
                            "What a saber kill does to the body".to_owned(),
                            color::QUIET,
                        ));
                    }
                    Tile::Tags => {
                        let tags = count(Tab::Tags);
                        summary.count = tags.label();
                        summary.share = share(tags);
                        if let Some(tag) = crate::tags::get(inputs.tag.trim()) {
                            summary
                                .lines
                                .push((format!("Chosen: {}", tag.name), tag.tier.colour()));
                            if let Some(finish) = crate::cards::FINISHES
                                .iter()
                                .find(|finish| finish.id == inputs.tag_finish && finish.index > 0)
                            {
                                summary
                                    .lines
                                    .push((format!("{} finish", finish.name), color::MUTED));
                            }
                        }
                    }
                    Tile::Toys => {
                        summary.count = count(Tab::Toys).label();
                        let on = [
                            inputs.illuminate,
                            inputs.cardboard_box,
                            inputs.order66,
                            inputs.hax,
                            inputs.ragdoll,
                            inputs.drone,
                            inputs.boombox,
                            inputs.monitor,
                            inputs.box_hunt,
                            inputs.banana,
                            inputs.fishing,
                            inputs.wheel_of_doom,
                            inputs.mining,
                            inputs.quests,
                        ];
                        let names: Vec<String> = on
                            .iter()
                            .enumerate()
                            .filter(|(_, on)| **on)
                            .map(|(toy, _)| toys::name(toy).to_owned())
                            .collect();
                        summary.lines.push(if names.is_empty() {
                            ("None on now".to_owned(), color::MUTED)
                        } else {
                            (
                                format!("{} on now: {}", names.len(), listed(&names, 3)),
                                color::GOLD,
                            )
                        });
                    }
                    Tile::Medals => {
                        let medals = count(Tab::Medals);
                        summary.count = medals.label();
                        summary.share = share(medals);
                        if let Some(me) = me {
                            let names: Vec<String> = crate::medals::awards(&me.medals)
                                .iter()
                                .map(crate::medals::Award::label)
                                .collect();
                            if !names.is_empty() {
                                summary.lines.push((listed(&names, 2), color::GOLD));
                            }
                        }
                    }
                    Tile::Achievements => {
                        let achievements = count(Tab::Achievements);
                        summary.count = achievements.label();
                        summary.share = share(achievements);
                        let next = inputs
                            .standings
                            .iter()
                            .filter(|standing| {
                                standing.listed()
                                    && standing.unlocked.is_none()
                                    && !standing.secret()
                            })
                            .max_by(|a, b| a.fraction().total_cmp(&b.fraction()));
                        if let Some(next) = next {
                            summary
                                .lines
                                .push((format!("Next up: {}", next.name()), color::TEXT));
                            summary.lines.push((
                                format!(
                                    "{} / {}",
                                    crate::cards::grouped(next.progress.min(next.kind.goal)),
                                    crate::cards::grouped(next.kind.goal)
                                ),
                                color::MUTED,
                            ));
                        }
                    }
                    Tile::Cards => {
                        let cards = count(Tab::Cards);
                        if let Some(held) = cards.held {
                            summary.count = cards.label();
                            summary.lines.push((
                                if held == 0 {
                                    "No cards yet: holocrons bring them".to_owned()
                                } else {
                                    counted(held as u64, "card held", "cards held")
                                },
                                color::MUTED,
                            ));
                        }
                    }
                    Tile::Holocrons => {
                        if let Some(me) = me {
                            let held = crate::holocrons::counts_of(&me.holocron_counts);
                            let total: u32 = held.iter().sum();
                            summary.count = if total > 0 {
                                format!("{} to open", crate::cards::grouped(u64::from(total)))
                            } else {
                                "None to open".to_owned()
                            };
                            for (tier, held) in crate::holocrons::TIERS.iter().zip(held).rev() {
                                if held > 0 && summary.lines.len() < 3 {
                                    summary.lines.push((
                                        format!("{held} {}", crate::terms::say(tier.name)),
                                        tier.colour,
                                    ));
                                }
                            }
                        }
                    }
                    Tile::Forge => {
                        summary.alert = crate::forge_alert::shown();
                        if let Some(shards) = me.map(|me| me.shards) {
                            summary.count = counted(shards, "kyber shard", "kyber shards");
                        }
                        if let Some(deal) = inputs
                            .snapshot
                            .filter(|_| inputs.enabled)
                            .and_then(|snapshot| snapshot.shop.as_ref())
                            .filter(|shop| shop.message.is_empty())
                            .and_then(|shop| shop.day.offers.iter().find(|offer| offer.deal))
                        {
                            let skin = unlockables::blade_skin(&deal.id);
                            summary.lines.push((
                                format!(
                                    "Deal of the day: {}",
                                    skin.map_or(deal.name.as_str(), |skin| skin.name)
                                ),
                                skin.map_or(color::GOLD, |skin| skin.tier.colour()),
                            ));
                        }
                        if known {
                            let (copies, _, _) = forge::duplicates(&forge::owned(
                                &inputs.holdings,
                                false,
                                &self.marks,
                            ));
                            if copies > 0 {
                                summary.lines.push((
                                    counted(
                                        u64::from(copies),
                                        "duplicate to break down",
                                        "duplicates to break down",
                                    ),
                                    color::MUTED,
                                ));
                            }
                        }
                    }
                    Tile::Inventory => {
                        if let (Some(me), true) = (me, known) {
                            summary.lines.push((
                                format!(
                                    "{} credits, {}",
                                    crate::cards::grouped(me.credits),
                                    counted(me.shards, "kyber shard", "kyber shards")
                                ),
                                color::TEXT,
                            ));
                            let owned = match inputs.holdings {
                                Holdings::Known(owned) => owned,
                                _ => &[],
                            };
                            let shaders = owned
                                .iter()
                                .filter(|unlock| unlockables::blade_skin(&unlock.id).is_some())
                                .count() as u64;
                            let tags = counts
                                .get(Tab::Tags.index())
                                .and_then(|count| count.held)
                                .unwrap_or(0) as u64;
                            let crystals = me
                                .holocrons
                                .iter()
                                .filter(|holocron| {
                                    holocron.is_sealed()
                                        && crate::kyber::from_id(&holocron.tier).is_some()
                                })
                                .count() as u64;
                            let mut parts = vec![
                                counted(shaders, "shader", "shaders"),
                                counted(tags, "tag", "tags"),
                            ];
                            if crystals > 0 {
                                parts.push(counted(crystals, "kyber crystal", "kyber crystals"));
                            }
                            summary.lines.push((parts.join(", "), color::MUTED));
                        }
                    }
                    Tile::History => {
                        if let Some(line) = inputs
                            .snapshot
                            .filter(|_| inputs.enabled)
                            .and_then(|snapshot| snapshot.history.as_ref())
                            .and_then(|history| history.lines.first())
                        {
                            let row = history::row(line);
                            let day: String = row.when.chars().take(5).collect();
                            summary.lines.push((
                                if day.is_empty() {
                                    row.text
                                } else {
                                    format!("{day} \u{b7} {}", row.text)
                                },
                                color::MUTED,
                            ));
                        } else {
                            summary.lines.push((
                                "What came, was opened, spent and traded".to_owned(),
                                color::QUIET,
                            ));
                        }
                    }
                    Tile::SaberLab => summary.lines.push((
                        "On while the lab runs: hold, swing and build a blade".to_owned(),
                        color::MUTED,
                    )),
                }
                summary
            })
            .collect()
    }

    /// The hall, drawn for a window of `viewport` at `seconds`.
    pub(super) fn hall_view(&mut self, inputs: &Inputs<'_>, viewport: [f32; 2], seconds: f32) {
        let (frame, width) = hall_frame(viewport);
        let style = crate::text::style::current();
        let spans: Vec<usize> = tiles().iter().map(|tile| tile.span()).collect();
        let layout = layout(width, &spans, style.scale);
        self.hall_state.chosen = self.hall_state.chosen.min(spans.len() - 1);
        self.hall_state.first = layout.keep_in_view(self.hall_state.first, self.hall_state.chosen);
        let counts = counts(inputs);
        let stale = self
            .hall_state
            .built_at
            .is_none_or(|at| seconds < at || seconds - at > 0.25)
            || self.hall_state.summaries.len() != spans.len();
        if stale {
            self.hall_state.summaries = self.summaries(inputs, &counts);
            self.hall_state.built_at = Some(seconds);
        }
        self.hall_scrims(&frame, viewport, &layout);
        let labels: Vec<String> = counts.iter().map(|count| count.label()).collect();
        let back = if self.hub {
            crate::profile_hub::back_words(self.back)
        } else {
            "Back"
        };
        crate::profile_hub::collection_header(
            &mut self.ui,
            &frame,
            back,
            BACK_TOKEN,
            Tab::Hall,
            (&labels, self.widen),
        );
        // The tally, at the window's right as the page's is at its frame's.
        self.tally(&frame.shifted(width - 1_920.0, 0.0), &counts);
        let first = self.hall_state.first;
        for (index, tile) in tiles().iter().enumerate() {
            let Some(rect) = layout.rect(index, first) else {
                continue;
            };
            let summary = self
                .hall_state
                .summaries
                .get(index)
                .cloned()
                .unwrap_or_default();
            self.tile(&frame, *tile, index, rect, &summary, style.scale);
        }
        if layout.shown_rows < layout.rows {
            self.hall_scroll_bar(&frame, &layout);
        }
        self.hall_stage(&frame, viewport, &layout, inputs);
        self.hall_keys(&frame, width);
        self.hall_state.layout = Some(layout);
    }

    /// The dark behind the grid, clear over the model's column when the model stands
    /// there; the whole page darkened when it does not.
    fn hall_scrims(&mut self, frame: &Frame, viewport: [f32; 2], layout: &Layout) {
        if self.backstage == Backstage::None {
            crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
            return;
        }
        let [width, height] = viewport;
        let space = |alpha| color::alpha(color::SPACE, alpha);
        let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
        let y = |frame_y: f32| frame.point(0.0, frame_y)[1];
        let solid = x(layout.right);
        fade_across(
            &mut self.ui,
            Rect::new(0.0, 0.0, solid, height),
            space(0.92),
            space(0.86),
        );
        fade_across(
            &mut self.ui,
            Rect::new(solid, 0.0, x(layout.stage[0]) - solid, height),
            space(0.86),
            space(0.0),
        );
        fade(
            &mut self.ui,
            Rect::new(0.0, 0.0, width, y(120.0)),
            space(0.8),
            space(0.0),
        );
        fade(
            &mut self.ui,
            Rect::new(0.0, y(880.0), width, height - y(880.0)),
            space(0.0),
            space(0.86),
        );
    }

    /// Tile `tile` (its place `index`) over `rect`: a dark glass card, its name and count,
    /// its lines and its bar; the chosen one framed gold.
    fn tile(
        &mut self,
        frame: &Frame,
        tile: Tile,
        index: usize,
        [x, y, width, height]: [f32; 4],
        summary: &Summary,
        text_scale: f32,
    ) {
        let s = frame.s;
        let token = TILE_BASE + index as u16;
        let chosen = index == self.hall_state.chosen;
        let hovered = self.ui.token_hovered(token);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 16.0 * s,
            color: if chosen {
                Color::new(0.07, 0.1, 0.17, 0.94)
            } else {
                Color::new(0.04, 0.06, 0.12, 0.86)
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(x, y, width, height),
            radius: 16.0 * s,
            width: if chosen { 2.0 } else { 1.2 } * s,
            color: if chosen {
                color::GOLD_BRIGHT
            } else if hovered {
                color::alpha(color::HOLO, 0.55)
            } else {
                color::alpha(color::HOLO, 0.22)
            },
        });
        let pad = 22.0;
        let ts = text_scale.max(0.1);
        let label = tile.label();
        let name_size = 26.0;
        let name_width = super::view::label_width(&label, name_size, self.widen);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(x + pad, y + 18.0, name_width + 20.0, 34.0),
            name_size * s,
            if chosen {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if summary.alert {
            kit::attention_dot(&mut self.ui, frame, [x + pad - 11.0, y + 35.0]);
        }
        if !summary.count.is_empty() {
            let left = x + pad + name_width + 12.0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", summary.count),
                frame.rect(left, y + 22.0, (x + width - pad - left).max(0.0), 26.0),
                17.0 * s,
                if chosen { color::GOLD } else { color::QUIET },
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // The lines under the name, as many as the tile holds over its bar, wrapped to its
        // width at the player's text size.
        let line_size = 17.0;
        let step = line_size * 1.45 * ts;
        let mut top = y + 18.0 + 34.0 * ts.max(1.0) + 10.0;
        let foot = y + height - pad - 8.0;
        let chars = (((width - pad * 2.0) / (line_size * 0.5 * ts)) as usize).max(8);
        'lines: for (words, colour) in &summary.lines {
            for part in crate::menu::sjk::wrap(words, chars) {
                if top + step > foot {
                    break 'lines;
                }
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(x + pad, top, width - pad * 2.0, step),
                    line_size * s,
                    *colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                top += step;
            }
        }
        if let Some(share) = summary.share {
            let bar = [x + pad, y + height - pad - 4.0, width - pad * 2.0, 4.0];
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(bar[0], bar[1], bar[2], bar[3]),
                radius: 2.0 * s,
                color: color::alpha(color::HOLO, 0.18),
            });
            if share > 0.0 {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(bar[0], bar[1], (bar[2] * share).max(bar[3]), bar[3]),
                    radius: 2.0 * s,
                    color: color::GOLD_BRIGHT,
                });
            }
        }
        self.ui.hit_region(token, frame.rect(x, y, width, height));
    }

    /// A thin bar at the grid's right saying which rows show.
    fn hall_scroll_bar(&mut self, frame: &Frame, layout: &Layout) {
        let s = frame.s;
        let x = layout.right + 10.0;
        let room = GRID_BOTTOM - GRID_Y;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, GRID_Y, 4.0, room),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let length = room * layout.shown_rows as f32 / layout.rows as f32;
        let top = GRID_Y + room * self.hall_state.first as f32 / layout.rows as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top, 4.0, length),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.6),
        });
    }

    /// The model's column: "Wearing now" over it, the model in the live preview over a
    /// match (on the menu map's stage it stands there itself), and under it the blade,
    /// finisher and tag worn and Loadouts (P).
    fn hall_stage(
        &mut self,
        frame: &Frame,
        viewport: [f32; 2],
        layout: &Layout,
        inputs: &Inputs<'_>,
    ) {
        let s = frame.s;
        let [x, width] = layout.stage;
        let middle = x + width * 0.5;
        if let Backstage::Preview { ready: true } = self.backstage {
            // The preview is rendered at the page's model area's size in its 16:9 frame:
            // drawn at that size, centred on the column.
            let page = Frame::new(viewport);
            let [_, top, area_width, area_height] = MODEL_AREA;
            let size = [area_width * page.s, area_height * page.s];
            let centre = frame.point(middle, 0.0)[0];
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: Rect::new(
                    centre - size[0] * 0.5,
                    page.point(0.0, top)[1],
                    size[0],
                    size[1],
                ),
                texture: crate::ui_renderer::PREVIEW_TEXTURE,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Wearing now"),
            frame.rect(x, GRID_Y, width, 26.0),
            18.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
        let owned = match inputs.holdings {
            Holdings::Known(owned) => owned,
            _ => &[],
        };
        let worn = crate::shader_marks::worn_of(inputs.setting, inputs.second, owned);
        let (blade, blade_colour) = match worn[0].and_then(unlockables::blade_skin) {
            Some(skin) => (skin.name, skin.tier.colour()),
            None => ("Stock blade", inputs.stock),
        };
        let held = |id: &str| inputs.holdings.debug() || inputs.holdings.unlock(id).is_some();
        let finisher = crate::finishers::own(inputs.finisher, held)
            .and_then(crate::finishers::get)
            .map(|finisher| finisher.name);
        let tag = crate::tags::get(inputs.tag.trim()).map(|tag| tag.name);
        let rest: Vec<&str> = [finisher, tag].into_iter().flatten().collect();
        let ts = crate::text::style::current().scale.max(1.0);
        let lines_top = 952.0 - 12.0 - 26.0 * ts * if rest.is_empty() { 1.0 } else { 2.0 };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{blade}"),
            frame.rect(x - 20.0, lines_top, width + 40.0, 26.0),
            18.0 * s,
            blade_colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        if !rest.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", rest.join(" \u{b7} ")),
                frame.rect(x - 20.0, lines_top + 26.0 * ts, width + 40.0, 26.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        kit::button(
            &mut self.ui,
            frame,
            [middle - 110.0, 952.0, 220.0, kit::CONTROL_HEIGHT],
            "Loadouts  (P)",
            false,
            true,
            false,
            loadouts::OPEN_TOKEN,
        );
    }

    /// The hall's keys, right-aligned at the window's right.
    fn hall_keys(&mut self, frame: &Frame, width: f32) {
        let s = frame.s;
        let open = format!("open {}", self.hall_state.chosen_tile().label());
        let keys: [(&[&str], &str); 4] = [
            (&["Arrows"], "choose"),
            (&["Enter"], &open),
            (&["P"], "loadouts"),
            (&["Esc"], "back"),
        ];
        let gap = 30.0 * s;
        let total: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(width - 96.0, KEYS_Y);
        let mut x = right - total;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }

    /// Choose `tile` on the hall (the Holocrons page's way back comes to its tile).
    pub(crate) fn choose_hall_tile(&mut self, tile: Tile) {
        if let Some(index) = tiles().iter().position(|each| *each == tile) {
            self.hall_state.chosen = index;
        }
    }

    /// The tile chosen on the hall, for a test or a shot.
    #[cfg(test)]
    pub(crate) fn hall_tile(&self) -> Tile {
        self.hall_state.chosen_tile()
    }

    /// Choose tile `index` on the hall, for a world shot.
    #[cfg(test)]
    pub(crate) fn hall_choose_for_shot(&mut self, index: usize) {
        self.hall_state.chosen = index;
    }

    /// Open tile `index` as Enter would, for a world shot.
    #[cfg(test)]
    pub(crate) fn hall_open_for_shot(&mut self, index: usize) -> PanelAction {
        self.open_tile(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans() -> Vec<usize> {
        TILES.iter().map(|tile| tile.span()).collect()
    }

    /// Twelve cells (Shaders takes two) are three by four at 4:3, four by three at 16:9 and
    /// six by two at 21:9: the grid reflows, the tiles keep the menus' size.
    #[test]
    fn the_grid_reflows_to_the_window() {
        for (viewport, columns, rows) in [
            ([1_440.0, 1_080.0], 3, 4),
            ([1_920.0, 1_080.0], 4, 3),
            ([2_560.0, 1_080.0], 6, 2),
            ([3_840.0, 2_160.0], 4, 3),
            ([1_024.0, 768.0], 3, 4),
        ] {
            let (frame, width) = hall_frame(viewport);
            assert!((frame.s - crate::menu::sjk::scale(viewport)).abs() < 1e-6);
            let grid = layout(width, &spans(), 1.0);
            assert_eq!((grid.columns, grid.rows), (columns, rows), "{viewport:?}");
            assert_eq!(grid.shown_rows, rows, "{viewport:?}: every row shows");
            // Shaders first across two cells, every tile inside the grid, none overlapping
            // and the model's column clear on the right, inside the window.
            assert_eq!(grid.places[0].span, 2);
            let rects: Vec<[f32; 4]> = (0..TILES.len())
                .map(|index| grid.rect(index, 0).expect("shown"))
                .collect();
            for (index, [x, y, w, h]) in rects.iter().enumerate() {
                assert!(
                    *x >= GRID_X && x + w <= grid.right + 0.01,
                    "{viewport:?} {index}"
                );
                assert!(
                    *y >= GRID_Y && y + h <= GRID_BOTTOM + 0.01,
                    "{viewport:?} {index}"
                );
                assert!(*h >= TILE_MIN_HEIGHT);
                for [ox, oy, ow, oh] in &rects[index + 1..] {
                    let apart = x + w <= *ox || ox + ow <= *x || y + h <= *oy || oy + oh <= *y;
                    assert!(apart, "{viewport:?}: tile {index} overlaps");
                }
            }
            assert!(grid.right < grid.stage[0]);
            assert!(grid.stage[0] + grid.stage[1] <= width);
        }
    }

    /// A larger text size grows the tiles; when the rows no longer fit, fewer show and the
    /// grid scrolls by rows to keep the chosen one in view.
    #[test]
    fn large_text_grows_the_tiles_and_scrolls_by_rows() {
        let ordinary = layout(1_440.0, &spans(), 1.0);
        let large = layout(1_440.0, &spans(), 1.4);
        assert_eq!(large.rows, 4);
        assert!(large.cell[1] >= TILE_MIN_HEIGHT * 1.4);
        assert!(large.shown_rows < large.rows, "4:3 at 1.4 scrolls");
        assert!(large.cell[1] > ordinary.cell[1]);
        // 16:9 at 4K, 150 %, the largest text: three rows still fit.
        let (_, width) = hall_frame([3_840.0, 2_160.0]);
        let big = layout(width, &spans(), 1.4);
        assert_eq!(big.shown_rows, big.rows);
        // Scrolling: the last tile's row comes into view, and back to the top.
        let last = TILES.len() - 1;
        let first = large.keep_in_view(0, last);
        assert_eq!(first, large.rows - large.shown_rows);
        assert!(large.rect(last, first).is_some());
        assert!(large.rect(0, first).is_none());
        assert_eq!(large.keep_in_view(first, 0), 0);
    }

    /// Left and Right walk the reading order, stopping at the ends; Up and Down go to the
    /// tile nearest straight above or below.
    #[test]
    fn the_arrows_move_across_the_tiles() {
        let grid = layout(1_920.0, &spans(), 1.0);
        // 16:9: Shaders (2) Finishers Tags / Toys Medals Achievements Cards / Holocrons
        // Forge Inventory History.
        assert_eq!(grid.across(0, -1), 0);
        assert_eq!(grid.across(0, 1), 1);
        assert_eq!(grid.across(10, 1), 10);
        assert_eq!(grid.vertical(0, true), 3, "under Shaders' left half: Toys");
        assert_eq!(grid.vertical(2, true), 6, "under Tags: Cards");
        assert_eq!(grid.vertical(5, false), 1, "over Achievements: Finishers");
        assert_eq!(grid.vertical(4, false), 0, "over Medals: Shaders");
        assert_eq!(grid.vertical(0, false), 0);
        assert_eq!(grid.vertical(9, true), 9);
        assert_eq!(grid.vertical(4, true), 8);
    }

    fn press(panel: &mut Panel, key: KeyCode) -> PanelAction {
        panel.hall_key(key, false, false)
    }

    /// Enter opens the kind chosen, Escape inside it comes back to the hall on its tile,
    /// Escape on the hall leaves; the Holocrons tile asks for the Holocrons page and the
    /// Finishers tile opens Shaders on its Finishers pill.
    #[test]
    fn enter_opens_a_kind_and_escape_comes_back() {
        let mut panel = Panel::new();
        panel.open(Tab::Hall, false, true, ReturnTarget::MainMenu);
        assert!(panel.on_hall());
        assert_eq!(panel.tab(), Tab::Hall);
        assert_eq!(panel.hall_tile(), Tile::Shaders);
        assert_eq!(press(&mut panel, KeyCode::ArrowRight), PanelAction::None);
        assert_eq!(panel.hall_tile(), Tile::Finishers);
        assert_eq!(press(&mut panel, KeyCode::Enter), PanelAction::None);
        assert!(!panel.on_hall());
        assert_eq!(panel.tab(), Tab::Shaders);
        assert!(panel.finishers_shown);
        // Escape inside a kind: back to the hall, never closed.
        let action = panel.key(KeyCode::Escape, false, false);
        assert_eq!(action, PanelAction::Close, "the tab's own answer");
        assert_eq!(
            panel.escape_to_hall(KeyCode::Escape, action),
            PanelAction::None
        );
        assert!(panel.on_hall() && panel.is_open());
        assert_eq!(panel.hall_tile(), Tile::Finishers);
        // The Holocrons tile: the console shows the Holocrons page.
        panel.hall_state.chosen = TILES.iter().position(|t| *t == Tile::Holocrons).unwrap();
        assert_eq!(
            press(&mut panel, KeyCode::Space),
            PanelAction::Hub(Tab::Holocrons)
        );
        // Shaders again leaves the Finishers pill.
        assert_eq!(press(&mut panel, KeyCode::Home), PanelAction::None);
        assert_eq!(press(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.tab(), Tab::Shaders);
        assert!(!panel.finishers_shown);
        // Back on the hall, Escape leaves the Collection.
        panel.show(Tab::Hall);
        assert_eq!(press(&mut panel, KeyCode::Escape), PanelAction::Close);
        // End and Tab.
        assert_eq!(press(&mut panel, KeyCode::End), PanelAction::None);
        assert_eq!(panel.hall_tile(), Tile::History);
        assert_eq!(press(&mut panel, KeyCode::Tab), PanelAction::None);
        assert_eq!(panel.hall_tile(), Tile::Shaders);
        // P shows the Loadouts card over the hall.
        assert_eq!(press(&mut panel, KeyCode::KeyP), PanelAction::None);
        assert!(panel.loadouts.is_open());
    }

    /// Opened on a kind (a command naming it), the page skips the hall; a kind shown
    /// while the hall shows leaves it.
    #[test]
    fn a_kind_named_skips_the_hall() {
        let mut panel = Panel::new();
        panel.open(Tab::Achievements, false, true, ReturnTarget::MainMenu);
        assert!(!panel.on_hall());
        assert_eq!(panel.tab(), Tab::Achievements);
        panel.show(Tab::Hall);
        assert_eq!(panel.tab(), Tab::Hall);
        assert_eq!(panel.hall_tile(), Tile::Achievements, "its tile is chosen");
        // The kind shown last is remembered for the setting that opens on it.
        assert_eq!(panel.kind(), Tab::Achievements);
        panel.show(Tab::Toys);
        assert_eq!(panel.tab(), Tab::Toys);
        assert_eq!(panel.kind(), Tab::Toys);
    }

    /// The pointer: resting on a tile chooses it, a click opens it, the breadcrumb's
    /// Collection inside a kind comes back to the hall.
    #[test]
    fn a_click_on_a_tile_opens_it_and_the_breadcrumb_comes_back() {
        let inputs = super::super::tests::inputs(Holdings::Known(&[]), "");
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let draw = |panel: &mut Panel| panel.build(&inputs, &fonts.font, [1_920.0, 1_080.0]);
        let mut panel = super::super::tests::drawn(Tab::Hall, &inputs);
        draw(&mut panel);
        let toys = TILES.iter().position(|t| *t == Tile::Toys).unwrap();
        let rect = panel
            .ui
            .rect_for(TILE_BASE + toys as u16)
            .expect("the Toys tile");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let _ = panel.handle_pointer(InputEvent::PointerMove(at), false);
        assert_eq!(panel.hall_tile(), Tile::Toys);
        for event in [
            InputEvent::PointerPress {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
        ] {
            let _ = panel.handle_pointer(event, false);
        }
        assert_eq!(panel.tab(), Tab::Toys);
        draw(&mut panel);
        let home = panel
            .ui
            .rect_for(crate::profile_hub::HALL_TOKEN)
            .expect("the breadcrumb");
        let at = sjk_ui::Vec2::new(home.x + home.width * 0.5, home.y + home.height * 0.5);
        for event in [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
        ] {
            let _ = panel.handle_pointer(event, false);
        }
        assert!(panel.on_hall());
        assert_eq!(panel.hall_tile(), Tile::Toys);
        // The hall wants the model, wearing what is worn.
        let wish = panel.stage_wish().expect("the model");
        assert!(wish.hall);
        assert_eq!(wish.skin, PreviewSkin::Worn);
    }
}
