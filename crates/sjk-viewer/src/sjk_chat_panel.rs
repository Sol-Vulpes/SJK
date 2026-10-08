//! The SJK chat page (`docs/hub-chat.md`): the chat every SJK player shares through
//! the SJK hub, in full. The history with names, verified and staff marks and how
//! long ago each message came; a field to write in and Send; and, for a chosen
//! message, Mute on this PC, and for SJK staff Delete and Mute at the hub.
//!
//! Opened by the main page's docked chat (Open chat), the `sjkchat` command or the
//! in-game SJK menu. Like the Staff page it lives in the console and has the SJK UI's
//! look ([`view`]). The field has the keyboard when the page opens: typing goes to
//! it, Enter sends, Up goes to the newest message; Up and Down move through the
//! messages, Page Up and Page Down scroll, Tab walks every control, Escape goes back.

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_identity::{ChatState, StaffRequest};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "sjk_chat_panel_view.rs"]
mod view;

const FIELD_TOKEN: u16 = 1_300;
const SEND_TOKEN: u16 = 1_301;
const MUTE_TOKEN: u16 = 1_302;
const DELETE_TOKEN: u16 = 1_303;
const HUB_MUTE_TOKEN: u16 = 1_304;
const HUB_UNMUTE_TOKEN: u16 = 1_305;
/// Messages on show, one token each.
const MESSAGE_BASE: u16 = 1_310;
const MESSAGES_SHOWN: usize = 40;
/// Longest message, as the hub takes it.
const DRAFT_MAX: usize = sjk_identity::chat::TEXT_MAX;
/// Messages Page Up and Page Down scroll by.
const PAGE: usize = 5;

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Send this message to the SJK chat.
    Send(String),
    /// Mute (true) or unmute this key on this PC.
    Mute(String, bool),
    /// A staff request to the hub.
    Staff(StaffRequest),
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// The chat, or `None` while the identity is not running.
    pub(crate) chat: Option<&'a ChatState>,
    /// The keys muted on this PC.
    pub(crate) muted: &'a [String],
    /// Whether the player's key is SJK staff.
    pub(crate) staff: bool,
    /// Whether `cl_sjkChat` is on.
    pub(crate) enabled: bool,
    /// Now, unix seconds, for how long ago messages came.
    pub(crate) now: u64,
    /// What the staff requests came to, for a staff key.
    pub(crate) staff_state: Option<StaffShown<'a>>,
}

/// The part of `sjk_identity::StaffState` the page shows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StaffShown<'a> {
    /// Counts answers.
    pub(crate) serial: u64,
    pub(crate) message: &'a str,
    pub(crate) failed: bool,
    /// A request is on its way.
    pub(crate) busy: bool,
}

