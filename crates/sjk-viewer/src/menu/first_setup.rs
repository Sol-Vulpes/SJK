//! The First setup screen: at the main menu a pop-up that asks one thing at a time
//! (`settings/sjk_popup.rs`, the SJK UI's look in every menu style), offered at every
//! start until it is finished (its Done on the last step) or its "Don't show at start"
//! row is ticked, and opened by the `firstsetup` command; in a game, and from Settings,
//! Settings' FIRST SETUP list. `firstsetup import` opens the Import page
//! (`config_import.rs`). The command's old name `quicksetup` is gone, so that `quit` is
//! the only command a `q` completes to.

use super::classic::layout::{Entry, Page};
use super::classic::panel::Frame;
use super::*;
use crate::settings::quick::HIDE_CVAR;

/// Console command that opens the screen.
pub(crate) const COMMAND: &str = "firstsetup";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Open First setup; firstsetup import <file.cfg> brings name, model, FOV and keys from another client";
/// The command's argument that opens the Import page instead.
pub(crate) const IMPORT: &str = "import";

impl ClientMenu {
    /// Open the First setup screen, returning to `target` when it closes: over the
    /// main menu its pop-up, in every menu style (the pop-up is a page with only an
    /// SJK look, and a look picked on it changes the style under it); in a game its
    /// list ([`Self::open_first_setup_list`]).
    pub(crate) fn open_first_setup(&mut self, console: &ViewerConsole, target: ReturnTarget) {
        if target == ReturnTarget::MainMenu {
            self.open_sjk_first_setup(console, target);
            return;
        }
        self.open_first_setup_list(console, target);
    }

    /// Open First setup as a list of every row: the classic Setup panel's FIRST SETUP
    /// group (what the pop-up's All settings shows under the classic style, and the
    /// `firstsetup` command in a game), the SJK UI's Settings on First setup if that
    /// panel cannot open.
    pub(crate) fn open_first_setup_list(&mut self, console: &ViewerConsole, target: ReturnTarget) {
        let frame = match target {
            ReturnTarget::MainMenu => Frame::Main,
            ReturnTarget::InGame => Frame::InGame,
        };
        if self.open_classic_panel(console, Page::Setup, Entry::FirstSetup, frame, target) {
            return;
        }
        self.open_sjk_settings(console, super::sjk::settings::FIRST_SETUP, target);
    }

