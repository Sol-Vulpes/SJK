//! The Unlockables page's drawing, in the SJK UI's look: how many the player owns (or
//! why that is not known) under the top bar (on the Profile screen its title and row
//! of tabs, the page moved down under them); down the left a card per unlockable, its
//! live swatch beside its kind, name, what it is, owned (date and the team's note) or
//! locked (how to get it), and Equip or Unequip, as many as fit ([`ROWS_SHOWN`]) with a
//! scroll bar and a line saying which show and how to see the others; on the right what
//! the player wears and how unlockables work.

use super::*;
use crate::menu::sjk::{
    Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar, wrap,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use crate::unlockables::{Kind, Unlockable};
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

#[path = "unlockables_swatch.rs"]
mod swatch;

/// The page's left edge and width, and where the header starts (frame pixels).
const LEFT_X: f32 = 96.0;
const WIDTH: f32 = 1_728.0;
const TOP: f32 = 170.0;
/// The cards: where they start, their size and the gap between them.
const CARDS_TOP: f32 = 300.0;
const CARD_WIDTH: f32 = 852.0;
const CARD_HEIGHT: f32 = 268.0;
const GAP: f32 = 24.0;
/// A card's swatch, from the card's corner, and its text column.
const SWATCH: [f32; 4] = [22.0, 22.0, 400.0, 224.0];
const TEXT_X: f32 = 452.0;
const TEXT_WIDTH: f32 = CARD_WIDTH - TEXT_X - 28.0;
/// The column on the right: what is worn and how unlockables work.
const SIDE_X: f32 = 1_012.0;
const SIDE_WIDTH: f32 = LEFT_X + WIDTH - SIDE_X;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;
/// Rows of cards that fit between the header and the keys; the others scroll.
pub(super) const ROWS_SHOWN: usize =
    ((KEYS_Y - 16.0 - CARDS_TOP + GAP) / (CARD_HEIGHT + GAP)) as usize;

/// How unlockables work, as the side column says it.
const ABOUT: [&str; 3] = [
    "Unlockables are looks your SJK identity owns at the SJK hub. For now the SJK team gives them; earning them with achievements and medals is planned.",
    "What you wear travels through the hub: every SJK player on your server sees your blade, its light and its sounds. Players on other clients see the stock blade.",
    "Equip one here, or type saberskin <id> in the console (saberskin alone lists them).",
];

/// How a card shows its unlockable.
#[derive(Clone, Copy)]
struct Showing<'a> {
    unlockable: &'static Unlockable,
    /// The profile's grant, when it lists it.
    grant: Option<&'a sjk_identity::Unlock>,
    /// What is known of the profile's unlocks: without a grant a card is locked only
    /// once it is read.
    holdings: Holdings<'a>,
    /// It is what the player wears.
    worn: bool,
    focused: bool,
}

