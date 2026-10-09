//! Console side of the Profile page (see `profile_panel.rs`): opening it on a tab,
//! saving the bio it asks to save, and routing keys and pointer events to it while it
//! is open, as for the Identity page. It is drawn in the SJK UI's look whatever the
//! menu style (`console_sjk_pages.rs`).

use super::profile_panel::{Inputs, Mode, PanelAction, Tab};
use super::*;
use crate::profile_hub::Tab as HubTab;
use sjk_ui::InputEvent;

/// The `profile` command.
pub(crate) const PROFILE_COMMAND: &str = "profile";
pub(crate) const PROFILE_HELP: &str = "Open your SJK profile: medals, bio and record";
/// The `achievements` command.
pub(crate) const ACHIEVEMENTS_COMMAND: &str = "achievements";
pub(crate) const ACHIEVEMENTS_HELP: &str = "Open the achievements board";

impl ViewerConsole {
    /// `profile` or `achievements` in the classic menus: show the page on `tab` in front
    /// of the console, opening it if needed, or close it when it already shows that tab.
    pub(crate) fn toggle_profile_panel(&mut self, tab: Tab) {
        if self.open && self.profile_panel.is_open() && self.profile_panel.tab() == tab {
            self.close_profile_panel();
            return;
        }
        self.open_profile_panel(tab);
    }

    /// Show the page on `tab`, on its own (the `profile` and `achievements` commands in
    /// the classic menus).
    pub(crate) fn open_profile_panel(&mut self, tab: Tab) {
        let owns_console = !self.open;
        self.show_profile_panel(tab, owns_console, Mode::Pages);
    }

    /// Show the page on `tab` as `mode` says; closing it closes the console too when
    /// `owns_console`.
    fn show_profile_panel(&mut self, tab: Tab, owns_console: bool, mode: Mode) {
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.debug_panel.close();
        self.changelog.close();
        self.update_panel.close();
        self.config_import.close();
        self.credits.close();
        self.identity_panel.close();
        self.staff_panel.close();
        self.sjk_chat_panel.close();
        self.unlockables_panel.close();
        self.dead_key.settle();
        self.profile_panel.open_as(tab, owns_console, mode);
    }

    /// The Profile screen's tab the console shows ([`crate::profile_hub`]): its
    /// Profile page or Unlockables page opened as one of the screen's tabs.
    pub(crate) fn profile_hub_tab(&self) -> Option<HubTab> {
        if !self.open {
            None
        } else if self.profile_panel.is_open() && self.profile_panel.mode() == Mode::Hub {
            Some(match self.profile_panel.tab() {
                Tab::Profile => HubTab::Profile,
                Tab::Achievements => HubTab::Achievements,
                Tab::Medals => HubTab::Medals,
            })
        } else if self.unlockables_panel.is_open() && self.unlockables_panel.is_hub() {
            Some(HubTab::Collection)
        } else {
            None
        }
    }

    /// Where the Profile screen returns while the console shows one of its tabs.
    pub(crate) fn profile_hub_return(&self) -> crate::player_menu::ReturnTarget {
        self.profile_hub_return
    }

    /// Show the Profile screen's tab `tab` (one the console shows) in place of the
    /// console, returning to `target`. When another of its tabs showed here, the
    /// console still closes with it if that one had opened it; between the Profile
    /// page's own tabs the page only turns, keeping a picture read and not sent.
    pub(crate) fn open_profile_hub_page(
        &mut self,
        tab: HubTab,
        target: crate::player_menu::ReturnTarget,
    ) {
        self.profile_hub_return = target;
        let page_tab = match tab {
            HubTab::Profile => Some(Tab::Profile),
            HubTab::Achievements => Some(Tab::Achievements),
            HubTab::Medals => Some(Tab::Medals),
            HubTab::Collection => None,
            // The player screen's pages are not the console's.
            HubTab::Character | HubTab::Saber | HubTab::Force => return,
        };
        let current = self.profile_hub_tab();
        if let (Some(page_tab), Some(HubTab::Profile | HubTab::Achievements | HubTab::Medals)) =
            (page_tab, current)
        {
            self.profile_panel.show_hub_tab(page_tab);
            return;
        }
        let owns_console = if current.is_some() {
            let profile = self.profile_panel.close();
            let unlockables = self.unlockables_panel.close();
            profile || unlockables
        } else {
            !self.open
        };
        match page_tab {
            Some(page_tab) => self.show_profile_panel(page_tab, owns_console, Mode::Hub),
            None => {
                self.show_unlockables_panel(owns_console);
                self.unlockables_panel.set_hub(true);
            }
        }
    }

    /// Close the Profile screen's page the console shows (for the player screen's
    /// tabs), and the console with it when the page had opened it.
    pub(crate) fn close_profile_hub_page(&mut self) {
        if self.profile_hub_tab().is_none() {
            return;
        }
        let profile = self.profile_panel.close();
        let unlockables = self.unlockables_panel.close();
        if profile || unlockables {
            self.set_open(false);
        }
    }

    fn close_profile_panel(&mut self) {
        if self.profile_panel.close() {
            self.set_open(false);
        }
    }

