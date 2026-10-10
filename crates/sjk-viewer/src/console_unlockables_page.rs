//! Console side of the Unlockables page (see `unlockables_panel.rs`): opening it,
//! setting `cg_saberSkin` when it equips or unequips a blade skin, and routing keys and
//! pointer events to it, as for the Staff page. It is drawn in the SJK UI's look
//! whatever the menu style (`console_sjk_pages.rs`).

use super::unlockables_panel::{Inputs, PanelAction};
use super::*;
use crate::unlockables::{Holdings, SABER_SKIN_CVAR};
use sjk_ui::InputEvent;

/// The `unlockables` command.
pub(crate) const UNLOCKABLES_COMMAND: &str = "unlockables";
pub(crate) const UNLOCKABLES_HELP: &str =
    "Open your SJK unlockables: blade skins you own or can unlock, and what you wear";

impl ViewerConsole {
    /// `unlockables` in the classic menus: show the page, or close it.
    pub(crate) fn toggle_unlockables_panel(&mut self) {
        if self.open && self.unlockables_panel.is_open() {
            self.close_unlockables_panel();
            return;
        }
        self.open_unlockables_panel();
    }

    /// Show the page on its own (the Profile page's See unlockables in the classic
    /// menus).
    pub(crate) fn open_unlockables_panel(&mut self) {
        let owns_console = !self.open;
        self.show_unlockables_panel(owns_console);
    }

    /// Show the page on its own; closing it closes the console too when
    /// `owns_console`.
    pub(super) fn show_unlockables_panel(&mut self, owns_console: bool) {
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
        self.profile_panel.close();
        self.staff_panel.close();
        self.sjk_chat_panel.close();
        self.holocrons_panel.close();
        self.dead_key.settle();
        self.unlockables_panel.open(owns_console);
    }

    /// Closing the page closes the console too (it was opened from the Profile page,
    /// which had opened it).
    pub(super) fn unlockables_panel_owns_console(&mut self) {
        self.unlockables_panel.own_console();
    }

    fn close_unlockables_panel(&mut self) {
        if self.unlockables_panel.close() {
            self.set_open(false);
        }
    }

    fn unlockables_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_unlockables_panel(),
            PanelAction::Wear(id) => crate::unlockables::wear(self, id),
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn unlockables_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.unlockables_panel.is_open() {
            return false;
        }
        let action = self.unlockables_panel.handle_key(event, self.shift);
        self.unlockables_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn unlockables_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.unlockables_panel.is_open() {
            return false;
        }
        let action = self.unlockables_panel.handle_pointer(event);
        self.unlockables_panel_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_unlockables_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        self.unlockables_panel.follow_skins();
        if self.unlockables_panel.is_hub() {
            let back = self.profile_hub_return;
            let name = self.text_value("name").unwrap_or_default();
            if !self.unlockables_panel.hub_header().is(back, name) {
                let header = crate::profile_hub::Header {
                    back,
                    name: name.to_owned(),
                };
                self.unlockables_panel.set_hub_header(header);
            }
        }
        // Copied out so the page can borrow itself mutably while it draws.
        let setting = self
            .text_cvar(SABER_SKIN_CVAR)
            .unwrap_or_default()
            .to_owned();
        #[cfg(test)]
        if let Some(unlocks) = self.unlockables_panel.preview.take() {
            let inputs = Inputs {
                holdings: Holdings::Known(&unlocks),
                setting: &setting,
            };
            self.unlockables_panel.append_sjk(&inputs, target, viewport);
            self.unlockables_panel.preview = Some(unlocks);
            return;
        }
        let snapshot = crate::player_identity::snapshot();
        let inputs = Inputs {
            holdings: Holdings::of(
                self.bool_cvar("cl_identity") == Some(true),
                snapshot.as_ref(),
            ),
            setting: &setting,
        };
        self.unlockables_panel.append_sjk(&inputs, target, viewport);
    }

    /// Open the page on `unlocks` in place of the own profile's (`None`: the live ones),
    /// its swatches drawn at `seconds`, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_unlockables(
        &mut self,
        unlocks: Option<Vec<sjk_identity::Unlock>>,
        seconds: f32,
    ) {
        self.open_unlockables_panel();
        self.unlockables_panel.preview = unlocks;
        self.unlockables_panel.shot_seconds = Some(seconds);
    }

    /// Show `unlocks` (`None`: the live ones) whenever the page opens, the Profile
    /// screen's Collection tab too, its swatches drawn at `seconds`, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_collection(
        &mut self,
        unlocks: Option<Vec<sjk_identity::Unlock>>,
        seconds: f32,
    ) {
        self.unlockables_panel.preview = unlocks;
        self.unlockables_panel.shot_seconds = Some(seconds);
    }

    /// Whether the Profile page's, the Unlockables page's or the Holocrons page's last frame ran out of
    /// room on its canvas, for the world shots.
    #[cfg(test)]
    pub(crate) fn profile_pages_overflowed(&self) -> bool {
        self.profile_panel.overflowed()
            || self.unlockables_panel.overflowed()
            || self.holocrons_panel.overflowed()
    }

    /// The page's draw list while it is shown.
    pub(super) fn unlockables_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.unlockables_panel.is_open()).then(|| self.unlockables_panel.draw_list())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_opens_alone_and_equipping_sets_the_cvar() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.open_profile_panel(super::super::profile_panel::Tab::Profile);
        console.toggle_unlockables_panel();
        assert!(console.unlockables_panel.is_open());
        assert!(!console.profile_panel.is_open());
        assert!(console.sjk_page_open(), "drawn in the SJK UI's look");
        assert!(console.covers_frame());
        console.unlockables_panel_action(PanelAction::Wear("saber_sun"));
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "saber_sun");
        console.unlockables_panel_action(PanelAction::Wear(""));
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "");
        // It was opened with the console already open (the Profile page's).
        console.unlockables_panel_action(PanelAction::Close);
        assert!(!console.unlockables_panel.is_open());
        assert!(console.is_open());
        console.toggle_unlockables_panel();
        console.unlockables_panel_owns_console();
        console.toggle_unlockables_panel();
        assert!(!console.is_open(), "it closes the console it took over");
    }
}
