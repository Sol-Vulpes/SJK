//! The Collection's Shaders tab: the saber's looks as a rack, a row to each, the stock
//! blade first in the player's colour, then every blade skin (`docs/unlockables.md`),
//! alive when owned, grey and still under a padlock when not. The one chosen is the
//! blade the player's model holds beside the page ([`Backstage`]), a locked one too, as
//! a preview only this screen draws; the words beside the model say what it is, since
//! when it is the player's or how to get it, and Equip or Unequip. Body shaders, for the
//! whole character, are to come: their kind is shown, not yet offered.

use super::view::{LEAD_Y, LEFT_X, STAGE_TEXT_WIDTH, STAGE_TEXT_X, left_lines};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::unlockables::Unlockable;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The rack: where its first row starts, a row's room and a swatch's size.
const RACK_TOP: f32 = 324.0;
const ROW_STEP: f32 = 106.0;
const ROW_HEIGHT: f32 = 96.0;
const SWATCH: [f32; 2] = [380.0, 84.0];
const RACK_WIDTH: f32 = 860.0;

/// The chroma mark's side beside a rack row's name and the detail's kind line, and the
/// room it keeps from the words after it (frame pixels).
const MARK: f32 = 24.0;
const MARK_GAP: f32 = 10.0;
/// The detail's state pill: its height, its least width and its gap from the name.
const TAG_HEIGHT: f32 = 26.0;
const TAG_MIN_WIDTH: f32 = 80.0;
const TAG_GAP: f32 = 18.0;

/// Where a rack row's words go (frame pixels, `[x, y, width, height]`), the row's top
/// at `y`: the chroma mark (a chroma only) and the name on the first line, the name
/// ending short of the row's right edge, and the state under them.
fn rack_words(y: f32, chroma: bool) -> ([f32; 4], [f32; 4], [f32; 4]) {
    let x = LEFT_X + SWATCH[0] + 34.0;
    let right = LEFT_X - 12.0 + RACK_WIDTH - 16.0;
    let mark = [x, y + 23.0, MARK, MARK];
    let name_x = if chroma { x + MARK + MARK_GAP } else { x };
    let name = [name_x, y + 18.0, right - name_x, 34.0];
    let status = [x, y + 56.0, right - x, 22.0];
    (mark, name, status)
}

/// The detail's name and state pill on the line at `y` of the column from `x`, `width`
/// wide: the pill at the column's right end, the name ellipsised short of it.
fn title_rects(x: f32, y: f32, width: f32, tag_width: f32) -> ([f32; 4], [f32; 4]) {
    let tag_width = tag_width.max(TAG_MIN_WIDTH);
    let tag = [x + width - tag_width, y + 18.0, tag_width, TAG_HEIGHT];
    let name = [x, y, (tag[0] - TAG_GAP - x).max(0.0), 62.0];
    (name, tag)
}

/// The first row the rack shows, from the one it showed first: moved just enough that
/// the chosen row is in view, never past either end.
fn rack_window(first: usize, chosen: usize, rows: usize) -> usize {
    let first = if chosen < first {
        chosen
    } else if chosen >= first + RACK_SHOWN {
        chosen + 1 - RACK_SHOWN
    } else {
        first
    };
    first.min(rows.saturating_sub(RACK_SHOWN))
}

/// The owned line of a shader the player holds: since when, and from the team or the
/// medal that brings it (a medal the client does not know reads as the team).
fn owned_line(grant: &sjk_identity::Unlock) -> String {
    let date = crate::medals::date_text(grant.granted);
    let from = grant
        .medal
        .as_deref()
        .and_then(crate::medals::Medal::from_id)
        .map_or_else(
            || "from the SJK team".to_owned(),
            |medal| format!("with your {} medal", medal.name()),
        );
    if date.is_empty() {
        format!("Yours, {from}")
    } else {
        format!("Yours since {date}, {from}")
    }
}

/// What a row shows: the stock blade, or a blade skin.
#[derive(Clone, Copy)]
struct Row {
    skin: Option<&'static Unlockable>,
    owned: bool,
    worn: bool,
}

impl Row {
    fn name(self) -> &'static str {
        self.skin.map_or("Stock blade", |skin| skin.name)
    }

    /// Its state, as the rack says it.
    fn status(self) -> &'static str {
        match (self.skin, self.worn, self.owned) {
            (None, true, _) => "Worn, in your colour",
            (None, false, _) => "In your colour",
            (Some(_), true, _) => "Worn",
            (Some(_), false, true) => "Yours",
            (Some(_), false, false) => "Locked",
        }
    }
}

