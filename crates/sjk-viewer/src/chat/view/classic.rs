//! The classic chat (`cg_chatStyle classic`, `chat/style.rs`): JoF EJK's chat box and
//! message field, drawn from its source.
//!
//! The box is `CG_ChatBox_DrawStrings` (JoF EJK `cgame/cg_draw.c`): each message as
//! the server wrote it, name and all, in the small game font over retail's drop shadow,
//! its rows a line feed apart, the messages stacked up from `cg_chatBoxHeight` with
//! nothing between them, shown whole until `cg_chatBox` milliseconds passed and then
//! gone. No slide, no fade, no emblem, tick, label or sender card, and an open field
//! changes nothing in it.
//!
//! An SJK chat message is drawn as JoF EJK draws one (`CG_SJKChatBoxPaint`): the icon of
//! the client it came from, the sender's name in its own colours with a green `+` for a
//! verified sender, the label (`SJK`, `ALL/SJK`, a clan chat's) and the text in SJK's
//! golds, the rest of a long message lined up after the icon.
//!
//! The field is `Con_DrawNotify`'s chat line (`client/cl_console.cpp`) as JoF EJK's
//! `beta` draws it: at the top left under the notify lines, in the console's
//! characters over their drop shadow, the channels (`All Team SJK:`) with the one in
//! use lit, Tab going round them, a whisper's `Whisper to <name>:` on a line of its own
//! above what is typed, and a cursor that blinks.

use super::*;
use crate::chat::style::classic;
use crate::console::edit_view::scroll_start;

/// Base colour of the box's text (`colorWhite`).
fn white(alpha: f32) -> Color {
    Color::new(1.0, 1.0, 1.0, alpha)
}

fn rgb([r, g, b]: [f32; 3], alpha: f32) -> Color {
    Color::new(r, g, b, alpha)
}

/// What an SJK chat message shows before its text in the classic box.
struct HubLead<'a> {
    /// The client's icon, for a message a client sent.
    icon: Option<sjk_ui::TextureId>,
    name: &'a str,
    verified: bool,
    label: &'static str,
    label_colour: Color,
    text_colour: Color,
}

impl HubLead<'_> {
    /// The room the icon takes before every row, in pixels.
    fn icon_room(&self, g: &Geometry) -> f32 {
        if self.icon.is_some() {
            g.font * (classic::ICON + classic::ICON_GAP) / classic::LINE
        } else {
            0.0
        }
    }

    /// The widths of the name with its `+` and the space after it, of the label, and of
    /// the `: ` before the text.
    fn widths(&self, font: &UiFont, g: &Geometry) -> [f32; 3] {
        let scale = g.font / font.height.max(1.0);
        let width = |text: &str| visible_text_width_face(font, text, scale, TextFace::Regular);
        let plus = if self.verified { width("+") } else { 0.0 };
        [
            width(self.name) + plus + width(" "),
            width(self.label),
            width(": "),
        ]
    }
}

/// How the classic box leads `line`: an SJK chat message's parts, else `None` (a game
/// line, drawn after [`ChatLine::lead`]).
fn hub_lead(line: &ChatLine, alpha: f32) -> Option<HubLead<'_>> {
    let hub = line.hub.as_ref()?;
    let clan = line.channel.clan();
    let text_colour = match (hub.tier, clan) {
        (Some(tier), _) => tier.colour_alpha(alpha),
        (None, Some(clan)) => clan_colour(clan, alpha),
        (None, None) => rgb(classic::SJK_TEXT, alpha),
    };
    Some(HubLead {
        // A drop and the SJK team's lines are the hub's news: no client sent them.
        icon: (hub.tier.is_none() && !hub.official)
            .then(|| crate::chat::client_icons::texture(hub.client)),
        name: &line.name,
        verified: hub.verified,
        label: match hub.tier {
            _ if hub.official => "",
            Some(_) => "",
            None if hub.in_all => "ALL/SJK",
            None => crate::chat::tabs::label(line.channel),
        },
        label_colour: clan.map_or(rgb(classic::SJK_LABEL, alpha), |clan| {
            clan_colour(clan, alpha)
        }),
        text_colour,
    })
}

