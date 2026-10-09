//! The SJK UI's Profile screen (`docs/sjk-ui.md`, Profile screen): existing screens
//! shown as one, under one row of tabs. Character, Saber and Force are the player
//! screen's pages (`player_menu`); SJK Profile, Achievements and Medals the console's
//! Profile page (`profile_panel`: bio, picture and record; the board; the medals) and
//! Collection the console's Unlockables page (`unlockables_panel`). Each screen keeps
//! its state, keys and pointer; the row is the player screen's own tabs grown to seven
//! ([`tabs`]), drawn at the same place by whichever screen shows, with the tab on show
//! lit, and the switch between them is here.
//!
//! The tabs change with Ctrl+Tab (Ctrl+Shift+Tab back) from any of them, even while a
//! field is typed in; on the player screen's pages Tab, `]` and `[` walk all seven as
//! they walked its three (Tab alone moves within the console's pages, as everywhere in
//! the UI). A click on a tab shows it; it never reaches the screen under the row.
//! Escape leaves the screen as each tab's own way back does: to the game menu on its
//! Profile entry, or to the main page.

use crate::GpuState;
use crate::menu::sjk::{Frame, color, text, top_bar};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::player_menu::ReturnTarget;
use sjk_ui::{DrawCommand, FontWeight, InputEvent, PointerButton, TextAlign, Vec2};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// A tab of the Profile screen, in the row's order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Tab {
    /// The player screen's pages: name and model, saber, Force.
    #[default]
    Character,
    Saber,
    Force,
    /// The SJK profile: picture, bio and record.
    Profile,
    /// The achievements board.
    Achievements,
    /// The medals the SJK team gave, and those it gives.
    Medals,
    /// Every unlockable, owned and locked ([`COLLECTION`]).
    Collection,
}

/// The name of the tab listing every unlockable, owned or not: one place to rename it.
pub(crate) const COLLECTION: &str = "Collection";

/// The tabs' names, in [`Tab`] order.
const LABELS: [&str; 7] = [
    "Character",
    "Saber",
    "Force",
    "SJK Profile",
    "Achievements",
    "Medals",
    COLLECTION,
];

impl Tab {
    pub(crate) const ALL: [Self; 7] = [
        Self::Character,
        Self::Saber,
        Self::Force,
        Self::Profile,
        Self::Achievements,
        Self::Medals,
        Self::Collection,
    ];

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    /// The tab's name on the row.
    pub(crate) const fn label(self) -> &'static str {
        LABELS[self.index()]
    }

    /// The tab after this one (`forward`) or before it, wrapping.
    pub(crate) fn next(self, forward: bool) -> Self {
        let count = Self::ALL.len();
        let index = if forward {
            (self.index() + 1) % count
        } else {
            (self.index() + count - 1) % count
        };
        Self::ALL[index]
    }

    /// The player screen's page this tab is (0 Character, 1 Saber, 2 Force), or
    /// `None` for a tab the console shows.
    pub(crate) const fn player_page(self) -> Option<usize> {
        match self {
            Self::Character | Self::Saber | Self::Force => Some(self.index()),
            _ => None,
        }
    }

    /// The tab of the player screen's page `index`.
    pub(crate) fn of_player_page(index: usize) -> Self {
        Self::ALL[index.min(2)]
    }
}

/// The row's tokens on each screen's canvas, tab `i` answering to `TOKEN + i`; clear
/// of the player screen's (to 1700), the Profile page's (to 1030) and the Unlockables
/// page's (to 1260).
pub(crate) const TOKEN: u16 = 2_000;
/// Where the row starts and its labels' top, in frame pixels: under the title, as the
/// player screen's own tabs stood.
const ROW_X: f32 = 96.0;
const ROW_Y: f32 = 146.0;
/// The room between two tabs' names.
const GAP: f32 = 48.0;
/// How far down the console's pages move what they show under their title, to leave
/// the row its room.
pub(crate) const SHIFT: f32 = 44.0;

/// A tab's name's size.
const LABEL_SIZE: f32 = 26.0;

/// How wide a tab's name is drawn, the player's text size and letter spacing
/// (`ui_textScale`, `ui_letterSpacing`) included: the row makes room for the styled
/// names, which would otherwise be cut at their tab's end.
fn label_width(label: &str) -> f32 {
    styled_width(label, crate::text::style::current())
}

/// [`label_width`] in `style`.
fn styled_width(label: &str, style: crate::text::TextStyle) -> f32 {
    let size = LABEL_SIZE * style.scale;
    let tracking = style.tracking * size * label.chars().count() as f32;
    (crate::text::display_width(label, size) + tracking).max(0.0)
}