    fn profile_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_profile_panel(),
            PanelAction::Staff => {
                let owns_console = self.profile_panel.close();
                self.open_staff_panel();
                if owns_console {
                    self.staff_panel_owns_console();
                }
            }
            PanelAction::Unlockables => {
                let owns_console = self.profile_panel.close();
                self.open_unlockables_panel();
                if owns_console {
                    self.unlockables_panel_owns_console();
                }
            }
            PanelAction::Hub(tab) => {
                let target = self.profile_hub_return;
                self.open_profile_hub_page(tab, target);
            }
            PanelAction::Save { bio } => {
                // The page shows the hub's answer, or why nothing could be sent.
                if !crate::player_identity::set_bio(bio) {
                    self.push_log("^3profile: the identity is not running yet");
                }
            }
            PanelAction::SetAvatar { png } => {
                if !crate::player_identity::set_avatar(png) {
                    self.profile_panel
                        .picture_failed("Not sent: turn the SJK identity on first (cl_identity 1)");
                }
            }
            PanelAction::RemoveAvatar => {
                if !crate::player_identity::remove_avatar() {
                    self.profile_panel
                        .picture_failed("Not sent: turn the SJK identity on first (cl_identity 1)");
                }
            }
        }
    }

    /// Whether the Profile page shows.
    pub(crate) fn profile_panel_shown(&self) -> bool {
        self.open && self.profile_panel.is_open()
    }

    /// Open the Profile page if needed, on its picture panel.
    pub(crate) fn profile_show_picture(&mut self) {
        if !self.profile_panel_shown() {
            self.open_profile_panel(Tab::Profile);
        }
        self.profile_panel.show_picture();
    }

    /// Read the picture file at `path` and show it on the Profile page, ready to send
    /// (a file dropped on the window, `sjkavatar <file>`).
    pub(crate) fn profile_load_picture(&mut self, path: &std::path::Path) {
        if !self.profile_panel_shown() {
            self.open_profile_panel(Tab::Profile);
        }
        self.profile_panel.load_picture(path.to_owned());
    }

    /// Take the player's picture down (`sjkavatar clear`), on the Profile page.
    pub(crate) fn profile_remove_picture(&mut self) {
        if !self.profile_panel_shown() {
            self.open_profile_panel(Tab::Profile);
        }
        let action = self.profile_panel.remove_picture_now();
        self.profile_panel_action(action);
    }

    /// The identity service's last notice, to tell a save's answer.
    fn profile_notice() -> Option<String> {
        crate::player_identity::snapshot().and_then(|snapshot| snapshot.notice)
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn profile_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.profile_panel.is_open() {
            return false;
        }
        let action = self
            .profile_panel
            .handle_key(event, self.shift, Self::profile_notice());
        self.profile_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn profile_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.profile_panel.is_open() {
            return false;
        }
        let action = self
            .profile_panel
            .handle_pointer(event, Self::profile_notice());
        self.profile_panel_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_profile_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        if self.profile_panel.mode() == Mode::Hub {
            let back = self.profile_hub_return;
            let name = self.text_value("name").unwrap_or_default();
            if !self.profile_panel.hub_header().is(back, name) {
                let header = crate::profile_hub::Header {
                    back,
                    name: name.to_owned(),
                };
                self.profile_panel.set_hub_header(header);
            }
        }
        #[cfg(test)]
        if let Some(preview) = self.profile_panel.preview.take() {
            let inputs = Inputs {
                enabled: true,
                snapshot: Some(&preview.snapshot),
                standings: &preview.standings,
                record: &preview.record,
            };
            self.profile_panel.append_sjk(&inputs, target, viewport);
            self.profile_panel.preview = Some(preview);
            return;
        }
        // Copied out so the page can borrow itself mutably while it draws.
        let snapshot = crate::player_identity::snapshot();
        let held = snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.me.as_ref())
            .map(|me| me.achievements.clone())
            .unwrap_or_default();
        let standings = crate::achievements::standings(&held);
        let record = record();
        let inputs = Inputs {
            enabled: self.bool_cvar("cl_identity") == Some(true),
            snapshot: snapshot.as_ref(),
            standings: &standings,
            record: &record,
        };
        self.profile_panel.append_sjk(&inputs, target, viewport);
    }

    /// Show `preview` in place of the live identity and counts, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_profile(&mut self, preview: super::profile_panel::Preview) {
        self.profile_panel.preview = Some(preview);
    }

    /// Type `text` into the open page's bio, for a world shot.
    #[cfg(test)]
    pub(crate) fn profile_type_bio(&mut self, text: &str) {
        self.profile_panel.type_for_shot(text);
    }

    /// The page's draw list while it is shown.
    pub(super) fn profile_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.profile_panel.is_open()).then(|| self.profile_panel.draw_list())
    }
}

/// The player's record as the Profile tab lists it.
fn record() -> Vec<(&'static str, String)> {
    use crate::achievements::{Counter, count};
    let minutes = count(Counter::Minutes);
    vec![
        ("Players defeated", count(Counter::Kills).to_string()),
        ("Saber kills", count(Counter::SaberKills).to_string()),
        ("Best streak", count(Counter::BestStreak).to_string()),
        ("Duels won", count(Counter::DuelWins).to_string()),
        ("Flags captured", count(Counter::Captures).to_string()),
        ("Maps played", count(Counter::Maps).to_string()),
        ("Servers played", count(Counter::Servers).to_string()),
        (
            "Time played",
            format!("{} h {:02} min", minutes / 60, minutes % 60),
        ),
    ]
}
