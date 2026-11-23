//! First setup as the SJK UI's pop-up (`docs/sjk-ui.md`, First setup): a card over
//! the darkened map that asks one thing at a time. Across its top the steps, the one on
//! show lit; then what the step asks, its text field if it has one (the player's name),
//! its picture buttons side by side (the two looks, the themes, the graphics presets,
//! each with a picture of what it gives), the few rows that belong with them as Settings
//! draws them ([`super::sjk_view`]), on the last step a recap of the steps before it, and
//! at its foot, always in view, the "Don't show at start" tick beside All settings, Back
//! and Next (Done on the last step, which also ticks it: a setup gone through to its end
//! is finished).
//!
//! What the steps are is a table ([`super::setup_steps`]); this file draws whatever it
//! holds and knows no step by name. The rows are still First setup's own
//! ([`super::ClassicRows`]), a step's worth at a time, so lists, defaults and typed
//! numbers work as in Settings. The tick is First setup's last row, pinned out of the
//! lines: the keyboard reaches it after a step's last row.
//!
//! The keyboard is in the step's field, on a picture button ([`Wizard::target`]) or on a
//! row. On the buttons Left and Right (or Tab) move, Enter chooses, and Enter on the one
//! in use goes on to the next step; Down goes to the rows, Up to the field or the tick.
//! In the field keys type (Ctrl+A, Ctrl+C, Ctrl+V and a held Backspace as in every field,
//! `text_entry.rs` through the menu), Enter goes on to the next step, Down or Tab to the
//! buttons. Page Up and Page Down change step from anywhere (Page Down on the last step
//! is Done), F2 opens all settings, Escape closes.
//! Positions are pixels of the SJK UI's 16:9 frame ([`Frame`]).

use super::classic_view::switch_text_on;
use super::help;
use super::setup_steps::{Act, Art, Button, Field, STEPS, Step, Words};
use super::sjk_view::{CONTROL_LEFT, KEYS_Y, LINE, ROWS_TOP, ROWS_WIDTH, ROWS_X};
use super::*;
use crate::graphics_quality;
use crate::menu::sjk::theme::{self, Palette};
use crate::menu::sjk::{Frame, TextTarget, color, kit, text, wrap};
use crate::menu_widgets::TextFamily;
use crate::setup_pictures;
use crate::sjk_chat_look::Measure;
use crate::text::TextFace;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign, UiEventKind};

/// The card.
const CARD: [f32; 4] = [340.0, 40.0, 1240.0, 950.0];
/// The card's inner margin, and where its text starts.
const MARGIN: f32 = 48.0;
const TEXT_X: f32 = CARD[0] + MARGIN;
const TEXT_WIDTH: f32 = CARD[2] - MARGIN * 2.0;
/// The steps' strip: its middle line, and a step's disc.
const STRIP_Y: f32 = CARD[1] + 42.0;
const DISC: f32 = 28.0;
/// What the step asks and its sentence.
const TITLE_TOP: f32 = CARD[1] + 70.0;
const SENTENCE_TOP: f32 = CARD[1] + 124.0;
/// Where a step's buttons (or its rows, when it has none) begin, and the lowest its
/// rows and their help reach.
const CONTENT_TOP: f32 = CARD[1] + 166.0;
/// The foot: its rule, and the middle of its tick and buttons.
const FOOT_RULE: f32 = CARD[1] + CARD[3] - 94.0;
const FOOT_MIDDLE: f32 = CARD[1] + CARD[3] - 47.0;
const CONTENT_BOTTOM: f32 = FOOT_RULE - 10.0;
const BUTTON_HEIGHT: f32 = 46.0;
/// The rows' column, centred in the card.
const ROWS_LEFT: f32 = CARD[0] + (CARD[2] - ROWS_WIDTH) * 0.5;
/// A picture button: the room round its words, its corner, and the room under its row.
const PAD: f32 = 20.0;
const RADIUS: f32 = 14.0;
const UNDER_BUTTONS: f32 = 12.0;
/// A preset choice under a button, the room the choices' strip takes under a button's
/// words, and the mark that says how a preset should run.
const CHIP_HEIGHT: f32 = 32.0;
const CHIP_STRIP: f32 = CHIP_HEIGHT + 14.0;
const RATING_HEIGHT: f32 = 24.0;
/// A step's field: the room it takes over the buttons, its own height and where it
/// starts and ends across the card, and a colour chip under it.
const FIELD_BLOCK: f32 = 118.0;
const FIELD_HEIGHT: f32 = 50.0;
const FIELD_X: f32 = TEXT_X + 92.0;
const FIELD_WIDTH: f32 = 470.0;
const FIELD_SIZE: f32 = 19.0;
const COLOUR_CHIP: f32 = 30.0;
/// The colour codes a name's chips offer: `^1` to `^7` (black, `^0`, does not show on
/// the game's dark grounds and many servers take it out of names).
const COLOUR_CODES: std::ops::RangeInclusive<u8> = 1..=7;
/// The last step's recap: its heading and a tile.
const RECAP_HEADING: f32 = 30.0;
const RECAP_TILE: f32 = 62.0;
const RECAP_BLOCK: f32 = 18.0 + RECAP_HEADING + RECAP_TILE;
/// A step whose pictures are drawn (a theme, an icon) grows them into room left
/// over, keeping this much free: a theme's page up to a little taller than wide, an
/// icon's ground by this much.
const ROOM_KEPT: f32 = 40.0;
const THEME_TALLEST: f32 = 1.12;
const ICON_GROWN: f32 = 56.0;
/// An icon's ground as first laid out, which its disc and picture are sized for.
const ICON_ART: f32 = 210.0;
/// A line of an aside or of the focused row's help.
const NOTE_LINE: f32 = 24.0;
const NOTE_CHARS: usize = 132;
/// The furthest a step that does not fill the card is lowered.
const MOST_LOWERED: f32 = 96.0;
/// Most steps the strip has room to name.
const MOST_STEPS: usize = 8;

/// The All settings button's target; Done is the way back's (900).
pub(super) const ALL_SETTINGS_TOKEN: u16 = 914;
const DONE_TOKEN: u16 = 900;
/// Choice `i` of a step's picture buttons answers to `TARGET_TOKEN + i`, step `i` of
/// the strip to `STEP_TOKEN + i`, and the foot's Next and Back to theirs: all clear of
/// the rows (0-499), the panel's own tokens (500 to 915) and a list's choices (1000 on).
const TARGET_TOKEN: u16 = 920;
const MOST_TARGETS: u16 = 16;
const STEP_TOKEN: u16 = 940;
const NEXT_TOKEN: u16 = 950;
const PREVIOUS_TOKEN: u16 = 951;
/// The step's field, colour chip `i` under it (`COLOUR_TOKEN + i`), and tile `i` of
/// the recap (`RECAP_TOKEN + i`, the step it leads to).
const FIELD_TOKEN: u16 = 952;
const COLOUR_TOKEN: u16 = 953;
const RECAP_TOKEN: u16 = 965;
/// No row has the keyboard: it is on a picture button.
const NO_ROW: usize = usize::MAX;

/// The dark ink on gold.
const INK: Color = Color::new(0.078, 0.063, 0.02, 1.0);

/// The pop-up's own state.
#[derive(Default)]
pub(super) struct Wizard {
    /// The step on show.
    step: usize,
    /// The choice among the step's picture buttons the keyboard is on; `None` while
    /// a row or the tick has it.
    target: Option<usize>,
    /// Which of the step's choices are in use, read with the rows' values.
    used: Vec<bool>,
    /// The step's aside, when it is shown.
    aside: Option<String>,
    /// What is typed in the step's field, whether the field has the keyboard, and
    /// what its setting held when it took it: an emptied field keeps that.
    field: String,
    on_field: bool,
    field_before: String,
    /// The last step's recap: a step and what it holds now.
    recap: Vec<(usize, String)>,
    /// The archived settings as last read ([`ViewerConsole::archive_revision`]): a
    /// change made elsewhere (EJK mode applying, a console command) is read again.
    revision: u64,
}

/// How a step's picture buttons are laid out: every button of a step has the same
/// size, set by what the widest and wordiest of them needs.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Plan {
    width: f32,
    gap: f32,
    /// Height of the picture at the top of a button.
    art: f32,
    /// Size of a button's name, of its lines, and how many characters fit a line.
    name: f32,
    words: f32,
    chars: usize,
    /// Lines of words a button with choices under them has room for.
    lines: usize,
    /// Lines more a button without choices has, in the room the others' choices take.
    more: usize,
    /// The buttons carry a rating, and choices under their words.
    rating: bool,
    chips: bool,
    /// How much taller the pictures can be: drawn ones (a theme, an icon) can.
    grows: f32,
}

impl Plan {
    fn of(step: &Step) -> Option<Self> {
        let count = step.buttons.len();
        if count == 0 {
            return None;
        }
        let gap = if count > 2 { 20.0 } else { 24.0 };
        let width = (TEXT_WIDTH - gap * (count - 1) as f32) / count as f32;
        let wide = width >= 480.0;
        let words = if wide { 16.0 } else { 15.0 };
        // Exo 2 runs about 0.42 of its size a character.
        let chars = ((width - PAD * 2.0) / (words * 0.44)) as usize;
        let art = step
            .buttons
            .iter()
            .map(|button| match button.art {
                Art::Picture(_) | Art::Preset => width * 9.0 / 16.0,
                Art::Theme(_) => 176.0,
                Art::Icon(_) => ICON_ART,
            })
            .fold(0.0, f32::max);
        let chips = step.buttons.iter().any(|button| button.act.options() > 1);
        // A button without choices writes on in the strip the others' choices take.
        let more = if chips {
            (CHIP_STRIP / (words + 6.0)) as usize
        } else {
            0
        };
        let lines = step
            .buttons
            .iter()
            .map(|button| {
                let lines = button
                    .lines
                    .iter()
                    .map(|line| wrap(line, chars).count())
                    .sum::<usize>();
                if button.act.options() > 1 {
                    lines
                } else {
                    lines.saturating_sub(more)
                }
            })
            .max()
            .unwrap_or(0)
            .min(3);
        Some(Self {
            width,
            gap,
            art,
            name: if wide { 30.0 } else { 25.0 },
            words,
            chars,
            lines,
            more,
            rating: step
                .buttons
                .iter()
                .any(|button| matches!(button.act, Act::Preset(_))),
            chips,
            grows: step
                .buttons
                .iter()
                .map(|button| match button.art {
                    Art::Theme(_) => (width * THEME_TALLEST - art).max(0.0),
                    Art::Icon(_) => ICON_GROWN,
                    Art::Picture(_) | Art::Preset => 0.0,
                })
                .fold(f32::MAX, f32::min),
        })
    }

    /// The plan with its pictures taller by what of `spare` they can take.
    fn grown(mut self, spare: f32) -> Self {
        self.art += (spare - ROOM_KEPT).clamp(0.0, self.grows.max(0.0));
        self
    }

    fn x(&self, button: usize) -> f32 {
        TEXT_X + button as f32 * (self.width + self.gap)
    }

    fn line(&self) -> f32 {
        self.words + 6.0
    }

    /// Tops, from a button's own top: its name, its rating, its lines, its choices.
    fn name_top(&self) -> f32 {
        self.art + 10.0
    }

    fn rating_top(&self) -> f32 {
        self.name_top() + self.name + 8.0
    }

    fn lines_top(&self) -> f32 {
        self.rating_top() + if self.rating { RATING_HEIGHT } else { 0.0 }
    }

