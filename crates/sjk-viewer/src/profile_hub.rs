//! The SJK UI's Profile screen (`docs/sjk-ui.md`, Profile screen): three existing
//! screens shown as one, with tabs at their top right. Character is the player
//! screen (`player_menu`, with its own Character, Saber and Force pages), Profile the
//! console's Profile page (`profile_panel`, bio, medals, record) and Identity the
//! console's Identity page (`identity_panel`, the key settings). Each screen keeps its
//! state, keys and pointer; what is new is the strip of tabs each draws when it is
//! shown as a tab ([`strip`]), and the switch between them here.
//!
//! The tabs change with Ctrl+Tab (Ctrl+Shift+Tab back), from any of them and even
//! while a field is typed in (Tab alone moves within a screen, as everywhere in the
//! UI), and with a click on the strip. A click there never reaches the screen under
//! it. Escape leaves the screen as each tab's own way back does: to the game menu on
//! its Profile entry, or to the main page.

use crate::GpuState;
use crate::menu::sjk::{Frame, key_hint, key_hint_width, kit};
use crate::menu_widgets::MenuCanvas;
use crate::player_menu::ReturnTarget;
use sjk_ui::{InputEvent, PointerButton, Vec2};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// A tab of the Profile screen.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Tab {
    /// The player screen: name, model, saber and Force.
    #[default]
    Character,
    /// The SJK profile: bio, medals, record.
    Profile,
    /// The SJK identity: the key, sharing it with the hub.
    Identity,
}

impl Tab {
    pub(crate) const ALL: [Self; 3] = [Self::Character, Self::Profile, Self::Identity];

    pub(crate) const fn index(self) -> usize {
        self as usize
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
}

/// The tabs' names, in [`Tab`] order.
const LABELS: [&str; 3] = ["Character", "Profile", "Identity"];
/// The strip's tokens on each screen's canvas, tab `i` answering to `TOKEN + i`;
/// clear of the player screen's (to 1700), the Profile page's (to 1020) and the
/// Identity page's.
pub(crate) const TOKEN: u16 = 2_000;
/// Where the strip ends and its middle line, in frame pixels: the top bar's right
/// end, as the Profile page's own segments stood.
const RIGHT: f32 = 1_824.0;
const MIDDLE: f32 = 87.0;

/// The strip of tabs at the top right of a screen shown as one of them, `current`
/// lit gold, with the key that changes it under it.
pub(crate) fn strip(canvas: &mut MenuCanvas, frame: &Frame, current: Tab) {
    kit::segments(
        canvas,
        frame,
        RIGHT,
        MIDDLE,
        &LABELS,
        current.index(),
        false,
        TOKEN,
    );
    let s = frame.s;
    let keys = ["Ctrl", "Tab"];
    let action = "next tab";
    let width = key_hint_width(&keys, action, s);
    let [right, y] = frame.point(RIGHT, MIDDLE + 30.0);
    key_hint(canvas, &keys, action, right - width, y, s);
}

/// The tab of the strip under `point` (window pixels) in a window of `viewport`.
pub(crate) fn tab_at(viewport: [f32; 2], point: Vec2) -> Option<Tab> {
    let frame = Frame::new(viewport);
    Tab::ALL.into_iter().find(|tab| {
        let [x, y, width, height] = kit::segment_rect(RIGHT, MIDDLE, &LABELS, tab.index());
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
                    .is_some_and(crate::menu::ClientMenu::player_hub_shown)
                    .then_some(Tab::Character)
            })
    }

    /// Open the Profile screen from the game menu on `tab`, else on the tab shown
    /// last (Character the first time); it returns to the menu's Profile entry.
    pub(crate) fn open_profile_hub_from_game(&mut self, tab: Option<Tab>) {
        let tab = tab.unwrap_or(self.profile_hub.last);
        self.show_profile_hub(tab, ReturnTarget::InGame);
    }

    /// Show `tab`, closing the one on show; `target` is where the screen returns.
    fn show_profile_hub(&mut self, tab: Tab, target: ReturnTarget) {
        use crate::ingame_menu::{Page, sjk_focus::Focus, sjk_view::Entry};
        self.profile_hub.last = tab;
        let in_game = target == ReturnTarget::InGame;
        if in_game {
            // Escape or the way back comes to the game menu's Profile entry.
            self.in_game_menu.remember_return(Entry::Profile.index());
        }
        match tab {
            Tab::Character => {
                if let Some(console) = &mut self.console {
                    console.close_profile_hub_page();
                }
                if let (Some(menu), Some(console)) = (&mut self.client_menu, &self.console) {
                    menu.open_player_hub(console, target);
                    if in_game {
                        self.game_menu = false;
                    }
                }
            }
            Tab::Profile | Tab::Identity => {
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
                    console.open_profile_hub_page(tab);
                }
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
        let target = match current {
            Tab::Character => self
                .client_menu
                .as_ref()
                .map_or(ReturnTarget::MainMenu, |menu| menu.player_return_target()),
            // The game menu stays up under the console's pages opened from it.
            Tab::Profile | Tab::Identity if self.game_menu => ReturnTarget::InGame,
            Tab::Profile | Tab::Identity => ReturnTarget::MainMenu,
        };
        self.show_profile_hub(tab, target);
    }

    /// Ctrl+Tab (Ctrl+Shift+Tab back) while the Profile screen shows: its next tab.
    /// Returns whether the key was taken.
    pub(crate) fn profile_hub_key(&mut self, event: &KeyEvent) -> bool {
        if event.physical_key != PhysicalKey::Code(KeyCode::Tab) {
            return false;
        }
        let Some(console) = self.console.as_ref() else {
            return false;
        };
        if !console.control_held() {
            return false;
        }
        let Some(current) = self.profile_hub_tab() else {
            return false;
        };
        if event.state == ElementState::Pressed && !event.repeat {
            let forward = !console.shift_held();
            self.switch_profile_hub(current.next(forward));
        }
        true
    }

    /// A press and release on one of the Profile screen's tabs switch to it; the
    /// screen under the strip sees neither. Returns whether the event was taken.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tabs_wrap_both_ways() {
        assert_eq!(Tab::Character.next(true), Tab::Profile);
        assert_eq!(Tab::Identity.next(true), Tab::Character);
        assert_eq!(Tab::Character.next(false), Tab::Identity);
        assert_eq!(Tab::Profile.next(false), Tab::Character);
    }

    /// The strip the screens draw and the areas the switch hit-tests are the same,
    /// in every window.
    #[test]
    fn the_strip_answers_where_it_is_drawn() {
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            let mut canvas = MenuCanvas::new();
            canvas.begin_transparent(viewport);
            let frame = Frame::new(viewport);
            strip(&mut canvas, &frame, Tab::Profile);
            assert!(!canvas.overflowed());
            for tab in Tab::ALL {
                let area = canvas
                    .rect_for(TOKEN + tab.index() as u16)
                    .expect("a tab's area");
                let centre = Vec2::new(area.x + area.width * 0.5, area.y + area.height * 0.5);
                assert_eq!(tab_at(viewport, centre), Some(tab), "{viewport:?}");
                assert!(area.right() <= viewport[0] && area.y >= 0.0);
            }
            assert_eq!(tab_at(viewport, Vec2::new(10.0, 10.0)), None);
        }
    }
}
