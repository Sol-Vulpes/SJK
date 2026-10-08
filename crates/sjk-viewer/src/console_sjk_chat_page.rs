//! Console side of the SJK chat page (see `sjk_chat_panel.rs`): opening it, sending
//! what it asks (messages to the hub, mutes here, staff requests), and routing keys
//! and pointer events to it, as for the Staff page.

use super::sjk_chat_panel::{Inputs, PanelAction};
use super::*;
use sjk_ui::InputEvent;

/// The `sjkchat` command.
pub(crate) const SJK_CHAT_COMMAND: &str = "sjkchat";
pub(crate) const SJK_CHAT_HELP: &str =
    "Open the SJK chat, which every SJK player shares through the SJK hub";

impl ViewerConsole {
    /// `sjkchat`: show the page, or close it.
    pub(crate) fn toggle_sjk_chat_panel(&mut self) {
        if self.open && self.sjk_chat_panel.is_open() {
            self.close_sjk_chat_panel();
            return;
        }
        self.open_sjk_chat_panel();
    }

    /// Show the page (the main page's docked chat, the in-game SJK menu).
    pub(crate) fn open_sjk_chat_panel(&mut self) {
        let owns_console = !self.open;
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
        self.dead_key.settle();
        self.sjk_chat_panel.open(owns_console);
    }

    fn close_sjk_chat_panel(&mut self) {
        if self.sjk_chat_panel.close() {
            self.set_open(false);
        }
    }

    fn sjk_chat_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_sjk_chat_panel(),
            PanelAction::Send(text) => {
                if self.bool_cvar("cl_sjkChat") == Some(false) {
                    self.push_log("^3sjkchat: SJK chat is off (cl_sjkChat 1 turns it on)");
                } else if !crate::player_identity::chat(text) {
                    self.push_log("^3sjkchat: the SJK identity is off (cl_identity 1)");
                }
            }
            PanelAction::Mute(key_id, muted) => crate::player_identity::set_muted(&key_id, muted),
            PanelAction::Staff(request) => {
                if !crate::player_identity::staff(request) {
                    self.push_log("^3sjkchat: the identity is not running");
                }
            }
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn sjk_chat_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.sjk_chat_panel.is_open() {
            return false;
        }
        let action = self.sjk_chat_panel.handle_key(event, self.shift);
        self.sjk_chat_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn sjk_chat_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.sjk_chat_panel.is_open() {
            return false;
        }
        let action = self.sjk_chat_panel.handle_pointer(event);
        self.sjk_chat_panel_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_sjk_chat_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        // Read before the chat's lock is taken: they lock the identity too.
        let muted = crate::player_identity::muted_keys();
        let staff = crate::player_identity::is_staff();
        let enabled = self.bool_cvar("cl_sjkChat") != Some(false);
        let now = crate::menu::sjk::recent::now();
        let panel = &mut self.sjk_chat_panel;
        #[cfg(test)]
        if let Some((chat, staff)) = panel.preview.take() {
            let inputs = Inputs {
                chat: Some(&chat),
                muted: &muted,
                staff,
                enabled,
                now: u64::try_from(chat.messages.back().map_or(0, |m| m.at)).unwrap_or(0) + 90,
            };
            panel.append_sjk(&inputs, target, viewport);
            panel.preview = Some((chat, staff));
            return;
        }
        let mut target = Some(target);
        let _ = crate::player_identity::with_chat(|chat| {
            let inputs = Inputs {
                chat: Some(chat),
                muted: &muted,
                staff,
                enabled,
                now,
            };
            if let Some(target) = target.take() {
                panel.append_sjk(&inputs, target, viewport);
            }
        });
        // The identity is not running: the page says so.
        if let Some(target) = target.take() {
            let inputs = Inputs {
                chat: None,
                muted: &muted,
                staff,
                enabled,
                now,
            };
            panel.append_sjk(&inputs, target, viewport);
        }
    }

    /// Open the page on `chat` in place of the live one, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_sjk_chat(&mut self, chat: sjk_identity::ChatState, staff: bool) {
        self.sjk_chat_panel.open(true);
        self.sjk_chat_panel.preview = Some((chat, staff));
        self.set_open(true);
    }

    /// The page's draw list while it is shown.
    pub(super) fn sjk_chat_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.sjk_chat_panel.is_open()).then(|| self.sjk_chat_panel.draw_list())
    }
}