    fn chips_top(&self) -> f32 {
        self.lines_top() + self.lines as f32 * self.line() + 8.0
    }

    fn height(&self) -> f32 {
        self.chips_top() + if self.chips { CHIP_STRIP } else { 8.0 }
    }
}

/// Where the parts of the step on show go, top to bottom.
struct Layout {
    /// Where the step's content begins: a step that does not fill the card sits a
    /// little lower, so the room left is shared.
    top: f32,
    /// Where its buttons begin, under its field when it has one.
    buttons_top: f32,
    /// Where the recap begins (the last step's).
    recap_top: f32,
    plan: Option<Plan>,
    /// The aside's top and how many lines it takes (none: no aside).
    aside: (f32, usize),
    rows_top: f32,
    /// Where the focused row's help goes.
    help_top: f32,
}

impl SettingsMenu {
    /// The step on show.
    fn step(&self) -> &'static Step {
        &STEPS[self.setup.step.min(STEPS.len() - 1)]
    }

    /// Lines the step's rows take.
    fn step_lines(&self) -> usize {
        self.classic
            .as_ref()
            .map_or(0, |classic| classic.lines.len())
    }

    fn layout(&self) -> Layout {
        let step = self.step();
        let lines = self
            .setup
            .aside
            .as_deref()
            .map_or(0, |aside| wrap(aside, NOTE_CHARS).count().min(2));
        let aside_height = if lines > 0 {
            lines as f32 * NOTE_LINE + 10.0
        } else {
            0.0
        };
        let rows = self.step_lines() as f32 * LINE;
        // The focused row's two lines of help, where the step shows them.
        let help = if rows > 0.0 && !step.explained {
            8.0 + 2.0 * NOTE_LINE
        } else {
            0.0
        };
        let field = if step.field.is_some() {
            FIELD_BLOCK
        } else {
            0.0
        };
        let recap = if self.setup.recap.is_empty() {
            0.0
        } else {
            RECAP_BLOCK
        };
        let room = CONTENT_BOTTOM - CONTENT_TOP - field - aside_height - rows - help - recap;
        // Drawn pictures take up room the step would leave empty.
        let plan = Plan::of(step).map(|plan| plan.grown(room - plan.height() - UNDER_BUTTONS));
        let buttons = plan.map_or(0.0, |plan| plan.height() + UNDER_BUTTONS);
        let spare = (room - buttons).max(0.0);
        let top = CONTENT_TOP + (spare * 0.4).min(MOST_LOWERED);
        let buttons_top = top + field;
        let mut y = buttons_top;
        match plan {
            // The aside belongs with the buttons: under them, over the rows.
            Some(plan) => {
                y += plan.height() + UNDER_BUTTONS;
                let aside = (y, lines);
                y += aside_height;
                Layout {
                    top,
                    buttons_top,
                    recap_top: y + rows,
                    plan: Some(plan),
                    aside,
                    rows_top: y,
                    help_top: y + rows + recap + 8.0,
                }
            }
            // A step of rows closes on its recap and its aside.
            None => Layout {
                top,
                buttons_top,
                recap_top: y + rows,
                plan: None,
                aside: (y + rows + recap + 18.0, lines),
                rows_top: y,
                help_top: y + rows + recap + 18.0 + aside_height,
            },
        }
    }

    /// Show step `step` (kept within the steps): its rows, and the keyboard on its
    /// picture button in use, or on its first row when it has no buttons.
    pub(super) fn show_step(&mut self, console: &ViewerConsole, step: usize) {
        let step = step.min(STEPS.len() - 1);
        self.setup.step = step;
        let table = &STEPS[step];
        let rows = self.rows();
        let mut lines = Vec::with_capacity(table.rows.len() * 2);
        for cvar in table.rows {
            let Some(row) = rows
                .iter()
                .position(|setting| setting.cvar.eq_ignore_ascii_case(cvar))
            else {
                continue;
            };
            lines.push(Line::Row(row));
            if table.explained
                && let Some(words) = help::help(cvar)
            {
                lines.push(Line::Note(words));
            }
        }
        if let Some(classic) = &mut self.classic {
            classic.lines = lines;
            classic.first = 0;
        }
        self.dropdown = None;
        self.editing = None;
        self.numeric = None;
        self.setup.on_field = false;
        self.refresh(console);
        self.focus_top();
    }

    /// Give the keyboard to the first thing of the step: its field, else its picture
    /// buttons, else its first row.
    fn focus_top(&mut self) {
        let table = self.step();
        if table.field.is_some() {
            self.focus_field();
        } else if table.buttons.is_empty() {
            self.focus_rows(1);
        } else {
            self.focus_buttons();
        }
    }

    /// Give the keyboard to the step's field.
    fn focus_field(&mut self) {
        if self.step().field.is_some() {
            self.setup.on_field = true;
            self.setup.target = None;
            self.selected = NO_ROW;
            crate::text_entry::deselect();
        }
    }

    /// Whether the step's field has the keyboard: keys type in it.
    pub(super) fn popup_on_field(&self) -> bool {
        self.popup() && self.setup.on_field
    }

    /// What is typed in the field that has the keyboard, for Ctrl+C.
    pub(super) fn popup_field_text(&self) -> Option<&str> {
        self.popup_on_field().then_some(self.setup.field.as_str())
    }

    /// A key typed in the step's field: Backspace takes the last character back, text
    /// is added (a name's length and characters, as the Character page's field), and
    /// the setting is written as it is typed; an emptied field is the name the field
    /// was entered with, until another is typed. Whether the key was the field's.
    pub(super) fn popup_field_key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
        console: &mut ViewerConsole,
    ) -> bool {
        let Some(field) = self.step().field.as_ref().filter(|_| self.popup_on_field()) else {
            return false;
        };
        let typed = text.filter(|text| text.chars().any(|c| !c.is_control()));
        match (key, typed) {
            (KeyCode::Backspace, _) => {
                self.setup.field.pop();
            }
            (_, Some(text)) if !console.control_held() => {
                crate::player_menu::append_legacy_name(&mut self.setup.field, text);
            }
            _ => return false,
        }
        self.write_field(field, console);
        true
    }

    /// Write what the field holds to its setting; emptied, what the setting held
    /// when the field took the keyboard.
    fn write_field(&mut self, field: &Field, console: &mut ViewerConsole) {
        let typed = match self.setup.field.trim() {
            "" => self.setup.field_before.as_str(),
            typed => typed,
        };
        if !typed.is_empty() && console.text_value(field.cvar) != Some(typed) {
            console.set_cvar(field.cvar, typed);
            self.setup.revision = console.archive_revision();
        }
    }

    /// Done on the last step: First setup was gone through to its end, so it no
    /// longer opens at start (the tick's setting); the pop-up closes.
    fn finish(&mut self, console: &mut ViewerConsole) -> SettingsResult {
        console.set_cvar(quick::HIDE_CVAR, "1");
        self.setup.on_field = false;
        self.back()
    }

    /// Whether the step on show is the last.
    fn on_last_step(&self) -> bool {
        self.setup.step + 1 >= STEPS.len()
    }

    /// The step on show.
    pub(crate) fn setup_step(&self) -> usize {
        self.setup.step
    }

    /// Show step `step` of the pop-up when it is the one on show (a page opened from
    /// it closed).
    pub(crate) fn show_setup_step(&mut self, console: &ViewerConsole, step: usize) {
        if self.popup() {
            self.show_step(console, step);
        }
    }

    /// Show step `step` with the keyboard on its choice `target`, or on its rows
    /// (shots).
    #[cfg(test)]
    pub(crate) fn setup_step_for_shot(
        &mut self,
        console: &ViewerConsole,
        step: usize,
        target: Option<usize>,
    ) {
        self.show_step(console, step);
        match target {
            Some(target) if !self.step().buttons.is_empty() => {
                self.setup.target = Some(target);
                self.setup.on_field = false;
                self.selected = NO_ROW;
            }
            Some(_) => {}
            None => self.focus_top(),
        }
    }

    /// What the pop-up does with `key` (tests of the menu around it).
    #[cfg(test)]
    pub(crate) fn popup_key_for_test(
        &mut self,
        key: KeyCode,
        console: &mut ViewerConsole,
    ) -> SettingsResult {
        self.popup_key(key, console).unwrap_or(SettingsResult::None)
    }

    /// Type `text` in the step's field, as keys would (shots and tests).
    #[cfg(test)]
    pub(crate) fn setup_type_for_shot(&mut self, console: &mut ViewerConsole, text: &str) {
        self.focus_field();
        self.setup.field.clear();
        self.popup_field_key(KeyCode::KeyX, Some(text), console);
    }

    /// Read what the buttons show of `console`: which choices are in use, and the
    /// step's aside. Called with every read of the rows' values.
    pub(super) fn read_setup(&mut self, console: &ViewerConsole) {
        if !self.popup() {
            return;
        }
        self.setup.revision = console.archive_revision();
        let step = self.step();
        self.setup.used.clear();
        self.setup.used.extend(step.targets().iter().map(|target| {
            step.buttons[target.button]
                .act
                .in_use(target.option, console)
        }));
        self.setup.aside = step
            .aside
            .as_ref()
            .filter(|aside| {
                aside
                    .when
                    .is_none_or(|cvar| console.cvar(cvar).is_some_and(switch_on))
            })
            .map(|aside| match aside.text {
                Words::Fixed(words) => words.to_owned(),
                Words::Made(make) => make(),
            });
        // The field shows its setting, but while it is being typed in: an import or
        // the Character page may have changed it.
        if let Some(field) = &step.field
            && !self.setup.on_field
        {
            let held = console
                .text_value(field.cvar)
                .filter(|held| !held.trim().is_empty())
                .unwrap_or(field.empty);
            self.setup.field.clear();
            self.setup.field.push_str(held);
            self.setup.field_before.clear();
            self.setup.field_before.push_str(held);
        }
        self.setup.recap.clear();
        if step.recap {
            let at = self.setup.step;
            self.setup.recap.extend(
                STEPS[..at]
                    .iter()
                    .enumerate()
                    .filter_map(|(index, step)| Some((index, held(step, console)?))),
            );
        }
    }

    /// Once a frame while the pop-up is up: read the settings again when one changed
    /// elsewhere since they were read (EJK mode applies a moment after its button, and
    /// sets many), so the buttons and rows never show a stale value.
    pub(crate) fn follow_console(&mut self, console: &ViewerConsole) {
        if self.popup() && self.setup.revision != console.archive_revision() {
            self.refresh(console);
        }
    }

    /// The choice a click on button `button` of the step takes: the one in use among
    /// its choices, else its usual one.
    fn body_target(&self, button: usize) -> usize {
        let step = self.step();
        let first = step.first_target(button);
        let act = step.buttons[button].act;
        let used =
            (0..act.options()).find(|option| self.setup.used.get(first + option) == Some(&true));
        first + used.unwrap_or_else(|| act.usual_option(graphics_quality::card_mib()))
    }

    /// Give the keyboard to the picture buttons: the choice in use, else the first
    /// button's.
    fn focus_buttons(&mut self) {
        if self.step().buttons.is_empty() {
            return;
        }
        let used = self.setup.used.iter().position(|used| *used);
        self.setup.target = Some(used.unwrap_or_else(|| self.body_target(0)));
        self.selected = NO_ROW;
    }

    /// Give the keyboard to the rows: the step's first (`direction` 1) or the tick
    /// at the foot (-1).
    fn focus_rows(&mut self, direction: i32) {
        self.setup.target = None;
        let Some(classic) = &self.classic else {
            return;
        };
        let row = if direction >= 0 {
            classic.rows().next()
        } else {
            classic.rows().last()
        };
        if let Some(row) = row {
            self.selected = row;
        }
    }

    /// Whether the keyboard is on a picture button, and whether that one is in use.
    pub(super) fn popup_on_button(&self) -> Option<bool> {
        let target = self.setup.target.filter(|_| self.popup())?;
        Some(self.setup.used.get(target) == Some(&true))
    }

    /// What the pop-up does with `key` before its rows see it: its steps' keys, the
    /// picture buttons' while the keyboard is on them, and the ways between the
    /// buttons and the rows. `None` for a key the rows take.
    pub(super) fn popup_key(
        &mut self,
        key: KeyCode,
        console: &mut ViewerConsole,
    ) -> Option<SettingsResult> {
        let step = self.setup.step;
        match key {
            // Next, which on the last step is Done.
            KeyCode::PageDown if self.on_last_step() => return Some(self.finish(console)),
            KeyCode::PageDown => {
                self.show_step(console, step + 1);
                return Some(SettingsResult::None);
            }
            KeyCode::PageUp => {
                self.show_step(console, step.saturating_sub(1));
                return Some(SettingsResult::None);
            }
            KeyCode::F2 => return Some(SettingsResult::AllSettings),
            // It has no search and no groups.
            KeyCode::Slash
            | KeyCode::NumpadDivide
            | KeyCode::F3
            | KeyCode::BracketLeft
            | KeyCode::BracketRight => return Some(SettingsResult::None),
            _ => {}
        }
        let count = self.step().targets().len();
        // In the field, the keys that do not type: Enter goes on, Down and Tab to the
        // buttons (the rows of a step without), Up round to the tick.
        if self.setup.on_field {
            match key {
                KeyCode::Enter | KeyCode::NumpadEnter if self.on_last_step() => {
                    return Some(self.finish(console));
                }
                KeyCode::Enter | KeyCode::NumpadEnter => self.show_step(console, step + 1),
                KeyCode::ArrowDown | KeyCode::Tab => {
                    self.setup.on_field = false;
                    self.refresh(console);
                    if count > 0 {
                        self.focus_buttons();
                    } else {
                        self.focus_rows(1);
                    }
                }
                KeyCode::ArrowUp => {
                    self.setup.on_field = false;
                    self.refresh(console);
                    self.focus_rows(-1);
                }
                KeyCode::Escape => return Some(self.back()),
                _ => {}
            }
            return Some(SettingsResult::None);
        }
        let field = self.step().field.is_some();
        if let Some(at) = self.setup.target.filter(|_| count > 0) {
            match key {
                KeyCode::ArrowLeft | KeyCode::KeyA => {
                    self.setup.target = Some((at + count - 1) % count);
                }
                KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::Tab => {
                    self.setup.target = Some((at + 1) % count);
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    return Some(self.pick(console, at, true));
                }
                KeyCode::ArrowDown | KeyCode::KeyS => self.focus_rows(1),
                KeyCode::ArrowUp | KeyCode::KeyW if field => self.focus_field(),
                KeyCode::ArrowUp | KeyCode::KeyW => self.focus_rows(-1),
                KeyCode::Escape => return Some(self.back()),
                _ => {}
            }
            return Some(SettingsResult::None);
        }
        if count == 0 {
            // Tab has nothing to move between.
            return (key == KeyCode::Tab).then_some(SettingsResult::None);
        }
        // Past either end of the rows are the buttons.
        let rows = self.classic.as_ref()?;
        let first = rows.rows().next() == Some(self.selected);
        let last = rows.rows().last() == Some(self.selected);
        match key {
            KeyCode::ArrowUp | KeyCode::KeyW if first => self.focus_buttons(),
            // Past the tick, the top of the step again.
            KeyCode::ArrowDown | KeyCode::KeyS if last => self.focus_top(),
            KeyCode::Tab => self.focus_buttons(),
            _ => return None,
        }
        Some(SettingsResult::None)
    }

    /// Choose choice `at` of the step's picture buttons. From the keyboard, the one
    /// already in use goes on to the next step instead: Enter chooses, Enter again
    /// continues.
    fn pick(&mut self, console: &mut ViewerConsole, at: usize, keyboard: bool) -> SettingsResult {
        let step = self.setup.step;
        let table = self.step();
        let Some(target) = table.targets().get(at).copied() else {
            return SettingsResult::None;
        };
        let act = table.buttons[target.button].act;
        self.setup.target = Some(at);
        self.setup.on_field = false;
        self.selected = NO_ROW;
        if keyboard && self.setup.used.get(at) == Some(&true) {
            self.show_step(console, step + 1);
            return SettingsResult::None;
        }
        match act {
            Act::Set { cvar, value } => {
                console.set_cvar(cvar, value);
            }
            Act::Theme(theme) => {
                self.wear_theme(console, theme);
                self.setup.target = Some(at);
                self.selected = NO_ROW;
                return SettingsResult::None;
            }
            Act::Preset(_) => {
                if let Some(level) = act.level(target.option) {
                    level.apply(console);
                }
            }
            Act::Import => return SettingsResult::OpenImport,
            Act::Character => return SettingsResult::OpenCharacter,
            Act::Next => {
                self.show_step(console, step + 1);
                return SettingsResult::None;
            }
        }
        self.refresh(console);
        SettingsResult::None
    }

    /// Wear theme `theme`, as its button does.
    pub(super) fn wear_theme(&mut self, console: &mut ViewerConsole, theme: theme::Theme) {
        console.set_cvar(theme::CVAR, theme.name());
        theme::sync(console);
        self.refresh(console);
    }

    /// What the pop-up does with the pointer on `token` before its rows see it: a
    /// picture button under the pointer takes the keyboard, a click chooses it; the
    /// strip's steps and the foot's Next and Back change step. `None` for the rest.
    pub(super) fn popup_pointer(
        &mut self,
        token: u16,
        kind: UiEventKind,
        console: &mut ViewerConsole,
    ) -> Option<SettingsResult> {
        let count = self.step().targets().len() as u16;
        let target = token
            .checked_sub(TARGET_TOKEN)
            .filter(|target| *target < count.min(MOST_TARGETS))
            .map(usize::from);
        let step = token
            .checked_sub(STEP_TOKEN)
            .filter(|step| usize::from(*step) < STEPS.len().min(MOST_STEPS))
            .map(usize::from);
        let colour = token
            .checked_sub(COLOUR_TOKEN)
            .filter(|index| usize::from(*index) < COLOUR_CODES.count())
            .map(|index| COLOUR_CODES.start() + index as u8);
        let recap = token
            .checked_sub(RECAP_TOKEN)
            .map(usize::from)
            .filter(|step| self.setup.recap.iter().any(|(at, _)| at == step));
        match kind {
            UiEventKind::HoverEnter | UiEventKind::Hover => {
                if self.drafting() {
                    return None;
                }
                // The field keeps the keyboard until a click takes it.
                if self.setup.on_field {
                    return Some(SettingsResult::None);
                }
                if let Some(target) = target {
                    self.setup.target = Some(target);
                    self.selected = NO_ROW;
                    return Some(SettingsResult::None);
                }
                // A row under the pointer takes the keyboard back.
                if self.setting_row(token).is_some() {
                    self.setup.target = None;
                }
                None
            }
            UiEventKind::Activate => {
                // The field, or a colour chip under it: the chip's code is typed.
                if let Some(field) = self
                    .step()
                    .field
                    .as_ref()
                    .filter(|_| token == FIELD_TOKEN || colour.is_some())
                {
                    self.focus_field();
                    match colour {
                        Some(code) => {
                            let code = format!("^{code}");
                            crate::player_menu::append_legacy_name(&mut self.setup.field, &code);
                            self.write_field(field, console);
                        }
                        None => crate::text_entry::clicked(&self.setup.field),
                    }
                    return Some(SettingsResult::None);
                }
                if self.setup.on_field {
                    self.setup.on_field = false;
                    self.refresh(console);
                }
                if let Some(target) = target {
                    return Some(self.pick(console, target, false));
                }
                if let Some(step) = step.or(recap) {
                    self.show_step(console, step);
                    return Some(SettingsResult::None);
                }
                match token {
                    DONE_TOKEN if self.on_last_step() => return Some(self.finish(console)),
                    NEXT_TOKEN => self.show_step(console, self.setup.step + 1),
                    PREVIOUS_TOKEN => self.show_step(console, self.setup.step.saturating_sub(1)),
                    _ => {
                        if self.setting_row(token).is_some() {
                            self.setup.target = None;
                        }
                        return None;
                    }
                }
                Some(SettingsResult::None)
            }
            _ => None,
        }
    }

    /// Draw First setup's pop-up at `reveal` opacity.
    pub(crate) fn append_sjk_popup(
        &mut self,
        target: TextTarget<'_>,
        viewport: [f32; 2],
        reveal: f32,
    ) {
        let frame = Frame::new(viewport);
        let layout = self.layout();
        let shift = [ROWS_LEFT - ROWS_X, layout.rows_top - ROWS_TOP];
        let rows = frame.shifted(shift[0], shift[1]);
        // An open list reaches no lower than the foot's rule.
        let list_bottom = CONTENT_BOTTOM - shift[1];
        let visible = self.step_lines().max(1);
        self.ui.begin_transparent(viewport);
        self.ui.push_opacity(reveal);
        kit::scrim(&mut self.ui, viewport);
        kit::card(&mut self.ui, &frame, CARD);
        self.popup_strip(&frame);
        self.popup_header(&frame);
        let measure = target.body_measure();
        self.popup_field(&frame, &measure, layout.top);
        if let Some(plan) = layout.plan {
            self.popup_buttons(&frame, &plan, layout.buttons_top);
        }
        self.popup_recap(&frame, layout.recap_top);
        self.popup_aside(&frame, layout.aside, layout.plan.is_some());
        let slots = self.sjk_rows(&rows, &measure, visible, list_bottom);
        if self.dropdown.is_none() && !self.step().explained {
            self.popup_help(&frame, layout.help_top);
        }
        self.popup_foot(&frame);
        self.sjk_keys(&frame.shifted(
            CARD[0] + CARD[2] - MARGIN - 1824.0,
            CARD[1] + CARD[3] + 22.0 - KEYS_Y,
        ));
        self.sjk_dropdown(&rows, &slots, visible, list_bottom);
        self.ui.pop_opacity();
        let focus = match self.setup.target {
            _ if self.setup.on_field => FIELD_TOKEN,
            Some(target) => TARGET_TOKEN + target as u16,
            None => self.selected as u16,
        };
        self.ui.finish(focus);
        target.append(&self.ui, viewport);
        self.sjk_controls = Some(rows.point(CONTROL_LEFT, 0.0)[0]);
    }

    /// The steps across the top of the card: each a numbered disc and its name, a
    /// line running on to the next; the one on show in gold, those before it lit. A
    /// click shows a step.
    fn popup_strip(&mut self, frame: &Frame) {
        let s = frame.s;
        let count = STEPS.len().min(MOST_STEPS);
        let width = TEXT_WIDTH / count as f32;
        let current = self.setup.step;
        for (index, step) in STEPS.iter().take(count).enumerate() {
            let x = TEXT_X + index as f32 * width;
            let token = STEP_TOKEN + index as u16;
            let lit = self.ui.token_hovered(token);
            let disc = frame.rect(x, STRIP_Y - DISC * 0.5, DISC, DISC);
            let list = self.ui.draw_list_mut();
            if index == current {
                let _ = list.push(DrawCommand::RoundedRect {
                    rect: disc,
                    radius: DISC * 0.5 * s,
                    color: color::gold(),
                });
            } else {
                if index < current {
                    let _ = list.push(DrawCommand::RoundedRect {
                        rect: disc,
                        radius: DISC * 0.5 * s,
                        color: color::alpha(color::holo(), 0.2),
                    });
                }
                let _ = list.push(DrawCommand::Border {
                    rect: disc,
                    radius: DISC * 0.5 * s,
                    width: 1.5 * s,
                    color: color::alpha(color::holo(), if lit { 0.9 } else { 0.4 }),
                });
            }
            // Rajdhani at 19 is about 9.4 pixels a character.
            let name_width = 9.4 * step.name.len() as f32 + 8.0;
            let from = x + DISC + 10.0 + name_width + 8.0;
            let to = x + width - 10.0;
            if index + 1 < count && to > from {
                // A whole pixel at least, on a small screen too.
                let mut line = frame.rect(from, STRIP_Y, to - from, 1.0);
                line.height = line.height.max(1.0);
                let _ = list.push(DrawCommand::SolidRect {
                    rect: line,
                    color: color::alpha(color::holo(), if index < current { 0.5 } else { 0.18 }),
                });
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", index + 1),
                disc,
                17.0 * s,
                match index.cmp(&current) {
                    std::cmp::Ordering::Equal => INK,
                    std::cmp::Ordering::Less => color::text(),
                    std::cmp::Ordering::Greater => color::muted(),
                },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", step.name),
                frame.rect(x + DISC + 10.0, STRIP_Y - 14.0, name_width + 20.0, 28.0),
                19.0 * s,
                if index == current || lit {
                    color::text()
                } else if index < current {
                    color::alpha(color::text(), 0.78)
                } else {
                    color::muted()
                },
                if index == current {
                    FontWeight::Semibold
                } else {
                    FontWeight::Regular
                },
                TextAlign::Start,
            );
            self.ui.hit_region(
                token,
                frame.rect(x - 6.0, STRIP_Y - 20.0, DISC + 16.0 + name_width, 40.0),
            );
        }
    }

    /// What the step asks, and its sentence.
    fn popup_header(&mut self, frame: &Frame) {
        let s = frame.s;
        let step = self.step();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", step.title),
            frame.rect(TEXT_X, TITLE_TOP, TEXT_WIDTH, 50.0),
            40.0 * s,
            color::text(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", step.sentence),
            frame.rect(TEXT_X, SENTENCE_TOP, TEXT_WIDTH, 26.0),
            18.0 * s,
            color::muted(),
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The step's field from `top`: its label, the field with what is typed as it is
    /// typed (a name's colour codes showing as characters), beside it the name as the
    /// game draws it, and under it the colours to add with a click.
    fn popup_field(&mut self, frame: &Frame, measure: &Measure<'_>, top: f32) {
        let Some(field) = &self.step().field else {
            return;
        };
        let s = frame.s;
        let focused = self.setup.on_field;
        let middle = top + FIELD_HEIGHT * 0.5;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", field.label),
            frame.rect(TEXT_X, middle - 14.0, FIELD_X - TEXT_X - 8.0, 28.0),
            19.0 * s,
            if focused {
                color::text()
            } else {
                color::alpha(color::text(), 0.88)
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        let rect = [FIELD_X, top, FIELD_WIDTH, FIELD_HEIGHT];
        let lit = self.ui.token_hovered(FIELD_TOKEN);
        kit::field(
            &mut self.ui,
            frame,
            rect,
            format_args!(""),
            focused || lit,
            false,
        );
        // What is typed, a piece at a time so that no piece holds a whole colour
        // code: `^` and its digit both show.
        let mut x = FIELD_X + 18.0;
        let caret = if focused { "_" } else { "" };
        let typed = format!("{}{caret}", self.setup.field);
        for piece in raw_pieces(&typed) {
            let width = measure.width(piece, FIELD_SIZE * s, TextFace::Regular) / s;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{piece}"),
                frame.rect(x, middle - 14.0, width + 12.0, 28.0),
                FIELD_SIZE * s,
                color::text(),
                FontWeight::Regular,
                TextAlign::Start,
            );
            x += width;
        }
        self.ui.hit_region(
            FIELD_TOKEN,
            frame.rect(FIELD_X, top, FIELD_WIDTH, FIELD_HEIGHT),
        );
        if !field.colours {
            return;
        }
        // As the game draws it.
        let shown_x = FIELD_X + FIELD_WIDTH + 36.0;
        let room = TEXT_X + TEXT_WIDTH - shown_x;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("In game"),
            frame.rect(shown_x, top - 2.0, room, 20.0),
            14.0 * s,
            color::muted(),
            FontWeight::Regular,
            TextAlign::Start,
        );
        let shown = match self.setup.field.trim() {
            "" => self.setup.field_before.as_str(),
            typed => typed,
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{shown}"),
            frame.rect(shown_x, top + 18.0, room, 34.0),
            26.0 * s,
            color::text(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        // The colours: a click types the code, and what follows takes the colour.
        let chips = top + FIELD_HEIGHT + 14.0;
        let mut x = FIELD_X;
        for (index, code) in COLOUR_CODES.enumerate() {
            let token = COLOUR_TOKEN + index as u16;
            let lit = self.ui.token_hovered(token);
            let rect = frame.rect(x, chips, COLOUR_CHIP + 22.0, COLOUR_CHIP);
            let colour = name_colour(code);
            let list = self.ui.draw_list_mut();
            let _ = list.push(DrawCommand::RoundedRect {
                rect,
                radius: COLOUR_CHIP * 0.5 * s,
                color: color::alpha(color::holo(), if lit { 0.2 } else { 0.07 }),
            });
            let _ = list.push(DrawCommand::Border {
                rect,
                radius: COLOUR_CHIP * 0.5 * s,
                width: if lit { 2.0 } else { 1.0 } * s,
                color: if lit {
                    Color::new(1.0, 1.0, 1.0, 0.8)
                } else {
                    color::alpha(color::holo(), 0.4)
                },
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 8.0, chips + COLOUR_CHIP * 0.5 - 7.0, 14.0, 14.0),
                radius: 7.0 * s,
                color: colour,
            });
            // The caret and the digit apart, so they show as typed.
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("^"),
                frame.rect(x + 27.0, chips, 10.0, COLOUR_CHIP),
                15.0 * s,
                color::text(),
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{code}"),
                frame.rect(x + 33.5, chips, 14.0, COLOUR_CHIP),
                15.0 * s,
                color::text(),
                FontWeight::Regular,
                TextAlign::Start,
            );
            self.ui.hit_region(token, rect);
            x += COLOUR_CHIP + 22.0 + 8.0;
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("A click adds a colour: the letters after it take it."),
            frame.rect(x + 10.0, chips, TEXT_X + TEXT_WIDTH - x - 10.0, COLOUR_CHIP),
            15.0 * s,
            color::muted(),
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The last step's recap from `top`: under its heading, a tile for each step
    /// before with what it holds now; a click shows that step.
    fn popup_recap(&mut self, frame: &Frame, top: f32) {
        if self.setup.recap.is_empty() {
            return;
        }
        let s = frame.s;
        let top = top + 18.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("YOUR SETUP"),
            frame.rect(TEXT_X, top, 400.0, RECAP_HEADING - 8.0),
            16.0 * s,
            color::muted(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let recap = std::mem::take(&mut self.setup.recap);
        let gap = 12.0;
        let count = recap.len() as f32;
        let width = (TEXT_WIDTH - gap * (count - 1.0)) / count;
        for (index, (step, value)) in recap.iter().enumerate() {
            let token = RECAP_TOKEN + *step as u16;
            let lit = self.ui.token_hovered(token);
            let x = TEXT_X + index as f32 * (width + gap);
            let y = top + RECAP_HEADING;
            let rect = frame.rect(x, y, width, RECAP_TILE);
            let list = self.ui.draw_list_mut();
            let _ = list.push(DrawCommand::RoundedRect {
                rect,
                radius: 10.0 * s,
                color: color::alpha(color::holo(), if lit { 0.16 } else { 0.06 }),
            });
            let _ = list.push(DrawCommand::Border {
                rect,
                radius: 10.0 * s,
                width: if lit { 2.0 } else { 1.0 } * s,
                color: if lit {
                    Color::new(1.0, 1.0, 1.0, 0.75)
                } else {
                    color::alpha(color::holo(), 0.35)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", STEPS[*step].name),
                frame.rect(x + 14.0, y + 7.0, width - 28.0, 20.0),
                14.0 * s,
                color::muted(),
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{value}"),
                frame.rect(x + 14.0, y + 28.0, width - 28.0, 28.0),
                21.0 * s,
                color::text(),
                FontWeight::Semibold,
                TextAlign::Start,
            );
            self.ui.hit_region(token, rect);
        }
        self.setup.recap = recap;
    }

    /// The step's aside, in up to two lines from `top`: across the card under its
    /// buttons (`wide`), else in the rows' column under them.
    fn popup_aside(&mut self, frame: &Frame, (top, lines): (f32, usize), wide: bool) {
        let Some(aside) = self.setup.aside.clone().filter(|_| lines > 0) else {
            return;
        };
        let (x, width) = if wide {
            (TEXT_X, TEXT_WIDTH)
        } else {
            (ROWS_LEFT + 22.0, TEXT_X + TEXT_WIDTH - ROWS_LEFT - 22.0)
        };
        for (index, line) in wrap(&aside, NOTE_CHARS).take(lines).enumerate() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(x, top + index as f32 * NOTE_LINE, width, NOTE_LINE),
                16.0 * frame.s,
                color::alpha(color::gold_bright(), 0.92),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The step's picture buttons, side by side from `top`.
    fn popup_buttons(&mut self, frame: &Frame, plan: &Plan, top: f32) {
        let step = self.step();
        let recommended = graphics_quality::recommended();
        let mib = graphics_quality::card_mib();
        let mut pictures = false;
        for (index, button) in step.buttons.iter().enumerate() {
            let first = step.first_target(index);
            let options = button.act.options();
            let body = self.body_target(index);
            // The choice the button speaks for now: the one the keyboard is on, else
            // the one a click would take.
            let on = self
                .setup
                .target
                .filter(|target| (first..first + options).contains(target));
            let shown = on.unwrap_or(body);
            let used = (first..first + options).any(|target| self.used(target));
            let token = TARGET_TOKEN + body as u16;
            let lit = on.is_some()
                || (first..first + options)
                    .any(|target| self.ui.token_hovered(TARGET_TOKEN + target as u16));
            let level = button.act.level(shown - first);
            let palette = match button.art {
                Art::Theme(theme) => Some(theme::preview(theme)),
                _ => None,
            };
            let x = plan.x(index);
            let tile = frame.rect(x, top, plan.width, plan.height());
            let radius = RADIUS * frame.s;
            let s = frame.s;
            let list = self.ui.draw_list_mut();
            let _ = list.push(DrawCommand::RoundedRect {
                rect: tile,
                radius,
                color: match &palette {
                    Some(palette) => palette.space,
                    None => color::alpha(color::space(), if lit { 0.72 } else { 0.55 }),
                },
            });
            // The picture, cut to the button's corners.
            let art = frame.rect(x, top, plan.width, plan.art);
            let _ = list.push(DrawCommand::PushRoundedClip { rect: tile, radius });
            match button.art {
                Art::Picture(picture) => {
                    pictures = true;
                    self.popup_picture(art, picture);
                }
                Art::Preset => {
                    pictures = true;
                    if let Some(level) = level {
                        self.popup_picture(art, setup_pictures::Picture::of_level(level));
                    }
                }
                Art::Theme(_) => {
                    if let Some(palette) = &palette {
                        self.popup_theme(frame, [x, top, plan.width, plan.art], palette);
                    }
                }
                Art::Icon(name) => self.popup_icon(frame, [x, top, plan.width, plan.art], name),
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::PopClip);
            self.popup_button_words(frame, plan, button, [x, top], palette.as_ref());
            if let Some(level) = level {
                let rating = level.rating(mib);
                self.popup_rating(frame, x + PAD, top + plan.rating_top(), rating);
                if (0..options).any(|option| button.act.level(option) == Some(recommended)) {
                    self.popup_mark(frame, [x + 10.0, top + 10.0], "Recommended", false);
                }
            }
            if used {
                self.popup_mark(frame, [x + plan.width - 10.0, top + 10.0], "In use", true);
            }
            // Its edge: gold in use, bright under the keyboard or the pointer.
            let gold = palette
                .as_ref()
                .map_or(color::gold(), |palette| palette.gold);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: tile,
                radius,
                width: if used || lit { 2.5 } else { 1.5 } * s,
                color: match (used, lit) {
                    (true, _) => gold,
                    (false, true) => Color::new(1.0, 1.0, 1.0, 0.75),
                    (false, false) => color::alpha(
                        palette
                            .as_ref()
                            .map_or(color::holo(), |palette| palette.holo),
                        0.4,
                    ),
                },
            });
            // The keyboard's ring, outside the edge, so it shows on the one in use too.
            if on.is_some() {
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(x - 5.0, top - 5.0, plan.width + 10.0, plan.height() + 10.0),
                    radius: (RADIUS + 5.0) * s,
                    width: 1.5 * s,
                    color: Color::new(1.0, 1.0, 1.0, 0.55),
                });
            }
            self.ui.hit_region(token, tile);
            if options > 1 {
                self.popup_chips(frame, plan, button, index, [x, top + plan.chips_top()]);
            }
        }
        if pictures {
            setup_pictures::request();
        }
    }

    /// Whether choice `target` of the step is in use.
    fn used(&self, target: usize) -> bool {
        self.setup.used.get(target) == Some(&true)
    }

    /// A bundled picture over `rect`, on a dark ground until it is uploaded.
    fn popup_picture(&mut self, rect: Rect, picture: setup_pictures::Picture) {
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::SolidRect {
            rect,
            color: Color::new(0.0, 0.0, 0.0, 0.55),
        });
        let _ = list.push(DrawCommand::TexturedQuad {
            rect,
            texture: picture.texture(),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
    }

    /// A theme's picture over `rect` (frame pixels): a panel of its lit glass on its
    /// ground, strokes of its text and line-work, a pill of its gold.
    fn popup_theme(&mut self, frame: &Frame, rect: [f32; 4], palette: &Palette) {
        let [x, y, width, height] = rect;
        let s = frame.s;
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 14.0, y + 14.0, width - 28.0, height - 20.0),
            radius: 10.0 * s,
            color: palette.glass_lit,
        });
        let _ = list.push(DrawCommand::Border {
            rect: frame.rect(x + 14.0, y + 14.0, width - 28.0, height - 20.0),
            radius: 10.0 * s,
            width: s,
            color: color::alpha(palette.holo, 0.5),
        });
        let inner = x + 32.0;
        let room = width - 64.0;
        // A title, three lines of a page, and the page's one gold button.
        for (down, share, colour, thick) in [
            (0.2, 0.46, palette.text, 8.0),
            (0.38, 0.8, color::alpha(palette.holo, 0.85), 4.0),
            (0.48, 0.62, color::alpha(palette.holo, 0.6), 4.0),
            (0.58, 0.7, color::alpha(palette.muted, 0.6), 4.0),
        ] {
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(inner, y + height * down, room * share, thick),
                radius: thick * 0.5 * s,
                color: colour,
            });
        }
        // A taller page has a row with its switch on, over the button.
        if height >= 250.0 {
            let row = y + height * 0.72;
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(inner, row - 2.0, room * 0.4, 4.0),
                radius: 2.0 * s,
                color: color::alpha(palette.text, 0.7),
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(inner + room - 40.0, row - 9.0, 40.0, 18.0),
                radius: 9.0 * s,
                color: palette.gold,
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(inner + room - 18.0, row - 6.0, 12.0, 12.0),
                radius: 6.0 * s,
                color: palette.text,
            });
        }
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(x + width - 32.0 - 66.0, y + height - 44.0, 66.0, 24.0),
            radius: 12.0 * s,
            color: palette.gold,
        });
    }

    /// A settings picture, large, in the middle of `rect` (frame pixels) on a disc.
    fn popup_icon(&mut self, frame: &Frame, rect: [f32; 4], name: &str) {
        let [x, y, width, height] = rect;
        // Larger on a ground grown taller.
        let larger = (height / ICON_ART).clamp(1.0, 1.4);
        let (disc, icon) = (118.0 * larger, 76.0 * larger);
        let (cx, cy) = (x + width * 0.5, y + height * 0.5 + 4.0);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 0.0,
            color: color::alpha(color::holo(), 0.05),
        });
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(cx - disc * 0.5, cy - disc * 0.5, disc, disc),
            radius: disc * 0.5 * frame.s,
            color: color::alpha(color::holo(), 0.1),
        });
        let _ = list.push(DrawCommand::Border {
            rect: frame.rect(cx - disc * 0.5, cy - disc * 0.5, disc, disc),
            radius: disc * 0.5 * frame.s,
            width: 1.5 * frame.s,
            color: color::alpha(color::holo(), 0.45),
        });
        if let Some(picture) = crate::settings_icons::picture(name) {
            let _ =
                list.push(picture.quad(frame.rect(cx - icon * 0.5, cy - icon * 0.5, icon, icon)));
        }
    }

    /// A button's name and its lines, under its picture; a theme's in its own colours.
    fn popup_button_words(
        &mut self,
        frame: &Frame,
        plan: &Plan,
        button: &Button,
        [x, top]: [f32; 2],
        palette: Option<&Palette>,
    ) {
        let s = frame.s;
        let room = plan.width - PAD * 2.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", button.name),
            frame.rect(x + PAD, top + plan.name_top(), room, plan.name + 8.0),
            plan.name * s,
            palette.map_or(color::text(), |palette| palette.text),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        // A button without choices writes on where the others have theirs.
        let most = if button.act.options() > 1 {
            plan.lines
        } else {
            plan.lines + plan.more
        };
        let lines = button
            .lines
            .iter()
            .flat_map(|line| wrap(line, plan.chars))
            .take(most);
        for (index, line) in lines.enumerate() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(
                    x + PAD,
                    top + plan.lines_top() + index as f32 * plan.line(),
                    room,
                    plan.line(),
                ),
                plan.words * s,
                palette.map_or(color::alpha(color::text(), 0.8), |palette| palette.muted),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// How a preset should run on this PC: its colour's dot and the words.
    fn popup_rating(&mut self, frame: &Frame, x: f32, top: f32, rating: graphics_quality::Rating) {
        let s = frame.s;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top + 7.0, 10.0, 10.0),
            radius: 5.0 * s,
            color: rating.color(),
        });
        let words = rating.words();
        let (head, rest) = words.split_at(words.chars().next().map_or(0, char::len_utf8));
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}{rest}", head.to_uppercase()),
            frame.rect(x + 18.0, top, 320.0, RATING_HEIGHT),
            15.0 * s,
            rating.color(),
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// A small mark over a button's picture, at `at` (its top left corner, or its top
    /// right when `gold`): "In use" in gold, or a note on dark glass.
    fn popup_mark(&mut self, frame: &Frame, [x, y]: [f32; 2], label: &str, gold: bool) {
        let s = frame.s;
        // Exo 2 at 14 is about 7 pixels a character.
        let width = 7.2 * label.len() as f32 + 24.0;
        let left = if gold { x - width } else { x };
        let rect = frame.rect(left, y, width, 26.0);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius: 13.0 * s,
            color: if gold {
                color::gold()
            } else {
                Color::new(0.0, 0.0, 0.0, 0.72)
            },
        });
        if !gold {
            let _ = list.push(DrawCommand::Border {
                rect,
                radius: 13.0 * s,
                width: s,
                color: color::alpha(color::gold_bright(), 0.8),
            });
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{label}"),
            rect,
            14.0 * s,
            if gold { INK } else { color::gold_bright() },
            FontWeight::Semibold,
            TextAlign::Center,
        );
    }

    /// The choices under button `index` (its presets), each a chip with its rating's
    /// dot: gold when in use, ringed under the keyboard or the pointer.
    fn popup_chips(
        &mut self,
        frame: &Frame,
        plan: &Plan,
        button: &Button,
        index: usize,
        [x, top]: [f32; 2],
    ) {
        let Act::Preset(levels) = button.act else {
            return;
        };
        let s = frame.s;
        let first = self.step().first_target(index);
        // Nearer the button's edges than its words, for the names' room.
        let (gap, side) = (6.0, 12.0);
        let room = plan.width - side * 2.0;
        let width = (room - gap * (levels.len() - 1) as f32) / levels.len() as f32;
        let mib = graphics_quality::card_mib();
        for (option, (level, name)) in levels.iter().enumerate() {
            let target = first + option;
            let token = TARGET_TOKEN + target as u16;
            let used = self.used(target);
            let lit = self.setup.target == Some(target) || self.ui.token_hovered(token);
            let left = x + side + option as f32 * (width + gap);
            let rect = frame.rect(left, top, width, CHIP_HEIGHT);
            let list = self.ui.draw_list_mut();
            let _ = list.push(DrawCommand::RoundedRect {
                rect,
                radius: CHIP_HEIGHT * 0.5 * s,
                color: if used {
                    color::gold()
                } else {
                    color::alpha(color::holo(), if lit { 0.16 } else { 0.06 })
                },
            });
            let _ = list.push(DrawCommand::Border {
                rect,
                radius: CHIP_HEIGHT * 0.5 * s,
                width: if lit { 2.0 } else { 1.0 } * s,
                color: if lit {
                    Color::new(1.0, 1.0, 1.0, 0.8)
                } else {
                    color::alpha(color::holo(), 0.4)
                },
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(left + 9.0, top + CHIP_HEIGHT * 0.5 - 4.0, 8.0, 8.0),
                radius: 4.0 * s,
                color: if used { INK } else { level.rating(mib).color() },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 19.0, top, width - 23.0, CHIP_HEIGHT),
                15.0 * s,
                if used { INK } else { color::text() },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
        }
    }

    /// What the focused row does, in up to two lines from `top`, while a row has the
    /// keyboard.
    fn popup_help(&mut self, frame: &Frame, top: f32) {
        let s = frame.s;
        if self.setup.target.is_some()
            || self.setup.on_field
            || top + NOTE_LINE * 2.0 > CONTENT_BOTTOM + 2.0
        {
            return;
        }
        let Some(help) = self
            .rows()
            .get(self.selected)
            .filter(|_| self.shows(self.selected))
            .and_then(|setting| help::help(setting.cvar))
        else {
            return;
        };
        for (index, line) in wrap(help, 96).take(2).enumerate() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(
                    ROWS_LEFT,
                    top + index as f32 * NOTE_LINE,
                    ROWS_WIDTH,
                    NOTE_LINE,
                ),
                16.0 * s,
                color::alpha(color::text(), 0.72),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The foot under its rule: the pinned "Don't show at start" tick on the left,
    /// focused with the row's band, then All settings, Back and Next on the right
    /// (Done on the last step).
    fn popup_foot(&mut self, frame: &Frame) {
        let s = frame.s;
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(CARD[0] + 24.0, FOOT_RULE, CARD[2] - 48.0, 1.0),
            color: color::alpha(color::holo(), 0.2),
        });
        let pinned = self.classic.as_ref().and_then(|classic| classic.pinned);
        if let Some(row) = pinned {
            let focused = row == self.selected;
            let ticked = self
                .values
                .get(row)
                .is_some_and(|value| switch_text_on(value));
            let label = self.rows().get(row).map_or("", |setting| {
                help::classic_label(setting.cvar, crate::terms::say_static(setting.label)).0
            });
            let band = [CARD[0] + 24.0, FOOT_MIDDLE - 26.0, 400.0, 52.0];
            if focused {
                kit::band(&mut self.ui, frame, band);
            }
            let after = kit::tick(&mut self.ui, frame, TEXT_X, FOOT_MIDDLE, ticked, focused);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(after + 14.0, FOOT_MIDDLE - 14.0, 330.0, 28.0),
                19.0 * s,
                if focused || ticked {
                    color::text()
                } else {
                    color::alpha(color::text(), 0.88)
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            let [x, y, width, height] = band;
            self.ui
                .hit_region(row as u16, frame.rect(x, y, width, height));
        }
        let right = CARD[0] + CARD[2] - MARGIN;
        let top = FOOT_MIDDLE - BUTTON_HEIGHT * 0.5;
        let last = self.on_last_step();
        kit::button(
            &mut self.ui,
            frame,
            [right - 160.0, top, 160.0, BUTTON_HEIGHT],
            if last { "Done" } else { "Next" },
            true,
            true,
            false,
            if last { DONE_TOKEN } else { NEXT_TOKEN },
        );
        kit::button(
            &mut self.ui,
            frame,
            [right - 160.0 - 12.0 - 120.0, top, 120.0, BUTTON_HEIGHT],
            "Back",
            false,
            self.setup.step > 0,
            false,
            PREVIOUS_TOKEN,
        );
        kit::button(
            &mut self.ui,
            frame,
            [
                right - 160.0 - 12.0 - 120.0 - 12.0 - 160.0,
                top,
                160.0,
                BUTTON_HEIGHT,
            ],
            "All settings",
            false,
            true,
            false,
            ALL_SETTINGS_TOKEN,
        );
    }
}

