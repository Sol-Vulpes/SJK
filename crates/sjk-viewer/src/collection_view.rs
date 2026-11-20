//! The Collection page's frame, in the SJK UI's look: the map darkened behind Medals
//! and Achievements, or dark on the left and clear over the player's model behind
//! Shaders and Toys; the way back, the title and the row of tabs, each
//! with how many of its things the player holds; at the top right how many of all of
//! them, as a tick per thing, lit for the ones held; the tab; the keys.

use super::*;
use crate::menu::sjk::{
    Frame, TextTarget, color, fade, fade_across, key_hint, key_hint_width, text,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The page's left edge, where a tab's things start, and the right edge.
pub(super) const LEFT_X: f32 = 96.0;
pub(super) const RIGHT_X: f32 = 1_824.0;
/// Where a tab's headline stands, under the row of tabs.
pub(super) const LEAD_Y: f32 = 222.0;
/// The keys' line.
pub(super) const KEYS_Y: f32 = 1_010.0;
/// The showcase on the right of Medals and Achievements: its left edge and width.
pub(super) const SHOWCASE_X: f32 = 1_120.0;
pub(super) const SHOWCASE_WIDTH: f32 = RIGHT_X - SHOWCASE_X;
/// Where the live preview of the player's model shows over a match (frame pixels):
/// where the model stands on the menu map's stage.
pub(crate) const MODEL_AREA: [f32; 4] = [760.0, 110.0, 820.0, 840.0];
/// Where the Shaders tab's saber lies on its own (frame pixels), over the clear right of
/// the screen above the shader's card: on the menu map's stage before the backdrop
/// camera, and over a match in the live preview drawn there.
pub(crate) const SABER_AREA: [f32; 4] = [1_000.0, 196.0, RIGHT_X - 1_000.0, 300.0];

/// The tick bar at the top right: a tick's size, the gaps between ticks and groups.
/// The tally's bar at the top right: as wide as its words, a stretch to a tab.
const TALLY_WIDTH: f32 = 500.0;
const TALLY_HEIGHT: f32 = 6.0;
const GROUP_GAP: f32 = 10.0;

/// How many of a tab's things the player holds, and how many there are; `None` while
/// that is not known (no identity, no answer from the hub).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Count {
    pub(super) held: Option<usize>,
    pub(super) total: usize,
}

impl Count {
    /// What the tab's name says after it: "2/4", how many for a tab with no set total
    /// (the holocrons held), empty when unknown or none.
    pub(super) fn label(self) -> String {
        match self.held {
            Some(held) if self.total > 0 => format!("{held}/{}", self.total),
            Some(held) if held > 0 => crate::cards::grouped(held as u64),
            _ => String::new(),
        }
    }
}

/// Each tab's count, in the row's order.
pub(super) fn counts(inputs: &Inputs<'_>) -> [Count; 10] {
    let medals = medals::held(inputs);
    // Out of the ones the player sees: a retired one not held is not counted.
    let (achievements, achievements_total) = crate::achievements::tally(inputs.standings);
    let shaders = (inputs.holdings.reason().is_none()).then(|| {
        unlockables::ALL
            .iter()
            .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
            .count()
    });
    // Out of the shaders the player sees: an unheld Secret one is not counted.
    let shaders_shown = inputs.holdings.listed_count();
    [
        Count {
            held: medals,
            total: crate::medals::Medal::COUNT,
        },
        Count {
            held: Some(achievements),
            total: achievements_total,
        },
        Count {
            held: shaders,
            total: shaders_shown,
        },
        // Every toy is everyone's (Illuminate, the Ragdoll, the Drone...).
        Count {
            held: Some(TOY_COUNT),
            total: TOY_COUNT,
        },
        // The tags held, the free ones with them.
        Count {
            held: (inputs.holdings.reason().is_none()).then(|| {
                crate::tags::CATALOGUE
                    .iter()
                    .filter(|tag| tag.free || inputs.holdings.unlock(tag.id).is_some())
                    .count()
            }),
            total: crate::tags::CATALOGUE.len(),
        },
        // The holocrons are opened, not kept: how many, and no total.
        Count {
            held: inputs
                .snapshot
                .filter(|_| inputs.enabled)
                .and_then(|snapshot| snapshot.me.as_ref())
                .map(|me| {
                    crate::holocrons::counts_of(&me.holocron_counts)
                        .iter()
                        .sum::<u32>() as usize
                }),
            total: 0,
        },
        // The Forge: the kyber shards the player holds, what it trades with.
        Count {
            held: inputs
                .snapshot
                .filter(|_| inputs.enabled)
                .and_then(|snapshot| snapshot.me.as_ref())
                .filter(|me| me.shards > 0)
                .map(|me| me.shards as usize),
            total: 0,
        },
        // The cards held, and no total until a set is out.
        Count {
            held: cards::held(inputs),
            total: 0,
        },
        // The history counts nothing.
        Count {
            held: None,
            total: 0,
        },
        // The inventory lists what the others count.
        Count {
            held: None,
            total: 0,
        },
    ]
}