impl Panel {
    /// The rows, and the lead's words.
    fn rows(inputs: &Inputs<'_>) -> Vec<Row> {
        let worn = inputs
            .holdings
            .worn_blade_skin(inputs.setting)
            .map(|skin| skin.id);
        std::iter::once(Row {
            skin: None,
            owned: true,
            worn: worn.is_none(),
        })
        .chain(unlockables::ALL.iter().map(|skin| Row {
            skin: Some(skin),
            owned: inputs.holdings.unlock(skin.id).is_some(),
            worn: worn == Some(skin.id),
        }))
        .collect()
    }

    /// The Shaders tab, swatches alive at `seconds`.
    pub(super) fn shaders(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        let rows = Self::rows(inputs);
        if self.shader >= rows.len() {
            self.shader = rows.iter().position(|row| row.worn).unwrap_or(0);
        }
        let owned = rows.iter().skip(1).filter(|row| row.owned).count();
        let headline = if inputs.holdings.reason().is_none() {
            format!("{owned} of {} owned", unlockables::ALL.len())
        } else {
            "Saber shaders".to_owned()
        };
        let line = match inputs.holdings {
            unlockables::Holdings::IdentityOff => {
                "Shaders are kept on the SJK hub: switch the identity on in Settings, Network."
            }
            unlockables::Holdings::NoHub => {
                "Shaders are kept on the SJK hub: no hub is set (cl_hubUrl)."
            }
            unlockables::Holdings::Waiting => "Waiting for the SJK hub to say which are yours.",
            unlockables::Holdings::Known(_) => {
                "How your blade looks and sounds, for every SJK player on your server."
            }
        };
        self.lead(frame, &headline, line);
        self.kinds(frame, owned);
        let unlocks = matches!(inputs.holdings, unlockables::Holdings::Known(_));
        // The rack scrolls just enough to keep the chosen row in view.
        self.shader_first = rack_window(self.shader_first, self.shader, rows.len());
        let first = self.shader_first;
        for (index, row) in rows.iter().enumerate() {
            let y = RACK_TOP + index.wrapping_sub(first) as f32 * ROW_STEP;
            self.shader_rows[index] = ShaderRow {
                wear: match row.skin {
                    // The stock blade, worn by taking the skin off.
                    None => (!row.worn).then_some(""),
                    Some(skin) if row.owned && unlocks => Some(if row.worn { "" } else { skin.id }),
                    Some(_) => None,
                },
                worn: row.worn,
            };
            if (first..first + RACK_SHOWN).contains(&index) {
                self.rack_row(frame, *row, index, y, inputs, seconds);
            }
        }
        self.rack_scroll_bar(frame, first, rows.len());
        if let Some(row) = rows.get(self.shader).copied() {
            self.shader_beside_model(frame, row, inputs, seconds);
        }
        let _ = s;
    }

