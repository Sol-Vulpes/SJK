//! First setup as the SJK UI's pop-up (`docs/sjk-ui.md`, First setup): a card
//! over the darkened map with its name and what it is for, the themes as four
//! tiles, each in its own colours (a click wears it at once), First setup's rows
//! scrolling inside it as Settings draws them ([`super::sjk_view`]), the
//! focused row's help under them, and at its foot, always in view, the "Don't
//! show at start" tick beside All settings and Done.
//!
//! The tick is First setup's own last row, pinned out of the scrolling lines
//! ([`super::ClassicRows`]): the keyboard reaches it after the last row, a
//! click flips it, and Backspace returns it to its default, as on any row.
//! Positions are pixels of the SJK UI's 16:9 frame ([`Frame`]).

use super::classic_view::switch_text_on;
use super::help;
use super::sjk_view::{CONTROL_LEFT, KEYS_Y, LINE, ROWS_TOP, ROWS_WIDTH, ROWS_X};
use super::*;
use crate::menu::sjk::{Frame, TextTarget, color, kit, text, wrap};
use crate::menu_widgets::TextFamily;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The card.
const CARD: [f32; 4] = [480.0, 56.0, 960.0, 948.0];
/// The card's inner margin, and where its text starts.
const MARGIN: f32 = 48.0;
const TEXT_X: f32 = CARD[0] + MARGIN;
const TEXT_WIDTH: f32 = CARD[2] - MARGIN * 2.0;
/// The themes' tiles, under the header: their top, height and the gap between two.
const TILES_TOP: f32 = CARD[1] + 150.0;
const TILE_HEIGHT: f32 = 104.0;
const TILE_GAP: f32 = 14.0;
/// Where Settings' rows column moves to sit in the card, under the tiles.
const SHIFT: [f32; 2] = [
    CARD[0] + 80.0 - ROWS_X,
    TILES_TOP + TILE_HEIGHT + 14.0 - ROWS_TOP,
];
/// Lines the card shows before its rows scroll.
const VISIBLE: usize = 9;
/// The focused row's help, under the rows.
const HELP_TOP: f32 = ROWS_TOP + SHIFT[1] + VISIBLE as f32 * LINE + 12.0;
/// The foot: its rule, and the middle of its tick and buttons.
const FOOT_RULE: f32 = CARD[1] + CARD[3] - 94.0;
const FOOT_MIDDLE: f32 = CARD[1] + CARD[3] - 47.0;
const BUTTON_HEIGHT: f32 = 46.0;
/// An open list reaches no lower than the help's first line.
const LIST_BOTTOM: f32 = FOOT_RULE - 10.0 - SHIFT[1];
/// The keys' line, under the card.
const KEYS_SHIFT: f32 = CARD[1] + CARD[3] + 28.0 - KEYS_Y;

/// The All settings button's target; Done is the way back's (900).
pub(super) const ALL_SETTINGS_TOKEN: u16 = 914;
const DONE_TOKEN: u16 = 900;
/// Theme tile `i` answers to `THEME_TOKEN + i`, clear of the rows (0-499), the
/// panel's own tokens (500 to 915) and a list's choices (1000 on).
pub(super) const THEME_TOKEN: u16 = 930;

/// The theme the tile answering to `token` wears, if it is one.
pub(super) fn theme_of(token: u16) -> Option<crate::menu::sjk::theme::Theme> {
    crate::menu::sjk::theme::Theme::ALL
        .get(usize::from(token.checked_sub(THEME_TOKEN)?))
        .copied()
}

impl SettingsMenu {
    /// Draw First setup's pop-up at `reveal` opacity.
    pub(crate) fn append_sjk_popup(
        &mut self,
        target: TextTarget<'_>,
        viewport: [f32; 2],
        reveal: f32,
    ) {
        let frame = Frame::new(viewport);
        let rows = frame.shifted(SHIFT[0], SHIFT[1]);
        self.ui.begin_transparent(viewport);
        self.ui.push_opacity(reveal);
        kit::scrim(&mut self.ui, viewport);
        kit::card(&mut self.ui, &frame, CARD);
        self.popup_header(&frame);
        self.popup_themes(&frame);
        let measure = target.body_measure();
        let slots = self.sjk_rows(&rows, &measure, VISIBLE, LIST_BOTTOM);
        if self.dropdown.is_none() {
            self.popup_help(&frame);
        }
        self.popup_foot(&frame);
        self.sjk_keys(&frame.shifted(0.0, KEYS_SHIFT));
        self.sjk_dropdown(&rows, &slots, VISIBLE, LIST_BOTTOM);
        self.ui.pop_opacity();
        self.ui.finish(self.selected as u16);
        target.append(&self.ui, viewport);
        self.sjk_controls = Some(rows.point(CONTROL_LEFT, 0.0)[0]);
    }

