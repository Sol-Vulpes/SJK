//! The Use prompt (Sol's idea, 30/10/2026): what the player's Use key does at what they
//! face, drawn to the right of the crosshair with a gap, as many games show it: a key cap
//! naming the key bound to `+use`, then the action and a line under it. A feature sets
//! the prompt each frame it applies ([`GpuState::want_use_prompt`]); the frame draws the
//! last one set and forgets it, so a prompt nobody sets again is gone the next frame.
//! The mystery boxes are its first user (`mystery_box`).
//!
//! A prompt may carry what the thing faced can give and how likely each is (Sol,
//! 15/11/2026: the mystery boxes and the WHEELS OF DOOM, by the hub's odds): a panel of
//! them fades in under the prompt once it has stood [`DROPS_DELAY`].

use crate::GpuState;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};
use std::time::{Duration, Instant};

/// The gap between the crosshair's middle and the key cap, in pixels at 1080 lines (Sol,
/// 30/10/2026: 34 first, too close).
const GAP: f32 = 56.0;
/// The key cap's height and its key's size, and the action's and the line's sizes.
const CAP: f32 = 30.0;
const KEY_SIZE: f32 = 17.0;
const ACTION_SIZE: f32 = 21.0;
const LINE_SIZE: f32 = 15.0;
/// The widest the words beside the cap may run.
const WORDS_WIDTH: f32 = 520.0;
/// The command whose key the cap names.
const USE: &str = "+use";
/// How long the prompt stands before its drops panel fades in, and the fade.
const DROPS_DELAY: Duration = Duration::from_millis(350);
const DROPS_FADE: Duration = Duration::from_millis(250);
/// The drops panel's width, its padding, its rows' size and a row's height, in pixels
/// at 1080 lines.
const PANEL_WIDTH: f32 = 330.0;
const PANEL_PAD: f32 = 12.0;
const ROW_SIZE: f32 = 16.0;
const ROW: f32 = 22.0;
/// The most rows drawn (the wheel groups its bad effects into one).
const MOST_DROPS: usize = 12;
/// The width of a row's chance, at its right.
const CHANCE_WIDTH: f32 = 70.0;

/// One thing the faced box or wheel may give, and its chance.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Drop {
    pub(crate) name: String,
    /// Its chance, per mille, as the hub rolls it.
    pub(crate) per_mille: u32,
    /// Its name's colour (a holocron's tier), white without.
    pub(crate) colour: Option<Color>,
}

/// `per_mille` as a player reads it: "45.5 %", "1 %".
pub(crate) fn percent(per_mille: u32) -> String {
    if per_mille.is_multiple_of(10) {
        format!("{} %", per_mille / 10)
    } else {
        format!("{}.{} %", per_mille / 10, per_mille % 10)
    }
}

/// `drops` likeliest first, the equally likely in the order given.
pub(crate) fn likeliest_first(mut drops: Vec<Drop>) -> Vec<Drop> {
    drops.sort_by_key(|drop| std::cmp::Reverse(drop.per_mille));
    drops
}

/// What Use does at what the player faces.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Prompt {
    /// What pressing Use does ("Open mystery box").
    pub(crate) action: String,
    /// A line under it ("100 credits"), or none.
    pub(crate) line: Option<String>,
    /// Use does it now; false greys the cap (it waits, or cannot).
    pub(crate) ready: bool,
    /// What it may give, likeliest first; none draws no panel.
    pub(crate) drops: Vec<Drop>,
}

/// The prompt asked for this frame and its draw list.
pub(crate) struct UsePrompt {
    wanted: Option<Prompt>,
    list: DrawList,
    lines: Vec<String>,
    /// Since when a prompt with drops has stood, for the panel's fade.
    drops_since: Option<Instant>,
}

impl UsePrompt {
    /// The prompt's key cap and words, for the frame's shape pass.
    pub(crate) fn list(&self) -> &DrawList {
        &self.list
    }
}

impl Default for UsePrompt {
    fn default() -> Self {
        Self {
            wanted: None,
            // The cap, its edge, three lines over their shadows, and the drops panel:
            // its plate, edge, title and two words a row, each over a shadow.
            list: DrawList::new(8 + 4 + MOST_DROPS * 4),
            lines: Vec::new(),
            drops_since: None,
        }
    }
}

/// The key cap's words for the keys bound to `+use` (the first one), "Use" when none is.
fn key_words(keys: &[String]) -> String {
    keys.first()
        .map(|key| crate::keybind_editor::sjk_view::key_label(key))
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| "Use".to_owned())
}

