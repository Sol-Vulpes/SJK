//! The Profile screen's Medals tab (`docs/sjk-ui.md`, Profile screen): every medal the
//! client knows as a card, the ones the SJK team gave the player first, lit, with the
//! whole medal on its ribbon, its name (and how many times for one given again), when
//! it was given, what it is for and the team's note; the others dimmed, saying what
//! they are for and that they are not given yet. The medals come from the player's own
//! hub profile (`crate::medals::awards`), as the Players page and the profile card
//! read them.

use super::*;
use crate::medals::{Award, Medal};
use crate::menu::sjk::{Frame, color, text, wrap};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The cards: two columns, their size and the gaps between them (frame pixels, in the
/// page's moved frame; the cards start where the board's do).
const LEFT_X: f32 = 96.0;
const TOP: f32 = 170.0;
const CARDS_TOP: f32 = 218.0;
const CARD_WIDTH: f32 = 852.0;
const CARD_HEIGHT: f32 = 300.0;
const GAP: f32 = 24.0;
const COLUMNS: usize = 2;
/// The medal's picture in a card, and the text beside it.
const ART: f32 = 260.0;
const TEXT_X: f32 = ART + 44.0;
const TEXT_WIDTH: f32 = CARD_WIDTH - TEXT_X - 28.0;
/// Characters a line of a card's text holds (Exo 2 at 17).
const LINE_CHARS: usize = 56;

/// A medal as its card shows it: the player's award, or the medal they do not hold.
struct Showing {
    medal: Medal,
    award: Option<Award>,
}

/// What the tab says above the cards, and the cards: the medals given first, in the
/// catalogue's order, then the others.
fn showing(inputs: &Inputs<'_>) -> ((String, &'static str), Vec<Showing>) {
    let me = inputs
        .snapshot
        .filter(|_| inputs.enabled)
        .and_then(|snapshot| snapshot.me.as_ref());
    let awards = me.map(|me| crate::medals::awards(&me.medals));
    let held = awards.as_ref().map_or(0, Vec::len);
    let headline = match (inputs.enabled, inputs.snapshot, &awards) {
        (false, _, _) => (
            "Identity is off".to_owned(),
            "Your medals are kept on the SJK hub: switch the identity on in Settings, Network, to see them.",
        ),
        (true, _, None) => (
            "Contacting the hub...".to_owned(),
            "Your medals show once the SJK hub has answered.",
        ),
        (true, _, Some(_)) if held == 0 => (
            "No medals yet".to_owned(),
            "The SJK team gives medals for testing SJK, contributing to it and more.",
        ),
        (true, _, Some(_)) => (
            if held == 1 {
                "1 medal".to_owned()
            } else {
                format!("{held} medals")
            },
            "Given by the SJK team, and shown beside your name to every SJK player.",
        ),
    };
    let mut cards: Vec<Showing> = awards
        .unwrap_or_default()
        .into_iter()
        .map(|award| Showing {
            medal: award.medal,
            award: Some(award),
        })
        .collect();
    for medal in Medal::ALL {
        if !cards.iter().any(|card| card.medal == medal) {
            cards.push(Showing { medal, award: None });
        }
    }
    (headline, cards)
}

impl Panel {
    /// The Medals tab, in the page's moved `frame`.
    pub(super) fn medals(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let ((headline, line), cards) = showing(inputs);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(LEFT_X, TOP - 14.0, 900.0, 44.0),
            34.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X + 360.0, TOP - 4.0, 1_368.0, 26.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let bottom = self.bottom();
        for (index, card) in cards.iter().enumerate() {
            let x = LEFT_X + (index % COLUMNS) as f32 * (CARD_WIDTH + GAP);
            let y = CARDS_TOP + (index / COLUMNS) as f32 * (CARD_HEIGHT + GAP);
            if y + CARD_HEIGHT > bottom {
                break;
            }
            self.medal_card(frame, card, x, y);
        }
    }