    /// The rack's scroll bar at its right, when it holds more rows than it shows.
    fn rack_scroll_bar(&mut self, frame: &Frame, first: usize, rows: usize) {
        if rows <= RACK_SHOWN {
            return;
        }
        let s = frame.s;
        let x = LEFT_X - 12.0 + RACK_WIDTH + 8.0;
        let height = RACK_SHOWN as f32 * ROW_STEP - (ROW_STEP - ROW_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, RACK_TOP, 4.0, height),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let thumb = height * RACK_SHOWN as f32 / rows as f32;
        let top = RACK_TOP + (height - thumb) * first as f32 / (rows - RACK_SHOWN) as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top, 4.0, thumb),
            radius: 2.0 * s,
            color: color::GOLD,
        });
    }

    /// The kinds of shader along the lead: Saber, and Body to come.
    fn kinds(&mut self, frame: &Frame, owned: usize) {
        let s = frame.s;
        let x = LEFT_X + 560.0;
        let y = LEAD_Y - 2.0;
        let pills: [(&str, String, bool); 2] = [
            ("Saber", format!("{owned}/{}", unlockables::ALL.len()), true),
            ("Body", "soon".to_owned(), false),
        ];
        let mut left = x;
        for (name, count, lit) in pills {
            let width = 132.0;
            let rect = frame.rect(left, y, width, 36.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 18.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, 0.25)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 18.0, y + 5.0, 70.0, 26.0),
                19.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else {
                    color::QUIET
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{count}"),
                frame.rect(left + 70.0, y + 7.0, width - 84.0, 22.0),
                14.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Regular,
                TextAlign::End,
            );
            left += width + 12.0;
        }
    }

    /// Row `index` of the rack at `y`.
    fn rack_row(
        &mut self,
        frame: &Frame,
        row: Row,
        index: usize,
        y: f32,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let chosen = index == self.shader;
        let token = SHADER_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        if chosen || hovered {
            kit::band(
                &mut self.ui,
                frame,
                [LEFT_X - 12.0, y, RACK_WIDTH, ROW_HEIGHT],
            );
        }
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(LEFT_X - 12.0, y + 14.0, 3.0, ROW_HEIGHT - 28.0),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        let rect = [
            LEFT_X + 6.0,
            y + (ROW_HEIGHT - SWATCH[1]) * 0.5,
            SWATCH[0],
            SWATCH[1],
        ];
        match row.skin {
            None => swatch::small_stock(self.ui.draw_list_mut(), frame, rect, inputs.stock),
            Some(skin) => {
                let look = self.skins.get(skin.id);
                let _ = swatch::small_swatch(
                    self.ui.draw_list_mut(),
                    frame,
                    rect,
                    look,
                    row.owned,
                    seconds,
                    swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                );
            }
        }
        let chroma = row.skin.is_some_and(|skin| skin.chroma);
        let (mark, name, status) = rack_words(y, chroma);
        if chroma {
            swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                mark[0],
                mark[1],
                mark[2],
                if row.owned { 1.0 } else { 0.55 },
            );
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", row.name()),
            frame.rect(name[0], name[1], name[2], name[3]),
            28.0 * s,
            match (chosen, row.owned) {
                (true, _) => color::GOLD_BRIGHT,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", row.status()),
            frame.rect(status[0], status[1], status[2], status[3]),
            15.0 * s,
            if row.worn || (row.owned && row.skin.is_some()) {
                color::GOLD
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        self.ui
            .hit_region(token, frame.rect(LEFT_X - 12.0, y, RACK_WIDTH, ROW_HEIGHT));
    }

    /// What the chosen row is, beside the model: its kind, name and state, what it is,
    /// since when it is the player's (with the team's note) or how to get it, and Equip
    /// or Unequip. Without a model it shows the swatch, larger.
    fn shader_beside_model(&mut self, frame: &Frame, row: Row, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        let x = STAGE_TEXT_X;
        let width = STAGE_TEXT_WIDTH;
        if self.backstage == Backstage::None {
            let rect = [x, 300.0, width, 118.0];
            match row.skin {
                None => swatch::small_stock(self.ui.draw_list_mut(), frame, rect, inputs.stock),
                Some(skin) => {
                    let look = self.skins.get(skin.id);
                    let _ = swatch::small_swatch(
                        self.ui.draw_list_mut(),
                        frame,
                        rect,
                        look,
                        row.owned,
                        seconds,
                        swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                    );
                }
            }
        } else if !row.owned {
            let rect = frame.rect(x, 232.0, 360.0, 34.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 17.0 * s,
                color: color::alpha(color::SPACE, 0.6),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 17.0 * s,
                width: 1.2 * s,
                color: color::alpha(color::HOLO, 0.6),
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Preview on your saber: not yours yet"),
                frame.rect(x, 238.0, 360.0, 22.0),
                15.0 * s,
                color::HOLO,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        let mut y = 618.0;
        // A chroma says so first, with its mark: it takes the player's saber colour.
        let chroma = row.skin.is_some_and(|skin| skin.chroma);
        let kind_x = if chroma {
            swatch::chroma_mark(self.ui.draw_list_mut(), frame, x, y - 1.0, 22.0, 1.0);
            x + 22.0 + MARK_GAP
        } else {
            x
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}",
                match (row.skin.is_some(), chroma) {
                    (true, true) => "Chroma saber shader: takes your saber colour",
                    (true, false) => "Saber shader",
                    (false, _) => "Your saber",
                }
            ),
            frame.rect(kind_x, y, x + width - kind_x, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 26.0;
        let name = row.name();
        let lit = row.owned && row.skin.is_some();
        let tag = match (row.skin, row.worn, row.owned) {
            (None, _, _) => None,
            (Some(_), true, _) => Some("Worn"),
            (Some(_), false, true) => Some("Yours"),
            (Some(_), false, false) => Some("Locked"),
        };
        // The name never runs under the state: it ends short of the pill, ellipsised,
        // whatever face and size the text is drawn in.
        let (name_rect, tag_rect) = match tag {
            Some(tag) => {
                let (name, pill) =
                    title_rects(x, y, width, crate::text::display_width(tag, 15.0) + 26.0);
                (name, Some((tag, pill)))
            }
            None => ([x, y, width, 62.0], None),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(name_rect[0], name_rect[1], name_rect[2], name_rect[3]),
            56.0 * s,
            if lit { color::GOLD_BRIGHT } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some((tag, [tag_x, tag_y, tag_width, tag_height])) = tag_rect {
            let rect = frame.rect(tag_x, tag_y, tag_width, tag_height);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 13.0 * s,
                width: 1.2 * s,
                color: if lit { color::GOLD } else { color::QUIET },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{tag}"),
                frame.rect(tag_x, tag_y + 2.0, tag_width, 22.0),
                15.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        y += 72.0;
        let description = row.skin.map_or(
            "Your blade in the colour you pick on Profile, Saber.",
            |skin| skin.description,
        );
        y = left_lines(
            &mut self.ui,
            frame,
            description,
            [x, y, width],
            (52, 3),
            18.0,
            color::TEXT,
            TextAlign::Start,
        );
        y += 6.0;
        let grant = row.skin.and_then(|skin| inputs.holdings.unlock(skin.id));
        let (line, colour) = match (row.skin, grant) {
            (None, _) if row.worn => ("What you hold now: no shader on.".to_owned(), color::MUTED),
            (None, _) => ("Wearing it takes your shader off.".to_owned(), color::MUTED),
            (Some(_), Some(grant)) => (owned_line(grant), color::GOLD),
            (Some(skin), None) => (format!("How to get it: {}", skin.how_to_get), color::MUTED),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(x, y, width, 24.0),
            16.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 26.0;
        if let Some(note) = grant
            .map(|grant| grant.note.as_str())
            .filter(|note| !note.is_empty())
        {
            let quoted = format!("\u{201c}{note}\u{201d}");
            y = left_lines(
                &mut self.ui,
                frame,
                &quoted,
                [x, y, width],
                (56, 1),
                16.0,
                color::MUTED,
                TextAlign::Start,
            );
        }
        let wear = self.shader_rows.get(self.shader).and_then(|row| row.wear);
        if let Some(wear) = wear {
            let label = if wear.is_empty() && row.skin.is_some() {
                "Unequip"
            } else if row.skin.is_none() {
                "Wear the stock blade"
            } else {
                "Equip"
            };
            let button_width = if row.skin.is_none() { 250.0 } else { 160.0 };
            kit::button(
                &mut self.ui,
                frame,
                [x, (y + 10.0).min(946.0), button_width, 44.0],
                label,
                !(wear.is_empty() && row.skin.is_some()),
                true,
                false,
                WEAR_TOKEN,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::sjk::Frame;

    #[test]
    fn the_owned_line_names_the_medal_that_brings_a_shader() {
        let grant = |medal: Option<&str>| sjk_identity::Unlock {
            id: "saber_glitch".to_owned(),
            granted: 0,
            note: String::new(),
            medal: medal.map(str::to_owned),
        };
        assert_eq!(owned_line(&grant(None)), "Yours, from the SJK team");
        assert_eq!(
            owned_line(&grant(Some("bug_hunter"))),
            "Yours, with your Bug Hunter medal"
        );
        assert_eq!(
            owned_line(&grant(Some("from_the_future"))),
            "Yours, from the SJK team"
        );
        let dated = sjk_identity::Unlock {
            granted: 1_791_000_000,
            ..grant(Some("early_tester"))
        };
        assert!(owned_line(&dated).starts_with("Yours since "));
        assert!(owned_line(&dated).ends_with(", with your Early Tester medal"));
    }

    /// Whether two window rectangles share any area.
    fn overlap(a: sjk_ui::Rect, b: sjk_ui::Rect) -> bool {
        a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
    }

    #[test]
    fn names_never_run_under_the_state_or_the_chroma_mark() {
        let longest = unlockables::blade_skins()
            .map(|skin| skin.name)
            .max_by(|a, b| {
                crate::text::display_width(a, 56.0).total_cmp(&crate::text::display_width(b, 56.0))
            })
            .unwrap();
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
        ] {
            let frame = Frame::new(viewport);
            let window = |r: [f32; 4]| frame.rect(r[0], r[1], r[2], r[3]);
            for chroma in [false, true] {
                let (mark, name, status) = rack_words(RACK_TOP, chroma);
                let (mark, name, status) = (window(mark), window(name), window(status));
                assert!(!overlap(name, status), "{viewport:?}");
                if chroma {
                    assert!(
                        !overlap(mark, name) && !overlap(mark, status),
                        "{viewport:?}"
                    );
                }
                // Inside the rack, with room for the longest name at its size.
                let rack_right = frame.rect(LEFT_X - 12.0 + RACK_WIDTH, 0.0, 0.0, 0.0).x;
                assert!(name.x + name.width <= rack_right);
                assert!(name.width >= crate::text::display_width(longest, 28.0) * frame.s);
            }
            for tag in ["Worn", "Yours", "Locked"] {
                let (name, pill) = title_rects(
                    STAGE_TEXT_X,
                    644.0,
                    STAGE_TEXT_WIDTH,
                    crate::text::display_width(tag, 15.0) + 26.0,
                );
                let (name, pill) = (window(name), window(pill));
                assert!(!overlap(name, pill), "{viewport:?} {tag}");
                assert!(
                    pill.x + pill.width
                        <= window([STAGE_TEXT_X + STAGE_TEXT_WIDTH, 0.0, 0.0, 0.0]).x + 0.01
                );
                assert!(
                    name.width > 200.0 * frame.s,
                    "{viewport:?}: room for a name"
                );
            }
        }
    }

    #[test]
    fn the_rack_scrolls_only_to_keep_the_chosen_row_in_view() {
        let rows = SHADER_ROWS;
        assert!(rows > RACK_SHOWN, "more shaders than the rack shows");
        // Within the window the rack holds still (a pointer moving over the rows
        // chooses them without scrolling them away).
        assert_eq!(rack_window(0, RACK_SHOWN - 1, rows), 0);
        assert_eq!(rack_window(3, 4, rows), 3);
        // Past either end it moves just enough.
        assert_eq!(rack_window(0, RACK_SHOWN, rows), 1);
        assert_eq!(rack_window(5, 2, rows), 2);
        assert_eq!(rack_window(0, rows - 1, rows), rows - RACK_SHOWN);
        assert_eq!(rack_window(40, 0, rows), 0);
    }

    #[test]
    fn the_wheel_scrolls_the_rack_and_keeps_the_chosen_row_shown() {
        let mut panel = Panel::new();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        // As the first frame leaves it, the stock blade worn.
        panel.shader = 0;
        let wheel = |panel: &mut Panel, y: f32| {
            let _ = panel.handle_pointer(
                InputEvent::PointerWheel {
                    position: sjk_ui::Vec2::new(500.0, 500.0),
                    delta: sjk_ui::Vec2::new(0.0, y),
                },
                false,
            );
        };
        for _ in 0..3 {
            wheel(&mut panel, -1.0);
        }
        assert_eq!(panel.shader_first, 3);
        assert_eq!(panel.shader, 3, "the chosen row follows into view");
        for _ in 0..40 {
            wheel(&mut panel, -1.0);
        }
        assert_eq!(panel.shader_first, SHADER_ROWS - RACK_SHOWN);
        wheel(&mut panel, 1.0);
        assert_eq!(panel.shader_first, SHADER_ROWS - RACK_SHOWN - 1);
    }

    /// Every row and the words beside the model fit the canvas over the keys, for an
    /// owned and worn skin, a locked one and the stock blade, with and without a model,
    /// at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_rack_and_its_words_fit_over_the_keys() {
        let sun = [sjk_identity::Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "a long note from the team ".repeat(8),
            medal: None,
        }];
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            for backstage in [Backstage::None, Backstage::World { head: None }] {
                for row in 0..SHADER_ROWS {
                    let inputs = super::super::tests::inputs(
                        unlockables::Holdings::Known(&sun),
                        "saber_sun",
                    );
                    let mut panel = Panel::new();
                    panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
                    panel.set_backstage(backstage);
                    panel.shader = row;
                    panel.build(&inputs, &font, viewport);
                    assert!(!panel.ui.overflowed(), "{viewport:?}");
                    let frame = Frame::new(viewport);
                    let keys = frame.point(0.0, super::super::view::KEYS_Y)[1];
                    for command in panel.ui.draw_list().commands() {
                        if let DrawCommand::Text { rect, .. } = command {
                            assert!(rect.bottom() <= keys || rect.y >= keys, "{row} {rect:?}");
                            assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                        }
                    }
                    // The rows in view answer the pointer, above the keys; the rest
                    // are scrolled off.
                    let first = panel.shader_first;
                    assert!((first..first + RACK_SHOWN).contains(&row), "{row}");
                    for index in 0..SHADER_ROWS {
                        let area = panel.ui.rect_for(SHADER_BASE + index as u16);
                        let shown = (first..first + RACK_SHOWN).contains(&index);
                        assert_eq!(area.is_some(), shown, "{index}");
                        if let Some(area) = area {
                            assert!(area.bottom() <= keys, "{index} {viewport:?}");
                        }
                    }
                }
            }
        }
    }
}