/// The key cap's width for `key`, in pixels at 1080 lines: square for one character,
/// wider for a word.
fn cap_width(key: &str) -> f32 {
    (12.0 + key.chars().count() as f32 * 11.0).max(CAP)
}

/// Where the cap and the words stand, by the crosshair's middle `at` (pixels), `unit`
/// pixels a 1080-line pixel, in a `viewport`: the cap's rectangle and the words' left
/// and top, kept on the screen.
fn layout(
    at: [f32; 2],
    key: &str,
    unit: f32,
    viewport: [f32; 2],
    two_lines: bool,
) -> (Rect, [f32; 2]) {
    let cap = Rect::new(
        (at[0] + GAP * unit)
            .min(viewport[0] - (cap_width(key) + 120.0) * unit)
            .max(0.0),
        at[1] - CAP * 0.5 * unit,
        cap_width(key) * unit,
        CAP * unit,
    );
    let words_top = if two_lines {
        at[1] - (ACTION_SIZE * 1.15) * unit
    } else {
        at[1] - ACTION_SIZE * 0.65 * unit
    };
    (cap, [cap.x + cap.width + 10.0 * unit, words_top])
}

/// How far the drops panel has faded in, `since` its prompt first stood.
fn drops_shade(since: Duration) -> f32 {
    let into = since.saturating_sub(DROPS_DELAY);
    (into.as_secs_f32() / DROPS_FADE.as_secs_f32()).clamp(0.0, 1.0)
}

/// The drops panel's rectangle for `rows` rows, its top left wanted at `at`, kept on the
/// screen above the `bottom` pixels the bottom status bar keeps (`status_bar.rs`).
fn drops_panel(at: [f32; 2], rows: usize, unit: f32, viewport: [f32; 2], bottom: f32) -> Rect {
    let width = PANEL_WIDTH * unit;
    let height = (PANEL_PAD * 2.0 + ROW * (rows as f32 + 1.0)) * unit;
    Rect::new(
        at[0].min(viewport[0] - width).max(0.0),
        at[1].min(viewport[1] - bottom - height).max(0.0),
        width,
        height,
    )
}

/// Push the drops panel's plate, its title and its rows to `state`'s list, `shade` faded
/// in.
fn push_drops(state: &mut UsePrompt, panel: Rect, drops: &[Drop], unit: f32, shade: f32) {
    let radius = 8.0 * unit;
    let gold = Color::new(1.0, 0.84, 0.42, 1.0);
    let _ = state.list.push(DrawCommand::RoundedRect {
        rect: panel,
        radius,
        color: Color::new(0.06, 0.06, 0.08, 0.86 * shade),
    });
    let _ = state.list.push(DrawCommand::Border {
        rect: panel,
        radius,
        width: 1.5 * unit,
        color: Color::new(gold.r, gold.g, gold.b, 0.9 * shade),
    });
    let inner = PANEL_WIDTH - PANEL_PAD * 2.0;
    let left = panel.x + PANEL_PAD * unit;
    let row_rect = |row: usize, from: f32, width: f32| {
        Rect::new(
            left + from * unit,
            panel.y + (PANEL_PAD + ROW * row as f32) * unit,
            width * unit,
            ROW * unit,
        )
    };
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    let grey = Color::new(0.82, 0.82, 0.82, 1.0);
    let rows = drops.iter().take(MOST_DROPS).enumerate();
    let words = std::iter::once((
        "Possible drops".to_owned(),
        row_rect(0, 0.0, inner),
        gold,
        TextAlign::Start,
    ))
    .chain(rows.flat_map(|(index, drop)| {
        [
            (
                drop.name.clone(),
                row_rect(index + 1, 0.0, inner - CHANCE_WIDTH),
                drop.colour.unwrap_or(white),
                TextAlign::Start,
            ),
            (
                percent(drop.per_mille),
                row_rect(index + 1, inner - CHANCE_WIDTH, CHANCE_WIDTH),
                grey,
                TextAlign::End,
            ),
        ]
    }));
    let size = ROW_SIZE * unit;
    for (text, rect, colour, align) in words {
        let id = state.lines.len() as u32;
        state.lines.push(text);
        for (offset, color) in [
            (1.5 * unit, Color::new(0.0, 0.0, 0.0, 0.75 * shade)),
            (
                0.0,
                Color::new(colour.r, colour.g, colour.b, colour.a * shade),
            ),
        ] {
            let _ = state.list.push(DrawCommand::Text {
                rect: Rect::new(rect.x + offset, rect.y + offset, rect.width, rect.height),
                text: TextId(id),
                size,
                color,
                align,
                overflow: TextOverflow::Ellipsis,
                weight: FontWeight::Semibold,
                letter_spacing: 0.0,
            });
        }
    }
}