    /// One medal's card at (`x`, `y`).
    fn medal_card(&mut self, frame: &Frame, card: &Showing, x: f32, y: f32) {
        let s = frame.s;
        let held = card.award.is_some();
        let rect = frame.rect(x, y, CARD_WIDTH, CARD_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 16.0 * s,
            color: if held {
                Color::new(0.07, 0.09, 0.16, 0.92)
            } else {
                color::alpha(color::SPACE, 0.62)
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 16.0 * s,
            width: 1.2 * s,
            color: if held {
                color::alpha(color::GOLD, 0.55)
            } else {
                color::alpha(color::HOLO, 0.18)
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: frame.rect(x + 20.0, y + 20.0, ART, ART),
            texture: card.medal.art(),
            color: if held {
                Color::new(1.0, 1.0, 1.0, 1.0)
            } else {
                Color::new(0.55, 0.58, 0.66, 0.35)
            },
        });
        let text_x = x + TEXT_X;
        let name = card
            .award
            .as_ref()
            .map_or_else(|| card.medal.name().to_owned(), Award::label);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(text_x, y + 26.0, TEXT_WIDTH, 40.0),
            32.0 * s,
            if held {
                color::GOLD_BRIGHT
            } else {
                color::alpha(color::TEXT, 0.7)
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let status = match &card.award {
            None => "Not given yet".to_owned(),
            Some(award) => {
                let date = crate::medals::date_text(award.awarded);
                match (award.count, date.is_empty()) {
                    (1, true) => "Given".to_owned(),
                    (1, false) => format!("Given {date}"),
                    (count, true) => format!("Given {count} times"),
                    (count, false) => format!("Given {count} times, last on {date}"),
                }
            }
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(text_x, y + 70.0, TEXT_WIDTH, 24.0),
            17.0 * s,
            if held { color::GOLD } else { color::QUIET },
            FontWeight::Regular,
            TextAlign::Start,
        );
        let mut line_y = y + 110.0;
        for part in wrap(card.medal.description(), LINE_CHARS).take(2) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(text_x, line_y, TEXT_WIDTH, 26.0),
                18.0 * s,
                if held { color::TEXT } else { color::MUTED },
                FontWeight::Regular,
                TextAlign::Start,
            );
            line_y += 26.0;
        }
        let Some(note) = card.award.as_ref().map(|award| award.note.as_str()) else {
            return;
        };
        if note.is_empty() {
            return;
        }
        line_y += 14.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("From the SJK team"),
            frame.rect(text_x, line_y, TEXT_WIDTH, 22.0),
            14.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        line_y += 24.0;
        let quoted = format!("\"{note}\"");
        let lines: Vec<&str> = wrap(&quoted, LINE_CHARS).collect();
        let room = ((y + CARD_HEIGHT - 16.0 - line_y) / 24.0).max(0.0) as usize;
        for (index, part) in lines.iter().take(room).enumerate() {
            // A note cut short ends with an ellipsis.
            let cut = index + 1 == room && lines.len() > room;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}{}", if cut { "..." } else { "" }),
                frame.rect(text_x, line_y, TEXT_WIDTH, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            line_y += 24.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::snapshot;
    use super::*;
    use sjk_identity::Profile;

    fn medal(id: &str, count: u32, note: &str) -> sjk_identity::Medal {
        sjk_identity::Medal {
            id: id.to_owned(),
            count,
            awarded: 1_791_336_225,
            note: note.to_owned(),
        }
    }

    fn inputs<'a>(enabled: bool, snapshot: Option<&'a Snapshot>) -> Inputs<'a> {
        Inputs {
            enabled,
            snapshot,
            standings: &[],
            record: &[],
        }
    }

    fn holding(medals: Vec<sjk_identity::Medal>) -> Snapshot {
        snapshot(
            Some(Profile {
                medals,
                ..super::super::tests::me("")
            }),
            None,
        )
    }

    /// The medals given come first, lit, then every other medal the client knows;
    /// a medal the client does not know is left out.
    #[test]
    fn the_medals_given_come_first_then_the_others() {
        let shot = holding(vec![
            medal("bug_hunter", 3, "three fixes"),
            medal("unknown", 1, ""),
            medal("early_tester", 1, ""),
        ]);
        let ((headline, _), cards) = showing(&inputs(true, Some(&shot)));
        assert_eq!(headline, "2 medals");
        let order: Vec<(Medal, bool)> = cards
            .iter()
            .map(|card| (card.medal, card.award.is_some()))
            .collect();
        assert_eq!(
            order,
            [
                (Medal::EarlyTester, true),
                (Medal::BugHunter, true),
                (Medal::EarlyContributor, false),
                (Medal::JofClan, false),
            ]
        );
        assert_eq!(
            cards[1].award.as_ref().map(Award::label).unwrap(),
            "Bug Hunter x3"
        );
        // Without the hub every medal shows, none given.
        for (enabled, snapshot) in [(false, Some(&shot)), (true, None)] {
            let (_, cards) = showing(&inputs(enabled, snapshot));
            assert_eq!(cards.len(), Medal::COUNT);
            assert!(cards.iter().all(|card| card.award.is_none()));
        }
        let ((headline, _), _) = showing(&inputs(true, Some(&holding(Vec::new()))));
        assert_eq!(headline, "No medals yet");
    }

    /// Every card fits the canvas and stands over the keys, notes at their longest, at
    /// 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn every_medal_fits_over_the_keys() {
        let note = "a very long note from the team ".repeat(10);
        let shot = holding(
            Medal::ALL
                .iter()
                .map(|kind| medal(kind.id(), 12, &note))
                .collect(),
        );
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            let mut panel = Panel::new();
            panel.open_as(super::super::Tab::Medals, true, Mode::Hub);
            panel.build(&inputs(true, Some(&shot)), &font, viewport);
            assert!(!panel.ui.overflowed(), "{viewport:?}");
            let frame = Frame::new(viewport);
            let keys = frame.point(0.0, 1_010.0)[1];
            let arts: Vec<sjk_ui::Rect> = panel
                .ui
                .draw_list()
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::TexturedQuad { rect, texture, .. }
                        if Medal::from_art(*texture).is_some() =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(arts.len(), Medal::COUNT, "{viewport:?}");
            for command in panel.ui.draw_list().commands() {
                if let DrawCommand::Text { rect, .. } = command {
                    assert!(rect.bottom() <= keys || rect.y >= keys, "{rect:?}");
                    assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                }
            }
        }
    }
}