/// The Collection row's counts, for the Holocrons page, which draws the row without the
/// Collection page's inputs.
pub(crate) fn row_labels(
    enabled: bool,
    snapshot: Option<&sjk_identity::Snapshot>,
    standings: &[Standing],
) -> Vec<String> {
    let inputs = Inputs {
        enabled,
        snapshot,
        holdings: unlockables::Holdings::of(enabled, snapshot),
        setting: "",
        standings,
        illuminate: true,
        ragdoll: false,
        drone: false,
        boombox: false,
        monitor: false,
        box_hunt: false,
        fishing: false,
        mining: false,
        quests: false,
        fishing_card: None,
        wheel_of_doom: false,
        mining_card: None,
        stock: color::holo(),
        grid: false,
        options: "",
        second: "",
        pair: crate::saber_skins::Pair::None,
        toy_keys: &[],
        toys_unseen: 0,
        toy_options: 0,
        tag: "",
        tag_finish: "",
        finisher: "",
        gif_tags: false,
    };
    counts(&inputs).iter().map(|count| count.label()).collect()
}

/// A soft round glow of `colour` at (`x`, `y`) (frame pixels), `radius` across its
/// half, `strength` its alpha at the middle: discs stacked from the rim inwards.
pub(super) fn glow(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    centre: [f32; 2],
    radius: f32,
    colour: Color,
    strength: f32,
) {
    const LAYERS: usize = 8;
    let [x, y] = centre;
    for layer in 0..LAYERS {
        let r = radius * (1.0 - layer as f32 / LAYERS as f32);
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x - r, y - r, r * 2.0, r * 2.0),
            radius: r * frame.s,
            color: color::alpha(colour, strength / LAYERS as f32),
        });
    }
}

/// Lines of `body`, centred in the column from `x` `width` wide, from `y`, `chars` to a
/// line and at most `lines` of them; returns where the next line would stand.
#[allow(clippy::too_many_arguments)]
pub(super) fn centred_lines(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    body: &str,
    [x, y, width]: [f32; 3],
    chars: usize,
    lines: usize,
    size: f32,
    colour: Color,
) -> f32 {
    left_lines(
        canvas,
        frame,
        body,
        [x, y, width],
        (chars, lines),
        size,
        colour,
        TextAlign::Center,
    )
}

/// Lines of `body` from (`x`, `y`) in a column `width` wide, aligned `align`; `fit` is
/// the characters to a line and the most lines. Returns where the next line would
/// stand.
#[allow(clippy::too_many_arguments)]
pub(super) fn left_lines(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    body: &str,
    [x, mut y, width]: [f32; 3],
    (chars, lines): (usize, usize),
    size: f32,
    colour: Color,
    align: TextAlign,
) -> f32 {
    let step = size * 1.42;
    let parts: Vec<&str> = crate::menu::sjk::wrap(body, chars).collect();
    for (index, part) in parts.iter().take(lines).enumerate() {
        let cut = index + 1 == lines && parts.len() > lines;
        text(
            canvas,
            TextFamily::Body,
            format_args!("{part}{}", if cut { "..." } else { "" }),
            frame.rect(x, y, width, step),
            size * frame.s,
            colour,
            FontWeight::Regular,
            align,
        );
        y += step;
    }
    y
}

/// The shape an empty page shows of what will fill it ([`empty_state`], 15/11/2026).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ghosts {
    /// Three card outlines, each with a "?".
    Cards,
    /// A dim holocron.
    Cube,
    /// A grey crystal (a dormant kyber).
    Crystal,
    /// Three medal outlines.
    Medals,
    /// A few dim rows, as a list's.
    Rows,
}

/// The one thing an empty page offers ([`empty_state`]): its button's label and token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EmptyButton<'a> {
    pub(crate) label: &'a str,
    pub(crate) token: u16,
}