impl ChatOverlay {
    /// The classic chat box in geometry `g` ([`options::Options::geometry`]).
    pub(in crate::chat) fn build_classic_feed(
        &mut self,
        font: &UiFont,
        g: &Geometry,
        viewport: [f32; 2],
        ms: u64,
    ) {
        self.follow_gif_switch();
        self.links.clear();
        // `cg_chatBoxShowHistory` with the console open: expired lines too.
        let history = self.options.history;
        let limit = self.options.lines;
        let unit = classic::unit(viewport[1]);
        let item_row = g.item_row();
        // JoF EJK measures a row at size 1 whatever `cg_chatBoxFontSize` is
        // (`CG_Text_Width(s, 0.65f, FONT_SMALL)`), so a row holds the same letters at
        // every size.
        let wrap_size = g.font / self.options.font.max(f32::EPSILON);
        let mut visible = [(0_usize, 0.0_f32, None); MAX_VISIBLE];
        let mut count = 0;
        let mut bottom = g.bottom;
        for index in (0..self.lines.len()).rev() {
            let line = &self.lines[index];
            if !self.shown(line) || line.muted {
                continue;
            }
            let age = ms.saturating_sub(line.received_ms);
            if !history && age >= self.options.lifetime {
                continue;
            }
            let lead = hub_lead(line, 1.0);
            let (room, indent) = match &lead {
                Some(lead) => {
                    let [name, label, colon] = lead.widths(font, g);
                    // Measured at size 1 as the rows are.
                    let at_one = wrap_size / g.font;
                    (
                        g.width - lead.icon_room(g) * at_one,
                        (name + label + colon) * at_one,
                    )
                }
                None => (
                    g.width,
                    visible_text_width_face(
                        font,
                        &line.lead,
                        wrap_size / font.height.max(1.0),
                        TextFace::Regular,
                    ),
                ),
            };
            let gif = self.gif_shown(index);
            let line = &mut self.lines[index];
            line.wrap
                .update_spaced(&line.body, font, room.max(1.0), wrap_size, indent, 0.0);
            let text_height = line.wrap.len as f32 * item_row;
            let mut gif = gif;
            let mut top = bottom - text_height - feed_gif::room(g, gif);
            if top < g.top && count == 0 && gif.is_some() {
                gif = None;
                top = bottom - text_height;
            }
            if top < g.top || count == limit {
                break;
            }
            visible[count] = (index, top, gif);
            count += 1;
            bottom = top;
        }
        // The text's top under the row's y: retail puts the baseline
        // `classic::BASELINE` under it, SJK Chat's is `classic::ASCENDER` under its top.
        let drop = g.font * (classic::BASELINE - classic::ASCENDER) / classic::LINE;
        let row_height = g.font + unit * 2.0;
        for &(index, top, gif) in visible[..count].iter().rev() {
            let line = &self.lines[index];
            let x = g.left;
            let y = top + drop;
            let rows = &line.wrap.rows[..line.wrap.len];
            let emojis = self.options.emojis.then_some(&self.emojis);
            let (text_x, first_x, base, lead_carry) = match hub_lead(line, 1.0) {
                Some(lead) => {
                    let text_x = x + lead.icon_room(g);
                    if let Some(texture) = lead.icon {
                        let side = g.font * classic::ICON / classic::LINE;
                        let icon_y = top
                            + g.font * classic::ICON_DROP.0 / classic::LINE
                            + classic::ICON_DROP.1 * unit;
                        let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                            rect: Rect::new(x, icon_y, side, side),
                            texture,
                            color: white(1.0),
                        });
                    }
                    let [name, label, colon] = lead.widths(font, g);
                    self.ui.text_fmt_aligned(
                        format_args!("{}{}", lead.name, if lead.verified { "^2+" } else { "" }),
                        Rect::new(text_x, y, name + g.font, row_height),
                        g.font,
                        white(1.0),
                        FontWeight::Regular,
                        0.0,
                        TextAlign::Start,
                    );
                    if !lead.label.is_empty() {
                        self.ui.text(
                            lead.label,
                            Rect::new(text_x + name, y, label + g.font, row_height),
                            g.font,
                            lead.label_colour,
                            FontWeight::Regular,
                            0.0,
                        );
                    }
                    self.ui.text(
                        ": ",
                        Rect::new(text_x + name + label, y, colon + g.font, row_height),
                        g.font,
                        white(1.0),
                        FontWeight::Regular,
                        0.0,
                    );
                    (
                        text_x,
                        text_x + name + label + colon,
                        lead.text_colour,
                        Carry::NONE,
                    )
                }
                None => {
                    let width = visible_text_width_face(
                        font,
                        &line.lead,
                        g.font / font.height.max(1.0),
                        TextFace::Regular,
                    );
                    if !line.lead.is_empty() {
                        self.ui.text(
                            &line.lead,
                            Rect::new(x, y, width + g.font, row_height),
                            g.font,
                            white(1.0),
                            FontWeight::Regular,
                            0.0,
                        );
                    }
                    let base = if crate::chat::translate::is_translation(line) {
                        rgb(crate::chat::translate::FEED_GREY, 1.0)
                    } else if line.encrypted {
                        // A tell's magenta (`^6`), which the game puts before one.
                        Color::new(1.0, 0.0, 1.0, 1.0)
                    } else {
                        white(1.0)
                    };
                    // The body goes on in the colour the prefix ended in.
                    (x, x + width, base, Carry::NONE.after(&line.lead))
                }
            };
            for (row, range) in rows.iter().enumerate() {
                let left = if row == 0 { first_x } else { text_x };
                let carry = line.wrap.carry[row];
                body_row(
                    &mut self.ui,
                    BodyRow {
                        forced: line.incoming_colour(self.options.incoming),
                        text: &line.body[range.clone()],
                        carry: if carry == Carry::NONE {
                            lead_carry
                        } else {
                            carry
                        },
                        marks: &line.emojis,
                        emojis,
                        truncated: row + 1 == rows.len() && range.end < line.body.len(),
                        spacing: 0.0,
                    },
                    font,
                    Rect::new(
                        left,
                        y + row as f32 * g.row,
                        (viewport[0] - left).max(1.0),
                        row_height,
                    ),
                    g.font,
                    base,
                );
            }
            if let Some(shown) = gif {
                crate::chat_gifs::draw::paint(
                    &mut self.ui,
                    feed_gif::LOOK,
                    shown,
                    [text_x, top + rows.len() as f32 * item_row],
                    g.width,
                    feed_gif::scale(g),
                    1.0,
                );
            }
        }
    }

    /// The channels the classic field's prompt names, in the row's order: what Tab goes
    /// round, a whisper left out (it has its own prompt).
    fn classic_channels(&self) -> impl Iterator<Item = Channel> + '_ {
        self.offered()
            .filter(|channel| *channel != Channel::Whisper)
    }

    /// The classic message field, its first line's top at `top` pixels (under the
    /// notify lines), in `font` (the console's characters).
    pub(in crate::chat) fn build_classic_field(
        &mut self,
        font: &UiFont,
        viewport: [f32; 2],
        top: f32,
        ms: u64,
    ) {
        let Some(input) = &self.input else {
            return;
        };
        let unit = classic::unit(viewport[1]);
        let size = classic::FIELD_LINE * unit;
        let scale = size / font.height.max(1.0);
        let width = |text: &str| visible_text_width_face(font, text, scale, TextFace::Regular);
        let left = classic::FIELD_LEFT * unit;
        // Room for the drop shadow under the line.
        let line_box = size + classic::FIELD_LINE * 0.25 * unit;
        let channel = input.channel;
        let mut y = top;
        let field_x;
        if channel == Channel::Whisper {
            // `Whisper to <name>:` on its own line, the field under it a cell in.
            let name = input
                .recipient
                .and_then(|target| self.roster.display_name(target))
                .unwrap_or("?");
            let encrypted = input
                .recipient
                .is_some_and(|target| self.takes_encrypted(target.slot()));
            self.ui.text_fmt_aligned(
                format_args!(
                    "Whisper to {name}^7{}:",
                    if encrypted { " (encrypted)" } else { "" }
                ),
                Rect::new(left, y, (viewport[0] - left).max(1.0), line_box),
                size,
                white(1.0),
                FontWeight::Regular,
                0.0,
                TextAlign::Start,
            );
            y += classic::FIELD_LINE * unit;
            field_x = classic::FIELD_CELL * unit;
        } else {
            // `All Team SJK:`, the channel in use in its colour, the others grey.
            let mut x = left;
            let grey = Color::new(0.5, 0.5, 0.5, 1.0);
            let space = width(" ");
            let mut named = false;
            let mut words = [Channel::Global; crate::chat::tabs::ROW.len()];
            let mut count = 0;
            for offered in self.classic_channels() {
                words[count] = offered;
                count += 1;
            }
            // A channel opened by its key while its button is off still says where
            // the text goes.
            if !words[..count].contains(&channel) {
                words[count] = channel;
                count += 1;
            }
            for &word in &words[..count] {
                if named {
                    x += space;
                }
                named = true;
                let label = crate::chat::tabs::label(word);
                let wide = width(label);
                self.ui.text(
                    label,
                    Rect::new(x, y, wide + size, line_box),
                    size,
                    if word == channel {
                        classic_tint(word)
                    } else {
                        grey
                    },
                    FontWeight::Regular,
                    0.0,
                );
                x += wide;
            }
            self.ui.text(
                ":",
                Rect::new(x, y, width(":") + size, line_box),
                size,
                white(1.0),
                FontWeight::Regular,
                0.0,
            );
            field_x = x + width(":") + left;
        }
        // What is typed, scrolled so the cursor stays in view, short of the HUD's upper
        // right.
        // A world shot always shows the cursor.
        let cursor_shows = classic::cursor_shows(ms) || self.shown_for_shot();
        let Some(input) = &mut self.input else {
            return;
        };
        let cursor = input.edit.cursor(&input.text);
        let room = (viewport[0]
            - field_x
            - (classic::FIELD_RIGHT_CELLS * classic::FIELD_CELL * unit).min(viewport[0] * 0.25)
            - width("_"))
        .max(size);
        let advance = |key| font.glyph_of(TextFace::Regular, key).advance * scale;
        let start = scroll_start(&input.text, cursor, room, advance);
        let end = start + layout::fitting_end(&input.text[start..], font, room, size);
        // The pointer stays the game's: nothing here is clicked.
        input.layout.rect = Rect::new(field_x, y, room, size);
        input.layout.len = 0;
        if let Some(range) = input.edit.selection(&input.text) {
            let from = range.start.clamp(start, end);
            let to = range.end.clamp(start, end);
            if from < to {
                self.ui.accent_bar(
                    Rect::new(
                        field_x + width(&input.text[start..from]),
                        y,
                        width(&input.text[from..to]),
                        size,
                    ),
                    Color::new(1.0, 1.0, 1.0, 0.3),
                );
            }
        }
        self.ui.text(
            &input.text[start..end],
            Rect::new(field_x, y, room + size, line_box),
            size,
            white(1.0),
            FontWeight::Regular,
            0.0,
        );
        if cursor_shows {
            let caret = field_x + width(&input.text[start..cursor.clamp(start, end)]);
            self.ui.text(
                "_",
                Rect::new(caret, y, width("_") + size, line_box),
                size,
                white(1.0),
                FontWeight::Regular,
                0.0,
            );
        }
        // Why a message cannot go (a whisper's player left, a channel not there).
        if !self.notice.is_empty() {
            self.ui.text(
                self.notice,
                Rect::new(
                    left,
                    y + classic::FIELD_LINE * unit,
                    (viewport[0] - left).max(1.0),
                    line_box,
                ),
                size * 0.75,
                Color::new(1.0, 1.0, 0.0, 1.0),
                FontWeight::Regular,
                0.0,
            );
        }
    }
}