/// What `step` holds now, for the recap: its field's text, else the name of its
/// picture button in use (with the choice's own name when it has several); `None` for
/// a step with neither, or whose buttons are none of them in use.
fn held(step: &Step, console: &ViewerConsole) -> Option<String> {
    if let Some(field) = &step.field {
        return Some(
            console
                .text_value(field.cvar)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(field.empty)
                .to_owned(),
        );
    }
    step.buttons.iter().find_map(|button| {
        let option =
            (0..button.act.options()).find(|option| button.act.in_use(*option, console))?;
        Some(match button.act {
            Act::Preset(levels) if levels.len() > 1 => {
                format!("{} {}", button.name, levels[option].1)
            }
            _ => button.name.to_owned(),
        })
    })
}

/// `text` cut so that no piece holds a whole colour code (`^` then a digit): drawn a
/// piece at a time, the code shows as its two characters instead of colouring.
fn raw_pieces(text: &str) -> impl Iterator<Item = &str> {
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut at = 0;
    std::iter::from_fn(move || {
        if start >= bytes.len() {
            return None;
        }
        while at < bytes.len() {
            at += 1;
            if bytes[at - 1] == b'^' && bytes.get(at).is_some_and(u8::is_ascii_digit) {
                break;
            }
        }
        let piece = &text[start..at];
        start = at;
        Some(piece)
    })
}