impl GpuState {
    /// Show `prompt` by the crosshair this frame.
    pub(crate) fn want_use_prompt(&mut self, prompt: Prompt) {
        self.use_prompt.wanted = Some(prompt);
    }

    /// Lay out this frame's prompt by the crosshair and add its words to the HUD's text;
    /// nothing while `shown` is false (menus, console, scoreboard, intermission). The
    /// prompt asked for is forgotten either way.
    pub(crate) fn append_use_prompt(&mut self, viewport: [f32; 2], shown: bool) {
        let wanted = self.use_prompt.wanted.take();
        self.use_prompt.list.clear();
        self.use_prompt.lines.clear();
        let Some(mut prompt) = wanted.filter(|_| shown) else {
            self.use_prompt.drops_since = None;
            return;
        };
        let drops = std::mem::take(&mut prompt.drops);
        let shade = if drops.is_empty() {
            self.use_prompt.drops_since = None;
            0.0
        } else {
            let now = Instant::now();
            let since = *self.use_prompt.drops_since.get_or_insert(now);
            drops_shade(now.saturating_duration_since(since))
        };
        let keys = self
            .console
            .as_ref()
            .map(|console| console.keys_for_command(USE))
            .unwrap_or_default();
        let key = key_words(&keys);
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let [x, y] = self.hud.targeting.center(viewport);
        let at = [x * viewport[0], y * viewport[1]];
        let (cap, [left, top]) = layout(at, &key, unit, viewport, prompt.line.is_some());
        let state = &mut self.use_prompt;
        let edge = if prompt.ready {
            Color::new(1.0, 0.84, 0.42, 1.0)
        } else {
            Color::new(0.55, 0.55, 0.55, 0.9)
        };
        let radius = 6.0 * unit;
        let _ = state.list.push(DrawCommand::RoundedRect {
            rect: cap,
            radius,
            color: Color::new(0.06, 0.06, 0.08, 0.86),
        });
        let _ = state.list.push(DrawCommand::Border {
            rect: cap,
            radius,
            width: 1.5 * unit,
            color: edge,
        });
        let words = [
            Some((key, cap, KEY_SIZE, edge, TextAlign::Center)),
            Some((
                prompt.action,
                Rect::new(left, top, WORDS_WIDTH * unit, ACTION_SIZE * 1.3 * unit),
                ACTION_SIZE,
                if prompt.ready {
                    Color::new(1.0, 1.0, 1.0, 1.0)
                } else {
                    Color::new(0.75, 0.75, 0.75, 1.0)
                },
                TextAlign::Start,
            )),
            prompt.line.map(|line| {
                (
                    line,
                    Rect::new(
                        left,
                        top + ACTION_SIZE * 1.25 * unit,
                        WORDS_WIDTH * unit,
                        LINE_SIZE * 1.3 * unit,
                    ),
                    LINE_SIZE,
                    Color::new(0.82, 0.82, 0.82, 1.0),
                    TextAlign::Start,
                )
            }),
        ];
        for (text, rect, size, colour, align) in words.into_iter().flatten() {
            let id = state.lines.len() as u32;
            state.lines.push(text);
            // The key sits in its cap; the words over a dark shadow, to read on any wall.
            let shadow = if align == TextAlign::Center {
                None
            } else {
                Some(Color::new(0.0, 0.0, 0.0, 0.75))
            };
            let size = size * unit;
            let rect = if align == TextAlign::Center {
                // Its line box about the cap's middle; its glyphs are centred on it once
                // drawn, below.
                Rect::new(
                    rect.x,
                    rect.y + (rect.height - size) * 0.5,
                    rect.width,
                    size,
                )
            } else {
                rect
            };
            for (offset, color) in shadow
                .map(|shadow| (1.5 * unit, shadow))
                .into_iter()
                .chain([(0.0, colour)])
            {
                let _ = state.list.push(DrawCommand::Text {
                    rect: Rect::new(rect.x + offset, rect.y + offset, rect.width, rect.height),
                    text: TextId(id),
                    size,
                    color,
                    align,
                    overflow: TextOverflow::Ellipsis,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
        }
        if shade > 0.0 {
            // Under the words, or the cap when they are shorter.
            let words_bottom = top
                + if state.lines.len() > 2 {
                    (ACTION_SIZE * 1.25 + LINE_SIZE * 1.3) * unit
                } else {
                    ACTION_SIZE * 1.3 * unit
                };
            let panel = drops_panel(
                [cap.x, words_bottom.max(cap.y + cap.height) + 12.0 * unit],
                drops.len().min(MOST_DROPS),
                unit,
                viewport,
                self.status_bar.reserved().bottom,
            );
            push_drops(state, panel, &drops, unit, shade);
        }
        let state = &self.use_prompt;
        // In the headline font, as centre prints are (Inter until it loads).
        let (vertices, font) = self.game_fonts.target(
            crate::game_font::RetailFont::Headline,
            &mut self.text_vertices,
            &self.ui_font,
        );
        let line = |id: TextId| state.lines.get(id.0 as usize).map_or("", String::as_str);
        // The key first, alone, its ink centred in the cap whatever the font's ascent and
        // descent (Sol, 30/10/2026: its letter sat off the middle); then the words.
        let key_from = vertices.len();
        crate::ui_renderer::append_text_commands_where(
            &state.list,
            line,
            |id, _| id.0 == 0,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
            crate::text::CodePalette::Legible,
        );
        crate::text::centre_vertices(
            &mut vertices[key_from..],
            [cap.x + cap.width * 0.5, cap.y + cap.height * 0.5],
            viewport,
        );
        crate::ui_renderer::append_text_commands_where(
            &state.list,
            line,
            |id, _| id.0 != 0,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
            crate::text::CodePalette::Legible,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cap_names_the_use_key_or_says_use() {
        assert_eq!(key_words(&["e".to_owned()]), "E");
        assert_eq!(key_words(&["MOUSE2".to_owned(), "f".to_owned()]), "Mouse 2");
        assert_eq!(key_words(&[]), "Use");
        // A letter's cap is square; a word's wider.
        assert_eq!(cap_width("E"), CAP);
        assert!(cap_width("Mouse 2") > CAP);
    }

    #[test]
    fn the_prompt_stands_right_of_the_crosshair_and_on_the_screen() {
        let viewport = [1920.0, 1080.0];
        let (cap, [left, top]) = layout([960.0, 540.0], "E", 1.0, viewport, true);
        assert_eq!(cap.x, 960.0 + GAP);
        assert!(
            (cap.y + cap.height * 0.5 - 540.0).abs() < 1e-3,
            "centred on it"
        );
        assert!(left > cap.x + cap.width && top < 540.0);
        // A crosshair at the right edge keeps the cap on the screen.
        let (cap, _) = layout([1915.0, 540.0], "E", 1.0, viewport, false);
        assert!(cap.x + cap.width <= viewport[0]);
    }

    #[test]
    fn the_drops_read_as_percents_likeliest_first_and_fade_in_after_a_moment() {
        assert_eq!(percent(455), "45.5 %");
        assert_eq!(percent(10), "1 %");
        assert_eq!(percent(5), "0.5 %");
        let drop = |name: &str, per_mille| Drop {
            name: name.to_owned(),
            per_mille,
            colour: None,
        };
        let sorted = likeliest_first(vec![drop("a", 10), drop("b", 455), drop("c", 10)]);
        let names: Vec<_> = sorted.iter().map(|drop| drop.name.as_str()).collect();
        assert_eq!(names, ["b", "a", "c"]);
        assert_eq!(drops_shade(Duration::ZERO), 0.0);
        assert_eq!(drops_shade(DROPS_DELAY), 0.0);
        assert_eq!(drops_shade(DROPS_DELAY + DROPS_FADE), 1.0);
    }

    #[test]
    fn the_drops_panel_stays_on_the_screen() {
        let viewport = [1920.0, 1080.0];
        let panel = drops_panel([1900.0, 1000.0], 12, 1.0, viewport, 0.0);
        assert!(panel.x + panel.width <= viewport[0]);
        assert!(panel.y + panel.height <= viewport[1]);
        assert_eq!(
            drops_panel([1000.0, 600.0], 3, 1.0, viewport, 0.0).x,
            1000.0
        );
        // Above the bottom status bar while it shows.
        let lifted = drops_panel([1900.0, 1000.0], 12, 1.0, viewport, 28.0);
        assert_eq!(lifted.bottom(), panel.bottom() - 28.0);
    }
}