/// Where tab `index` of `labels` starts and how wide its name is (frame pixels).
fn places(labels: &[&str], index: usize) -> (f32, f32) {
    let x = labels
        .iter()
        .take(index)
        .map(|label| label_width(label) + GAP)
        .sum::<f32>();
    (
        ROW_X + x,
        labels.get(index).map_or(0.0, |label| label_width(label)),
    )
}

/// The area that answers the pointer for tab `index` of `labels`, frame pixels.
fn hit_area(labels: &[&str], index: usize) -> [f32; 4] {
    let (x, width) = places(labels, index);
    [x - 8.0, ROW_Y - 4.0, width + 16.0, 46.0]
}

/// A row of tabs under a screen's title: `labels` from the left edge of the form, the
/// one at `current` gold and underlined, one hovered brighter; tab `i` answers to
/// `token_base + i`. The player screen draws its own three this way, and the Profile
/// screen's seven ([`row`]).
pub(crate) fn tabs(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    labels: &[&str],
    current: usize,
    token_base: u16,
) {
    let s = frame.s;
    for (index, label) in labels.iter().enumerate() {
        let (x, width) = places(labels, index);
        let token = token_base + index as u16;
        let lit = index == current;
        let hovered = canvas.token_hovered(token);
        text(
            canvas,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(x, ROW_Y, width + 20.0, 34.0),
            LABEL_SIZE * s,
            match (lit, hovered) {
                (true, _) => color::GOLD_BRIGHT,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        if lit {
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x, ROW_Y + 38.0, width, 3.0),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        let [x, y, width, height] = hit_area(labels, index);
        canvas.hit_region(token, frame.rect(x, y, width, height));
    }
}

/// The Profile screen's row of tabs, `current` lit.
pub(crate) fn row(canvas: &mut MenuCanvas, frame: &Frame, current: Tab) {
    tabs(canvas, frame, &LABELS, current.index(), TOKEN);
}

/// What a console page shown as one of the tabs puts at its top: the way back and the
/// player's name, as the player screen does.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Header {
    /// Where the screen returns.
    pub(crate) back: ReturnTarget,
    /// The `name` cvar.
    pub(crate) name: String,
}

impl Header {
    /// Whether it says `back` and `name`.
    pub(crate) fn is(&self, back: ReturnTarget, name: &str) -> bool {
        self.back == back && self.name == name
    }

    /// The way back's words.
    fn back(&self) -> &'static str {
        match self.back {
            ReturnTarget::MainMenu => "Main menu",
            ReturnTarget::InGame => "Game menu",
        }
    }
}

/// The player's name as the screen's title: "Padawan" while it is empty, as retail's
/// default name.
pub(crate) fn title(name: &str) -> &str {
    if name.trim().is_empty() {
        "Padawan"
    } else {
        name
    }
}

/// The top of a console page shown as tab `current`: the way back (`back_token`), the
/// player's name and the row of tabs.
pub(crate) fn header(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    header: &Header,
    back_token: u16,
    current: Tab,
) {
    top_bar(
        canvas,
        frame,
        header.back(),
        back_token,
        title(&header.name),
        None,
    );
    row(canvas, frame, current);
}

/// The tab of the row under `point` (window pixels) in a window of `viewport`.
pub(crate) fn tab_at(viewport: [f32; 2], point: Vec2) -> Option<Tab> {
    let frame = Frame::new(viewport);
    Tab::ALL.into_iter().find(|tab| {
        let [x, y, width, height] = hit_area(&LABELS, tab.index());
        frame.rect(x, y, width, height).contains(point)
    })
}

/// What the switch remembers: the tab shown last (the game menu's Profile opens on
/// it) and a tab pressed and not yet released.
#[derive(Debug, Default)]
pub(crate) struct Hub {
    last: Tab,
    pressed: Option<Tab>,
}

impl GpuState {
    /// The Profile screen's tab on show, when it shows.
    pub(crate) fn profile_hub_tab(&self) -> Option<Tab> {
        self.console
            .as_ref()
            .and_then(crate::console::ViewerConsole::profile_hub_tab)
            .or_else(|| {
                self.client_menu
                    .as_ref()
                    .and_then(crate::menu::ClientMenu::player_hub_page)
                    .map(Tab::of_player_page)
            })
    }

    /// Note the tab on show, for the game menu's Profile to open on it again: the
    /// console's pages change some tabs on their own (See the board). Each frame.
    pub(crate) fn remember_profile_hub_tab(&mut self) {
        if let Some(tab) = self.profile_hub_tab() {
            self.profile_hub.last = tab;
        }
    }

    /// Open the Profile screen from the game menu on `tab`, else on the tab shown
    /// last (Character the first time); it returns to the menu's Profile entry.
    pub(crate) fn open_profile_hub_from_game(&mut self, tab: Option<Tab>) {
        let tab = tab.unwrap_or(self.profile_hub.last);
        self.show_profile_hub(tab, ReturnTarget::InGame);
    }

