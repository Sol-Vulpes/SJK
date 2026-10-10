//! The SJK chat page's drawing, in the SJK UI's look: the messages down the left,
//! newest at the bottom over the field and Send, and the chosen message with what
//! can be done about it on the right. A message flows as one line, as SJK chat does
//! everywhere ([`crate::sjk_chat_look`]): the JoF emblem for the clan's tag
//! ([`crate::jof_tag`]), the name, the verified tick for a verified
//! sender, then the text in the SJK chat's gold, wrapping only when it is too long. Over
//! the chosen message, a small window lists who is online and who was seen last
//! ([`super::people`]). A name under the pointer, in either, shows that player's sender
//! card over the page.

use super::*;
use crate::menu::sjk::recent::ago;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar};
use crate::menu_widgets::TextFamily;
use crate::sjk_chat_look::{self, Measure};
use crate::text::TextFace;
use sjk_identity::ChatMessage;
use sjk_identity::chat::name_for_display;
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
/// The who-is-online window: its box, its two parts' headings and first rows, and a
/// player's row (frame pixels).
const PEOPLE_BOX: [f32; 4] = [SIDE_X - 18.0, 204.0, SIDE_WIDTH + 36.0, 318.0];
const ONLINE_Y: f32 = 230.0;
const RECENT_Y: f32 = 394.0;
const PERSON_ROW: f32 = 24.0;
const PERSON_SIZE: f32 = 17.0;
/// The room on a player's row for what is said of them ("playing", "3 minutes ago").
const PERSON_META: f32 = 150.0;
/// The chosen message's heading, under the window.
const CHOSEN_TOP: f32 = 556.0;

/// A name under the pointer: whose card it shows, who, and the name's rectangle.
type Shows = (CardSource, crate::sender_card::Person, sjk_ui::Rect);

/// A message's height on the page, its text in `rows` rows.
fn height(rows: usize) -> f32 {
    NAME_ROW + rows.saturating_sub(1) as f32 * BODY_ROW + GAP
}

/// A message laid out on the page, in frame pixels.
struct Laid {
    /// The name as shown, with its colour codes.
    name: String,
    /// The room the JoF emblem takes before the name ([`crate::jof_tag`]), or 0.
    jof: f32,
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
    /// The message is a holocron drop of this tier (or of one this client does not
    /// know, `None` with a text that says so): the tier's gem stands where the tick
    /// does, there is no colon, and the text is in the tier's colour.
    tier: Option<&'static crate::holocrons::Tier>,
    drop: bool,
}