/// An empty page that points the way (the ideas page's "Empty states", 15/11/2026): in
/// `rect` (frame pixels), the outlines of what fills the page (`ghosts`), its title and
/// line (how it fills) and at most one `action`. A rect under 160 pixels tall lays it out
/// as a row, the ghost at the left and the button at the right; a taller one as a column,
/// centred. The words are the caller's, already through [`crate::terms::say`].
pub(crate) fn empty_state(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y, width, height]: [f32; 4],
    ghosts: Ghosts,
    (title, line): (&str, &str),
    action: Option<EmptyButton<'_>>,
) {
    let s = frame.s;
    let control = crate::menu::sjk::kit::CONTROL_HEIGHT;
    let button_width = |label: &str, pad: f32| label_width(label, 17.0, 1.0) + pad;
    if height < 160.0 {
        let side = (height - 8.0).max(24.0);
        ghost(canvas, frame, [x + 4.0, y + 4.0, side, side], ghosts, true);
        let button = action.map_or(0.0, |action| button_width(action.label, 48.0));
        let left = x + side + 22.0;
        let room = (width - side - 22.0 - button - 16.0).max(40.0);
        let middle = y + height * 0.5;
        text(
            canvas,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(left, middle - 26.0, room, 26.0),
            20.0 * s,
            color::text(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            canvas,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(left, middle + 1.0, room, 22.0),
            14.0 * s,
            color::muted(),
            FontWeight::Regular,
            TextAlign::Start,
        );
        if let Some(action) = action {
            crate::menu::sjk::kit::button(
                canvas,
                frame,
                [x + width - button, middle - control * 0.5, button, control],
                action.label,
                true,
                true,
                false,
                action.token,
            );
        }
        return;
    }
    let ghost_height = (height * 0.42).clamp(90.0, 300.0);
    let ghost_width = match ghosts {
        Ghosts::Cards | Ghosts::Medals => ghost_height * 2.2,
        Ghosts::Rows => width.min(640.0),
        Ghosts::Cube | Ghosts::Crystal => ghost_height,
    };
    let top = y + 8.0;
    ghost(
        canvas,
        frame,
        [
            x + (width - ghost_width) * 0.5,
            top,
            ghost_width,
            ghost_height,
        ],
        ghosts,
        false,
    );
    let mut at = top + ghost_height + 28.0;
    text(
        canvas,
        TextFamily::Display,
        format_args!("{title}"),
        frame.rect(x, at, width, 40.0),
        30.0 * s,
        color::text(),
        FontWeight::Semibold,
        TextAlign::Center,
    );
    at += 46.0;
    let column = width.min(760.0);
    at = centred_lines(
        canvas,
        frame,
        line,
        [x + (width - column) * 0.5, at, column],
        ((column / 8.6) as usize).max(20),
        3,
        17.0,
        color::muted(),
    );
    if let Some(action) = action {
        let button = button_width(action.label, 64.0);
        crate::menu::sjk::kit::button(
            canvas,
            frame,
            [x + (width - button) * 0.5, at + 18.0, button, control],
            action.label,
            true,
            true,
            false,
            action.token,
        );
    }
}

/// An empty state's button on a Collection page ([`empty_state`]).
pub(super) const EMPTY_ACTION_TOKEN: u16 = 8_601;

/// What an empty Collection page's button (or Enter) does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EmptyAction {
    /// Show the Holocrons page, where the waiting ones open.
    Holocrons,
    /// Show the server list: holocrons drop while playing.
    Servers,
}

impl Panel {
    /// The empty state's button was pressed (or Enter on its page).
    pub(super) fn press_empty_action(&mut self) -> PanelAction {
        match self.empty_action {
            Some(EmptyAction::Holocrons) => {
                ui_cues_click();
                PanelAction::Hub(Tab::Holocrons)
            }
            Some(EmptyAction::Servers) => {
                ui_cues_click();
                PanelAction::Servers
            }
            None => PanelAction::None,
        }
    }
}