    /// The first time the main menu is up in a run, once the menu style is known,
    /// open the screen over it, unless the player finished it or ticked "Don't show
    /// at start" (`ui_hideFirstSetup`, which Done on its last step sets too). Escape
    /// leaves it for this run.
    pub(crate) fn offer_first_setup(&mut self, console: &mut ViewerConsole) -> bool {
        if self.first_setup_offered
            || console.bool_cvar(HIDE_CVAR) == Some(true)
            || *self.state.phase() != ClientPhase::MainMenu
        {
            return false;
        }
        self.first_setup_offered = true;
        self.open_first_setup(console, ReturnTarget::MainMenu);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn every_start_offers_the_screen_once_until_it_is_hidden() {
        let (_directory, mut console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        assert_eq!(console.bool_cvar(HIDE_CVAR), Some(false));
        assert!(menu.offer_first_setup(&mut console));
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        // Not again in the same run.
        menu.state.main_menu();
        assert!(!menu.offer_first_setup(&mut console));
        // The next start offers it again, until the player finishes or hides it.
        let mut next = ClientMenu::new(true, String::new());
        assert!(next.offer_first_setup(&mut console));
        console.set_cvar(HIDE_CVAR, "1");
        let mut hidden = ClientMenu::new(true, String::new());
        assert!(!hidden.offer_first_setup(&mut console));
        assert_eq!(*hidden.state.phase(), ClientPhase::MainMenu);
    }

    /// Over the main menu First setup is its pop-up in every menu style; in a game,
    /// its list.
    #[test]
    fn the_popup_opens_over_the_main_menu_in_every_menu_style() {
        for style in [MenuStyle::Sjk, MenuStyle::Classic] {
            let (_directory, mut console) = console();
            let mut menu = ClientMenu::new(true, String::new());
            menu.menu_style = style;
            // At start.
            assert!(menu.offer_first_setup(&mut console));
            assert!(
                menu.sjk_settings_on_show() && menu.settings.popup(),
                "{style:?}"
            );
            assert!(menu.classic_panel.is_none());
            assert!(menu.sjk_screen(), "it is drawn as the SJK UI's");
            assert_eq!(
                menu.sjk_settings.category(),
                super::super::sjk::settings::FIRST_SETUP
            );
            // And by the command.
            menu.settings_result(crate::settings::SettingsResult::Back, &mut console);
            assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
            menu.open_first_setup(&console, ReturnTarget::MainMenu);
            assert!(
                menu.sjk_settings_on_show() && menu.settings.popup(),
                "{style:?}"
            );
            // In a game it is the list, in the in-game frame.
            menu.open_first_setup(&console, ReturnTarget::InGame);
            assert!(!menu.sjk_settings_on_show() && !menu.settings.popup());
            assert_eq!(
                menu.classic_panel.map(|panel| (panel.entry, panel.frame)),
                Some((Entry::FirstSetup, Frame::InGame))
            );
        }
    }

    /// The list is still what the classic Setup page's First setup opens.
    #[test]
    fn the_list_opens_as_the_classic_setup_panel_group() {
        let (_directory, console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        menu.menu_style = MenuStyle::Classic;
        menu.open_first_setup_list(&console, ReturnTarget::MainMenu);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        let panel = menu.classic_panel.expect("a classic panel");
        assert_eq!((panel.page, panel.entry), (Page::Setup, Entry::FirstSetup));
        assert_eq!(panel.frame, Frame::Main);
        assert!(!menu.settings.popup());
    }

    /// Done on the last step finishes First setup: the next start does not offer it,
    /// in either menu style, and the command still opens it. Closed before its end
    /// (Escape), it is offered again.
    #[test]
    fn a_finished_first_setup_is_not_offered_at_the_next_start() {
        use winit::keyboard::KeyCode;
        for style in [MenuStyle::Sjk, MenuStyle::Classic] {
            let (_directory, mut console) = console();
            let mut menu = ClientMenu::new(true, String::new());
            menu.menu_style = style;
            assert!(menu.offer_first_setup(&mut console));
            // Closed early: offered at the next start.
            let result = menu
                .settings
                .popup_key_for_test(KeyCode::Escape, &mut console);
            menu.settings_result(result, &mut console);
            assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
            let mut next = ClientMenu::new(true, String::new());
            next.menu_style = style;
            assert!(next.offer_first_setup(&mut console));
            // Gone through to its end: Page Down is Next, and Done on the last step.
            let mut result = crate::settings::SettingsResult::None;
            for _ in 0..crate::settings::setup_steps_len() {
                result = next
                    .settings
                    .popup_key_for_test(KeyCode::PageDown, &mut console);
            }
            assert!(matches!(result, crate::settings::SettingsResult::Back));
            next.settings_result(result, &mut console);
            assert_eq!(*next.state.phase(), ClientPhase::MainMenu);
            assert_eq!(console.bool_cvar(HIDE_CVAR), Some(true));
            let mut after = ClientMenu::new(true, String::new());
            after.menu_style = style;
            assert!(!after.offer_first_setup(&mut console));
            after.open_first_setup(&console, ReturnTarget::MainMenu);
            assert!(after.settings.popup(), "the command opens it again");
        }
    }

    /// Choose a character opens the Character page, and closing it shows the pop-up
    /// on its step again; a Character page opened another way goes back to the menu.
    #[test]
    fn the_character_page_comes_back_to_the_popups_step() {
        use crate::settings::SettingsResult;
        for style in [MenuStyle::Sjk, MenuStyle::Classic] {
            let (_directory, mut console) = console();
            let mut menu = ClientMenu::new(true, String::new());
            menu.menu_style = style;
            menu.open_first_setup(&console, ReturnTarget::MainMenu);
            menu.settings.setup_step_for_shot(&console, 2, Some(1));
            menu.settings_result(SettingsResult::OpenCharacter, &mut console);
            assert_eq!(*menu.state.phase(), ClientPhase::Player);
            menu.player_result(
                crate::player_menu::PlayerMenuResult::Back(ReturnTarget::MainMenu),
                &console,
            );
            assert_eq!(*menu.state.phase(), ClientPhase::Settings);
            assert!(menu.sjk_settings_on_show() && menu.settings.popup());
            assert_eq!(menu.settings.setup_step(), 2);
            // From the main menu: back to the main menu.
            menu.settings_result(SettingsResult::Back, &mut console);
            menu.open_player(&console, ReturnTarget::MainMenu);
            menu.player_result(
                crate::player_menu::PlayerMenuResult::Back(ReturnTarget::MainMenu),
                &console,
            );
            assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
        }
    }

    /// The pop-up is the SJK UI's in every menu style: a look picked on it changes
    /// the style under it (EJK mode turns the classic menus on) and it stays open, on
    /// its step, in either direction. Done then goes back to the main menu.
    #[test]
    fn the_popup_stays_when_the_menu_style_changes_under_it() {
        use crate::settings::SettingsResult;
        let (_directory, mut console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        menu.open_first_setup(&console, ReturnTarget::MainMenu);
        menu.settings.setup_step_for_shot(&console, 3, None);
        assert!(menu.sjk_settings_on_show() && menu.settings.popup());
        menu.set_menu_style(MenuStyle::Classic, &console);
        assert!(menu.classic_panel.is_none());
        assert!(menu.sjk_settings_on_show() && menu.settings.popup());
        assert!(menu.sjk_screen(), "it is drawn as the SJK UI's");
        assert_eq!(menu.settings.setup_step(), 3);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        menu.set_menu_style(MenuStyle::Sjk, &console);
        assert!(menu.sjk_settings_on_show() && menu.settings.popup());
        assert_eq!(menu.settings.setup_step(), 3);
        // EJK mode itself: its switch, the mode applying, the style following.
        console.set_cvar(crate::ejk_mode::CVAR, "1");
        crate::ejk_mode::follow(&mut console);
        let style = MenuStyle::from_cvar(console.text_value(crate::menu::style::CVAR));
        assert_eq!(style, MenuStyle::Classic);
        menu.set_menu_style(style, &console);
        assert!(menu.sjk_settings_on_show() && menu.settings.popup());
        assert_eq!(menu.settings.setup_step(), 3);
        // All settings under the classic style: that style's panel, the whole list.
        menu.settings_result(SettingsResult::AllSettings, &mut console);
        let panel = menu.classic_panel.expect("a classic panel");
        assert_eq!((panel.page, panel.entry), (Page::Setup, Entry::FirstSetup));
        assert!(!menu.settings.popup() && !menu.sjk_settings_on_show());
        menu.settings_result(SettingsResult::Back, &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
        // Opened again under the classic style, First setup is the pop-up again.
        menu.open_first_setup(&console, ReturnTarget::MainMenu);
        assert!(menu.classic_panel.is_none());
        assert!(menu.sjk_settings_on_show() && menu.settings.popup());
    }

    /// All settings (or Tab) leaves the pop-up for the whole screen on First
    /// setup; Done (or Escape) goes back to the main page.
    #[test]
    fn all_settings_opens_the_screen_on_first_setup_and_done_closes() {
        use crate::settings::SettingsResult;
        let (_directory, mut console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        menu.open_first_setup(&console, ReturnTarget::MainMenu);
        let action = menu.settings_result(SettingsResult::AllSettings, &mut console);
        assert_eq!(action, MenuAction::None);
        assert!(menu.sjk_settings_on_show() && !menu.settings.popup());
        assert_eq!(
            menu.sjk_settings.category(),
            super::super::sjk::settings::FIRST_SETUP
        );
        menu.open_first_setup(&console, ReturnTarget::MainMenu);
        menu.settings_result(SettingsResult::Back, &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
        assert!(!menu.sjk_settings_on_show());
    }

    /// Picking the SJK UI on First setup's Menu style row, from the classic
    /// panel, goes on as the pop-up.
    #[test]
    fn first_setup_picked_to_the_sjk_ui_goes_on_as_its_popup() {
        let (_directory, console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        menu.menu_style = MenuStyle::Classic;
        menu.open_first_setup_list(&console, ReturnTarget::MainMenu);
        assert!(!menu.settings.popup());
        menu.set_menu_style(MenuStyle::Sjk, &console);
        assert!(menu.sjk_settings_on_show() && menu.settings.popup());
        assert!(menu.classic_panel.is_none());
        // Another screen switched to the SJK UI stays itself.
        let mut menu = ClientMenu::new(true, String::new());
        menu.menu_style = MenuStyle::Classic;
        menu.set_menu_style(MenuStyle::Sjk, &console);
        assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
    }
}
