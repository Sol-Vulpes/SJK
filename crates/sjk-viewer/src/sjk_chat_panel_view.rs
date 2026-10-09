//! The SJK chat page's drawing, in the SJK UI's look: the messages down the left,
//! newest at the bottom over the field and Send, and the chosen message with what
//! can be done about it on the right. A message flows as one line, as SJK chat does
//! everywhere ([`crate::sjk_chat_look`]): the name, the verified tick for a verified
//! sender, then the text in the SJK chat's gold, wrapping only when it is too long. A
//! name under the pointer shows its sender's profile card over the page.

use super::*;
use crate::menu::sjk::recent::ago;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar};
use crate::menu_widgets::TextFamily;
use crate::sjk_chat_look::{self, Measure};
use crate::text::TextFace;
use sjk_identity::ChatMessage;
use sjk_identity::chat::for_display;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};
use std::ops::Range;

/// The columns (frame pixels).
const LIST_X: f32 = 96.0;
const LIST_WIDTH: f32 = 1_180.0;
const SIDE_X: f32 = 1_340.0;
const SIDE_WIDTH: f32 = 484.0;
const TOP: f32 = 170.0;
/// The messages' area.
const LIST_TOP: f32 = 236.0;
const LIST_BOTTOM: f32 = 880.0;
/// A message: its first row (the name's), each row after it and the gap after it.
const NAME_ROW: f32 = 28.0;
const BODY_ROW: f32 = 24.0;
const GAP: f32 = 12.0;
/// The sizes of a message's name, its text and what is said of it (frame pixels).
const NAME_SIZE: f32 = 19.0;
const TEXT_SIZE: f32 = 17.0;
const META_SIZE: f32 = 15.0;
/// The field and Send.
const FIELD_Y: f32 = 900.0;
const FIELD_WIDTH: f32 = 1_040.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;

/// A message's height on the page, its text in `rows` rows.
fn height(rows: usize) -> f32 {
    NAME_ROW + rows.saturating_sub(1) as f32 * BODY_ROW + GAP
}

/// A message laid out on the page, in frame pixels.
struct Laid {
    /// The name as shown, with its colour codes.
    name: String,
    name_width: f32,
    verified: bool,
    /// Where the text starts on the first row, after the name, the tick and the colon.
    indent: f32,
    /// Staff, and how long ago it came, on the right of the first row.
    meta: String,
    staff: bool,
    /// The text (or that its sender is muted here) and its rows.
    text: String,
    rows: Vec<Range<usize>>,
    muted: bool,
}