/// What the player wears, in big, and a line under it.
fn wearing(inputs: &Inputs<'_>) -> (String, String) {
    match crate::unlockables::blade_skin(inputs.setting) {
        Some(skin) if inputs.holdings.unlock(skin.id).is_some() => (
            skin.name.to_owned(),
            "In your hand, on the Character page and for SJK players on your server.".to_owned(),
        ),
        Some(skin) if inputs.holdings.reason().is_none() => (
            "Stock blade".to_owned(),
            format!("The {} is chosen and shows once unlocked.", skin.name),
        ),
        Some(_) => (
            "Stock blade".to_owned(),
            "The blade skin chosen shows once the hub says it is yours.".to_owned(),
        ),
        None => (
            "Stock blade".to_owned(),
            "Equip a blade skin you own to wear it.".to_owned(),
        ),
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
        let seconds = self.epoch.elapsed().as_secs_f32();
        #[cfg(test)]
        let seconds = self.shot_seconds.unwrap_or(seconds);
        self.order.clear();
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        if self.hub {
            crate::profile_hub::header(
                &mut self.ui,
                &frame,
                &self.hub_header,
                BACK_TOKEN,
                crate::profile_hub::Tab::Collection,
            );
        } else {
            top_bar(
                &mut self.ui,
                &frame,
                "Back",
                BACK_TOKEN,
                "Unlockables",
                None,
            );
        }
        let keys = frame;
        // Under the Profile screen's row of tabs the page moves down.
        let frame = frame.shifted(0.0, self.shift());
        self.header(&frame, inputs);
        self.side(&frame, inputs);
        let worn = crate::unlockables::blade_skin(inputs.setting)
            .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
            .map(|skin| skin.id);
        let count = crate::unlockables::ALL.len();
        let rows = count.div_ceil(COLUMNS);
        // The chosen card always shows: the list scrolls to it.
        let row = usize::from(self.focus.saturating_sub(CARD_BASE)).min(count - 1) / COLUMNS;
        if row < self.first {
            self.first = row;
        } else if row >= self.first + ROWS_SHOWN {
            self.first = row + 1 - ROWS_SHOWN;
        }
        self.first = self.first.min(rows.saturating_sub(ROWS_SHOWN));
        for (index, unlockable) in crate::unlockables::ALL.iter().enumerate() {
            let token = CARD_BASE + index as u16;
            let grant = inputs.holdings.unlock(unlockable.id);
            let is_worn = worn == Some(unlockable.id);
            // What Enter does on it, shown or scrolled away.
            self.cards[index] = Card {
                wear: (grant.is_some() && unlockable.is_blade_skin()).then_some(if is_worn {
                    ""
                } else {
                    unlockable.id
                }),
            };
            self.order.push(token);
            let Some(shown_row) = (index / COLUMNS)
                .checked_sub(self.first)
                .filter(|shown_row| *shown_row < ROWS_SHOWN)
            else {
                continue;
            };
            let y = CARDS_TOP + shown_row as f32 * (CARD_HEIGHT + GAP);
            let showing = Showing {
                unlockable,
                grant,
                holdings: inputs.holdings,
                worn: is_worn,
                focused: self.focus == token,
            };
            self.card(&frame, index, [LEFT_X, y], showing, seconds);
        }
        if rows > ROWS_SHOWN {
            self.scrolled(&frame, rows);
        } else {
            self.more_to_come(&frame);
        }
        self.keys(&keys);
        if !self.order.contains(&self.focus) {
            self.focus = self.order.first().copied().unwrap_or(CARD_BASE);
        }
        self.ui.finish(self.focus);
    }

    /// How far the page is moved down: under the Profile screen's row of tabs.
    fn shift(&self) -> f32 {
        if self.hub {
            crate::profile_hub::SHIFT
        } else {
            0.0
        }
    }

    /// How many the player owns, or why that is not known.
    fn header(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let total = crate::unlockables::ALL.len();
        let (headline, line) = match inputs.holdings.reason() {
            None => {
                let owned = crate::unlockables::ALL
                    .iter()
                    .filter(|unlockable| inputs.holdings.unlock(unlockable.id).is_some())
                    .count();
                (
                    format!("{owned} of {total} owned"),
                    "Looks for your sabers, kept with your SJK identity.",
                )
            }
            Some(reason) => (
                match inputs.holdings {
                    Holdings::IdentityOff => "Identity is off".to_owned(),
                    Holdings::NoHub => "No hub is set".to_owned(),
                    _ => "Contacting the hub...".to_owned(),
                },
                reason,
            ),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(LEFT_X, TOP - 4.0, 900.0, 48.0),
            38.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X, TOP + 50.0, 1_200.0, 26.0),
            17.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(LEFT_X, TOP + 98.0, WIDTH, 1.0),
            color: color::alpha(color::HOLO, 0.18),
        });
    }

    /// What the player wears, then how unlockables work.
    fn side(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            SIDE_X,
            CARDS_TOP + 14.0,
            SIDE_WIDTH,
            "You wear",
        );
        let (worn, line) = wearing(inputs);
        let lit = inputs
            .holdings
            .unlock(crate::unlockables::blade_skin(inputs.setting).map_or("", |skin| skin.id))
            .is_some();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{worn}"),
            frame.rect(SIDE_X, CARDS_TOP + 40.0, SIDE_WIDTH, 44.0),
            36.0 * s,
            if lit { color::GOLD_BRIGHT } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(SIDE_X, CARDS_TOP + 90.0, SIDE_WIDTH, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let mut y = CARDS_TOP + 170.0;
        kit::heading(
            &mut self.ui,
            frame,
            SIDE_X,
            y,
            SIDE_WIDTH,
            "How unlockables work",
        );
        y += 30.0;
        for paragraph in ABOUT {
            for part in wrap(paragraph, 92) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(SIDE_X, y, SIDE_WIDTH, 26.0),
                    16.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 25.0;
            }
            y += 14.0;
        }
    }

    /// With more cards than fit: a scroll bar beside them and, under them, which show and
    /// how to see the others.
    fn scrolled(&mut self, frame: &Frame, rows: usize) {
        let s = frame.s;
        let count = crate::unlockables::ALL.len();
        let height = ROWS_SHOWN as f32 * (CARD_HEIGHT + GAP) - GAP;
        let bar_x = LEFT_X + CARD_WIDTH + 14.0;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(bar_x, CARDS_TOP, 5.0, height),
            radius: 2.5 * s,
            color: color::alpha(color::HOLO, 0.12),
        });
        let thumb = height * ROWS_SHOWN as f32 / rows as f32;
        let travel = (height - thumb) * self.first as f32 / (rows - ROWS_SHOWN) as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(bar_x, CARDS_TOP + travel, 5.0, thumb),
            radius: 2.5 * s,
            color: color::alpha(color::HOLO, 0.55),
        });
        let from = self.first * COLUMNS + 1;
        let to = ((self.first + ROWS_SHOWN) * COLUMNS).min(count);
        let (above, below) = (from - 1, count - to);
        let how = match (above, below) {
            (0, _) => format!("{below} more below (Down or the mouse wheel)"),
            (_, 0) => format!("{above} more above (Up or the mouse wheel)"),
            _ => format!("{above} above, {below} below (Up, Down or the mouse wheel)"),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Unlockables {from} to {to} of {count}: {how}."),
            frame.rect(LEFT_X + 4.0, CARDS_TOP + height + 18.0, CARD_WIDTH, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// Under the last card, while there is room: what is planned.
    fn more_to_come(&mut self, frame: &Frame) {
        let s = frame.s;
        let rows = crate::unlockables::ALL.len().div_ceil(COLUMNS);
        let y = CARDS_TOP + rows as f32 * (CARD_HEIGHT + GAP);
        let height = 104.0;
        if y + height > KEYS_Y - 16.0 - self.shift() {
            return;
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(LEFT_X, y, CARD_WIDTH, height),
            radius: 18.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("More to come"),
            frame.rect(LEFT_X + 28.0, y + 22.0, CARD_WIDTH - 56.0, 30.0),
            24.0 * s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "More blade skins, and looks for the holocron, trails and emotes, are planned."
            ),
            frame.rect(LEFT_X + 28.0, y + 58.0, CARD_WIDTH - 56.0, 24.0),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// Card `index` at `at` (frame pixels).
    fn card(
        &mut self,
        frame: &Frame,
        index: usize,
        at: [f32; 2],
        showing: Showing<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let [x, y] = at;
        let Showing {
            unlockable,
            grant,
            holdings,
            worn,
            focused,
        } = showing;
        let known = holdings.reason().is_none();
        let owned = grant.is_some();
        let token = CARD_BASE + index as u16;
        let rect = frame.rect(x, y, CARD_WIDTH, CARD_HEIGHT);
        let hovered = self.ui.token_hovered(token);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 18.0 * s,
            color: if owned {
                Color::new(0.07, 0.09, 0.16, 0.92)
            } else {
                color::alpha(color::SPACE, 0.66)
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 18.0 * s,
            width: if focused { 2.0 * s } else { 1.2 * s },
            color: if focused {
                Color::new(1.0, 1.0, 1.0, 0.7)
            } else if worn {
                color::alpha(color::GOLD, 0.75)
            } else if owned {
                color::alpha(color::GOLD, 0.4)
            } else if hovered {
                color::alpha(color::HOLO, 0.4)
            } else {
                color::alpha(color::HOLO, 0.18)
            },
        });
        // The card chooses it; the button over it acts, so it is registered after.
        self.ui.hit_region(token, rect);
        let [sx, sy, width, height] = SWATCH;
        let shown = match unlockable.kind {
            Kind::BladeSkin => swatch::blade(
                &mut self.ui,
                frame,
                [x + sx, y + sy, width, height],
                self.skins.get(unlockable.id),
                owned,
                seconds,
            ),
        };
        if shown == swatch::Shown::Waiting {
            // Its art is SJK's own, delivered by the hub, not part of this client.
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Its look downloads from the SJK hub"),
                frame.rect(x + sx + 16.0, y + sy + height - 40.0, width - 32.0, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        let tx = x + TEXT_X;
        let kind = match unlockable.kind {
            Kind::BladeSkin => "Blade skin",
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{kind}"),
            frame.rect(tx, y + 26.0, 200.0, 22.0),
            16.0 * s,
            color::alpha(color::HOLO, 0.9),
            FontWeight::Regular,
            TextAlign::Start,
        );
        let (tag, tag_colour) = match (owned, worn, known) {
            (true, true, _) => ("Worn", color::GOLD_BRIGHT),
            (true, false, _) => ("Owned", color::GOLD_BRIGHT),
            (false, _, true) => ("Locked", color::QUIET),
            (false, _, false) => ("", color::QUIET),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{tag}"),
            frame.rect(tx + TEXT_WIDTH - 160.0, y + 26.0, 160.0, 22.0),
            16.0 * s,
            tag_colour,
            FontWeight::Semibold,
            TextAlign::End,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", unlockable.name),
            frame.rect(tx, y + 50.0, TEXT_WIDTH, 40.0),
            34.0 * s,
            if owned {
                color::GOLD_BRIGHT
            } else {
                color::MUTED
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut line_y = y + 96.0;
        for part in wrap(unlockable.description, 50).take(2) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(tx, line_y, TEXT_WIDTH, 22.0),
                15.0 * s,
                if owned { color::MUTED } else { color::QUIET },
                FontWeight::Regular,
                TextAlign::Start,
            );
            line_y += 22.0;
        }
        line_y += 8.0;
        let button = [tx, y + CARD_HEIGHT - 66.0, 156.0, 44.0];
        match grant {
            Some(grant) => {
                let since = crate::medals::date_text(grant.granted);
                let given = if since.is_empty() {
                    "Yours, from the SJK team".to_owned()
                } else {
                    format!("Yours since {since}, from the SJK team")
                };
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{given}"),
                    frame.rect(tx, line_y, TEXT_WIDTH, 22.0),
                    15.0 * s,
                    color::GOLD_BRIGHT,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                line_y += 22.0;
                let note = grant.note.trim();
                if !note.is_empty() {
                    // One line, so the button keeps its place: the hub allows 200
                    // characters; a long note ends in an ellipsis.
                    let note = cut(note, 46);
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!("\u{201c}{note}\u{201d}"),
                        frame.rect(tx, line_y, TEXT_WIDTH, 22.0),
                        15.0 * s,
                        color::QUIET,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
                if unlockable.is_blade_skin() {
                    let equip = EQUIP_BASE + index as u16;
                    kit::button(
                        &mut self.ui,
                        frame,
                        button,
                        if worn { "Unequip" } else { "Equip" },
                        !worn,
                        true,
                        focused,
                        equip,
                    );
                }
            }
            None => {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("How to get it: {}", unlockable.how_to_get),
                    frame.rect(tx, line_y, TEXT_WIDTH, 22.0),
                    15.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                // A pill in place of Equip, inert: the card's own token.
                kit::button(
                    &mut self.ui,
                    frame,
                    button,
                    match holdings {
                        Holdings::Known(_) => "Locked",
                        Holdings::IdentityOff => "Needs identity",
                        Holdings::NoHub => "Needs a hub",
                        Holdings::Waiting => "Waiting",
                    },
                    false,
                    false,
                    false,
                    token,
                );
            }
        }
    }

    /// The keys, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let enter = match self
            .focus
            .checked_sub(CARD_BASE)
            .and_then(|index| self.cards.get(usize::from(index)))
            .and_then(|card| card.wear)
        {
            Some("") => "unequip",
            Some(_) => "equip",
            None => "",
        };
        let mut keys: Vec<(&[&str], &str)> = Vec::with_capacity(4);
        if self.order.len() > 1 {
            keys.push((&["Up", "Down"], "choose"));
        }
        if !enter.is_empty() {
            keys.push((&["Enter"], enter));
        }
        keys.push((&["Tab"], "next"));
        if self.hub {
            let next = crate::profile_hub::Tab::Collection.next(true);
            keys.push((&["Ctrl", "Tab"], next.label()));
        }
        keys.push((&["Esc"], "back"));
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

/// `text` cut to `chars` characters with an ellipsis.
fn cut(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_owned();
    }
    let kept: String = text.chars().take(chars.saturating_sub(3)).collect();
    format!("{}...", kept.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::Unlock;

    /// Owned and worn with the longest note, owned, locked and unknown, every focus, fit
    /// the canvas at 1080p, 4K, 4:3 and 21:9, in the families and in Inter.
    #[test]
    fn every_state_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let owned = [Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "n ".repeat(100),
        }];
        for (holdings, setting) in [
            (Holdings::Known(&owned), "saber_sun"),
            (Holdings::Known(&owned), ""),
            (Holdings::Known(&[]), "saber_sun"),
            (Holdings::IdentityOff, "saber_moon"),
            (Holdings::NoHub, ""),
            (Holdings::Waiting, ""),
        ] {
            let inputs = Inputs { holdings, setting };
            let mut panel = Panel::new();
            panel.open(true);
            for font in [&body.font, &inter.font] {
                for viewport in [
                    [1920.0, 1080.0],
                    [3840.0, 2160.0],
                    [1440.0, 1080.0],
                    [2560.0, 1080.0],
                ] {
                    panel.build(&inputs, font, viewport);
                    assert!(!panel.ui.overflowed(), "{holdings:?} at {viewport:?}");
                    // The cards stay inside the window and above the keys.
                    let frame = Frame::new(viewport);
                    let keys = frame.point(0.0, KEYS_Y)[1];
                    let rect = panel.ui.rect_for(CARD_BASE).expect("a card");
                    assert!(rect.x >= 0.0 && rect.right() <= viewport[0], "{viewport:?}");
                    assert!(rect.y + rect.height < keys, "{viewport:?}");
                }
            }
        }
    }

    #[test]
    fn the_header_says_what_is_worn() {
        let owned = [Unlock {
            id: "saber_sun".into(),
            granted: 0,
            note: String::new(),
        }];
        let worn = |holdings, setting| wearing(&Inputs { holdings, setting });
        assert_eq!(worn(Holdings::Known(&owned), "saber_sun").0, "Sun blade");
        let (name, line) = worn(Holdings::Known(&[]), "saber_sun");
        assert_eq!(name, "Stock blade");
        assert!(line.contains("shows once unlocked"));
        let (name, line) = worn(Holdings::IdentityOff, "saber_sun");
        assert_eq!(name, "Stock blade");
        assert!(line.contains("once the hub says"));
        assert_eq!(worn(Holdings::IdentityOff, "").0, "Stock blade");
        assert_eq!(cut("abcdefgh", 6), "abc...");
        assert_eq!(cut("abc", 6), "abc");
    }
}
