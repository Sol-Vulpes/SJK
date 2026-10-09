//! What the main page's docked SJK chat shows ([`super::home::ChatDock`]), read from
//! the identity service only when the chat changed: its last lines, who is online
//! and why it cannot send or read.

use super::home::{ChatDock, DockLine};
use crate::player_identity;

/// Lines the dock keeps, as many as it shows.
pub(crate) const LINES: usize = 5;

/// The dock's copy of the chat.
#[derive(Default)]
pub(crate) struct DockCache {
    /// The chat's revision, outcome serial and mutes revision last read; `None` while
    /// the service is not running.
    mark: Option<Option<(u64, u64, u64)>>,
    /// Name, text and verified, oldest first.
    lines: Vec<(String, String, bool)>,
    online: u32,
    live: bool,
    /// Why the hub refused the last message.
    refused: String,
    /// Why the last message could not go at all, set by the menu.
    pub(crate) local: &'static str,
}

impl DockCache {
    /// Read the chat again if it changed since the last frame.
    pub(crate) fn refresh(&mut self) {
        let mutes = player_identity::mutes_revision();
        let mark = player_identity::with_chat(|chat| {
            (
                chat.revision,
                chat.outcome.as_ref().map_or(0, |outcome| outcome.serial),
                mutes,
            )
        });
        if self.mark == Some(mark) {
            return;
        }
        self.mark = Some(mark);
        self.lines.clear();
        self.refused.clear();
        let muted = player_identity::muted_keys();
        let read = player_identity::with_chat(|chat| {
            let shown = chat
                .messages
                .iter()
                .filter(|message| !muted.contains(&message.key_id));
            let skip = shown.clone().count().saturating_sub(LINES);
            for message in shown.skip(skip) {
                self.lines.push((
                    sjk_identity::chat::for_display(&message.name),
                    crate::sjk_chat_look::message_text(&message.text),
                    message.verified,
                ));
            }
            if let Some(outcome) = chat.outcome.as_ref().filter(|outcome| !outcome.sent) {
                self.refused.clone_from(&outcome.message);
            }
            (chat.online, chat.live)
        });
        (self.online, self.live) = read.unwrap_or((0, false));
        if read.is_some() {
            self.local = "";
        }
    }

    /// The dock's view.
    pub(crate) fn view<'a>(&'a self, lines: &'a mut [DockLine<'a>; LINES]) -> ChatDock<'a> {
        for (slot, (name, text, verified)) in lines.iter_mut().zip(&self.lines) {
            *slot = DockLine {
                name,
                text,
                verified: *verified,
            };
        }
        let notice = if !self.local.is_empty() {
            self.local
        } else if self.mark == Some(None) {
            "Turn the SJK identity on to chat"
        } else {
            self.refused.as_str()
        };
        ChatDock {
            lines: &lines[..self.lines.len()],
            online: self.online,
            live: self.live,
            notice,
            measure: None,
        }
    }
}

/// A blank line for the dock's array.
pub(crate) const BLANK: DockLine<'static> = DockLine {
    name: "",
    text: "",
    verified: false,
};