/// Lay `message` out in the page's column, measured as `measure` draws at the frame
/// scale `s`.
fn lay(message: &ChatMessage, inputs: &Inputs<'_>, measure: &Measure<'_>, s: f32) -> Laid {
    let s = s.max(0.001);
    let width = |value: &str, size: f32, face| measure.width(value, size * s, face) / s;
    let name = for_display(&message.name);
    let name = if name.is_empty() {
        "(no name)".to_owned()
    } else {
        name
    };
    let name_width = width(&name, NAME_SIZE, TextFace::Semibold).min(LIST_WIDTH * 0.4);
    let tick = if message.verified {
        sjk_chat_look::tick_room(NAME_SIZE)
    } else {
        0.0
    };
    let indent = name_width + tick + width(": ", TEXT_SIZE, TextFace::Regular);
    let staff = if message.staff { "Staff  ·  " } else { "" };
    let when = ago(u64::try_from(message.at).unwrap_or(0), inputs.now);
    let meta = format!("{staff}{when}");
    let meta_room = width(&meta, META_SIZE, TextFace::Regular) + 24.0;
    let muted = inputs.muted.contains(&message.key_id);
    let text = if muted {
        "Muted on this PC".to_owned()
    } else {
        sjk_chat_look::message_text(&message.text)
    };
    let rows = sjk_chat_look::flow(
        &text,
        (LIST_WIDTH - indent - meta_room).max(0.0),
        LIST_WIDTH,
        |value, room| measure.fitting(value, room * s, TEXT_SIZE * s, TextFace::Regular),
    );
    Laid {
        name,
        name_width,
        verified: message.verified,
        indent,
        meta,
        staff: message.staff,
        text,
        rows,
        muted,
    }
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        let measure = target.body_measure();
        self.build(inputs, &measure, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out, its messages measured in the body family (`measure`).
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, measure: &Measure<'_>, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.order.clear();
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        top_bar(&mut self.ui, &frame, "Back", BACK_TOKEN, "SJK chat", None);
        self.status(&frame, inputs);
        self.messages(&frame, inputs, measure);
        self.composer(&frame, inputs);
        self.chosen(&frame, inputs, measure);
        self.keys(&frame);
        self.card(&frame, inputs, measure, viewport);
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
    fn messages(&mut self, frame: &Frame, inputs: &Inputs<'_>, measure: &Measure<'_>) {
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
        let mut laid = Vec::with_capacity(MESSAGES_SHOWN);
        while first > 0 && end - first < MESSAGES_SHOWN {
            let message = &messages[first - 1];
            let layout = lay(message, inputs, measure, s);
            let needed = height(layout.rows.len());
            if needed > room {
                break;
            }
            room -= needed;
            first -= 1;
            laid.push(layout);
        }
        laid.reverse();
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
        let mut hovered = None;
        for (index, (message, laid)) in messages.range(first..end).zip(&laid).enumerate() {
            let token = MESSAGE_BASE + index as u16;
            let tall = height(laid.rows.len());
            let row = [LIST_X - 18.0, y - 4.0, LIST_WIDTH + 18.0, tall - 4.0];
            if self.focus == token || self.selected == Some(message.id) {
                kit::band(&mut self.ui, frame, row);
            }
            self.message(frame, laid, y);
            self.ui
                .hit_region(token, frame.rect(row[0], row[1], row[2], row[3]));
            // The name over its row: resting the pointer on it shows the card.
            let name = frame.rect(LIST_X, y, laid.name_width + 2.0, NAME_ROW - 2.0);
            self.ui.hit_region(NAME_BASE + index as u16, name);
            if self.ui.token_hovered(NAME_BASE + index as u16) {
                hovered = Some((message, laid, name));
            }
            self.order.push(token);
            self.shown
                .messages
                .push((message.id, message.key_id.clone()));
            y += tall;
        }
        self.follow_card(hovered);
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

    /// Draw a laid out message whose first row's top is `y`: the name, the tick, the
    /// colon and the text going on after them, then its other rows from the left
    /// edge, and what is said of it on the right of the first row.
    fn message(&mut self, frame: &Frame, laid: &Laid, y: f32) {
        let s = frame.s;
        let first = frame.rect(LIST_X, y, LIST_WIDTH, NAME_ROW - 2.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", laid.name),
            frame.rect(LIST_X, y, laid.name_width + 2.0, NAME_ROW - 2.0),
            NAME_SIZE * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut x = LIST_X + laid.name_width;
        if laid.verified {
            let [left, _] = frame.point(x, y);
            sjk_chat_look::tick(
                self.ui.draw_list_mut(),
                left,
                first.y + first.height * 0.5,
                NAME_SIZE * s,
                1.0,
            );
            x += sjk_chat_look::tick_room(NAME_SIZE);
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(":"),
            frame.rect(x, y, 12.0, NAME_ROW - 2.0),
            TEXT_SIZE * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", laid.meta),
            frame.rect(LIST_X + LIST_WIDTH - 420.0, y + 2.0, 420.0, NAME_ROW - 6.0),
            META_SIZE * s,
            if laid.staff {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::End,
        );
        let colour = if laid.muted {
            color::QUIET
        } else {
            sjk_chat_look::GOLD
        };
        for (row, range) in laid.rows.iter().enumerate() {
            if range.is_empty() {
                continue;
            }
            let rect = if row == 0 {
                frame.rect(
                    LIST_X + laid.indent,
                    y,
                    (LIST_WIDTH - laid.indent).max(1.0),
                    NAME_ROW - 2.0,
                )
            } else {
                frame.rect(
                    LIST_X,
                    y + NAME_ROW + (row - 1) as f32 * BODY_ROW,
                    LIST_WIDTH,
                    BODY_ROW - 2.0,
                )
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", &laid.text[range.clone()]),
                rect,
                TEXT_SIZE * s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
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
    fn chosen(&mut self, frame: &Frame, inputs: &Inputs<'_>, measure: &Measure<'_>) {
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
        // The name in the body family, which the page measures, and the tick alone
        // after it for a verified sender.
        let name = for_display(&message.name);
        self.shown.chosen = Some((message.id, message.key_id.clone(), name.clone(), muted));
        let size = 26.0;
        let room = SIDE_WIDTH - sjk_chat_look::tick_room(size);
        let width = (measure.width(&name, size * s, TextFace::Semibold) / s.max(0.001)).min(room);
        let rect = frame.rect(SIDE_X, LIST_TOP + 8.0, width + 2.0, 34.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{name}"),
            rect,
            size * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if message.verified {
            sjk_chat_look::tick(
                self.ui.draw_list_mut(),
                frame.point(SIDE_X + width, 0.0)[0],
                rect.y + rect.height * 0.5,
                size * s,
                1.0,
            );
        }
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
            format_args!("Hides their chat, model, sabers and sounds on this PC."),
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
        // What the page's last staff request came to, else what staff requests are.
        let answer = inputs
            .staff_state
            .zip(self.staff_after)
            .and_then(|(state, after)| {
                if state.serial > after {
                    Some(if state.failed {
                        (format!("Not done: {}", state.message), color::EMBER)
                    } else {
                        (format!("Done: {}", state.message), color::GOLD_BRIGHT)
                    })
                } else {
                    state
                        .busy
                        .then(|| ("Sending to the hub...".to_owned(), color::MUTED))
                }
            });
        let (line, colour) =
            answer.unwrap_or_else(|| ("The hub logs what staff do.".to_owned(), color::QUIET));
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(SIDE_X, y + 3.0 * 54.0, SIDE_WIDTH, 22.0),
            14.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(SIDE_X - 30.0, LIST_TOP - 20.0, 1.0, LIST_BOTTOM - LIST_TOP),
            color: color::alpha(color::HOLO, 0.25),
        });
    }

    /// Show the profile card of the sender whose name is under the pointer
    /// (`hovered`), keep it while the pointer is on the card, else hide it.
    fn follow_card(&mut self, hovered: Option<(&ChatMessage, &Laid, sjk_ui::Rect)>) {
        match hovered {
            Some((message, laid, anchor)) => match &mut self.card {
                Some(card) if card.id == message.id => card.anchor = anchor,
                _ => {
                    self.card = Some(Hovered {
                        id: message.id,
                        person: crate::profile_card::Person {
                            name: laid.name.clone(),
                            key_id: Some(message.key_id.clone()),
                            hub_name: None,
                            verified: message.verified,
                            staff: message.staff,
                            medals: crate::medals::Medals::default(),
                            place: crate::profile_card::Place::Unknown,
                        },
                        anchor,
                        placed: false,
                    });
                }
            },
            None if self.ui.token_hovered(CARD_TOKEN) || self.ui.token_hovered(CARD_MUTE_TOKEN) => {
            }
            None => self.card = None,
        }
    }

    /// The profile card on show, beside its name and over the page; its targets last.
    fn card(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        measure: &Measure<'_>,
        viewport: [f32; 2],
    ) {
        let Some(card) = &self.card else {
            self.shown.card_muted = false;
            return;
        };
        let muted = card
            .person
            .key_id
            .as_ref()
            .is_some_and(|key| inputs.muted.contains(key));
        self.shown.card_muted = muted;
        let s = frame.s;
        let size = crate::profile_card::size(&card.person, s);
        let origin = crate::profile_card::beside(card.anchor, size, viewport, 12.0 * s);
        crate::profile_card::draw(
            &mut self.ui,
            &crate::profile_card::Card {
                person: &card.person,
                muted,
                measure: Some(measure),
            },
            origin,
            s,
            crate::profile_card::Tokens {
                card: CARD_TOKEN,
                mute: CARD_MUTE_TOKEN,
            },
        );
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
    use crate::text::{TextStyle, UiFont};
    use sjk_ui::{Color, Rect};

    impl Panel {
        /// [`Panel::build`] measuring in `font` without a text style.
        pub(in crate::console) fn build_with(
            &mut self,
            inputs: &Inputs<'_>,
            font: &UiFont,
            viewport: [f32; 2],
        ) {
            self.build(inputs, &Measure::new(font, TextStyle::NEUTRAL), viewport);
        }
    }

    /// Every text the page drew, with its rectangle and colour.
    fn texts(panel: &Panel) -> Vec<(String, Rect, Color)> {
        panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    rect, text, color, ..
                } => Some((panel.ui.stored_text(*text).to_owned(), *rect, *color)),
                _ => None,
            })
            .collect()
    }

    fn ticks(panel: &Panel) -> usize {
        panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command, DrawCommand::TexturedQuad { texture, .. }
                    if *texture == crate::ui_renderer::VERIFIED_TEXTURE)
            })
            .count()
    }

    fn found<'a>(texts: &'a [(String, Rect, Color)], text: &str) -> &'a (String, Rect, Color) {
        texts
            .iter()
            .find(|(drawn, ..)| drawn == text)
            .unwrap_or_else(|| panic!("{text:?} not in {texts:?}"))
    }

    #[test]
    fn a_message_flows_after_its_name_in_gold() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = chat(0);
        let mut short = message(1, "aaaaaaaaaaaaaaaa", "gg ^1all");
        short.name = "^2Sol".to_owned();
        short.verified = true;
        // As long as the hub takes, in wide letters: more than the column's row.
        let long = message(2, "bbbbbbbbbbbbbbbb", &"WMWMWMWM ".repeat(17)[..150]);
        state.messages.extend([short, long.clone()]);
        let mut panel = Panel::new();
        panel.open(true);
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        let texts = texts(&panel);
        let (_, name, _) = found(&texts, "^2Sol");
        let (_, body, colour) = found(&texts, "gg all");
        // Centred on the same line (the name's text is larger): the name's row.
        assert!((name.y - body.y).abs() < 4.0, "on the name's row");
        assert!(body.x > name.right());
        assert_eq!(*colour, sjk_chat_look::GOLD);
        // The name keeps its codes and is not gold.
        assert_ne!(found(&texts, "^2Sol").2, sjk_chat_look::GOLD);
        // The verified sender has the tick, and no word says so.
        assert_eq!(ticks(&panel), 1);
        assert!(
            texts
                .iter()
                .all(|(text, ..)| !text.to_ascii_lowercase().contains("verified")),
            "{texts:?}"
        );
        // The long one wraps: its rows after the first start at the column's edge.
        let (_, long_name, _) = found(&texts, &long.name);
        let rows: Vec<_> = texts
            .iter()
            .filter(|(_, rect, colour)| *colour == sjk_chat_look::GOLD && rect.y >= long_name.y)
            .collect();
        assert!(rows.len() >= 2, "{rows:?}");
        assert!((rows[0].1.y - long_name.y).abs() < 4.0);
        assert!(rows[1].1.y > rows[0].1.y + 10.0);
        assert!(rows[1].1.x <= long_name.x + 0.5);
    }

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
            panel.build_with(&inputs, &inter.font, [1920.0, 1080.0]);
            for focus in panel.order.clone() {
                for font in [&body.font, &inter.font] {
                    for viewport in [
                        [1920.0, 1080.0],
                        [3840.0, 2160.0],
                        [1440.0, 1080.0],
                        [2560.0, 1080.0],
                    ] {
                        panel.focus = focus;
                        panel.build_with(&inputs, font, viewport);
                        assert!(!panel.ui.overflowed(), "{focus} at {viewport:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_staff_request_says_what_it_came_to() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(3);
        let runs =
            |panel: &Panel| -> Vec<String> { panel.ui.text_runs().map(str::to_owned).collect() };
        // Nothing sent from the page: no line, whatever the staff state holds.
        let old = StaffShown {
            serial: 4,
            message: "Gave bug_hunter",
            failed: false,
            busy: false,
        };
        let quiet = Inputs {
            staff_state: Some(old),
            ..inputs(&state, &[], true)
        };
        panel.build_with(&quiet, &fonts.font, [1920.0, 1080.0]);
        assert!(!runs(&panel).iter().any(|run| run == "Gave bug_hunter"));
        // Sent: waiting, then the hub's answer.
        panel.staff_sent(4);
        let waiting = StaffShown { busy: true, ..old };
        panel.build_with(
            &Inputs {
                staff_state: Some(waiting),
                ..inputs(&state, &[], true)
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(
            runs(&panel)
                .iter()
                .any(|run| run == "Sending to the hub...")
        );
        let refused = StaffShown {
            serial: 5,
            message: "you sent as many staff requests as you may for now; try again later",
            failed: true,
            busy: false,
        };
        panel.build_with(
            &Inputs {
                staff_state: Some(refused),
                ..inputs(&state, &[], true)
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(
            runs(&panel)
                .iter()
                .any(|run| run.starts_with("Not done: you sent as many staff")),
            "{:?}",
            runs(&panel)
        );
    }

    #[test]
    fn the_status_says_why_the_chat_is_silent() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut panel = Panel::new();
        panel.open(true);
        let state = chat(0);
        let mut off = inputs(&state, &[], false);
        off.chat = None;
        panel.build_with(&off, &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("The SJK identity is off"))
        );
        off.enabled = false;
        panel.build_with(&off, &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("SJK chat is off"))
        );
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run == "9 SJK players reading")
        );
    }
}