/// Lay `message` out in the page's column, measured as `measure` draws at the frame
/// scale `s`.
fn lay(message: &ChatMessage, inputs: &Inputs<'_>, measure: &Measure<'_>, s: f32) -> Laid {
    let s = s.max(0.001);
    let width = |value: &str, size: f32, face| measure.width(value, size * s, face) / s;
    let name = name_for_display(&message.name);
    let name = if name.is_empty() {
        "(no name)".to_owned()
    } else {
        name
    };
    let jof = if crate::jof_tag::tagged(&message.name) {
        crate::jof_tag::room(crate::jof_tag::side(NAME_SIZE))
    } else {
        0.0
    };
    let name_width = width(&name, NAME_SIZE, TextFace::Semibold).min(LIST_WIDTH * 0.4);
    // A holocron drop reads `Sol <gem> found a Rare Holocron.` on the page: the name, the
    // tier's gem for the tick, no colon.
    let drop = message.holocron.is_some();
    let words = crate::holocrons::line::words(message);
    let tick = if message.verified || drop {
        sjk_chat_look::tick_room(NAME_SIZE)
    } else {
        0.0
    };
    let colon = if drop {
        0.0
    } else {
        width(": ", TEXT_SIZE, TextFace::Regular)
    };
    let indent = jof + name_width + tick + colon;
    let staff = if message.staff { "Staff  ·  " } else { "" };
    let when = ago(u64::try_from(message.at).unwrap_or(0), inputs.now);
    let meta = format!("{staff}{when}");
    let meta_room = width(&meta, META_SIZE, TextFace::Regular) + 24.0;
    let muted = inputs.muted.contains(&message.key_id);
    let text = if muted {
        "Muted on this PC".to_owned()
    } else if let Some(words) = &words {
        words.rest.clone()
    } else if drop {
        "found a holocron.".to_owned()
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
        jof,
        name_width,
        verified: message.verified && !drop,
        indent,
        meta,
        staff: message.staff,
        text,
        rows,
        muted,
        tier: words.map(|words| words.tier),
        drop,
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
        let named = self.messages(&frame, inputs, measure);
        let listed = self.people(&frame, inputs, measure);
        self.follow_card(named.or(listed));
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

    /// The messages that fit, newest at the bottom, `scroll` messages up from it; returns
    /// the sender whose name is under the pointer, for the card.
    fn messages(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        measure: &Measure<'_>,
    ) -> Option<Shows> {
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
            let name = frame.rect(LIST_X + laid.jof, y, laid.name_width + 2.0, NAME_ROW - 2.0);
            self.ui.hit_region(NAME_BASE + index as u16, name);
            if self.ui.token_hovered(NAME_BASE + index as u16) {
                let person = crate::sender_card::Person {
                    name: laid.name.clone(),
                    key_id: Some(message.key_id.clone()),
                    hub_name: None,
                    verified: message.verified,
                    staff: message.staff,
                    medals: crate::medals::Medals::default(),
                    place: crate::sender_card::Place::Unknown,
                    avatar: None,
                };
                hovered = Some((CardSource::Message(message.id), person, name));
            }
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
        hovered
    }

    /// Draw a laid out message whose first row's top is `y`: the JoF emblem, the
    /// name, the tick, the
    /// colon and the text going on after them, then its other rows from the left
    /// edge, and what is said of it on the right of the first row.
    fn message(&mut self, frame: &Frame, laid: &Laid, y: f32) {
        let s = frame.s;
        let first = frame.rect(LIST_X, y, LIST_WIDTH, NAME_ROW - 2.0);
        if laid.jof > 0.0 {
            crate::jof_tag::draw(
                first.x,
                first.y + first.height * 0.5,
                crate::jof_tag::side(NAME_SIZE * s),
                1.0,
                |command| {
                    let _ = self.ui.draw_list_mut().push(command);
                },
            );
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", laid.name),
            frame.rect(LIST_X + laid.jof, y, laid.name_width + 2.0, NAME_ROW - 2.0),
            NAME_SIZE * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut x = LIST_X + laid.jof + laid.name_width;
        if let Some(tier) = laid.tier {
            let [left, _] = frame.point(x, y);
            sjk_chat_look::gem_mark(
                self.ui.draw_list_mut(),
                left,
                first.y + first.height * 0.5,
                NAME_SIZE * s,
                tier,
                1.0,
            );
            x += sjk_chat_look::tick_room(NAME_SIZE);
        } else if laid.verified {
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
        if !laid.drop {
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
        }
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
            sjk_chat_look::text_colour(laid.tier)
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

    /// The who-is-online window over the chosen message: who reads the chat now
    /// (playing first) and who was seen most recently, or, from a hub that does not
    /// list them, the count and the chat's last senders. Returns the player whose name
    /// is under the pointer, for the card.
    fn people(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        measure: &Measure<'_>,
    ) -> Option<Shows> {
        let s = frame.s;
        self.shown.people.clear();
        let [x, y, width, tall] = PEOPLE_BOX;
        let rect = frame.rect(x, y, width, tall);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 14.0 * s,
            color: color::alpha(color::HOLO, 0.05),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 14.0 * s,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.25),
        });
        let chat = inputs.chat.filter(|chat| inputs.enabled && chat.live);
        let listed = chat.map_or_else(Default::default, |chat| {
            super::people::derive(chat, inputs.muted)
        });
        let online = match chat {
            Some(_) => format!("Online  \u{b7}  {}", listed.count),
            None => "Online".to_owned(),
        };
        kit::heading(&mut self.ui, frame, SIDE_X, ONLINE_Y, SIDE_WIDTH, &online);
        let mut hovered = None;
        let mut row_y = ONLINE_Y + 18.0;
        let note = match chat {
            None => Some("Shows once the SJK hub answers."),
            Some(_) if !listed.listed => Some("This hub counts them but does not say who."),
            Some(_) if listed.online.is_empty() => Some("Nobody else."),
            Some(_) => None,
        };
        if let Some(note) = note {
            self.quiet_row(frame, row_y, note);
        }
        for row in &listed.online {
            hovered = self
                .person(frame, row, row_y, true, inputs.now, measure)
                .or(hovered);
            row_y += PERSON_ROW;
        }
        if listed.more > 0 {
            self.quiet_row(frame, row_y, &format!("and {} more", listed.more));
        }
        let recent = if chat.is_some() && !listed.listed {
            "Recently in chat"
        } else {
            "Recently active"
        };
        kit::heading(&mut self.ui, frame, SIDE_X, RECENT_Y, SIDE_WIDTH, recent);
        row_y = RECENT_Y + 18.0;
        if chat.is_some() && listed.recent.is_empty() {
            self.quiet_row(frame, row_y, "Nobody yet.");
        }
        for row in &listed.recent {
            hovered = self
                .person(frame, row, row_y, false, inputs.now, measure)
                .or(hovered);
            row_y += PERSON_ROW;
        }
        hovered
    }

    /// A quiet line in the who-is-online window, on the row whose top is `y`.
    fn quiet_row(&mut self, frame: &Frame, y: f32, line: &str) {
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(SIDE_X + 18.0, y, SIDE_WIDTH - 18.0, PERSON_ROW - 2.0),
            15.0 * frame.s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// A player's row in the who-is-online window, its top at `y`: a dot (filled for
    /// someone `online`, gold while they play), the name cut to its room, the tick for
    /// a verified player and, on the right, "playing" or how long ago they were seen.
    /// Returns them when the pointer is on their name.
    fn person(
        &mut self,
        frame: &Frame,
        row: &super::people::Row,
        y: f32,
        online: bool,
        now: u64,
        measure: &Measure<'_>,
    ) -> Option<Shows> {
        let s = frame.s;
        let token = PERSON_BASE + self.shown.people.len() as u16;
        self.shown.people.push(row.key_id.clone());
        let middle = y + (PERSON_ROW - 2.0) * 0.5;
        let dot = frame.rect(SIDE_X + 3.0, middle - 4.0, 8.0, 8.0);
        let _ = self.ui.draw_list_mut().push(if online {
            DrawCommand::RoundedRect {
                rect: dot,
                radius: 4.0 * s,
                color: if row.playing {
                    color::GOLD_BRIGHT
                } else {
                    color::HOLO
                },
            }
        } else {
            DrawCommand::Border {
                rect: dot,
                radius: 4.0 * s,
                width: 1.0 * s,
                color: color::alpha(color::QUIET, 0.8),
            }
        });
        let full = name_for_display(&row.name);
        let full = if full.is_empty() {
            "(no name)".to_owned()
        } else {
            full
        };
        let x = SIDE_X + 18.0;
        let tick = if row.verified {
            sjk_chat_look::tick_room(PERSON_SIZE)
        } else {
            0.0
        };
        let room = SIDE_WIDTH - 18.0 - PERSON_META - tick;
        let scale = s.max(0.001);
        let width = |value: &str| measure.width(value, PERSON_SIZE * s, TextFace::Semibold) / scale;
        let shown = if width(&full) <= room {
            full.clone()
        } else {
            let fits = (room - width("...")).max(0.0);
            let cut = measure.fitting(&full, fits * s, PERSON_SIZE * s, TextFace::Semibold);
            format!("{}...", &full[..cut])
        };
        let name_width = width(&shown).min(room);
        let rect = frame.rect(x, y, name_width + 2.0, PERSON_ROW - 2.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{shown}"),
            rect,
            PERSON_SIZE * s,
            if row.muted { color::QUIET } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if row.verified {
            sjk_chat_look::tick(
                self.ui.draw_list_mut(),
                frame.point(x + name_width, 0.0)[0],
                rect.y + rect.height * 0.5,
                PERSON_SIZE * s,
                1.0,
            );
        }
        if online && row.playing {
            self.meta(frame, y, format_args!("playing"), color::GOLD_BRIGHT);
        } else if !online {
            self.meta(
                frame,
                y,
                format_args!("{}", ago(row.seen, now)),
                color::QUIET,
            );
        }
        self.ui.hit_region(token, rect);
        self.ui.token_hovered(token).then(|| {
            let person = crate::sender_card::Person {
                name: full,
                key_id: Some(row.key_id.clone()),
                hub_name: None,
                verified: row.verified,
                staff: row.staff,
                medals: crate::medals::Medals::default(),
                place: crate::sender_card::Place::Unknown,
                avatar: row.avatar.clone(),
            };
            (CardSource::Person(row.key_id.clone()), person, rect)
        })
    }

    /// What is said of a player in the who-is-online window, on the right of their row.
    fn meta(
        &mut self,
        frame: &Frame,
        y: f32,
        line: std::fmt::Arguments<'_>,
        colour: sjk_ui::Color,
    ) {
        text(
            &mut self.ui,
            TextFamily::Body,
            line,
            frame.rect(
                SIDE_X + SIDE_WIDTH - PERSON_META,
                y + 1.0,
                PERSON_META,
                PERSON_ROW - 4.0,
            ),
            14.0 * frame.s,
            colour,
            FontWeight::Regular,
            TextAlign::End,
        );
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
            CHOSEN_TOP,
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
                        CHOSEN_TOP + 30.0 + row as f32 * 24.0,
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
        let name = name_for_display(&message.name);
        self.shown.chosen = Some((message.id, message.key_id.clone(), name.clone(), muted));
        let size = 26.0;
        let room = SIDE_WIDTH - sjk_chat_look::tick_room(size);
        let width = (measure.width(&name, size * s, TextFace::Semibold) / s.max(0.001)).min(room);
        let rect = frame.rect(SIDE_X, CHOSEN_TOP + 28.0, width + 2.0, 34.0);
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
        // The player's own key stays off screen, on their own messages too.
        if let Some(key) = crate::player_identity::printable_key_id(&message.key_id) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Key id {key}"),
                frame.rect(SIDE_X, CHOSEN_TOP + 66.0, SIDE_WIDTH, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = CHOSEN_TOP + 110.0;
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

    /// Show the sender card of the player whose name is under the pointer
    /// (`hovered`), keep it while the pointer is on the card, else hide it.
    fn follow_card(&mut self, hovered: Option<Shows>) {
        match hovered {
            Some((source, person, anchor)) => match &mut self.card {
                Some(card) if card.source == source => card.anchor = anchor,
                _ => {
                    self.card = Some(Hovered {
                        source,
                        person,
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

    /// The sender card on show, beside its name and over the page; its targets last.
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
        let size = crate::sender_card::size(&card.person, s);
        // A player of the who-is-online window has their card left of the window, over
        // the chat's quiet top, rather than over the window's other rows (text draws
        // over every shape, so a card over them would show them through it).
        let origin = match card.source {
            CardSource::Person(_) => [
                (frame.point(PEOPLE_BOX[0], 0.0)[0] - 12.0 * s - size[0]).max(0.0),
                card.anchor.y.min(viewport[1] - size[1]).max(0.0),
            ],
            CardSource::Message(_) => {
                crate::sender_card::beside(card.anchor, size, viewport, 12.0 * s)
            }
        };
        crate::sender_card::draw(
            &mut self.ui,
            &crate::sender_card::Card {
                person: &card.person,
                muted,
                measure: Some(measure),
            },
            origin,
            s,
            crate::sender_card::Tokens {
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

    /// The verified ticks in the messages' column (at 1920 by 1080), the who-is-online
    /// window's left out.
    fn ticks(panel: &Panel) -> usize {
        panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command, DrawCommand::TexturedQuad { texture, rect, .. }
                    if *texture == crate::ui_renderer::VERIFIED_TEXTURE && rect.x < SIDE_X - 30.0)
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

    /// A holocron drop reads `name gem sentence` in its tier's colour: no colon, no tick.
    #[test]
    fn a_holocron_drop_is_the_name_a_gem_and_the_sentence_in_the_tiers_colour() {
        use crate::holocrons::TIERS;
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = chat(0);
        let mut drop = message(1, "aaaaaaaaaaaaaaaa", "");
        drop.name = "^2Sol".to_owned();
        drop.verified = true;
        drop.holocron = Some(sjk_identity::DropMark {
            tier: "legendary".to_owned(),
            own: false,
        });
        let mut future = message(2, "bbbbbbbbbbbbbbbb", "");
        future.holocron = Some(sjk_identity::DropMark {
            tier: "from_the_future".to_owned(),
            own: false,
        });
        state.messages.extend([drop, future]);
        let mut panel = Panel::new();
        panel.open(true);
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        let texts = texts(&panel);
        let (_, name, _) = found(&texts, "^2Sol");
        let (_, body, colour) = found(&texts, "found a Legendary Holocron!");
        assert_eq!(*colour, TIERS[2].colour);
        assert!((name.y - body.y).abs() < 4.0 && body.x > name.right());
        assert!(!texts.iter().any(|(text, ..)| text == ":"), "{texts:?}");
        assert_eq!(ticks(&panel), 0, "the gem stands for the tick");
        // A tier this client does not know is still a line, worded plainly.
        found(&texts, "found a holocron.");
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

    fn listed_person(key: &str, name: &str, seen: i64, playing: bool) -> sjk_identity::Person {
        sjk_identity::Person {
            key_id: key.to_owned(),
            name: name.to_owned(),
            seen,
            playing,
            ..sjk_identity::Person::default()
        }
    }

    /// The window lists who is online (playing first, a last row for the rest) and who
    /// was seen last with how long ago, every name inside the window; a name under the
    /// pointer shows its card, and a click chooses that player's newest message.
    #[test]
    fn the_window_lists_who_is_online_and_who_was_seen_last() {
        use sjk_ui::{InputEvent, Vec2};
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = chat(3);
        let now = 1_791_400_000_i64;
        let second = format!("{:016x}", 2);
        let mut sol = listed_person(&second, "^2Sol", now, true);
        sol.verified = true;
        sol.avatar = "0123456789abcdef".to_owned();
        let long = format!("A{}", "MW".repeat(16));
        state.people = Some(sjk_identity::People {
            online: vec![
                sol,
                listed_person("aaaaaaaaaaaaaaaa", "Fox", now, false),
                listed_person("bbbbbbbbbbbbbbbb", &long, now, false),
                listed_person("cccccccccccccccc", "Kyle", now, false),
                listed_person("dddddddddddddddd", "Mara", now, false),
                listed_person("eeeeeeeeeeeeeeee", "Zed", now, false),
            ],
            recent: vec![
                listed_person("ffffffffffffffff", "Bastila", now - 180, false),
                listed_person("1111111111111111", "Jan", now - 7_200, false),
            ],
        });
        let mut panel = Panel::new();
        panel.open(true);
        let inputs = inputs(&state, &[], false);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Online  \u{b7}  9");
        found(&drawn, "Recently active");
        let (_, sol, _) = found(&drawn, "^2Sol");
        let (_, playing, colour) = found(&drawn, "playing");
        assert!((sol.y - playing.y).abs() < 4.0 && *colour == color::GOLD_BRIGHT);
        // Six online: four rows, then one for the other two.
        found(&drawn, "and 2 more");
        assert!(!drawn.iter().any(|(text, ..)| text == "Zed"), "{drawn:?}");
        let (_, cut, _) = drawn
            .iter()
            .find(|(text, ..)| text.starts_with("AMW") && text.ends_with("..."))
            .unwrap_or_else(|| panic!("the long name cut: {drawn:?}"));
        let (_, three, _) = found(&drawn, "3 minutes ago");
        found(&drawn, "2 hours ago");
        let [x, y, width, tall] = PEOPLE_BOX;
        let window = Rect::new(x, y, width, tall);
        for rect in [sol, cut, three] {
            assert!(
                rect.x >= window.x
                    && rect.right() <= window.right()
                    && rect.y >= window.y
                    && rect.bottom() <= window.bottom(),
                "{rect:?} in {window:?}"
            );
        }
        assert!(cut.right() <= SIDE_X + SIDE_WIDTH - PERSON_META + 1.0);
        // Hovering Sol's name: their card, with the picture the hub listed.
        let name = panel.ui.rect_for(PERSON_BASE).expect("Sol's name");
        let centre = Vec2::new(name.x + name.width * 0.5, name.y + name.height * 0.5);
        let _ = panel.handle_pointer(InputEvent::PointerMove(centre));
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let card = panel.card.as_ref().expect("the card");
        assert_eq!(card.source, CardSource::Person(second.clone()));
        assert_eq!(card.person.avatar.as_deref(), Some("0123456789abcdef"));
        assert!(card.person.verified);
        let target = panel.ui.rect_for(CARD_TOKEN).expect("the card's target");
        assert!(target.right() <= window.x, "left of the window: {target:?}");
        // A click chooses their newest message on show; someone who said nothing, none.
        assert_eq!(panel.activate(PERSON_BASE), PanelAction::None);
        assert_eq!(panel.selected, Some(2));
        panel.selected = None;
        let _ = panel.activate(PERSON_BASE + 1);
        assert_eq!(panel.selected, None);
        // It fits the canvas at every size, the chosen message's staff tools too.
        panel.selected = Some(3);
        let staff = super::super::tests::inputs(&state, &[], true);
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
        ] {
            panel.build_with(&staff, &fonts.font, viewport);
            assert!(!panel.ui.overflowed(), "{viewport:?}");
        }
        // The chosen message's tools stay under the window and above the keys.
        panel.build_with(&staff, &fonts.font, [1920.0, 1080.0]);
        let mute = panel.ui.rect_for(MUTE_TOKEN).expect("Mute");
        let unmute = panel
            .ui
            .rect_for(HUB_UNMUTE_TOKEN)
            .expect("Unmute at the hub");
        assert!(mute.y > window.bottom() && unmute.bottom() < KEYS_Y - 20.0);
    }

    /// A hub that does not list who is online: the count, and the last senders.
    #[test]
    fn without_a_list_the_window_shows_the_count_and_the_last_senders() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Online  \u{b7}  9");
        found(&drawn, "This hub counts them but does not say who.");
        found(&drawn, "Recently in chat");
        let rows: Vec<f32> = ["^3Player 3", "^2Player 2", "^1Player 1"]
            .iter()
            .map(|name| {
                drawn
                    .iter()
                    .filter(|(text, rect, _)| text == name && rect.x >= SIDE_X)
                    .map(|(_, rect, _)| rect.y)
                    .next()
                    .unwrap_or_else(|| panic!("{name} in the window: {drawn:?}"))
            })
            .collect();
        assert!(
            rows.windows(2).all(|pair| pair[0] < pair[1]),
            "newest first"
        );
        // Not reading: the window says why it is empty.
        let mut quiet = state.clone();
        quiet.live = false;
        panel.build_with(&inputs(&quiet, &[], false), &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Shows once the SJK hub answers.");
        found(&drawn, "Online");
    }
}