    /// The card's name, what it is for and how to bring another client's
    /// settings over.
    fn popup_header(&mut self, frame: &Frame) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("First setup"),
            frame.rect(TEXT_X, CARD[1] + 30.0, TEXT_WIDTH, 52.0),
            40.0 * s,
            color::text(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Pick a look, then the few settings worth choosing first. Settings has them all."
            ),
            frame.rect(TEXT_X, CARD[1] + 88.0, TEXT_WIDTH, 26.0),
            18.0 * s,
            color::muted(),
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Coming from another client? Import from a client brings your setup, and the FPS your movement felt like."
            ),
            frame.rect(TEXT_X, CARD[1] + 116.0, TEXT_WIDTH, 24.0),
            15.0 * s,
            color::quiet(),
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The themes, a tile each in the theme's own colours whatever the one worn: its
    /// ground and glass, a stroke of its line-work, a dot of its gold, its name in its
    /// text colour. The one in use has a gold edge and "In use"; a click wears one at
    /// once. The Theme row under them is the same choice for the keyboard.
    fn popup_themes(&mut self, frame: &Frame) {
        use crate::menu::sjk::theme::{self, Theme};
        let s = frame.s;
        let width = (TEXT_WIDTH - TILE_GAP * 3.0) / 4.0;
        let worn = theme::current();
        for (index, choice) in Theme::ALL.into_iter().enumerate() {
            let palette = theme::preview(choice);
            let x = TEXT_X + index as f32 * (width + TILE_GAP);
            let tile = frame.rect(x, TILES_TOP, width, TILE_HEIGHT);
            let radius = 14.0 * s;
            let list = self.ui.draw_list_mut();
            let _ = list.push(DrawCommand::RoundedRect {
                rect: tile,
                radius,
                color: palette.space,
            });
            // A band of the theme's glass across the foot, as its panels are.
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 8.0, TILES_TOP + 58.0, width - 16.0, 38.0),
                radius: 9.0 * s,
                color: palette.glass_lit,
            });
            // Its line-work and its gold.
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 20.0, TILES_TOP + 74.0, width - 86.0, 6.0),
                radius: 3.0 * s,
                color: palette.holo,
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(x + width - 52.0, TILES_TOP + 66.0, 34.0, 22.0),
                radius: 11.0 * s,
                color: palette.gold,
            });
            let _ = list.push(DrawCommand::Border {
                rect: tile,
                radius,
                width: if choice == worn { 2.5 } else { 1.5 } * s,
                color: if choice == worn {
                    palette.gold
                } else {
                    color::alpha(palette.holo, 0.45)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", choice.label()),
                frame.rect(x + 18.0, TILES_TOP + 12.0, width - 36.0, 34.0),
                26.0 * s,
                palette.text,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if choice == worn {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("In use"),
                    frame.rect(x + 18.0, TILES_TOP + 18.0, width - 36.0, 24.0),
                    14.0 * s,
                    palette.gold,
                    FontWeight::Regular,
                    TextAlign::End,
                );
            }
            self.ui.hit_region(THEME_TOKEN + index as u16, tile);
        }
    }

    /// Wear theme `theme`, as a click on its tile does.
    pub(super) fn wear_theme(
        &mut self,
        console: &mut ViewerConsole,
        theme: crate::menu::sjk::theme::Theme,
    ) {
        console.set_cvar(crate::menu::sjk::theme::CVAR, theme.name());
        crate::menu::sjk::theme::sync(console);
        self.refresh(console);
    }

    /// What the focused row does, in up to two lines under the rows.
    fn popup_help(&mut self, frame: &Frame) {
        let s = frame.s;
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
                    ROWS_X + SHIFT[0],
                    HELP_TOP + index as f32 * 24.0,
                    ROWS_WIDTH,
                    24.0,
                ),
                16.0 * s,
                color::alpha(color::text(), 0.72),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The foot under its rule: the pinned "Don't show at start" tick on the
    /// left, focused with the row's band, then All settings and Done on the
    /// right.
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
        kit::button(
            &mut self.ui,
            frame,
            [right - 160.0, top, 160.0, BUTTON_HEIGHT],
            "Done",
            true,
            true,
            false,
            DONE_TOKEN,
        );
        kit::button(
            &mut self.ui,
            frame,
            [right - 160.0 - 14.0 - 170.0, top, 170.0, BUTTON_HEIGHT],
            "All settings",
            false,
            true,
            false,
            ALL_SETTINGS_TOKEN,
        );
    }
}