/// `ghosts` drawn dim over `[x, y, width, height]` (`small`: a row's, one of them).
fn ghost(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y, width, height]: [f32; 4],
    ghosts: Ghosts,
    small: bool,
) {
    let s = frame.s;
    let outline = |canvas: &mut crate::menu_widgets::MenuCanvas, rect: [f32; 4], radius: f32| {
        let area = frame.rect(rect[0], rect[1], rect[2], rect[3]);
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: radius * s,
            color: color::alpha(color::holo(), 0.04),
        });
        let _ = canvas.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: radius * s,
            width: 1.5 * s,
            color: color::alpha(color::holo(), 0.26),
        });
    };
    let mark = |canvas: &mut crate::menu_widgets::MenuCanvas, rect: [f32; 4], mark: char| {
        let size = rect[2].min(rect[3]) * 0.42;
        text(
            canvas,
            TextFamily::Body,
            format_args!("{mark}"),
            frame.rect(
                rect[0],
                rect[1] + (rect[3] - size * 1.3) * 0.5,
                rect[2],
                size * 1.3,
            ),
            size * s,
            color::alpha(color::holo(), 0.32),
            FontWeight::Semibold,
            TextAlign::Center,
        );
    };
    let count = if small { 1 } else { 3 };
    match ghosts {
        Ghosts::Cards | Ghosts::Medals => {
            let medal = ghosts == Ghosts::Medals;
            let item_width = if medal {
                height
            } else {
                height / crate::cards::look::ASPECT
            };
            let gap = item_width * 0.18;
            let all = item_width * count as f32 + gap * (count - 1) as f32;
            let left = x + (width - all) * 0.5;
            for index in 0..count {
                let rect = [
                    left + index as f32 * (item_width + gap),
                    y,
                    item_width,
                    height,
                ];
                if medal {
                    outline(canvas, rect, item_width * 0.5);
                    mark(canvas, rect, crate::symbols::MEDAL);
                } else {
                    outline(canvas, rect, item_width * 0.07);
                    mark(canvas, rect, '?');
                }
            }
        }
        Ghosts::Cube | Ghosts::Crystal => {
            let side = width.min(height);
            let rect = [
                x + (width - side) * 0.5,
                y + (height - side) * 0.5,
                side,
                side,
            ];
            outline(canvas, rect, side * 0.2);
            mark(
                canvas,
                rect,
                if ghosts == Ghosts::Cube {
                    crate::symbols::HOLOCRON
                } else {
                    crate::symbols::SHARD
                },
            );
        }
        Ghosts::Rows => {
            let rows = if small { 1 } else { 4 };
            let step = height / rows as f32;
            for index in 0..rows {
                let top = y + index as f32 * step;
                outline(canvas, [x, top + 3.0, width, step - 6.0], 8.0);
            }
        }
    }
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        let body = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        // Inter, where the families are not loaded, is wider than the display family.
        self.widen = if matches!(target, TextTarget::Inter(..)) {
            1.3
        } else {
            1.0
        };
        self.build(inputs, body, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out; `_body` is the body family, kept for measured layouts.
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, _body: &UiFont, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.follow_news(inputs);
        self.empty_action = None;
        self.ragdoll_on = inputs.ragdoll;
        self.drone_on = inputs.drone;
        self.boombox_on = inputs.boombox;
        self.monitor_on = inputs.monitor;
        self.box_hunt_on = inputs.box_hunt;
        self.fishing_on = inputs.fishing;
        self.mining_on = inputs.mining;
        self.quests_on = inputs.quests;
        self.toys_unseen = inputs.toys_unseen;
        self.toy_options = inputs.toy_options;
        let seconds = self.epoch.elapsed().as_secs_f32();
        #[cfg(test)]
        let seconds = self.shot_seconds.unwrap_or(seconds);
        self.ui.begin_transparent(viewport);
        // The Saber Lab's tab goes with the lab: Shaders shows when it stops.
        if self.tab == Tab::SaberLab && !crate::saber_lab::active() {
            self.tab = Tab::Shaders;
        }
        // The motion from the page shown last frame to this one (it only draws).
        let now = self.look_clock();
        self.transition.follow(
            super::transition::Place {
                hall: self.hall,
                tab: self.tab,
                finishers: self.finishers_shown,
            },
            now,
            crate::rarity_fx::enabled(),
        );
        self.transition.now = self.transition.at(now);
        // The hall, or the Loadouts card over it (the hall left out under it).
        if self.hall {
            // Not on show: shown again, the History tab reads the newest lines.
            self.history.shown = false;
            if self.loadouts.is_open() {
                crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
                self.loadouts
                    .draw(&mut self.ui, &frame, viewport, &self.loadout_slots);
                self.ui.finish(u16::MAX);
                return;
            }
            self.hall_view(inputs, viewport, seconds);
            self.closing_card(&frame);
            self.ui
                .finish(hall::TILE_BASE + self.hall_state.chosen as u16);
            return;
        }
        let kit = matches!(self.tab, Tab::Shaders | Tab::Toys);
        if self.lab_building() {
            lab_build_scrims(&mut self.ui, viewport, &frame);
        } else if self.tab == Tab::SaberLab && self.backstage != Backstage::None {
            lab_scrims(&mut self.ui, viewport, &frame);
        } else if kit && self.backstage != Backstage::None {
            stage_scrims(&mut self.ui, viewport, &frame);
        } else {
            crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        }
        if let Backstage::Preview { ready: true } = self.backstage
            && kit
        {
            // The Shaders tab's preview is its saber on its own.
            let [x, y, width, height] = if self.tab == Tab::Shaders {
                SABER_AREA
            } else {
                MODEL_AREA
            };
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(x, y, width, height),
                texture: crate::ui_renderer::PREVIEW_TEXTURE,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        if self.tab != Tab::History {
            // Not on show: shown again, the History tab reads the newest lines.
            self.history.shown = false;
        }
        let counts = counts(inputs);
        let mut labels: Vec<String> = counts.iter().map(|count| count.label()).collect();
        // The Forge says the shards it trades with, as it read them this frame.
        if let Some(shards) = self.forge.data.shards.filter(|shards| *shards > 0) {
            labels[Tab::Forge.index()] = crate::cards::grouped(shards);
        }
        // The Loadouts card covers the page: text draws over every shape, so the page is
        // left out under it.
        if self.loadouts.is_open() {
            self.loadouts
                .draw(&mut self.ui, &frame, viewport, &self.loadout_slots);
            self.ui.finish(u16::MAX);
            return;
        }
        // The page arriving: faint, or sliding in from the side, for its motion.
        let arriving = self.arriving_page(&frame);
        let asking = self.tab == Tab::Forge && self.forge.ask.is_some();
        let back = if self.hub {
            crate::profile_hub::back_words(self.back)
        } else {
            "Back"
        };
        // A card asking on the Forge covers the page: the header is left out under it.
        if !asking && self.tab == Tab::Shaders && self.finishers_shown {
            // The Finishers pill's page is not kybers: the breadcrumb says Finishers.
            let held = crate::finishers::CATALOGUE
                .iter()
                .filter(|f| inputs.holdings.debug() || inputs.holdings.unlock(f.id).is_some())
                .count();
            let count = Count {
                held: Some(held),
                total: crate::finishers::CATALOGUE.len(),
            };
            crate::profile_hub::collection_header_as(
                &mut self.ui,
                &frame,
                (back, BACK_TOKEN),
                "Finishers",
                (&count.label(), self.widen),
            );
            self.tally(&frame, &counts);
        } else if !asking {
            crate::profile_hub::collection_header(
                &mut self.ui,
                &frame,
                back,
                BACK_TOKEN,
                self.tab,
                (&labels, self.widen),
            );
            self.tally(&frame, &counts);
        }
        match self.tab {
            Tab::Medals => self.medals(&frame, inputs, seconds),
            Tab::Achievements => self.achievements(&frame, inputs),
            Tab::Shaders => self.shaders(&frame, inputs, seconds),
            Tab::Toys => self.toys(&frame, inputs),
            Tab::Tags => self.tags(&frame, inputs),
            Tab::Banners => self.banners(&frame, inputs),
            Tab::Cards => self.cards(&frame, inputs, seconds),
            Tab::History => self.history_tab(&frame, inputs),
            Tab::Inventory => self.inventory_tab(&frame, inputs, seconds),
            Tab::Forge => {
                self.grid = inputs.grid;
                self.forge(&frame, inputs, seconds, viewport);
            }
            Tab::SaberLab => self.saber_lab(&frame, inputs),
            _ => {}
        }
        if self.offers_loadouts() {
            // Under the tally, clear of each tab's lead line and card.
            crate::menu::sjk::kit::button(
                &mut self.ui,
                &frame,
                [
                    RIGHT_X - 200.0,
                    206.0,
                    200.0,
                    crate::menu::sjk::kit::CONTROL_HEIGHT,
                ],
                "Loadouts  (P)",
                false,
                true,
                false,
                super::loadouts::OPEN_TOKEN,
            );
        }
        self.keys(&frame, inputs);
        self.lab_build_overlay(&frame);
        self.hover_card(&frame, inputs);
        self.arrived_page(&frame, arriving);
        self.ui.finish(self.focus_token());
    }

    /// The window rectangle a kind's page fills, where an opening tile grows to.
    fn page_area(frame: &Frame) -> sjk_ui::Rect {
        frame.rect(LEFT_X, 150.0, RIGHT_X - LEFT_X, 870.0)
    }

    /// Begin the page under this frame's motion: its opacity, and a slide's offset. What
    /// was begun, for [`Self::arrived_page`] to end.
    fn arriving_page(&mut self, frame: &Frame) -> (bool, bool) {
        use super::transition::{Motion, SLIDE_FROM, arriving};
        let Some((motion, t)) = self
            .transition
            .now
            .filter(|(motion, _)| *motion != Motion::Close)
        else {
            return (false, false);
        };
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PushOpacity(arriving(motion, t)));
        let slid = if let Motion::Slide { by } = motion {
            let dx = by * SLIDE_FROM * frame.s * (1.0 - t);
            let _ = list.push(DrawCommand::PushTransform([
                1.0, 0.0, dx, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ]));
            true
        } else {
            false
        };
        (true, slid)
    }

    /// End what [`Self::arriving_page`] began, then draw an opening's tile on its way
    /// from the hall into the page.
    fn arrived_page(&mut self, frame: &Frame, (faded, slid): (bool, bool)) {
        use super::transition::{Motion, between};
        let list = self.ui.draw_list_mut();
        if slid {
            let _ = list.push(DrawCommand::PopTransform);
        }
        if faded {
            let _ = list.push(DrawCommand::PopOpacity);
        }
        if let Some((Motion::Open { from }, t)) = self.transition.now {
            let rect = between(from, Self::page_area(frame), t);
            Self::moving_card(&mut self.ui, frame, rect, 1.0 - t * t);
        }
    }

    /// Over the hall, a closing page on its way back onto its tile.
    fn closing_card(&mut self, frame: &Frame) {
        use super::transition::{Motion, between};
        let (Some((Motion::Close, t)), Some(tile)) = (self.transition.now, self.transition.tile)
        else {
            return;
        };
        let page = Self::page_area(frame);
        Self::moving_card(&mut self.ui, frame, between(page, tile, t), 1.0 - t * t);
    }

    /// A card on its way between a hall tile and its page (window pixels), `alpha`
    /// opaque: the tile's dark glass and the gold of the one chosen.
    fn moving_card(
        canvas: &mut crate::menu_widgets::MenuCanvas,
        frame: &Frame,
        rect: sjk_ui::Rect,
        alpha: f32,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        let list = canvas.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius: 12.0 * frame.s,
            color: color::alpha(color::space(), 0.85 * alpha),
        });
        let _ = list.push(DrawCommand::Border {
            rect,
            radius: 12.0 * frame.s,
            width: 2.0 * frame.s,
            color: color::alpha(color::gold(), alpha),
        });
    }

    /// How many of all the things the player holds, at the top right: the words, and a
    /// bar under them, a stretch to a tab in the tabs' order, each filled as far as its
    /// share held (one tick a thing no longer fits two hundred things, 21/10/2026).
    pub(super) fn tally(&mut self, frame: &Frame, counts: &[Count]) {
        let s = frame.s;
        // Only the things with a set total: the holocrons are opened, not collected.
        let total: usize = counts.iter().map(|count| count.total).sum();
        let held: usize = counts
            .iter()
            .filter(|count| count.total > 0)
            .filter_map(|count| count.held)
            .sum();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{held} of {total} collected"),
            frame.rect(RIGHT_X - TALLY_WIDTH, 58.0, TALLY_WIDTH, 30.0),
            22.0 * s,
            color::text(),
            FontWeight::Semibold,
            TextAlign::End,
        );
        let groups: Vec<Count> = counts.iter().copied().filter(|c| c.total > 0).collect();
        if groups.is_empty() {
            return;
        }
        let stretch = (TALLY_WIDTH - GROUP_GAP * (groups.len() - 1) as f32) / groups.len() as f32;
        let radius = TALLY_HEIGHT / 2.0 * s;
        let mut x = RIGHT_X - TALLY_WIDTH;
        for count in groups {
            let share = count.held.unwrap_or(0).min(count.total) as f32 / count.total as f32;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x, 98.0, stretch, TALLY_HEIGHT),
                radius,
                color: color::alpha(color::holo(), 0.24),
            });
            if share > 0.0 {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x, 98.0, (stretch * share).max(TALLY_HEIGHT), TALLY_HEIGHT),
                    radius,
                    color: color::gold_bright(),
                });
            }
            x += stretch + GROUP_GAP;
        }
    }

    /// A tab's headline at the top left and the line under it.
    pub(super) fn lead(&mut self, frame: &Frame, headline: &str, line: &str) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(LEFT_X, LEAD_Y, 900.0, 40.0),
            30.0 * s,
            color::text(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if !line.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(LEFT_X, LEAD_Y + 42.0, 940.0, 24.0),
                16.0 * s,
                color::muted(),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The keys of the tab on show, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let next_tab = crate::terms::say(self.tab.next(true).label());
        let mut keys: Vec<(&[&str], &str)> = Vec::with_capacity(10);
        match self.tab {
            Tab::Medals => keys.push((&["Left", "Right"], "choose")),
            Tab::Inventory if self.inventory.search_focus => {
                keys.push((&["Esc"], "clear the search"));
                keys.push((&["Enter"], "done"));
            }
            Tab::Inventory => {
                keys.push((&["Up", "Down"], "choose"));
                keys.push((&["1-7"], "kind"));
                keys.push((&["S"], "sort"));
                keys.push((&["/"], "search"));
                for offer in self.inventory.offers.iter().filter(|offer| offer.enabled) {
                    let key: &[&str] = match offer.key {
                        "B" => &["B"],
                        "G" => &["G"],
                        _ => &["Enter"],
                    };
                    keys.push((key, super::inventory::verb(offer.act)));
                }
            }
            Tab::History => {
                keys.push((&["Left", "Right"], "filter"));
                keys.push((&["Up", "Down"], "choose"));
                let chosen = self.history.chosen;
                if self.history.more_row(chosen) {
                    keys.push((&["Enter"], "load older"));
                } else if self
                    .history
                    .rows
                    .get(chosen)
                    .is_some_and(|row| row.group.is_some())
                {
                    keys.push((&["Enter"], "open, close"));
                }
                keys.push((&["R"], "refresh"));
            }
            Tab::Cards if self.empty_action.is_some() => {
                keys.push((&["Enter"], "the way to your first card"))
            }
            Tab::Cards if self.card_inspect => {
                if self.cards_shown > 1 {
                    keys.push((&["Left", "Right"], "previous, next"));
                }
                if self.chosen_finishes_held() > 1 {
                    keys.push((&["V"], "finish"));
                }
                keys.push((&["Esc"], "put back"));
            }
            Tab::Cards => {
                if self.cards_shown > 0 {
                    keys.push((&["Enter"], "look closer"));
                }
                if self.cards_shown > 1 {
                    keys.push((&["Arrows"], "choose"));
                }
                if self.chosen_finishes_held() > 1 {
                    keys.push((&["V"], "finish"));
                }
                if self.cards_shown > cards::COLUMNS * cards::LINES {
                    keys.push((&["PgUp", "PgDn"], "page"));
                }
                keys.push((&["O", "T", "K", "F"], "filter"));
                if self.revealable > 0 {
                    keys.push((&["R"], "reveal"));
                }
            }
            Tab::Achievements => {
                keys.push((&["Arrows"], "choose"));
                keys.push((&["F"], "General or Fun"));
            }
            Tab::Forge if self.forge.ask.is_some() => {
                keys.push((&["Tab"], "switch"));
                keys.push((&["Enter"], "answer"));
                keys.push((&["Esc"], "cancel"));
            }
            Tab::Forge => {
                keys.push((&["Tab"], self.forge.section.next(true).label()));
                match self.forge.section {
                    forge::Section::Shop => {
                        keys.push((&["Arrows"], "choose"));
                        keys.push((&["Enter"], "buy"));
                    }
                    section => {
                        keys.push((&["Arrows"], "choose"));
                        keys.push((&["V"], if self.grid { "list" } else { "grid" }));
                        keys.push((&["/"], "search"));
                        // The marks (29/10/2026): the star, and the padlock on what is held.
                        if self.marks.kept && section == forge::Section::Break {
                            keys.push((&["F", "L"], "star, lock"));
                        } else if self.marks.kept {
                            keys.push((&["F"], "star"));
                        }
                        let picking = section == forge::Section::Break && self.forge.selecting;
                        if picking {
                            // Picking (05/11/2026): what picks, the copies, and the send.
                            keys.push((&["Space"], "pick"));
                            keys.push((&["+", "-"], "copies"));
                        } else if section == forge::Section::Break && self.forge.data.all {
                            keys.push((&["B"], "all duplicates"));
                        }
                        if section == forge::Section::Break && self.forge.data.many {
                            keys.push((&["S"], if picking { "stop" } else { "select" }));
                        }
                        keys.push((
                            &["Enter"],
                            if section == forge::Section::Craft {
                                "craft"
                            } else if picking {
                                "break down selected"
                            } else {
                                "break down"
                            },
                        ));
                    }
                }
            }
            // The finishers page has no rows to step: only Tab and Escape, below.
            Tab::Shaders if self.finishers_shown => {}
            Tab::Shaders => {
                if self.grid {
                    keys.push((&["Arrows"], "choose"));
                    keys.push((&["V"], "list"));
                } else {
                    keys.push((&["Up", "Down"], "choose"));
                    keys.push((&["S"], "sort"));
                    keys.push((&["V"], "tiles"));
                }
                keys.push((&["T"], "tier"));
                keys.push((&["E"], "edition"));
                keys.push((&["/"], "search"));
                // The marks (29/10/2026): the star and the padlock, and every New one seen.
                if self.marks.kept && self.shader > 0 {
                    keys.push((&["F", "L"], "star, lock"));
                }
                if self.marks.new_count() > 0 {
                    keys.push((&["N"], "all seen"));
                }
                let worn = self.shader_rows.get(self.shader).and_then(|row| row.wear);
                match worn {
                    Some("") if self.shader > 0 => keys.push((&["Enter"], "unequip")),
                    Some(_) => keys.push((&["Enter"], "equip")),
                    None => {}
                }
            }
            Tab::Banners => {
                keys.push((&["Arrows"], "choose"));
                keys.push((&["Enter"], "equip / unequip"));
            }
            Tab::Tags => {
                keys.push((&["Arrows"], "choose"));
                if self.tag_choosable(inputs) {
                    keys.push((&["Enter"], "choose for toy_tag"));
                }
            }
            Tab::Toys if self.bind_wait.is_some() => {
                keys.push((&["Esc"], "cancel"));
                keys.push((&["Backspace"], "clear the key"));
            }
            Tab::Toys => keys.push((
                &["Enter"],
                match (self.toy, inputs.illuminate) {
                    (RAGDOLL_TOY, _) if inputs.ragdoll => "get up",
                    (RAGDOLL_TOY, _) => "go limp",
                    (DRONE_TOY, _) if inputs.drone => "turn off",
                    (DRONE_TOY, _) => "turn on",
                    (BOOMBOX_TOY, _) if inputs.boombox => "put away",
                    (BOOMBOX_TOY, _) => "play",
                    (MONITOR_TOY, _) if inputs.monitor => "turn off",
                    (MONITOR_TOY, _) => "turn on",
                    (BOX_HUNT_TOY, _) if inputs.box_hunt => "turn off",
                    (BOX_HUNT_TOY, _) => "turn on",
                    (FISHING_TOY, _) if inputs.fishing => "put away",
                    (FISHING_TOY, _) => "get the rod out",
                    (MINING_TOY, _) if inputs.mining => "put away",
                    (MINING_TOY, _) => "take the pickaxe",
                    (QUESTS_TOY, _) if inputs.quests => "turn off",
                    (QUESTS_TOY, _) => "turn on",
                    (_, true) => "put out",
                    _ => "light",
                },
            )),
            Tab::SaberLab => self.lab_keys(&mut keys),
            _ => {}
        }
        if self.tab == Tab::Toys && self.bind_wait.is_none() {
            keys.push((&["K"], "bind a key"));
        }
        let covered = (self.tab == Tab::Cards && self.card_inspect)
            || (self.tab == Tab::Forge && self.forge.ask.is_some())
            || (self.tab == Tab::Toys && self.bind_wait.is_some())
            || (self.tab == Tab::Inventory && self.inventory.search_focus);
        if !covered {
            keys.push((&["Ctrl", "Tab"], &*next_tab));
            keys.push((&["Esc"], crate::profile_hub::COLLECTION));
        }
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(RIGHT_X, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// Where the band behind the title and the tabs holds solid, and where it has faded out,
/// in frame pixels (25/10/2026, Sol: the tabs on top must read over any map).
const HEADER_SOLID: f32 = 200.0;
const HEADER_CLEAR: f32 = 290.0;

/// The Saber Lab's: dark behind the choices on the right, under the floating saber (which
/// keeps the scene's own dark behind it, as on Shaders), fading out before the model on
/// the left; the band behind the title and the tabs, as the other stage pages', and a
/// light fade behind the keys.
/// The Saber Lab's builder's darkness: the left, where its library and groups stand,
/// fading out before the model on the right; the header's and the keys' as on the
/// stage's view.
fn lab_build_scrims(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    viewport: [f32; 2],
    frame: &Frame,
) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::space(), alpha);
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let y = |frame_y: f32| frame.point(0.0, frame_y)[1];
    fade_across(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, x(1_170.0), height),
        space(0.6),
        space(0.5),
    );
    fade_across(
        canvas,
        sjk_ui::Rect::new(x(1_170.0), 0.0, x(1_300.0) - x(1_170.0), height),
        space(0.5),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, width, y(HEADER_SOLID)),
        space(0.86),
        space(0.86),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(
            0.0,
            y(HEADER_SOLID),
            width,
            y(HEADER_CLEAR) - y(HEADER_SOLID),
        ),
        space(0.86),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, y(900.0), width, height - y(900.0)),
        space(0.0),
        space(0.7),
    );
}