/// What the last frame showed, for keys and clicks.
#[derive(Debug, Default)]
struct Shown {
    /// The messages on show, by token, as (id, sender's key), oldest first.
    messages: Vec<(u64, String)>,
    /// The chosen message's sender, and whether they are muted here.
    chosen: Option<(u64, String, bool)>,
    /// How many messages there are in all.
    total: usize,
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    ui: MenuCanvas,
    draft: String,
    /// The control the keyboard is on, by its token.
    focus: u16,
    /// The controls in the order Tab visits them, laid out by the last frame.
    order: Vec<u16>,
    /// The chosen message's id.
    selected: Option<u64>,
    /// Messages hidden below the list, scrolled up past.
    scroll: usize,
    /// The staff answers' serial when the page last sent a staff request: answers
    /// past it are this page's.
    staff_after: Option<u64>,
    shown: Shown,
    epoch: Instant,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            ui: MenuCanvas::with_capacities(260, 160, 1_600),
            draft: String::new(),
            focus: FIELD_TOKEN,
            order: Vec::with_capacity(64),
            selected: None,
            scroll: 0,
            staff_after: None,
            shown: Shown::default(),
            epoch: Instant::now(),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page, the keyboard in the field; `owns_console` when the console was
    /// closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.focus = FIELD_TOKEN;
        self.scroll = 0;
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    /// The page sent a staff request while the staff answers' serial was `serial`.
    pub(crate) fn staff_sent(&mut self, serial: u64) {
        self.staff_after = Some(serial);
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// What Enter (or a click) on `token` does.
    fn activate(&mut self, token: u16) -> PanelAction {
        self.focus = token;
        let chosen = self.shown.chosen.clone();
        match token {
            FIELD_TOKEN => PanelAction::None,
            SEND_TOKEN => {
                let text = self.draft.trim().to_owned();
                if text.is_empty() {
                    return PanelAction::None;
                }
                self.draft.clear();
                self.focus = FIELD_TOKEN;
                PanelAction::Send(text)
            }
            MUTE_TOKEN => match chosen {
                Some((_, key, muted)) => PanelAction::Mute(key, !muted),
                None => PanelAction::None,
            },
            DELETE_TOKEN | HUB_MUTE_TOKEN | HUB_UNMUTE_TOKEN => {
                // Only offered (laid out) for staff; the hub refuses others too.
                let Some((id, key_id, _)) = chosen.filter(|_| self.order.contains(&token)) else {
                    return PanelAction::None;
                };
                PanelAction::Staff(match token {
                    DELETE_TOKEN => StaffRequest::ChatDelete { id },
                    _ => StaffRequest::ChatMute {
                        key_id,
                        muted: token == HUB_MUTE_TOKEN,
                    },
                })
            }
            token if (MESSAGE_BASE..MESSAGE_BASE + MESSAGES_SHOWN as u16).contains(&token) => {
                if let Some((id, _)) = self.shown.messages.get(usize::from(token - MESSAGE_BASE)) {
                    self.selected = Some(*id);
                }
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// Move the keyboard to the next control (or the one before) of the last frame.
    fn step(&mut self, forward: bool) {
        if self.order.is_empty() {
            return;
        }
        let at = self.order.iter().position(|token| *token == self.focus);
        let next = match (at, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % self.order.len(),
            (Some(at), false) => (at + self.order.len() - 1) % self.order.len(),
        };
        self.focus = self.order[next];
    }

    /// Choose message `index` of those on show, scrolling when it is past an end.
    fn choose(&mut self, index: isize) {
        let count = self.shown.messages.len() as isize;
        if count == 0 {
            return;
        }
        if index < 0 {
            // Older than the oldest on show: scroll up by one, if there is more.
            if self.scroll + self.shown.messages.len() < self.shown.total {
                self.scroll += 1;
                self.selected = None;
            }
            let _ = self.activate(MESSAGE_BASE);
            return;
        }
        if index >= count {
            if self.scroll > 0 {
                self.scroll -= 1;
                let _ = self.activate(MESSAGE_BASE + (count - 1) as u16);
            } else {
                self.focus = FIELD_TOKEN;
            }
            return;
        }
        let _ = self.activate(MESSAGE_BASE + index as u16);
    }

    /// A key.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent, shift: bool) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        // Held keys repeat in the field (Backspace); elsewhere a key acts once.
        if event.repeat && self.focus != FIELD_TOKEN {
            return PanelAction::None;
        }
        self.edit(key, event.text.as_deref(), shift)
    }

    /// A pressed `key` that typed `text`, Shift held or not.
    fn edit(&mut self, key: KeyCode, text: Option<&str>, shift: bool) -> PanelAction {
        let typing = self.focus == FIELD_TOKEN;
        let in_list = (MESSAGE_BASE..MESSAGE_BASE + MESSAGES_SHOWN as u16).contains(&self.focus);
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(!shift),
            KeyCode::PageUp => {
                let most = self.shown.total.saturating_sub(1);
                self.scroll = (self.scroll + PAGE).min(most);
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_sub(PAGE),
            KeyCode::ArrowUp if typing => self.choose(self.shown.messages.len() as isize - 1),
            KeyCode::ArrowUp | KeyCode::ArrowDown if in_list => {
                let at = (self.focus - MESSAGE_BASE) as isize;
                self.choose(if key == KeyCode::ArrowUp {
                    at - 1
                } else {
                    at + 1
                });
            }
            KeyCode::ArrowDown if !typing => self.step(true),
            KeyCode::ArrowUp => self.step(false),
            KeyCode::Enter | KeyCode::NumpadEnter if typing => {
                // The keyboard stays in the field, sent or not.
                let action = self.activate(SEND_TOKEN);
                self.focus = FIELD_TOKEN;
                return action;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => return self.activate(self.focus),
            KeyCode::Space if !typing => return self.activate(self.focus),
            KeyCode::Backspace if typing => {
                self.draft.pop();
            }
            _ if typing => {
                let pasted;
                let text = match text {
                    Some("\u{16}") => {
                        pasted = crate::console::clipboard::paste().unwrap_or_default();
                        pasted.as_str()
                    }
                    Some(text) => text,
                    None => return PanelAction::None,
                };
                for c in text.chars().filter(|c| !c.is_control()) {
                    if self.draft.chars().count() >= DRAFT_MAX {
                        break;
                    }
                    self.draft.push(c);
                }
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        if let InputEvent::PointerWheel { delta, .. } = event {
            if delta.y > 0.0 {
                let most = self.shown.total.saturating_sub(1);
                self.scroll = (self.scroll + 1).min(most);
            } else if delta.y < 0.0 {
                self.scroll = self.scroll.saturating_sub(1);
            }
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(token) => self.activate(token),
            None => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::ChatMessage;

    pub(super) fn message(id: u64, key: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id,
            at: 1_791_300_000 + id as i64 * 60,
            key_id: key.to_owned(),
            name: format!("^{}Player {id}", id % 8),
            verified: id.is_multiple_of(3),
            staff: id.is_multiple_of(5),
            text: text.to_owned(),
        }
    }

    pub(super) fn chat(count: u64) -> ChatState {
        ChatState {
            messages: (1..=count)
                .map(|id| message(id, &format!("{id:016x}"), &format!("message {id}")))
                .collect(),
            revision: 1,
            online: 9,
            live: true,
            outcome: None,
            loaded: Some(1),
        }
    }

    pub(super) fn inputs<'a>(chat: &'a ChatState, muted: &'a [String], staff: bool) -> Inputs<'a> {
        Inputs {
            chat: Some(chat),
            muted,
            staff,
            enabled: true,
            now: 1_791_400_000,
            staff_state: None,
        }
    }

    /// The panel after one frame, so its controls are known.
    pub(super) fn drawn(panel: &mut Panel, inputs: &Inputs<'_>) {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(inputs, &fonts.font, [1920.0, 1080.0]);
    }

    fn press(panel: &mut Panel, key: KeyCode, text: Option<&str>) -> PanelAction {
        panel.edit(key, text, false)
    }

    #[test]
    fn the_page_lists_messages_newest_at_the_bottom_and_scrolls() {
        let chat = chat(60);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        let ids: Vec<u64> = panel.shown.messages.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.last(), Some(&60), "the newest at the bottom");
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(ids.len() < 60, "a page of them");
        // Up from the field chooses the newest; Page Up scrolls older ones in.
        press(&mut panel, KeyCode::ArrowUp, None);
        assert_eq!(panel.selected, Some(60));
        press(&mut panel, KeyCode::PageUp, None);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert_eq!(panel.shown.messages.last().map(|(id, _)| *id), Some(55));
        press(&mut panel, KeyCode::PageDown, None);
        assert_eq!(panel.scroll, 0);
    }

    #[test]
    fn up_and_down_move_through_the_messages() {
        let chat = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        press(&mut panel, KeyCode::ArrowUp, None);
        press(&mut panel, KeyCode::ArrowUp, None);
        assert_eq!(panel.selected, Some(2));
        press(&mut panel, KeyCode::ArrowDown, None);
        assert_eq!(panel.selected, Some(3));
        // Down from the newest goes back to the field.
        press(&mut panel, KeyCode::ArrowDown, None);
        assert_eq!(panel.focus, FIELD_TOKEN);
    }

    #[test]
    fn send_takes_the_draft_and_clears_it() {
        let chat = chat(1);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        for c in "gg wp".chars() {
            let text = c.to_string();
            assert_eq!(
                press(&mut panel, KeyCode::KeyA, Some(&text)),
                PanelAction::None
            );
        }
        // Keys that would act elsewhere type here.
        press(&mut panel, KeyCode::KeyS, Some("s"));
        press(&mut panel, KeyCode::Backspace, Some("\u{8}"));
        assert_eq!(
            press(&mut panel, KeyCode::Enter, Some("\r")),
            PanelAction::Send("gg wp".into())
        );
        assert!(panel.draft.is_empty());
        assert_eq!(
            press(&mut panel, KeyCode::Enter, Some("\r")),
            PanelAction::None
        );
        // The draft holds at most what the hub takes.
        for _ in 0..200 {
            press(&mut panel, KeyCode::KeyA, Some("a"));
        }
        assert_eq!(panel.draft.chars().count(), DRAFT_MAX);
        assert_eq!(press(&mut panel, KeyCode::Escape, None), PanelAction::Close);
    }

    #[test]
    fn a_chosen_message_can_be_muted_on_this_pc() {
        let chat = chat(2);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(!panel.order.contains(&MUTE_TOKEN), "nothing chosen yet");
        let _ = panel.activate(MESSAGE_BASE + 1);
        assert_eq!(panel.selected, Some(2));
        drawn(&mut panel, &inputs(&chat, &[], false));
        let key = format!("{:016x}", 2);
        assert_eq!(
            panel.activate(MUTE_TOKEN),
            PanelAction::Mute(key.clone(), true)
        );
        let muted = [key.clone()];
        drawn(&mut panel, &inputs(&chat, &muted, false));
        assert_eq!(panel.activate(MUTE_TOKEN), PanelAction::Mute(key, false));
    }

    #[test]
    fn staff_actions_only_for_staff() {
        let chat = chat(4);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        let _ = panel.activate(MESSAGE_BASE + 3);
        drawn(&mut panel, &inputs(&chat, &[], false));
        for token in [DELETE_TOKEN, HUB_MUTE_TOKEN, HUB_UNMUTE_TOKEN] {
            assert!(!panel.order.contains(&token));
            assert_eq!(panel.activate(token), PanelAction::None);
        }
        drawn(&mut panel, &inputs(&chat, &[], true));
        let key = format!("{:016x}", 4);
        assert_eq!(
            panel.activate(DELETE_TOKEN),
            PanelAction::Staff(StaffRequest::ChatDelete { id: 4 })
        );
        assert_eq!(
            panel.activate(HUB_MUTE_TOKEN),
            PanelAction::Staff(StaffRequest::ChatMute {
                key_id: key.clone(),
                muted: true
            })
        );
        assert_eq!(
            panel.activate(HUB_UNMUTE_TOKEN),
            PanelAction::Staff(StaffRequest::ChatMute {
                key_id: key,
                muted: false
            })
        );
    }

    #[test]
    fn tab_walks_every_control() {
        let chat = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], true));
        let _ = panel.activate(MESSAGE_BASE);
        drawn(&mut panel, &inputs(&chat, &[], true));
        for token in [
            FIELD_TOKEN,
            SEND_TOKEN,
            MESSAGE_BASE,
            MUTE_TOKEN,
            DELETE_TOKEN,
        ] {
            assert!(panel.order.contains(&token), "{token}");
        }
        let start = panel.focus;
        for _ in 0..panel.order.len() {
            panel.edit(KeyCode::Tab, Some("\t"), false);
        }
        assert_eq!(panel.focus, start, "Tab comes round");
        panel.edit(KeyCode::Tab, Some("\t"), true);
        assert_ne!(panel.focus, start);
    }
}
