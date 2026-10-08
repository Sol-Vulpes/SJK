//! The SJK chat page's drawing, in the SJK UI's look: the messages down the left,
//! newest at the bottom over the field and Send, and the chosen message with what
//! can be done about it on the right.

use super::*;
use crate::menu::sjk::recent::ago;
use crate::menu::sjk::{
    Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar, wrap,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use sjk_identity::ChatMessage;
use sjk_identity::chat::for_display;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The columns (frame pixels).
const LIST_X: f32 = 96.0;
const LIST_WIDTH: f32 = 1_180.0;
const SIDE_X: f32 = 1_340.0;
const SIDE_WIDTH: f32 = 484.0;
const TOP: f32 = 170.0;
/// The messages' area.
const LIST_TOP: f32 = 236.0;
const LIST_BOTTOM: f32 = 880.0;
/// A message: its name row, each row of its text and the gap after it.
const NAME_ROW: f32 = 28.0;
const BODY_ROW: f32 = 24.0;
const GAP: f32 = 12.0;
/// Characters a row of a message holds (each row is also cut to the column).
const WRAP_CHARS: usize = 136;
/// The field and Send.
const FIELD_Y: f32 = 900.0;
const FIELD_WIDTH: f32 = 1_040.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;

/// A message's height on the page, its text in `rows` rows.
fn height(rows: usize) -> f32 {
    NAME_ROW + rows.max(1) as f32 * BODY_ROW + GAP
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        let body = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        self.build(inputs, body, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out; `_body` is the body family, kept for measured layouts.
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, _body: &UiFont, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.order.clear();
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        top_bar(&mut self.ui, &frame, "Back", BACK_TOKEN, "SJK chat", None);
        self.status(&frame, inputs);
        self.messages(&frame, inputs);
        self.composer(&frame, inputs);
        self.chosen(&frame, inputs);
        self.keys(&frame);
        if !self.order.contains(&self.focus) {
            self.focus = FIELD_TOKEN;
        }
        self.ui.finish(self.focus);
    }

    /// Who reads the chat, or why it cannot be read, and what it is.
    fn status(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let (line, colour) = match inputs.chat {
            _ if !inputs.enabled => (
                "SJK chat is off (Settings > Network > SJK chat)".to_owned(),
                color::EMBER,
            ),
            None => (
                "The SJK identity is off: turn it on to chat (Settings > Network)".to_owned(),
                color::EMBER,
            ),
            Some(chat) if !chat.live => ("Connecting to the SJK hub...".to_owned(), color::MUTED),
            Some(chat) if chat.online == 1 => {
                ("1 SJK player reading".to_owned(), color::GOLD_BRIGHT)
            }
            Some(chat) => (
                format!("{} SJK players reading", chat.online),
                color::GOLD_BRIGHT,
            ),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{line}"),
            frame.rect(LIST_X, TOP - 4.0, LIST_WIDTH, 28.0),
            22.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Every SJK player reads this, in game and in the menus. The hub keeps the last 200 messages, in memory only."
            ),
            frame.rect(LIST_X, TOP + 26.0, LIST_WIDTH, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The messages that fit, newest at the bottom, `scroll` messages up from it.
    fn messages(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        self.shown.messages.clear();
        let empty = std::collections::VecDeque::new();
        let messages = inputs.chat.map_or(&empty, |chat| &chat.messages);
        self.shown.total = messages.len();
        self.scroll = self.scroll.min(messages.len().saturating_sub(1));
        // From the newest on show upwards, while they fit.
        let mut room = LIST_BOTTOM - LIST_TOP;
        let mut first = messages.len().saturating_sub(self.scroll);
        let end = first;
        while first > 0 && end - first < MESSAGES_SHOWN {
            let message = &messages[first - 1];
            let rows = self.body(message, inputs).len();
            let needed = height(rows);
            if needed > room {
                break;
            }
            room -= needed;
            first -= 1;
        }
        if first == end {
            let line = if inputs.chat.is_some() && inputs.enabled {
                "Nobody has said anything yet. Say hello."
            } else {
                ""
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(LIST_X, LIST_BOTTOM - 40.0, LIST_WIDTH, 24.0),
                17.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = LIST_TOP + room;
        for (index, message) in messages.range(first..end).enumerate() {
            let token = MESSAGE_BASE + index as u16;
            let body = self.body(message, inputs);
            let tall = height(body.len());
            let row = [LIST_X - 18.0, y - 4.0, LIST_WIDTH + 18.0, tall - 4.0];
            if self.focus == token || self.selected == Some(message.id) {
                kit::band(&mut self.ui, frame, row);
            }
            let name = for_display(&message.name);
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", if name.is_empty() { "(no name)" } else { &name }),
                frame.rect(LIST_X, y, LIST_WIDTH - 440.0, NAME_ROW - 2.0),
                20.0 * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let tags = match (message.staff, message.verified) {
                (true, true) => "Staff, verified  ·  ",
                (true, false) => "Staff  ·  ",
                (false, true) => "Verified  ·  ",
                (false, false) => "",
            };
            let when = ago(u64::try_from(message.at).unwrap_or(0), inputs.now);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{tags}{when}"),
                frame.rect(LIST_X + LIST_WIDTH - 420.0, y + 2.0, 420.0, NAME_ROW - 6.0),
                15.0 * s,
                if tags.is_empty() {
                    color::QUIET
                } else {
                    color::GOLD_BRIGHT
                },
                FontWeight::Regular,
                TextAlign::End,
            );
            let muted = inputs.muted.contains(&message.key_id);
            for (line, rect_y) in body
                .iter()
                .zip((0..).map(|row| y + NAME_ROW + row as f32 * BODY_ROW))
            {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{line}"),
                    frame.rect(LIST_X, rect_y, LIST_WIDTH, BODY_ROW - 2.0),
                    17.0 * s,
                    if muted { color::QUIET } else { color::TEXT },
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            self.ui
                .hit_region(token, frame.rect(row[0], row[1], row[2], row[3]));
            self.order.push(token);
            self.shown
                .messages
                .push((message.id, message.key_id.clone()));
            y += tall;
        }
        if first > 0 {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{first} older: Page Up"),
                frame.rect(LIST_X, LIST_TOP - 26.0, LIST_WIDTH, 20.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        if self.scroll > 0 {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{} newer: Page Down", self.scroll),
                frame.rect(LIST_X, LIST_BOTTOM - 2.0, LIST_WIDTH, 20.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
    }

    /// A message's text in rows, or that its sender is muted here.
    fn body(&self, message: &ChatMessage, inputs: &Inputs<'_>) -> Vec<String> {
        if inputs.muted.contains(&message.key_id) {
            return vec!["Muted on this PC".to_owned()];
        }
        let text = for_display(&message.text);
        let rows: Vec<String> = wrap(&text, WRAP_CHARS).map(str::to_owned).collect();
        if rows.is_empty() {
            vec![String::new()]
        } else {
            rows
        }
    }

    /// The field, Send, and what became of the last message.
    fn composer(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let focused = self.focus == FIELD_TOKEN;
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let count = self.draft.chars().count();
        let shown = if count > 96 {
            let tail: String = self.draft.chars().skip(count - 93).collect();
            format!("...{tail}")
        } else if self.draft.is_empty() && !focused {
            "Say something to every SJK player".to_owned()
        } else {
            self.draft.clone()
        };
        let field = [LIST_X, FIELD_Y, FIELD_WIDTH, 46.0];
        kit::field(
            &mut self.ui,
            frame,
            field,
            format_args!("{shown}{}", if caret { "|" } else { "" }),
            focused,
            false,
        );
        self.ui.hit_region(
            FIELD_TOKEN,
            frame.rect(field[0], field[1], field[2], field[3]),
        );
        self.order.push(FIELD_TOKEN);
        let can_send = !self.draft.trim().is_empty() && inputs.enabled && inputs.chat.is_some();
        kit::button(
            &mut self.ui,
            frame,
            [LIST_X + FIELD_WIDTH + 20.0, FIELD_Y + 2.0, 120.0, 42.0],
            "Send",
            true,
            can_send,
            self.focus == SEND_TOKEN,
            SEND_TOKEN,
        );
        self.order.push(SEND_TOKEN);
        let notice = inputs
            .chat
            .and_then(|chat| chat.outcome.as_ref())
            .filter(|outcome| !outcome.sent)
            .map(|outcome| outcome.message.as_str());
        let line = match notice {
            Some(why) => format!("Not sent: {why}"),
            None => format!("{count} of {DRAFT_MAX}"),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LIST_X + 4.0, FIELD_Y + 54.0, FIELD_WIDTH, 20.0),
            14.0 * s,
            if notice.is_some() {
                color::EMBER
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The chosen message's sender and what can be done: Mute here, and for staff
    /// Delete and Mute at the hub.
    fn chosen(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            SIDE_X,
            LIST_TOP - 20.0,
            SIDE_WIDTH,
            "Chosen message",
        );
        let message = self.selected.and_then(|id| {
            inputs
                .chat
                .and_then(|chat| chat.messages.iter().find(|message| message.id == id))
        });
        let Some(message) = message else {
            self.shown.chosen = None;
            for (row, line) in [
                "Choose a message (Up from the field, or a click)",
                "to mute its sender on this PC.",
            ]
            .into_iter()
            .enumerate()
            {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{line}"),
                    frame.rect(
                        SIDE_X,
                        LIST_TOP + 10.0 + row as f32 * 24.0,
                        SIDE_WIDTH,
                        22.0,
                    ),
                    16.0 * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            return;
        };
        let muted = inputs.muted.contains(&message.key_id);
        self.shown.chosen = Some((message.id, message.key_id.clone(), muted));
        let name = for_display(&message.name);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(SIDE_X, LIST_TOP + 8.0, SIDE_WIDTH, 34.0),
            28.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Key id {}", message.key_id),
            frame.rect(SIDE_X, LIST_TOP + 46.0, SIDE_WIDTH, 22.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let mut y = LIST_TOP + 90.0;
        kit::button(
            &mut self.ui,
            frame,
            [SIDE_X, y, 240.0, 42.0],
            if muted {
                "Unmute on this PC"
            } else {
                "Mute on this PC"
            },
            false,
            true,
            self.focus == MUTE_TOKEN,
            MUTE_TOKEN,
        );
        self.order.push(MUTE_TOKEN);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Hides their messages here until you quit."),
            frame.rect(SIDE_X, y + 50.0, SIDE_WIDTH, 22.0),
            14.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        if !inputs.staff {
            return;
        }
        y += 104.0;
        kit::heading(&mut self.ui, frame, SIDE_X, y, SIDE_WIDTH, "Staff");
        y += 24.0;
        for (index, (label, token)) in [
            ("Delete for everyone", DELETE_TOKEN),
            ("Mute at the hub", HUB_MUTE_TOKEN),
            ("Unmute at the hub", HUB_UNMUTE_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            kit::button(
                &mut self.ui,
                frame,
                [SIDE_X, y + index as f32 * 54.0, 240.0, 42.0],
                label,
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("The hub logs what staff do."),
            frame.rect(SIDE_X, y + 3.0 * 54.0, SIDE_WIDTH, 22.0),
            14.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(SIDE_X - 30.0, LIST_TOP - 20.0, 1.0, LIST_BOTTOM - LIST_TOP),
            color: color::alpha(color::HOLO, 0.25),
        });
    }

    /// The keys of the page, bottom right.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let keys: [(&[&str], &str); 4] = [
            (&["Enter"], "send"),
            (&["Up", "Down"], "messages"),
            (&["PgUp", "PgDn"], "scroll"),
            (&["Esc"], "back"),
        ];
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(1_824.0, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{chat, inputs, message};
    use super::*;

    /// Every focus, long messages, a muted sender, a refusal and staff fit the canvas
    /// at 1080p, 4K, 4:3 and 21:9, in the families and in Inter.
    #[test]
    fn every_state_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let mut crowded = chat(120);
        for id in 121..140 {
            crowded.messages.push_back(message(
                id,
                "aaaaaaaaaaaaaaaa",
                &"long words make many rows here ".repeat(5),
            ));
        }
        crowded.outcome = Some(sjk_identity::ReportOutcome {
            serial: 1,
            sent: false,
            message: "x".repeat(200),
        });
        let muted = ["aaaaaaaaaaaaaaaa".to_owned()];
        let empty = sjk_identity::ChatState::default();
        for (state, staff) in [(&crowded, true), (&empty, false)] {
            let mut panel = Panel::new();
            panel.open(true);
            panel.draft = "d".repeat(150);
            panel.selected = Some(130);
            let inputs = inputs(state, &muted, staff);
            panel.build(&inputs, &inter.font, [1920.0, 1080.0]);
            for focus in panel.order.clone() {
                for font in [&body.font, &inter.font] {
                    for viewport in [
                        [1920.0, 1080.0],
                        [3840.0, 2160.0],
                        [1440.0, 1080.0],
                        [2560.0, 1080.0],
                    ] {
                        panel.focus = focus;
                        panel.build(&inputs, font, viewport);
                        assert!(!panel.ui.overflowed(), "{focus} at {viewport:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_status_says_why_the_chat_is_silent() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut panel = Panel::new();
        panel.open(true);
        let state = chat(0);
        let mut off = inputs(&state, &[], false);
        off.chat = None;
        panel.build(&off, &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("The SJK identity is off"))
        );
        off.enabled = false;
        panel.build(&off, &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("SJK chat is off"))
        );
        panel.build(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run == "9 SJK players reading")
        );
    }
}