fn lab_scrims(canvas: &mut crate::menu_widgets::MenuCanvas, viewport: [f32; 2], frame: &Frame) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::space(), alpha);
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let y = |frame_y: f32| frame.point(0.0, frame_y)[1];
    let top = y(540.0);
    fade_across(
        canvas,
        sjk_ui::Rect::new(x(1_000.0), top, x(1_200.0) - x(1_000.0), height - top),
        space(0.0),
        space(0.8),
    );
    fade_across(
        canvas,
        sjk_ui::Rect::new(x(1_200.0), top, width - x(1_200.0), height - top),
        space(0.8),
        space(0.9),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, width, y(HEADER_SOLID)),
        space(0.86),
        space(0.86),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(
            0.0,
            y(HEADER_SOLID),
            width,
            y(HEADER_CLEAR) - y(HEADER_SOLID),
        ),
        space(0.86),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, y(900.0), width, height - y(900.0)),
        space(0.0),
        space(0.7),
    );
}

/// Dark behind the things on the left, fading out at the stage before the model; a band
/// behind the title and the tabs, solid under them and fading out above the stage, as
/// the Holocrons page's; dark at the bottom behind the keys. The words beside the model
/// stand on cards of their own.
fn stage_scrims(canvas: &mut crate::menu_widgets::MenuCanvas, viewport: [f32; 2], frame: &Frame) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::space(), alpha);
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let y = |frame_y: f32| frame.point(0.0, frame_y)[1];
    fade_across(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, x(900.0), height),
        space(0.94),
        space(0.88),
    );
    fade_across(
        canvas,
        sjk_ui::Rect::new(x(900.0), 0.0, x(1_060.0) - x(900.0), height),
        space(0.88),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, width, y(HEADER_SOLID)),
        space(0.86),
        space(0.86),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(
            0.0,
            y(HEADER_SOLID),
            width,
            y(HEADER_CLEAR) - y(HEADER_SOLID),
        ),
        space(0.86),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, y(860.0), width, height - y(860.0)),
        space(0.0),
        space(0.86),
    );
}

/// How wide `words` are drawn in the display family at `size` (frame pixels): the
/// player's text size, `widen` (Inter in place of the families) and the letter spacing
/// (`ui_letterSpacing`), which labels measured without it overflowed (10/11/2026).
pub(super) fn label_width(words: &str, size: f32, widen: f32) -> f32 {
    let style = crate::text::style::current();
    let size = size * style.scale;
    let tracking = style.tracking * size * words.chars().count() as f32;
    (crate::text::display_width(words, size) * widen + tracking).max(0.0)
}