/// The colour a name's code `code` gives, as the game's palette has it.
fn name_colour(code: u8) -> Color {
    let [r, g, b, a] = crate::text::quake_color(code);
    Color::new(r, g, b, a)
}

// The strip, the title and the sentence in order over the content.
const _: () = assert!(STRIP_Y + 20.0 <= TITLE_TOP && TITLE_TOP + 50.0 <= SENTENCE_TOP);
const _: () = assert!(SENTENCE_TOP + 26.0 <= CONTENT_TOP - 8.0);
// The rows' column inside the card, the foot under the content.
const _: () = assert!(ROWS_LEFT >= CARD[0] + MARGIN);
const _: () = assert!(ROWS_LEFT + ROWS_WIDTH + 18.0 <= CARD[0] + CARD[2] - 24.0);
const _: () = assert!(FOOT_MIDDLE + BUTTON_HEIGHT * 0.5 < CARD[1] + CARD[3]);
// The card and the keys under it inside the frame.
const _: () = assert!(CARD[0] + CARD[2] <= 1824.0 && CARD[1] + CARD[3] + 22.0 + 28.0 <= 1080.0);
// The steps fit the strip, the choices their tokens.
const _: () = assert!(STEPS.len() <= MOST_STEPS);
const _: () = assert!(TARGET_TOKEN + MOST_TARGETS <= STEP_TOKEN);
const _: () = assert!(STEP_TOKEN + (MOST_STEPS as u16) <= NEXT_TOKEN);
// The field's tokens and the recap's stay apart, under a list's choices.
const _: () = assert!(COLOUR_TOKEN + 8 <= RECAP_TOKEN && RECAP_TOKEN + (MOST_STEPS as u16) < 1000);
// The field and the name beside it inside the card.
const _: () = assert!(FIELD_X + FIELD_WIDTH + 36.0 + 200.0 <= TEXT_X + TEXT_WIDTH);
const _: () = assert!(FIELD_HEIGHT + 14.0 + COLOUR_CHIP <= FIELD_BLOCK - 16.0);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics_quality::Level;

    fn popup() -> (tempfile::TempDir, ViewerConsole, SettingsMenu) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut menu = SettingsMenu::new();
        menu.open_first_setup_popup(&console);
        (directory, console, menu)
    }

    fn hide_row(menu: &SettingsMenu) -> usize {
        menu.rows()
            .iter()
            .position(|setting| setting.cvar == quick::HIDE_CVAR)
            .unwrap()
    }

    fn row_of(menu: &SettingsMenu, cvar: &str) -> usize {
        menu.rows()
            .iter()
            .position(|setting| setting.cvar.eq_ignore_ascii_case(cvar))
            .unwrap()
    }

    /// The step whose buttons do `wanted`.
    fn step_with(wanted: impl Fn(&Act) -> bool) -> usize {
        STEPS
            .iter()
            .position(|step| step.buttons.iter().any(|button| wanted(&button.act)))
            .unwrap()
    }

    /// The pop-up opens on its first step, the keyboard on the look in use, and shows
    /// only that step's rows; the tick stays pinned at the foot.
    #[test]
    fn the_popup_opens_on_its_first_step_with_only_its_rows() {
        let (_directory, _console, menu) = popup();
        assert!(menu.popup() && menu.on_first_setup());
        assert_eq!(menu.setup_step(), 0);
        assert_eq!(menu.setup.target, Some(0), "a new profile wears SJK");
        assert_eq!(menu.popup_on_button(), Some(true));
        let hide = hide_row(&menu);
        let classic = menu.classic.as_ref().unwrap();
        assert_eq!(classic.pinned, Some(hide));
        let rows: Vec<usize> = classic.rows().collect();
        let wanted: Vec<usize> = STEPS[0]
            .rows
            .iter()
            .map(|cvar| row_of(&menu, cvar))
            .chain([hide])
            .collect();
        assert_eq!(rows, wanted);
        // No row looks focused while a button has the keyboard.
        assert!(!menu.shows(menu.selected));
    }

    /// Every step shows its rows without scrolling, inside the card over the foot.
    #[test]
    fn every_step_fits_the_card() {
        let (_directory, console, mut menu) = popup();
        for (index, step) in STEPS.iter().enumerate() {
            menu.show_step(&console, index);
            let layout = menu.layout();
            let lines = menu.step_lines();
            assert_eq!(
                lines,
                step.rows.len() * if step.explained { 2 } else { 1 },
                "{}: a row of the step is not a First setup row",
                step.title
            );
            let rows_bottom = layout.rows_top + lines as f32 * LINE;
            assert!(
                rows_bottom <= CONTENT_BOTTOM,
                "{}: {rows_bottom}",
                step.title
            );
            let (aside_top, aside_lines) = layout.aside;
            assert!(
                aside_top + aside_lines as f32 * NOTE_LINE <= CONTENT_BOTTOM,
                "{}",
                step.title
            );
            // The focused row's two lines of help have room where a step shows them.
            if !step.explained && !step.rows.is_empty() {
                assert!(
                    layout.help_top + 2.0 * NOTE_LINE <= CONTENT_BOTTOM + 2.0,
                    "{}: {}",
                    step.title,
                    layout.help_top
                );
            }
            if let Some(plan) = layout.plan {
                assert!(plan.x(step.buttons.len() - 1) + plan.width <= TEXT_X + TEXT_WIDTH + 0.5);
                assert!(step.targets().len() <= usize::from(MOST_TARGETS));
                assert!(
                    layout.buttons_top + plan.height() <= CONTENT_BOTTOM,
                    "{}",
                    step.title
                );
                // Its field stands over its buttons.
                assert_eq!(
                    layout.buttons_top - layout.top,
                    if step.field.is_some() {
                        FIELD_BLOCK
                    } else {
                        0.0
                    }
                );
            }
            assert!(layout.top >= CONTENT_TOP);
            if step.recap {
                assert!(!menu.setup.recap.is_empty(), "{}", step.title);
                assert!(layout.recap_top >= rows_bottom);
                assert!(layout.recap_top + RECAP_BLOCK <= aside_top);
            }
        }
        // Worst case: the aside a step shows only under EJK mode.
        let mut console = console;
        console.set_cvar(crate::ejk_mode::CVAR, "1");
        for (index, step) in STEPS.iter().enumerate() {
            menu.show_step(&console, index);
            let layout = menu.layout();
            let bottom = layout.rows_top + menu.step_lines() as f32 * LINE;
            assert!(bottom <= CONTENT_BOTTOM, "{}", step.title);
        }
    }

    /// Page Down and Page Up change step and stop at the ends; the foot's Next and
    /// Back and the strip's steps do the same for the pointer.
    #[test]
    fn the_steps_change_by_key_and_by_pointer() {
        let (_directory, mut console, mut menu) = popup();
        assert!(matches!(
            menu.popup_key(KeyCode::PageUp, &mut console),
            Some(SettingsResult::None)
        ));
        assert_eq!(menu.setup_step(), 0);
        for step in 1..STEPS.len() {
            menu.popup_key(KeyCode::PageDown, &mut console);
            assert_eq!(menu.setup_step(), step);
        }
        // A step of rows opens with the keyboard on its first row.
        assert_eq!(menu.setup.target, None);
        assert_eq!(menu.selected, row_of(&menu, STEPS[STEPS.len() - 1].rows[0]));
        menu.popup_pointer(PREVIOUS_TOKEN, UiEventKind::Activate, &mut console);
        assert_eq!(menu.setup_step(), STEPS.len() - 2);
        menu.popup_pointer(NEXT_TOKEN, UiEventKind::Activate, &mut console);
        assert_eq!(menu.setup_step(), STEPS.len() - 1);
        menu.popup_pointer(STEP_TOKEN + 2, UiEventKind::Activate, &mut console);
        assert_eq!(menu.setup_step(), 2);
        assert!(
            menu.popup_pointer(
                STEP_TOKEN + STEPS.len() as u16,
                UiEventKind::Activate,
                &mut console
            )
            .is_none()
        );
        // F2 leaves for all settings; the search's and the groups' keys do nothing.
        assert!(matches!(
            menu.popup_key(KeyCode::F2, &mut console),
            Some(SettingsResult::AllSettings)
        ));
        for key in [KeyCode::Slash, KeyCode::BracketLeft, KeyCode::BracketRight] {
            assert!(matches!(
                menu.popup_key(key, &mut console),
                Some(SettingsResult::None)
            ));
        }
    }

    /// The keyboard's order on a step with buttons: Left and Right (and Tab) through
    /// the buttons, Down to the first row, on down the rows to the tick, Down again
    /// back to the buttons; Up from the buttons to the tick.
    #[test]
    fn the_keyboard_goes_round_the_buttons_the_rows_and_the_tick() {
        let (_directory, mut console, mut menu) = popup();
        let hide = hide_row(&menu);
        assert_eq!(menu.setup.target, Some(0));
        menu.popup_key(KeyCode::ArrowRight, &mut console);
        assert_eq!(menu.setup.target, Some(1));
        menu.popup_key(KeyCode::ArrowRight, &mut console);
        assert_eq!(menu.setup.target, Some(0), "it wraps");
        menu.popup_key(KeyCode::Tab, &mut console);
        assert_eq!(menu.setup.target, Some(1));
        menu.popup_key(KeyCode::ArrowLeft, &mut console);
        assert_eq!(menu.setup.target, Some(0));
        // Down: the step's first row; the rows take their own keys from there.
        menu.popup_key(KeyCode::ArrowDown, &mut console);
        assert_eq!(menu.setup.target, None);
        let rows: Vec<usize> = STEPS[0]
            .rows
            .iter()
            .map(|cvar| row_of(&menu, cvar))
            .collect();
        assert_eq!(menu.selected, rows[0]);
        assert_eq!(menu.popup_on_button(), None);
        assert!(menu.popup_key(KeyCode::ArrowDown, &mut console).is_none());
        menu.step_selection(1);
        assert_eq!(menu.selected, rows[1]);
        menu.step_selection(1);
        assert_eq!(menu.selected, hide);
        // Down from the tick, and Up from the first row, are the buttons.
        assert!(menu.popup_key(KeyCode::ArrowDown, &mut console).is_some());
        assert_eq!(menu.setup.target, Some(0));
        menu.popup_key(KeyCode::ArrowUp, &mut console);
        assert_eq!((menu.setup.target, menu.selected), (None, hide));
        menu.selected = rows[0];
        assert!(menu.popup_key(KeyCode::ArrowUp, &mut console).is_some());
        assert_eq!(menu.setup.target, Some(0));
        // A step of rows only: Up from its first row wraps to the tick, as before.
        let plain = STEPS
            .iter()
            .position(|step| step.buttons.is_empty())
            .unwrap();
        menu.show_step(&console, plain);
        assert!(menu.popup_key(KeyCode::ArrowUp, &mut console).is_none());
        menu.step_selection(-1);
        assert_eq!(menu.selected, hide);
        assert!(matches!(
            menu.popup_key(KeyCode::Tab, &mut console),
            Some(SettingsResult::None)
        ));
    }

    /// Choosing EJK turns `ui_ejkMode` on and the pop-up stays on its step; choosing
    /// SJK turns it off. Enter on the look in use goes on to the next step.
    #[test]
    fn picking_a_look_sets_ejk_mode_and_the_popup_stays() {
        let (_directory, mut console, mut menu) = popup();
        let cvar = crate::ejk_mode::CVAR;
        assert_eq!(console.bool_cvar(cvar), Some(false));
        menu.popup_key(KeyCode::ArrowRight, &mut console);
        assert_eq!(menu.popup_on_button(), Some(false));
        menu.popup_key(KeyCode::Enter, &mut console);
        assert_eq!(console.bool_cvar(cvar), Some(true));
        assert!(menu.popup() && menu.setup_step() == 0);
        assert_eq!(menu.setup.used, [false, true]);
        // The mode applies a moment later and sets many settings: they are read again.
        crate::ejk_mode::follow(&mut console);
        assert_eq!(
            console.text_value(crate::menu::style::CVAR),
            Some("classic")
        );
        let style = row_of(&menu, crate::menu::style::CVAR);
        assert_ne!(menu.values[style].to_ascii_lowercase(), "classic");
        menu.follow_console(&console);
        assert_eq!(menu.values[style].to_ascii_lowercase(), "classic");
        assert!(menu.popup() && menu.setup_step() == 0);
        // The pointer on SJK turns it off again.
        menu.popup_pointer(TARGET_TOKEN, UiEventKind::Activate, &mut console);
        assert_eq!(console.bool_cvar(cvar), Some(false));
        assert_eq!(menu.setup.used, [true, false]);
        assert_eq!(menu.setup_step(), 0, "a click on the one in use stays");
        menu.popup_pointer(TARGET_TOKEN, UiEventKind::Activate, &mut console);
        assert_eq!(menu.setup_step(), 0);
        // Enter on the one in use: the next step.
        menu.popup_key(KeyCode::Enter, &mut console);
        assert_eq!(console.bool_cvar(cvar), Some(false));
        assert_eq!(menu.setup_step(), 1);
    }

    /// A theme's button wears its theme at once.
    #[test]
    fn a_theme_button_wears_its_theme() {
        use crate::menu::sjk::theme::Theme;
        let (_directory, mut console, mut menu) = popup();
        let step = step_with(|act| matches!(act, Act::Theme(_)));
        menu.show_step(&console, step);
        assert_eq!(menu.setup.target, Some(0), "Solar is worn");
        let sith = STEPS[step]
            .buttons
            .iter()
            .position(|button| button.act == Act::Theme(Theme::Sith))
            .unwrap();
        menu.popup_pointer(
            TARGET_TOKEN + sith as u16,
            UiEventKind::Activate,
            &mut console,
        );
        assert_eq!(console.text_value(theme::CVAR), Some("sith"));
        assert_eq!(theme::current(), Theme::Sith);
        assert!(menu.setup.used[sith]);
        assert_eq!(menu.setup.target, Some(sith));
        menu.wear_theme(&mut console, Theme::Warm);
        assert_eq!(console.text_value(theme::CVAR), Some("warm"));
        assert!(menu.setup.used[0]);
    }

    /// The presets' buttons set their preset; the Solar Renderer's sets the level of
    /// the chip chosen, and a click on the button itself one of its levels.
    #[test]
    fn a_preset_button_sets_its_preset() {
        let (_directory, mut console, mut menu) = popup();
        let step = step_with(|act| matches!(act, Act::Preset(_)));
        menu.show_step(&console, step);
        let targets = STEPS[step].targets();
        assert_eq!(targets.len(), Level::ALL.len());
        for (target, level) in Level::ALL.into_iter().enumerate() {
            menu.popup_pointer(
                TARGET_TOKEN + target as u16,
                UiEventKind::Activate,
                &mut console,
            );
            assert_eq!(Level::current(&console), Some(level));
            let used: Vec<usize> = (0..targets.len()).filter(|at| menu.used(*at)).collect();
            assert_eq!(used, [target]);
            // A click on the button it belongs to keeps the level in use.
            assert_eq!(menu.body_target(targets[target].button), target);
        }
        // With another button's preset in use, the Solar Renderer's own click is one of
        // its levels.
        Level::Vanilla.apply(&mut console);
        menu.follow_console(&console);
        let solar = STEPS[step]
            .buttons
            .iter()
            .position(|button| button.act.options() > 1)
            .unwrap();
        let body = menu.body_target(solar);
        assert_eq!(targets[body].button, solar);
        menu.popup_pointer(
            TARGET_TOKEN + body as u16,
            UiEventKind::Activate,
            &mut console,
        );
        assert_eq!(
            Level::current(&console).map(Level::family),
            Some(graphics_quality::Family::Solar)
        );
    }

    /// Import opens the Import page, and Start fresh goes on to the next step.
    #[test]
    fn import_opens_its_page_and_start_fresh_goes_on() {
        let (_directory, mut console, mut menu) = popup();
        let step = step_with(|act| *act == Act::Import);
        menu.show_step(&console, step);
        let import = STEPS[step]
            .buttons
            .iter()
            .position(|button| button.act == Act::Import)
            .unwrap();
        let fresh = STEPS[step]
            .buttons
            .iter()
            .position(|button| button.act == Act::Next)
            .unwrap();
        assert!(matches!(
            menu.popup_pointer(
                TARGET_TOKEN + import as u16,
                UiEventKind::Activate,
                &mut console
            ),
            Some(SettingsResult::OpenImport)
        ));
        assert_eq!(menu.setup_step(), step);
        menu.popup_pointer(
            TARGET_TOKEN + fresh as u16,
            UiEventKind::Activate,
            &mut console,
        );
        assert_eq!(menu.setup_step(), step + 1);
    }

    /// The pointer on a button takes the keyboard there; on a row, gives it back.
    #[test]
    fn the_pointer_moves_the_keyboard_between_buttons_and_rows() {
        let (_directory, mut console, mut menu) = popup();
        let row = row_of(&menu, STEPS[0].rows[0]);
        menu.popup_pointer(TARGET_TOKEN + 1, UiEventKind::Hover, &mut console);
        assert_eq!(menu.setup.target, Some(1));
        assert!(
            menu.popup_pointer(row as u16, UiEventKind::Hover, &mut console)
                .is_none()
        );
        assert_eq!(menu.setup.target, None);
        // A token of no button and no row changes nothing.
        menu.popup_pointer(TARGET_TOKEN + 1, UiEventKind::Hover, &mut console);
        assert!(
            menu.popup_pointer(TARGET_TOKEN + 9, UiEventKind::Hover, &mut console)
                .is_none()
        );
        assert_eq!(menu.setup.target, Some(1));
    }

    /// The tick is the row: stepping it hides First setup at start, and Backspace's
    /// reset shows it again; it is reached from every step.
    #[test]
    fn the_tick_hides_first_setup_at_start() {
        let (_directory, mut console, mut menu) = popup();
        let hide = hide_row(&menu);
        for (index, step) in STEPS.iter().enumerate() {
            menu.show_step(&console, index);
            assert!(menu.shows(hide), "{}", step.title);
        }
        menu.show_step(&console, 0);
        menu.popup_key(KeyCode::ArrowUp, &mut console);
        assert_eq!(menu.selected, hide);
        menu.adjust(&mut console, 1);
        assert_eq!(console.bool_cvar(quick::HIDE_CVAR), Some(true));
        assert!(switch_text_on(&menu.values[hide]));
        menu.reset_to_default(&mut console, hide);
        assert_eq!(console.bool_cvar(quick::HIDE_CVAR), Some(false));
    }

    /// Done on the last step finishes First setup: it closes and no longer opens at
    /// start, by the pointer on Done or Page Down there; before the last step neither
    /// does, and Escape closes without finishing.
    #[test]
    fn done_on_the_last_step_finishes_first_setup() {
        let last = STEPS.len() - 1;
        for by_key in [true, false] {
            let (_directory, mut console, mut menu) = popup();
            // Next's place holds no Done before the last step.
            assert!(
                menu.popup_pointer(DONE_TOKEN, UiEventKind::Activate, &mut console)
                    .is_none()
            );
            assert!(matches!(
                menu.popup_key(KeyCode::Escape, &mut console),
                Some(SettingsResult::Back)
            ));
            assert_eq!(console.bool_cvar(quick::HIDE_CVAR), Some(false));
            menu.show_step(&console, last);
            let result = if by_key {
                menu.popup_key(KeyCode::PageDown, &mut console)
            } else {
                menu.popup_pointer(DONE_TOKEN, UiEventKind::Activate, &mut console)
            };
            assert!(matches!(result, Some(SettingsResult::Back)));
            assert_eq!(console.bool_cvar(quick::HIDE_CVAR), Some(true));
            // Opened again (the firstsetup command), the tick shows it.
            menu.open_first_setup_popup(&console);
            assert!(switch_text_on(&menu.values[hide_row(&menu)]));
        }
    }

    /// The step with a field opens with the keyboard in it: keys type the player's
    /// name, written as it is typed; an emptied field keeps the name before; Enter
    /// goes on, Down to the buttons and Up from them back.
    #[test]
    fn the_name_is_typed_in_its_field_and_written_as_typed() {
        let (_directory, mut console, mut menu) = popup();
        let step = STEPS.iter().position(|step| step.field.is_some()).unwrap();
        let field = STEPS[step].field.as_ref().unwrap();
        assert_eq!(field.cvar, "name");
        menu.show_step(&console, step);
        assert!(menu.popup_on_field() && menu.typing());
        assert_eq!(menu.field_text(), Some("Padawan"));
        assert_eq!(menu.setup.target, None);
        // Letters that move the keyboard elsewhere type here, held ones too.
        for _ in 0.."Padawan".len() {
            assert!(menu.popup_field_key(KeyCode::Backspace, None, &mut console));
        }
        assert_eq!(menu.field_text(), Some(""));
        assert_eq!(
            console.text_value("name"),
            Some("Padawan"),
            "kept while empty"
        );
        for (key, text) in [
            (KeyCode::Digit6, "^"),
            (KeyCode::Digit1, "1"),
            (KeyCode::KeyW, "W"),
            (KeyCode::KeyA, "a"),
            (KeyCode::KeyD, "d"),
            (KeyCode::KeyS, "s"),
        ] {
            assert!(menu.popup_field_key(key, Some(text), &mut console));
        }
        assert_eq!(console.text_value("name"), Some("^1Wads"));
        assert_eq!(menu.setup_step(), step);
        // A paste arrives as one key's text; a name stops at the game's length.
        assert!(menu.popup_field_key(KeyCode::KeyV, Some(&"x".repeat(64)), &mut console));
        assert_eq!(console.text_value("name").map(str::len), Some(31));
        // Keys that type nothing are not the field's.
        assert!(!menu.popup_field_key(KeyCode::Enter, Some("\r"), &mut console));
        assert!(!menu.popup_field_key(KeyCode::ArrowDown, None, &mut console));
        // Down: the buttons, the field's text kept; Up from them: the field again.
        menu.popup_key(KeyCode::ArrowDown, &mut console);
        assert!(!menu.popup_on_field() && !menu.typing());
        assert_eq!(menu.setup.target, Some(0));
        assert!(!menu.popup_field_key(KeyCode::KeyA, Some("a"), &mut console));
        menu.popup_key(KeyCode::ArrowUp, &mut console);
        assert!(menu.popup_on_field());
        // Up from the field: the tick; Down from the tick: the field.
        menu.popup_key(KeyCode::ArrowUp, &mut console);
        assert_eq!(menu.selected, hide_row(&menu));
        assert!(menu.popup_key(KeyCode::ArrowDown, &mut console).is_some());
        assert!(menu.popup_on_field());
        // Emptied, the name is the one the field was entered with again, whatever was
        // typed on the way; left, the field shows it.
        menu.setup_type_for_shot(&mut console, "Sol");
        assert_eq!(console.text_value("name"), Some("Sol"));
        menu.popup_key(KeyCode::ArrowDown, &mut console);
        menu.popup_key(KeyCode::ArrowUp, &mut console);
        menu.popup_field_key(KeyCode::KeyO, Some("o"), &mut console);
        assert_eq!(console.text_value("name"), Some("Solo"));
        for _ in 0..4 {
            menu.popup_field_key(KeyCode::Backspace, None, &mut console);
        }
        assert_eq!(menu.field_text(), Some(""));
        assert_eq!(console.text_value("name"), Some("Sol"));
        menu.popup_key(KeyCode::ArrowDown, &mut console);
        assert_eq!(menu.setup.field, "Sol");
        // Enter in the field goes on to the next step.
        menu.popup_key(KeyCode::ArrowUp, &mut console);
        menu.popup_key(KeyCode::Enter, &mut console);
        assert_eq!(menu.setup_step(), step + 1);
        assert!(!menu.popup_on_field());
    }

    /// A click on the field gives it the keyboard, a colour chip types its code, and
    /// a click elsewhere takes the keyboard away; a name changed elsewhere (an import)
    /// shows in the field.
    #[test]
    fn the_pointer_reaches_the_field_and_its_colours() {
        let (_directory, mut console, mut menu) = popup();
        let step = STEPS.iter().position(|step| step.field.is_some()).unwrap();
        menu.show_step(&console, step);
        menu.popup_key(KeyCode::ArrowDown, &mut console);
        assert!(!menu.popup_on_field());
        menu.popup_pointer(FIELD_TOKEN, UiEventKind::Activate, &mut console);
        assert!(menu.popup_on_field());
        // The pointer over a button does not take the keyboard from the field.
        menu.popup_pointer(TARGET_TOKEN, UiEventKind::Hover, &mut console);
        assert!(menu.popup_on_field() && menu.setup.target.is_none());
        // `^5`, the fifth colour.
        menu.popup_pointer(COLOUR_TOKEN + 4, UiEventKind::Activate, &mut console);
        assert_eq!(console.text_value("name"), Some("Padawan^5"));
        assert!(menu.popup_on_field());
        // A click on a button: the button's.
        let fresh = STEPS[step]
            .buttons
            .iter()
            .position(|button| button.act == Act::Next)
            .unwrap();
        console.set_cvar("name", "^2Imported");
        menu.follow_console(&console);
        assert_eq!(menu.setup.field, "Padawan^5", "not while it is typed in");
        menu.popup_pointer(PREVIOUS_TOKEN, UiEventKind::Activate, &mut console);
        menu.show_step(&console, step);
        assert_eq!(menu.setup.field, "^2Imported");
        menu.popup_pointer(
            TARGET_TOKEN + fresh as u16,
            UiEventKind::Activate,
            &mut console,
        );
        assert_eq!(menu.setup_step(), step + 1);
        // A step without a field has none of its tokens, and there are seven colours.
        assert!(
            menu.popup_pointer(FIELD_TOKEN, UiEventKind::Activate, &mut console)
                .is_none()
        );
        menu.show_step(&console, step);
        assert!(
            menu.popup_pointer(COLOUR_TOKEN + 7, UiEventKind::Activate, &mut console)
                .is_none()
        );
        assert!(
            !menu.popup_on_field(),
            "a click elsewhere takes the keyboard"
        );
    }

    /// A name's colour codes show as typed: no piece drawn holds a whole one.
    #[test]
    fn a_names_codes_are_drawn_as_their_characters() {
        let pieces = |text: &'static str| raw_pieces(text).collect::<Vec<_>>();
        assert_eq!(pieces("^1Kyle^7Katarn"), ["^", "1Kyle^", "7Katarn"]);
        assert_eq!(pieces("Padawan"), ["Padawan"]);
        assert_eq!(pieces("a^b^^2_"), ["a^b^^", "2_"]);
        assert_eq!(pieces("^"), ["^"]);
        assert_eq!(pieces("").len(), 0);
        for text in ["^1Kyle^7Katarn", "^^11^", "é^3ü"] {
            assert_eq!(raw_pieces(text).collect::<String>(), text);
            for piece in raw_pieces(text) {
                assert_eq!(crate::text::Plain(piece).to_string(), piece, "{piece}");
            }
        }
        assert_eq!(name_colour(1), Color::new(1.0, 0.0, 0.0, 1.0));
    }

    /// The last step's recap names what each step before it holds: the look, the
    /// theme, the player's name, the preset with its level; a click shows the step.
    #[test]
    fn the_last_step_recaps_the_steps_before_it() {
        let (_directory, mut console, mut menu) = popup();
        let last = STEPS.len() - 1;
        assert!(STEPS[last].recap);
        menu.show_step(&console, 0);
        assert!(menu.setup.recap.is_empty());
        console.set_cvar("name", "^3Fox");
        Level::SolarHigh.apply(&mut console);
        menu.show_step(&console, last);
        let recap: Vec<(&str, &str)> = menu
            .setup
            .recap
            .iter()
            .map(|(step, value)| (STEPS[*step].name, value.as_str()))
            .collect();
        assert_eq!(
            recap,
            [
                ("Look", "SJK"),
                ("Colours", "Solar"),
                ("Player", "^3Fox"),
                ("Graphics", "Solar Renderer High"),
            ]
        );
        // Settings no preset matches: that step says nothing.
        console.set_cvar("r_sunShadowTaps", "12");
        menu.follow_console(&console);
        assert_eq!(menu.setup.recap.len(), 3);
        let player = STEPS.iter().position(|step| step.field.is_some()).unwrap();
        menu.popup_pointer(
            RECAP_TOKEN + player as u16,
            UiEventKind::Activate,
            &mut console,
        );
        assert_eq!(menu.setup_step(), player);
        // A step that has no tile has no token.
        menu.show_step(&console, last);
        assert!(
            menu.popup_pointer(
                RECAP_TOKEN + last as u16,
                UiEventKind::Activate,
                &mut console
            )
            .is_none()
        );
    }

    /// Choose a character asks for the Character page.
    #[test]
    fn choose_a_character_asks_for_the_character_page() {
        let (_directory, mut console, mut menu) = popup();
        let step = step_with(|act| *act == Act::Character);
        menu.show_step(&console, step);
        let character = STEPS[step]
            .buttons
            .iter()
            .position(|button| button.act == Act::Character)
            .unwrap();
        assert!(matches!(
            menu.popup_pointer(
                TARGET_TOKEN + character as u16,
                UiEventKind::Activate,
                &mut console
            ),
            Some(SettingsResult::OpenCharacter)
        ));
        assert_eq!(menu.setup_step(), step);
    }

    /// First setup's other views pin nothing and keep every row in one list.
    #[test]
    fn the_other_views_of_first_setup_keep_the_whole_list() {
        let (_directory, console, mut menu) = popup();
        menu.show_step(&console, 3);
        menu.open_classic_group(
            &console,
            Group::Quick,
            crate::menu::classic::panel::Frame::Main,
        );
        assert!(!menu.popup() && menu.on_first_setup());
        let rows = menu.classic.as_ref().unwrap().rows().count();
        assert_eq!(rows, quick::rows().len());
    }
}