    /// The `profile`, `achievements` and `unlockables` commands in the SJK UI: show the
    /// Profile screen on `tab`, or close it when it shows that tab. It returns to the
    /// main page from the menus, else to the game menu.
    pub(crate) fn toggle_profile_hub(&mut self, tab: Tab) {
        if self.profile_hub_tab() == Some(tab) {
            if let Some(console) = &mut self.console {
                console.close_profile_hub_page();
            }
            self.sync_cursor_policy();
            return;
        }
        self.open_profile_hub(tab);
    }

    /// Show the Profile screen on `tab` unless it shows it already (a picture dropped
    /// on the window, `sjkavatar`): it returns to the main page from the menus, else to
    /// the game menu.
    pub(crate) fn open_profile_hub(&mut self, tab: Tab) {
        if self.profile_hub_tab() == Some(tab) {
            return;
        }
        let in_menus = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_visible);
        let target = if in_menus {
            ReturnTarget::MainMenu
        } else {
            ReturnTarget::InGame
        };
        self.show_profile_hub(tab, target);
    }

    /// Show `tab`, closing the screen on show if it is another; `target` is where the
    /// screen returns.
    fn show_profile_hub(&mut self, tab: Tab, target: ReturnTarget) {
        use crate::ingame_menu::{Page, sjk_focus::Focus, sjk_view::Entry};
        self.profile_hub.last = tab;
        let in_game = target == ReturnTarget::InGame;
        if in_game {
            // Escape or the way back comes to the game menu's Profile entry.
            self.in_game_menu.remember_return(Entry::Profile.index());
        }
        if let Some(page) = tab.player_page() {
            if let Some(console) = &mut self.console {
                console.close_profile_hub_page();
            }
            if let (Some(menu), Some(console)) = (&mut self.client_menu, &self.console) {
                // Between the player screen's own pages it only turns the page.
                if menu.player_hub_page().is_none() {
                    menu.open_player_hub(console, target);
                }
                menu.show_player_hub_page(page);
                if in_game {
                    self.game_menu = false;
                }
            }
        } else {
            if let Some(menu) = self
                .client_menu
                .as_mut()
                .filter(|menu| menu.player_hub_shown())
            {
                menu.leave_player_hub();
            }
            if in_game {
                // The game menu stays under the console's page and shows again,
                // on Profile, when the page closes.
                self.game_menu = true;
                self.game_menu_page = Page::Main;
                self.game_menu_row = self.in_game_menu.return_row();
                self.in_game_menu.focus = Focus::List;
            }
            if let Some(console) = &mut self.console {
                console.open_profile_hub_page(tab, target);
            }
        }
        self.sync_cursor_policy();
    }

    /// Switch the Profile screen to `tab`, keeping where it returns.
    fn switch_profile_hub(&mut self, tab: Tab) {
        let Some(current) = self.profile_hub_tab() else {
            return;
        };
        if tab == current {
            return;
        }
        let target = if current.player_page().is_some() {
            self.client_menu
                .as_ref()
                .map_or(ReturnTarget::MainMenu, |menu| menu.player_return_target())
        } else {
            self.console
                .as_ref()
                .map_or(ReturnTarget::MainMenu, |console| {
                    console.profile_hub_return()
                })
        };
        self.show_profile_hub(tab, target);
    }

    /// The keys that change the Profile screen's tab while it shows: Ctrl+Tab
    /// (Ctrl+Shift+Tab back) from any tab, and on the player screen's pages Tab
    /// (Shift+Tab back), `]` and `[`, unless a field there is typed in. Returns whether
    /// the key was taken.
    pub(crate) fn profile_hub_key(&mut self, event: &KeyEvent) -> bool {
        let PhysicalKey::Code(key) = event.physical_key else {
            return false;
        };
        if !matches!(
            key,
            KeyCode::Tab | KeyCode::BracketLeft | KeyCode::BracketRight
        ) {
            return false;
        }
        let Some(console) = self.console.as_ref() else {
            return false;
        };
        let Some(current) = self.profile_hub_tab() else {
            return false;
        };
        let (control, shift) = (console.control_held(), console.shift_held());
        let typing = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::player_hub_typing);
        let forward = match key {
            KeyCode::Tab if control => !shift,
            _ if control || current.player_page().is_none() || typing => return false,
            KeyCode::Tab => !shift,
            KeyCode::BracketRight => true,
            _ => false,
        };
        if event.state == ElementState::Pressed && !event.repeat {
            self.switch_profile_hub(current.next(forward));
        }
        true
    }

    /// A press and release on one of the Profile screen's tabs switch to it; the
    /// screen under the row sees neither. Returns whether the event was taken.
    pub(crate) fn profile_hub_pointer(&mut self, event: InputEvent) -> bool {
        if self.profile_hub_tab().is_none() {
            self.profile_hub.pressed = None;
            return false;
        }
        let viewport = [
            self.configuration.width as f32,
            self.configuration.height as f32,
        ];
        match event {
            InputEvent::PointerPress {
                position,
                button: PointerButton::Primary,
            } => match tab_at(viewport, position) {
                Some(tab) => {
                    self.profile_hub.pressed = Some(tab);
                    true
                }
                None => false,
            },
            InputEvent::PointerRelease { position, .. } => {
                let Some(pressed) = self.profile_hub.pressed.take() else {
                    return false;
                };
                if tab_at(viewport, position) == Some(pressed) {
                    self.switch_profile_hub(pressed);
                }
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// Ctrl+Tab without a keyboard, for the world shots.
    pub(crate) fn profile_hub_next_for_shot(&mut self) {
        if let Some(current) = self.profile_hub_tab() {
            self.switch_profile_hub(current.next(true));
        }
    }

    /// A click on `tab`, for the world shots.
    pub(crate) fn profile_hub_show_for_shot(&mut self, tab: Tab) {
        self.switch_profile_hub(tab);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tabs_wrap_both_ways_and_the_player_screens_pages_come_first() {
        assert_eq!(Tab::Character.next(true), Tab::Saber);
        assert_eq!(Tab::Force.next(true), Tab::Profile);
        assert_eq!(Tab::Collection.next(true), Tab::Character);
        assert_eq!(Tab::Character.next(false), Tab::Collection);
        assert_eq!(Tab::Profile.next(false), Tab::Force);
        for (index, tab) in Tab::ALL.into_iter().enumerate() {
            assert_eq!(tab.index(), index);
            assert_eq!(tab.player_page(), (index < 3).then_some(index));
        }
        assert_eq!(Tab::of_player_page(2), Tab::Force);
        assert_eq!(Tab::Collection.label(), COLLECTION);
        assert_eq!(Tab::Profile.label(), "SJK Profile");
    }

    /// A larger text style widens every tab's name by as much as it is drawn wider, so
    /// none is cut ("Achievements" was at `ui_textScale 1.2`), and even the largest
    /// style keeps the seven within the frame.
    #[test]
    fn the_tabs_make_room_for_the_players_text_style() {
        let neutral = crate::text::TextStyle::NEUTRAL;
        let larger = crate::text::TextStyle {
            scale: 1.2,
            tracking: 0.0,
        };
        let widest = crate::text::TextStyle {
            scale: crate::text::style::SCALE_RANGE.1,
            tracking: crate::text::style::TRACKING_RANGE.1,
        };
        let plain = styled_width("Achievements", neutral);
        assert!(plain > 100.0);
        assert!((styled_width("Achievements", larger) - plain * 1.2).abs() < 0.01);
        let row: f32 = LABELS
            .iter()
            .map(|label| styled_width(label, widest) + GAP)
            .sum();
        assert!(ROW_X + row < 1_824.0, "the row reaches {}", ROW_X + row);
    }

    /// The row the screens draw and the areas the switch hit-tests are the same, in
    /// every window, the tabs side by side within the frame, under the title.
    #[test]
    fn the_row_answers_where_it_is_drawn() {
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            let mut canvas = MenuCanvas::new();
            canvas.begin_transparent(viewport);
            let frame = Frame::new(viewport);
            header(
                &mut canvas,
                &frame,
                &Header::default(),
                crate::menu_widgets::BACK_TOKEN,
                Tab::Medals,
            );
            assert!(!canvas.overflowed());
            let mut previous: Option<sjk_ui::Rect> = None;
            for tab in Tab::ALL {
                let area = canvas
                    .rect_for(TOKEN + tab.index() as u16)
                    .expect("a tab's area");
                let centre = Vec2::new(area.x + area.width * 0.5, area.y + area.height * 0.5);
                assert_eq!(tab_at(viewport, centre), Some(tab), "{viewport:?}");
                assert!(area.right() <= viewport[0] && area.y >= 0.0);
                if let Some(previous) = previous {
                    assert!(previous.right() <= area.x, "{tab:?} overlaps");
                }
                previous = Some(area);
            }
            let back = canvas
                .rect_for(crate::menu_widgets::BACK_TOKEN)
                .expect("the way back");
            let first = canvas.rect_for(TOKEN).expect("the first tab");
            assert!(back.bottom() <= first.y, "the row is under the title");
            assert_eq!(tab_at(viewport, Vec2::new(10.0, 10.0)), None);
        }
    }

    #[test]
    fn an_empty_name_is_titled_as_retail_names_a_new_player() {
        assert_eq!(title("  "), "Padawan");
        assert_eq!(title("^1Sol"), "^1Sol");
    }
}