// The tiles under the header's last line, the rows under the tiles.
const _: () = assert!(TILES_TOP >= CARD[1] + 116.0 + 24.0);
const _: () = assert!(ROWS_TOP + SHIFT[1] >= TILES_TOP + TILE_HEIGHT);
// The rows, their scrollbar, the help and the foot inside the card, in order.
const _: () = assert!(ROWS_X + SHIFT[0] >= CARD[0] + MARGIN);
const _: () = assert!(ROWS_X + SHIFT[0] + ROWS_WIDTH + 18.0 <= CARD[0] + CARD[2] - 24.0);
const _: () = assert!(HELP_TOP + 2.0 * 24.0 <= FOOT_RULE - 8.0);
const _: () = assert!(FOOT_MIDDLE + BUTTON_HEIGHT * 0.5 < CARD[1] + CARD[3]);
// The card and the keys under it inside the frame.
const _: () = assert!(CARD[0] + CARD[2] <= 1824.0 && CARD[1] + CARD[3] + 48.0 <= 1080.0);

#[cfg(test)]
mod tests {
    use super::*;

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

    /// "Don't show at start" leaves the scrolling lines for the foot, and the
    /// keyboard still reaches it: up from the first row, and back down.
    #[test]
    fn the_hide_row_is_pinned_at_the_foot_and_the_keyboard_reaches_it() {
        let (_directory, _console, mut menu) = popup();
        assert!(menu.popup() && menu.on_first_setup());
        let hide = hide_row(&menu);
        let classic = menu.classic.as_ref().unwrap();
        assert!(!classic.lines.contains(&Line::Row(hide)));
        assert_eq!(classic.rows().last(), Some(hide));
        assert!(menu.shows(hide));
        assert_eq!(menu.selected, 0);
        menu.step_selection(-1);
        assert_eq!(menu.selected, hide);
        menu.step_selection(1);
        assert_eq!(menu.selected, 0);
    }

    /// The tick is the row: stepping it hides First setup at start, and
    /// Backspace's reset shows it again.
    #[test]
    fn the_tick_hides_first_setup_at_start() {
        let (_directory, mut console, mut menu) = popup();
        let hide = hide_row(&menu);
        menu.step_selection(-1);
        menu.adjust(&mut console, 1);
        assert_eq!(console.bool_cvar(quick::HIDE_CVAR), Some(true));
        assert!(switch_text_on(&menu.values[hide]));
        menu.reset_to_default(&mut console, hide);
        assert_eq!(console.bool_cvar(quick::HIDE_CVAR), Some(false));
    }

    /// A tile wears its theme at once, and the Theme row under them says the same.
    #[test]
    fn a_theme_tile_wears_its_theme() {
        use crate::menu::sjk::theme::{self, Theme};
        let (_directory, mut console, mut menu) = popup();
        assert_eq!(theme_of(THEME_TOKEN), Some(Theme::Warm));
        assert_eq!(theme_of(THEME_TOKEN + 1), Some(Theme::Sith));
        assert_eq!(theme_of(THEME_TOKEN + 4), None);
        assert_eq!(theme_of(THEME_TOKEN - 1), None);
        menu.wear_theme(&mut console, Theme::Sith);
        assert_eq!(console.text_value(theme::CVAR), Some("sith"));
        let row = menu
            .rows()
            .iter()
            .position(|setting| setting.cvar == theme::CVAR)
            .unwrap();
        assert_eq!(row, 0, "the look comes first");
        assert_eq!(menu.values[row], "sith");
        menu.wear_theme(&mut console, Theme::Warm);
        assert_eq!(console.text_value(theme::CVAR), Some("warm"));
    }

    #[test]
    fn tab_leaves_for_all_settings_and_the_search_keys_do_nothing() {
        assert!(matches!(
            popup_key(KeyCode::Tab),
            Some(SettingsResult::AllSettings)
        ));
        for key in [KeyCode::Slash, KeyCode::BracketLeft, KeyCode::BracketRight] {
            assert!(matches!(popup_key(key), Some(SettingsResult::None)));
        }
        assert!(popup_key(KeyCode::ArrowUp).is_none());
        // First setup's other views pin nothing.
        let (_directory, console, mut menu) = popup();
        menu.open_classic_group(
            &console,
            Group::Quick,
            crate::menu::classic::panel::Frame::Main,
        );
        assert!(!menu.popup() && menu.on_first_setup());
    }
}