/// A channel's colour in the classic field's prompt: JoF EJK's for All, Team and SJK
/// (`Con_ChatChannels`: white, cyan, yellow), SJK's own for the chats JoF EJK has not.
fn classic_tint(channel: Channel) -> Color {
    match channel {
        Channel::Global => white(1.0),
        Channel::Team => Color::new(0.0, 1.0, 1.0, 1.0),
        Channel::Sjk => Color::new(1.0, 1.0, 0.0, 1.0),
        other => tint(other, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::style::Style;
    use sjk_client::ServerEventKind;
    use sjk_identity::ChatMessage;
    use winit::keyboard::{Key, KeyCode, NamedKey};

    const VIEWPORT: [f32; 2] = [1920.0, 1080.0];

    fn classic_chat() -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        chat.options.style = Style::Classic;
        // JoF EJK's size; SJK's own default is 0.85.
        chat.options.font = 1.0;
        chat
    }

    /// A roster with `names` in their slots, the player in slot 0.
    fn roster(chat: &mut ChatOverlay, names: &[(usize, &str)]) {
        let mut game = sjk_protocol::GameState::empty_local(0);
        for (slot, name) in names {
            game.replace_config_string(1_131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                .expect("a player");
        }
        chat.update_roster(&game);
    }

    /// Every text the frame draws with its rectangle and colour.
    fn texts(chat: &ChatOverlay) -> Vec<(String, Rect, Color)> {
        chat.ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    rect, text, color, ..
                } => Some((chat.ui.stored_text(*text).to_owned(), *rect, *color)),
                _ => None,
            })
            .collect()
    }

    fn build(chat: &mut ChatOverlay, ms: u64) -> Vec<(String, Rect, Color)> {
        chat.build(true, &crate::text::test_font(), VIEWPORT, ms);
        texts(chat)
    }

    fn say(chat: &mut ChatOverlay, kind: ServerEventKind, text: &str, sender: Option<u16>) {
        chat.receive(kind, text.to_owned(), sender, chat.clock);
    }

    fn press(chat: &mut ChatOverlay, key: KeyCode, named: NamedKey) -> ChatInputResult {
        chat.edit_key(key, &Key::Named(named), None)
    }

    fn type_text(chat: &mut ChatOverlay, text: &str) {
        for character in text.chars() {
            let typed = character.to_string();
            chat.edit_key(
                KeyCode::KeyA,
                &Key::Character(winit::keyboard::SmolStr::new(&typed)),
                Some(&typed),
            );
        }
    }

    /// The box's numbers against JoF EJK's defaults (`cgame/cg_xcvar.h:76-82` at `beta`
    /// 10b940314: `cg_chatBox` 10000, `cg_chatBoxFontSize` 1, `cg_chatBoxLines` 5,
    /// `cg_chatBoxHeight` 350, `cg_chatBoxX` 30, `cg_chatBoxCutOffLength` 350) and its
    /// drawing (`cg_draw.c:11116-11215`).
    #[test]
    fn the_box_sits_where_jof_ejks_does() {
        let mut options = options::Options {
            style: Style::Classic,
            font: 1.0,
            ..options::Options::default()
        };
        assert_eq!(
            (options.x, options.height, options.width),
            (30.0, 350.0, 350.0)
        );
        assert_eq!((options.lines, options.lifetime), (5, 10_000));
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
            let unit = viewport[1] / 480.0;
            let g = options.geometry(viewport);
            // x = cg_chatBoxX * widthRatioCoef of 640: 30 units of a 480th of the height.
            assert_eq!(g.left, 30.0 * unit, "{viewport:?}");
            // y = cg_chatBoxHeight, the box's bottom.
            assert_eq!(g.bottom, 350.0 * unit);
            // The cut-off is in 640ths of the width, as text widths are.
            assert_eq!(g.width, 350.0 / 640.0 * viewport[0]);
            // ocr_a at 0.65: a 21-unit line box, rows 18 * 0.65 apart, 20 * 0.65 a row.
            assert!((g.font - 21.0 * 0.65 * unit).abs() < 1e-3);
            assert!((g.row - 18.0 * 0.65 * unit).abs() < 1e-3);
            assert!((g.item_row() - 20.0 * 0.65 * unit).abs() < 1e-3);
            assert_eq!((g.gap, g.spacing), (0.0, 0.0));
        }
        // On a 4:3 frame the units are the 640x480 ones both ways.
        assert_eq!(options.geometry([640.0, 480.0]).left, 30.0);
        // cg_chatBoxFontSize scales the text and its rows, not the box's place.
        options.font = 2.0;
        let g = options.geometry(VIEWPORT);
        assert_eq!((g.left, g.bottom), (67.5, 787.5));
        assert!((g.item_row() - 58.5).abs() < 1e-3);
    }

    #[test]
    fn messages_stack_up_from_the_bottom_with_nothing_between_them() {
        let mut chat = classic_chat();
        roster(&mut chat, &[(3, "Fox"), (4, "Wolf")]);
        say(
            &mut chat,
            ServerEventKind::Chat,
            "^7Fox^7: ^2first",
            Some(3),
        );
        say(
            &mut chat,
            ServerEventKind::TeamChat,
            "(^7Wolf^7): ^5second",
            Some(4),
        );
        let drawn = build(&mut chat, 1_000);
        let unit = 2.25;
        // Two messages of one row each: the first row's y is 350 - 2 * 13.
        let place = |text: &str| {
            drawn
                .iter()
                .find(|(run, ..)| run == text)
                .unwrap_or_else(|| panic!("{text:?} in {drawn:?}"))
                .1
        };
        // The text's top is the baseline's 19 * 0.65 less SJK Chat's 17 * 0.65.
        let drop = 2.0 * 0.65 * unit;
        let first = place("^7Fox^7: ");
        let second = place("(^7Wolf^7): ");
        assert_eq!(first.x, 67.5);
        assert!((first.y - ((350.0 - 26.0) * unit + drop)).abs() < 1e-2);
        assert!((second.y - ((350.0 - 13.0) * unit + drop)).abs() < 1e-2);
        // The names are as the server wrote them: no label, no emblem, no tick.
        let runs: Vec<&str> = drawn.iter().map(|(run, ..)| run.as_str()).collect();
        assert_eq!(
            runs,
            ["^7Fox^7: ", "^7^2first", "(^7Wolf^7): ", "^7^5second"]
        );
        assert!(!chat.ui.draw_list().commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::TexturedQuad { .. } | DrawCommand::TexturedQuadUv { .. }
            )
        }));
        // Pure white under the codes.
        assert!(drawn.iter().all(|(.., colour)| *colour == white(1.0)));
    }

    #[test]
    fn a_long_message_wraps_at_the_cut_off_its_rows_a_line_feed_apart() {
        let mut chat = classic_chat();
        roster(&mut chat, &[(3, "Fox")]);
        let words = "the quick brown fox jumps over the lazy dog ".repeat(5);
        say(
            &mut chat,
            ServerEventKind::Chat,
            &format!("^7Fox^7: ^2{}", words.trim_end()),
            Some(3),
        );
        say(
            &mut chat,
            ServerEventKind::Chat,
            "^7Fox^7: ^2after",
            Some(3),
        );
        let drawn = build(&mut chat, 1_000);
        let rows: Vec<&(String, Rect, Color)> = drawn
            .iter()
            .filter(|(run, ..)| run.contains("the") || run.contains("dog"))
            .collect();
        assert!(rows.len() >= 2, "{drawn:?}");
        let unit = 2.25;
        // Rows of one message are ocr_a's point size apart (18 * 0.65), under the
        // name's row; the rows after the first start at the box's left.
        assert!((rows[1].1.y - rows[0].1.y - 18.0 * 0.65 * unit).abs() < 1e-2);
        assert_eq!(rows[1].1.x, 67.5);
        assert!(rows[0].1.x > 67.5);
        // Every row but the last is green as the first (the carried code).
        assert!(rows.iter().all(|(run, ..)| run.contains("^2")));
        // The message takes 20 * 0.65 a row: the next one starts that far under.
        let after = drawn
            .iter()
            .rfind(|(run, ..)| run == "^7Fox^7: ")
            .expect("the second message")
            .1;
        let first = drawn[0].1;
        assert!((after.y - first.y - rows.len() as f32 * 13.0 * unit).abs() < 1e-2);
        // The same letters fit a row at any text size (JoF EJK measures at size 1).
        let at_one: Vec<String> = rows.iter().map(|(run, ..)| run.clone()).collect();
        chat.options.font = 1.5;
        for line in &mut chat.lines {
            line.y = None;
        }
        let larger = build(&mut chat, 1_000);
        let larger: Vec<String> = larger
            .into_iter()
            .map(|(run, ..)| run)
            .filter(|run| run.contains("the") || run.contains("dog"))
            .collect();
        assert_eq!(larger, at_one);
    }

    #[test]
    fn a_message_shows_whole_until_its_time_and_then_is_gone() {
        let mut chat = classic_chat();
        say(&mut chat, ServerEventKind::Chat, "^7Fox^7: ^2hello", None);
        for (ms, shown) in [(0, true), (5_000, true), (9_999, true), (10_000, false)] {
            let drawn = build(&mut chat, ms);
            assert_eq!(!drawn.is_empty(), shown, "{ms} ms");
            // No fade and no slide: full white at the box's left.
            assert!(
                drawn
                    .iter()
                    .all(|(_, rect, colour)| colour.a == 1.0 && rect.x == 67.5)
            );
        }
        // The open field does not bring an expired message back, as SJK's does.
        chat.open(false);
        let drawn = build(&mut chat, 20_000);
        assert!(!drawn.iter().any(|(run, ..)| run.contains("hello")));
    }

    fn hub_message(id: u64, name: &str, text: &str, verified: bool, client: &str) -> ChatMessage {
        ChatMessage {
            id,
            at: 0,
            key_id: String::new(),
            name: name.to_owned(),
            text: text.to_owned(),
            verified,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: client.to_owned(),
            holocron: None,
            channel: sjk_identity::HubChannel::Sjk,
            reactions: Default::default(),
            official: None,
        }
    }

    /// As JoF EJK's `CG_SJKChatBoxPaint` (`cg_draw.c:11008-11035`): icon, name with a
    /// green `+`, the label and the text in SJK's golds.
    #[test]
    fn an_sjk_chat_message_is_drawn_as_jof_ejk_draws_one() {
        let mut chat = classic_chat();
        let now = chat.clock;
        chat.push_sjk(
            &hub_message(7, "^1Vixen", "hello there", true, "ejk"),
            false,
            now,
        );
        let drawn = build(&mut chat, 1_000);
        let runs: Vec<&str> = drawn.iter().map(|(run, ..)| run.as_str()).collect();
        assert_eq!(runs, ["^1Vixen^2+", "SJK", ": ", "hello there"]);
        assert_eq!(drawn[1].2, rgb(classic::SJK_LABEL, 1.0));
        assert_eq!(drawn[3].2, rgb(classic::SJK_TEXT, 1.0));
        // The icon is 17 * 0.65 units square at the box's left, the text 3 * 0.65 after.
        let unit = 2.25;
        let icon = chat
            .ui
            .draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::TexturedQuad { rect, texture, .. } => Some((*rect, *texture)),
                _ => None,
            })
            .expect("the client's icon");
        assert_eq!(icon.1, crate::ui_renderer::EJK_TEXTURE);
        assert_eq!(icon.0.x, 67.5);
        assert!((icon.0.width - 17.0 * 0.65 * unit).abs() < 1e-2);
        assert!((drawn[0].1.x - (67.5 + 20.0 * 0.65 * unit)).abs() < 1e-2);
        // It stays in the box when the field opens on another channel.
        chat.open(true);
        assert!(build(&mut chat, 1_000).iter().any(|(run, ..)| run == "SJK"));
    }

    #[test]
    fn whispers_keep_the_games_brackets() {
        let mut chat = classic_chat();
        roster(&mut chat, &[(3, "Fox"), (4, "Wolf")]);
        say(
            &mut chat,
            ServerEventKind::Chat,
            "[^7Fox^7]: ^6psst",
            Some(3),
        );
        let now = chat.clock;
        assert!(chat.receive_encrypted(Some(4), (4, "Wolf"), "secret", false, now));
        let drawn = build(&mut chat, 1_000);
        let runs: Vec<&str> = drawn.iter().map(|(run, ..)| run.as_str()).collect();
        assert_eq!(runs, ["[^7Fox^7]: ", "^7^6psst", "[Wolf^7]: ", "^7secret"]);
        // An encrypted whisper is in a tell's magenta under its codes.
        assert_eq!(drawn[3].2, Color::new(1.0, 0.0, 1.0, 1.0));
    }

    /// The prompt as JoF EJK's `Con_ChatChannels` (`cl_console.cpp:985-992`).
    #[test]
    fn the_field_names_its_channels_and_lights_the_one_in_use() {
        let mut chat = classic_chat();
        chat.open(false);
        let grey = Color::new(0.5, 0.5, 0.5, 1.0);
        let prompt = |chat: &mut ChatOverlay| -> Vec<(String, Color)> {
            build(chat, 0)
                .into_iter()
                .map(|(run, _, colour)| (run, colour))
                .collect()
        };
        let drawn = prompt(&mut chat);
        assert_eq!(
            drawn,
            [
                ("All".to_owned(), white(1.0)),
                ("Team".to_owned(), grey),
                ("SJK".to_owned(), grey),
                (":".to_owned(), white(1.0)),
                (String::new(), white(1.0)),
                ("_".to_owned(), white(1.0)),
            ]
        );
        // Tab: All, Team, SJK, All, keeping what is typed.
        type_text(&mut chat, "hi");
        press(&mut chat, KeyCode::Tab, NamedKey::Tab);
        let drawn = prompt(&mut chat);
        assert_eq!(drawn[0].1, grey);
        assert_eq!(
            drawn[1],
            ("Team".to_owned(), Color::new(0.0, 1.0, 1.0, 1.0))
        );
        assert_eq!(drawn[4].0, "hi");
        press(&mut chat, KeyCode::Tab, NamedKey::Tab);
        assert_eq!(
            prompt(&mut chat)[2],
            ("SJK".to_owned(), Color::new(1.0, 1.0, 0.0, 1.0))
        );
        press(&mut chat, KeyCode::Tab, NamedKey::Tab);
        assert_eq!(prompt(&mut chat)[0], ("All".to_owned(), white(1.0)));
        // No SJK chrome: no hit regions, no placeholder, no button row's underline.
        assert!(chat.ui.rect_for(crate::chat::interaction::GLOBAL).is_none());
        assert!(
            !build(&mut chat, 0)
                .iter()
                .any(|(run, ..)| run.contains("Say something"))
        );
    }

    #[test]
    fn the_field_sits_at_the_top_left_under_the_notify_lines() {
        let mut chat = classic_chat();
        chat.open(false);
        type_text(&mut chat, "hello");
        let unit = 2.25;
        let drawn = build(&mut chat, 0);
        // 8 units in, at the frame's top, 16 units tall.
        assert_eq!((drawn[0].1.x, drawn[0].1.y), (8.0 * unit, 0.0));
        let size = chat
            .ui
            .draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { size, .. } => Some(*size),
                _ => None,
            })
            .expect("the prompt");
        assert_eq!(size, 16.0 * unit);
        // Three notify lines above it move it down.
        chat.set_notify_bottom(108.0);
        let drawn = build(&mut chat, 0);
        assert!(drawn.iter().all(|(_, rect, _)| rect.y == 108.0));
        // The cursor blinks 256 ms on, 256 off.
        assert!(build(&mut chat, 100).iter().any(|(run, ..)| run == "_"));
        assert!(!build(&mut chat, 300).iter().any(|(run, ..)| run == "_"));
    }

    #[test]
    fn a_whisper_is_named_on_its_own_line_above_the_field() {
        let mut chat = classic_chat();
        roster(&mut chat, &[(3, "^1Fox"), (4, "Wolf")]);
        chat.set_encrypted_slots(1 << 4);
        assert!(chat.whisper_to(Some(3)));
        type_text(&mut chat, "psst");
        let unit = 2.25;
        let drawn = build(&mut chat, 0);
        assert_eq!(drawn[0].0, "Whisper to ^1Fox^7:");
        assert_eq!((drawn[0].1.x, drawn[0].1.y), (8.0 * unit, 0.0));
        assert_eq!(drawn[1].0, "psst");
        assert_eq!((drawn[1].1.x, drawn[1].1.y), (16.0 * unit, 16.0 * unit));
        // No red note: the prompt says when the whisper is encrypted.
        assert!(!drawn.iter().any(|(run, ..)| run.contains("Not private")));
        chat.close_composer();
        assert!(chat.whisper_to(Some(4)));
        assert_eq!(build(&mut chat, 0)[0].0, "Whisper to Wolf^7 (encrypted):");
    }

    /// Where each channel's Enter goes from the classic field.
    #[test]
    fn enter_sends_each_channel_where_sjks_field_does() {
        let mut chat = classic_chat();
        roster(&mut chat, &[(3, "Fox")]);
        let enter = |chat: &mut ChatOverlay| press(chat, KeyCode::Enter, NamedKey::Enter);
        chat.open(false);
        type_text(&mut chat, "to all");
        assert!(matches!(enter(&mut chat), ChatInputResult::Submit(command)
            if command.starts_with("say ") && command.contains("to all")));
        chat.settle_submitted(true);
        assert!(!chat.is_typing());

        chat.open(true);
        type_text(&mut chat, "to team");
        assert!(matches!(enter(&mut chat), ChatInputResult::Submit(command)
            if command.starts_with("say_team ")));
        chat.settle_submitted(true);

        chat.open_sjk();
        type_text(&mut chat, "to the hub");
        assert!(matches!(enter(&mut chat), ChatInputResult::Sjk(text) if text == "to the hub"));

        assert!(chat.whisper_to(Some(3)));
        type_text(&mut chat, "to fox");
        assert!(matches!(enter(&mut chat), ChatInputResult::Submit(command)
            if command.starts_with("tell 3 ")));
        chat.settle_submitted(true);

        // Tab from All reaches Team then SJK, and a whisper goes back to All.
        chat.open(false);
        let channel = |chat: &ChatOverlay| chat.input.as_ref().expect("field open").channel;
        press(&mut chat, KeyCode::Tab, NamedKey::Tab);
        assert_eq!(channel(&chat), Channel::Team);
        press(&mut chat, KeyCode::Tab, NamedKey::Tab);
        assert_eq!(channel(&chat), Channel::Sjk);
        chat.close_composer();
        assert!(chat.whisper_to(Some(3)));
        press(&mut chat, KeyCode::Tab, NamedKey::Tab);
        assert_eq!(channel(&chat), Channel::Global);
        // Escape closes it and drops what was typed.
        type_text(&mut chat, "never mind");
        press(&mut chat, KeyCode::Escape, NamedKey::Escape);
        assert!(!chat.is_typing());
    }

    /// The repo's text-field rule in the classic field: Ctrl+A selects all (typing then
    /// replaces it, Backspace clears it), Backspace deletes and goes on while held (a
    /// repeat is another press), the arrows move the cursor.
    #[test]
    fn the_field_takes_the_text_field_keys() {
        let mut chat = classic_chat();
        chat.open(false);
        type_text(&mut chat, "hello world");
        let text = |chat: &ChatOverlay| chat.input.as_ref().expect("field open").text.clone();
        for _ in 0..3 {
            press(&mut chat, KeyCode::Backspace, NamedKey::Backspace);
        }
        assert_eq!(text(&chat), "hello wo");
        // Ctrl+A, then a typed letter replaces everything.
        chat.input
            .as_mut()
            .expect("field open")
            .edit
            .select_all("hello wo");
        assert!(
            build(&mut chat, 300)
                .iter()
                .any(|(run, ..)| run == "hello wo")
        );
        // The selection shows behind the text.
        assert!(chat.ui.draw_list().commands().iter().any(|command| {
            matches!(command, DrawCommand::SolidRect { color, .. } if color.a == 0.3)
        }));
        type_text(&mut chat, "x");
        assert_eq!(text(&chat), "x");
        // Ctrl+A then Backspace clears it.
        type_text(&mut chat, "yz");
        chat.input
            .as_mut()
            .expect("field open")
            .edit
            .select_all("xyz");
        press(&mut chat, KeyCode::Backspace, NamedKey::Backspace);
        assert_eq!(text(&chat), "");
        // With nothing typed the arrows do not change channel, as they do in SJK's.
        press(&mut chat, KeyCode::ArrowRight, NamedKey::ArrowRight);
        assert_eq!(
            chat.input.as_ref().expect("field open").channel,
            Channel::Global
        );
        // Left then a letter inserts before the end.
        type_text(&mut chat, "ac");
        press(&mut chat, KeyCode::ArrowLeft, NamedKey::ArrowLeft);
        type_text(&mut chat, "b");
        assert_eq!(text(&chat), "abc");
        // The same shortcuts as SJK's field (`ChatOverlay::edit_shortcut`), whatever
        // the style: nothing in it reads the style.
        assert!(chat.takes_keys_only());
    }

    #[test]
    fn the_pointer_stays_the_games_in_the_classic_field() {
        let mut chat = classic_chat();
        say(&mut chat, ServerEventKind::Chat, "^7Fox^7: ^2hello", None);
        chat.open(false);
        assert!(chat.takes_keys_only());
        build(&mut chat, 0);
        chat.pointer(sjk_ui::InputEvent::PointerMove(sjk_ui::Vec2::new(
            80.0, 760.0,
        )));
        assert!(chat.pointer_at.is_none());
        let mut sjk = ChatOverlay::new();
        sjk.open(false);
        assert!(!sjk.takes_keys_only());
        // Closed, neither takes anything.
        chat.close_composer();
        assert!(!chat.takes_keys_only());
    }

    /// The SJK style draws what it drew: the classic branch is not taken.
    #[test]
    fn the_sjk_style_keeps_its_field_and_its_feed() {
        let mut chat = ChatOverlay::new();
        roster(&mut chat, &[(3, "Fox")]);
        say(
            &mut chat,
            ServerEventKind::TeamChat,
            "(^7Fox^7): ^5hi",
            Some(3),
        );
        chat.open(false);
        let drawn = build(&mut chat, 1_000);
        let runs: Vec<&str> = drawn.iter().map(|(run, ..)| run.as_str()).collect();
        assert!(runs.contains(&"TEAM") && runs.contains(&"Say something..."));
        assert!(chat.ui.rect_for(crate::chat::interaction::GLOBAL).is_some());
    }
}
